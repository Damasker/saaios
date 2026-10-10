/*
 * Separate, opt-in S5300 RFS quarantine diagnostic. The passive owner and
 * one-grant owner are unchanged. The original EFS is never opened for writing.
 * CP bytes may only replace a bounded prefix of a private candidate copy.
 * A final success response is gated on durable readback and a binary SHA-256
 * sidecar; neither response nor sidecar authorizes promotion to live NV.
 * Never logs NV bytes, digests, or identifiers.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <poll.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/file.h>
#include <sys/ioctl.h>
#include <sys/prctl.h>
#include <sys/random.h>
#include <sys/resource.h>
#include <sys/stat.h>
#include <sys/sysmacros.h>
#include <sys/vfs.h>
#include <linux/magic.h>
#include <time.h>
#include <unistd.h>
#include "sit-network-layout.h"
#include "sit-sim-layout.h"

/* The combined owner (SAAIOS_RFS_CAMP) adds an active IPC camp dispatcher on
 * top of the unchanged RFS quarantine serving: on the early radio edge it
 * issues the stock stage-1 0x093f -> 0x0404 -> 0x0800 while the RFS machinery
 * serves the protected-NV cmd7/cmd3/cmd6 sequence into the quarantine copy.
 * The SGC compile modes belong to the separate modem-channel-owner.c program. */
#if defined(SAAIOS_RFS_CAMP) && (defined(SAAIOS_SCAN_ONCE) || \
    defined(SAAIOS_SGC_ONCE) || defined(SAAIOS_SGC_EARLY_ONCE) || \
    defined(SAAIOS_SGC_SEQ_ONCE) || defined(SAAIOS_SGC_CAMP_ONCE))
#error "SAAIOS_RFS_CAMP is mutually exclusive with the SGC owner modes"
#endif

enum { BASELINE_BYTES = 524288, FIRST_CHUNK = 2012,
       RFS_TRANSFER_BYTES = 189446, RFS_GRANTS_MAX = 95,
       RFS_FRAME_MAX = 20 + FIRST_CHUNK, RX_CAP = 4096,
       POLL_MS = 250, EMPTY_BACKOFF_MS = 100,
       FIRST_DEADLINE_MS = 60000, STEP_DEADLINE_MS = 30000,
       TOTAL_DEADLINE_MS = 300000, BOOTING_LIMIT_MS = 60000,
       SIT_RX_CAP = 65536, SIT_READ_SLICE = 1024,
       SIT_REPLY_MS = 10000, SIT_SETTLED_MS = 60000,
       SIT_RFS_GUARD_MS = 1000, SIT_GET_COUNT = 4,
       SIT_EVENT_WINDOW_MS = 60000, SIT_EVENT_TRACE_LIMIT = 8,
       SIT_EVENT_QUIET_COVERAGE_MS = 300000, RFS_TRACE_MAX = 512 };
#define IOCTL_GET_OPENED_STATUS _IOR('o', 0x59, int)
#define SOURCE_NAME "nv_protected.bin"
#define PIN_NAME "expected.sha256"
#define USED_PIN_NAME "consumed.sha256"
#define MARKER_NAME "NO_PROMOTION"
#define CANDIDATE_NAME "candidate.bin"
#define SIDECAR_NAME "candidate.sha256"

enum phase { WAIT_7, WAIT_3, WAIT_6, WAIT_DATA, TERMINAL };
enum action { BAD_FRAME, NO_REPLY, STATUS_7, GRANT_1, STORE_CHUNK };
enum cp_state { CP_UNKNOWN, CP_OFFLINE, CP_BOOTING, CP_ONLINE, CP_CRASH };
enum failure_stage {
    STAGE_NONE, STAGE_GRANT, STAGE_FINAL_GRANT, STAGE_RFS_FRAME,
    STAGE_FINAL_FRAME, STAGE_STORE_CHUNK, STAGE_FINALIZE, STAGE_FINAL_ACK,
    STAGE_DEADLINE
};
enum failure_reason {
    REASON_NONE, REASON_STATE, REASON_GATE, REASON_SEND, REASON_NO_READ,
    REASON_PARTIAL_READ, REASON_LENGTH, REASON_MALFORMED, REASON_TRAILING,
    REASON_IO, REASON_READBACK, REASON_TIMEOUT, REASON_RX_OVERFLOW
};
enum padding_zero { PADDING_UNKNOWN, PADDING_YES, PADDING_NO };
enum {
    MISMATCH_LENGTH = 1u << 0, MISMATCH_COMMAND = 1u << 1,
    MISMATCH_SEQUENCE = 1u << 2, MISMATCH_PAYLOAD_SIZE = 1u << 3,
    MISMATCH_STATUS = 1u << 4, MISMATCH_FILE = 1u << 5,
    MISMATCH_CHUNK_SIZE = 1u << 6
};

static const char *const failure_stage_name[] = {
    "none", "grant", "final_grant", "rfs_frame", "final_rfs_frame",
    "store_chunk", "finalize", "final_ack", "deadline"
};
static const char *const failure_reason_name[] = {
    "none", "state", "gate", "send_ambiguous", "no_read", "partial_read",
    "length", "malformed", "trailing", "io", "readback", "timeout",
    "rx_overflow"
};

struct sha256 {
    uint32_t h[8];
    uint64_t bits;
    uint8_t block[64];
    size_t used;
};

struct sit_observer {
    uint8_t rx[SIT_RX_CAP];
    size_t used;
    uint32_t token;
    unsigned pass, next;
    int started, pending, poisoned, endpoint_failed;
    int64_t ready_ms, deadline_ms, backoff_until_ms;
    int64_t event_window_ms;
    unsigned event_seen, event_logged;
    uint64_t event_overflow;
};

#ifdef SAAIOS_RFS_CAMP
enum { SEQ_CONFIG_COMMAND = 0x093f, SEQ_CONFIG_LEN = 13,
       SGC_COMMAND = 0x0404, SGC_LEN = 24,
       CAMP_POWER_COMMAND = 0x0800, CAMP_POWER_LEN = 18, CAMP_POWER_ON = 2,
       /* RADIO_POWER (0x0800) power word, recovered from
        * ProtocolNetworkBuilder::BuildRadioPower(arg1,arg2,arg3): [12] = arg1 ?
        * 2 : 1, so OFF = 1 and ON = 2; [16]=arg2, [17]=arg3 (both 0 here). A
        * clean airplane-mode-style radio OFF -> ON cycle to force a deregistered
        * / limited-service window. Not IOCTL_POWER_OFF, not do_cp_crash. */
       CAMP_POWER_OFF = 1,
       /* Deregister-then-scan sub-sequence (VERDICT 15): internal step markers
        * (NOT wire opcodes). The scan/manual are fired from the limited-service
        * window right after RADIO_POWER ON, before the modem re-camps. */
       DRG_NONE = 0, DRG_OFF, DRG_OFF_GET, DRG_ON, DRG_SCAN, DRG_MANUAL,
       CAMP_RX_CAP = 4096, CAMP_WINDOW_MS = 30000,
       CAMP_PAIR_GAP_MS = 1000, CAMP_DISPATCH_MS = 1000,
       PROBE_START_MS = 2000, PROBE_REPLY_MS = 3000, PROBE_GAP_MS = 1500,
       PROBE_SIGNAL_MIN = 210,
       REG_RADIO_GET = 0x0801, REG_SEL_GET = 0x0703, REG_SEL_AUTO_SET = 0x0704,
       REG_PREF_GET = 0x070b, REG_PREF_SET = 0x070a, REG_ALLOW_DATA = 0x0710,
       REG_PREF_LEN = 16, REG_ALLOW_LEN = 13, REG_GET_MAX = 3,
       /* GET_AVAILABLE_NETWORKS (0x0706, header-only len 12) and
        * SET_NETWORK_SELECTION_MANUAL (0x0705, len 22) wire IDs; the frame
        * layouts live in sit-network-layout.h. The scan triggers a real RF
        * network scan, so its reply deadline is far longer than a normal GET. */
       REG_SCAN = SIT_NET_AVAILABLE_NETWORKS, REG_SCAN_LEN = 12,
       /* VERDICT 17: the len-16 GET_AVAILABLE_NETWORKS variant
        * (ProtocolNetworkBuilder::BuildQueryAvailableNetwork(int), @0x236950):
        * same opcode 0x0706 but total len 16, carrying an explicit scanType
        * int32 at payload[12]. The stock clamps the arg: scanType =
        * (1 <= arg <= 5) ? arg : 0 (sub w8,arg,#1; cmp #5; csel). So the
        * distinct accepted values are 0..5; the HAL's DoQueryBplmnSearch passes
        * 0, DoQueryAvailableNetwork passes a framework scanType (field logged as
        * "scanType=%d"). We sweep 0..5, each once. Same wire opcode as REG_SCAN;
        * distinguished internally by scan16_sent. */
       REG_SCAN16 = SIT_NET_AVAILABLE_NETWORKS, REG_SCAN16_LEN = 16,
       SCAN16_MODE_OFF = 12, SCAN16_MODE_MIN = 1, SCAN16_MODE_MAX = 5,
       SCAN16_MODE_COUNT = 6,
       REG_MANUAL_SEL = SIT_NET_SELECTION_MANUAL,
       REG_MANUAL_LEN = SIT_NET_SEL_MANUAL_LEN,
       PROBE_SCAN_REPLY_MS = 120000,
       /* SIT_SET_INITIAL_ATTACH_APN (0x0603): a 250-byte request, recovered
        * from ProtocolPsBuilder::BuildSetInitialAttachApn + FillApnInfo
        * <sit_pdp_set_initial_attach_apn_req> in the factory libsitril.so
        * (efcca0d5). The modem treats this as the PS-attach precondition: the
        * network-attach/default-bearer APN. All body bytes are recovered, none
        * invented. Frame-relative offsets: [12]=attach pdp cid, [13]=0x0e
        * (fixed in the builder), [14]=dataProfileId, [15]=apnType,
        * [16..115]=APN string (strlcpy, 100), [117..165]=username (49),
        * [167..215]=password (49), [217]=authType, [218]=pdpType,
        * [219]=pcscfReqType. For a plain IP APN with no auth every enum byte
        * resolves to a recovered constant (GetPdpType("IP")=1,
        * ConvertAuthTypeToProtocolAuthType(0)=0, default profile/apnType=0,
        * pcscfReqType=0); the attach cid is RetrieveAttachPdpContext's
        * profile-base+1 (base 0 on the default config => 1). This is not an
        * NV/EFS write.
        *
        * CP2A (the flashed build) differs: the full-body stock rild capture
        * (2026-10-05, boot-capture-2) shows a 251-byte frame with pdpType
        * [218]=3 (IPV4V6) and one appended byte [248]=3; every other body byte
        * matches the layout above. The frame below is that captured layout. */
       REG_INIT_ATTACH_APN = 0x0603, REG_IA_LEN = 251,
       REG_IA_APN_OFF = 16, REG_IA_APN_MAX = 100,
       REG_IA_CID = 1, REG_IA_CONST13 = 0x0e,
       REG_IA_AUTH_OFF = 217, REG_IA_PDPTYPE_OFF = 218, REG_IA_PCSCF_OFF = 219,
       REG_IA_PDPTYPE_IPV4V6 = 3, REG_IA_CP2A_OFF = 248, REG_IA_CP2A_VAL = 3,
       RAT_LTE_ONLY = 11, RAT_LTE_WCDMA = 12, RADIO_STATE_ON = 10,
       /* Untried operational-state SETs/GETs (open sitdef.h, same SIT family as
        * the proven 0x0700/0x0800/0x0710/0x093f wire IDs; 0x091a body also
        * cross-checked against the factory libsitril BuildSetVoiceOperation,
        * payload int32=3, len16). None is an NV/EFS write. One SET per boot,
        * selected by /data/saaios/etc/opx-step; GETs always read for the log. */
       OPX_STACK_GET = 0x0810, OPX_STACK_SET = 0x080f, OPX_STACK_LEN = 13,
       OPX_VOICE_GET = 0x091b, OPX_VOICE_SET = 0x091a, OPX_VOICE_LEN = 16,
       OPX_INTPS_SET = 0x0933, OPX_INTPS_LEN = 16,
       OPX_DEVSVC_GET = 0x0957, OPX_DEVSVC_SET = 0x0956, OPX_DEVSVC_LEN = 16,
       OPX_DUAL_SET = 0x072b, OPX_DUAL_LEN = 28,
       OPX_VOICE_MODE = 3, OPX_INTPS_MODE = 1, OPX_STACK_MODE_ENABLE = 1,
       OPX_DEVSVC_MODE_DATA = 2,
       OPX_DUAL_NET = 12, OPX_DUAL_ALLOW = 1,
       OPX_STEP_NONE = 0, OPX_STEP_VOICE = 1, OPX_STEP_INTPS = 2,
       OPX_STEP_STACK = 3, OPX_STEP_DEVSVC = 4, OPX_STEP_DUAL = 5,
       /* VERDICT 20: modern Android-13 RAT gate. Recovered byte-for-byte from
        * the factory libsitril.so (efcca0d5):
        *  - SetAllowedNetworkTypeBitmap (ProtocolNetworkBuilder::
        *    BuildSetAllowedNetworkTypeBitmap(int) @0x238670) -> opcode 0x074f,
        *    len 16, SIT-wire RAT bitmap int32 at payload[12]. The int arg is an
        *    Android RadioAccessFamily (RAF) bitmap that the builder re-encodes
        *    bit-for-bit into the wire bitmap (see raf_to_sit_ratbm, which mirrors
        *    the builder's exact bit transform).
        *  - GetAllowedNetworkTypeBitmap (@0x2387a0) -> opcode 0x0750, len 12
        *    (header-only); reply carries the wire bitmap int32 at payload[12]
        *    (confirmed in ProtocolNetGetAllowNetworkAdapter::GetRat @0x2340d0,
        *    which checks opcode 0x750 then reads frame[+12]).
        *  - QueryAvailableBandMode (@0x236ae0) -> opcode 0x0709, len 12
        *    (header-only); reply is a modem-defined band-mode list.
        * This is a revertible RAT-selection SET, not an NV/EFS/RF-cal write. */
       RATBM_SET = 0x074f, RATBM_SET_LEN = 16, RATBM_OFF = 12,
       RATBM_GET = 0x0750, RATBM_GET_LEN = 12,
       RATBM_BANDMODE_GET = 0x0709, RATBM_BANDMODE_LEN = 12,
       /* Android RadioAccessFamily bit positions (public AOSP
        * RadioAccessFamily.java); used only to feed the recovered builder
        * transform -- no wire bytes are invented. */
       RAF_GPRS = 1 << 1, RAF_EDGE = 1 << 2, RAF_UMTS = 1 << 3,
       RAF_HSDPA = 1 << 9, RAF_HSUPA = 1 << 10, RAF_HSPA = 1 << 11,
       RAF_LTE = 1 << 14, RAF_HSPAP = 1 << 15, RAF_GSM = 1 << 16,
       /* LTE + full WCDMA family + full GSM family (what step C arms). This RAF
        * passed through raf_to_sit_ratbm() yields wire bitmap 0x3fe. */
       RATBM_RAF_LTE_WCDMA_GSM = (1 << 1) | (1 << 2) | (1 << 3) | (1 << 9) |
                                 (1 << 10) | (1 << 11) | (1 << 14) | (1 << 15) |
                                 (1 << 16),
       RATBM_WIRE_LTE_WCDMA_GSM = 0x3fe,
       /* The CP2A modem refuses 0x3fe (error 2). Stock rild sends this RAF plus
        * NR (raf bit 20 -> wire bit 18): body fe 03 04 00 = 0x403fe, ACKed with
        * error 0 two seconds before LTE registration (boot-capture-2). */
       RAF_NR = 1 << 20,
       RATBM_RAF_STOCK = RATBM_RAF_LTE_WCDMA_GSM | RAF_NR,
       RATBM_WIRE_STOCK = 0x403fe,
       /* VERDICT 21/22: GET_BASEBAND_VERSION -> opcode 0x0901, len 13, a field
        * selector byte at payload[12]. The reply (opcode 0x0901) carries the SW
        * version C-string at frame offset 13 (HW ver @45, RF-cal date @77 are
        * NOT logged). The build string is a firmware id, not a secret.
        *
        * VERDICT 22: offsets are byte-identical across TD1A libsitril (efcca0d5,
        * GetSwVer @0x2268d0) and the CP2A vendor stream lib (cef87564,
        * ProtocolMiscVersionAdapter::GetSwVer @0x41e90, both frame+0xd) -- NO
        * drift. V21's empty SW-version was our bug: the selector byte must be
        * 0xFF. The stock stack calls ProtocolDeviceInfoBuilder::GetBaseBandVersion
        * with h=0xFF (BasebandVersionHandler::OnRequest @0x176e5c, CP2A
        * libsitril b488325d: "mov w1,#0xff"); sending 0 makes the CP return an
        * empty version. BBVER_MASK restores the stock selector. */
       BBVER_GET = 0x0901, BBVER_LEN = 13, BBVER_MASK = 0xFF,
       BBVER_SWVER_OFF = 13, BBVER_SWVER_MAX = 32,
       /* VERDICT 26: StartNetworkScan, byte-exact from the stock
        * ProtocolNetworkBuilder::BuildStartNetworkScan(int,int,int,
        * RIL_RadioAccessSpecifier_V1_5*,int,bool,int,int,char**) @0x75870 in the
        * CP2A SIT stream lib (cef87564). Request opcode 0x0734. Frame = 12-byte
        * SIT header + 10-byte scan scalars + numSpecifiers*78 + numMccMncs*6.
        * Scan scalars (relative to frame start): off12 scanType(1B),
        * off13 interval(2B, periodic only), off15 maxSearchTime(2B),
        * off17 incrementalResults(1B&1), off18 incrementalResultsPeriodicity
        * (2B, stock clamps to [3,10]), off20 numSpecifiers(1B,<=8),
        * off21 numMccMncs(1B,<=20). Each 78-byte specifier: off0 RAN(1B),
        * off1 bands_length(4B,<=8), off5 bands[8](1B each, EutranBands=3GPP
        * band numbers), off13 channels_length(1B,<=32), off14 channels[32]
        * (2B each). Empty channel list => scan the whole band. Result arrives
        * as unsolicited opcode 0x0736 (ProtocolNetScanResultAdapter, scanStatus
        * byte at frame+8, len>=13). RadioAccessNetworks: GERAN=1,UTRAN=2,
        * EUTRAN=3,NGRAN=4. No wire bytes invented. */
       SCAN734_GET = 0x0734, SCAN734_RESULT = 0x0736,
       SCAN734_LEN = 100, SCAN734_RAN_EUTRAN = 3,
       /* VERDICT 16: SIM PIN1 unlock + one activation voice call. All wire
        * IDs/body offsets recovered from the factory libsitril.so (efcca0d5);
        * nothing invented. ProtocolSimBuilderLegacy::BuildSimVerifyPin(0,...)
        * => PIN1 opcode 0x0201, total len 38: [12]=PIN length (strlen capped 8),
        * [13..] = PIN ASCII (AID omitted). The PIN is a secret: it is written
        * only into the request frame and is never printed, logged, or stored
        * anywhere else. app_state RIL enum: 2=PIN required, 5=READY. */
       CALL_PIN_VERIFY = 0x0201, CALL_PIN_LEN = 38,
       CALL_PIN_LEN_OFF = 12, CALL_PIN_OFF = 13, CALL_PIN_MAX = 8,
       SIM_APP_STATE_PIN = 2, SIM_APP_STATE_READY = 5,
       /* ProtocolCallBuilder voice MO call family:
        *   BuildDial      opcode 0x0001, len 104
        *   BuildGetCallList opcode 0x0000, len 12 (header-only GET)
        *   BuildHangup    opcode 0x0008, len 20
        * BuildDial body (frame-relative): [12]=call type (voice=0),
        * [13]=arg5(0), [14]=number length (<=82), [15..]=number ASCII,
        * [97]=TOA (0x10 if leading '+', else 0x20), [98]=1, [99]=clir (default
        * 0), [103]=arg6(0). GetCallList reply: count=int32 at [12], per-call
        * list at [16], stride 327 (v1_1); within an entry [0]=SIT call state,
        * [1..4]=call index. BuildHangup body: [12]=call index int32, [16]=1. */
       CALL_DIAL = 0x0001, CALL_DIAL_LEN = 104,
       CALL_DIAL_TYPE_OFF = 12, CALL_DIAL_ARG5_OFF = 13,
       CALL_DIAL_NUMLEN_OFF = 14, CALL_DIAL_NUM_OFF = 15, CALL_DIAL_NUM_MAX = 82,
       CALL_DIAL_TOA_OFF = 97, CALL_DIAL_PRESENT_OFF = 98,
       CALL_DIAL_CLIR_OFF = 99, CALL_DIAL_ARG6_OFF = 103,
       CALL_DIAL_TOA_INTL = 0x10, CALL_DIAL_TOA_NATL = 0x20,
       CALL_LIST = 0x0000, CALL_LIST_LEN = 12,
       CALL_LIST_COUNT_OFF = 12, CALL_LIST_ENTRY_OFF = 16,
       CALL_LIST_STATE_OFF = 0, CALL_LIST_INDEX_OFF = 1,
       CALL_HANGUP = 0x0008, CALL_HANGUP_LEN = 20,
       CALL_HANGUP_INDEX_OFF = 12, CALL_HANGUP_FLAG_OFF = 16,
       CALL_POLL_MAX = 6, CALL_POLL_REPLY_MS = 5000,
       /* call sub-sequence internal step markers (NOT wire opcodes) */
       CLL_NONE = 0, CLL_DIAL, CLL_POLL, CLL_HANGUP,
       /* ProtocolSmsBuilder::BuildSendSms. Opcode 0x0100 (0x0101 only when
        * the more-messages flag is set; this one-shot leaves it clear).
        * InitRequestHeader writes a 271-byte frame: opcode at +2, length at
        * +4, token at +6. Body: +12 is the 0..3 property byte (stock clamps
        * a missing property to 0), +13/+14 is the empty-SMSC fallback stock
        * writes when the SMSC string is absent (halfword 1), +26 is the raw
        * TPDU length, +27 is the TPDU (max 244). The TPDU is the public
        * SMS-SUBMIT of 3GPP TS 23.040; stock copies it in unchanged. */
       SMS_SEND = 0x0100, SMS_SEND_LEN = 271,
       SMS_PROP_OFF = 12, SMS_SMSC_OFF = 13, SMS_PDU_LEN_OFF = 26,
       SMS_PDU_OFF = 27, SMS_PDU_MAX = 244, SMS_DIGITS_MAX = 20,
       /* ProtocolMiscBuilder::BuildGetVoLteProvisionUpdate. Header-only
        * GET, opcode 0x0938, length 12. The reply status is one byte at
        * +12: zero or nonzero. The matching SET 0x0939 is the "update
        * done" notice and is not sent here. */
       VOLTE_PROV_GET = 0x0938, VOLTE_PROV_LEN = 12,
       VOLTE_PROV_STATUS_OFF = 12,
       /* ProtocolNetworkBuilder::BuildSetEmergencyCallStatus as called by
        * QueryEmergencyCallAvailableRadioTech: opcode 0x0712, length 14,
        * byte +12 = 1, byte +13 = 0xff. No dial string. The reply body is
        * not logged. */
       EM_QUERY = 0x0712, EM_QUERY_LEN = 14,
       EM_QUERY_MODE_OFF = 12, EM_QUERY_MODE = 1,
       EM_QUERY_ARG_OFF = 13, EM_QUERY_ARG = 0xff,
       /* ProtocolPsBuilder::BuildGetDataCallList. Header-only GET,
        * opcode 0x0602, length 12. The adapter reads the call count as
        * the first payload byte. The rest of the payload is not logged. */
       DCALL_LIST = 0x0602, DCALL_COUNT_OFF = 12 };

/* Active camp dispatcher. Isolated from the passive SIT observer: it keeps its
 * own streaming framer and token, never a SET on the RFS channel. It arms on
 * the exact 0x0803 -> 0x0802-raw0 radio edge and sends the stock stage-1 trio
 * once. A malformed/oversized IPC stream only disables the dispatcher. */
struct camp_driver {
    uint8_t rx[CAMP_RX_CAP];
    size_t used;
    int poisoned;
    int64_t owner_start_ms;
    int radio_stage;              /* 0 none, 1 saw exact 0x0803 */
    int radio_invalidated;
    int64_t radio_unavail_ms, radio_ready0_ms;
    uint32_t token;
    int dispatched;
    uint32_t cfg_token, sgc_token, camp_token;
    int cfg_sent, sgc_sent, camp_sent;
    int cfg_acked, sgc_acked, camp_acked;
    unsigned cfg_error, sgc_error, camp_error;
    /* Active prober: drives the SIM to READY and sustains signal/registration
     * tracing. It owns a private token namespace and a single outstanding GET.
     * A reply timeout only backs this prober off; it never self-poisons and
     * never stops the dispatcher or RFS quarantine. */
    uint32_t probe_token;
    int probe_pending;
    unsigned probe_idx, probe_id;
    const char *probe_name;
    int64_t probe_deadline_ms, probe_next_ms;
    int sim_change_pending, sim_ready;
    unsigned probe_sent, probe_replies, probe_timeouts;
    unsigned last_mask_low7;
    int mask_seen;
    /* One-shot registration-trigger sequence, run once the SIM is READY: confirm
     * radio ON, read selection mode (set auto if manual), read preferred RAT
     * (broaden LTE_ONLY->LTE_WCDMA so the present UMTS signal can be used), then
     * AllowData(1). Recovered/self-tested factory builders; each step sent at
     * most once, matched by id+token, non-poisoning. Then observation resumes. */
    int radio_on;
    int radio_get_tries, sel_get_tries, pref_get_tries;
    int sel_known, sel_mode, sel_auto_sent;
    int pref_known; unsigned preferred_raw; int pref_set_sent;
    /* Preferred-RAT target: 0 => default RAT_LTE_WCDMA (proven boot). Loaded at
     * startup from /data/saaios/etc/pref_rat; "11" selects RAT_LTE_ONLY for the
     * one controlled LTE-only acquisition trial. Revert = remove the file. */
    unsigned pref_target;
    int allow_data_sent, reg_complete;
    /* Data registration HOME (raw 1). Then stock 0x0625, the internet
     * 0x0613 profile, one SetupDataCall 0x0600, then 0x0605 fast dormancy. */
    int data_home, vonr_sent, profile_sent, setup_sent, fd_sent;
    int activity_sent, ims_sent, sos_sent;
    int endc_sent, vonrcapa_sent, rcnet_sent;
    int throttle_sent, unsol_wide_sent, unsol_sent, screen_sent;
    int cellinfo_sent, smsc_sent, vonrget_sent;
    /* Early stock frames that are named and carry no identifiers:
     * 0x0949 AP clock from localtime, 0x090b debug-trace byte 0,
     * 0x0903 TTY word 0, 0x0711 PS-service GET, 0x0740 preferred-data
     * modem byte 0. Each once, after the named post-setup chain. */
    int aptime_sent, dbgtrace_sent, tty_sent, pssvc_sent, prefmodem_sent;
    /* 0x024d slot-status GET (body stays off the log), 0x0943 signal
     * report criteria, 0x0107 SMS broadcast activation word 0. */
    int slot_sent, sigcrit_sent, smsact_sent;
    /* Initial stock 0x0943 bodies after the GERAN criteria already sent.
     * The later capture rows are live threshold updates and stay out. */
    int sigcrit_extra;
    /* Stock SIT_GET_PHONE_CAPABILITY. The 12-byte header keeps a zero
     * length field; every other empty GET in the capture stores 12. */
    int phonecap_sent;
    /* After the phone-capability read: one more XCAPM stop whose body
     * is 00 02 (the 0x0d3a twin has no paired reply), one empty
     * SIT_GET_ATR, and one SendSvnInfo. ATR and SVN replies stay off
     * the log. */
    int xcapstop2_sent, atr_sent, svn_sent;
    /* Named stock indications the 0x07xx/0x08xx tracer does not cover.
     * Each is recorded once. Signal keeps the low seven bits of the
     * signed halfword at +8. Link capacity keeps the four kbps words
     * at +8/+12/+16/+20. Other bodies stay off the log. */
    int ind_datacall, ind_signal, ind_linkcap;
    int ind_signal_mask;
    int link_dl, link_ul, link_dl2, link_ul2;
    /* 0x0604: count at +8, then the same 292-byte data-call item the
     * setup reply carries. Only cid, active and PDP type are logged. */
    int ind_dc_count, ind_dc_cid, ind_dc_active, ind_dc_pdp;
    /* 0x0742 V1.6 first record. Count is the signed word at +8.
     * Context ids stay off the log. */
    int ind_phy;
    int ind_phy_count, ind_phy_status, ind_phy_rat;
    int ind_phy_dl_ch, ind_phy_ul_ch, ind_phy_dl_bw, ind_phy_ul_bw;
    int ind_phy_pci, ind_phy_band;
    /* 0x0720: the five flags OnAcBarringInfo forwards. The factor
     * and timer words in the same frame stay off the log. */
    int ind_acbar;
    int ac_emc, ac_mosig, ac_modata, ac_voice, ac_video;
    /* 0x074b first barring record. The cell-identity prefix stays
     * off the log. cell is the wire type byte; prefix length comes
     * from the stock jump table. */
    int ind_barring;
    int bar_cell, bar_count, bar_svc, bar_kind, bar_factor, bar_time, bar_barred;
    /* LTE identity inside the 0x074b prefix. Operator strings stay off. */
    int bar_ci, bar_pci, bar_tac, bar_earfcn;
    /* Second barring record and how many records were logged. */
    int bar_nlogged;
    int bar2_svc, bar2_kind, bar2_factor, bar2_time, bar2_barred;
    /* How many of the five stock link-capacity criteria frames have
     * been sent. Access words are camp_link_access[]. */
    int linkcrit_next;
    /* 0 then 1: the two stock broadcast-SMS configs. */
    int smscb_next;
    /* Stock reads the call list once at boot. Empty GET. */
    int calllist_sent;
    /* Stock SetVoiceOperation (0x091a, int32 3) when no opx-step file
     * selected a different SET. */
    int voice_stock_sent;
    /* Named in rcmMsgToString. Stock bytes, no identifiers:
     * 0x0c20 GPS lock mode byte 1, 0x0c33 GPS NFW status byte 0,
     * 0x0755 GetSaMode empty GET (reply body stays off the log). */
    int gpslock_sent, gpsnfw_sent, samode_sent;
    /* First stock AIMS frames that ack with err 0 and body 01 01:
     * 0x0d3c XCAPM stop, 0x0d3a stack stop, 0x0d3b XCAPM start.
     * 0x0d39 stack start had no reply in the capture, so it stays unsent.
     * Stock then sends the same two stop opcodes again with body 00 01. */
    int xcapstop_sent, aimstop_sent, xcapstart_sent;
    int aimstop0_sent, xcapstop0_sent;
    /* Initial-attach APN (stock SET_INITIAL_ATTACH_APN 0x0603) sent once before
     * allow-data, only when an APN is configured. apn[] is loaded at startup from
     * /data/saaios/etc/apn; empty => the step is skipped (proven boot unchanged). */
    int ia_apn_sent;
    char apn[REG_IA_APN_MAX];
    /* Available-networks scan (0x0706) and manual network selection (0x0705),
     * both one-shot and config-gated. scan_enabled is set from the presence of
     * /data/saaios/etc/do_scan; manual_plmn[] is loaded from
     * /data/saaios/etc/manual_plmn (numeric MCC/MNC, e.g. "25506"). When
     * manual_plmn is set, step 2 issues manual selection instead of automatic.
     * Absent files => proven boot unchanged. */
    int scan_enabled, scan_sent;
    char manual_plmn[8];
    int manual_sel_sent;
    /* Deregister-then-scan sub-sequence (VERDICT 15), one-shot, config-gated by
     * /data/saaios/etc/dereg_scan. After the normal bring-up completes (modem
     * camped on the foreign cell), cycle RADIO_POWER OFF->ON and fire the scan
     * (and, if the home PLMN is then visible, a manual-select) from the
     * limited-service window before re-camp. */
    int dereg_enabled;
    int drg_off_sent, drg_off_ack;
    int drg_off_get_sent, drg_off_get_tries, drg_off_confirmed;
    int drg_on_sent, drg_on_ack;
    int drg_scan_sent, drg_scan_got;
    int drg_target_visible;
    int drg_manual_sent, drg_manual_ack;
    int drg_done;
    /* VERDICT 16: SIM PIN1 unlock + one activation voice call. pin[] is loaded
     * at startup from /data/saaios/etc/sim_pin and is a secret -- it is only
     * ever written into the PIN-verify request frame, never logged or copied
     * elsewhere. call_number[] (an ordinary dial string, not a secret) is
     * loaded from /data/saaios/etc/call_number; call_enabled gates the one-shot
     * activation call. voice_reg_state tracks the CS/voice registration state
     * (1=home, 5=roaming) so the call is only placed once CS registration is
     * achieved. Every step runs at most once. */
    int pin_required, pin_verify_sent, pin_verified;
    char pin[CALL_PIN_MAX + 1];
    int voice_reg_known, voice_reg_state;
    int call_enabled;
    char call_number[CALL_DIAL_NUM_MAX + 1];
    int call_dial_sent, call_dialed_ok;
    int call_poll_count, call_seen_count, call_index;
    unsigned call_last_state;
    int call_hangup_sent, call_done;
    /* One SMS-SUBMIT, armed by /data/saaios/etc/sms_number. The number is
     * loaded at start and written only into the TPDU. The log records the
     * length and error_raw. One attempt per boot. */
    int sms_enabled;
    char sms_number[SMS_DIGITS_MAX + 2];
    int sms_sent, sms_done;
    /* One read of the VoLTE provision-update flag. Header only. */
    int volteprov_sent, volteprov_done;
    /* One emergency-availability query. Not a dial. */
    int emquery_sent, emquery_done;
    /* One read of the data-call list. Count only. */
    int dcall_sent, dcall_done, dcall_count;
    /* VERDICT 17: len-16 scanType sweep (config /data/saaios/etc/scan16). After
     * bring-up completes, fire BuildQueryAvailableNetwork(int) once per distinct
     * accepted scanType 0..5, stopping early if any returns a result list. */
    int scan16_enabled;
    int scan16_mode;       /* scanType of the outstanding scan */
    int scan16_sent;       /* a len-16 scan is awaiting reply */
    int scan16_fired;      /* number of modes already fired (0..6) */
    int scan16_done, scan16_got_list;
    /* VERDICT 20: modern RAT-gate experiment (config /data/saaios/etc/ratbm).
     * After reg_complete: read GET 0x750 + band-mode 0x709 (diagnostic), SET
     * 0x074f with an LTE-inclusive bitmap, then re-read 0x750 to confirm it
     * took. Every step runs at most once. */
    int ratbm_enabled;
    int ratbm_get1_sent, ratbm_band_sent, ratbm_set_sent, ratbm_get2_sent;
    int ratbm_done;
    /* VERDICT 21: read-only baseband/SW-version query (config
     * /data/saaios/etc/bbver). Fires once after reg_complete. */
    int bbver_enabled, bbver_sent, bbver_done;
    /* VERDICT 25: read-only CP capability diagnostic (config
     * /data/saaios/etc/capquery). Skips every operator-control/registration SET
     * (0x070a preferred, 0x0704 selection, 0x0710 allow-data, 0x074f bitmap) and
     * fires ONLY GET opcodes: selection/preferred reads, allowed-RAT bitmap GET
     * 0x750, available-band GET 0x709, baseband-version GET 0x901. Nothing is
     * written and no modem state is changed. */
    int capquery;
    /* VERDICT 26: one-shot StartNetworkScan (0x0734), one EUTRAN specifier
     * with an empty band list (config /data/saaios/etc/scan734). After the
     * normal bring-up, one RADIO_POWER OFF→confirm→ON cycle, then exactly one
     * 0x0734 in that pre-camp window. Logs 0x0736 (scanStatus + bounded header
     * hex + elapsed_ms). Does not also send the legacy 0x0706 scan. */
    int scan734, scan734_sent, scan734_done;
    /* Post-registration one-shot operational-SET experiment. After reg_complete
     * the three GETs below are read once for the log, then the single SET named
     * by opx_step is sent once (matched by id+token, non-poisoning). */
    int opx_step;                 /* OPX_STEP_* selected at startup from file */
    int opx_stack_get_sent, opx_voice_get_sent, opx_devsvc_get_sent;
    int opx_stack_known; unsigned opx_stack_mode;
    int opx_set_sent, opx_done;
};
#endif

struct owner {
    int ipc, rfs, ready, lock;
    int source, pin_fd, source_dir, pin_dir;
    int quarantine_parent, quarantine_dir, candidate, marker_fd, sidecar_fd;
    struct stat source_stat, pin_stat, candidate_stat, marker_stat, sidecar_stat;
    struct stat source_dir_stat, pin_dir_stat;
    struct stat quarantine_parent_stat, quarantine_dir_stat;
    char quarantine_leaf[48];
    uint8_t pin_digest[32];
    uint8_t rx[RX_CAP];
    size_t used;
    enum phase phase;
    int64_t deadline_ms;
    int64_t total_deadline_ms;
    int grant_attempted;
    int chunks_stored;
    uint32_t received_bytes, expected_chunk;
    uint16_t nv_seq;
    struct sha256 received_hash;
    uint8_t candidate_digest[32];
    int final_ack_attempted;
    int final_ack_sent;
    int pin_consumed;
    enum failure_stage failure_stage;
    enum failure_reason failure_reason;
    unsigned frame_mismatch_mask;
    unsigned final_parsed_len, final_outer_payload_len, final_trailing;
    enum padding_zero final_padding_zero;
    struct sit_observer sit;
#ifdef SAAIOS_RFS_CAMP
    struct camp_driver camp;
#endif
};

static volatile sig_atomic_t stop_requested;
#ifdef RFS_HOST_TEST
static ssize_t (*host_write_override)(int, const void *, size_t);
static unsigned host_write_calls;
static int host_gate_override;
static int host_store_override;
static int host_finish_override;
static int host_sidecar_close_error;
static int host_sidecar_sync_error;
static ssize_t (*host_sit_write_override)(int, const void *, size_t);
static unsigned host_sit_write_calls;
#endif
static const uint8_t request_7[12] =
    {7,0,0,0, 4,0,0,0, 3,0,0,0};
static const uint8_t request_3[20] =
    {3,0,0,0, 12,0,0,0, 0,0,0,0, 3,0,0,0, 0,0,0,0};
static const uint8_t request_6[24] =
    {6,0,1,0, 16,0,0,0, 3,0,0,0, 0,0,0,0,
     0x06,0xe4,0x02,0, 2,0,0,0};
static const uint8_t status_7[16] =
    {3,0,0,0, 8,0,0,0, 0,0,0,0, 3,0,0,0};
static const uint8_t final_status[16] =
    {3,0,1,0, 8,0,0,0, 0,0,0,0, 3,0,0,0};

static void zero_bytes(void *pointer, size_t count)
{
    volatile uint8_t *p = (volatile uint8_t *)pointer;
    while (count--) *p++ = 0;
}

/* Only fixed stage/reason labels reach the terminal log; never frame data. */
static void diagnose(struct owner *o, enum failure_stage stage,
                     enum failure_reason reason)
{
    if (o->failure_stage == STAGE_NONE) {
        o->failure_stage = stage;
        o->failure_reason = reason;
    }
}

static enum failure_stage frame_stage(const struct owner *o)
{
    return o->grant_attempted == RFS_GRANTS_MAX &&
           o->chunks_stored == RFS_GRANTS_MAX - 1 ?
           STAGE_FINAL_FRAME : STAGE_RFS_FRAME;
}

static uint32_t rotate_right(uint32_t n, unsigned bits)
{
    return (n >> bits) | (n << (32u - bits));
}

static uint32_t big32(const uint8_t *p)
{
    return ((uint32_t)p[0] << 24) | ((uint32_t)p[1] << 16) |
           ((uint32_t)p[2] << 8) | p[3];
}

static uint16_t little16(const uint8_t *p)
{
    return (uint16_t)((uint16_t)p[0] | ((uint16_t)p[1] << 8));
}

static uint32_t little32(const uint8_t *p)
{
    return (uint32_t)p[0] | ((uint32_t)p[1] << 8) |
           ((uint32_t)p[2] << 16) | ((uint32_t)p[3] << 24);
}

/* CP access-tech -> RIL RADIO_TECH_* map, recovered byte-for-byte from the
 * libsitril.so (efcca0d5) .rodata table at 0xd8afc that Get{Voice,Data}
 * RegStateAdapter::GetRadioTech() indexes with (raw_tech - 1) after bounding
 * (raw-1) <= 0x14. Returns 0 (unknown) for out-of-range raw, matching the
 * stock fall-through. RIL RAT: 3=UMTS 14=LTE 16=GSM 20=NR. */
static unsigned sit_net_rat_map(unsigned raw)
{
    static const uint8_t table[21] = {
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 17, 15, 14,
        20
    };
    unsigned idx = raw - 1u;
    return (raw >= 1u && idx < sizeof table) ? table[idx] : 0u;
}

/* Decode a voice (0x0700) / data (0x0701) registration-state response using the
 * stock fixed-offset layout. Read-only; logs only RAT/LAC/cell/PSC topology,
 * never a subscriber identifier. Each field is length-guarded so a short frame
 * degrades gracefully. The TRUE reject cause is reject_raw (offset 13), as the
 * stock adapter GetRejectCause() reads it -- confirmed, not a misread. */
static void sit_net_log_regstate(const char *prefix, const char *name,
                                 const uint8_t *p, size_t n, unsigned id)
{
    unsigned tech_off = id == SIT_NET_DATA_REG ? SIT_NET_DATA_TECH_OFFSET
                                               : SIT_NET_VOICE_TECH_OFFSET;
    unsigned lac_off = id == SIT_NET_DATA_REG ? SIT_NET_DATA_LAC_OFFSET
                                              : SIT_NET_VOICE_LAC_OFFSET;
    unsigned cid_off = id == SIT_NET_DATA_REG ? SIT_NET_DATA_CID_OFFSET
                                              : SIT_NET_VOICE_CID_OFFSET;
    unsigned psc_off = id == SIT_NET_DATA_REG ? SIT_NET_DATA_PSC_OFFSET
                                              : SIT_NET_VOICE_PSC_OFFSET;
    printf("%s field=%s registration_raw=%u reject_raw=%u", prefix, name,
           p[SIT_NET_REG_STATE_OFFSET], p[SIT_NET_REJECT_OFFSET]);
    if (n > tech_off)
        printf(" tech_raw=%u rat_mapped=%u", p[tech_off],
               sit_net_rat_map(p[tech_off]));
    if (n >= (size_t)lac_off + 2u)
        printf(" lac=%u", little16(p + lac_off));
    if (n >= (size_t)cid_off + 4u)
        printf(" cid=%lu", (unsigned long)little32(p + cid_off));
    if (n > psc_off)
        printf(" psc=%u", p[psc_off]);
    printf(" frame_len=%zu\n", n);
}

/* Decode a GET_OPERATOR (0x0702) response. The serving/registered PLMN numeric
 * (MCC+MNC ASCII) is 6 bytes at payload offset 12; a trailing '#' (0x23) marks a
 * 2-digit MNC (3GPP TS 24.008 filler). Recovered from libsitril
 * ProtocolNetOperatorAdapter::Init(): [12..17]=PLMN numeric, [18..]=short name,
 * [50..]=long name. Read-only; MCC/MNC is network topology, not a subscriber id.
 * Names are not logged (operator long/short names can embed non-numeric PII-ish
 * branding; the numeric PLMN is sufficient for home-vs-foreign classification). */
static void sit_net_log_operator(const char *prefix, const uint8_t *p, size_t n)
{
    char plmn[7];
    size_t i;
    if (n < 18) { printf("%s field=operator status=unknown_short frame_len=%zu\n",
                         prefix, n); return; }
    for (i = 0; i < 6; i++) {
        uint8_t c = p[12 + i];
        plmn[i] = (c >= '0' && c <= '9') ? (char)c : (c == 0x23 ? '#' : '.');
    }
    plmn[6] = '\0';
    printf("%s field=operator plmn_numeric=%s frame_len=%zu\n", prefix, plmn, n);
}

/* Scan-entry access-tech -> RIL RADIO_TECH_* map, recovered byte-for-byte from
 * libsitril.so ProtocolNetAvailableNetworkAdapter::GetNetwork(): only raw values
 * 0x11..0x15 are remapped via the .rodata table at 0xd8ac8
 * {0,0x11,0x0f,0x0e,0x14}; every other raw value passes through unchanged (so
 * raw 3=UMTS, 16=GSM map directly). This differs from the reg-state RAT map. */
static unsigned sit_scan_rat_map(unsigned raw)
{
    static const uint8_t table[5] = { 0, 17, 15, 14, 20 };
    return (raw >= 0x11u && raw <= 0x15u) ? table[raw - 0x11u] : raw;
}

static const char *sit_scan_status_name(unsigned status)
{
    switch (status) {
    case 1: return "available";
    case 2: return "current";
    case 3: return "forbidden";
    default: return "unknown";
    }
}

/* Decode a GET_AVAILABLE_NETWORKS (0x0706) response per the stock adapter: a
 * count int32 at payload[12] followed by 14-byte entries at payload[16]. Each
 * entry is reported as its numeric PLMN (MCC/MNC ASCII, network topology -- not a
 * subscriber id), availability status and RAT. Length-guarded against a truncated
 * frame. Returns the count of PLMNs parsed. */
static int sit_net_log_available(const char *prefix, const uint8_t *p, size_t n,
                                 const char *want, int *seen)
{
    unsigned count, i;
    if (seen) *seen = 0;
    if (n < (size_t)SIT_NET_AVN_COUNT_OFFSET + 4u) {
        printf("%s field=available_networks status=short frame_len=%zu\n",
               prefix, n);
        return 0;
    }
    count = little32(p + SIT_NET_AVN_COUNT_OFFSET);
    printf("%s field=available_networks count=%u frame_len=%zu\n",
           prefix, count, n);
    for (i = 0; i < count; i++) {
        size_t base = (size_t)SIT_NET_AVN_LIST_OFFSET +
                      (size_t)i * SIT_NET_AVN_ENTRY_STRIDE;
        const uint8_t *e;
        char plmn[7];
        unsigned rat_raw, status;
        size_t j;
        if (base + SIT_NET_AVN_ENTRY_STRIDE > n) {
            printf("%s field=available_networks status=truncated_at=%u\n",
                   prefix, i);
            break;
        }
        e = p + base;
        rat_raw = little32(e + SIT_NET_AVN_ENTRY_RAT);
        status = little32(e + SIT_NET_AVN_ENTRY_STATUS);
        for (j = 0; j < 6; j++) {
            uint8_t c = e[SIT_NET_AVN_ENTRY_PLMN + j];
            plmn[j] = (c >= '0' && c <= '9') ? (char)c
                                             : (c == 0x23 ? '#' : '.');
        }
        plmn[6] = '\0';
        printf("%s field=available_networks idx=%u plmn_numeric=%s "
               "status_raw=%u status=%s rat_raw=%u rat_mapped=%u\n",
               prefix, i, plmn, status, sit_scan_status_name(status),
               rat_raw, sit_scan_rat_map(rat_raw));
        if (seen && want && want[0]) {
            size_t wl = strlen(want);
            if (wl >= 5 && wl <= 6 && !strncmp(plmn, want, wl)) *seen = 1;
        }
    }
    return (int)count;
}

static int same_bytes(const uint8_t *a, const uint8_t *b, size_t n)
{
    uint8_t difference = 0;
    for (size_t i = 0; i < n; ++i) difference |= (uint8_t)(a[i] ^ b[i]);
    return difference == 0;
}

/* SHA-256 is local so the ARM diagnostic needs no Android crypto service. */
static void sha_transform(struct sha256 *ctx, const uint8_t block[64])
{
    static const uint32_t k[64] = {
        0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,
        0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
        0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,
        0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
        0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,
        0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
        0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,
        0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
        0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,
        0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
        0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,
        0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
        0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,
        0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
        0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,
        0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2
    };
    uint32_t w[64];
    uint32_t a, b, c, d, e, f, g, h;
    for (unsigned i = 0; i < 16; ++i) w[i] = big32(block + 4u * i);
    for (unsigned i = 16; i < 64; ++i) {
        uint32_t s0 = rotate_right(w[i-15], 7) ^
                      rotate_right(w[i-15], 18) ^ (w[i-15] >> 3);
        uint32_t s1 = rotate_right(w[i-2], 17) ^
                      rotate_right(w[i-2], 19) ^ (w[i-2] >> 10);
        w[i] = w[i-16] + s0 + w[i-7] + s1;
    }
    a=ctx->h[0]; b=ctx->h[1]; c=ctx->h[2]; d=ctx->h[3];
    e=ctx->h[4]; f=ctx->h[5]; g=ctx->h[6]; h=ctx->h[7];
    for (unsigned i = 0; i < 64; ++i) {
        uint32_t s1 = rotate_right(e,6) ^ rotate_right(e,11) ^ rotate_right(e,25);
        uint32_t ch = (e & f) ^ (~e & g);
        uint32_t t1 = h + s1 + ch + k[i] + w[i];
        uint32_t s0 = rotate_right(a,2) ^ rotate_right(a,13) ^ rotate_right(a,22);
        uint32_t maj = (a & b) ^ (a & c) ^ (b & c);
        uint32_t t2 = s0 + maj;
        h=g; g=f; f=e; e=d+t1; d=c; c=b; b=a; a=t1+t2;
    }
    ctx->h[0]+=a; ctx->h[1]+=b; ctx->h[2]+=c; ctx->h[3]+=d;
    ctx->h[4]+=e; ctx->h[5]+=f; ctx->h[6]+=g; ctx->h[7]+=h;
    zero_bytes(w, sizeof w);
}

static void sha_init(struct sha256 *ctx)
{
    static const uint32_t initial[8] = {
        0x6a09e667,0xbb67ae85,0x3c6ef372,0xa54ff53a,
        0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19
    };
    memset(ctx, 0, sizeof *ctx);
    memcpy(ctx->h, initial, sizeof initial);
}

static void sha_update(struct sha256 *ctx, const uint8_t *data, size_t len)
{
    ctx->bits += (uint64_t)len * 8u;
    while (len) {
        size_t space = 64u - ctx->used;
        size_t take = len < space ? len : space;
        memcpy(ctx->block + ctx->used, data, take);
        ctx->used += take;
        data += take;
        len -= take;
        if (ctx->used == 64u) {
            sha_transform(ctx, ctx->block);
            ctx->used = 0;
        }
    }
}

static void sha_final(struct sha256 *ctx, uint8_t out[32])
{
    uint64_t bits = ctx->bits;
    ctx->block[ctx->used++] = 0x80;
    if (ctx->used > 56u) {
        memset(ctx->block + ctx->used, 0, 64u - ctx->used);
        sha_transform(ctx, ctx->block);
        ctx->used = 0;
    }
    memset(ctx->block + ctx->used, 0, 56u - ctx->used);
    for (unsigned i = 0; i < 8; ++i)
        ctx->block[56u+i] = (uint8_t)(bits >> (56u - 8u*i));
    sha_transform(ctx, ctx->block);
    for (unsigned i = 0; i < 8; ++i) {
        out[4u*i] = (uint8_t)(ctx->h[i] >> 24);
        out[4u*i+1] = (uint8_t)(ctx->h[i] >> 16);
        out[4u*i+2] = (uint8_t)(ctx->h[i] >> 8);
        out[4u*i+3] = (uint8_t)ctx->h[i];
    }
    zero_bytes(ctx, sizeof *ctx);
}

static int read_all_at(int fd, void *out, size_t len, off_t offset)
{
    size_t done = 0;
    while (done < len) {
        ssize_t got = pread(fd, (uint8_t *)out + done, len - done,
                            offset + (off_t)done);
        if (got < 0 && errno == EINTR) continue;
        if (got <= 0) return -1;
        done += (size_t)got;
    }
    return 0;
}

static int write_all_at(int fd, const void *bytes, size_t len, off_t offset)
{
    size_t done = 0;
    while (done < len) {
        ssize_t count = pwrite(fd, (const uint8_t *)bytes + done, len - done,
                               offset + (off_t)done);
        if (count < 0 && errno == EINTR) continue;
        if (count <= 0) return -1;
        done += (size_t)count;
    }
    return 0;
}

static int same_file(const struct stat *a, const struct stat *b)
{
    return a->st_dev == b->st_dev && a->st_ino == b->st_ino &&
           a->st_mode == b->st_mode && a->st_uid == b->st_uid &&
           a->st_nlink == b->st_nlink && a->st_size == b->st_size &&
           a->st_mtim.tv_sec == b->st_mtim.tv_sec &&
           a->st_mtim.tv_nsec == b->st_mtim.tv_nsec &&
           a->st_ctim.tv_sec == b->st_ctim.tv_sec &&
           a->st_ctim.tv_nsec == b->st_ctim.tv_nsec;
}

static int same_directory_identity(const struct stat *a,
                                   const struct stat *b)
{
    return a->st_dev == b->st_dev && a->st_ino == b->st_ino &&
           a->st_mode == b->st_mode && a->st_uid == b->st_uid &&
           a->st_gid == b->st_gid && a->st_nlink == b->st_nlink &&
           S_ISDIR(a->st_mode);
}

static int host_fixture_mode(void)
{
#ifdef RFS_HOST_TEST
    return host_gate_override;
#else
    return 0;
#endif
}

static int regular_exact(int fd, off_t size, mode_t mode, struct stat *out)
{
    int flags = fcntl(fd, F_GETFL);
    if (flags < 0 || (flags & O_ACCMODE) != O_RDONLY ||
        fstat(fd, out) || !S_ISREG(out->st_mode) ||
        out->st_uid != 0 || out->st_nlink != 1 ||
        (out->st_mode & 07777) != mode || out->st_size != size)
        return -1;
    return 0;
}

static int directory_exact(int fd, mode_t mode)
{
    struct stat st;
    if (fstat(fd, &st) || !S_ISDIR(st.st_mode) || st.st_uid != 0 ||
        (st.st_mode & 07777) != mode || st.st_nlink < 2) return -1;
    return 0;
}

/* O_NOFOLLOW on every component, including trusted fixed-path ancestors. */
static int walk_directory(const char *const *parts, size_t count)
{
    int fd = open("/", O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    if (fd < 0) return -1;
    for (size_t i = 0; i < count; ++i) {
        int next = openat(fd, parts[i],
                          O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
        close(fd);
        if (next < 0) return -1;
        fd = next;
    }
    return fd;
}

static int named_regular(int dirfd, const char *name, int fd,
                         const struct stat *expected)
{
    struct stat named, held;
    return fstat(fd, &held) == 0 && same_file(&held, expected) &&
           fstatat(dirfd, name, &named, AT_SYMLINK_NOFOLLOW) == 0 &&
           same_file(&named, expected) && S_ISREG(named.st_mode) ? 0 : -1;
}

static int named_directory(const char *path, int fd,
                           const struct stat *expected)
{
    struct stat held, named;
    return fstat(fd, &held) == 0 && same_file(&held, expected) &&
           lstat(path, &named) == 0 && same_file(&named, expected) &&
           S_ISDIR(named.st_mode) ? 0 : -1;
}

static int decode_lower_hex(const uint8_t chars[64], uint8_t digest[32])
{
    for (size_t i = 0; i < 32; ++i) {
        unsigned high, low;
        uint8_t a = chars[2*i], b = chars[2*i+1];
        if (a >= '0' && a <= '9') high = a - '0';
        else if (a >= 'a' && a <= 'f') high = a - 'a' + 10u;
        else return -1;
        if (b >= '0' && b <= '9') low = b - '0';
        else if (b >= 'a' && b <= 'f') low = b - 'a' + 10u;
        else return -1;
        digest[i] = (uint8_t)((high << 4) | low);
    }
    return 0;
}

static int original_efs_unmounted(void)
{
    FILE *f = fopen("/sys/block/sda/sda5/uevent", "r");
    char line[4096];
    unsigned partname = 0, devname = 0;
    unsigned major_seen = 0, minor_seen = 0;
    unsigned major = 0, minor = 0;
    if (!f) return -1;
    while (fgets(line, sizeof line, f)) {
        if (!strchr(line, '\n') && !feof(f)) { fclose(f); return -1; }
        line[strcspn(line, "\r\n")] = 0;
        if (!strcmp(line, "PARTNAME=efs")) ++partname;
        else if (!strcmp(line, "DEVNAME=sda5")) ++devname;
        else if (!strncmp(line, "MAJOR=", 6)) {
            char extra;
            if (sscanf(line + 6, "%u%c", &major, &extra) != 1) {
                fclose(f); return -1;
            }
            ++major_seen;
        } else if (!strncmp(line, "MINOR=", 6)) {
            char extra;
            if (sscanf(line + 6, "%u%c", &minor, &extra) != 1) {
                fclose(f); return -1;
            }
            ++minor_seen;
        } else if (!strncmp(line, "PARTNAME=", 9) ||
                   !strncmp(line, "DEVNAME=", 8)) {
            fclose(f); return -1;
        }
    }
    int io_error = ferror(f);
    int close_error = fclose(f);
    if (io_error || close_error != 0 || partname != 1 || devname != 1 ||
        major_seen != 1 || minor_seen != 1) return -1;
    f = fopen("/sys/block/sda/sda5/dev", "r");
    if (!f) return -1;
    unsigned actual_major = 0, actual_minor = 0;
    int got = fscanf(f, "%u:%u", &actual_major, &actual_minor);
    if (fclose(f) != 0 || got != 2 || actual_major != major ||
        actual_minor != minor) return -1;
    f = fopen("/proc/self/mountinfo", "r");
    if (!f) return -1;
    while (fgets(line, sizeof line, f)) {
        unsigned mounted_major = 0, mounted_minor = 0;
        if (!strchr(line, '\n') && !feof(f)) { fclose(f); return -1; }
        if (sscanf(line, "%*u %*u %u:%u", &mounted_major,
                   &mounted_minor) != 2) { fclose(f); return -1; }
        if (mounted_major == major && mounted_minor == minor) {
            fclose(f); return -1;
        }
    }
    io_error = ferror(f);
    close_error = fclose(f);
    int ok = !io_error && close_error == 0;
    return ok ? 0 : -1;
}

/* A dedicated /run tmpfs makes this pin boot-local, not persisted userdata. */
static int run_tmpfs(int pin_dir)
{
    int run_fd = open("/run", O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    struct stat held, named, pin_stat;
    struct statfs fs;
    char line[4096], mountpoint[256], fstype[64];
    unsigned matches = 0;
    if (run_fd < 0) return -1;
    int valid = fstat(run_fd, &held) == 0 &&
                lstat("/run", &named) == 0 &&
                S_ISDIR(held.st_mode) && held.st_uid == 0 &&
                (held.st_mode & 07777) == 0755 &&
                held.st_dev == named.st_dev && held.st_ino == named.st_ino &&
                held.st_mode == named.st_mode &&
                fstatfs(run_fd, &fs) == 0 &&
                (unsigned long)fs.f_type == TMPFS_MAGIC &&
                fstat(pin_dir, &pin_stat) == 0 &&
                pin_stat.st_dev == held.st_dev;
    close(run_fd);
    if (!valid) return -1;
    FILE *f = fopen("/proc/self/mountinfo", "r");
    if (!f) return -1;
    while (fgets(line, sizeof line, f)) {
        if (!strchr(line, '\n') && !feof(f)) {
            fclose(f); return -1;
        }
        char *split = strstr(line, " - ");
        if (!split) { fclose(f); return -1; }
        *split = 0;
        if (sscanf(line, "%*s %*s %*s %*s %255s", mountpoint) != 1 ||
            sscanf(split + 3, "%63s", fstype) != 1) {
            fclose(f); return -1;
        }
        if (!strcmp(mountpoint, "/run")) {
            ++matches;
            if (strcmp(fstype, "tmpfs")) { fclose(f); return -1; }
        }
    }
    int io_error = ferror(f);
    int close_error = fclose(f);
    return !io_error && close_error == 0 && matches == 1 ? 0 : -1;
}

static int load_source_and_pin(struct owner *o)
{
    static const char *const source_path[] =
        {"data", "saaios", "var", "efs-copy"};
    static const char *const pin_path[] =
        {"run", "saaios-rfs-one-grant"};
    uint8_t chars[64];
    o->source_dir = walk_directory(source_path, 4);
    o->pin_dir = walk_directory(pin_path, 2);
    if (o->source_dir < 0 || o->pin_dir < 0) return -1;
    if (fstat(o->source_dir, &o->source_dir_stat) ||
        !S_ISDIR(o->source_dir_stat.st_mode) ||
        o->source_dir_stat.st_uid != 0 ||
        ((o->source_dir_stat.st_mode & 07777) != 0700 &&
         (o->source_dir_stat.st_mode & 07777) != 0755) ||
        fstat(o->pin_dir, &o->pin_dir_stat) ||
        directory_exact(o->pin_dir, 0700) || run_tmpfs(o->pin_dir) ||
        named_directory("/data/saaios/var/efs-copy", o->source_dir,
                        &o->source_dir_stat) ||
        named_directory("/run/saaios-rfs-one-grant", o->pin_dir,
                        &o->pin_dir_stat))
        return -1;
    struct stat used;
    if (fstatat(o->pin_dir, USED_PIN_NAME, &used,
                AT_SYMLINK_NOFOLLOW) == 0 || errno != ENOENT) return -1;
    o->source = openat(o->source_dir, SOURCE_NAME,
                       O_RDONLY | O_NOFOLLOW | O_CLOEXEC);
    o->pin_fd = openat(o->pin_dir, PIN_NAME,
                       O_RDONLY | O_NOFOLLOW | O_CLOEXEC);
    if (o->source < 0 || o->pin_fd < 0 ||
        regular_exact(o->source, BASELINE_BYTES, 0600, &o->source_stat) ||
        regular_exact(o->pin_fd, 64, 0600, &o->pin_stat) ||
        named_regular(o->source_dir, SOURCE_NAME, o->source,
                      &o->source_stat) ||
        named_regular(o->pin_dir, PIN_NAME, o->pin_fd, &o->pin_stat) ||
        read_all_at(o->pin_fd, chars, sizeof chars, 0) ||
        decode_lower_hex(chars, o->pin_digest)) {
        zero_bytes(chars, sizeof chars);
        return -1;
    }
    zero_bytes(chars, sizeof chars);
    return named_regular(o->pin_dir, PIN_NAME, o->pin_fd,
                         &o->pin_stat);
}

static int stable_sources(const struct owner *o)
{
    if (named_directory("/data/saaios/var/efs-copy", o->source_dir,
                        &o->source_dir_stat) ||
        named_directory("/run/saaios-rfs-one-grant", o->pin_dir,
                        &o->pin_dir_stat) ||
        named_regular(o->source_dir, SOURCE_NAME, o->source,
                      &o->source_stat) ||
        directory_exact(o->pin_dir, 0700) || run_tmpfs(o->pin_dir))
        return 0;
    if (!o->pin_consumed)
        return named_regular(o->pin_dir, PIN_NAME, o->pin_fd,
                             &o->pin_stat) == 0;
    struct stat unexpected;
    if (named_regular(o->pin_dir, USED_PIN_NAME, o->pin_fd,
                      &o->pin_stat) ||
        fstatat(o->pin_dir, PIN_NAME, &unexpected,
                AT_SYMLINK_NOFOLLOW) == 0 || errno != ENOENT)
        return 0;
    return 1;
}

/* Once linked under the no-overwrite consumed name, no retry may use it. */
static int consume_pin(struct owner *o)
{
    struct stat used;
    if (!stable_sources(o) || o->pin_consumed ||
        fstatat(o->pin_dir, USED_PIN_NAME, &used,
                AT_SYMLINK_NOFOLLOW) == 0 || errno != ENOENT ||
        linkat(o->pin_dir, PIN_NAME, o->pin_dir, USED_PIN_NAME, 0) ||
        unlinkat(o->pin_dir, PIN_NAME, 0) ||
        regular_exact(o->pin_fd, 64, 0600, &o->pin_stat) ||
        named_regular(o->pin_dir, USED_PIN_NAME, o->pin_fd,
                      &o->pin_stat) || fsync(o->pin_dir) ||
        fstat(o->pin_dir, &o->pin_dir_stat)) return -1;
    o->pin_consumed = 1;
    return stable_sources(o) ? 0 : -1;
}

static int fresh_leaf(char leaf[48])
{
    static const char hex[] = "0123456789abcdef";
    uint8_t random_bytes[16];
    size_t done = 0;
    memset(leaf, 0, 48);
    memcpy(leaf, "full-rfs-", 9);
    while (done < sizeof random_bytes) {
        ssize_t got = getrandom(random_bytes + done,
                                sizeof random_bytes - done, 0);
        if (got < 0 && errno == EINTR) continue;
        if (got <= 0) return -1;
        done += (size_t)got;
    }
    for (size_t i = 0; i < sizeof random_bytes; ++i) {
        leaf[9 + 2*i] = hex[random_bytes[i] >> 4];
        leaf[10 + 2*i] = hex[random_bytes[i] & 15];
    }
    leaf[41] = 0;
    zero_bytes(random_bytes, sizeof random_bytes);
    return 0;
}

static int quarantine_location_stable(const struct owner *o)
{
    static const uint8_t marker[] = "QUARANTINE_NO_PROMOTION\n";
    struct stat held_parent, named_parent, held_dir, named_dir;
    uint8_t content[sizeof marker - 1];
    int valid = o->candidate >= 0 && o->marker_fd >= 0 &&
           o->quarantine_dir >= 0 &&
           directory_exact(o->quarantine_parent, 0700) == 0 &&
           fstat(o->quarantine_parent, &held_parent) == 0 &&
           same_file(&held_parent, &o->quarantine_parent_stat) &&
           lstat("/data/saaios/var/rfs-quarantine", &named_parent) == 0 &&
           same_file(&named_parent, &o->quarantine_parent_stat) &&
           directory_exact(o->quarantine_dir, 0700) == 0 &&
           fstat(o->quarantine_dir, &held_dir) == 0 &&
           same_file(&held_dir, &o->quarantine_dir_stat) &&
           fstatat(o->quarantine_parent, o->quarantine_leaf, &named_dir,
                   AT_SYMLINK_NOFOLLOW) == 0 &&
           same_file(&named_dir, &o->quarantine_dir_stat) &&
           named_regular(o->quarantine_dir, MARKER_NAME,
                         o->marker_fd, &o->marker_stat) == 0 &&
           read_all_at(o->marker_fd, content, sizeof content, 0) == 0 &&
           same_bytes(content, marker, sizeof content) &&
           named_regular(o->quarantine_dir, MARKER_NAME,
                         o->marker_fd, &o->marker_stat) == 0;
    zero_bytes(content, sizeof content);
    return valid ? 0 : -1;
}

static int candidate_stable(const struct owner *o)
{
    struct stat st;
    return quarantine_location_stable(o) == 0 &&
           fstat(o->candidate, &st) == 0 &&
           same_file(&st, &o->candidate_stat) &&
           named_regular(o->quarantine_dir, CANDIDATE_NAME,
                         o->candidate, &o->candidate_stat) == 0 ? 0 : -1;
}

static int prepare_quarantine(struct owner *o)
{
    static const char *const quarantine_path[] =
        {"data", "saaios", "var", "rfs-quarantine"};
    static const uint8_t marker[] = "QUARANTINE_NO_PROMOTION\n";
    uint8_t buffer[4096], digest[32];
    struct sha256 source_hash, candidate_hash;
    char leaf[48];
    o->quarantine_parent = walk_directory(quarantine_path, 4);
    if (o->quarantine_parent < 0 ||
        directory_exact(o->quarantine_parent, 0700) ||
        fstat(o->quarantine_parent, &o->quarantine_parent_stat)) return -1;
    for (unsigned attempt = 0; attempt < 16; ++attempt) {
        if (fresh_leaf(leaf)) return -1;
        if (mkdirat(o->quarantine_parent, leaf, 0700) == 0) break;
        if (errno != EEXIST || attempt == 15) return -1;
    }
    o->quarantine_dir = openat(o->quarantine_parent, leaf,
                               O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    if (o->quarantine_dir < 0 ||
        directory_exact(o->quarantine_dir, 0700) ||
        fstat(o->quarantine_dir, &o->quarantine_dir_stat)) return -1;
    memcpy(o->quarantine_leaf, leaf, sizeof leaf);
    o->marker_fd = openat(o->quarantine_dir, MARKER_NAME,
                          O_RDWR | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC,
                          0600);
    if (o->marker_fd < 0 || fstat(o->marker_fd, &o->marker_stat) ||
        !S_ISREG(o->marker_stat.st_mode) || o->marker_stat.st_uid != 0 ||
        o->marker_stat.st_nlink != 1 ||
        (o->marker_stat.st_mode & 07777) != 0600 ||
        write_all_at(o->marker_fd, marker, sizeof marker - 1, 0) ||
        fsync(o->marker_fd) ||
        fstat(o->marker_fd, &o->marker_stat) ||
        o->marker_stat.st_size != (off_t)(sizeof marker - 1)) goto failure;
    o->candidate = openat(o->quarantine_dir, CANDIDATE_NAME,
                          O_RDWR | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC,
                          0600);
    if (o->candidate < 0) return -1;
    sha_init(&source_hash);
    for (off_t off = 0; off < BASELINE_BYTES; off += sizeof buffer) {
        if (read_all_at(o->source, buffer, sizeof buffer, off) ||
            write_all_at(o->candidate, buffer, sizeof buffer, off))
            goto failure;
        sha_update(&source_hash, buffer, sizeof buffer);
    }
    sha_final(&source_hash, digest);
    if (!same_bytes(digest, o->pin_digest, sizeof digest) ||
        !stable_sources(o) || fsync(o->candidate)) goto failure;
    sha_init(&candidate_hash);
    for (off_t off = 0; off < BASELINE_BYTES; off += sizeof buffer) {
        if (read_all_at(o->candidate, buffer, sizeof buffer, off)) goto failure;
        sha_update(&candidate_hash, buffer, sizeof buffer);
    }
    sha_final(&candidate_hash, digest);
    if (!same_bytes(digest, o->pin_digest, sizeof digest) ||
        fstat(o->candidate, &o->candidate_stat) ||
        !S_ISREG(o->candidate_stat.st_mode) ||
        o->candidate_stat.st_uid != 0 || o->candidate_stat.st_nlink != 1 ||
        (o->candidate_stat.st_mode & 07777) != 0600 ||
        o->candidate_stat.st_size != BASELINE_BYTES ||
        fstat(o->quarantine_parent, &o->quarantine_parent_stat) ||
        fstat(o->quarantine_dir, &o->quarantine_dir_stat) ||
        candidate_stable(o) || fsync(o->quarantine_dir) ||
        fsync(o->quarantine_parent) || !stable_sources(o)) goto failure;
    zero_bytes(buffer, sizeof buffer);
    zero_bytes(digest, sizeof digest);
    return 0;
failure:
    zero_bytes(buffer, sizeof buffer);
    zero_bytes(digest, sizeof digest);
    zero_bytes(&source_hash, sizeof source_hash);
    zero_bytes(&candidate_hash, sizeof candidate_hash);
    return -1;
}

static int64_t monotonic_ms(void)
{
    struct timespec now;
    if (clock_gettime(CLOCK_MONOTONIC, &now)) return -1;
    return (int64_t)now.tv_sec * 1000 + now.tv_nsec / 1000000;
}

static enum cp_state cp_state(void)
{
    char state[32] = {0};
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    if (!f) return CP_UNKNOWN;
    int got = fscanf(f, "%31s", state);
    if (fclose(f) != 0 || got != 1) return CP_UNKNOWN;
    if (!strcmp(state, "OFFLINE")) return CP_OFFLINE;
    if (!strcmp(state, "BOOTING")) return CP_BOOTING;
    if (!strcmp(state, "ONLINE")) return CP_ONLINE;
    if (!strcmp(state, "CRASH_EXIT")) return CP_CRASH;
    return CP_UNKNOWN;
}

static int verify_device(int fd, const char *sysdev)
{
    FILE *f = fopen(sysdev, "r");
    unsigned expected_major = 0, expected_minor = 0;
    struct stat st;
    if (!f) return -1;
    int got = fscanf(f, "%u:%u", &expected_major, &expected_minor);
    if (fclose(f) != 0 || got != 2 || fstat(fd, &st) ||
        !S_ISCHR(st.st_mode) || major(st.st_rdev) != expected_major ||
        minor(st.st_rdev) != expected_minor) return -1;
    int flags = fcntl(fd, F_GETFL);
    if (flags < 0 || (flags & O_ACCMODE) != O_RDWR) return -1;
    if (!(flags & O_NONBLOCK) && fcntl(fd, F_SETFL, flags | O_NONBLOCK))
        return -1;
    return 0;
}

static int opened_once(int fd)
{
    int opened = -1;
    return ioctl(fd, IOCTL_GET_OPENED_STATUS, &opened) == 0 &&
           opened == 1 ? 0 : -1;
}

static int endpoint_fault_free(int fd)
{
    struct pollfd p = {.fd = fd, .events = 0};
    int checked = poll(&p, 1, 0);
    return checked >= 0 && !(p.revents & (POLLERR | POLLHUP | POLLNVAL)) ?
           0 : -1;
}

static int ready_pipe(int fd)
{
    struct stat st;
    int flags = fcntl(fd, F_GETFL);
    return flags >= 0 && (flags & O_ACCMODE) == O_WRONLY &&
           fstat(fd, &st) == 0 && S_ISFIFO(st.st_mode) ? 0 : -1;
}

static int acquire_lock(void)
{
    static const char path[] = "/run/saaios-sit-status.lock";
    int fd = open(path, O_RDWR | O_CREAT | O_NOFOLLOW | O_CLOEXEC, 0600);
    struct stat held, named;
    if (fd < 0) return -1;
    if (fstat(fd, &held) || !S_ISREG(held.st_mode) ||
        held.st_uid != 0 || held.st_nlink != 1 ||
        (held.st_mode & 07777) != 0600 ||
        flock(fd, LOCK_EX | LOCK_NB) ||
        lstat(path, &named) || !same_file(&held, &named)) {
        close(fd);
        return -1;
    }
    return fd;
}

static int send_ready(int fd)
{
    static const char ready[] = "READY\n";
    /* This pipe is not a modem endpoint; handling a short write is safe. */
    size_t done = 0;
    while (done < sizeof ready - 1) {
        ssize_t n = write(fd, ready + done, sizeof ready - 1 - done);
        if (n < 0 && errno == EINTR) continue;
        if (n <= 0) return -1;
        done += (size_t)n;
    }
    return 0;
}

/* The CP's RFS sequence counter is shared with carrierconfig, so the NV
 * cmd7/cmd3/cmd6 frames keep their captured shape except bytes 2..3. */
static int same_except_seq(const uint8_t *frame, const uint8_t *proto,
                           size_t len)
{
    return len >= 4 && frame[0] == proto[0] && frame[1] == proto[1] &&
           memcmp(frame + 4, proto + 4, len - 4) == 0;
}

static enum action classify(const struct owner *o, const uint8_t *frame,
                            size_t len)
{
    if (!frame) return BAD_FRAME;
    switch (o->phase) {
    case WAIT_7:
        return len == sizeof request_7 &&
               same_except_seq(frame, request_7, len) ? STATUS_7 : BAD_FRAME;
    case WAIT_3:
        return len == sizeof request_3 &&
               same_except_seq(frame, request_3, len) ? NO_REPLY : BAD_FRAME;
    case WAIT_6:
        return len == sizeof request_6 &&
               same_except_seq(frame, request_6, len) ? GRANT_1 : BAD_FRAME;
    case WAIT_DATA:
        return o->grant_attempted > 0 &&
               o->grant_attempted <= RFS_GRANTS_MAX &&
               o->chunks_stored + 1 == o->grant_attempted &&
               o->expected_chunk > 0 && o->expected_chunk <= FIRST_CHUNK &&
               /* The observed final CP frame has two zero bytes after the
                * 318 granted bytes. Accept only that exact final shape. */
               ((len == 20u + o->expected_chunk &&
                 little32(frame + 4) == 12u + o->expected_chunk) ||
                (o->grant_attempted == RFS_GRANTS_MAX &&
                 o->chunks_stored == RFS_GRANTS_MAX - 1 &&
                 o->expected_chunk == 318 && len == 340 &&
                 little32(frame + 4) == 332 &&
                 frame[338] == 0 && frame[339] == 0)) &&
               little16(frame) == 2 &&
               little16(frame + 2) == (o->nv_seq ? o->nv_seq : 1) &&
               little32(frame + 8) == 0 &&
               little32(frame + 12) == 3 &&
               little32(frame + 16) == o->expected_chunk ?
               STORE_CHUNK : BAD_FRAME;
    case TERMINAL:
        return BAD_FRAME;
    }
    return BAD_FRAME;
}

/* Compare only against public protocol constants; retain no observed fields. */
static unsigned data_mismatch_mask(const struct owner *o,
                                   const uint8_t *frame, size_t len)
{
    unsigned mask = 0;
    if (len != 20u + o->expected_chunk) mask |= MISMATCH_LENGTH;
    if (!frame || len < 2 || little16(frame) != 2) mask |= MISMATCH_COMMAND;
    if (!frame || len < 4 ||
        little16(frame + 2) != (o->nv_seq ? o->nv_seq : 1))
        mask |= MISMATCH_SEQUENCE;
    if (!frame || len < 8 ||
        little32(frame + 4) != 12u + o->expected_chunk)
        mask |= MISMATCH_PAYLOAD_SIZE;
    if (!frame || len < 12 || little32(frame + 8) != 0)
        mask |= MISMATCH_STATUS;
    if (!frame || len < 16 || little32(frame + 12) != 3)
        mask |= MISMATCH_FILE;
    if (!frame || len < 20 || little32(frame + 16) != o->expected_chunk)
        mask |= MISMATCH_CHUNK_SIZE;
    return mask;
}

/* Lengths are bounded by the parser; inspect only bytes already in this frame. */
static void diagnose_final_frame_shape(struct owner *o, const uint8_t *frame,
                                       size_t len, size_t trailing)
{
    if (!frame || len < 8 || len > RFS_FRAME_MAX || trailing > RX_CAP)
        return;
    uint32_t outer_payload_len = little32(frame + 4);
    if (outer_payload_len > RFS_FRAME_MAX - 8u) return;
    o->final_parsed_len = (unsigned)len;
    o->final_outer_payload_len = outer_payload_len;
    o->final_trailing = (unsigned)trailing;
    o->final_padding_zero = PADDING_UNKNOWN;
    if (o->expected_chunk > FIRST_CHUNK ||
        len < 20u + o->expected_chunk) return;
    o->final_padding_zero = PADDING_YES;
    for (size_t i = 20u + o->expected_chunk; i < len; ++i) {
        if (frame[i]) {
            o->final_padding_zero = PADDING_NO;
            break;
        }
    }
}

static void put_little32(uint8_t *p, uint32_t value)
{
    p[0] = (uint8_t)value;
    p[1] = (uint8_t)(value >> 8);
    p[2] = (uint8_t)(value >> 16);
    p[3] = (uint8_t)(value >> 24);
}

static int reply_gate(const struct owner *o);
static int send_modem_once(const struct owner *o, const uint8_t *reply,
                           size_t len);

static uint32_t next_chunk_length(const struct owner *o)
{
    if (o->received_bytes >= RFS_TRANSFER_BYTES) return 0;
    uint32_t remain = RFS_TRANSFER_BYTES - o->received_bytes;
    return remain < FIRST_CHUNK ? remain : FIRST_CHUNK;
}

static int send_next_grant(struct owner *o)
{
    uint32_t length = next_chunk_length(o);
    enum failure_stage stage = o->grant_attempted == RFS_GRANTS_MAX - 1 ?
                               STAGE_FINAL_GRANT : STAGE_GRANT;
    if (!length || o->grant_attempted >= RFS_GRANTS_MAX ||
        o->chunks_stored != o->grant_attempted) {
        diagnose(o, stage, REASON_STATE);
        return -1;
    }
    if (reply_gate(o)) {
        diagnose(o, stage, REASON_GATE);
        return -1;
    }
    uint8_t grant[20] = {2,0,1,0, 12,0,0,0, 3,0,0,0};
    if (o->nv_seq) {
        grant[2] = (uint8_t)o->nv_seq;
        grant[3] = (uint8_t)(o->nv_seq >> 8);
    }
    put_little32(grant + 12, o->received_bytes);
    put_little32(grant + 16, length);
    /* Attempt is consumed even if the endpoint reports a short write. */
    o->grant_attempted++;
    o->expected_chunk = length;
    int rc = send_modem_once(o, grant, sizeof grant);
    zero_bytes(grant, sizeof grant);
    if (rc) {
        diagnose(o, stage, REASON_SEND);
        return -1;
    }
    o->phase = WAIT_DATA;
    return 0;
}

static int valid_frame_length(const uint8_t *rx, size_t used)
{
    if (used < 8) return 0;
    uint32_t payload = little32(rx + 4);
    if (payload < 4 || payload > RFS_FRAME_MAX - 8u) return -1;
    size_t total = 8u + payload;
    return used < total ? 0 : (int)total;
}

static int reply_gate(const struct owner *o)
{
    int64_t now = monotonic_ms();
    if (stop_requested || now < 0 || now >= o->deadline_ms ||
        now >= o->total_deadline_ms) return -1;
#ifdef RFS_HOST_TEST
    if (host_gate_override) return host_gate_override > 0 ? 0 : -1;
#endif
    return cp_state() == CP_ONLINE && opened_once(o->ipc) == 0 &&
           endpoint_fault_free(o->ipc) == 0 &&
           opened_once(o->rfs) == 0 && original_efs_unmounted() == 0 &&
           stable_sources(o) && candidate_stable(o) == 0 ? 0 : -1;
}

/* An unexpected result may already have reached the CP. Never retry it. */
static int send_modem_once(const struct owner *o, const uint8_t *reply,
                           size_t len)
{
    sigset_t blocked, old;
    sigemptyset(&blocked);
    sigaddset(&blocked, SIGTERM);
    sigaddset(&blocked, SIGINT);
    if (sigprocmask(SIG_BLOCK, &blocked, &old)) return -1;
    int64_t now = monotonic_ms();
    ssize_t written = -1;
    if (!stop_requested && now >= 0 && now < o->deadline_ms &&
        now < o->total_deadline_ms) {
#ifdef RFS_HOST_TEST
        if (host_write_override) {
            ++host_write_calls;
            written = host_write_override(o->rfs, reply, len);
        } else
#endif
        written = write(o->rfs, reply, len);
    }
    int restore = sigprocmask(SIG_SETMASK, &old, NULL);
    return restore == 0 && written == (ssize_t)len ? 0 : -1;
}

struct chunk_io {
    int (*sync)(int);
    int (*read)(int, void *, size_t, off_t);
};

/* A grant is never advanced until this exact chunk is durable and read back. */
static int apply_chunk(int candidate, int source, uint32_t offset,
                       const uint8_t *bytes, uint32_t length,
                       const struct chunk_io *io,
                       enum failure_reason *failure_reason)
{
    uint8_t check[4096];
    enum failure_reason reason = REASON_STATE;
    if (!length || length > FIRST_CHUNK ||
        offset > RFS_TRANSFER_BYTES - length) goto failure;
    reason = REASON_IO;
    if (write_all_at(candidate, bytes, length, offset) ||
        io->sync(candidate)) goto failure;
    reason = REASON_READBACK;
    if (io->read(candidate, check, length, offset) ||
        !same_bytes(check, bytes, length)) goto failure;
    for (off_t off = (off_t)offset + length; off < BASELINE_BYTES;) {
        size_t amount = (size_t)(BASELINE_BYTES - off);
        if (amount > sizeof check / 2) amount = sizeof check / 2;
        if (io->read(source, check, amount, off) ||
            io->read(candidate, check + sizeof check / 2, amount, off) ||
            !same_bytes(check, check + sizeof check / 2, amount))
            goto failure;
        off += (off_t)amount;
    }
    zero_bytes(check, sizeof check);
    return 0;
failure:
    if (failure_reason) *failure_reason = reason;
    zero_bytes(check, sizeof check);
    return -1;
}

static int store_chunk(struct owner *o, const uint8_t *bytes)
{
    static const struct chunk_io io = {fsync, read_all_at};
    enum failure_reason reason = REASON_NONE;
    if (reply_gate(o)) {
        diagnose(o, STAGE_STORE_CHUNK, REASON_GATE);
        return -1;
    }
    if (o->received_bytes > RFS_TRANSFER_BYTES - o->expected_chunk) {
        diagnose(o, STAGE_STORE_CHUNK, REASON_STATE);
        return -1;
    }
    if (apply_chunk(o->candidate, o->source, o->received_bytes, bytes,
                    o->expected_chunk, &io, &reason)) {
        diagnose(o, STAGE_STORE_CHUNK, reason);
        return -1;
    }
    struct stat current;
    if (fstat(o->candidate, &current)) {
        diagnose(o, STAGE_STORE_CHUNK, REASON_IO);
        return -1;
    }
    if (current.st_dev != o->candidate_stat.st_dev ||
        current.st_ino != o->candidate_stat.st_ino ||
        current.st_uid != 0 || current.st_nlink != 1 ||
        (current.st_mode & 07777) != 0600 ||
        current.st_size != BASELINE_BYTES ||
        quarantine_location_stable(o) ||
        named_regular(o->quarantine_dir, CANDIDATE_NAME,
                      o->candidate, &current) ||
        !stable_sources(o) || original_efs_unmounted() || stop_requested) {
        diagnose(o, STAGE_STORE_CHUNK, REASON_GATE);
        return -1;
    }
    o->candidate_stat = current;
    sha_update(&o->received_hash, bytes, o->expected_chunk);
    o->received_bytes += o->expected_chunk;
    o->chunks_stored++;
    return 0;
}

/* Verify the source pin, all received bytes, and the unchanged baseline tail. */
static int verify_full_candidate(struct owner *o, uint8_t digest[32])
{
    uint8_t source[4096], candidate[4096], source_digest[32];
    uint8_t prefix_digest[32], received_digest[32];
    struct sha256 source_hash, prefix_hash, candidate_hash, received_hash;
    int result = -1;
    if (stop_requested ||
        (!host_fixture_mode() &&
         (original_efs_unmounted() || !stable_sources(o) ||
          candidate_stable(o))) ||
        o->received_bytes != RFS_TRANSFER_BYTES ||
        o->chunks_stored != RFS_GRANTS_MAX ||
        o->grant_attempted != RFS_GRANTS_MAX) return -1;
    sha_init(&source_hash);
    sha_init(&prefix_hash);
    sha_init(&candidate_hash);
    received_hash = o->received_hash;
    sha_final(&received_hash, received_digest);
    for (off_t off = 0; off < BASELINE_BYTES; off += sizeof source) {
        size_t amount = (size_t)(BASELINE_BYTES - off);
        if (amount > sizeof source) amount = sizeof source;
        if (read_all_at(o->source, source, amount, off) ||
            read_all_at(o->candidate, candidate, amount, off)) goto done;
        sha_update(&source_hash, source, amount);
        sha_update(&candidate_hash, candidate, amount);
        size_t prefix = off >= RFS_TRANSFER_BYTES ? 0 :
                        (size_t)(RFS_TRANSFER_BYTES - off);
        if (prefix > amount) prefix = amount;
        if (prefix) sha_update(&prefix_hash, candidate, prefix);
        if (prefix < amount &&
            !same_bytes(source + prefix, candidate + prefix,
                        amount - prefix)) goto done;
    }
    sha_final(&source_hash, source_digest);
    sha_final(&prefix_hash, prefix_digest);
    sha_final(&candidate_hash, digest);
    if (same_bytes(source_digest, o->pin_digest, 32) &&
        same_bytes(prefix_digest, received_digest, 32) &&
        !stop_requested &&
        (host_fixture_mode() ||
         (!original_efs_unmounted() && stable_sources(o) &&
          candidate_stable(o) == 0)))
        result = 0;
done:
    zero_bytes(source, sizeof source);
    zero_bytes(candidate, sizeof candidate);
    zero_bytes(source_digest, sizeof source_digest);
    zero_bytes(prefix_digest, sizeof prefix_digest);
    zero_bytes(received_digest, sizeof received_digest);
    zero_bytes(&source_hash, sizeof source_hash);
    zero_bytes(&prefix_hash, sizeof prefix_hash);
    zero_bytes(&candidate_hash, sizeof candidate_hash);
    zero_bytes(&received_hash, sizeof received_hash);
    if (result) zero_bytes(digest, 32);
    return result;
}

static int sidecar_stable(const struct owner *o)
{
    struct stat held;
    uint8_t digest[32];
    uid_t owner_uid = host_fixture_mode() ? geteuid() : 0;
    int valid = o->sidecar_fd >= 0 &&
        (fcntl(o->sidecar_fd, F_GETFL) & O_ACCMODE) == O_RDONLY &&
        fstat(o->sidecar_fd, &held) == 0 &&
        same_file(&held, &o->sidecar_stat) &&
        S_ISREG(held.st_mode) && held.st_uid == owner_uid &&
        held.st_nlink == 1 && (held.st_mode & 07777) == 0600 &&
        held.st_size == 32 &&
        named_regular(o->quarantine_dir, SIDECAR_NAME, o->sidecar_fd,
                      &o->sidecar_stat) == 0 &&
        read_all_at(o->sidecar_fd, digest, sizeof digest, 0) == 0 &&
        same_bytes(digest, o->candidate_digest, sizeof digest) &&
        (host_fixture_mode() || quarantine_location_stable(o) == 0);
    zero_bytes(digest, sizeof digest);
    return valid ? 0 : -1;
}

static int close_sidecar_once(struct owner *o)
{
    int fd = o->sidecar_fd;
    o->sidecar_fd = -1;
    int result = close(fd);
#ifdef RFS_HOST_TEST
    if (host_sidecar_close_error) return -1;
#endif
    return result;
}

static int sync_sidecar_once(int fd)
{
#ifdef RFS_HOST_TEST
    if (host_sidecar_sync_error) {
        errno = EIO;
        return -1;
    }
#endif
    return fsync(fd);
}

/* The new sidecar changes directory timestamps; only its identity may carry. */
static int refresh_quarantine_dir_stat(struct owner *o)
{
    struct stat parent, held, named;
    uid_t owner_uid = host_fixture_mode() ? geteuid() : 0;
    if (fstat(o->quarantine_parent, &parent) ||
        !same_file(&parent, &o->quarantine_parent_stat) ||
        fstat(o->quarantine_dir, &held) ||
        !same_directory_identity(&held, &o->quarantine_dir_stat) ||
        held.st_uid != owner_uid || (held.st_mode & 07777) != 0700 ||
        fstatat(o->quarantine_parent, o->quarantine_leaf, &named,
                AT_SYMLINK_NOFOLLOW) ||
        !same_file(&held, &named)) return -1;
    if (!host_fixture_mode() &&
        (directory_exact(o->quarantine_parent, 0700) ||
         named_directory("/data/saaios/var/rfs-quarantine",
                         o->quarantine_parent, &o->quarantine_parent_stat) ||
         named_regular(o->quarantine_dir, MARKER_NAME, o->marker_fd,
                       &o->marker_stat) ||
         named_regular(o->quarantine_dir, CANDIDATE_NAME, o->candidate,
                       &o->candidate_stat))) return -1;
    o->quarantine_dir_stat = held;
    return host_fixture_mode() || quarantine_location_stable(o) == 0 ? 0 : -1;
}

/* The sidecar is a quarantine integrity record, never a promotion signal. */
static int finish_candidate(struct owner *o)
{
    uint8_t digest[32], again[32];
    int rc = -1;
    uid_t owner_uid = host_fixture_mode() ? geteuid() : 0;
    if (reply_gate(o) || fsync(o->candidate) ||
        verify_full_candidate(o, digest)) goto done;
    o->sidecar_fd = openat(o->quarantine_dir, SIDECAR_NAME,
                           O_RDWR | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC,
                           0600);
    if (o->sidecar_fd < 0 ||
        write_all_at(o->sidecar_fd, digest, sizeof digest, 0) ||
        sync_sidecar_once(o->sidecar_fd) ||
        fstat(o->sidecar_fd, &o->sidecar_stat) ||
        !S_ISREG(o->sidecar_stat.st_mode) ||
        o->sidecar_stat.st_uid != owner_uid ||
        o->sidecar_stat.st_nlink != 1 ||
        (o->sidecar_stat.st_mode & 07777) != 0600 ||
        o->sidecar_stat.st_size != 32 ||
        fsync(o->quarantine_dir) || fsync(o->quarantine_parent) ||
        refresh_quarantine_dir_stat(o))
        goto done;
    /* A close failure is ambiguous: never retry and never ACK. */
    if (close_sidecar_once(o)) goto done;
    o->sidecar_fd = openat(o->quarantine_dir, SIDECAR_NAME,
                            O_RDONLY | O_NOFOLLOW | O_CLOEXEC);
    if (o->sidecar_fd < 0 ||
        named_regular(o->quarantine_dir, SIDECAR_NAME, o->sidecar_fd,
                      &o->sidecar_stat)) goto done;
    memcpy(o->candidate_digest, digest, sizeof digest);
    if (sidecar_stable(o) || verify_full_candidate(o, again) ||
        !same_bytes(digest, again, sizeof digest) ||
        sidecar_stable(o) || reply_gate(o))
        goto done;
    rc = 0;
done:
    zero_bytes(digest, sizeof digest);
    zero_bytes(again, sizeof again);
    return rc;
}

static void log_terminal(FILE *out, const struct owner *o, const char *reason)
{
    char final_shape[160] = {0};
    if (o->final_parsed_len)
        (void)snprintf(final_shape, sizeof final_shape,
                       " final_parsed_len=%u final_outer_payload_len=%u "
                       "final_trailing=%u final_padding_zero=%s",
                       o->final_parsed_len, o->final_outer_payload_len,
                       o->final_trailing,
                       o->final_padding_zero == PADDING_YES ? "1" :
                       o->final_padding_zero == PADDING_NO ? "0" : "unknown");
    fprintf(out, "rfs_full_quarantine=%s grants_attempted=%d chunks_stored=%d "
            "bytes_stored=%u final_ack_attempted=%d final_ack_sent=%d "
            "failure_stage=%s failure_reason=%s frame_mismatch_mask=0x%02x%s\n",
            reason, o->grant_attempted, o->chunks_stored,
            o->received_bytes, o->final_ack_attempted, o->final_ack_sent,
            failure_stage_name[o->failure_stage],
            failure_reason_name[o->failure_reason], o->frame_mismatch_mask,
            final_shape);
}

static void terminal(struct owner *o, const char *reason)
{
    if (o->phase == TERMINAL) return;
    o->phase = TERMINAL;
    o->used = 0;
    zero_bytes(o->rx, sizeof o->rx);
    log_terminal(stdout, o, reason);
}

static void diagnose_waiting(struct owner *o)
{
    if (o->phase == WAIT_DATA && frame_stage(o) == STAGE_FINAL_FRAME)
        diagnose(o, STAGE_FINAL_FRAME,
                 o->used ? REASON_PARTIAL_READ : REASON_NO_READ);
    else
        diagnose(o, STAGE_DEADLINE, REASON_TIMEOUT);
}

/* Passive early-boot RFS trace. Logs ONLY numeric protocol header fields
 * (command, numeric file handle, offset/size counters) and the byte length;
 * never any payload content (protected-NV bytes are secret). This RFS variant
 * addresses files by a numeric handle, not an ASCII path, so no path string is
 * present to leak. Bounded so a misbehaving endpoint cannot flood the log. */
static unsigned rfs_trace_count;
static void rfs_trace(const char *note, enum phase phase,
                      const uint8_t *frame, size_t len, int64_t now)
{
    if (!frame || len < 4 || rfs_trace_count >= RFS_TRACE_MAX) return;
    rfs_trace_count++;
    uint32_t cmd = little32(frame);
    uint32_t f1 = len >= 8 ? little32(frame + 4) : 0;
    uint32_t handle = len >= 12 ? little32(frame + 8) : 0;
    uint32_t w3 = len >= 16 ? little32(frame + 12) : 0;
    uint32_t w4 = len >= 20 ? little32(frame + 16) : 0;
    /* Bytes at offset 20+ are request-header fields only on short control
     * frames (<=24 bytes); on data-carrying frames offset 20 is the start of
     * the quarantined NV payload, which must never be logged. */
    uint32_t w5 = len == 24 ? little32(frame + 20) : 0;
    printf("RFS_TRACE note=%s t=%lld phase=%d len=%zu cmd=%u paylen=%u "
           "handle=%u off=%u w4=%u w5=%u\n",
           note, (long long)now, (int)phase, len,
           cmd, f1, handle, w3, w4, w5);
    fflush(stdout);
}

/* Copy the immutable nv_normal baseline into a new quarantine file, then
 * let the CP's handle-1 write overlay the front. The tail past the CP's
 * transfer size stays the baseline. Never truncates, never writes the
 * baseline or original EFS, never logs payload bytes. */
static int seed_normal_candidate(struct owner *o, int *out_fd)
{
    uint8_t buffer[4096];
    struct stat st;
    int src = openat(o->source_dir, "nv_normal.bin",
                     O_RDONLY | O_NOFOLLOW | O_CLOEXEC);
    int fd = -1;
    int rc = -1;
    if (src < 0 || regular_exact(src, BASELINE_BYTES, 0600, &st) ||
        o->quarantine_dir < 0)
        goto done;
    fd = openat(o->quarantine_dir, "normal-candidate.bin",
                O_RDWR | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC, 0600);
    if (fd < 0) goto done;
    for (off_t off = 0; off < BASELINE_BYTES; off += (off_t)sizeof buffer) {
        if (read_all_at(src, buffer, sizeof buffer, off) ||
            write_all_at(fd, buffer, sizeof buffer, off))
            goto done;
    }
    if (fsync(fd)) goto done;
    *out_fd = fd;
    fd = -1;
    rc = 0;
done:
    if (fd >= 0) {
        close(fd);
        if (o->quarantine_dir >= 0)
            (void)unlinkat(o->quarantine_dir, "normal-candidate.bin", 0);
    }
    if (src >= 0) close(src);
    zero_bytes(buffer, sizeof buffer);
    return rc;
}

/* Read-only carrierconfig file service, recovered from CP2A rfsd
 * RfsService::File (not invented):
 *   cmd 4 OPEN: payload u32 id, skip 4, path at +8 must contain
 *     "carrierconfig/". Reply is 20 bytes: cmd 3, seq, len 8, status 0,
 *     file id, st_size (rfsd OpenV2 success path).
 *   cmd 6 op 1: read. Reply is cmd 1, seq, len=n+12, id, offset, n, bytes
 *     (rfsd read sender, chunk cap 2012). op 2 (write) gets status 3,
 *     the "Not allowed" status constant.
 *   cmd 5 CLOSE: 16-byte status 0.
 *   EOF / zero read: 16-byte status 8 (rfsd movi #8).
 * Files open only under CC_ROOT, O_RDONLY|O_NOFOLLOW. No NV, no EFS.
 * A frame whose id is not one of ours is left for the NV handlers. */
#define CC_ROOT "/data/saaios/var/carrierconfig"
#define CC_SLOTS 8
#define CC_CHUNK 2012u
static const char *cc_root_path = CC_ROOT;
static int cc_root_fd = -1;
static struct {
    uint32_t id;
    int fd;
    int live;
    uint32_t rd_off, rd_left;
    uint16_t rd_seq;
} cc_slot[CC_SLOTS];

static int cc_out(int fd, const uint8_t *p, size_t n)
{
    size_t done = 0;
    while (done < n) {
        ssize_t w = write(fd, p + done, n - done);
        if (w < 0 && errno == EINTR) continue;
        if (w <= 0) return -1;
        done += (size_t)w;
    }
    return 0;
}

static void cc_status(int fd, uint16_t seq, uint32_t id, uint32_t status)
{
    uint8_t f[16] = {3, 0, 0, 0, 8, 0, 0, 0};
    f[2] = (uint8_t)seq;
    f[3] = (uint8_t)(seq >> 8);
    put_little32(f + 8, status);
    put_little32(f + 12, id);
    (void)cc_out(fd, f, sizeof f);
}

static int cc_relative(const char *path, char *out, size_t cap)
{
    const char *mark = strstr(path, "carrierconfig/");
    const char *rel;
    size_t n, i;
    if (!mark) return -1;
    rel = mark + strlen("carrierconfig/");
    n = strlen(rel);
    if (!n || n >= cap || strstr(rel, "..")) return -1;
    for (i = 0; i < n; i++) {
        unsigned char c = (unsigned char)rel[i];
        int ok = (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') ||
                 (c >= '0' && c <= '9') || c == '/' || c == '.' ||
                 c == '_' || c == '-';
        if (!ok) return -1;
    }
    memcpy(out, rel, n + 1);
    return 0;
}

static int cc_find(uint32_t id)
{
    int i;
    for (i = 0; i < CC_SLOTS; i++)
        if (cc_slot[i].live && cc_slot[i].id == id) return i;
    return -1;
}

/* 1 = this frame was a carrierconfig file op (answered or refused). */
static int cc_serve(int rfs, const uint8_t *frame, size_t size)
{
    uint16_t cmd, seq;
    uint32_t id;
    char rel[192];
    int slot;
    if (size < 8) return 0;
    cmd = little16(frame);
    seq = little16(frame + 2);
    if (cmd == 4) {
        const char *path;
        struct stat st;
        int fd, i;
        if (size < 18 || frame[16] != '/') return 0;
        path = (const char *)(frame + 16);
        if (strnlen(path, size - 16) >= size - 16) return 0;
        id = little32(frame + 8);
        if (cc_relative(path, rel, sizeof rel)) {
            char prefix[49];
            size_t pi, pn = strnlen(path, 48);
            int run = 0, safe = 1;
            for (pi = 0; pi < pn; pi++) {
                unsigned char c = (unsigned char)path[pi];
                int ok = (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') ||
                         (c >= '0' && c <= '9') || c == '/' || c == '.' ||
                         c == '_' || c == '-';
                if (!ok) { safe = 0; break; }
                prefix[pi] = (char)c;
                if (c >= '0' && c <= '9') {
                    if (++run >= 15) safe = 0;
                } else run = 0;
            }
            prefix[safe ? pi : 0] = '\0';
            if (safe && prefix[0])
                printf("cc_open id=%u reject=path prefix=%s\n", id, prefix);
            else
                printf("cc_open id=%u reject=path\n", id);
            cc_status(rfs, seq, id, 3);
            return 1;
        }
        if (cc_root_fd < 0)
            cc_root_fd = open(cc_root_path, O_RDONLY | O_DIRECTORY | O_CLOEXEC);
        if (cc_root_fd < 0) {
            printf("cc_open id=%u reject=root\n", id);
            cc_status(rfs, seq, id, 3);
            return 1;
        }
        fd = openat(cc_root_fd, rel, O_RDONLY | O_NOFOLLOW | O_CLOEXEC);
        if (fd < 0 || fstat(fd, &st) || !S_ISREG(st.st_mode) ||
            st.st_size < 0 || st.st_size > 0x7fffffff) {
            if (fd >= 0) close(fd);
            printf("cc_open id=%u rel=%s reject=open\n", id, rel);
            cc_status(rfs, seq, id, 3);
            return 1;
        }
        slot = cc_find(id);
        if (slot < 0) {
            for (i = 0; i < CC_SLOTS; i++)
                if (!cc_slot[i].live) { slot = i; break; }
        }
        if (slot < 0) {
            close(fd);
            printf("cc_open id=%u rel=%s reject=slots\n", id, rel);
            cc_status(rfs, seq, id, 6);
            return 1;
        }
        if (cc_slot[slot].live && cc_slot[slot].fd >= 0)
            close(cc_slot[slot].fd);
        cc_slot[slot].live = 1;
        cc_slot[slot].id = id;
        cc_slot[slot].fd = fd;
        {
            uint8_t ok[20];
            memset(ok, 0, sizeof ok);
            ok[0] = 3;
            ok[2] = (uint8_t)seq;
            ok[3] = (uint8_t)(seq >> 8);
            put_little32(ok + 4, 8);
            put_little32(ok + 12, id);
            put_little32(ok + 16, (uint32_t)st.st_size);
            (void)cc_out(rfs, ok, sizeof ok);
        }
        printf("cc_open id=%u rel=%s size=%lld\n", id, rel,
               (long long)st.st_size);
        return 1;
    }
    /* rfsd File handler: cmd 3 with the open file id at frame+12 and a
     * zero at frame+8 sets an internal flag and sends nothing. A reply
     * here, or letting the NV parser call it malformed, stops the read. */
    if (cmd == 3 && size >= 16) {
        id = little32(frame + 12);
        slot = cc_find(id);
        if (slot < 0) return 0;
        /* The cmd 3 after CLOSE still names the file id. rfsd sends
         * nothing; this is what releases the id. */
        if (cc_slot[slot].live == 2) cc_slot[slot].live = 0;
        printf("cc_cmd3 id=%u\n", id);
        return 1;
    }
    /* After a finished read, rfsd's cmd 1 (handler 0x9c80) sees remaining
     * length 0 and replies with the 16-byte status 0 at rodata 0x5110. */
    if (cmd == 1 && size >= 16) {
        id = little32(frame + 12);
        slot = cc_find(id);
        if (slot < 0 || cc_slot[slot].live != 1) return 0;
        if (cc_slot[slot].rd_left > 0) {
            uint32_t off = cc_slot[slot].rd_off;
            uint32_t want = cc_slot[slot].rd_left;
            uint32_t chunk = want > CC_CHUNK ? CC_CHUNK : want;
            uint8_t buf[20 + CC_CHUNK];
            ssize_t n = pread(cc_slot[slot].fd, buf + 20, chunk, (off_t)off);
            if (n <= 0) {
                cc_slot[slot].rd_left = 0;
                cc_status(rfs, cc_slot[slot].rd_seq, id, n < 0 ? 3 : 8);
                return 1;
            }
            memset(buf, 0, 20);
            buf[0] = 1;
            buf[2] = (uint8_t)cc_slot[slot].rd_seq;
            buf[3] = (uint8_t)(cc_slot[slot].rd_seq >> 8);
            put_little32(buf + 4, (uint32_t)n + 12u);
            put_little32(buf + 8, id);
            put_little32(buf + 12, off);
            put_little32(buf + 16, (uint32_t)n);
            (void)cc_out(rfs, buf, 20u + (size_t)n);
            cc_slot[slot].rd_off = off + (uint32_t)n;
            cc_slot[slot].rd_left = want - (uint32_t)n;
            printf("cc_read id=%u off=%u n=%d\n", id, off, (int)n);
            return 1;
        }
        cc_status(rfs, cc_slot[slot].rd_seq, id, 0);
        printf("cc_cmd1 id=%u\n", id);
        return 1;
    }
    if (cmd != 5 && cmd != 6) return 0;
    if (size < 12) return 0;
    id = little32(frame + 8);
    slot = cc_find(id);
    if (slot < 0 || (cmd == 6 && cc_slot[slot].live != 1)) return 0;
    if (cmd == 5) {
        if (cc_slot[slot].live != 1) return 0;
        close(cc_slot[slot].fd);
        cc_slot[slot].live = 2;
        cc_slot[slot].fd = -1;
        cc_status(rfs, seq, id, 0);
        printf("cc_close id=%u\n", id);
        return 1;
    }
    if (size < 24) {
        cc_status(rfs, seq, id, 6);
        return 1;
    }
    {
        uint32_t off = little32(frame + 12);
        uint32_t len = little32(frame + 16);
        uint32_t op = little32(frame + 20);
        uint8_t buf[20 + CC_CHUNK];
        ssize_t n;
        if (op == 2) {
            printf("cc_io id=%u reject=write\n", id);
            cc_status(rfs, seq, id, 3);
            return 1;
        }
        if (op != 1) {
            cc_status(rfs, seq, id, 6);
            return 1;
        }
        {
            uint32_t want = len;
            uint32_t chunk = want > CC_CHUNK ? CC_CHUNK : want;
            n = pread(cc_slot[slot].fd, buf + 20, chunk, (off_t)off);
            if (n < 0) {
                cc_status(rfs, seq, id, 3);
                return 1;
            }
            if (n == 0) {
                cc_slot[slot].rd_left = 0;
                cc_status(rfs, seq, id, 8);
                printf("cc_read id=%u off=%u eof\n", id, off);
                return 1;
            }
            memset(buf, 0, 20);
            buf[0] = 1;
            buf[2] = (uint8_t)seq;
            buf[3] = (uint8_t)(seq >> 8);
            put_little32(buf + 4, (uint32_t)n + 12u);
            put_little32(buf + 8, id);
            put_little32(buf + 12, off);
            put_little32(buf + 16, (uint32_t)n);
            (void)cc_out(rfs, buf, 20u + (size_t)n);
            cc_slot[slot].rd_seq = seq;
            cc_slot[slot].rd_off = off + (uint32_t)n;
            cc_slot[slot].rd_left =
                ((uint32_t)n < chunk || (uint32_t)n >= want) ? 0 :
                want - (uint32_t)n;
            printf("cc_read id=%u off=%u n=%d\n", id, off, (int)n);
            return 1;
        }
    }
}

/* Post-completion RFS observer. After the protected-NV write-out quarantine is
 * done (TERMINAL + final_ack_sent) the main loop normally stops reading RFS.
 * This drain keeps reading so any RFS request the CP issues during the later
 * RadioPower-ON / MM registration-gate window is captured. It LOGS headers
 * only and, to keep the CP's handshake alive so subsequent requests keep
 * flowing, replies to an exact unprotect request_7 with the known status_7.
 * Handle 1 (nv_normal) is answered with the same recovered grant layout as
 * handle 3, but only after a full 524288-byte quarantine copy of the
 * immutable nv_normal baseline exists. It invents no reply bytes. */
static void post_terminal_rfs_drain(struct owner *o, const uint8_t *bytes,
                                    size_t got, int64_t now)
{
    static uint8_t pt_rx[RX_CAP];
    static size_t pt_used;
    static int normal_fd = -1;
    static uint32_t normal_total, normal_received;
    static unsigned normal_grants;
    static uint16_t normal_seq;
    static int normal_done;
    static int normal_seed_logged;
    if (!bytes || got > sizeof pt_rx - pt_used) {
        pt_used = 0; /* resynchronize rather than retain ambiguous bytes */
        return;
    }
    memcpy(pt_rx + pt_used, bytes, got);
    pt_used += got;
    while (pt_used) {
        int size = valid_frame_length(pt_rx, pt_used);
        if (size <= 0) break;
        rfs_trace("post_term", TERMINAL, pt_rx, (size_t)size, now);
        if (cc_serve(o->rfs, pt_rx, (size_t)size)) {
            memmove(pt_rx, pt_rx + size, pt_used - (size_t)size);
            pt_used -= (size_t)size;
            continue;
        }
        if ((size_t)size == sizeof request_7 &&
            same_bytes(pt_rx, request_7, sizeof request_7)) {
            size_t done = 0;
            while (done < sizeof status_7) {
                ssize_t n = write(o->rfs, status_7 + done,
                                  sizeof status_7 - done);
                if (n < 0 && errno == EINTR) continue;
                if (n <= 0) break;
                done += (size_t)n;
            }
        }
        if (!normal_done && (size_t)size >= 20) {
            uint16_t cmd = little16(pt_rx);
            if (cmd == 7 && (size_t)size == 12 && little32(pt_rx + 8) == 1) {
                /* handle-1 open: ack with the proven status layout, file id 1 */
                static const uint8_t st1[16] =
                    {3,0,0,0, 8,0,0,0, 0,0,0,0, 1,0,0,0};
                size_t done = 0;
                while (done < sizeof st1) {
                    ssize_t n = write(o->rfs, st1 + done, sizeof st1 - done);
                    if (n < 0 && errno == EINTR) continue;
                    if (n <= 0) break;
                    done += (size_t)n;
                }
            } else if (cmd == 6 && (size_t)size == 24 &&
                       little32(pt_rx + 8) == 1) {
                /* handle-1 grant-request: w4 = total normal-NV size; echo the
                 * CP's sequence (w0 high 16) in our grant so it is accepted. */
                uint32_t total = little32(pt_rx + 16);
                normal_seq = little16(pt_rx + 2);
                if (normal_fd < 0 && total > 0 && total <= BASELINE_BYTES &&
                    o->quarantine_dir >= 0 && o->source_dir >= 0 &&
                    seed_normal_candidate(o, &normal_fd) == 0) {
                    normal_total = total;
                    normal_received = 0;
                    printf("NORMAL_SEED baseline=%d total=%u seq=%u\n",
                           BASELINE_BYTES, total, normal_seq);
                    fflush(stdout);
                } else if (normal_fd < 0 && !normal_seed_logged) {
                    normal_seed_logged = 1;
                    printf("NORMAL_SEED failed total=%u\n", total);
                    fflush(stdout);
                }
                if (normal_fd >= 0 && normal_received < normal_total) {
                    uint32_t remain = normal_total - normal_received;
                    uint32_t len = remain < FIRST_CHUNK ? remain : FIRST_CHUNK;
                    uint8_t grant[20] = {2,0,0,0, 12,0,0,0, 1,0,0,0};
                    put_little32(grant, 2u | ((uint32_t)normal_seq << 16));
                    put_little32(grant + 12, normal_received);
                    put_little32(grant + 16, len);
                    size_t done = 0;
                    while (done < sizeof grant) {
                        ssize_t n = write(o->rfs, grant + done,
                                          sizeof grant - done);
                        if (n < 0 && errno == EINTR) continue;
                        if (n <= 0) break;
                        done += (size_t)n;
                    }
                    normal_grants++;
                }
            } else if (cmd == 2 && normal_fd >= 0 &&
                       little32(pt_rx + 12) == 1) {
                /* handle-1 CP response: data chunk (len>0) or status (len==0) */
                uint32_t clen = little32(pt_rx + 16);
                if (clen == 0) {
                    printf("NORMAL_CAPTURE status=%u seq=%u received=%u\n",
                           little32(pt_rx + 8), little16(pt_rx + 2),
                           normal_received);
                    fflush(stdout);
                }
                if (clen > 0 && clen <= FIRST_CHUNK &&
                    (size_t)size >= 20u + clen &&
                    normal_received + clen <= normal_total &&
                    write_all_at(normal_fd, pt_rx + 20, clen,
                                 (off_t)normal_received) == 0)
                    normal_received += clen;
                if (normal_received >= normal_total) {
                    (void)fsync(normal_fd);
                    normal_done = 1;
                    printf("NORMAL_CAPTURE done received=%u grants=%u\n",
                           normal_received, normal_grants);
                    fflush(stdout);
                } else if (clen > 0) {
                    uint32_t remain = normal_total - normal_received;
                    uint32_t len = remain < FIRST_CHUNK ? remain : FIRST_CHUNK;
                    uint8_t grant[20] = {2,0,0,0, 12,0,0,0, 1,0,0,0};
                    put_little32(grant, 2u | ((uint32_t)normal_seq << 16));
                    put_little32(grant + 12, normal_received);
                    put_little32(grant + 16, len);
                    size_t done = 0;
                    while (done < sizeof grant) {
                        ssize_t n = write(o->rfs, grant + done,
                                          sizeof grant - done);
                        if (n < 0 && errno == EINTR) continue;
                        if (n <= 0) break;
                        done += (size_t)n;
                    }
                    normal_grants++;
                }
            }
        }
        memmove(pt_rx, pt_rx + size, pt_used - (size_t)size);
        pt_used -= (size_t)size;
    }
    if (pt_used == sizeof pt_rx) pt_used = 0;
}

static int complete_frame(struct owner *o, const uint8_t *frame,
                          size_t len, size_t trailing, int64_t now)
{
    rfs_trace("serve", o->phase, frame, len, now);
    if (stop_requested || now < 0 || now >= o->deadline_ms ||
        now >= o->total_deadline_ms) {
        diagnose(o, frame_stage(o), REASON_TIMEOUT);
        return -1;
    }
    /* File-service OPEN/READ/CLOSE can arrive before the NV cmd7 sequence.
     * Answer them here so a carrierconfig open is not a malformed NV frame. */
    if (cc_serve(o->rfs, frame, len)) {
        int64_t progressed_at = monotonic_ms();
        if (progressed_at >= 0 && progressed_at < o->total_deadline_ms) {
            o->deadline_ms = progressed_at + STEP_DEADLINE_MS;
            if (o->deadline_ms > o->total_deadline_ms)
                o->deadline_ms = o->total_deadline_ms;
        }
        return 0;
    }
    enum action action = classify(o, frame, len);
    if (action == BAD_FRAME) {
        if (o->phase == WAIT_DATA) {
            o->frame_mismatch_mask = data_mismatch_mask(o, frame, len);
            if (frame_stage(o) == STAGE_FINAL_FRAME)
                diagnose_final_frame_shape(o, frame, len, trailing);
        }
        diagnose(o, frame_stage(o), REASON_MALFORMED);
        return -1;
    }
    if (trailing && action != NO_REPLY) {
        diagnose(o, frame_stage(o), REASON_TRAILING);
        return -1;
    }
    if (action == STATUS_7) {
        uint8_t st[sizeof status_7];
        memcpy(st, status_7, sizeof st);
        st[2] = frame[2];
        st[3] = frame[3];
        if (reply_gate(o) ||
            send_modem_once(o, st, sizeof st)) return -1;
        o->phase = WAIT_3;
    } else if (action == NO_REPLY) {
        o->phase = WAIT_6;
    } else if (action == GRANT_1) {
        o->nv_seq = little16(frame + 2);
        if (send_next_grant(o)) return -1;
    } else if (action == STORE_CHUNK) {
        int stored;
#ifdef RFS_HOST_TEST
        if (host_store_override) stored = 0;
        else
#endif
        stored = store_chunk(o, frame + 20);
        if (stored) return -1;
#ifdef RFS_HOST_TEST
        if (host_store_override) {
            sha_update(&o->received_hash, frame + 20, o->expected_chunk);
            o->received_bytes += o->expected_chunk;
            o->chunks_stored++;
        }
#endif
        if (o->received_bytes < RFS_TRANSFER_BYTES) {
            if (send_next_grant(o)) return -1;
        } else {
            if (o->received_bytes != RFS_TRANSFER_BYTES ||
                o->chunks_stored != RFS_GRANTS_MAX ||
                o->grant_attempted != RFS_GRANTS_MAX) {
                diagnose(o, STAGE_FINALIZE, REASON_STATE);
                return -1;
            }
            int finished;
#ifdef RFS_HOST_TEST
            if (host_finish_override) finished = 0;
            else
#endif
            finished = finish_candidate(o);
            if (finished) {
                diagnose(o, STAGE_FINALIZE, REASON_IO);
                return -1;
            }
            if (reply_gate(o)) {
                diagnose(o, STAGE_FINAL_ACK, REASON_GATE);
                return -1;
            }
#ifndef RFS_HOST_TEST
            if (sidecar_stable(o)) {
                diagnose(o, STAGE_FINAL_ACK, REASON_GATE);
                return -1;
            }
#else
            if (!host_finish_override && sidecar_stable(o)) {
                diagnose(o, STAGE_FINAL_ACK, REASON_GATE);
                return -1;
            }
#endif
            o->final_ack_attempted = 1;
            uint8_t fin[sizeof final_status];
            memcpy(fin, final_status, sizeof fin);
            if (o->nv_seq) {
                fin[2] = (uint8_t)o->nv_seq;
                fin[3] = (uint8_t)(o->nv_seq >> 8);
            }
            if (send_modem_once(o, fin, sizeof fin)) {
                diagnose(o, STAGE_FINAL_ACK, REASON_SEND);
                return -1;
            }
            o->final_ack_sent = 1;
            int64_t ack_returned_at = monotonic_ms();
            if (ack_returned_at >= o->sit.ready_ms)
                printf("rfs_final_ack_local_write_returned owner_elapsed_ms=%lld\n",
                       (long long)(ack_returned_at - o->sit.ready_ms));
            else
                puts("rfs_final_ack_local_write_returned owner_elapsed_ms=unknown");
            terminal(o, "complete_quarantined_ack");
            return 0;
        }
    } else {
        diagnose(o, frame_stage(o), REASON_STATE);
        return -1;
    }
    int64_t progressed_at = monotonic_ms();
    if (progressed_at < 0 || progressed_at >= o->total_deadline_ms) {
        diagnose(o, STAGE_DEADLINE, REASON_TIMEOUT);
        return -1;
    }
    o->deadline_ms = progressed_at + STEP_DEADLINE_MS;
    if (o->deadline_ms > o->total_deadline_ms)
        o->deadline_ms = o->total_deadline_ms;
    return 0;
}

static int feed_rfs(struct owner *o, const uint8_t *bytes,
                    size_t len, int64_t now)
{
    if (o->phase == TERMINAL || !bytes || len > sizeof o->rx - o->used) {
        diagnose(o, frame_stage(o), REASON_RX_OVERFLOW);
        return -1;
    }
    memcpy(o->rx + o->used, bytes, len);
    o->used += len;
    while (o->used) {
        int size = valid_frame_length(o->rx, o->used);
        if (size < 0) {
            diagnose(o, frame_stage(o), REASON_LENGTH);
            return -1;
        }
        if (size == 0) break;
        size_t trailing = o->used - (size_t)size;
        if (complete_frame(o, o->rx, (size_t)size, trailing, now)) return -1;
        if (o->phase == TERMINAL) return 0;
        memmove(o->rx, o->rx + size, trailing);
        memset(o->rx + trailing, 0, (size_t)size);
        o->used = trailing;
    }
    if (o->used == sizeof o->rx) {
        diagnose(o, frame_stage(o), REASON_RX_OVERFLOW);
        return -1;
    }
    return 0;
}

/* This observer shares the existing exclusive IPC owner. It never opens a
 * second endpoint, issues a SET, or gives RFS work to the SIT parser. */
static const struct {
    uint16_t id;
    const char *name;
} sit_gets[SIT_GET_COUNT] = {
    {0x0200, "sim"}, {0x0801, "radio"},
    {SIT_NET_VOICE_REG, "voice"}, {SIT_NET_DATA_REG, "data"}
};

static const char *sit_pass_name(unsigned pass)
{
    return pass == 0 ? "online" : "settled";
}

static void sit_unknown(struct sit_observer *s, const char *status)
{
    printf("sit_snapshot pass=%s field=%s status=%s\n",
           sit_pass_name(s->pass), sit_gets[s->next].name, status);
}

static void sit_disable(struct sit_observer *s, const char *reason)
{
    if (!s->poisoned)
        printf("sit_observer=%s no_more_gets=1\n", reason);
    s->poisoned = 1;
    s->pending = 0;
    s->used = 0;
    zero_bytes(s->rx, sizeof s->rx);
}

static void sit_poison(struct sit_observer *s)
{
    sit_disable(s, "framing_unknown");
}

static void sit_endpoint_fault(struct owner *o)
{
    o->sit.endpoint_failed = 1;
    sit_disable(&o->sit, "endpoint_lost");
    /* The RFS reply gate requires both channel descriptors. Losing IPC
     * before the final ACK disarms quarantine, as in the prior owner. */
    if (o->phase != TERMINAL) terminal(o, "ipc_endpoint_lost");
}

static void poll_faults_before_rfs(struct owner *o,
                                   short rfs_revents, short ipc_revents)
{
    const short faults = POLLERR | POLLHUP | POLLNVAL;
    /* A co-reported final RFS chunk must not be acknowledged while IPC is
     * already in fault state. Inspect both endpoints before either read. */
    if (ipc_revents & faults) sit_endpoint_fault(o);
    if (rfs_revents & faults) terminal(o, "rfs_endpoint_lost");
}

/* Response size is the exact SIT header length, not the read() boundary. */
static int sit_frame_length(const uint8_t *p, size_t used)
{
    if (used < 6) return 0;
    if (p[0] > 2) return -1;
    uint16_t length = little16(p + 4);
    unsigned minimum = p[0] == 2 ? 8u : 12u;
    /* The 16-bit wire length cannot exceed the 65536-byte parser cap. */
    if (length < minimum) return -1;
    return used < length ? 0 : (int)length;
}

static int sit_sim_status_complete(const uint8_t *p, size_t len)
{
    if (len < 15) return 0;
    unsigned apps = p[SIT_SIM_APPS];
    return apps <= 4 && len >= 15u + (size_t)apps * SIT_SIM_APP_STRIDE;
}

/* Record only framed unsolicited network/radio headers. The payload may
 * contain network or subscriber data and must not be inspected or logged. */
static int sit_event_trace_take(struct sit_observer *s, const uint8_t *p,
                                size_t len, unsigned *id)
{
    if (len < 8 || p[0] != 2 || little16(p + 4) != len) return 0;
    unsigned value = little16(p + 2);
    if ((value & 0xff00u) != 0x0700u &&
        (value & 0xff00u) != 0x0800u) return 0;
    if (s->event_seen != UINT_MAX) s->event_seen++;
    if (s->event_logged >= SIT_EVENT_TRACE_LIMIT) {
        if (s->event_overflow != UINT64_MAX) s->event_overflow++;
        return 0;
    }
    s->event_logged++;
    *id = value;
    return 1;
}

static int sit_event_quiet_window(const struct sit_observer *s)
{
    return s->event_window_ms >= s->ready_ms &&
           s->event_window_ms - s->ready_ms <
           SIT_EVENT_QUIET_COVERAGE_MS;
}

static void sit_event_trace_window(struct sit_observer *s, int64_t now)
{
    if (now < s->event_window_ms ||
        now - s->event_window_ms < SIT_EVENT_WINDOW_MS) return;
    if (s->event_seen || sit_event_quiet_window(s))
        printf("sit_event_window owner_elapsed_ms=%lld window_ms=%lld seen=%u logged=%u overflow=%llu trace_active=%u\n",
               (long long)(now - s->ready_ms),
               (long long)(now - s->event_window_ms), s->event_seen,
               s->event_logged, (unsigned long long)s->event_overflow,
               !s->poisoned && !s->endpoint_failed);
    s->event_window_ms = now;
    s->event_seen = s->event_logged = 0;
    s->event_overflow = 0;
}

static void sit_on_frame(struct sit_observer *s, const uint8_t *p, size_t len,
                         int64_t received_at)
{
    unsigned event_id;
    if (sit_event_trace_take(s, p, len, &event_id))
        printf("sit_event elapsed_ms=%lld id=0x%04x len=%zu\n",
               (long long)(received_at - s->ready_ms), event_id, len);
    if (!s->pending || s->pass >= 2 || s->next >= SIT_GET_COUNT ||
        (s->deadline_ms > 0 && received_at >= s->deadline_ms) ||
        len < 12 || p[0] != 1 ||
        little16(p + 2) != sit_gets[s->next].id ||
        little16(p + 4) != len || little32(p + 6) != s->token)
        return; /* Unsolicited and stale responses are never evidence. */
    unsigned id = sit_gets[s->next].id;
    unsigned error = little16(p + 10);
    const char *name = sit_gets[s->next].name;
    printf("sit_snapshot pass=%s field=%s response=yes error_raw=%u",
           sit_pass_name(s->pass), name, error);
    if (error) {
        puts(" status=unknown_remote_error");
    } else if (id == 0x0200) {
        if (!sit_sim_status_complete(p, len))
            puts(" status=unknown_short");
        else {
            unsigned apps = p[SIT_SIM_APPS];
            printf(" card_raw=%u apps=%u", p[SIT_SIM_CARD], apps);
            if (apps)
                printf(" app_state_raw=%u pin1_raw=%u",
                       p[SIT_SIM_APP_STATE], p[SIT_SIM_PIN1]);
            putchar('\n');
        }
    } else if (id == 0x0801) {
        if (len < 16) puts(" status=unknown_short");
        else printf(" radio_raw=%u\n", little32(p + 12));
    } else if (len < (id == SIT_NET_DATA_REG ? 16u : 14u)) {
        puts(" status=unknown_short");
    } else {
        putchar('\n');
        sit_net_log_regstate("sit_snapshot", name, p, len, id);
    }
    s->pending = 0;
    s->next++;
}

/* A separate, bounded streaming buffer handles fragmented and coalesced IPC.
 * Malformed/oversized IPC disables only this observer, never RFS quarantine. */
static void sit_feed(struct sit_observer *s, const uint8_t *bytes, size_t len,
                     int64_t received_at)
{
    if (s->poisoned) return;
    while (len && !s->poisoned) {
        size_t space = sizeof s->rx - s->used;
        if (!space) { sit_poison(s); break; }
        size_t take = len < space ? len : space;
        memcpy(s->rx + s->used, bytes, take);
        s->used += take;
        bytes += take;
        len -= take;
        size_t offset = 0;
        while (offset < s->used) {
            int length = sit_frame_length(s->rx + offset, s->used - offset);
            if (length < 0) { sit_poison(s); break; }
            if (!length) break;
            sit_on_frame(s, s->rx + offset, (size_t)length, received_at);
            offset += (size_t)length;
        }
        if (s->poisoned) break;
        if (offset) {
            size_t remaining = s->used - offset;
            memmove(s->rx, s->rx + offset, remaining);
            zero_bytes(s->rx + remaining, offset);
            s->used = remaining;
        }
    }
}

/* A short or ambiguous IPC write consumes the request; never resend it. */
static int sit_send_get_once(int ipc, uint16_t id, uint32_t token)
{
    uint8_t request[12] = {0};
    request[2] = (uint8_t)id;
    request[3] = (uint8_t)(id >> 8);
    request[4] = sizeof request;
    put_little32(request + 6, token);
    sigset_t blocked, old;
    sigemptyset(&blocked);
    sigaddset(&blocked, SIGTERM);
    sigaddset(&blocked, SIGINT);
    if (sigprocmask(SIG_BLOCK, &blocked, &old)) return -1;
    ssize_t written = -1;
    if (!stop_requested) {
#ifdef RFS_HOST_TEST
        if (host_sit_write_override) {
            host_sit_write_calls++;
            written = host_sit_write_override(ipc, request, sizeof request);
        } else
#endif
        written = write(ipc, request, sizeof request);
    }
    int restored = sigprocmask(SIG_SETMASK, &old, NULL);
    zero_bytes(request, sizeof request);
    return restored == 0 && written == 12 ? 0 : -1;
}

static void sit_advance(struct owner *o, int64_t now)
{
    struct sit_observer *s = &o->sit;
    if (s->poisoned || s->pass >= 2 || stop_requested) return;
    /* RFS failure or ambiguous ACK forbids any further modem writes, even
     * read-only GETs. A completed, acknowledged quarantine may be observed. */
    if (o->phase == TERMINAL && !o->final_ack_sent) return;
    if (s->pending) {
        if (now < s->deadline_ms) return;
        sit_unknown(s, "unknown_timeout");
        /* The CP may still be completing this request. Do not overlap it
         * with another GET, even if its late reply is ignored by token. */
        sit_disable(s, "reply_timeout");
        return;
    }
    if (s->next == SIT_GET_COUNT) {
        s->pass++;
        s->next = 0;
        s->started = 0;
        if (s->pass >= 2) return;
    }
    if (s->pass == 1 && now < s->ready_ms + SIT_SETTLED_MS) return;
    /* The loop services RFS before IPC and reaches here only afterward.
     * Keep away from partial RFS frames and its deadline; do not poll the
     * CPIF fd again here, because it may report POLLIN for an empty read. */
    if (o->phase != TERMINAL) {
        if (now + SIT_RFS_GUARD_MS >= o->deadline_ms ||
            o->used != 0) return;
    }
    if (!s->started) {
        s->started = 1;
        printf("sit_snapshot=%s_started elapsed_ms=%lld read_only=1\n",
               sit_pass_name(s->pass), (long long)(now - s->ready_ms));
    }
    if (!s->token)
        s->token = (uint32_t)now ^ (uint32_t)getpid() ^ 0x5a170000u;
    ++s->token;
    if (sit_send_get_once(o->ipc, sit_gets[s->next].id, s->token)) {
        sit_unknown(s, "unknown_write_no_retry");
        s->poisoned = 1; /* Do not start a later pass after ambiguous write. */
        return;
    }
    s->pending = 1;
    s->deadline_ms = now + SIT_REPLY_MS;
}

#ifdef SAAIOS_RFS_CAMP
#include "cc-sit-4600.inc"
#include "cc-sit-0600.inc"
#include "cc-sit-pre0600.inc"
/* Exact stock stage-1 request builders, recovered from the vendor RIL and
 * matching modem-channel-owner.c byte-for-byte. No CLI value or NV access. */
static void make_setmodemsconfig_request(uint8_t request[SEQ_CONFIG_LEN],
                                         uint32_t token)
{
    memset(request, 0, SEQ_CONFIG_LEN);
    request[2] = (uint8_t)SEQ_CONFIG_COMMAND;        /* 0x3f */
    request[3] = (uint8_t)(SEQ_CONFIG_COMMAND >> 8); /* 0x09 */
    request[4] = SEQ_CONFIG_LEN;
    put_little32(request + 6, token);
    request[12] = 0; /* one modem */
}

static void make_sgc_request(uint8_t request[SGC_LEN], uint32_t token)
{
    memset(request, 0, SGC_LEN);
    request[2] = 0x04;
    request[3] = 0x04;
    request[4] = SGC_LEN;
    put_little32(request + 6, token);
    put_little32(request + 12, 0x0101);
}

static void make_radiopower_request(uint8_t request[CAMP_POWER_LEN],
                                    uint32_t token, uint32_t power)
{
    memset(request, 0, CAMP_POWER_LEN);
    request[2] = (uint8_t)CAMP_POWER_COMMAND;        /* 0x00 */
    request[3] = (uint8_t)(CAMP_POWER_COMMAND >> 8); /* 0x08 */
    request[4] = CAMP_POWER_LEN;
    put_little32(request + 6, token);
    put_little32(request + 12, power); /* 2 = ON, 1 = OFF (BuildRadioPower) */
    request[16] = 0; /* arg2 */
    request[17] = 0; /* arg3 */
}

/* Fed every framed IPC indication. Tracks the factory-callback trigger pair:
 * an exact 8-byte type-2 0x0803 (UNAVAILABLE) then an exact 12-byte type-2
 * 0x0802 with raw scalar 0. Any duplicate, reversed, malformed or superseding
 * radio event invalidates eligibility; no later event rearms it. */
static void camp_observe(struct camp_driver *c, const uint8_t *p, size_t n,
                         int64_t now)
{
    if (c->radio_invalidated || c->radio_ready0_ms) {
        if (n >= 8 && p[0] == 2 &&
            (little16(p + 2) == 0x0803 || little16(p + 2) == 0x0802))
            c->radio_invalidated = 1;
        return;
    }
    if (n < 8 || p[0] != 2) return;
    unsigned id = little16(p + 2);
    if (id == 0x0803) {
        if (n != 8 || little16(p + 4) != n) { c->radio_invalidated = 1; return; }
        if (now - c->owner_start_ms > CAMP_WINDOW_MS) return;
        if (c->radio_stage != 0) { c->radio_invalidated = 1; return; }
        c->radio_stage = 1;
        c->radio_unavail_ms = now;
    } else if (id == 0x0802) {
        int parsed = (n == 12 && little16(p + 4) == n) ? 1 : -1;
        uint32_t raw = parsed == 1 ? little32(p + 8) : UINT32_MAX;
        if (c->radio_stage != 1 || parsed != 1 || raw != 0 ||
            now - c->radio_unavail_ms > CAMP_PAIR_GAP_MS ||
            now - c->owner_start_ms > CAMP_WINDOW_MS) {
            c->radio_invalidated = 1;
            return;
        }
        c->radio_ready0_ms = now;
    }
}

/* Match the three camp ACKs by id and echoed token. Only fixed opcodes and the
 * public error word reach the log; never any payload. */
static void camp_ack(struct camp_driver *c, const uint8_t *p, size_t n)
{
    if (n < 12 || p[0] != 1 || little16(p + 4) != n) return;
    unsigned id = little16(p + 2);
    uint32_t token = little32(p + 6);
    unsigned error = little16(p + 10);
    if (c->cfg_sent && !c->cfg_acked &&
        id == SEQ_CONFIG_COMMAND && token == c->cfg_token) {
        c->cfg_acked = 1;
        c->cfg_error = error;
        printf("camp_ack cmd=0x093f response=yes error_raw=%u\n", error);
    } else if (c->sgc_sent && !c->sgc_acked &&
               id == SGC_COMMAND && token == c->sgc_token) {
        c->sgc_acked = 1;
        c->sgc_error = error;
        printf("camp_ack cmd=0x0404 response=yes error_raw=%u\n", error);
    } else if (c->camp_sent && !c->camp_acked &&
               id == CAMP_POWER_COMMAND && token == c->camp_token) {
        c->camp_acked = 1;
        c->camp_error = error;
        printf("camp_ack cmd=0x0800 response=yes error_raw=%u\n", error);
    }
}

/* sit-stream.so GetSignalStrength reads a 16-bit technology-presence mask at
 * response +12; its V4 parser needs 196 bytes after the mask (210 total).
 * Only the low-seven presence bits are extracted; never any measurement. */
static int camp_signal_mask(const uint8_t *p, size_t n, uint32_t token,
                            uint32_t *mask)
{
    if (n < PROBE_SIGNAL_MIN || p[0] != 1 || little16(p + 2) != 0x0900 ||
        little32(p + 6) != token || little16(p + 4) != n ||
        little16(p + 10)) return 0;
    *mask = little16(p + 12) & 0x7fu;
    return 1;
}

/* Prober GET rotation: SIM status, signal presence, and voice/data
 * registration. SIM is also re-queried immediately on each 0x0210. */
static const struct { unsigned id; const char *name; } camp_probe_gets[] = {
    {0x0200, "sim"}, {0x0900, "signal"},
    {SIT_NET_VOICE_REG, "voice"}, {SIT_NET_DATA_REG, "data"},
    {SIT_NET_OPERATOR, "operator"}
};
enum { CAMP_PROBE_COUNT =
           (int)(sizeof camp_probe_gets / sizeof camp_probe_gets[0]) };

/* Factory BuildSetPreferredNetworkType (0x070a): 16-byte request, the RAT value
 * as a little-endian word at +12. Recovered and self-tested in
 * ready-network-once.c; no invented bytes. */
static void make_setpref_request(uint8_t request[REG_PREF_LEN], uint32_t value,
                                 uint32_t token)
{
    memset(request, 0, REG_PREF_LEN);
    request[2] = (uint8_t)REG_PREF_SET;        /* 0x0a */
    request[3] = (uint8_t)(REG_PREF_SET >> 8); /* 0x07 */
    request[4] = REG_PREF_LEN;
    put_little32(request + 6, token);
    put_little32(request + 12, value);
}

/* Factory BuildAllowData (0x0710): 13-byte request, allow flag 1 at +12. */
static void make_allowdata_request(uint8_t request[REG_ALLOW_LEN],
                                   uint32_t token)
{
    memset(request, 0, REG_ALLOW_LEN);
    request[2] = (uint8_t)REG_ALLOW_DATA;        /* 0x10 */
    request[3] = (uint8_t)(REG_ALLOW_DATA >> 8); /* 0x07 */
    request[4] = REG_ALLOW_LEN;
    put_little32(request + 6, token);
    request[12] = 1;
}

/* Factory BuildSetNetworkSelectionManual (0x0705): 22-byte request. RAT int32 at
 * +12 (0 = any; the builder maps an out-of-range RIL type to 0, so "any" lets the
 * modem pick the RAT the PLMN is on). PLMN numeric ASCII copied to +16 (5 or 6
 * chars); the stock builder presets +21 to '#' (0x23) so a 5-digit (2-digit MNC)
 * PLMN keeps the filler. Recovered byte-for-byte; no invented bytes, no NV/EFS
 * write -- this is a radio network-selection command, not a provisioning write. */
static void make_manual_select_request(uint8_t request[SIT_NET_SEL_MANUAL_LEN],
                                       const char *plmn, uint32_t token)
{
    size_t i;
    memset(request, 0, SIT_NET_SEL_MANUAL_LEN);
    request[2] = (uint8_t)SIT_NET_SELECTION_MANUAL;        /* 0x05 */
    request[3] = (uint8_t)(SIT_NET_SELECTION_MANUAL >> 8); /* 0x07 */
    request[4] = (uint8_t)SIT_NET_SEL_MANUAL_LEN;          /* 22 */
    put_little32(request + 6, token);
    /* RAT at +12 left 0 (any). */
    request[SIT_NET_SEL_MANUAL_PLMN_OFFSET + 5] = 0x23;    /* '#' filler */
    for (i = 0; i < (size_t)SIT_NET_SEL_MANUAL_PLMN_MAX && plmn && plmn[i]; i++)
        request[SIT_NET_SEL_MANUAL_PLMN_OFFSET + i] = (uint8_t)plmn[i];
}

/* SIM PIN1 verify (ProtocolSimBuilderLegacy::BuildSimVerifyPin, which=0 => PIN1
 * opcode 0x0201, len 38). [12]=PIN length (strlen capped 8), [13..]=PIN ASCII,
 * AID omitted (NULL-AID verify of the primary application). The PIN is a secret:
 * callers must scrub the frame after sending; it is never logged. */
static void make_pin_verify_request(uint8_t request[CALL_PIN_LEN],
                                    const char *pin, uint32_t token)
{
    size_t plen = pin ? strlen(pin) : 0;
    if (plen > CALL_PIN_MAX) plen = CALL_PIN_MAX;
    memset(request, 0, CALL_PIN_LEN);
    request[2] = (uint8_t)CALL_PIN_VERIFY;        /* 0x01 */
    request[3] = (uint8_t)(CALL_PIN_VERIFY >> 8); /* 0x02 */
    request[4] = (uint8_t)CALL_PIN_LEN;           /* 38 */
    put_little32(request + 6, token);
    request[CALL_PIN_LEN_OFF] = (uint8_t)plen;
    if (plen) memcpy(request + CALL_PIN_OFF, pin, plen);
}

/* Voice MO call (ProtocolCallBuilder::BuildDial, opcode 0x0001, len 104). Plain
 * voice call with default call type (0) and default CLIR (0); the number is an
 * ordinary dial string (not a secret). */
static void make_dial_request(uint8_t request[CALL_DIAL_LEN],
                              const char *number, uint32_t token)
{
    size_t nlen = number ? strlen(number) : 0;
    if (nlen > CALL_DIAL_NUM_MAX) nlen = CALL_DIAL_NUM_MAX;
    memset(request, 0, CALL_DIAL_LEN);
    request[2] = (uint8_t)CALL_DIAL;        /* 0x01 */
    request[3] = (uint8_t)(CALL_DIAL >> 8); /* 0x00 */
    request[4] = (uint8_t)CALL_DIAL_LEN;    /* 104 */
    put_little32(request + 6, token);
    request[CALL_DIAL_TYPE_OFF] = 0;        /* call type: voice */
    request[CALL_DIAL_ARG5_OFF] = 0;
    request[CALL_DIAL_NUMLEN_OFF] = (uint8_t)nlen;
    if (nlen) memcpy(request + CALL_DIAL_NUM_OFF, number, nlen);
    request[CALL_DIAL_TOA_OFF] =
        (nlen && number[0] == '+') ? CALL_DIAL_TOA_INTL : CALL_DIAL_TOA_NATL;
    request[CALL_DIAL_PRESENT_OFF] = 1;
    request[CALL_DIAL_CLIR_OFF] = 0;        /* CLIR default */
    request[CALL_DIAL_ARG6_OFF] = 0;
}

/* SMS-SUBMIT TPDU (3GPP TS 23.040): MTI submit, no validity period, GSM 7-bit
 * alphabet, two septets. Destination digits are BCD. Returns the TPDU length,
 * or 0 when the number is not a short digit string. */
static int gsm_submit_pdu(uint8_t *dst, size_t cap, const char *number)
{
    const char *digits = number ? number : "";
    int intl = 0;
    size_t n = 0, bcd, len, i, p;
    if (*digits == '+') { intl = 1; digits++; }
    while (digits[n] >= '0' && digits[n] <= '9') n++;
    if (n < 1 || n > SMS_DIGITS_MAX || digits[n] != 0) return 0;
    bcd = (n + 1u) / 2u;
    len = 7u + bcd + 2u;
    if (len > cap || len > SMS_PDU_MAX) return 0;
    dst[0] = 0x01;
    dst[1] = 0x00;
    dst[2] = (uint8_t)n;
    dst[3] = intl ? 0x91 : 0x81;
    for (i = 0; i < bcd; i++) {
        unsigned lo = (unsigned)(digits[i * 2u] - '0');
        unsigned hi = 0xfu;
        if (i * 2u + 1u < n) hi = (unsigned)(digits[i * 2u + 1u] - '0');
        dst[4u + i] = (uint8_t)(lo | (hi << 4));
    }
    p = 4u + bcd;
    dst[p++] = 0x00;
    dst[p++] = 0x00;
    dst[p++] = 2;
    dst[p++] = 0xef;
    dst[p++] = 0x35;
    return (int)p;
}

/* One SMS (ProtocolSmsBuilder::BuildSendSms). Empty SMSC uses the stock
 * fallback so the modem keeps the SMSC already on the card. */
static int make_sms_request(uint8_t request[SMS_SEND_LEN],
                            const char *number, uint32_t token)
{
    int plen;
    memset(request, 0, SMS_SEND_LEN);
    request[2] = (uint8_t)SMS_SEND;
    request[3] = (uint8_t)(SMS_SEND >> 8);
    request[4] = (uint8_t)SMS_SEND_LEN;
    request[5] = (uint8_t)(SMS_SEND_LEN >> 8);
    put_little32(request + 6, token);
    request[SMS_PROP_OFF] = 0;
    request[SMS_SMSC_OFF] = 1;
    plen = gsm_submit_pdu(request + SMS_PDU_OFF, SMS_PDU_MAX, number);
    if (plen <= 0) return 0;
    request[SMS_PDU_LEN_OFF] = (uint8_t)plen;
    return 1;
}

/* Hang up one call by index (ProtocolCallBuilder::BuildHangup, opcode 0x0008,
 * len 20). [12]=call index int32, [16]=1. */
static void make_hangup_request(uint8_t request[CALL_HANGUP_LEN],
                                int index, uint32_t token)
{
    memset(request, 0, CALL_HANGUP_LEN);
    request[2] = (uint8_t)CALL_HANGUP;        /* 0x08 */
    request[3] = (uint8_t)(CALL_HANGUP >> 8); /* 0x00 */
    request[4] = (uint8_t)CALL_HANGUP_LEN;    /* 20 */
    put_little32(request + 6, token);
    put_little32(request + CALL_HANGUP_INDEX_OFF, (uint32_t)index);
    put_little32(request + CALL_HANGUP_FLAG_OFF, 1);
}

/* VERDICT 17: len-16 GET_AVAILABLE_NETWORKS (BuildQueryAvailableNetwork(int),
 * opcode 0x0706, len 16). scanType int32 at [12], clamped exactly as the stock:
 * (1<=mode<=5) ? mode : 0. Nothing invented. */
/* VERDICT 20: replicate ProtocolNetworkBuilder::BuildSetAllowedNetworkTypeBitmap
 * (int) @0x238670 bit-for-bit. Maps an Android RadioAccessFamily (RAF) bitmap to
 * the SIT wire bitmap written at request payload[12]. Each line mirrors one
 * bfi/orr in the stock builder; verified against GetRat's inverse (@0x2340d0).
 * No wire bytes are invented -- the transform is recovered from the binary. */
static uint32_t raf_to_sit_ratbm(uint32_t raf)
{
    uint32_t w = raf & 0xfu;                  /* wire[0..3] = raf[0..3]        */
    w |= ((raf >> 9) & 1u) << 4;              /* wire4  = raf9  (HSDPA)        */
    w |= ((raf >> 10) & 1u) << 5;             /* wire5  = raf10 (HSUPA)        */
    w |= ((raf >> 11) & 1u) << 6;             /* wire6  = raf11 (HSPA)         */
    w |= ((raf >> 4) & 1u) << 11;             /* wire11 = raf4  (IS95A)        */
    w |= ((raf >> 5) & 1u) << 12;             /* wire12 = raf5  (IS95B)        */
    w |= ((raf >> 6) & 1u) << 13;             /* wire13 = raf6  (1xRTT)        */
    w |= ((raf >> 7) & 1u) << 14;             /* wire14 = raf7  (EVDO_0)       */
    w |= ((raf >> 8) & 1u) << 15;             /* wire15 = raf8  (EVDO_A)       */
    w |= ((raf >> 12) & 1u) << 16;            /* wire16 = raf12 (EVDO_B)       */
    w |= ((raf >> 13) & 1u) << 17;            /* wire17 = raf13 (EHRPD)        */
    if (raf & ((1u << 14) | (1u << 19)))      /* LTE or LTE_CA                 */
        w |= 0x80u;                           /* wire7  = LTE                  */
    w |= ((raf >> 15) & 1u) << 8;             /* wire8  = raf15 (HSPAP)        */
    w |= ((raf >> 16) & 1u) << 9;             /* wire9  = raf16 (GSM)          */
    w |= ((raf >> 17) & 1u) << 10;            /* wire10 = raf17 (TD-SCDMA)     */
    w |= ((raf >> 20) & 1u) << 18;            /* wire18 = raf20 (NR)           */
    return w;
}

static void make_scan16_request(uint8_t request[REG_SCAN16_LEN], int mode,
                                uint32_t token)
{
    int valid = (mode >= SCAN16_MODE_MIN && mode <= SCAN16_MODE_MAX) ? mode : 0;
    memset(request, 0, REG_SCAN16_LEN);
    request[2] = (uint8_t)SIT_NET_AVAILABLE_NETWORKS;        /* 0x06 */
    request[3] = (uint8_t)(SIT_NET_AVAILABLE_NETWORKS >> 8); /* 0x07 */
    request[4] = (uint8_t)REG_SCAN16_LEN;                     /* 16 */
    put_little32(request + 6, token);
    put_little32(request + SCAN16_MODE_OFF, (uint32_t)valid);
}

/* VERDICT 26: StartNetworkScan (0x0734), one EUTRAN specifier, empty band list
 * and empty channel list. Byte-exact from TD1A
 * ProtocolNetworkBuilder::BuildStartNetworkScan @0x2376a0 (size 0x62c):
 *   scan scalars: [12]=scanType (0 when arg w1==0), [13]=interval u16 (0 unless
 *   periodic), [15]=maxSearchTime u16 (60 is inside the builder's 59..3600
 *   window), [17]=incrementalResults & 1, [18]=periodicity clamped to [3,10],
 *   [20]=numSpecifiers (min(count,8)), [21]=numMccMncs.
 *   specifier stride 78 at [22]: [0]=RAN byte, [1]=bands_length u32,
 *   [5]=bands[8] bytes, [13]=channels_length. bands_length<1 skips the band
 *   jump table; channels_length<1 skips the channel packer. Both lengths 0 is
 *   the frame the builder emits for an empty band/channel vector (scan every
 *   band of that RAN). RAN 3 = EUTRAN. One specifier keeps the builder's
 *   trailing numSpecifiers clear (it zeros the count only when its RAN
 *   accounting sum exceeds 2). Frame = 12 + 10 + 78 = 100. No NV/EFS write. */
static void make_startscan_request(uint8_t request[SCAN734_LEN], uint32_t token)
{
    memset(request, 0, SCAN734_LEN);
    request[2] = (uint8_t)SCAN734_GET;         /* 0x34 */
    request[3] = (uint8_t)(SCAN734_GET >> 8);  /* 0x07 */
    request[4] = (uint8_t)SCAN734_LEN;         /* 100, [5]=0 => LE 100 */
    put_little32(request + 6, token);
    /* scan scalars (off12..21) */
    request[12] = 0;      /* scanType = ONE_SHOT */
    /* [13..14] interval = 0 (one-shot) */
    request[15] = 60;     /* maxSearchTime = 60s (LE16; [16]=0) */
    request[17] = 0;      /* incrementalResults = false */
    request[18] = 3;      /* incrementalResultsPeriodicity = 3 (stock minimum) */
    request[20] = 1;      /* numSpecifiers */
    request[21] = 0;      /* numMccMncs */
    /* specifier[0] at off22. bands_length and channels_length stay 0. */
    request[22] = SCAN734_RAN_EUTRAN;  /* radio_access_network = EUTRAN(3) */
}

_Static_assert(REG_IA_LEN == 251, "CP2A SET_INITIAL_ATTACH_APN frame must be 251 bytes");
_Static_assert(REG_IA_CP2A_OFF < REG_IA_LEN, "CP2A trailing field within frame");

/* Factory BuildSetInitialAttachApn (0x0603): 250-byte request. Layout and every
 * body byte are recovered from ProtocolPsBuilder::BuildSetInitialAttachApn +
 * FillApnInfo<sit_pdp_set_initial_attach_apn_req> in the factory libsitril.so;
 * see the REG_IA_* enum comment. For a plain IP APN with no username/password
 * and no auth, the enum-converted bytes are constants (pdpType=IP=1, authType=0,
 * dataProfileId/apnType/pcscfReqType=0); the APN string is copied verbatim. No
 * invented bytes; nothing here is an NV/EFS write. */
static void make_initial_attach_apn_request(uint8_t request[REG_IA_LEN],
                                            const char *apn, uint32_t token)
{
    memset(request, 0, REG_IA_LEN);
    request[2] = (uint8_t)REG_INIT_ATTACH_APN;        /* 0x03 */
    request[3] = (uint8_t)(REG_INIT_ATTACH_APN >> 8); /* 0x06 */
    request[4] = (uint8_t)REG_IA_LEN;                 /* 251, [5]=0 => LE 251 */
    put_little32(request + 6, token);
    request[12] = REG_IA_CID;        /* attach pdp cid (RetrieveAttachPdpContext) */
    request[13] = REG_IA_CONST13;    /* fixed 0x0e in BuildSetInitialAttachApn */
    /* request[14]=dataProfileId=0, request[15]=apnType=0 (default APN) */
    if (apn) {
        size_t n = 0;
        while (n < (size_t)(REG_IA_APN_MAX - 1) && apn[n]) n++;
        memcpy(request + REG_IA_APN_OFF, apn, n);   /* NUL already from memset */
    }
    /* username (+117) / password (+167) left empty (zeroed) */
    request[REG_IA_AUTH_OFF] = 0;                 /* ConvertAuthType(0)=0 */
    request[REG_IA_PDPTYPE_OFF] = REG_IA_PDPTYPE_IPV4V6;
    request[REG_IA_PCSCF_OFF] = 0;                /* pcscfReqType */
    request[REG_IA_CP2A_OFF] = REG_IA_CP2A_VAL;
}

/* Operational-SET request with a 4-byte little-endian payload at +12 (matches
 * the factory sitril builders, e.g. BuildSetVoiceOperation stores int32 mode at
 * body+0 of a len-16 request). Used for 0x091a (mode 3) and 0x0933 (mode 1). */
static void make_opx_u32_request(uint8_t *request, uint16_t id, uint8_t len,
                                 uint32_t token, uint32_t value)
{
    memset(request, 0, len);
    request[2] = (uint8_t)id;
    request[3] = (uint8_t)(id >> 8);
    request[4] = len;
    put_little32(request + 6, token);
    put_little32(request + 12, value);
}

/* Operational-SET request with a single payload byte at +12 (SIT_SET_STACK_
 * STATUS 0x080f, len 13, mode byte 1 = ENABLE). */
static void make_opx_byte_request(uint8_t *request, uint16_t id, uint8_t len,
                                  uint32_t token, uint8_t value)
{
    memset(request, 0, len);
    request[2] = (uint8_t)id;
    request[3] = (uint8_t)(id >> 8);
    request[4] = len;
    put_little32(request + 6, token);
    request[12] = value;
}

/* Two stock payload bytes at +12. Used for the AIMS frames whose capture
 * body is the pair 01 01 and whose length is 14. */
static void camp_fill_pair(uint8_t *request, uint16_t id, uint32_t token,
                           uint8_t a, uint8_t b)
{
    memset(request, 0, 14);
    request[2] = (uint8_t)id;
    request[3] = (uint8_t)(id >> 8);
    request[4] = 14;
    put_little32(request + 6, token);
    request[12] = a;
    request[13] = b;
}

/* SIT_SET_DUAL_NETWORK_AND_ALLOW_DATA (0x072b, len 28): 4 x int32 payload,
 * recovered from ProtocolNetworkBuilder::BuildSetDualNetworkAndAllowData in the
 * full libsitril.so. Layout (log "Primary(net,allow), Secondary(net,allow)"):
 *   +12 translate(primaryNet)   +16 translate(secondaryNet)
 *   +20 primaryAllowData        +24 secondaryAllowData
 * translateNetworktype is the SAME table BuildSetPreferredNetworkType (0x070a)
 * uses, and translate(12)=12, so the already-proven wire RAT 12 (LTE/WCDMA) and
 * allow-data 1 are reused for all four fields -- no invented bytes. */
static void make_opx_dual_request(uint8_t *request, uint32_t token)
{
    memset(request, 0, OPX_DUAL_LEN);
    request[2] = (uint8_t)OPX_DUAL_SET;
    request[3] = (uint8_t)(OPX_DUAL_SET >> 8);
    request[4] = OPX_DUAL_LEN;
    put_little32(request + 6, token);
    put_little32(request + 12, OPX_DUAL_NET);   /* primary net (wire 12) */
    put_little32(request + 16, OPX_DUAL_NET);   /* secondary net (wire 12) */
    put_little32(request + 20, OPX_DUAL_ALLOW); /* primary allow data */
    put_little32(request + 24, OPX_DUAL_ALLOW); /* secondary allow data */
}

/* Next post-registration operational step, or 0 when the experiment is idle or
 * complete. Reads the three operational GETs for the log, then issues the one
 * selected SET. GETs and the SET are each dispatched at most once. */
static unsigned camp_opx_next(const struct camp_driver *c)
{
    if (!c->reg_complete || c->opx_done) return 0;
    if (!c->opx_stack_get_sent) return OPX_STACK_GET;
    if (!c->opx_voice_get_sent) return OPX_VOICE_GET;
    if (!c->opx_devsvc_get_sent) return OPX_DEVSVC_GET;
    if (!c->opx_set_sent) {
        if (c->opx_step == OPX_STEP_VOICE) return OPX_VOICE_SET;
        if (c->opx_step == OPX_STEP_INTPS) return OPX_INTPS_SET;
        if (c->opx_step == OPX_STEP_STACK) return OPX_STACK_SET;
        if (c->opx_step == OPX_STEP_DEVSVC) return OPX_DEVSVC_SET;
        if (c->opx_step == OPX_STEP_DUAL) return OPX_DUAL_SET;
    }
    return 0;
}

/* Next one-shot registration step given current known state, or 0 when the
 * sequence has nothing to do this tick (waiting, blocked, or complete). */
static unsigned camp_pref_target(const struct camp_driver *c)
{
    return c->pref_target ? c->pref_target : (unsigned)RAT_LTE_WCDMA;
}

static unsigned camp_reg_next(const struct camp_driver *c)
{
    if (!c->sim_ready || c->reg_complete) return 0;

    /* 1. Confirm radio ON (idempotent GET, bounded retries). If the modem
     *    never answers, stop: the SETs below are meaningless with radio off. */
    if (!c->radio_on)
        return c->radio_get_tries < REG_GET_MAX ? REG_RADIO_GET : 0;

    /* 1b. Available-networks scan (0x0706), one-shot, config-gated. RF is up once
     *     radio is ON, so the scan can succeed here; issued before selection so
     *     its result can inform a manual pick. */
    if (c->scan_enabled && !c->scan_sent) return REG_SCAN;

    /* 2. Selection mode. When a manual PLMN is configured, select it explicitly
     *    (0x0705) instead of forcing automatic -- this is the lever a real phone
     *    uses to leave a foreign network for its home PLMN. Otherwise read
     *    (bounded), then force automatic unless the card already reads automatic.
     *    A slow/absent 0x0703 reply must not block the experiment, so after
     *    REG_GET_MAX reads we send 0x0704 once regardless. */
    if (!c->sel_auto_sent && !c->manual_sel_sent) {
        if (c->manual_plmn[0]) return REG_MANUAL_SEL;
        if (!c->sel_known && c->sel_get_tries < REG_GET_MAX) return REG_SEL_GET;
        if (!c->capquery && !(c->sel_known && c->sel_mode == 0)) return REG_SEL_AUTO_SET;
    }

    /* 3. Preferred RAT: read (bounded), then set LTE_WCDMA once so the present
     *    UMTS signal is usable. Any value other than an explicit LTE_WCDMA
     *    readback (including unknown-after-retries and the observed raw 16) is
     *    broadened — PS sitting NOT_SEARCHING on UMTS means WCDMA is excluded. */
    if (!c->pref_set_sent) {
        if (!c->pref_known && c->pref_get_tries < REG_GET_MAX) return REG_PREF_GET;
        if (!c->capquery && !(c->pref_known && c->preferred_raw == camp_pref_target(c))) return REG_PREF_SET;
    }

    /* 4. Initial-attach APN: stock issues SET_INITIAL_ATTACH_APN (0x0603) as the
     *    PS-attach precondition (the default-bearer/attach APN) before allowing
     *    data. Sent once, only when an APN is configured. */
    if (!c->capquery && c->apn[0] && !c->ia_apn_sent) return REG_INIT_ATTACH_APN;

    /* 5. Allow PS data once. */
    if (!c->capquery && !c->allow_data_sent) return REG_ALLOW_DATA;
    return 0;
}

/* Radio-cycle is armed by dereg_scan (legacy 0x0706) or by scan734 (one
 * 0x0734 in the same pre-camp window). */
static int camp_radio_cycle_armed(const struct camp_driver *c)
{
    return c->dereg_enabled || c->scan734;
}

/* Deregister-then-scan sub-sequence (VERDICT 15), run only after the normal
 * bring-up completes. Cycles RADIO_POWER OFF -> (confirm) -> ON, then fires
 * ONE scan from the limited-service window before the modem re-camps. When
 * scan734 is armed that scan is StartNetworkScan 0x0734 and manual-select is
 * not sent. Otherwise (dereg_scan only) it is the legacy available-networks
 * scan and, if the home PLMN is then visible, ONE manual-select. Each step is
 * dispatched at most once; a DRG_NONE return means "waiting for the pending
 * reply" and the prober's probe_pending gate holds until it arrives. */
static unsigned camp_dereg_next(const struct camp_driver *c)
{
    if (!c->reg_complete || !camp_radio_cycle_armed(c) || c->drg_done)
        return DRG_NONE;
    if (!c->drg_off_sent) return DRG_OFF;
    if (!c->drg_off_ack) return DRG_NONE;
    if (!c->drg_off_confirmed && c->drg_off_get_tries < REG_GET_MAX)
        return DRG_OFF_GET;
    if (!c->drg_on_sent) return DRG_ON;
    if (!c->drg_on_ack) return DRG_NONE;
    if (!c->drg_scan_sent) return DRG_SCAN;
    if (!c->drg_scan_got) return DRG_NONE;
    if (c->scan734) return DRG_NONE;
    if (c->manual_plmn[0] && c->drg_target_visible && !c->drg_manual_sent)
        return DRG_MANUAL;
    return DRG_NONE;
}

/* Match a prober reply or note a SIM-status-changed indication. Only scalar
 * status fields and the public error word ever reach the log. */
/* Advance the deregister-scan sub-sequence on a matching reply. Returns 1 when
 * the reply was consumed here (RADIO_POWER OFF/ON acks, or a scan/manual error).
 * Scan success is left for the REG_SCAN branch (it must parse the list); manual
 * success is also finalized here. Keeps the sequence from stalling or respamming
 * on an error by marking it done. */
static int camp_dereg_reply(struct camp_driver *c, unsigned id, unsigned error)
{
    if (!camp_radio_cycle_armed(c) || c->drg_done) return 0;
    if (id == CAMP_POWER_COMMAND) {
        if (c->drg_off_sent && !c->drg_off_ack) {
            c->drg_off_ack = 1;
            printf("camp_dereg radio_off response=yes error_raw=%u\n", error);
            if (error) c->drg_done = 1;
            return 1;
        }
        if (c->drg_on_sent && !c->drg_on_ack) {
            c->drg_on_ack = 1;
            printf("camp_dereg radio_on response=yes error_raw=%u\n", error);
            if (error) c->drg_done = 1;
            return 1;
        }
        return 0;
    }
    if (id == REG_SCAN && c->drg_scan_sent && !c->drg_scan_got && error) {
        c->drg_scan_got = 1;
        c->drg_done = 1;
        printf("camp_dereg scan response=yes error_raw=%u\n", error);
        return 1;
    }
    if (id == REG_MANUAL_SEL && c->drg_manual_sent && !c->drg_manual_ack) {
        c->drg_manual_ack = 1;
        c->drg_done = 1;
        printf("camp_dereg manual_select response=yes error_raw=%u\n", error);
        return 1;
    }
    return 0;
}

/* VERDICT 16 activation-call sub-sequence, run only after the normal bring-up
 * completes (reg_complete), only when armed by /data/saaios/etc/call_number, and
 * only once CS/voice registration is achieved (state 1=home or 5=roaming; never
 * when denied/searching). Steps: DIAL once -> poll the call list a few times to
 * observe progression -> HANGUP once. One activation attempt; never redials. A
 * CLL_NONE return means "waiting for the pending reply". */
static unsigned camp_call_next(const struct camp_driver *c)
{
    if (!c->reg_complete || !c->call_enabled || !c->call_number[0] || c->call_done)
        return CLL_NONE;
    if (!c->voice_reg_known) return CLL_NONE;
    if (!(c->voice_reg_state == 1 || c->voice_reg_state == 5)) return CLL_NONE;
    if (!c->call_dial_sent) return CLL_DIAL;
    if (!c->call_dialed_ok) return CLL_NONE;
    if (c->call_poll_count < CALL_POLL_MAX) return CLL_POLL;
    if (!c->call_hangup_sent) return CLL_HANGUP;
    return CLL_NONE;
}

/* One SMS after CS registration. Absent file, a read-only capability boot,
 * or a finished attempt stays idle. */
static unsigned camp_sms_next(const struct camp_driver *c)
{
    if (!c->reg_complete || c->capquery || !c->sms_enabled ||
        !c->sms_number[0] || c->sms_done || c->sms_sent)
        return 0;
    if (!c->voice_reg_known) return 0;
    if (!(c->voice_reg_state == 1 || c->voice_reg_state == 5)) return 0;
    return SMS_SEND;
}

static int camp_sms_reply(struct camp_driver *c, unsigned id, unsigned error)
{
    if (id != SMS_SEND || !c->sms_sent || c->sms_done) return 0;
    c->sms_done = 1;
    printf("camp_sms response=yes error_raw=%u\n", error);
    return 1;
}

/* One header-only read after registration. The status byte is published
 * as 0 or 1. */
static unsigned camp_volteprov_next(const struct camp_driver *c)
{
    if (!c->reg_complete || c->capquery || c->volteprov_done ||
        c->volteprov_sent)
        return 0;
    return VOLTE_PROV_GET;
}

static int camp_volteprov_reply(struct camp_driver *c, const uint8_t *p,
                                size_t n, unsigned id, unsigned error)
{
    unsigned status;
    if (id != VOLTE_PROV_GET || !c->volteprov_sent || c->volteprov_done)
        return 0;
    c->volteprov_done = 1;
    if (!error && p && n > VOLTE_PROV_STATUS_OFF) {
        status = p[VOLTE_PROV_STATUS_OFF] ? 1u : 0u;
        printf("camp_volteprov response=yes error_raw=0 status=%u\n", status);
    } else {
        printf("camp_volteprov response=yes error_raw=%u\n", error);
    }
    return 1;
}

static void make_emquery_request(uint8_t request[EM_QUERY_LEN], uint32_t token)
{
    memset(request, 0, EM_QUERY_LEN);
    request[2] = (uint8_t)EM_QUERY;
    request[3] = (uint8_t)(EM_QUERY >> 8);
    request[4] = (uint8_t)EM_QUERY_LEN;
    put_little32(request + 6, token);
    request[EM_QUERY_MODE_OFF] = EM_QUERY_MODE;
    request[EM_QUERY_ARG_OFF] = (uint8_t)EM_QUERY_ARG;
}

static unsigned camp_emquery_next(const struct camp_driver *c)
{
    if (!c->reg_complete || c->capquery || c->emquery_done || c->emquery_sent)
        return 0;
    return EM_QUERY;
}

static int camp_emquery_reply(struct camp_driver *c, unsigned id, unsigned error)
{
    if (id != EM_QUERY || !c->emquery_sent || c->emquery_done) return 0;
    c->emquery_done = 1;
    printf("camp_emquery response=yes error_raw=%u\n", error);
    return 1;
}

static unsigned camp_dcall_next(const struct camp_driver *c)
{
    if (!c->reg_complete || c->capquery || c->dcall_done || c->dcall_sent)
        return 0;
    return DCALL_LIST;
}

static int camp_dcall_reply(struct camp_driver *c, const uint8_t *p, size_t n,
                            unsigned id, unsigned error)
{
    if (id != DCALL_LIST || !c->dcall_sent || c->dcall_done) return 0;
    c->dcall_done = 1;
    if (!error && p && n > (size_t)DCALL_COUNT_OFF) {
        c->dcall_count = p[DCALL_COUNT_OFF];
        printf("camp_dcall response=yes error_raw=0 count=%u\n",
               p[DCALL_COUNT_OFF]);
    } else {
        c->dcall_count = -1;
        printf("camp_dcall response=yes error_raw=%u\n", error);
    }
    return 1;
}

/* Advance the PIN-unlock and activation-call sequences on a matching reply.
 * Returns 1 when the reply was consumed here. The PIN never reaches the log;
 * only the public error word, call state/index, and counts do. */
static int camp_call_reply(struct camp_driver *c, const uint8_t *p, size_t n,
                           unsigned id, unsigned error)
{
    if (id == CALL_PIN_VERIFY && c->pin_verify_sent && !c->pin_verified) {
        c->pin_verified = 1;
        c->pin_required = 0;
        printf("camp_sim pin_verify response=yes error_raw=%u\n", error);
        if (!error) c->sim_change_pending = 1; /* re-read SIM status -> READY */
        return 1;
    }
    if (id == CALL_DIAL && c->call_dial_sent && !c->call_dialed_ok &&
        !c->call_done) {
        printf("camp_call dial response=yes error_raw=%u\n", error);
        if (error) c->call_done = 1;
        else c->call_dialed_ok = 1;
        return 1;
    }
    if (id == CALL_HANGUP && c->call_hangup_sent && !c->call_done) {
        c->call_done = 1;
        printf("camp_call hangup response=yes error_raw=%u\n", error);
        return 1;
    }
    if (id == CALL_LIST && c->call_dialed_ok && !c->call_done) {
        if (!error && n >= (size_t)CALL_LIST_COUNT_OFF + 4u) {
            unsigned cnt = little32(p + CALL_LIST_COUNT_OFF);
            c->call_seen_count = (int)cnt;
            if (cnt >= 1 &&
                n >= (size_t)CALL_LIST_ENTRY_OFF + CALL_LIST_INDEX_OFF + 4u) {
                c->call_last_state =
                    p[CALL_LIST_ENTRY_OFF + CALL_LIST_STATE_OFF];
                c->call_index = (int)little32(
                    p + CALL_LIST_ENTRY_OFF + CALL_LIST_INDEX_OFF);
            }
            printf("camp_call list count=%u state_raw=%u index=%d "
                   "error_raw=%u\n", cnt, c->call_last_state, c->call_index,
                   error);
        } else {
            printf("camp_call list status=short_or_error error_raw=%u\n", error);
        }
        return 1;
    }
    return 0;
}

/* VERDICT 17 len-16 scanType sweep: after bring-up completes and only when armed
 * (/data/saaios/etc/scan16), fire BuildQueryAvailableNetwork(int) once per
 * distinct accepted scanType 0..5, stopping early if any returns a result list.
 * Returns REG_SCAN16 to fire the next mode, or 0 while idle/awaiting/done. */
static unsigned camp_scan16_next(const struct camp_driver *c)
{
    if (!c->reg_complete || !c->scan16_enabled || c->scan16_done) return 0;
    if (c->scan16_sent) return 0;                 /* awaiting reply */
    if (c->scan16_fired >= SCAN16_MODE_COUNT) return 0;
    return REG_SCAN16;
}

/* Consume a len-16 scan reply (opcode 0x0706 while scan16_sent). On error, log
 * and advance to the next scanType; on success, log every PLMN and stop the
 * sweep. Returns 1 when consumed. */
static int camp_scan16_reply(struct camp_driver *c, const uint8_t *p, size_t n,
                             unsigned id, unsigned error)
{
    if (id != REG_SCAN16 || !c->scan16_sent) return 0;
    c->scan16_sent = 0;
    c->scan16_fired++;
    if (!error) {
        int seen = 0;
        c->scan16_got_list = 1;
        c->scan16_done = 1;   /* a list came back -- stop sweeping */
        sit_net_log_available("camp_scan16", p, n, c->manual_plmn, &seen);
        printf("camp_scan16 mode=%d result=list target_visible=%d\n",
               c->scan16_mode, seen);
    } else {
        printf("camp_scan16 mode=%d response=yes error_raw=%u\n",
               c->scan16_mode, error);
        if (c->scan16_fired >= SCAN16_MODE_COUNT) c->scan16_done = 1;
    }
    return 1;
}

/* VERDICT 21: read the running CP baseband/SW version once (read-only). */
static unsigned camp_bbver_next(const struct camp_driver *c)
{
    if (!c->reg_complete || !c->bbver_enabled || c->bbver_done) return 0;
    if (c->probe_pending) return 0;
    if (!c->bbver_sent) return BBVER_GET;
    return 0;
}

/* Consume the GET_BASEBAND_VERSION reply (opcode 0x0901). Logs the SW version
 * build string only (a firmware id, not a secret). Returns 1 if handled. */
static int camp_bbver_reply(struct camp_driver *c, const uint8_t *p, size_t n,
                            unsigned id, unsigned error)
{
    if (id != BBVER_GET || !c->bbver_enabled || !c->bbver_sent) return 0;
    c->bbver_done = 1;
    if (!error && n > (size_t)BBVER_SWVER_OFF) {
        char v[BBVER_SWVER_MAX + 1];
        size_t i = 0;
        for (; i < (size_t)BBVER_SWVER_MAX &&
               (size_t)BBVER_SWVER_OFF + i < n; i++) {
            uint8_t ch = p[BBVER_SWVER_OFF + i];
            if (ch == 0) break;
            v[i] = (ch >= 0x20 && ch < 0x7f) ? (char)ch : '.';
        }
        v[i] = 0;
        printf("camp_bbver sw_version=%s error_raw=%u\n", v, error);
    } else {
        printf("camp_bbver response=yes error_raw=%u len=%zu\n", error, n);
    }
    return 1;
}

/* The single 0x0734 is dispatched from the radio-cycle scan step, not from
 * here, so a camped bring-up cannot send it early. */
static unsigned camp_scan734_next(const struct camp_driver *c)
{
    (void)c;
    return 0;
}

/* Consume the 0x0734 ACK (accepted/refused). Scan results arrive separately as
 * unsolicited 0x0736 notifications (see camp_scan734_observe). Returns 1 if
 * handled. error_raw 0 = scan accepted; nonzero = refused (e.g. GENERIC). */
static int camp_scan734_reply(struct camp_driver *c, unsigned id, unsigned error)
{
    if (id != SCAN734_GET || !c->scan734 || !c->scan734_sent) return 0;
    c->scan734_done = 1;
    if (c->drg_scan_sent && !c->drg_scan_got) {
        c->drg_scan_got = 1;
        c->drg_done = 1;
    }
    printf("camp_scan734 ack=yes error_raw=%u precamp=1\n", error);
    return 1;
}

/* Record unsolicited 0x0736 scan-result notifications. Logs the scanStatus byte
 * (frame+8, per ProtocolNetScanResultAdapter::GetScanStatus) and a bounded hex
 * dump of the frame header/first cell fields so PLMN/band/EARFCN can be read
 * back offline -- those are network identifiers, not subscriber secrets. */
static void camp_scan734_observe(struct camp_driver *c, const uint8_t *p,
                                 size_t n, int64_t now)
{
    if (!c->scan734 || n < 13 || p[0] != 2) return;
    if (little16(p + 2) != SCAN734_RESULT || little16(p + 4) != n) return;
    unsigned status = p[8];
    size_t dump = n < 96 ? n : 96;   /* header + leading cell fields only */
    char hex[96 * 2 + 1];
    for (size_t i = 0; i < dump; i++)
        snprintf(hex + i * 2, 3, "%02x", p[i]);
    printf("camp_scan734 result=yes scan_status=%u len=%zu elapsed_ms=%lld "
           "head=%s\n",
           status, n, (long long)(now - c->owner_start_ms), hex);
}

/* VERDICT 20: modern RAT-gate experiment. Runs only when armed by
 * /data/saaios/etc/ratbm and after reg_complete. Sequence (each once):
 * GET 0x750 (diagnostic read of current allowed-RAT bitmap) -> band-mode GET
 * 0x709 -> SET 0x074f with an LTE-inclusive bitmap -> GET 0x750 (confirm).
 * Returns the next opcode to fire, or 0 while idle/awaiting/done. */
static unsigned camp_ratbm_next(const struct camp_driver *c)
{
    if (!c->reg_complete || !c->ratbm_enabled || c->ratbm_done) return 0;
    if (c->probe_pending) return 0;             /* awaiting a reply */
    if (!c->ratbm_get1_sent) return RATBM_GET;          /* before: read */
    if (!c->ratbm_band_sent) return RATBM_BANDMODE_GET; /* before: band */
    if (c->capquery) return 0;          /* VERDICT 25: read-only, no SET/confirm */
    if (!c->ratbm_set_sent) return RATBM_SET;           /* arm LTE bitmap */
    if (!c->ratbm_get2_sent) return RATBM_GET;          /* after: confirm */
    return 0;
}

/* Consume a RAT-gate reply (0x750 read-back, 0x709 band mode, or 0x074f SET).
 * PLMN/RAT scalars and the public error word only. Returns 1 if handled. */
static int camp_ratbm_reply(struct camp_driver *c, const uint8_t *p, size_t n,
                            unsigned id, unsigned error)
{
    if (!c->ratbm_enabled) {
        /* The stock bitmap is also sent once on the default boot, with
         * the file gate left off. Log only the public wire constant. */
        if (id == RATBM_SET && c->ratbm_set_sent) {
            printf("camp_ratbm set=allowed_bitmap response=yes error_raw=%u "
                   "wire=0x%x\n", error, (unsigned)RATBM_WIRE_STOCK);
            return 1;
        }
        return 0;
    }
    if (id == RATBM_GET && (c->ratbm_get1_sent || c->ratbm_get2_sent)) {
        const char *when = c->ratbm_set_sent ? "after" : "before";
        if (!error && n >= (size_t)RATBM_OFF + 4u) {
            uint32_t w = little32(p + RATBM_OFF);
            printf("camp_ratbm get=allowed_bitmap when=%s wire=0x%x lte=%u "
                   "wcdma=%u gsm=%u tdscdma=%u nr=%u error_raw=%u\n",
                   when, w, (w >> 7) & 1u, (w >> 3) & 1u, (w >> 9) & 1u,
                   (w >> 10) & 1u, (w >> 18) & 1u, error);
        } else {
            printf("camp_ratbm get=allowed_bitmap when=%s response=yes "
                   "error_raw=%u len=%zu\n", when, error, n);
        }
        if (c->ratbm_get2_sent) c->ratbm_done = 1;   /* read-back complete */
        return 1;
    }
    if (id == RATBM_BANDMODE_GET && c->ratbm_band_sent) {
        /* VERDICT 25: dump the CP's available-band payload verbatim so the band
         * list can be decoded offline against the SIT band enum. Band IDs are
         * radio capability, not secrets; no bytes are invented. */
        char hex[3 * 48 + 1];
        size_t off = n > 12 ? 12 : n, h = 0;
        for (size_t i = off; i < n && h + 3 < sizeof hex; i++)
            h += (size_t)snprintf(hex + h, sizeof hex - h, "%02x ", p[i]);
        hex[h] = 0;
        printf("camp_ratbm get=band_mode response=yes error_raw=%u len=%zu "
               "paylen=%zu payload=%s\n", error, n, n > 12 ? n - 12 : 0, hex);
        if (c->capquery) c->ratbm_done = 1;   /* read-only sequence complete */
        return 1;
    }
    if (id == RATBM_SET && c->ratbm_set_sent) {
        printf("camp_ratbm set=allowed_bitmap response=yes error_raw=%u "
               "wire=0x%x\n", error, (unsigned)RATBM_WIRE_STOCK);
        return 1;
    }
    return 0;
}

/* sit_pdp_data_call_item in a SetupDataCall response, from
 * ProtocolPsSetupDataCallAdapter::Init in libsitril. Payload starts at
 * byte 12: cid at +2, PDP type at +4 (1 or 3 carries IPv4), IPv4 at +5.
 * Stock maps cid 2 to rmnet1, so the interface is rmnet(cid-1). */
static int camp_data_call_v4(const uint8_t *p, size_t n,
                             unsigned *ifindex, uint8_t addr[4])
{
    unsigned cid, ptype;
    if (n < 21) return -1;
    cid = p[14];
    ptype = p[16];
    if (cid < 1 || cid > 16) return -1;
    if (ptype != 1 && ptype != 3) return -1;
    if ((p[17] | p[18] | p[19] | p[20]) == 0) return -1;
    if (p[17] == 0xff && p[18] == 0xff && p[19] == 0xff && p[20] == 0xff)
        return -1;
    *ifindex = cid - 1;
    memcpy(addr, p + 17, 4);
    return 0;
}

/* DNS family is item+25. Types 1 and 3 carry IPv4 at item+26 and item+46,
 * the same word loads as the interface address. */
static int v4_present(const uint8_t *a)
{
    if ((a[0] | a[1] | a[2] | a[3]) == 0) return 0;
    if (a[0] == 0xff && a[1] == 0xff && a[2] == 0xff && a[3] == 0xff) return 0;
    return 1;
}

static int camp_data_call_dns(const uint8_t *p, size_t n, uint8_t dns[][4],
                              int *ndns)
{
    *ndns = 0;
    if (n < 62) return -1;
    if (p[37] != 1 && p[37] != 3) return -1;
    if (v4_present(p + 38)) {
        memcpy(dns[*ndns], p + 38, 4);
        *ndns += 1;
    }
    if (v4_present(p + 58)) {
        memcpy(dns[*ndns], p + 58, 4);
        *ndns += 1;
    }
    return *ndns ? 0 : -1;
}

/* IPv6 is the 16 bytes at item+9. Types 2 and 3 store it. SetIfAddrIpv6
 * uses prefix length 64. */
static int v6_present(const uint8_t *a)
{
    int i, any = 0, all = 1;
    for (i = 0; i < 16; i++) {
        if (a[i]) any = 1;
        if (a[i] != 0xff) all = 0;
    }
    return any && !all;
}

static int camp_data_call_v6(const uint8_t *p, size_t n,
                             unsigned *ifindex, uint8_t addr[16])
{
    unsigned cid, ptype;
    if (n < 37) return -1;
    cid = p[14];
    ptype = p[16];
    if (cid < 1 || cid > 16) return -1;
    if (ptype != 2 && ptype != 3) return -1;
    if (!v6_present(p + 21)) return -1;
    *ifindex = cid - 1;
    memcpy(addr, p + 21, 16);
    return 0;
}

/* DNS family 2 and 3 also carry IPv6 at item+30 and item+50. Init copies
 * those 16-byte loads only for those families. */
static int camp_data_call_dns6(const uint8_t *p, size_t n, uint8_t dns[][16],
                               int *ndns)
{
    *ndns = 0;
    if (n < 78) return -1;
    if (p[37] != 2 && p[37] != 3) return -1;
    if (v6_present(p + 42)) {
        memcpy(dns[*ndns], p + 42, 16);
        *ndns += 1;
    }
    if (v6_present(p + 62)) {
        memcpy(dns[*ndns], p + 62, 16);
        *ndns += 1;
    }
    return *ndns ? 0 : -1;
}

#ifndef RFS_HOST_TEST
static void camp_apply_v4(unsigned ifindex, const uint8_t addr[4])
{
    char cmd[96];
    int up, add;
    snprintf(cmd, sizeof cmd, "ip link set rmnet%u up", ifindex);
    up = system(cmd);
    snprintf(cmd, sizeof cmd, "ip addr add %u.%u.%u.%u/32 dev rmnet%u",
             addr[0], addr[1], addr[2], addr[3], ifindex);
    add = system(cmd);
    snprintf(cmd, sizeof cmd, "ip route replace default dev rmnet%u", ifindex);
    int route = system(cmd);
    zero_bytes(cmd, sizeof cmd);
    printf("camp_setup if=rmnet%u ipv4=yes prefix=32 up=%d add=%d route=%d\n",
           ifindex, up == 0, add == 0, route == 0);
}

static void camp_write_dns(uint8_t dns[][4], int n)
{
    FILE *f = fopen("/run/resolv.conf", "w");
    int i;
    if (!f) {
        printf("camp_setup dns=no\n");
        return;
    }
    for (i = 0; i < n; i++)
        fprintf(f, "nameserver %u.%u.%u.%u\n",
                dns[i][0], dns[i][1], dns[i][2], dns[i][3]);
    fclose(f);
    printf("camp_setup dns=yes count=%d\n", n);
}

static void camp_write_dns6(uint8_t dns[][16], int n)
{
    FILE *f = fopen("/run/resolv.conf", "a");
    int i;
    if (!f) {
        printf("camp_setup dns6=no\n");
        return;
    }
    for (i = 0; i < n; i++)
        fprintf(f, "nameserver "
                "%02x%02x:%02x%02x:%02x%02x:%02x%02x:"
                "%02x%02x:%02x%02x:%02x%02x:%02x%02x\n",
                dns[i][0], dns[i][1], dns[i][2], dns[i][3],
                dns[i][4], dns[i][5], dns[i][6], dns[i][7],
                dns[i][8], dns[i][9], dns[i][10], dns[i][11],
                dns[i][12], dns[i][13], dns[i][14], dns[i][15]);
    fclose(f);
    printf("camp_setup dns6=yes count=%d\n", n);
}

static void camp_apply_v6(unsigned ifindex, const uint8_t addr[16])
{
    char cmd[160];
    int up, add, route;
    snprintf(cmd, sizeof cmd, "ip link set rmnet%u up", ifindex);
    up = system(cmd);
    snprintf(cmd, sizeof cmd,
             "ip -6 addr add "
             "%02x%02x:%02x%02x:%02x%02x:%02x%02x:"
             "%02x%02x:%02x%02x:%02x%02x:%02x%02x/64 dev rmnet%u",
             addr[0], addr[1], addr[2], addr[3],
             addr[4], addr[5], addr[6], addr[7],
             addr[8], addr[9], addr[10], addr[11],
             addr[12], addr[13], addr[14], addr[15], ifindex);
    add = system(cmd);
    snprintf(cmd, sizeof cmd, "ip -6 route replace default dev rmnet%u",
             ifindex);
    route = system(cmd);
    zero_bytes(cmd, sizeof cmd);
    printf("camp_setup if=rmnet%u ipv6=yes prefix=64 up=%d add=%d route=%d\n",
           ifindex, up == 0, add == 0, route == 0);
}
#endif

/* Cell-type byte -> barring prefix length. libsitril rodata 0xd8958.
 * Parser versions 3 and 4 return the same sizes: GSM 150, CDMA 21,
 * LTE 282, WCDMA 258, TD-SCDMA 258, NR 282. Other bytes are the
 * stock failure path. */
static unsigned camp_bar_prefix(unsigned cell)
{
    static const unsigned prefix[6] = {150, 21, 282, 258, 258, 282};
    return cell < 6u ? prefix[cell] : 0u;
}

/* FillCellIdentityLte keeps a word only when it passes the stock
 * bound, and stores 0x7fffffff otherwise. which: 0 CI (28 bits),
 * 1 PCI (<=503), 2 TAC (16 bits), 3 EARFCN (18 bits). */
static int camp_lte_id(uint32_t word, int which)
{
    int ok = which == 0 ? (word >> 28) == 0 :
             which == 1 ? word <= 0x1f7u :
             which == 2 ? (word >> 16) == 0 :
                          (word >> 18) == 0;
    return ok ? (int)word : 0x7fffffff;
}

/* Match a prober reply or note a SIM-status-changed indication. Only scalar
 * status fields and the public error word ever reach the log. */
static void camp_probe_match(struct camp_driver *c, const uint8_t *p, size_t n,
                             int64_t now)
{
    (void)now;
    if (n >= 8 && p[0] == 2 && little16(p + 2) == 0x0210) {
        if (!c->sim_ready) c->sim_change_pending = 1;
        return;
    }
    if (n >= 8 && p[0] == 2) {
        unsigned ind = little16(p + 2);
        const char *ind_name = NULL;
        int *ind_seen = NULL;
        if (ind == 0x0604) {
            ind_name = "datacall";
            ind_seen = &c->ind_datacall;
        } else if (ind == 0x0906 && n >= 10) {
            ind_name = "signal";
            ind_seen = &c->ind_signal;
        } else if (ind == 0x0945 && n >= 24) {
            ind_name = "linkcap";
            ind_seen = &c->ind_linkcap;
        } else if (ind == 0x0742 && n >= 74) {
            /* FillPhysicalChannelConfigV1_6: length 1004 takes this
             * path. Record stride is 62, the first record starts at
             * +12. Build (HAL > 0x15) names the words status, rat,
             * downlink/uplink channel, downlink/uplink bandwidth,
             * physical cell id and band. */
            ind_name = "phy";
            ind_seen = &c->ind_phy;
        } else if (ind == 0x0720 && n >= 41) {
            /* OnAcBarringInfo logs five bytes: forEmc, forMoSig,
             * forMoData, forMmtelVoice, forMmtelVideo. */
            ind_name = "acbar";
            ind_seen = &c->ind_acbar;
        } else if (ind == 0x074b && n >= 311) {
            /* DecodingBarringInfos: type byte, then a cell-identity
             * prefix, then a count word and 17-byte records. */
            ind_name = "barring";
            ind_seen = &c->ind_barring;
        }
        if (ind_seen && !*ind_seen) {
            *ind_seen = 1;
            if (ind == 0x0906) {
                c->ind_signal_mask = (int)(little16(p + 8) & 0x7fu);
                printf("camp_ind signal id=0x0906 len=%zu mask_low7=%d\n",
                       n, c->ind_signal_mask);
            } else if (ind == 0x0945) {
                c->link_dl = (int)little32(p + 8);
                c->link_ul = (int)little32(p + 12);
                c->link_dl2 = (int)little32(p + 16);
                c->link_ul2 = (int)little32(p + 20);
                printf("camp_ind linkcap id=0x0945 len=%zu dl=%d ul=%d "
                       "dl2=%d ul2=%d\n",
                       n, c->link_dl, c->link_ul, c->link_dl2, c->link_ul2);
            } else if (ind == 0x0604 && n >= 9) {
                unsigned count = p[8];
                c->ind_dc_count = (int)count;
                if (count >= 1 && count <= 16 &&
                    (n - 9) == (size_t)count * 292u) {
                    c->ind_dc_cid = p[11];
                    c->ind_dc_active = p[12];
                    c->ind_dc_pdp = p[13];
                    printf("camp_ind datacall id=0x0604 len=%zu count=%u "
                           "cid=%u active=%u pdp=%u\n",
                           n, count, p[11], p[12], p[13]);
                } else {
                    printf("camp_ind datacall id=0x0604 len=%zu count=%u\n",
                           n, count);
                }
            } else if (ind == 0x0742) {
                int count = (int)little32(p + 8);
                c->ind_phy_count = count;
                if (count >= 1) {
                    c->ind_phy_status = p[12];
                    c->ind_phy_rat = (int)sit_net_rat_map(p[17]);
                    c->ind_phy_dl_bw = (int)little32(p + 13);
                    c->ind_phy_dl_ch = (int)little32(p + 19);
                    c->ind_phy_pci = (int)little32(p + 60);
                    c->ind_phy_ul_ch = (int)little32(p + 64);
                    c->ind_phy_ul_bw = (int)little32(p + 68);
                    c->ind_phy_band = (int)(int16_t)little16(p + 72);
                    printf("camp_ind phy id=0x0742 len=%zu count=%d "
                           "status=%d rat=%d dl_ch=%d ul_ch=%d dl_bw=%d "
                           "ul_bw=%d pci=%d band=%d\n",
                           n, count, c->ind_phy_status, c->ind_phy_rat,
                           c->ind_phy_dl_ch, c->ind_phy_ul_ch,
                           c->ind_phy_dl_bw, c->ind_phy_ul_bw,
                           c->ind_phy_pci, c->ind_phy_band);
                } else {
                    printf("camp_ind phy id=0x0742 len=%zu count=%d\n",
                           n, count);
                }
            } else if (ind == 0x0720) {
                c->ac_emc = p[8];
                c->ac_mosig = p[9];
                c->ac_modata = p[17];
                c->ac_voice = p[25];
                c->ac_video = p[33];
                printf("camp_ind acbar id=0x0720 len=%zu emc=%d mo_sig=%d "
                       "mo_data=%d voice=%d video=%d\n",
                       n, c->ac_emc, c->ac_mosig, c->ac_modata,
                       c->ac_voice, c->ac_video);
            } else if (ind == 0x074b) {
                unsigned cell = p[8];
                unsigned prefix = camp_bar_prefix(cell);
                c->bar_cell = (int)cell;
                if (prefix != 0 && n >= (size_t)8u + prefix + 21u) {
                    int count = (int)little32(p + 8 + prefix);
                    c->bar_count = count;
                    if (count >= 1) {
                        char list[512];
                        size_t used = 0;
                        int logged = 0;
                        int limit = count > 8 ? 8 : count;
                        list[0] = 0;
                        for (int i = 0; i < limit; i++) {
                            size_t start = (size_t)8u + prefix + 4u +
                                           (size_t)i * 17u;
                            int svc, kind, factor, time, barred, wrote;
                            const uint8_t *r;
                            if (start + 17u > n) break;
                            r = p + start;
                            svc = (int)little32(r);
                            kind = (int)little32(r + 4);
                            factor = (int)little32(r + 8);
                            time = (int)little32(r + 12);
                            barred = r[16] != 0;
                            if (i == 0) {
                                c->bar_svc = svc;
                                c->bar_kind = kind;
                                c->bar_factor = factor;
                                c->bar_time = time;
                                c->bar_barred = barred;
                            } else if (i == 1) {
                                c->bar2_svc = svc;
                                c->bar2_kind = kind;
                                c->bar2_factor = factor;
                                c->bar2_time = time;
                                c->bar2_barred = barred;
                            }
                            wrote = snprintf(list + used, sizeof list - used,
                                             "%s%d/%d/%d/%d/%d",
                                             used ? "," : "",
                                             svc, kind, factor, time, barred);
                            if (wrote < 0 || (size_t)wrote >= sizeof list - used)
                                break;
                            used += (size_t)wrote;
                            logged++;
                        }
                        c->bar_nlogged = logged;
                        if (cell == 2) {
                            const uint8_t *id = p + 9;
                            c->bar_ci = camp_lte_id(little32(id + 6), 0);
                            c->bar_pci = camp_lte_id(little32(id + 10), 1);
                            c->bar_tac = camp_lte_id(little32(id + 14), 2);
                            c->bar_earfcn = camp_lte_id(little32(id + 18), 3);
                            printf("camp_ind barring id=0x074b len=%zu cell=%u "
                                   "count=%d ci=%d pci=%d tac=%d earfcn=%d "
                                   "recs=%s\n",
                                   n, cell, count, c->bar_ci, c->bar_pci,
                                   c->bar_tac, c->bar_earfcn, list);
                        } else {
                            printf("camp_ind barring id=0x074b len=%zu cell=%u "
                                   "count=%d recs=%s\n",
                                   n, cell, count, list);
                        }
                    } else {
                        printf("camp_ind barring id=0x074b len=%zu cell=%u "
                               "count=%d\n",
                               n, cell, count);
                    }
                } else {
                    printf("camp_ind barring id=0x074b len=%zu cell=%u\n",
                           n, cell);
                }
            } else {
                printf("camp_ind %s id=0x%04x len=%zu\n", ind_name, ind, n);
            }
        }
    }
    if (!c->probe_pending || n < 12 || p[0] != 1 ||
        little16(p + 4) != n || little32(p + 6) != c->probe_token ||
        little16(p + 2) != c->probe_id)
        return;
    c->probe_pending = 0;
    c->probe_replies++;
    unsigned id = c->probe_id;
    unsigned error = little16(p + 10);
    if (camp_dereg_reply(c, id, error)) return;
    if (camp_scan16_reply(c, p, n, id, error)) return;
    if (camp_bbver_reply(c, p, n, id, error)) return;
    if (camp_scan734_reply(c, id, error)) return;
    if (camp_ratbm_reply(c, p, n, id, error)) return;
    if (camp_dcall_reply(c, p, n, id, error)) return;
    if (camp_emquery_reply(c, id, error)) return;
    if (camp_volteprov_reply(c, p, n, id, error)) return;
    if (camp_sms_reply(c, id, error)) return;
    if (camp_call_reply(c, p, n, id, error)) return;
    if (id == 0x073e || id == 0x0954 || id == 0x0718) {
        const char *step = id == 0x073e ? "endc" :
                           id == 0x0954 ? "vonrcapa" : "rcnet";
        printf("camp_%s response=yes error_raw=%u len=%zu\n", step, error, n);
        return;
    }
    if (id == 0x094d || id == 0x0928 || id == 0x0902) {
        const char *step = id == 0x094d ? "throttle" :
                           id == 0x0928 ? (c->probe_name ? c->probe_name : "unsol") :
                           "screen";
        printf("camp_%s response=yes error_raw=%u len=%zu\n", step, error, n);
        return;
    }
    if (id == 0x070c || id == 0x0108) {
        const char *step = id == 0x070c ? "cellinfo" : "smsc";
        printf("camp_%s response=yes error_raw=%u len=%zu\n", step, error, n);
        return;
    }
    if (id == 0x0953) {
        printf("camp_vonrget response=yes error_raw=%u len=%zu\n", error, n);
        return;
    }
    if (id == 0x0615) {
        if (!error && n >= 16)
            printf("camp_phonecap response=yes error_raw=0 len=%zu "
                   "pay=%u,%u,%u,%u\n",
                   n, p[12], p[13], p[14], p[15]);
        else
            printf("camp_phonecap response=yes error_raw=%u len=%zu\n",
                   error, n);
        return;
    }
    if (id == 0x0212 || id == 0x4605) {
        const char *step = id == 0x0212 ? "atr" : "svn";
        printf("camp_%s response=yes error_raw=%u len=%zu\n", step, error, n);
        return;
    }
    if (id == 0x0949 || id == 0x090b || id == 0x0903 ||
        id == 0x0711 || id == 0x0740 || id == 0x024d ||
        id == 0x0943 || id == 0x0107 || id == 0x0944 || id == 0x0106 ||
        id == 0x0000 || id == 0x0c20 || id == 0x0c33 || id == 0x0755) {
        const char *step = id == 0x0949 ? "aptime" :
                           id == 0x090b ? "dbgtrace" :
                           id == 0x0903 ? "tty" :
                           id == 0x0711 ? "pssvc" :
                           id == 0x0740 ? "prefmodem" :
                           id == 0x024d ? "slot" :
                           id == 0x0943 ? "sigcrit" :
                           id == 0x0107 ? "smsact" :
                           id == 0x0944 ? "linkcrit" :
                           id == 0x0106 ? "smscb" :
                           id == 0x0000 ? "calllist" :
                           id == 0x0c20 ? "gpslock" :
                           id == 0x0c33 ? "gpsnfw" : "samode";
        if (id == 0x0000 && !error && n >= 16) {
            printf("camp_calllist response=yes error_raw=0 len=%zu count=%u\n",
                   n, little32(p + 12));
            return;
        }
        printf("camp_%s response=yes error_raw=%u len=%zu\n", step, error, n);
        return;
    }
    if (id == 0x0d3c || id == 0x0d3a || id == 0x0d3b) {
        printf("camp_%s response=yes error_raw=%u len=%zu\n",
               c->probe_name, error, n);
        return;
    }
    if (id == 0x090c) {
        printf("camp_activity response=yes error_raw=%u len=%zu\n", error, n);
        return;
    }
    if (id == 0x0605) {
        printf("camp_fastdorm response=yes error_raw=%u len=%zu\n", error, n);
        return;
    }
    if (id == 0x0625 || id == 0x0613 || id == 0x0600) {
        const char *step = id == 0x0625 ? "vonr" :
                           id == 0x0613 ? c->probe_name : "setup";
        unsigned ifindex = 0;
        unsigned ifindex6 = 0;
        uint8_t addr[4];
        uint8_t addr6[16];
        int got4, got6;
        printf("camp_%s response=yes error_raw=%u len=%zu\n", step, error, n);
        got4 = id == 0x0600 && !error &&
               camp_data_call_v4(p, n, &ifindex, addr) == 0;
        got6 = id == 0x0600 && !error &&
               camp_data_call_v6(p, n, &ifindex6, addr6) == 0;
        if (got4 || got6) {
            uint8_t dns[2][4];
            uint8_t dns6[2][16];
            int ndns = 0;
            int ndns6 = 0;
#ifndef RFS_HOST_TEST
            if (got4) {
                camp_apply_v4(ifindex, addr);
                if (camp_data_call_dns(p, n, dns, &ndns) == 0)
                    camp_write_dns(dns, ndns);
            }
            if (got6) camp_apply_v6(ifindex6, addr6);
            if (camp_data_call_dns6(p, n, dns6, &ndns6) == 0)
                camp_write_dns6(dns6, ndns6);
#else
            (void)ifindex;
            (void)ifindex6;
            (void)ndns;
            (void)ndns6;
#endif
            zero_bytes(addr, sizeof addr);
            zero_bytes(addr6, sizeof addr6);
            zero_bytes(dns, sizeof dns);
            zero_bytes(dns6, sizeof dns6);
        }
        return;
    }
    if (error) {
        printf("camp_probe field=%s response=yes error_raw=%u\n",
               c->probe_name, error);
        return;
    }
    if (id == 0x0200) {
        if (!sit_sim_status_complete(p, n)) {
            printf("camp_probe field=sim status=unknown_short\n");
            return;
        }
        unsigned apps = p[SIT_SIM_APPS];
        printf("camp_probe field=sim card_raw=%u apps=%u", p[SIT_SIM_CARD],
               apps);
        if (apps) {
            unsigned app_state = p[SIT_SIM_APP_STATE];
            printf(" app_state_raw=%u pin1_raw=%u", app_state,
                   p[SIT_SIM_PIN1]);
            if (app_state == 5 && !c->sim_ready) {
                c->sim_ready = 1;
                c->pin_required = 0;
                putchar('\n');
                printf("camp_sim=ready app_state_raw=5\n");
                return;
            }
            if (app_state == SIM_APP_STATE_PIN && !c->sim_ready &&
                c->pin[0] && !c->pin_verified) {
                c->pin_required = 1;
                putchar('\n');
                printf("camp_sim pin_required=1 app_state_raw=2\n");
                return;
            }
        }
        putchar('\n');
    } else if (id == 0x0900) {
        uint32_t mask;
        if (camp_signal_mask(p, n, c->probe_token, &mask)) {
            c->mask_seen = 1;
            c->last_mask_low7 = (unsigned)mask;
            printf("camp_probe field=signal mask_low7=%u\n", (unsigned)mask);
        } else {
            printf("camp_probe field=signal status=unknown_short\n");
        }
    } else if (id == OPX_STACK_GET) {
        if (n >= 16) {
            c->opx_stack_known = 1;
            c->opx_stack_mode = little32(p + 12);
            printf("camp_opx get=stack_status mode_raw=%u\n", little32(p + 12));
        } else if (n >= 13) {
            c->opx_stack_known = 1;
            c->opx_stack_mode = p[12];
            printf("camp_opx get=stack_status mode_raw=%u\n", p[12]);
        } else {
            printf("camp_opx get=stack_status status=unknown_short len=%zu\n", n);
        }
    } else if (id == OPX_VOICE_GET) {
        if (n >= 16)
            printf("camp_opx get=voice_operation mode_raw=%u\n", little32(p + 12));
        else
            printf("camp_opx get=voice_operation status=unknown_short\n");
    } else if (id == OPX_DEVSVC_GET) {
        if (n >= 16)
            printf("camp_opx get=device_service mode_raw=%u\n", little32(p + 12));
        else
            printf("camp_opx get=device_service status=unknown_short\n");
    } else if (id == OPX_VOICE_SET || id == OPX_INTPS_SET ||
               id == OPX_STACK_SET || id == OPX_DEVSVC_SET ||
               id == OPX_DUAL_SET) {
        printf("camp_opx set=%s response=yes error_raw=%u\n", c->probe_name,
               error);
    } else if (id == REG_ALLOW_DATA) {
        /* SET acks are short; handle before the generic length guard so the
         * reg_complete gate (and the opx experiment) can actually fire. */
        c->reg_complete = 1;
        printf("camp_reg set=allow_data response=yes error_raw=%u\n", error);
    } else if (id == REG_INIT_ATTACH_APN) {
        printf("camp_reg set=initial_attach_apn response=yes error_raw=%u\n",
               error);
    } else if (id == REG_SEL_AUTO_SET) {
        printf("camp_reg set=selection_auto response=yes error_raw=%u\n", error);
    } else if (id == REG_MANUAL_SEL) {
        printf("camp_reg set=network_selection_manual response=yes "
               "error_raw=%u\n", error);
    } else if (id == REG_PREF_SET) {
        printf("camp_reg set=preferred_lte_wcdma response=yes error_raw=%u\n",
               error);
    } else if (n < (id == SIT_NET_DATA_REG ? 16u : 14u)) {
        printf("camp_probe field=%s status=unknown_short\n", c->probe_name);
    } else if (id == SIT_NET_VOICE_REG || id == SIT_NET_DATA_REG) {
        sit_net_log_regstate("camp_probe", c->probe_name, p, n, id);
        if (id == SIT_NET_VOICE_REG && n > SIT_NET_REG_STATE_OFFSET) {
            c->voice_reg_known = 1;
            c->voice_reg_state = p[SIT_NET_REG_STATE_OFFSET];
        }
        if (id == SIT_NET_DATA_REG && n > SIT_NET_REG_STATE_OFFSET &&
            p[SIT_NET_REG_STATE_OFFSET] == 1)
            c->data_home = 1;
    } else if (id == SIT_NET_OPERATOR) {
        sit_net_log_operator("camp_probe", p, n);
    } else if (id == REG_SCAN) {
        int seen = 0;
        const char *prefix = (c->drg_scan_sent && !c->drg_scan_got)
                                 ? "camp_dereg" : "camp_probe";
        sit_net_log_available(prefix, p, n, c->manual_plmn, &seen);
        if (c->drg_scan_sent && !c->drg_scan_got) {
            c->drg_scan_got = 1;
            c->drg_target_visible = seen;
            if (!(c->manual_plmn[0] && seen)) c->drg_done = 1;
            printf("camp_dereg scan_done target=%s visible=%d\n",
                   c->manual_plmn[0] ? "armed" : "none", seen);
        }
    } else if (id == REG_RADIO_GET) {
        if (n >= 16) {
            c->radio_on = little32(p + 12) == RADIO_STATE_ON;
            printf("camp_reg field=radio radio_raw=%u\n", little32(p + 12));
        } else {
            printf("camp_reg field=radio status=unknown_short\n");
        }
        if (c->drg_off_get_sent && !c->drg_on_sent && !c->radio_on) {
            c->drg_off_confirmed = 1;
            printf("camp_dereg radio_off_confirmed=1\n");
        }
    } else if (id == REG_SEL_GET) {
        if (n >= 13 && p[12] <= 1) {
            c->sel_known = 1;
            c->sel_mode = p[12];
            printf("camp_reg field=selection mode_raw=%u\n", p[12]);
        } else {
            c->sel_known = 1; /* unparseable: treat as auto, do not re-GET */
            c->sel_mode = 0;
            printf("camp_reg field=selection status=unknown_short\n");
        }
    } else if (id == REG_PREF_GET) {
        if (n >= 16) {
            c->pref_known = 1;
            c->preferred_raw = little32(p + 12);
            printf("camp_reg field=preferred preferred_raw=%u\n",
                   little32(p + 12));
        } else {
            c->pref_known = 1;
            c->preferred_raw = 0;
            printf("camp_reg field=preferred status=unknown_short\n");
        }
    } else if (id == REG_SEL_AUTO_SET || id == REG_PREF_SET ||
               id == REG_ALLOW_DATA) {
        /* Handled above, before the generic length guard. */
    }
}

/* Independent bounded framer, separate from the SIT observer's buffer. */
static void camp_feed(struct camp_driver *c, const uint8_t *bytes, size_t len,
                      int64_t now)
{
    if (c->poisoned) return;
    while (len) {
        size_t space = sizeof c->rx - c->used;
        if (!space) { c->poisoned = 1; break; }
        size_t take = len < space ? len : space;
        memcpy(c->rx + c->used, bytes, take);
        c->used += take;
        bytes += take;
        len -= take;
        size_t offset = 0;
        while (offset < c->used) {
            int length = sit_frame_length(c->rx + offset, c->used - offset);
            if (length < 0) { c->poisoned = 1; break; }
            if (!length) break;
            camp_observe(c, c->rx + offset, (size_t)length, now);
            camp_scan734_observe(c, c->rx + offset, (size_t)length, now);
            camp_ack(c, c->rx + offset, (size_t)length);
            camp_probe_match(c, c->rx + offset, (size_t)length, now);
            offset += (size_t)length;
        }
        if (c->poisoned) break;
        if (offset) {
            size_t remaining = c->used - offset;
            memmove(c->rx, c->rx + offset, remaining);
            zero_bytes(c->rx + remaining, offset);
            c->used = remaining;
        }
    }
    if (c->poisoned) { c->used = 0; zero_bytes(c->rx, sizeof c->rx); }
}

/* A short or ambiguous IPC write consumes the request; never resend it. */
static int camp_send_once(int ipc, const uint8_t *frame, size_t len)
{
    sigset_t blocked, old;
    sigemptyset(&blocked);
    sigaddset(&blocked, SIGTERM);
    sigaddset(&blocked, SIGINT);
    if (sigprocmask(SIG_BLOCK, &blocked, &old)) return -1;
    ssize_t written = -1;
    if (!stop_requested) {
#ifdef RFS_HOST_TEST
        if (host_sit_write_override) {
            host_sit_write_calls++;
            written = host_sit_write_override(ipc, frame, len);
        } else
#endif
        written = write(ipc, frame, len);
    }
    int restored = sigprocmask(SIG_SETMASK, &old, NULL);
    return restored == 0 && written == (ssize_t)len ? 0 : -1;
}

/* Dispatch the stock stage-1 trio once, back-to-back on the radio edge. */
static void camp_advance(struct owner *o, int64_t now)
{
    struct camp_driver *c = &o->camp;
    if (c->dispatched || c->poisoned || c->radio_invalidated || stop_requested)
        return;
    /* A failed RFS terminal forbids further modem writes; a successful,
     * acknowledged quarantine (final ACK sent) does not block the IPC camp. */
    if (o->phase == TERMINAL && !o->final_ack_sent) return;
    if (!c->radio_ready0_ms) return;       /* trigger pair not yet armed */
    if (now - c->radio_ready0_ms > CAMP_DISPATCH_MS) {
        c->radio_invalidated = 1;          /* missed the bounded window */
        return;
    }
    if (o->used || o->sit.used || c->used || o->sit.pending)
        return;                            /* mid-frame or GET in flight */
#ifndef RFS_HOST_TEST
    if (cp_state() != CP_ONLINE) return;
#endif
    if (!c->token)
        c->token = (uint32_t)now ^ (uint32_t)getpid() ^ 0x0ca70000u;
    uint32_t cfg_token = ++c->token;
    uint8_t cfg[SEQ_CONFIG_LEN];
    make_setmodemsconfig_request(cfg, cfg_token);
    if (camp_send_once(o->ipc, cfg, sizeof cfg)) {
        c->poisoned = 1;
        puts("camp_dispatch=write_failed step=cfg");
        return;
    }
    c->cfg_token = cfg_token;
    c->cfg_sent = 1;
    uint32_t sgc_token = ++c->token;
    uint8_t sgc[SGC_LEN];
    make_sgc_request(sgc, sgc_token);
    if (camp_send_once(o->ipc, sgc, sizeof sgc)) {
        c->dispatched = 1;
        puts("camp_dispatch=write_failed step=sgc");
        return;
    }
    c->sgc_token = sgc_token;
    c->sgc_sent = 1;
    /* Stock sends SetCpCarrierConfig before RADIO_POWER. Two captured
     * requests, in capture order; only the token is replaced. */
    {
        const uint8_t *srcs[2] = {cc_sit_0, cc_sit_1};
        int i;
        for (i = 0; i < 2; i++) {
            uint8_t cc[532];
            memcpy(cc, srcs[i], sizeof cc);
            put_little32(cc + 6, ++c->token);
            if (camp_send_once(o->ipc, cc, sizeof cc)) {
                c->dispatched = 1;
                puts("camp_dispatch=write_failed step=cc");
                return;
            }
            printf("camp_cc=sent n=%d man=%02x%02x%02x%02x\n", i + 1,
                   cc[276], cc[277], cc[278], cc[279]);
        }
    }
    uint32_t camp_token = ++c->token;
    uint8_t power[CAMP_POWER_LEN];
    make_radiopower_request(power, camp_token, CAMP_POWER_ON);
    if (camp_send_once(o->ipc, power, sizeof power)) {
        c->dispatched = 1;
        puts("camp_dispatch=write_failed step=power");
        return;
    }
    c->camp_token = camp_token;
    c->camp_sent = 1;
    c->dispatched = 1;
    printf("camp_dispatch=sent elapsed_ms=%lld trigger=0x0803-0x0802-raw0"
           " seq=0x093f,0x0404,0x0800 power=on\n",
           (long long)(now - c->owner_start_ms));
}

/* BuildSetApSystemTime stores localtime fields as six bytes at frame+12:
 * tm_year, tm_mon, tm_mday, tm_hour, tm_min, tm_sec. tm_year is years
 * since 1900 and fits in a byte for this century. */
static void camp_ap_time_bytes(uint8_t out[6], const struct tm *tm)
{
    out[0] = (uint8_t)tm->tm_year;
    out[1] = (uint8_t)tm->tm_mon;
    out[2] = (uint8_t)tm->tm_mday;
    out[3] = (uint8_t)tm->tm_hour;
    out[4] = (uint8_t)tm->tm_min;
    out[5] = (uint8_t)tm->tm_sec;
}

/* One initial SetSignalReportCriteria body. Hysteresis 3000 and the
 * following word 2 are the same on every stock frame. Thresholds, the
 * access word at +61 and the two flag bytes are per row. */
struct camp_sig_row {
    int32_t t0, t1, t2, t3;
    uint8_t count;
    uint32_t access;
    uint8_t flag;
    uint8_t on;
};

/* The other nine bodies from the first second of the capture, in order.
 * Rows that appear a second later differ by a single threshold and are
 * the live update, so they are not in this table. */
static const struct camp_sig_row camp_sig_more[] = {
    { -114, -104, -94, -84, 4, 2, 2, 1 },
    { -128, -118, -108, -98, 4, 3, 3, 1 },
    { -105, -90, -75, -65, 4, 5, 1, 1 },
    { 0, 0, 0, 0, 0, 3, 4, 0 },
    { -3, 1, 5, 13, 4, 3, 5, 1 },
    { -110, -90, -80, -65, 4, 4, 6, 1 },
    { 0, 0, 0, 0, 0, 4, 7, 0 },
    { 0, 0, 0, 0, 0, 4, 8, 0 },
    { 0, 0, 0, 0, 0, 2, 9, 0 },
};

static void camp_fill_signal_row(uint8_t *f, const struct camp_sig_row *row)
{
    memset(f, 0, 67);
    f[2] = 0x43;
    f[3] = 0x09;
    f[4] = 67;
    put_little32(f + 12, 3000);
    put_little32(f + 16, 2);
    f[20] = row->count;
    if (row->count) {
        put_little32(f + 21, (uint32_t)row->t0);
        put_little32(f + 25, (uint32_t)row->t1);
        put_little32(f + 29, (uint32_t)row->t2);
        put_little32(f + 33, (uint32_t)row->t3);
    }
    put_little32(f + 61, row->access);
    f[65] = row->flag;
    f[66] = row->on;
}

/* BuildSetSignalReportCriteria: the first stock body (access 1,
 * thresholds -109/-103/-97/-89, flags 1,1). */
static void camp_fill_signal_criteria(uint8_t *f)
{
    static const struct camp_sig_row first = {
        -109, -103, -97, -89, 4, 1, 1, 1
    };
    camp_fill_signal_row(f, &first);
}

/* BuildSetLinkCapaReportCriteria. The body is the same in every stock
 * frame; the word at +186 is the non-zero entry of the access table at
 * libsitril 0xd8914, in table order: 1, 2, 3, 5, 4. */
static const uint32_t camp_link_access[5] = {1, 2, 3, 5, 4};

static void camp_fill_link_criteria(uint8_t *f, uint32_t access)
{
    static const uint32_t up[14] = {
        100, 500, 1000, 5000, 10000, 20000, 50000,
        75000, 100000, 200000, 500000, 1000000, 1500000, 2000000};
    static const uint32_t down[11] = {
        100, 500, 1000, 5000, 10000, 20000, 50000,
        75000, 100000, 200000, 500000};
    int i;
    memset(f, 0, 190);
    f[2] = 0x44;
    f[3] = 0x09;
    f[4] = 190;
    put_little32(f + 12, 3000);
    put_little32(f + 16, 50);
    put_little32(f + 20, 50);
    f[24] = 14;
    for (i = 0; i < 14; i++)
        put_little32(f + 25 + 4 * i, up[i]);
    f[105] = 11;
    for (i = 0; i < 11; i++)
        put_little32(f + 106 + 4 * i, down[i]);
    put_little32(f + 186, access);
}

/* BuildSetBroadcastSmsConfig writes a count byte at +12 and then 7-byte
 * records: service-id from, to, and three trailing bytes. Both stock
 * frames (7 records, then 9) are err 0. Service ids are the public
 * ETWS/CMAS ranges. */
struct camp_smscb_rec {
    uint16_t from_id, to_id;
    uint8_t tail0, tail1, tail2;
};

static const struct camp_smscb_rec camp_smscb_a[7] = {
    {4352, 4354, 0, 255, 1},
    {4355, 4355, 0, 255, 0},
    {4356, 4356, 0, 255, 1},
    {4370, 4379, 0, 255, 1},
    {4380, 4382, 0, 255, 0},
    {4383, 4392, 0, 255, 1},
    {4393, 4395, 0, 255, 0},
};

static const struct camp_smscb_rec camp_smscb_b[9] = {
    {4352, 4354, 0, 255, 1},
    {4355, 4355, 0, 255, 0},
    {4356, 4356, 0, 255, 1},
    {4370, 4371, 0, 255, 1},
    {4373, 4379, 0, 255, 1},
    {4380, 4382, 0, 255, 0},
    {4383, 4384, 0, 255, 1},
    {4386, 4392, 0, 255, 1},
    {4393, 4395, 0, 255, 0},
};

static size_t camp_fill_smscb(uint8_t *f, int which)
{
    const struct camp_smscb_rec *recs =
        which == 0 ? camp_smscb_a : camp_smscb_b;
    int n = which == 0 ? 7 : 9;
    size_t len = (size_t)(13 + n * 7);
    int i;
    memset(f, 0, len);
    f[2] = 0x06;
    f[3] = 0x01;
    f[4] = (uint8_t)len;
    f[12] = (uint8_t)n;
    for (i = 0; i < n; i++) {
        size_t o = (size_t)(13 + i * 7);
        f[o] = (uint8_t)recs[i].from_id;
        f[o + 1] = (uint8_t)(recs[i].from_id >> 8);
        f[o + 2] = (uint8_t)recs[i].to_id;
        f[o + 3] = (uint8_t)(recs[i].to_id >> 8);
        f[o + 4] = recs[i].tail0;
        f[o + 5] = recs[i].tail1;
        f[o + 6] = recs[i].tail2;
    }
    return len;
}

/* Sustained, non-self-poisoning observer. Drives the SIM to READY by re-GETting
 * 0x0200 on every 0x0210, and keeps polling signal and voice/data registration
 * across the whole settle window. A reply timeout or ambiguous write only backs
 * the prober off; it never disables itself, the dispatcher, or RFS quarantine. */
static void camp_probe_advance(struct owner *o, int64_t now)
{
    struct camp_driver *c = &o->camp;
    if (c->poisoned || stop_requested) return;
    /* A failed RFS terminal forbids further modem writes; a completed,
     * acknowledged quarantine does not block read-only GETs. */
    if (o->phase == TERMINAL && !o->final_ack_sent) return;
#ifndef RFS_HOST_TEST
    if (cp_state() != CP_ONLINE) return;
#endif
    if (now - c->owner_start_ms < PROBE_START_MS) return;
    /* Let the one-shot stage-1 dispatch win the armed radio edge. */
    if (c->radio_ready0_ms && !c->dispatched && !c->radio_invalidated) return;
    if (c->probe_pending) {
        if (now < c->probe_deadline_ms) return;
        c->probe_pending = 0;
        c->probe_timeouts++;
        printf("camp_probe field=%s status=timeout\n", c->probe_name);
        c->probe_next_ms = now + PROBE_GAP_MS;
        return;
    }
    if (now < c->probe_next_ms) return;
    if (o->used || o->sit.used || c->used) return; /* avoid mid-frame writes */
    if (!c->probe_token)
        c->probe_token = (uint32_t)now ^ (uint32_t)getpid() ^ 0x0b0b0000u;
    /* VERDICT 16: if the card is PIN-locked, unlock PIN1 once before anything
     * else. The PIN lives only in the request frame and is scrubbed right after
     * the write; it is never logged. */
    if (c->pin_required && c->pin[0] && !c->pin_verify_sent) {
        ++c->probe_token;
        uint8_t f[CALL_PIN_LEN];
        make_pin_verify_request(f, c->pin, c->probe_token);
        c->pin_verify_sent = 1;
        int wrote = camp_send_once(o->ipc, f, sizeof f);
        zero_bytes(f, sizeof f);
        if (wrote) { c->probe_next_ms = now + PROBE_GAP_MS; return; }
        c->probe_pending = 1;
        c->probe_id = CALL_PIN_VERIFY;
        c->probe_name = "verify_pin1";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_sim=sent step=verify_pin1 elapsed_ms=%lld\n",
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* Priority: once the SIM is READY, run the one-shot registration-trigger
     * sequence before resuming round-robin observation. GETs are idempotent and
     * may retry on timeout; the three SETs are marked sent and never resent. */
    unsigned reg = camp_reg_next(c);
    if (c->capquery && !reg && !c->reg_complete && c->radio_on) {
        /* VERDICT 25: read-only mode reaches no allow-data SET, so declare the
         * registration-trigger phase complete once radio is up and the
         * selection/preferred reads are done; this gates the GET diagnostics. */
        c->reg_complete = 1;
        printf("camp_reg capquery_ready response=yes radio_on=1\n");
    }
    if (reg) {
        ++c->probe_token;
        int wrote;
        int long_wait = 0;
        const char *rname;
        if (reg == REG_PREF_SET) {
            uint8_t f[REG_PREF_LEN];
            make_setpref_request(f, camp_pref_target(c), c->probe_token);
            c->pref_set_sent = 1;
            wrote = camp_send_once(o->ipc, f, sizeof f);
            rname = "set_preferred_lte_wcdma";
        } else if (reg == REG_SCAN) {
            c->scan_sent = 1;
            long_wait = 1;
            wrote = sit_send_get_once(o->ipc, (uint16_t)REG_SCAN, c->probe_token);
            rname = "query_available_networks";
        } else if (reg == REG_MANUAL_SEL) {
            uint8_t f[REG_MANUAL_LEN];
            make_manual_select_request(f, c->manual_plmn, c->probe_token);
            c->manual_sel_sent = 1;
            long_wait = 1;
            wrote = camp_send_once(o->ipc, f, sizeof f);
            rname = "set_network_selection_manual";
        } else if (reg == REG_ALLOW_DATA) {
            uint8_t f[REG_ALLOW_LEN];
            make_allowdata_request(f, c->probe_token);
            c->allow_data_sent = 1;
            wrote = camp_send_once(o->ipc, f, sizeof f);
            rname = "allow_data";
        } else if (reg == REG_INIT_ATTACH_APN) {
            uint8_t f[REG_IA_LEN];
            make_initial_attach_apn_request(f, c->apn, c->probe_token);
            c->ia_apn_sent = 1;
            wrote = camp_send_once(o->ipc, f, sizeof f);
            rname = "set_initial_attach_apn";
        } else {
            if (reg == REG_SEL_AUTO_SET) c->sel_auto_sent = 1;
            else if (reg == REG_RADIO_GET) c->radio_get_tries++;
            else if (reg == REG_SEL_GET) c->sel_get_tries++;
            else if (reg == REG_PREF_GET) c->pref_get_tries++;
            wrote = sit_send_get_once(o->ipc, (uint16_t)reg, c->probe_token);
            rname = reg == REG_RADIO_GET ? "get_radio" :
                    reg == REG_SEL_GET ? "get_selection" :
                    reg == REG_PREF_GET ? "get_preferred" : "set_selection_auto";
        }
        if (wrote) { c->probe_next_ms = now + PROBE_GAP_MS; return; }
        c->probe_pending = 1;
        c->probe_id = reg;
        c->probe_name = rname;
        c->probe_deadline_ms = now + (long_wait ? PROBE_SCAN_REPLY_MS
                                                : PROBE_REPLY_MS);
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_reg=sent step=%s elapsed_ms=%lld\n", rname,
               (long long)(now - c->owner_start_ms));
        return;
    }
    unsigned drg = camp_dereg_next(c);
    if (drg) {
        ++c->probe_token;
        int wrote;
        int long_wait = 0;
        unsigned wire_id;
        const char *rname;
        if (drg == DRG_OFF) {
            uint8_t f[CAMP_POWER_LEN];
            make_radiopower_request(f, c->probe_token, CAMP_POWER_OFF);
            c->drg_off_sent = 1;
            wrote = camp_send_once(o->ipc, f, sizeof f);
            rname = "dereg_radio_off"; wire_id = CAMP_POWER_COMMAND;
        } else if (drg == DRG_OFF_GET) {
            c->drg_off_get_sent = 1;
            c->drg_off_get_tries++;
            wrote = sit_send_get_once(o->ipc, (uint16_t)REG_RADIO_GET,
                                      c->probe_token);
            rname = "dereg_radio_get"; wire_id = REG_RADIO_GET;
        } else if (drg == DRG_ON) {
            uint8_t f[CAMP_POWER_LEN];
            make_radiopower_request(f, c->probe_token, CAMP_POWER_ON);
            c->drg_on_sent = 1;
            wrote = camp_send_once(o->ipc, f, sizeof f);
            rname = "dereg_radio_on"; wire_id = CAMP_POWER_COMMAND;
        } else if (drg == DRG_SCAN) {
            c->drg_scan_sent = 1;
            long_wait = 1;
            if (c->scan734) {
                uint8_t f[SCAN734_LEN];
                make_startscan_request(f, c->probe_token);
                c->scan734_sent = 1;
                wrote = camp_send_once(o->ipc, f, sizeof f);
                rname = "precamp_start_network_scan_lte";
                wire_id = SCAN734_GET;
            } else {
                wrote = sit_send_get_once(o->ipc, (uint16_t)REG_SCAN,
                                          c->probe_token);
                rname = "dereg_query_available_networks";
                wire_id = REG_SCAN;
            }
        } else { /* DRG_MANUAL */
            uint8_t f[REG_MANUAL_LEN];
            make_manual_select_request(f, c->manual_plmn, c->probe_token);
            c->drg_manual_sent = 1;
            wrote = camp_send_once(o->ipc, f, sizeof f);
            rname = "dereg_network_selection_manual"; wire_id = REG_MANUAL_SEL;
        }
        if (wrote) { c->probe_next_ms = now + PROBE_GAP_MS; return; }
        c->probe_pending = 1;
        c->probe_id = wire_id;
        c->probe_name = rname;
        c->probe_deadline_ms = now + (long_wait ? PROBE_SCAN_REPLY_MS
                                                : PROBE_REPLY_MS);
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_dereg=sent step=%s elapsed_ms=%lld\n", rname,
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* After data HOME: stock 0x0625, then the internet data profile, then
     * SetupDataCall. Each frame is the capture with a fresh token. */
    if (c->data_home && c->reg_complete && !c->setup_sent) {
        const uint8_t *src;
        size_t len;
        uint16_t id;
        const char *name;
        int *flag;
        if (!c->vonr_sent) {
            src = cc_sit_0625; len = sizeof cc_sit_0625; id = 0x0625;
            name = "vonr"; flag = &c->vonr_sent;
        } else if (!c->profile_sent) {
            src = cc_sit_0613; len = sizeof cc_sit_0613; id = 0x0613;
            name = "profile"; flag = &c->profile_sent;
        } else {
            src = cc_sit_0600; len = sizeof cc_sit_0600; id = 0x0600;
            name = "setup"; flag = &c->setup_sent;
        }
        ++c->probe_token;
        uint8_t f[sizeof cc_sit_0600];
        memcpy(f, src, len);
        put_little32(f + 6, c->probe_token);
        *flag = 1;
        int wrote = camp_send_once(o->ipc, f, len);
        if (wrote) {
            *flag = 0;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = id;
        c->probe_name = name;
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_%s=sent len=%zu elapsed_ms=%lld\n", name, len,
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* Stock reads modem activity once the data call exists, then fast dormancy. */
    if (c->data_home && c->reg_complete && c->setup_sent && !c->activity_sent) {
        ++c->probe_token;
        c->activity_sent = 1;
        int wrote = sit_send_get_once(o->ipc, 0x090c, c->probe_token);
        if (wrote) {
            c->activity_sent = 0;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = 0x090c;
        c->probe_name = "activity";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_activity=sent len=12 elapsed_ms=%lld\n",
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* Stock sends SetFastDormancy once the data call is up. */
    if (c->data_home && c->reg_complete && c->setup_sent && c->activity_sent &&
        !c->fd_sent) {
        ++c->probe_token;
        uint8_t f[sizeof cc_sit_0605];
        memcpy(f, cc_sit_0605, sizeof f);
        put_little32(f + 6, c->probe_token);
        c->fd_sent = 1;
        int wrote = camp_send_once(o->ipc, f, sizeof f);
        if (wrote) {
            c->fd_sent = 0;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = 0x0605;
        c->probe_name = "fastdorm";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_fastdorm=sent len=%zu elapsed_ms=%lld\n", sizeof f,
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* Stock then sets the ims and sos profiles. Each is the capture frame. */
    if (c->data_home && c->reg_complete && c->fd_sent &&
        (!c->ims_sent || !c->sos_sent)) {
        const uint8_t *src;
        const char *name;
        int *flag;
        if (!c->ims_sent) {
            src = cc_sit_0613_ims;
            name = "ims";
            flag = &c->ims_sent;
        } else {
            src = cc_sit_0613_sos;
            name = "sos";
            flag = &c->sos_sent;
        }
        ++c->probe_token;
        uint8_t f[sizeof cc_sit_0613_ims];
        memcpy(f, src, sizeof f);
        put_little32(f + 6, c->probe_token);
        *flag = 1;
        int wrote = camp_send_once(o->ipc, f, sizeof f);
        if (wrote) {
            *flag = 0;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = 0x0613;
        c->probe_name = name;
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_%s=sent len=%zu elapsed_ms=%lld\n", name, sizeof f,
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* After the profiles, stock reads ENDC mode, sets VoNR capability to the
     * captured zero word, then reads the RC network type. */
    if (c->data_home && c->reg_complete && c->sos_sent &&
        (!c->endc_sent || !c->vonrcapa_sent || !c->rcnet_sent)) {
        ++c->probe_token;
        uint16_t id;
        const char *name;
        int wrote;
        int *flag;
        if (!c->endc_sent) {
            id = 0x073e;
            name = "endc";
            flag = &c->endc_sent;
            *flag = 1;
            wrote = sit_send_get_once(o->ipc, id, c->probe_token);
        } else if (!c->vonrcapa_sent) {
            uint8_t f[16];
            id = 0x0954;
            name = "vonrcapa";
            flag = &c->vonrcapa_sent;
            make_opx_u32_request(f, id, 16, c->probe_token, 0);
            *flag = 1;
            wrote = camp_send_once(o->ipc, f, sizeof f);
        } else {
            id = 0x0718;
            name = "rcnet";
            flag = &c->rcnet_sent;
            *flag = 1;
            wrote = sit_send_get_once(o->ipc, id, c->probe_token);
        }
        if (wrote) {
            *flag = 0;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = id;
        c->probe_name = name;
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_%s=sent elapsed_ms=%lld\n", name,
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* After RC network type, stock sets data throttling (byte 0, duration
     * 30000), indication filter 0xff, then the settled filter 0x7f, then
     * screen state 1. Each builder stores that scalar in a 16- or 21-byte
     * request. The last filter word is 0x7f. */
    if (c->data_home && c->reg_complete && c->rcnet_sent &&
        (!c->throttle_sent || !c->unsol_sent || !c->screen_sent)) {
        ++c->probe_token;
        uint16_t id;
        const char *name;
        int wrote;
        int *flag;
        uint8_t f[21];
        size_t nsend;
        memset(f, 0, sizeof f);
        if (!c->throttle_sent) {
            id = 0x094d;
            name = "throttle";
            flag = &c->throttle_sent;
            nsend = 21;
            f[12] = 0;
            put_little32(f + 13, 30000u);
        } else if (!c->unsol_wide_sent) {
            id = 0x0928;
            name = "unsolff";
            flag = &c->unsol_wide_sent;
            nsend = 16;
            put_little32(f + 12, 0xffu);
        } else if (!c->unsol_sent) {
            id = 0x0928;
            name = "unsol";
            flag = &c->unsol_sent;
            nsend = 16;
            put_little32(f + 12, 0x7fu);
        } else {
            id = 0x0902;
            name = "screen";
            flag = &c->screen_sent;
            nsend = 16;
            put_little32(f + 12, 1u);
        }
        f[2] = (uint8_t)id;
        f[3] = (uint8_t)(id >> 8);
        f[4] = (uint8_t)nsend;
        put_little32(f + 6, c->probe_token);
        *flag = 1;
        wrote = camp_send_once(o->ipc, f, nsend);
        if (wrote) {
            *flag = 0;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = id;
        c->probe_name = name;
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_%s=sent len=%zu elapsed_ms=%lld\n", name, nsend,
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* Stock then reads the cell-info list and the SMSC address. Both are
     * empty GETs. Replies stay off the log. */
    if (c->data_home && c->reg_complete && c->screen_sent &&
        (!c->cellinfo_sent || !c->smsc_sent)) {
        ++c->probe_token;
        uint16_t id;
        const char *name;
        int *flag;
        if (!c->cellinfo_sent) {
            id = 0x070c;
            name = "cellinfo";
            flag = &c->cellinfo_sent;
        } else {
            id = 0x0108;
            name = "smsc";
            flag = &c->smsc_sent;
        }
        *flag = 1;
        int wrote = sit_send_get_once(o->ipc, id, c->probe_token);
        if (wrote) {
            *flag = 0;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = id;
        c->probe_name = name;
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_%s=sent elapsed_ms=%lld\n", name,
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* Stock later reads VoNR capability. Empty GET, reply body stays off
     * the log. */
    if (c->data_home && c->reg_complete && c->smsc_sent && !c->vonrget_sent) {
        ++c->probe_token;
        c->vonrget_sent = 1;
        int wrote = sit_send_get_once(o->ipc, 0x0953, c->probe_token);
        if (wrote) {
            c->vonrget_sent = 0;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = 0x0953;
        c->probe_name = "vonrget";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_vonrget=sent elapsed_ms=%lld\n",
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* Named early stock frames that carry no identifiers. AP time is the
     * current local clock, not a captured timestamp. Replies log error
     * and length only. */
    if (c->data_home && c->reg_complete && c->vonrget_sent &&
        (!c->aptime_sent || !c->dbgtrace_sent || !c->tty_sent ||
         !c->pssvc_sent || !c->prefmodem_sent)) {
        ++c->probe_token;
        if (!c->pssvc_sent && c->aptime_sent && c->dbgtrace_sent &&
            c->tty_sent) {
            c->pssvc_sent = 1;
            int wrote = sit_send_get_once(o->ipc, 0x0711, c->probe_token);
            if (wrote) {
                c->pssvc_sent = 0;
                c->probe_next_ms = now + PROBE_GAP_MS;
                return;
            }
            c->probe_pending = 1;
            c->probe_id = 0x0711;
            c->probe_name = "pssvc";
            c->probe_deadline_ms = now + PROBE_REPLY_MS;
            c->probe_next_ms = now + PROBE_GAP_MS;
            c->probe_sent++;
            printf("camp_pssvc=sent elapsed_ms=%lld\n",
                   (long long)(now - c->owner_start_ms));
            return;
        }
        uint8_t f[18];
        uint16_t id;
        const char *name;
        int *flag;
        size_t nsend;
        memset(f, 0, sizeof f);
        if (!c->aptime_sent) {
            time_t wall = time(NULL);
            struct tm tm;
            id = 0x0949;
            name = "aptime";
            flag = &c->aptime_sent;
            nsend = 18;
            if (!localtime_r(&wall, &tm)) {
                c->probe_next_ms = now + PROBE_GAP_MS;
                return;
            }
            camp_ap_time_bytes(f + 12, &tm);
        } else if (!c->dbgtrace_sent) {
            id = 0x090b;
            name = "dbgtrace";
            flag = &c->dbgtrace_sent;
            nsend = 13;
        } else if (!c->tty_sent) {
            id = 0x0903;
            name = "tty";
            flag = &c->tty_sent;
            nsend = 16;
        } else {
            id = 0x0740;
            name = "prefmodem";
            flag = &c->prefmodem_sent;
            nsend = 13;
        }
        f[2] = (uint8_t)id;
        f[3] = (uint8_t)(id >> 8);
        f[4] = (uint8_t)nsend;
        put_little32(f + 6, c->probe_token);
        *flag = 1;
        int wrote = camp_send_once(o->ipc, f, nsend);
        if (wrote) {
            *flag = 0;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = id;
        c->probe_name = name;
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_%s=sent len=%zu elapsed_ms=%lld\n", name, nsend,
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* Slot status is an empty GET; the 433-byte body stays off the log.
     * Signal criteria and SMS broadcast activation are the stock scalars. */
    if (c->data_home && c->reg_complete && c->prefmodem_sent &&
        (!c->slot_sent || !c->sigcrit_sent || !c->smsact_sent)) {
        ++c->probe_token;
        if (!c->slot_sent) {
            c->slot_sent = 1;
            int wrote = sit_send_get_once(o->ipc, 0x024d, c->probe_token);
            if (wrote) {
                c->slot_sent = 0;
                c->probe_next_ms = now + PROBE_GAP_MS;
                return;
            }
            c->probe_pending = 1;
            c->probe_id = 0x024d;
            c->probe_name = "slot";
            c->probe_deadline_ms = now + PROBE_REPLY_MS;
            c->probe_next_ms = now + PROBE_GAP_MS;
            c->probe_sent++;
            printf("camp_slot=sent elapsed_ms=%lld\n",
                   (long long)(now - c->owner_start_ms));
            return;
        }
        if (!c->sigcrit_sent) {
            uint8_t f[67];
            camp_fill_signal_criteria(f);
            put_little32(f + 6, c->probe_token);
            c->sigcrit_sent = 1;
            int wrote = camp_send_once(o->ipc, f, sizeof f);
            if (wrote) {
                c->sigcrit_sent = 0;
                c->probe_next_ms = now + PROBE_GAP_MS;
                return;
            }
            c->probe_pending = 1;
            c->probe_id = 0x0943;
            c->probe_name = "sigcrit";
            c->probe_deadline_ms = now + PROBE_REPLY_MS;
            c->probe_next_ms = now + PROBE_GAP_MS;
            c->probe_sent++;
            printf("camp_sigcrit=sent len=67 elapsed_ms=%lld\n",
                   (long long)(now - c->owner_start_ms));
            return;
        }
        {
            uint8_t f[16];
            make_opx_u32_request(f, 0x0107, 16, c->probe_token, 0);
            c->smsact_sent = 1;
            int wrote = camp_send_once(o->ipc, f, sizeof f);
            if (wrote) {
                c->smsact_sent = 0;
                c->probe_next_ms = now + PROBE_GAP_MS;
                return;
            }
            c->probe_pending = 1;
            c->probe_id = 0x0107;
            c->probe_name = "smsact";
            c->probe_deadline_ms = now + PROBE_REPLY_MS;
            c->probe_next_ms = now + PROBE_GAP_MS;
            c->probe_sent++;
            printf("camp_smsact=sent len=16 elapsed_ms=%lld\n",
                   (long long)(now - c->owner_start_ms));
            return;
        }
    }
    /* Five stock link-capacity criteria frames, one per non-zero access
     * table entry. Bodies stay off the log; only the access word is a
     * small index. */
    if (c->data_home && c->reg_complete && c->smsact_sent &&
        c->linkcrit_next < 5) {
        uint8_t f[190];
        uint32_t access = camp_link_access[c->linkcrit_next];
        ++c->probe_token;
        camp_fill_link_criteria(f, access);
        put_little32(f + 6, c->probe_token);
        c->linkcrit_next++;
        int wrote = camp_send_once(o->ipc, f, sizeof f);
        if (wrote) {
            c->linkcrit_next--;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = 0x0944;
        c->probe_name = "linkcrit";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_linkcrit=sent len=190 access=%u elapsed_ms=%lld\n",
               access, (long long)(now - c->owner_start_ms));
        return;
    }
    /* Two stock broadcast-SMS configs. The record list stays off the log. */
    if (c->data_home && c->reg_complete && c->linkcrit_next >= 5 &&
        c->smscb_next < 2) {
        uint8_t f[76];
        int which = c->smscb_next;
        size_t nsend;
        ++c->probe_token;
        nsend = camp_fill_smscb(f, which);
        put_little32(f + 6, c->probe_token);
        c->smscb_next++;
        int wrote = camp_send_once(o->ipc, f, nsend);
        if (wrote) {
            c->smscb_next--;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = 0x0106;
        c->probe_name = "smscb";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_smscb=sent len=%zu count=%u elapsed_ms=%lld\n",
               nsend, f[12], (long long)(now - c->owner_start_ms));
        return;
    }
    /* Stock reads the call list once. Empty GET. A live call still owns
     * the reply through camp_call_reply; this boot read only logs the
     * count. */
    if (c->data_home && c->reg_complete && c->smscb_next >= 2 &&
        !c->calllist_sent) {
        ++c->probe_token;
        c->calllist_sent = 1;
        int wrote = sit_send_get_once(o->ipc, 0x0000, c->probe_token);
        if (wrote) {
            c->calllist_sent = 0;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = 0x0000;
        c->probe_name = "calllist";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_calllist=sent elapsed_ms=%lld\n",
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* Stock sets voice operation to 3 once. A selected opx-step still
     * owns that SET, so this fires only on the default GET-only boot. */
    if (c->data_home && c->reg_complete && c->calllist_sent &&
        c->opx_step == OPX_STEP_NONE && !c->opx_set_sent &&
        !c->voice_stock_sent) {
        uint8_t f[OPX_VOICE_LEN];
        ++c->probe_token;
        make_opx_u32_request(f, OPX_VOICE_SET, OPX_VOICE_LEN,
                             c->probe_token, OPX_VOICE_MODE);
        c->voice_stock_sent = 1;
        int wrote = camp_send_once(o->ipc, f, sizeof f);
        if (wrote) {
            c->voice_stock_sent = 0;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = OPX_VOICE_SET;
        c->probe_name = "set_voice_operation";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_voiceop=sent mode=%u elapsed_ms=%lld\n",
               (unsigned)OPX_VOICE_MODE,
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* Stock reads the baseband version once (selector 0xFF). The reply
     * logger prints the SW build string and leaves HW/RF-cal off the log.
     * bbver_sent blocks the later file-gated path from sending it again. */
    if (c->data_home && c->reg_complete && c->calllist_sent &&
        (c->voice_stock_sent || c->opx_step != OPX_STEP_NONE ||
         c->opx_set_sent) &&
        !c->bbver_sent) {
        uint8_t f[BBVER_LEN];
        ++c->probe_token;
        make_opx_byte_request(f, BBVER_GET, BBVER_LEN, c->probe_token,
                              BBVER_MASK);
        c->bbver_sent = 1;
        c->bbver_enabled = 1;
        int wrote = camp_send_once(o->ipc, f, sizeof f);
        if (wrote) {
            c->bbver_sent = 0;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = BBVER_GET;
        c->probe_name = "get_baseband_version";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_bbver=sent elapsed_ms=%lld\n",
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* SIT_SET_GPS_LOCK_MODE. Stock sends one byte, 1. */
    if (c->data_home && c->reg_complete && c->bbver_sent &&
        !c->gpslock_sent) {
        uint8_t f[13];
        ++c->probe_token;
        make_opx_byte_request(f, 0x0c20, 13, c->probe_token, 1);
        c->gpslock_sent = 1;
        int wrote = camp_send_once(o->ipc, f, sizeof f);
        if (wrote) {
            c->gpslock_sent = 0;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = 0x0c20;
        c->probe_name = "gpslock";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_gpslock=sent elapsed_ms=%lld\n",
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* SIT_SET_GPS_NFW_STATUS. Stock sends one byte, 0. */
    if (c->data_home && c->reg_complete && c->gpslock_sent &&
        !c->gpsnfw_sent) {
        uint8_t f[13];
        ++c->probe_token;
        make_opx_byte_request(f, 0x0c33, 13, c->probe_token, 0);
        c->gpsnfw_sent = 1;
        int wrote = camp_send_once(o->ipc, f, sizeof f);
        if (wrote) {
            c->gpsnfw_sent = 0;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = 0x0c33;
        c->probe_name = "gpsnfw";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_gpsnfw=sent elapsed_ms=%lld\n",
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* SIT_GET_SA_MODE. Empty GET. The reply string stays off the log. */
    if (c->data_home && c->reg_complete && c->gpsnfw_sent &&
        !c->samode_sent) {
        ++c->probe_token;
        c->samode_sent = 1;
        int wrote = sit_send_get_once(o->ipc, 0x0755, c->probe_token);
        if (wrote) {
            c->samode_sent = 0;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = 0x0755;
        c->probe_name = "samode";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_samode=sent elapsed_ms=%lld\n",
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* SIT_AIMS_XCAPM_STOP_REQ. Stock body 01 01, err 0. */
    if (c->data_home && c->reg_complete && c->samode_sent &&
        !c->xcapstop_sent) {
        uint8_t f[14];
        ++c->probe_token;
        camp_fill_pair(f, 0x0d3c, c->probe_token, 1, 1);
        c->xcapstop_sent = 1;
        int wrote = camp_send_once(o->ipc, f, sizeof f);
        if (wrote) {
            c->xcapstop_sent = 0;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = 0x0d3c;
        c->probe_name = "xcapstop";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_xcapstop=sent elapsed_ms=%lld\n",
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* SIT_AIMS_STACK_STOP_REQ. Stock body 01 01, err 0. */
    if (c->data_home && c->reg_complete && c->xcapstop_sent &&
        !c->aimstop_sent) {
        uint8_t f[14];
        ++c->probe_token;
        camp_fill_pair(f, 0x0d3a, c->probe_token, 1, 1);
        c->aimstop_sent = 1;
        int wrote = camp_send_once(o->ipc, f, sizeof f);
        if (wrote) {
            c->aimstop_sent = 0;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = 0x0d3a;
        c->probe_name = "aimstop";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_aimstop=sent elapsed_ms=%lld\n",
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* SIT_AIMS_XCAPM_START_REQ. Stock body 01 01, err 0. */
    if (c->data_home && c->reg_complete && c->aimstop_sent &&
        !c->xcapstart_sent) {
        uint8_t f[14];
        ++c->probe_token;
        camp_fill_pair(f, 0x0d3b, c->probe_token, 1, 1);
        c->xcapstart_sent = 1;
        int wrote = camp_send_once(o->ipc, f, sizeof f);
        if (wrote) {
            c->xcapstart_sent = 0;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = 0x0d3b;
        c->probe_name = "xcapstart";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_xcapstart=sent elapsed_ms=%lld\n",
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* Second stock SIT_AIMS_STACK_STOP_REQ. Body 00 01, err 0. */
    if (c->data_home && c->reg_complete && c->xcapstart_sent &&
        !c->aimstop0_sent) {
        uint8_t f[14];
        ++c->probe_token;
        camp_fill_pair(f, 0x0d3a, c->probe_token, 0, 1);
        c->aimstop0_sent = 1;
        int wrote = camp_send_once(o->ipc, f, sizeof f);
        if (wrote) {
            c->aimstop0_sent = 0;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = 0x0d3a;
        c->probe_name = "aimstop0";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_aimstop0=sent elapsed_ms=%lld\n",
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* Second stock SIT_AIMS_XCAPM_STOP_REQ. Body 00 01, err 0. */
    if (c->data_home && c->reg_complete && c->aimstop0_sent &&
        !c->xcapstop0_sent) {
        uint8_t f[14];
        ++c->probe_token;
        camp_fill_pair(f, 0x0d3c, c->probe_token, 0, 1);
        c->xcapstop0_sent = 1;
        int wrote = camp_send_once(o->ipc, f, sizeof f);
        if (wrote) {
            c->xcapstop0_sent = 0;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = 0x0d3c;
        c->probe_name = "xcapstop0";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_xcapstop0=sent elapsed_ms=%lld\n",
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* Stock SetAllowedNetworkTypeBitmap. Body is the recovered wire
     * word 0x403fe. The /data file still owns the longer GET/SET
     * experiment; this fires only when that file is absent. */
    if (c->data_home && c->reg_complete && c->xcapstop0_sent &&
        !c->ratbm_enabled && !c->ratbm_set_sent) {
        uint8_t f[RATBM_SET_LEN];
        ++c->probe_token;
        make_opx_u32_request(f, RATBM_SET, RATBM_SET_LEN, c->probe_token,
                             raf_to_sit_ratbm(RATBM_RAF_STOCK));
        c->ratbm_set_sent = 1;
        int wrote = camp_send_once(o->ipc, f, sizeof f);
        if (wrote) {
            c->ratbm_set_sent = 0;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = RATBM_SET;
        c->probe_name = "set_allowed_bitmap_stock";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_ratbm=sent step=set_allowed_bitmap_stock wire=0x%x "
               "elapsed_ms=%lld\n",
               (unsigned)raf_to_sit_ratbm(RATBM_RAF_STOCK),
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* The other nine initial signal-criteria bodies. Each once, after
     * the bitmap, so the first GERAN criteria stays where it already is. */
    if (c->data_home && c->reg_complete && c->xcapstop0_sent &&
        (c->ratbm_enabled || c->ratbm_set_sent) &&
        c->sigcrit_extra < (int)(sizeof camp_sig_more / sizeof camp_sig_more[0])) {
        const struct camp_sig_row *row = &camp_sig_more[c->sigcrit_extra];
        uint8_t f[67];
        ++c->probe_token;
        camp_fill_signal_row(f, row);
        put_little32(f + 6, c->probe_token);
        c->sigcrit_extra++;
        int wrote = camp_send_once(o->ipc, f, sizeof f);
        if (wrote) {
            c->sigcrit_extra--;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = 0x0943;
        c->probe_name = "sigcrit";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_sigcrit=sent access=%u flag=%u on=%u elapsed_ms=%lld\n",
               (unsigned)row->access, row->flag, row->on,
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* Stock reads the phone capability twice with the same 12-byte
     * header. The length field stays 0. Reply is four small counts. */
    if (c->data_home && c->reg_complete && c->xcapstop0_sent &&
        (c->ratbm_enabled || c->ratbm_set_sent) &&
        c->sigcrit_extra >= (int)(sizeof camp_sig_more / sizeof camp_sig_more[0]) &&
        !c->phonecap_sent) {
        uint8_t f[12];
        ++c->probe_token;
        memset(f, 0, sizeof f);
        f[2] = 0x15;
        f[3] = 0x06;
        put_little32(f + 6, c->probe_token);
        c->phonecap_sent = 1;
        int wrote = camp_send_once(o->ipc, f, sizeof f);
        if (wrote) {
            c->phonecap_sent = 0;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = 0x0615;
        c->probe_name = "phonecap";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_phonecap=sent elapsed_ms=%lld\n",
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* Stock sends 0x0d3c a third time, body 00 02, err 0. The matching
     * 0x0d3a write has no paired reply, so it stays unsent. */
    if (c->data_home && c->reg_complete && c->phonecap_sent &&
        !c->xcapstop2_sent) {
        uint8_t f[14];
        ++c->probe_token;
        camp_fill_pair(f, 0x0d3c, c->probe_token, 0, 2);
        c->xcapstop2_sent = 1;
        int wrote = camp_send_once(o->ipc, f, sizeof f);
        if (wrote) {
            c->xcapstop2_sent = 0;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = 0x0d3c;
        c->probe_name = "xcapstop2";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_xcapstop2=sent elapsed_ms=%lld\n",
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* SIT_GET_ATR. Empty GET, length field 12. The 47-byte reply can
     * carry card bytes, so the log keeps error and length only. */
    if (c->data_home && c->reg_complete && c->xcapstop2_sent &&
        !c->atr_sent) {
        ++c->probe_token;
        c->atr_sent = 1;
        int wrote = sit_send_get_once(o->ipc, 0x0212, c->probe_token);
        if (wrote) {
            c->atr_sent = 0;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = 0x0212;
        c->probe_name = "atr";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_atr=sent elapsed_ms=%lld\n",
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* SendSvnInfo. Stock writes the two vendor-build digits. They are
     * version metadata; the log records error and length only. */
    if (c->data_home && c->reg_complete && c->atr_sent && !c->svn_sent) {
        uint8_t f[14];
        ++c->probe_token;
        camp_fill_pair(f, 0x4605, c->probe_token, 0x39, 0x34);
        c->svn_sent = 1;
        int wrote = camp_send_once(o->ipc, f, sizeof f);
        if (wrote) {
            c->svn_sent = 0;
            c->probe_next_ms = now + PROBE_GAP_MS;
            return;
        }
        c->probe_pending = 1;
        c->probe_id = 0x4605;
        c->probe_name = "svn";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_svn=sent elapsed_ms=%lld\n",
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* VERDICT 21: read the running CP baseband/SW version (read-only). */
    unsigned bbv = camp_bbver_next(c);
    if (bbv) {
        ++c->probe_token;
        uint8_t f[BBVER_LEN];
        make_opx_byte_request(f, BBVER_GET, BBVER_LEN, c->probe_token,
                              BBVER_MASK);
        c->bbver_sent = 1;
        int wrote = camp_send_once(o->ipc, f, sizeof f);
        if (wrote) { c->probe_next_ms = now + PROBE_GAP_MS; return; }
        c->probe_pending = 1;
        c->probe_id = BBVER_GET;
        c->probe_name = "get_baseband_version";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_bbver=sent elapsed_ms=%lld\n",
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* VERDICT 20: modern RAT-gate experiment -- diagnostic reads, one SET,
     * then a confirming read-back. Each step fires once. */
    unsigned rbm = camp_ratbm_next(c);
    if (rbm) {
        ++c->probe_token;
        int wrote;
        const char *rname;
        if (rbm == RATBM_SET) {
            uint8_t f[RATBM_SET_LEN];
            make_opx_u32_request(f, RATBM_SET, RATBM_SET_LEN, c->probe_token,
                                 raf_to_sit_ratbm(RATBM_RAF_STOCK));
            c->ratbm_set_sent = 1;
            wrote = camp_send_once(o->ipc, f, sizeof f);
            rname = "set_allowed_bitmap_stock";
        } else {
            if (rbm == RATBM_BANDMODE_GET) c->ratbm_band_sent = 1;
            else if (!c->ratbm_get1_sent) c->ratbm_get1_sent = 1;
            else c->ratbm_get2_sent = 1;
            wrote = sit_send_get_once(o->ipc, (uint16_t)rbm, c->probe_token);
            rname = rbm == RATBM_BANDMODE_GET ? "get_band_mode" :
                    c->ratbm_set_sent ? "get_allowed_bitmap_after" :
                                        "get_allowed_bitmap_before";
        }
        if (wrote) { c->probe_next_ms = now + PROBE_GAP_MS; return; }
        c->probe_pending = 1;
        c->probe_id = rbm;
        c->probe_name = rname;
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_ratbm=sent step=%s elapsed_ms=%lld\n", rname,
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* VERDICT 26: one-shot StartNetworkScan (0x0734) targeting EUTRAN LTE. */
    unsigned sns = camp_scan734_next(c);
    if (sns) {
        ++c->probe_token;
        uint8_t f[SCAN734_LEN];
        make_startscan_request(f, c->probe_token);
        c->scan734_sent = 1;
        int wrote = camp_send_once(o->ipc, f, sizeof f);
        if (wrote) { c->probe_next_ms = now + PROBE_GAP_MS; return; }
        c->probe_pending = 1;
        c->probe_id = SCAN734_GET;
        c->probe_name = "start_network_scan_lte";
        c->probe_deadline_ms = now + PROBE_SCAN_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_scan734=sent ran=eutran bands=all elapsed_ms=%lld\n",
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* VERDICT 17: len-16 scanType sweep (one scan per distinct mode 0..5). */
    unsigned s16 = camp_scan16_next(c);
    if (s16) {
        ++c->probe_token;
        int mode = c->scan16_fired;   /* 0..5 */
        uint8_t f[REG_SCAN16_LEN];
        make_scan16_request(f, mode, c->probe_token);
        c->scan16_sent = 1;
        c->scan16_mode = mode;
        int wrote = camp_send_once(o->ipc, f, sizeof f);
        if (wrote) { c->probe_next_ms = now + PROBE_GAP_MS; return; }
        c->probe_pending = 1;
        c->probe_id = REG_SCAN16;
        c->probe_name = "query_available_networks_v16";
        c->probe_deadline_ms = now + PROBE_SCAN_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_scan16=sent mode=%d elapsed_ms=%lld\n", mode,
               (long long)(now - c->owner_start_ms));
        return;
    }
    unsigned opx = camp_opx_next(c);
    if (opx) {
        ++c->probe_token;
        int wrote;
        const char *rname;
        if (opx == OPX_VOICE_SET) {
            uint8_t f[OPX_VOICE_LEN];
            make_opx_u32_request(f, OPX_VOICE_SET, OPX_VOICE_LEN,
                                 c->probe_token, OPX_VOICE_MODE);
            c->opx_set_sent = 1;
            wrote = camp_send_once(o->ipc, f, sizeof f);
            rname = "set_voice_operation";
        } else if (opx == OPX_INTPS_SET) {
            uint8_t f[OPX_INTPS_LEN];
            make_opx_u32_request(f, OPX_INTPS_SET, OPX_INTPS_LEN,
                                 c->probe_token, OPX_INTPS_MODE);
            c->opx_set_sent = 1;
            wrote = camp_send_once(o->ipc, f, sizeof f);
            rname = "set_intps_service";
        } else if (opx == OPX_STACK_SET) {
            uint8_t f[OPX_STACK_LEN];
            make_opx_byte_request(f, OPX_STACK_SET, OPX_STACK_LEN,
                                  c->probe_token, OPX_STACK_MODE_ENABLE);
            c->opx_set_sent = 1;
            wrote = camp_send_once(o->ipc, f, sizeof f);
            rname = "set_stack_status";
        } else if (opx == OPX_DEVSVC_SET) {
            uint8_t f[OPX_DEVSVC_LEN];
            make_opx_u32_request(f, OPX_DEVSVC_SET, OPX_DEVSVC_LEN,
                                 c->probe_token, OPX_DEVSVC_MODE_DATA);
            c->opx_set_sent = 1;
            wrote = camp_send_once(o->ipc, f, sizeof f);
            rname = "set_device_service";
        } else if (opx == OPX_DUAL_SET) {
            uint8_t f[OPX_DUAL_LEN];
            make_opx_dual_request(f, c->probe_token);
            c->opx_set_sent = 1;
            wrote = camp_send_once(o->ipc, f, sizeof f);
            rname = "set_dual_network_allow_data";
        } else {
            if (opx == OPX_STACK_GET) c->opx_stack_get_sent = 1;
            else if (opx == OPX_VOICE_GET) c->opx_voice_get_sent = 1;
            else if (opx == OPX_DEVSVC_GET) c->opx_devsvc_get_sent = 1;
            wrote = sit_send_get_once(o->ipc, (uint16_t)opx, c->probe_token);
            rname = opx == OPX_STACK_GET ? "get_stack_status" :
                    opx == OPX_VOICE_GET ? "get_voice_operation" :
                                           "get_device_service";
        }
        /* After the single SET is dispatched the experiment is done; the normal
         * round-robin then keeps snapshotting voice/data registration. */
        if (c->opx_set_sent) c->opx_done = 1;
        if (wrote) { c->probe_next_ms = now + PROBE_GAP_MS; return; }
        c->probe_pending = 1;
        c->probe_id = opx;
        c->probe_name = rname;
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_opx=sent step=%s elapsed_ms=%lld\n", rname,
               (long long)(now - c->owner_start_ms));
        return;
    }
    /* VERDICT 16 activation call: DIAL once, poll the call list, then HANGUP. */
    unsigned cll = camp_call_next(c);
    if (cll) {
        ++c->probe_token;
        int wrote;
        int long_wait = 0;
        unsigned wire_id;
        const char *rname;
        if (cll == CLL_DIAL) {
            uint8_t f[CALL_DIAL_LEN];
            make_dial_request(f, c->call_number, c->probe_token);
            c->call_dial_sent = 1;
            long_wait = 1;
            wrote = camp_send_once(o->ipc, f, sizeof f);
            rname = "dial_voice"; wire_id = CALL_DIAL;
        } else if (cll == CLL_POLL) {
            c->call_poll_count++;
            wrote = sit_send_get_once(o->ipc, (uint16_t)CALL_LIST,
                                      c->probe_token);
            rname = "get_call_list"; wire_id = CALL_LIST;
        } else { /* CLL_HANGUP */
            uint8_t f[CALL_HANGUP_LEN];
            make_hangup_request(f, c->call_index ? c->call_index : 1,
                                c->probe_token);
            c->call_hangup_sent = 1;
            wrote = camp_send_once(o->ipc, f, sizeof f);
            rname = "hangup"; wire_id = CALL_HANGUP;
        }
        if (wrote) { c->probe_next_ms = now + PROBE_GAP_MS; return; }
        c->probe_pending = 1;
        c->probe_id = wire_id;
        c->probe_name = rname;
        c->probe_deadline_ms = now + (long_wait ? CALL_POLL_REPLY_MS
                                                : PROBE_REPLY_MS);
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_call=sent step=%s elapsed_ms=%lld\n", rname,
               (long long)(now - c->owner_start_ms));
        return;
    }
    if (camp_sms_next(c) == SMS_SEND) {
        uint8_t f[SMS_SEND_LEN];
        ++c->probe_token;
        if (!make_sms_request(f, c->sms_number, c->probe_token)) {
            c->sms_done = 1;
            puts("camp_sms=skipped");
            zero_bytes(f, sizeof f);
            return;
        }
        c->sms_sent = 1;
        int wrote = camp_send_once(o->ipc, f, sizeof f);
        zero_bytes(f, sizeof f);
        if (wrote) { c->probe_next_ms = now + PROBE_GAP_MS; return; }
        c->probe_pending = 1;
        c->probe_id = SMS_SEND;
        c->probe_name = "send_sms";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_sms=sent step=send elapsed_ms=%lld\n",
               (long long)(now - c->owner_start_ms));
        return;
    }
    if (camp_volteprov_next(c) == VOLTE_PROV_GET) {
        ++c->probe_token;
        c->volteprov_sent = 1;
        int wrote = sit_send_get_once(o->ipc, (uint16_t)VOLTE_PROV_GET,
                                      c->probe_token);
        if (wrote) { c->probe_next_ms = now + PROBE_GAP_MS; return; }
        c->probe_pending = 1;
        c->probe_id = VOLTE_PROV_GET;
        c->probe_name = "volteprov";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_volteprov=sent elapsed_ms=%lld\n",
               (long long)(now - c->owner_start_ms));
        return;
    }
    if (camp_emquery_next(c) == EM_QUERY) {
        uint8_t f[EM_QUERY_LEN];
        ++c->probe_token;
        make_emquery_request(f, c->probe_token);
        c->emquery_sent = 1;
        int wrote = camp_send_once(o->ipc, f, sizeof f);
        if (wrote) { c->probe_next_ms = now + PROBE_GAP_MS; return; }
        c->probe_pending = 1;
        c->probe_id = EM_QUERY;
        c->probe_name = "emquery";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_emquery=sent elapsed_ms=%lld\n",
               (long long)(now - c->owner_start_ms));
        return;
    }
    if (camp_dcall_next(c) == DCALL_LIST) {
        ++c->probe_token;
        c->dcall_sent = 1;
        int wrote = sit_send_get_once(o->ipc, (uint16_t)DCALL_LIST,
                                      c->probe_token);
        if (wrote) { c->probe_next_ms = now + PROBE_GAP_MS; return; }
        c->probe_pending = 1;
        c->probe_id = DCALL_LIST;
        c->probe_name = "dcall";
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_dcall=sent elapsed_ms=%lld\n",
               (long long)(now - c->owner_start_ms));
        return;
    }
    unsigned pick;
    const char *name;
    if (c->sim_change_pending && !c->sim_ready) {
        pick = 0x0200;
        name = "sim";
        c->sim_change_pending = 0;
    } else {
        pick = camp_probe_gets[c->probe_idx].id;
        name = camp_probe_gets[c->probe_idx].name;
        c->probe_idx = (c->probe_idx + 1u) % (unsigned)CAMP_PROBE_COUNT;
    }
    ++c->probe_token;
    if (sit_send_get_once(o->ipc, (uint16_t)pick, c->probe_token)) {
        c->probe_next_ms = now + PROBE_GAP_MS; /* ambiguous write: back off */
        return;
    }
    c->probe_pending = 1;
    c->probe_id = pick;
    c->probe_name = name;
    c->probe_deadline_ms = now + PROBE_REPLY_MS;
    c->probe_next_ms = now + PROBE_GAP_MS;
    c->probe_sent++;
    printf("camp_probe=sent field=%s elapsed_ms=%lld\n", name,
           (long long)(now - c->owner_start_ms));
}

/* One operational SET is run per boot; the operator selects which by writing a
 * single token to /data/saaios/etc/opx-step. Absent/empty/unknown => GET-only
 * (no SET). Read once at startup; never a secret and never an NV/EFS path. */
static int read_opx_step(void)
{
    int fd = open("/data/saaios/etc/opx-step", O_RDONLY | O_CLOEXEC);
    if (fd < 0) return OPX_STEP_NONE;
    char buf[16] = {0};
    ssize_t r = read(fd, buf, sizeof buf - 1);
    close(fd);
    if (r <= 0) return OPX_STEP_NONE;
    if (!strncmp(buf, "voice", 5)) return OPX_STEP_VOICE;
    if (!strncmp(buf, "intps", 5)) return OPX_STEP_INTPS;
    if (!strncmp(buf, "stack", 5)) return OPX_STEP_STACK;
    if (!strncmp(buf, "devsvc", 6)) return OPX_STEP_DEVSVC;
    if (!strncmp(buf, "dual", 4)) return OPX_STEP_DUAL;
    return OPX_STEP_NONE;
}

/* Load the configured attach APN from /data/saaios/etc/apn (e.g. "internet").
 * Trailing whitespace/newlines are stripped. Returns 1 and fills out[] on
 * success; 0 (and out[0]=0) when absent/empty. The APN is network config, not a
 * secret, and this is never an NV/EFS path. */
static int read_apn(char *out, size_t cap)
{
    if (cap) out[0] = 0;
    int fd = open("/data/saaios/etc/apn", O_RDONLY | O_CLOEXEC);
    if (fd < 0) return 0;
    char buf[REG_IA_APN_MAX] = {0};
    ssize_t r = read(fd, buf, sizeof buf - 1);
    close(fd);
    if (r <= 0) return 0;
    size_t n = (size_t)r;
    while (n && (buf[n - 1] == '\n' || buf[n - 1] == '\r' ||
                 buf[n - 1] == ' ' || buf[n - 1] == '\t'))
        n--;
    if (!n || n >= cap) return 0;
    memcpy(out, buf, n);
    out[n] = 0;
    return 1;
}

/* Load the preferred-RAT target override from /data/saaios/etc/pref_rat.
 * Returns the SIT preferred value to request, or 0 (=> default RAT_LTE_WCDMA).
 * Only the recovered values are honored: "11"=RAT_LTE_ONLY, "12"=RAT_LTE_WCDMA.
 * This is a RAT preference (no NV), revertible by removing the file. */
static unsigned read_pref_rat(void)
{
    int fd = open("/data/saaios/etc/pref_rat", O_RDONLY | O_CLOEXEC);
    if (fd < 0) return 0;
    char buf[16] = {0};
    ssize_t r = read(fd, buf, sizeof buf - 1);
    close(fd);
    if (r <= 0) return 0;
    if (!strncmp(buf, "11", 2)) return RAT_LTE_ONLY;
    if (!strncmp(buf, "12", 2)) return RAT_LTE_WCDMA;
    return 0;
}

/* The available-networks scan (0x0706) is one deliberate, operator-armed probe:
 * it runs only when /data/saaios/etc/do_scan exists. Returns 1 if armed. */
static int read_do_scan(void)
{
    int fd = open("/data/saaios/etc/do_scan", O_RDONLY | O_CLOEXEC);
    if (fd < 0) return 0;
    close(fd);
    return 1;
}

/* The deregister-then-scan sub-sequence (VERDICT 15) runs only when
 * /data/saaios/etc/dereg_scan exists. One deliberate RADIO_POWER OFF->ON cycle
 * plus one scan (and at most one manual-select); never spam. Returns 1 if armed. */
static int read_dereg_scan(void)
{
    int fd = open("/data/saaios/etc/dereg_scan", O_RDONLY | O_CLOEXEC);
    if (fd < 0) return 0;
    close(fd);
    return 1;
}

/* VERDICT 17: the len-16 scanType sweep runs only when /data/saaios/etc/scan16
 * exists. One scan per distinct accepted scanType 0..5, never a spam loop.
 * Returns 1 if armed. */
static int read_scan16(void)
{
    int fd = open("/data/saaios/etc/scan16", O_RDONLY | O_CLOEXEC);
    if (fd < 0) return 0;
    close(fd);
    return 1;
}

/* VERDICT 26: the one-shot StartNetworkScan (0x0734) LTE scan runs only when
 * /data/saaios/etc/scan734 exists. Returns 1 if armed. */
static int read_scan734(void)
{
    int fd = open("/data/saaios/etc/scan734", O_RDONLY | O_CLOEXEC);
    if (fd < 0) return 0;
    close(fd);
    return 1;
}

/* VERDICT 20: the modern RAT-gate experiment runs only when
 * /data/saaios/etc/ratbm exists. Diagnostic GET 0x750/0x709, one revertible SET
 * 0x074f (LTE+WCDMA+GSM), then a confirming GET 0x750. Returns 1 if armed. */
static int read_ratbm(void)
{
    int fd = open("/data/saaios/etc/ratbm", O_RDONLY | O_CLOEXEC);
    if (fd < 0) return 0;
    close(fd);
    return 1;
}

/* VERDICT 21: the read-only baseband-version query runs only when
 * /data/saaios/etc/bbver exists. Returns 1 if armed. */
static int read_bbver(void)
{
    int fd = open("/data/saaios/etc/bbver", O_RDONLY | O_CLOEXEC);
    if (fd < 0) return 0;
    close(fd);
    return 1;
}

/* VERDICT 25: the read-only capability diagnostic runs only when
 * /data/saaios/etc/capquery exists. It fires GET opcodes only (no SET of any
 * kind) and changes no modem state. Returns 1 if armed. */
static int read_capquery(void)
{
    int fd = open("/data/saaios/etc/capquery", O_RDONLY | O_CLOEXEC);
    if (fd < 0) return 0;
    close(fd);
    return 1;
}

/* Load a manual network-selection target (numeric MCC/MNC, e.g. "25506") from
 * /data/saaios/etc/manual_plmn. Returns 1 and fills out[] (5 or 6 digits) when a
 * plausible PLMN is present; 0 otherwise. A PLMN is network topology, not a
 * secret, and manual selection is a radio command, never an NV/EFS write. */
static int read_manual_plmn(char *out, size_t cap)
{
    if (cap) out[0] = 0;
    int fd = open("/data/saaios/etc/manual_plmn", O_RDONLY | O_CLOEXEC);
    if (fd < 0) return 0;
    char buf[16] = {0};
    ssize_t r = read(fd, buf, sizeof buf - 1);
    close(fd);
    if (r <= 0) return 0;
    size_t n = 0;
    while (n < sizeof buf && buf[n] >= '0' && buf[n] <= '9') n++;
    if (n < 5 || n > 6 || n >= cap) return 0;
    memcpy(out, buf, n);
    out[n] = 0;
    return 1;
}

/* VERDICT 16: load the SIM PIN1 from /data/saaios/etc/sim_pin. The PIN is a
 * secret -- this reader copies it into out[] for immediate use in the
 * PIN-verify request and nothing else; it is never logged. Accepts 4..8 digit
 * strings. Returns 1 on success. */
static int read_sim_pin(char *out, size_t cap)
{
    if (cap) out[0] = 0;
    int fd = open("/data/saaios/etc/sim_pin", O_RDONLY | O_CLOEXEC);
    if (fd < 0) return 0;
    char buf[16] = {0};
    ssize_t r = read(fd, buf, sizeof buf - 1);
    close(fd);
    if (r <= 0) { zero_bytes(buf, sizeof buf); return 0; }
    size_t n = 0;
    while (n < sizeof buf && buf[n] >= '0' && buf[n] <= '9') n++;
    if (n < 4 || n > CALL_PIN_MAX || n >= cap) { zero_bytes(buf, sizeof buf); return 0; }
    memcpy(out, buf, n);
    out[n] = 0;
    zero_bytes(buf, sizeof buf);
    return 1;
}

/* VERDICT 16: load the one-shot activation dial string from
 * /data/saaios/etc/call_number (an ordinary dial string, not a secret).
 * Accepts a leading '+' followed by digits/'*'/'#', up to CALL_DIAL_NUM_MAX. */
static int read_call_number(char *out, size_t cap)
{
    if (cap) out[0] = 0;
    int fd = open("/data/saaios/etc/call_number", O_RDONLY | O_CLOEXEC);
    if (fd < 0) return 0;
    char buf[CALL_DIAL_NUM_MAX + 2] = {0};
    ssize_t r = read(fd, buf, sizeof buf - 1);
    close(fd);
    if (r <= 0) return 0;
    size_t n = 0;
    if (buf[0] == '+') n = 1;
    while (n < sizeof buf && (( buf[n] >= '0' && buf[n] <= '9') ||
                              buf[n] == '*' || buf[n] == '#'))
        n++;
    if (n < 2 || n >= cap) return 0;
    memcpy(out, buf, n);
    out[n] = 0;
    return 1;
}

/* One-shot SMS destination. Same file shape as the dial string, digits only,
 * and never written to the log. */
static int read_sms_number(char *out, size_t cap)
{
    if (cap) out[0] = 0;
    int fd = open("/data/saaios/etc/sms_number", O_RDONLY | O_CLOEXEC);
    if (fd < 0) return 0;
    char buf[SMS_DIGITS_MAX + 3] = {0};
    ssize_t r = read(fd, buf, sizeof buf - 1);
    close(fd);
    if (r <= 0) return 0;
    size_t n = 0;
    if (buf[0] == '+') n = 1;
    while (n < sizeof buf && buf[n] >= '0' && buf[n] <= '9') n++;
    if (n < 2 || n >= cap) return 0;
    if (n == 1 || (buf[0] == '+' && n - 1 > SMS_DIGITS_MAX) ||
        (buf[0] != '+' && n > SMS_DIGITS_MAX))
        return 0;
    memcpy(out, buf, n);
    out[n] = 0;
    return 1;
}
#endif /* SAAIOS_RFS_CAMP */

static void request_stop(int signal_number)
{
    (void)signal_number;
    stop_requested = 1;
}

static int parse_fd(const char *text)
{
    char *end = NULL;
    errno = 0;
    long value = strtol(text, &end, 10);
    return errno || end == text || *end || value < 3 || value > INT_MAX ?
           -1 : (int)value;
}

static int parse_args(int argc, char **argv, int *ipc, int *rfs, int *ready)
{
    *ipc = *rfs = *ready = -1;
    if (argc != 7) return -1;
    for (int i = 1; i < argc; i += 2) {
        int fd = parse_fd(argv[i + 1]);
        if (fd < 0) return -1;
        if (!strcmp(argv[i], "--ipc-fd") && *ipc < 0) *ipc = fd;
        else if (!strcmp(argv[i], "--rfs-fd") && *rfs < 0) *rfs = fd;
        else if (!strcmp(argv[i], "--ready-fd") && *ready < 0) *ready = fd;
        else return -1;
    }
    return *ipc >= 0 && *rfs >= 0 && *ready >= 0 &&
           *ipc != *rfs && *ipc != *ready && *rfs != *ready ? 0 : -1;
}

static void close_if_open(int fd)
{
    if (fd >= 0) (void)close(fd);
}

static int run_owner(int ipc, int rfs, int ready)
{
    struct owner o = {
        .ipc = ipc, .rfs = rfs, .ready = ready,
        .lock = -1, .source = -1, .pin_fd = -1,
        .source_dir = -1, .pin_dir = -1,
        .quarantine_parent = -1, .quarantine_dir = -1,
        .candidate = -1, .marker_fd = -1, .sidecar_fd = -1,
        .phase = WAIT_7
    };
    struct rlimit no_core = {0, 0};
    struct sigaction action = {0};
    enum cp_state state = CP_UNKNOWN;
    int rc = 1;
    if (geteuid() != 0 || setrlimit(RLIMIT_CORE, &no_core) ||
        prctl(PR_SET_DUMPABLE, 0, 0, 0, 0)) {
        fputs("ABORT root or dump protection unavailable\n", stderr);
        goto done;
    }
    (void)umask(0077);
    action.sa_handler = request_stop;
    sigemptyset(&action.sa_mask);
    if (sigaction(SIGTERM, &action, NULL) ||
        sigaction(SIGINT, &action, NULL) ||
        signal(SIGHUP, SIG_IGN) == SIG_ERR ||
        signal(SIGPIPE, SIG_IGN) == SIG_ERR ||
        verify_device(ipc, "/sys/class/cpif/umts_ipc0/dev") ||
        verify_device(rfs, "/sys/class/cpif/umts_rfs0/dev") ||
        ready_pipe(ready) || cp_state() != CP_BOOTING) {
        fputs("ABORT owner handoff descriptors or CP state refused\n", stderr);
        goto done;
    }
    if (original_efs_unmounted()) {
        fputs("ABORT original EFS mount gate refused\n", stderr);
        goto done;
    }
    o.lock = acquire_lock();
    if (o.lock < 0 || opened_once(ipc) || opened_once(rfs) ||
        original_efs_unmounted() || load_source_and_pin(&o) ||
        consume_pin(&o) ||
        prepare_quarantine(&o) || !stable_sources(&o) ||
        candidate_stable(&o) || original_efs_unmounted() ||
        cp_state() != CP_BOOTING || opened_once(ipc) || opened_once(rfs) ||
        stop_requested) {
        fputs("ABORT source, quarantine, EFS, or exclusive owner gate refused\n",
              stderr);
        goto done;
    }
    int64_t started = monotonic_ms();
    if (started < 0 || send_ready(ready)) {
        fputs("ABORT READY unavailable\n", stderr);
        goto done;
    }
    setvbuf(stdout, NULL, _IOLBF, 0);
    puts("owner=rfs-full-quarantine-ready payload=redacted "
         "responses_max=97 grants_max=95 no_promotion=1");
#ifdef SAAIOS_RFS_CAMP
    puts("owner=rfs-camp-combined camp=armed "
         "trigger=0x0803-0x0802-raw0 seq=0x093f,0x0404,0x0800 "
         "probe=sim,signal,voice,data sim_drive=0x0210->0x0200 "
         "reg=sel_auto,pref_lte_wcdma,allow_data");
#endif
    sha_init(&o.received_hash);
    o.deadline_ms = started + FIRST_DEADLINE_MS;
    o.total_deadline_ms = started + TOTAL_DEADLINE_MS;
    o.sit.ready_ms = started;
    o.sit.event_window_ms = started;
#ifdef SAAIOS_RFS_CAMP
    o.camp.owner_start_ms = started;
    o.camp.opx_step = read_opx_step();
    printf("camp_opx_step=%d\n", o.camp.opx_step);
    if (read_apn(o.camp.apn, sizeof o.camp.apn))
        printf("camp_apn=loaded len=%zu\n", strlen(o.camp.apn));
    else
        puts("camp_apn=none");
    o.camp.pref_target = read_pref_rat();
    printf("camp_pref_target=%u\n", camp_pref_target(&o.camp));
    o.camp.scan_enabled = read_do_scan();
    printf("camp_scan=%s\n", o.camp.scan_enabled ? "armed" : "off");
    o.camp.dereg_enabled = read_dereg_scan();
    printf("camp_dereg=%s\n", o.camp.dereg_enabled ? "armed" : "off");
    o.camp.scan16_enabled = read_scan16();
    printf("camp_scan16=%s\n", o.camp.scan16_enabled ? "armed" : "off");
    o.camp.ratbm_enabled = read_ratbm();
    printf("camp_ratbm=%s\n", o.camp.ratbm_enabled ? "armed" : "off");
    o.camp.bbver_enabled = read_bbver();
    printf("camp_bbver=%s\n", o.camp.bbver_enabled ? "armed" : "off");
    o.camp.capquery = read_capquery();
    if (o.camp.capquery) {
        /* Read-only capability diagnostic: force the bitmap/band GETs and the
         * baseband GET on, and guarantee no operator-control/registration SET
         * can fire (camp_reg_next/camp_ratbm_next gate every SET on !capquery). */
        o.camp.ratbm_enabled = 1;
        o.camp.bbver_enabled = 1;
    }
    printf("camp_capquery=%s\n",
           o.camp.capquery ? "armed-readonly" : "off");
    o.camp.scan734 = read_scan734();
    printf("camp_scan734=%s\n",
           o.camp.scan734 ? "armed-precamp-lte-scan" : "off");
    if (read_manual_plmn(o.camp.manual_plmn, sizeof o.camp.manual_plmn))
        printf("camp_manual_plmn=armed len=%zu\n", strlen(o.camp.manual_plmn));
    else
        puts("camp_manual_plmn=none");
    /* VERDICT 16: PIN1 unlock + one-shot activation call. The PIN is loaded but
     * never logged (only presence is reported); the dial number is config. */
    if (read_sim_pin(o.camp.pin, sizeof o.camp.pin))
        puts("camp_sim_pin=loaded");
    else
        puts("camp_sim_pin=none");
    if (read_call_number(o.camp.call_number, sizeof o.camp.call_number)) {
        o.camp.call_enabled = 1;
        printf("camp_call_number=armed len=%zu\n", strlen(o.camp.call_number));
    } else {
        puts("camp_call_number=none");
    }
    if (read_sms_number(o.camp.sms_number, sizeof o.camp.sms_number)) {
        o.camp.sms_enabled = 1;
        printf("camp_sms_number=armed len=%zu\n", strlen(o.camp.sms_number));
    } else {
        puts("camp_sms_number=none");
    }
#endif
    for (;;) {
        int64_t now = monotonic_ms();
        if (now >= 0) sit_event_trace_window(&o.sit, now);
        state = cp_state();
        if (state == CP_OFFLINE) {
            if (o.phase != TERMINAL) {
                if (o.phase == WAIT_DATA &&
                    frame_stage(&o) == STAGE_FINAL_FRAME)
                    diagnose_waiting(&o);
                terminal(&o, "cp_offline");
            }
            break;
        }
        if (now < 0) {
            terminal(&o, "clock_refused");
        } else if (state == CP_CRASH || state == CP_UNKNOWN) {
            terminal(&o, "cp_state_lost");
        } else if (state == CP_BOOTING &&
                   now - started > BOOTING_LIMIT_MS) {
            terminal(&o, "booting_deadline");
        } else if (o.phase != TERMINAL && now >= o.total_deadline_ms) {
            diagnose_waiting(&o);
            terminal(&o, "total_deadline");
        } else if (o.phase != TERMINAL && now >= o.deadline_ms) {
            diagnose_waiting(&o);
            terminal(&o, "step_deadline");
        }
        if (stop_requested) {
            terminal(&o, "requested_stop");
            /* Close the inherited descriptors and return. Do not power the CP off. */
            break;
        }
        if (o.phase == TERMINAL && !o.final_ack_sent && !o.sit.poisoned)
            sit_disable(&o.sit, "rfs_terminal");
        if (state != CP_ONLINE) {
            (void)poll(NULL, 0, POLL_MS);
            continue;
        }
        struct pollfd fds[2] = {
            {.fd = (o.phase == TERMINAL && !o.final_ack_sent) ? -1 : rfs,
             .events = POLLIN},
            {.fd = o.sit.endpoint_failed ||
                   now < o.sit.backoff_until_ms ? -1 : ipc,
             .events = POLLIN}
        };
        int events = poll(fds, 2, POLL_MS);
        if (events < 0 && errno == EINTR) continue;
        if (events < 0) {
            terminal(&o, "poll_refused");
            (void)poll(NULL, 0, POLL_MS);
            continue;
        }
        poll_faults_before_rfs(&o, fds[0].revents, fds[1].revents);
        if (o.phase == TERMINAL && !o.final_ack_sent && !o.sit.poisoned)
            sit_disable(&o.sit, "rfs_terminal");
        for (size_t i = 0; i < 2; ++i) {
            if (fds[i].revents & (POLLERR | POLLHUP | POLLNVAL)) continue;
            if (i == 0 && o.phase == TERMINAL && !o.final_ack_sent) continue;
            if (!(fds[i].revents & POLLIN)) continue;
            uint8_t bytes[RX_CAP];
            size_t cap = i == 0 ? sizeof bytes : SIT_READ_SLICE;
            ssize_t got = read(fds[i].fd, bytes, cap);
            if (got < 0 && errno == EINTR) continue;
            if (got == 0 || (got < 0 &&
                (errno == EAGAIN || errno == EWOULDBLOCK))) {
                if (i == 0) (void)poll(NULL, 0, EMPTY_BACKOFF_MS);
                else o.sit.backoff_until_ms = now + EMPTY_BACKOFF_MS;
                continue;
            }
            if (got < 0) {
                if (i == 0) {
                    diagnose(&o, frame_stage(&o), REASON_IO);
                    terminal(&o, "rfs_read_refused");
                }
                else sit_endpoint_fault(&o);
            } else if (i == 0) {
                int64_t received_at = monotonic_ms();
                if (o.phase == TERMINAL) {
                    if (received_at >= 0)
                        post_terminal_rfs_drain(&o, bytes, (size_t)got,
                                                received_at);
                    zero_bytes(bytes, sizeof bytes);
                    continue;
                }
                if (received_at < 0 || received_at >= o.deadline_ms)
                    diagnose(&o, frame_stage(&o), REASON_TIMEOUT);
                if (received_at < 0 || received_at >= o.deadline_ms ||
                    feed_rfs(&o, bytes, (size_t)got, received_at))
                    terminal(&o, "rfs_frame_or_io_refused");
                if (o.phase == TERMINAL && !o.final_ack_sent &&
                    !o.sit.poisoned)
                    sit_disable(&o.sit, "rfs_terminal");
            } else {
                int64_t received_at = monotonic_ms();
                if (received_at < 0) sit_poison(&o.sit);
                else sit_feed(&o.sit, bytes, (size_t)got, received_at);
#ifdef SAAIOS_RFS_CAMP
                if (received_at >= 0)
                    camp_feed(&o.camp, bytes, (size_t)got, received_at);
#endif
            }
            zero_bytes(bytes, sizeof bytes);
        }
        int64_t observed_at = monotonic_ms();
#ifndef SAAIOS_RFS_CAMP
        if (observed_at >= 0) sit_advance(&o, observed_at);
#else
        /* Camp mode drives IPC via the active prober, not the passive SIT
         * observer, so it never self-poisons on a settled-GET timeout. The
         * observer definition is retained for the host self-test. */
        (void)sit_advance;
        if (observed_at >= 0) camp_advance(&o, observed_at);
        if (observed_at >= 0) camp_probe_advance(&o, observed_at);
#endif
    }
    if (state != CP_OFFLINE)
        fputs("WARNING owner exit before CP OFFLINE may purge RX\n", stderr);
    if (o.chunks_stored &&
        (candidate_stable(&o) || !stable_sources(&o) ||
         original_efs_unmounted())) {
        fputs("ABORT post-run quarantine or source identity changed\n", stderr);
        rc = 1;
    } else {
        rc = o.final_ack_sent && sidecar_stable(&o) == 0 ? 0 : 1;
    }
done:
    close_if_open(o.candidate);
    close_if_open(o.sidecar_fd);
    close_if_open(o.marker_fd);
    close_if_open(o.quarantine_dir);
    close_if_open(o.quarantine_parent);
    close_if_open(o.source);
    close_if_open(o.pin_fd);
    close_if_open(o.source_dir);
    close_if_open(o.pin_dir);
    close_if_open(o.lock);
    close_if_open(ready);
    close_if_open(rfs);
    close_if_open(ipc);
    zero_bytes(o.pin_digest, sizeof o.pin_digest);
    zero_bytes(o.candidate_digest, sizeof o.candidate_digest);
    zero_bytes(&o.received_hash, sizeof o.received_hash);
    zero_bytes(o.rx, sizeof o.rx);
    zero_bytes(o.sit.rx, sizeof o.sit.rx);
#ifdef SAAIOS_RFS_CAMP
    zero_bytes(o.camp.rx, sizeof o.camp.rx);
#endif
    return rc;
}

static int test_framing(void)
{
    int64_t now = monotonic_ms();
    if (now < 0) return -1;
    struct owner o = {.phase = WAIT_3, .deadline_ms = now + 10000,
                      .total_deadline_ms = now + 10000};
    uint8_t coalesced[sizeof request_3 + 12];
    if (feed_rfs(&o, request_3, 7, now) || o.phase != WAIT_3 || o.used != 7 ||
        feed_rfs(&o, request_3 + 7, sizeof request_3 - 7, now) ||
        o.phase != WAIT_6 || o.used != 0)
        return -1;
    memset(&o, 0, sizeof o);
    o.phase = WAIT_3;
    o.deadline_ms = o.total_deadline_ms = now + 10000;
    memcpy(coalesced, request_3, sizeof request_3);
    memcpy(coalesced + sizeof request_3, request_6, 12);
    if (feed_rfs(&o, coalesced, sizeof coalesced, now) ||
        o.phase != WAIT_6 || o.used != 12)
        return -1;
    o.phase = TERMINAL;
    if (feed_rfs(&o, request_6, sizeof request_6, now) == 0)
        return -1;
    zero_bytes(coalesced, sizeof coalesced);
    zero_bytes(&o, sizeof o);
    return 0;
}

#ifdef RFS_HOST_TEST
static uint8_t host_last_reply[20];
static size_t host_last_reply_len;
static uint8_t host_last_sit_request[12];

static ssize_t host_short_write(int fd, const void *bytes, size_t len)
{
    (void)fd; (void)bytes;
    return (ssize_t)len - 1;
}

static ssize_t host_full_write(int fd, const void *bytes, size_t len)
{
    (void)fd;
    if (len > sizeof host_last_reply) return -1;
    memcpy(host_last_reply, bytes, len);
    host_last_reply_len = len;
    return (ssize_t)len;
}

static ssize_t host_sit_full_write(int fd, const void *bytes, size_t len)
{
    (void)fd;
    if (len != sizeof host_last_sit_request) return -1;
    memcpy(host_last_sit_request, bytes, len);
    return (ssize_t)len;
}

static ssize_t host_sit_short_write(int fd, const void *bytes, size_t len)
{
    (void)fd; (void)bytes;
    return (ssize_t)len - 1;
}

static void host_sit_reply(uint8_t *frame, size_t length, uint16_t id,
                           uint32_t token, uint16_t error)
{
    memset(frame, 0, length);
    frame[0] = 1;
    frame[2] = (uint8_t)id;
    frame[3] = (uint8_t)(id >> 8);
    frame[4] = (uint8_t)length;
    frame[5] = (uint8_t)(length >> 8);
    put_little32(frame + 6, token);
    frame[10] = (uint8_t)error;
    frame[11] = (uint8_t)(error >> 8);
}

static int test_sit_event_trace(void)
{
    struct sit_observer s = {.ready_ms = 100,
                             .event_window_ms = 100};
    uint8_t frame[8] = {2, 0, 0x10, 0x07, 8, 0, 0, 0};
    unsigned id = 0;
    for (unsigned i = 0; i < SIT_EVENT_TRACE_LIMIT; ++i)
        if (!sit_event_trace_take(&s, frame, sizeof frame, &id) ||
            id != 0x0710) return -1;
    frame[3] = 0x08;
    if (sit_event_trace_take(&s, frame, sizeof frame, &id) ||
        s.event_seen != SIT_EVENT_TRACE_LIMIT + 1 ||
        s.event_logged != SIT_EVENT_TRACE_LIMIT ||
        s.event_overflow != 1) return -1;
    frame[0] = 1;
    if (sit_event_trace_take(&s, frame, sizeof frame, &id) ||
        s.event_seen != SIT_EVENT_TRACE_LIMIT + 1) return -1;
    frame[0] = 2;
    frame[4] = 7; /* A malformed header is not a traced indication. */
    if (sit_event_trace_take(&s, frame, sizeof frame, &id) ||
        s.event_seen != SIT_EVENT_TRACE_LIMIT + 1) return -1;
    sit_event_trace_window(&s, 100 + SIT_EVENT_WINDOW_MS);
    if (s.event_seen || s.event_logged || s.event_overflow ||
        s.event_window_ms != 100 + SIT_EVENT_WINDOW_MS) return -1;
    frame[4] = 8;
    if (!sit_event_trace_take(&s, frame, sizeof frame, &id) ||
        id != 0x0810) return -1;
    s.event_window_ms = s.ready_ms + SIT_EVENT_QUIET_COVERAGE_MS - 1;
    if (!sit_event_quiet_window(&s)) return -1;
    s.event_window_ms++;
    if (sit_event_quiet_window(&s)) return -1;
    return 0;
}

static int test_sit_observer(void)
{
    struct sit_observer s = {.pending = 1, .token = 0xabcdef12u};
    struct owner o = {.phase = TERMINAL, .final_ack_sent = 1,
                      .ipc = -1, .rfs = -1};
    uint8_t sim[15 + SIT_SIM_APP_STRIDE], sim_short[15];
    uint8_t radio[16], voice[14], data[16];
    uint8_t combined[32], invalid[6] = {3,0,0,0,12,0};
    uint8_t indication[8] = {2,0,0x10,0x02,8,0,0,0};
    uint8_t final_frame[20 + 318] = {0};
    int64_t now = monotonic_ms();
    int rc = -1;
    if (now < 0) return -1;
    host_sit_reply(sim, sizeof sim, 0x0200, s.token, 0);
    sim[SIT_SIM_CARD] = 1;
    sim[SIT_SIM_APPS] = 1;
    sim[SIT_SIM_APP_STATE] = 5;
    sim[SIT_SIM_PIN1] = 3;
    host_sit_reply(sim_short, sizeof sim_short, 0x0200, s.token, 0);
    sim_short[SIT_SIM_APPS] = 1;
    if (!sit_sim_status_complete(sim, sizeof sim) ||
        sit_sim_status_complete(sim_short, sizeof sim_short)) goto done;
    sit_feed(&s, sim, 5, now);
    if (s.used != 5 || !s.pending || s.next) goto done;
    sit_feed(&s, sim + 5, sizeof sim - 5, now);
    if (s.used || s.pending || s.next != 1) goto done;
    sit_feed(&s, sim, sizeof sim, now); /* Old reply cannot match next field. */
    if (s.next != 1) goto done;
    s.pending = 1;
    ++s.token;
    host_sit_reply(radio, sizeof radio, 0x0801, s.token, 0);
    put_little32(radio + 12, 10);
    memcpy(combined, radio, sizeof radio);
    memcpy(combined + sizeof radio, radio, sizeof radio);
    sit_feed(&s, combined, sizeof combined, now);
    if (s.used || s.pending || s.next != 2) goto done;
    s.pending = 1;
    ++s.token;
    host_sit_reply(voice, sizeof voice, SIT_NET_VOICE_REG, s.token + 1, 0);
    sit_feed(&s, voice, sizeof voice, now); /* Stale token. */
    if (!s.pending || s.next != 2) goto done;
    sit_feed(&s, indication, sizeof indication, now); /* Not a reply. */
    if (!s.pending || s.next != 2) goto done;
    host_sit_reply(data, sizeof data, SIT_NET_DATA_REG, s.token, 0);
    sit_feed(&s, data, sizeof data, now); /* Wrong ID. */
    if (!s.pending || s.next != 2) goto done;
    host_sit_reply(voice, sizeof voice, SIT_NET_VOICE_REG, s.token, 0);
    sit_feed(&s, voice, sizeof voice, now);
    if (s.pending || s.next != 3) goto done;
    s.pending = 1;
    ++s.token;
    host_sit_reply(data, sizeof data, SIT_NET_DATA_REG, s.token, 5);
    sit_feed(&s, data, sizeof data, now); /* Error is unknown, not a scalar. */
    if (s.pending || s.next != 4) goto done;
    s = (struct sit_observer){.pending = 1, .token = 42};
    host_sit_reply(sim_short, sizeof sim_short, 0x0200, s.token, 0);
    sim_short[SIT_SIM_APPS] = 1;
    sit_feed(&s, sim_short, sizeof sim_short, now);
    if (s.pending || s.next != 1 || s.poisoned) goto done;
    s = (struct sit_observer){.pending = 1, .token = 10,
                              .deadline_ms = now - 1, .ready_ms = now};
    host_sit_reply(sim, sizeof sim, 0x0200, s.token, 0);
    sit_feed(&s, sim, sizeof sim, now); /* Late reply is not evidence. */
    if (!s.pending || s.next) goto done;
    o.sit = s;
    host_sit_write_override = host_sit_full_write;
    host_sit_write_calls = 0;
    o.final_ack_sent = 0;
    sit_advance(&o, now);
    if (host_sit_write_calls || o.sit.next != 0 || !o.sit.pending)
        goto done; /* A failed RFS terminal cannot dispatch new GETs. */
    o.final_ack_sent = 1;
    sit_advance(&o, now);
    if (!o.sit.poisoned || o.sit.pending || host_sit_write_calls)
        goto done; /* Timed-out GET cannot be followed by another. */
    o.sit = (struct sit_observer){.ready_ms = now};
    sit_advance(&o, now);
    if (!o.sit.pending || host_sit_write_calls != 1 ||
        little16(host_last_sit_request + 2) != 0x0200 ||
        little16(host_last_sit_request + 4) != 12 ||
        host_last_sit_request[0] != 0) goto done;
    o.sit.deadline_ms = now + 1;
    sit_advance(&o, now + 1);
    sit_advance(&o, now + SIT_SETTLED_MS);
    if (!o.sit.poisoned || o.sit.pending || host_sit_write_calls != 1)
        goto done; /* One send total, even after the settled threshold. */
    o.sit = (struct sit_observer){.ready_ms = now,
                                  .started = 1, .next = SIT_GET_COUNT};
    host_sit_write_calls = 0;
    sit_advance(&o, now + SIT_SETTLED_MS - 1);
    if (o.sit.pass != 1 || o.sit.pending || host_sit_write_calls)
        goto done;
    sit_advance(&o, now + SIT_SETTLED_MS);
    if (o.sit.pass != 1 || !o.sit.pending ||
        host_sit_write_calls != 1 ||
        little16(host_last_sit_request + 2) != 0x0200) goto done;
    o.sit = (struct sit_observer){.ready_ms = now};
    host_sit_write_override = host_sit_short_write;
    host_sit_write_calls = 0;
    sit_advance(&o, now);
    if (!o.sit.poisoned || o.sit.pending ||
        host_sit_write_calls != 1) goto done;
    host_sit_write_override = host_sit_full_write;
    sit_advance(&o, now + SIT_SETTLED_MS);
    if (host_sit_write_calls != 1) goto done; /* No retry or later pass. */
    s = (struct sit_observer){0};
    sit_feed(&s, invalid, sizeof invalid, now);
    if (!s.poisoned || s.used) goto done;
    o.phase = WAIT_6;
    o.final_ack_sent = 0;
    o.sit = (struct sit_observer){0};
    sit_endpoint_fault(&o);
    if (o.phase != TERMINAL || !o.sit.poisoned ||
        !o.sit.endpoint_failed) goto done;
    o.phase = TERMINAL;
    o.final_ack_sent = 1;
    o.sit = (struct sit_observer){0};
    sit_endpoint_fault(&o);
    if (o.phase != TERMINAL || !o.final_ack_sent || !o.sit.poisoned ||
        !o.sit.endpoint_failed)
        goto done;
    final_frame[0] = 2;
    final_frame[2] = 1;
    put_little32(final_frame + 4, 12 + 318);
    put_little32(final_frame + 12, 3);
    put_little32(final_frame + 16, 318);
    o.phase = WAIT_DATA;
    o.final_ack_sent = 0;
    o.final_ack_attempted = 0;
    o.sit = (struct sit_observer){0};
    o.deadline_ms = o.total_deadline_ms = now + 10000;
    host_write_override = host_full_write;
    host_write_calls = 0;
    poll_faults_before_rfs(&o, POLLIN, POLLERR);
    if (o.phase != TERMINAL || !o.sit.poisoned ||
        o.final_ack_attempted || host_write_calls ||
        feed_rfs(&o, final_frame, sizeof final_frame, now) == 0)
        goto done; /* Simultaneous final RFS data + IPC fault: no ACK. */
    rc = 0;
done:
    host_write_override = NULL;
    host_write_calls = 0;
    host_sit_write_override = NULL;
    host_sit_write_calls = 0;
    zero_bytes(host_last_sit_request, sizeof host_last_sit_request);
    zero_bytes(&s, sizeof s);
    zero_bytes(&o.sit, sizeof o.sit);
    zero_bytes(sim, sizeof sim);
    zero_bytes(sim_short, sizeof sim_short);
    zero_bytes(radio, sizeof radio);
    zero_bytes(voice, sizeof voice);
    zero_bytes(data, sizeof data);
    zero_bytes(combined, sizeof combined);
    zero_bytes(final_frame, sizeof final_frame);
    return rc;
}

static int host_sync_fail(int fd)
{
    (void)fd;
    return -1;
}

static int host_readback_corrupt(int fd, void *out, size_t len, off_t offset)
{
    if (read_all_at(fd, out, len, offset)) return -1;
    if (offset == 0 && len == FIRST_CHUNK)
        ((uint8_t *)out)[0] ^= 1;
    return 0;
}

static int test_host_storage_faults(void)
{
    static const struct chunk_io actual = {fsync, read_all_at};
    static const struct chunk_io failed_sync = {host_sync_fail, read_all_at};
    static const struct chunk_io failed_readback = {fsync, host_readback_corrupt};
    enum failure_reason reason = REASON_NONE;
    uint8_t zero[4096] = {0}, chunk[FIRST_CHUNK], verify[FIRST_CHUNK];
    FILE *source = tmpfile(), *candidate = tmpfile();
    int rc = -1;
    if (!source || !candidate) goto done;
    memset(chunk, 0x5a, sizeof chunk);
    int source_fd = fileno(source), candidate_fd = fileno(candidate);
    if (source_fd < 0 || candidate_fd < 0) goto done;
    for (off_t off = 0; off < BASELINE_BYTES; off += sizeof zero) {
        if (write_all_at(source_fd, zero, sizeof zero, off) ||
            write_all_at(candidate_fd, zero, sizeof zero, off)) goto done;
    }
    if (apply_chunk(candidate_fd, source_fd, 0, chunk, FIRST_CHUNK,
                    &failed_sync, &reason) == 0 || reason != REASON_IO ||
        apply_chunk(candidate_fd, source_fd, 0, chunk, FIRST_CHUNK,
                    &failed_readback, &reason) == 0 ||
        reason != REASON_READBACK ||
        apply_chunk(candidate_fd, source_fd, 0, chunk, FIRST_CHUNK,
                    &actual, NULL) ||
        read_all_at(candidate_fd, verify, FIRST_CHUNK, 0) ||
        !same_bytes(verify, chunk, FIRST_CHUNK) ||
        apply_chunk(candidate_fd, source_fd, RFS_TRANSFER_BYTES - 318,
                    chunk, 318, &actual, NULL) ||
        read_all_at(candidate_fd, verify, 318,
                    RFS_TRANSFER_BYTES - 318) ||
        !same_bytes(verify, chunk, 318) ||
        apply_chunk(candidate_fd, source_fd, RFS_TRANSFER_BYTES - 317,
                    chunk, 318, &actual, NULL) == 0)
        goto done;
    rc = 0;
done:
    if (source) fclose(source);
    if (candidate) fclose(candidate);
    zero_bytes(zero, sizeof zero);
    zero_bytes(chunk, sizeof chunk);
    zero_bytes(verify, sizeof verify);
    return rc;
}

static int test_host_failure_diagnostics(void)
{
    struct owner o = {0};
    uint8_t frame[20 + 318 + 1] = {0};
    int64_t now = monotonic_ms();
    int rc = -1;
    if (now < 0) return -1;
    o.deadline_ms = o.total_deadline_ms = now + 10000;
    o.phase = WAIT_DATA;
    o.grant_attempted = o.chunks_stored = RFS_GRANTS_MAX - 1;
    o.received_bytes = (RFS_GRANTS_MAX - 1) * FIRST_CHUNK;
    host_gate_override = -1;
    host_write_override = host_short_write;
    host_write_calls = 0;
    if (send_next_grant(&o) == 0 ||
        o.failure_stage != STAGE_FINAL_GRANT ||
        o.failure_reason != REASON_GATE ||
        o.grant_attempted != RFS_GRANTS_MAX - 1 || host_write_calls)
        goto done;
    o.failure_stage = STAGE_NONE;
    o.failure_reason = REASON_NONE;
    host_gate_override = 1;
    if (send_next_grant(&o) == 0 ||
        o.failure_stage != STAGE_FINAL_GRANT ||
        o.failure_reason != REASON_SEND ||
        o.grant_attempted != RFS_GRANTS_MAX || host_write_calls != 1)
        goto done;

    o.failure_stage = STAGE_NONE;
    o.failure_reason = REASON_NONE;
    o.expected_chunk = 318;
    put_little32(frame + 4, 12 + 318);
    put_little32(frame + 12, 3);
    put_little32(frame + 16, 318);
    frame[0] = 2;
    frame[2] = 2;
    if (feed_rfs(&o, frame, 20 + 318, now) == 0 ||
        o.failure_stage != STAGE_FINAL_FRAME ||
        o.failure_reason != REASON_MALFORMED ||
        o.frame_mismatch_mask != MISMATCH_SEQUENCE) goto done;
    o.failure_stage = STAGE_NONE;
    o.failure_reason = REASON_NONE;
    o.frame_mismatch_mask = 0;
    o.used = 0;
    frame[2] = 1;
    if (feed_rfs(&o, frame, sizeof frame, now) == 0 ||
        o.failure_stage != STAGE_FINAL_FRAME ||
        o.failure_reason != REASON_TRAILING ||
        o.frame_mismatch_mask) goto done;
    o.failure_stage = STAGE_NONE;
    o.failure_reason = REASON_NONE;
    o.used = 0;
    diagnose_waiting(&o);
    if (o.failure_stage != STAGE_FINAL_FRAME ||
        o.failure_reason != REASON_NO_READ) goto done;
    o.failure_stage = STAGE_NONE;
    o.failure_reason = REASON_NONE;
    o.used = 7;
    diagnose_waiting(&o);
    if (o.failure_stage != STAGE_FINAL_FRAME ||
        o.failure_reason != REASON_PARTIAL_READ) goto done;
    o.failure_stage = STAGE_NONE;
    o.failure_reason = REASON_NONE;
    host_gate_override = -1;
    if (store_chunk(&o, frame + 20) == 0 ||
        o.failure_stage != STAGE_STORE_CHUNK ||
        o.failure_reason != REASON_GATE) goto done;
    o.failure_stage = STAGE_NONE;
    o.failure_reason = REASON_NONE;
    o.used = 0;
    host_gate_override = 1;
    o.candidate = -1;
    if (store_chunk(&o, frame + 20) == 0 ||
        o.failure_stage != STAGE_STORE_CHUNK ||
        o.failure_reason != REASON_IO) goto done;
    rc = 0;
done:
    host_gate_override = 0;
    host_write_override = NULL;
    host_write_calls = 0;
    zero_bytes(frame, sizeof frame);
    zero_bytes(&o, sizeof o);
    return rc;
}

static int test_host_final_frame_shape(void)
{
    struct owner o = {0};
    uint8_t frame[RFS_FRAME_MAX + 1] = {0};
    char line[512];
    FILE *log = tmpfile();
    int64_t now = monotonic_ms();
    int rc = -1;
    if (!log || now < 0) goto done;
    o.phase = WAIT_DATA;
    o.grant_attempted = RFS_GRANTS_MAX;
    o.chunks_stored = RFS_GRANTS_MAX - 1;
    o.expected_chunk = 318;
    o.deadline_ms = o.total_deadline_ms = now + 10000;
    frame[0] = 2;
    frame[2] = 1;
    put_little32(frame + 4, RFS_FRAME_MAX - 8);
    put_little32(frame + 12, 3);
    put_little32(frame + 16, 318);
    if (feed_rfs(&o, frame, RFS_FRAME_MAX, now) == 0 ||
        o.failure_stage != STAGE_FINAL_FRAME ||
        o.failure_reason != REASON_MALFORMED ||
        o.frame_mismatch_mask !=
            (MISMATCH_LENGTH | MISMATCH_PAYLOAD_SIZE) ||
        o.final_parsed_len != 2032 || o.final_outer_payload_len != 2024 ||
        o.final_trailing != 0 || o.final_padding_zero != PADDING_YES)
        goto done;
    log_terminal(log, &o, "rfs_frame_or_io_refused");
    if (fflush(log) || fseek(log, 0, SEEK_SET) ||
        !fgets(line, sizeof line, log) ||
        !strstr(line, "frame_mismatch_mask=0x09") ||
        !strstr(line, "final_parsed_len=2032 final_outer_payload_len=2024 "
                      "final_trailing=0 final_padding_zero=1"))
        goto done;

    memset(&o, 0, sizeof o);
    o.phase = WAIT_DATA;
    o.grant_attempted = RFS_GRANTS_MAX;
    o.chunks_stored = RFS_GRANTS_MAX - 1;
    o.expected_chunk = 318;
    o.deadline_ms = o.total_deadline_ms = now + 10000;
    frame[20 + 318] = 1;
    if (feed_rfs(&o, frame, sizeof frame, now) == 0 ||
        o.final_parsed_len != 2032 || o.final_outer_payload_len != 2024 ||
        o.final_trailing != 1 || o.final_padding_zero != PADDING_NO)
        goto done;

    memset(&o, 0, sizeof o);
    o.phase = WAIT_DATA;
    o.grant_attempted = RFS_GRANTS_MAX;
    o.chunks_stored = RFS_GRANTS_MAX - 1;
    o.expected_chunk = 318;
    o.deadline_ms = o.total_deadline_ms = now + 10000;
    put_little32(frame + 4, 12);
    if (feed_rfs(&o, frame, 20, now) == 0 ||
        o.final_parsed_len != 20 || o.final_outer_payload_len != 12 ||
        o.final_trailing != 0 || o.final_padding_zero != PADDING_UNKNOWN)
        goto done;
    rc = 0;
done:
    if (log) fclose(log);
    zero_bytes(frame, sizeof frame);
    zero_bytes(line, sizeof line);
    zero_bytes(&o, sizeof o);
    return rc;
}

static int host_reject_data_frame(const uint8_t *frame, size_t len,
                                  int final, int64_t now)
{
    struct owner o = {.phase = WAIT_DATA, .rfs = -1,
                      .deadline_ms = now + 10000,
                      .total_deadline_ms = now + 10000,
                      .grant_attempted = final ? RFS_GRANTS_MAX : 1,
                      .chunks_stored = final ? RFS_GRANTS_MAX - 1 : 0,
                      .received_bytes = final ? 94 * FIRST_CHUNK : 0,
                      .expected_chunk = final ? 318 : FIRST_CHUNK};
    sha_init(&o.received_hash);
    host_gate_override = 1;
    host_store_override = 1;
    host_finish_override = 1;
    host_write_override = host_full_write;
    host_write_calls = 0;
    int refused = feed_rfs(&o, frame, len, now);
    int valid = refused != 0 && o.phase == WAIT_DATA &&
                o.grant_attempted == (final ? RFS_GRANTS_MAX : 1) &&
                o.chunks_stored == (final ? RFS_GRANTS_MAX - 1 : 0) &&
                o.received_bytes == (final ? 94 * FIRST_CHUNK : 0) &&
                !o.final_ack_attempted && !o.final_ack_sent &&
                host_write_calls == 0;
    host_gate_override = 0;
    host_store_override = 0;
    host_finish_override = 0;
    host_write_override = NULL;
    host_write_calls = 0;
    zero_bytes(&o, sizeof o);
    return valid ? 0 : -1;
}

static int test_host_padded_final_refusals(void)
{
    uint8_t frame[RFS_FRAME_MAX + 2] = {0};
    int64_t now = monotonic_ms();
    int rc = -1;
    if (now < 0) return -1;
    frame[0] = 2;
    frame[2] = 1;
    put_little32(frame + 4, 332);
    put_little32(frame + 12, 3);
    put_little32(frame + 16, 318);
    frame[338] = 1;
    if (host_reject_data_frame(frame, 340, 1, now)) goto done;
    frame[338] = 0;
    frame[339] = 1;
    if (host_reject_data_frame(frame, 340, 1, now)) goto done;
    frame[339] = 0;
    put_little32(frame + 4, 331);
    if (host_reject_data_frame(frame, 340, 1, now) ||
        host_reject_data_frame(frame, 339, 1, now)) goto done;
    put_little32(frame + 4, 333);
    if (host_reject_data_frame(frame, 341, 1, now)) goto done;
    put_little32(frame + 4, 332);
    if (host_reject_data_frame(frame, 341, 1, now)) goto done;
    frame[0] = 3;
    if (host_reject_data_frame(frame, 340, 1, now)) goto done;
    frame[0] = 2;
    frame[2] = 2;
    if (host_reject_data_frame(frame, 340, 1, now)) goto done;
    frame[2] = 1;
    frame[8] = 1;
    if (host_reject_data_frame(frame, 340, 1, now)) goto done;
    frame[8] = 0;
    frame[12] = 4;
    if (host_reject_data_frame(frame, 340, 1, now)) goto done;
    frame[12] = 3;
    put_little32(frame + 16, 319);
    if (host_reject_data_frame(frame, 340, 1, now)) goto done;
    put_little32(frame + 16, FIRST_CHUNK);
    put_little32(frame + 4, 12 + FIRST_CHUNK + 2);
    if (host_reject_data_frame(frame, RFS_FRAME_MAX + 2, 0, now)) goto done;
    rc = 0;
done:
    zero_bytes(frame, sizeof frame);
    return rc;
}

/* Test the real finish/sidecar durability path with only device gates mocked. */
static int test_host_finish_candidate(int fault, int padded_final)
{
    char parent_path[] = "/tmp/saaios-rfs-full-test-XXXXXX";
    static const char leaf[] = "run";
    uint8_t zero[4096] = {0}, chunk[FIRST_CHUNK], digest[32], again[32];
    uint8_t frame[20 + 318 + 2] = {0};
    struct sha256 source_hash;
    struct owner o = {.source = -1, .candidate = -1, .sidecar_fd = -1,
                      .quarantine_parent = -1, .quarantine_dir = -1};
    FILE *source = NULL;
    int parent_created = 0, child_created = 0, rc = -1;
    if (!mkdtemp(parent_path)) goto done;
    parent_created = 1;
    o.quarantine_parent = open(parent_path,
                                O_RDONLY | O_DIRECTORY | O_NOFOLLOW |
                                O_CLOEXEC);
    if (o.quarantine_parent < 0 ||
        mkdirat(o.quarantine_parent, leaf, 0700)) goto done;
    child_created = 1;
    o.quarantine_dir = openat(o.quarantine_parent, leaf,
                              O_RDONLY | O_DIRECTORY | O_NOFOLLOW |
                              O_CLOEXEC);
    if (o.quarantine_dir < 0) goto done;
    memcpy(o.quarantine_leaf, leaf, sizeof leaf);
    o.candidate = openat(o.quarantine_dir, CANDIDATE_NAME,
                         O_RDWR | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC,
                         0600);
    source = tmpfile();
    if (o.candidate < 0 || !source) goto done;
    o.source = fileno(source);
    if (o.source < 0) goto done;
    sha_init(&source_hash);
    for (off_t off = 0; off < BASELINE_BYTES; off += sizeof zero) {
        if (write_all_at(o.source, zero, sizeof zero, off) ||
            write_all_at(o.candidate, zero, sizeof zero, off)) goto done;
        sha_update(&source_hash, zero, sizeof zero);
    }
    sha_final(&source_hash, o.pin_digest);
    sha_init(&o.received_hash);
    for (unsigned i = 0; i < RFS_GRANTS_MAX; ++i) {
        uint32_t length = i == RFS_GRANTS_MAX - 1 ? 318 : FIRST_CHUNK;
        uint32_t offset = i * FIRST_CHUNK;
        memset(chunk, (int)(i + 1), length);
        if (write_all_at(o.candidate, chunk, length, offset)) goto done;
        sha_update(&o.received_hash, chunk, length);
    }
    o.received_bytes = RFS_TRANSFER_BYTES;
    o.chunks_stored = o.grant_attempted = RFS_GRANTS_MAX;
    int64_t now = monotonic_ms();
    if (now < 0 || fsync(o.candidate) ||
        fstat(o.candidate, &o.candidate_stat) ||
        fstat(o.quarantine_parent, &o.quarantine_parent_stat) ||
        fstat(o.quarantine_dir, &o.quarantine_dir_stat)) goto done;
    o.deadline_ms = o.total_deadline_ms = now + 10000;
    host_gate_override = 1; /* Non-root test fixture only. */
    if (fault) {
        sha_init(&o.received_hash);
        for (unsigned i = 0; i < RFS_GRANTS_MAX - 1; ++i) {
            memset(chunk, (int)(i + 1), FIRST_CHUNK);
            sha_update(&o.received_hash, chunk, FIRST_CHUNK);
        }
        o.received_bytes = 94 * FIRST_CHUNK;
        o.chunks_stored = RFS_GRANTS_MAX - 1;
        o.grant_attempted = RFS_GRANTS_MAX;
        o.expected_chunk = 318;
        o.phase = WAIT_DATA;
        host_store_override = 1;
        host_write_override = host_short_write;
        host_write_calls = 0;
        host_sidecar_close_error = fault == 1;
        host_sidecar_sync_error = fault == 2;
        memset(frame, RFS_GRANTS_MAX, sizeof frame);
        frame[0] = 2; frame[1] = 0;
        frame[2] = 1; frame[3] = 0;
        put_little32(frame + 4, 12 + 318);
        put_little32(frame + 8, 0);
        put_little32(frame + 12, 3);
        put_little32(frame + 16, 318);
        size_t frame_len = 20 + 318;
        if (padded_final) {
            frame_len += 2;
            put_little32(frame + 4, 332);
            frame[338] = frame[339] = 0;
        }
        struct stat sidecar;
        if (feed_rfs(&o, frame, frame_len, now) == 0 ||
            o.chunks_stored != RFS_GRANTS_MAX ||
            o.received_bytes != RFS_TRANSFER_BYTES ||
            o.final_ack_attempted || o.final_ack_sent ||
            host_write_calls != 0 ||
            fstatat(o.quarantine_dir, SIDECAR_NAME, &sidecar,
                    AT_SYMLINK_NOFOLLOW))
            goto done;
        if (fault == 1 && o.sidecar_fd != -1) goto done;
        rc = 0;
        goto done;
    }
    int finish_result = finish_candidate(&o);
    if (finish_result || sidecar_stable(&o) ||
        verify_full_candidate(&o, digest) ||
        !same_bytes(digest, o.candidate_digest, sizeof digest) ||
        read_all_at(o.sidecar_fd, again, sizeof again, 0) ||
        !same_bytes(digest, again, sizeof digest))
        goto done;
    /* Corrupting the unchanged tail must make a second content gate fail. */
    chunk[0] = 0x7f;
    if (write_all_at(o.candidate, chunk, 1, RFS_TRANSFER_BYTES) ||
        fsync(o.candidate) ||
        verify_full_candidate(&o, again) == 0)
        goto done;
    rc = 0;
done:
    host_gate_override = 0;
    host_store_override = 0;
    host_sidecar_close_error = 0;
    host_sidecar_sync_error = 0;
    host_write_override = NULL;
    host_write_calls = 0;
    close_if_open(o.sidecar_fd);
    close_if_open(o.candidate);
    if (source) fclose(source);
    if (o.quarantine_dir >= 0) {
        (void)unlinkat(o.quarantine_dir, SIDECAR_NAME, 0);
        (void)unlinkat(o.quarantine_dir, CANDIDATE_NAME, 0);
    }
    close_if_open(o.quarantine_dir);
    if (o.quarantine_parent >= 0 && child_created)
        (void)unlinkat(o.quarantine_parent, leaf, AT_REMOVEDIR);
    close_if_open(o.quarantine_parent);
    if (parent_created) (void)rmdir(parent_path);
    zero_bytes(zero, sizeof zero);
    zero_bytes(chunk, sizeof chunk);
    zero_bytes(digest, sizeof digest);
    zero_bytes(again, sizeof again);
    zero_bytes(frame, sizeof frame);
    zero_bytes(&o, sizeof o);
    return rc;
}

static int test_host_transcript(int padded_final)
{
    struct owner o = {.rfs = -1, .phase = WAIT_6};
    uint8_t frame[RFS_FRAME_MAX] = {0};
    int rc = -1;
    int64_t now = monotonic_ms();
    if (now < 0) return -1;
    o.deadline_ms = o.total_deadline_ms = now + 10000;
    sha_init(&o.received_hash);
    host_gate_override = 1;
    host_write_override = host_short_write;
    host_write_calls = 0;
    if (complete_frame(&o, request_6, sizeof request_6, 0, now) == 0 ||
        host_write_calls != 1 || o.grant_attempted != 1)
        goto done;
    memset(&o, 0, sizeof o);
    o.rfs = -1; o.phase = WAIT_6;
    o.deadline_ms = o.total_deadline_ms = now + 10000;
    stop_requested = 1;
    host_write_calls = 0;
    if (complete_frame(&o, request_6, sizeof request_6, 0, now) == 0 ||
        host_write_calls != 0 || o.grant_attempted != 0)
        goto done;
    stop_requested = 0;
    memset(&o, 0, sizeof o);
    o.rfs = -1; o.phase = WAIT_7;
    o.deadline_ms = o.total_deadline_ms = now + 10000;
    sha_init(&o.received_hash);
    host_store_override = 1;
    host_finish_override = 1;
    host_write_override = host_full_write;
    host_write_calls = 0;
    if (feed_rfs(&o, request_7, sizeof request_7, now) ||
        o.phase != WAIT_3 || host_write_calls != 1 ||
        host_last_reply_len != sizeof status_7 ||
        !same_bytes(host_last_reply, status_7, sizeof status_7) ||
        feed_rfs(&o, request_3, sizeof request_3, now) ||
        o.phase != WAIT_6 ||
        feed_rfs(&o, request_6, sizeof request_6, now) ||
        o.phase != WAIT_DATA || host_write_calls != 2)
        goto done;
    for (unsigned i = 0; i < RFS_GRANTS_MAX; ++i) {
        uint32_t expected_offset = i * FIRST_CHUNK;
        uint32_t expected_length = i == RFS_GRANTS_MAX - 1 ?
                                   318 : FIRST_CHUNK;
        if (o.received_bytes != expected_offset ||
            o.expected_chunk != expected_length ||
            host_last_reply_len != 20 ||
            little16(host_last_reply) != 2 ||
            little16(host_last_reply + 2) != 1 ||
            little32(host_last_reply + 12) != expected_offset ||
            little32(host_last_reply + 16) != expected_length)
            goto done;
        memset(frame, (int)(i & 0xff), sizeof frame);
        frame[0] = 2; frame[1] = 0;
        frame[2] = 1; frame[3] = 0;
        put_little32(frame + 4, 12 + expected_length);
        put_little32(frame + 8, 0);
        put_little32(frame + 12, 3);
        put_little32(frame + 16, expected_length);
        size_t frame_len = 20 + expected_length;
        if (padded_final && i == RFS_GRANTS_MAX - 1) {
            frame_len += 2;
            put_little32(frame + 4, 332);
            frame[338] = frame[339] = 0;
        }
        if (feed_rfs(&o, frame, frame_len, monotonic_ms()))
            goto done;
        if (i == 19) {
            uint8_t sim[15 + SIT_SIM_APP_STRIDE];
            unsigned grants = (unsigned)o.grant_attempted;
            o.sit.pending = 1;
            o.sit.token = 0x15302026u;
            host_sit_reply(sim, sizeof sim, 0x0200, o.sit.token, 0);
            sim[SIT_SIM_CARD] = 1;
            sim[SIT_SIM_APPS] = 1;
            sit_feed(&o.sit, sim, 7, now);
            sit_feed(&o.sit, sim + 7, sizeof sim - 7, now);
            zero_bytes(sim, sizeof sim);
            if (o.sit.next != 1 || o.sit.pending ||
                o.grant_attempted != (int)grants || o.phase != WAIT_DATA)
                goto done;
        }
    }
    if (o.phase != TERMINAL || !o.final_ack_sent ||
        !o.final_ack_attempted || o.grant_attempted != RFS_GRANTS_MAX ||
        o.chunks_stored != RFS_GRANTS_MAX ||
        o.received_bytes != RFS_TRANSFER_BYTES ||
        host_write_calls != 97 ||
        host_last_reply_len != sizeof final_status ||
        !same_bytes(host_last_reply, final_status, sizeof final_status) ||
        feed_rfs(&o, frame, 20 + 318, monotonic_ms()) == 0)
        goto done;
    rc = 0;
done:
    host_gate_override = 0;
    host_store_override = 0;
    host_finish_override = 0;
    host_write_override = NULL;
    host_write_calls = 0;
    host_last_reply_len = 0;
    stop_requested = 0;
    zero_bytes(host_last_reply, sizeof host_last_reply);
    zero_bytes(frame, sizeof frame);
    zero_bytes(&o, sizeof o);
    return rc;
}
#endif

#ifdef SAAIOS_RFS_CAMP
static void camp_build_ind(uint8_t *buf, unsigned id, size_t len, uint32_t raw)
{
    memset(buf, 0, len);
    buf[0] = 2;
    buf[2] = (uint8_t)id;
    buf[3] = (uint8_t)(id >> 8);
    buf[4] = (uint8_t)len;
    buf[5] = (uint8_t)(len >> 8);
    if (id == 0x0802 && len >= 12) put_little32(buf + 8, raw);
}

static int test_camp_builders(void)
{
    uint8_t cfg[SEQ_CONFIG_LEN];
    static const uint8_t cfg_e[SEQ_CONFIG_LEN] =
        {0,0,0x3f,0x09,13,0,0x78,0x56,0x34,0x12,0,0,0};
    make_setmodemsconfig_request(cfg, 0x12345678);
    if (!same_bytes(cfg, cfg_e, sizeof cfg)) return 101;
    uint8_t sgc[SGC_LEN];
    static const uint8_t sgc_e[SGC_LEN] =
        {0,0,0x04,0x04,24,0,0x78,0x56,0x34,0x12,0,0,
         0x01,0x01,0,0,0,0,0,0,0,0,0,0};
    make_sgc_request(sgc, 0x12345678);
    if (!same_bytes(sgc, sgc_e, sizeof sgc)) return 102;
    uint8_t power[CAMP_POWER_LEN];
    static const uint8_t power_e[CAMP_POWER_LEN] =
        {0,0,0x00,0x08,18,0,0x78,0x56,0x34,0x12,0,0,2,0,0,0,0,0};
    make_radiopower_request(power, 0x12345678, CAMP_POWER_ON);
    if (!same_bytes(power, power_e, sizeof power)) return 103;
    static const uint8_t power_off_e[CAMP_POWER_LEN] =
        {0,0,0x00,0x08,18,0,0x78,0x56,0x34,0x12,0,0,1,0,0,0,0,0};
    make_radiopower_request(power, 0x12345678, CAMP_POWER_OFF);
    if (!same_bytes(power, power_off_e, sizeof power)) return 115;
    return 0;
}

static int test_camp_observer(void)
{
    struct camp_driver c;
    uint8_t e3[8], e2[12], e2n[12];
    camp_build_ind(e3, 0x0803, 8, 0);
    camp_build_ind(e2, 0x0802, 12, 0);
    camp_build_ind(e2n, 0x0802, 12, 2);
    /* A clean 0x0803 -> 0x0802 raw 0 pair arms the dispatcher. */
    memset(&c, 0, sizeof c);
    camp_feed(&c, e3, 8, 100);
    camp_feed(&c, e2, 12, 200);
    if (!c.radio_ready0_ms || c.radio_invalidated) return 104;
    /* Reversed order invalidates. */
    memset(&c, 0, sizeof c);
    camp_feed(&c, e2, 12, 100);
    if (!c.radio_invalidated || c.radio_ready0_ms) return 105;
    /* Duplicate 0x0803 invalidates. */
    memset(&c, 0, sizeof c);
    camp_feed(&c, e3, 8, 100);
    camp_feed(&c, e3, 8, 150);
    if (!c.radio_invalidated || c.radio_ready0_ms) return 106;
    /* Non-zero scalar after 0x0803 invalidates. */
    memset(&c, 0, sizeof c);
    camp_feed(&c, e3, 8, 100);
    camp_feed(&c, e2n, 12, 200);
    if (!c.radio_invalidated || c.radio_ready0_ms) return 107;
    /* Second event beyond the inter-event gap invalidates. */
    memset(&c, 0, sizeof c);
    camp_feed(&c, e3, 8, 100);
    camp_feed(&c, e2, 12, 100 + CAMP_PAIR_GAP_MS + 1);
    if (!c.radio_invalidated || c.radio_ready0_ms) return 108;
    /* A superseding radio event after a settled pair invalidates. */
    memset(&c, 0, sizeof c);
    camp_feed(&c, e3, 8, 100);
    camp_feed(&c, e2, 12, 200);
    camp_feed(&c, e3, 8, 300);
    if (!c.radio_invalidated) return 109;
    return 0;
}

#ifdef RFS_HOST_TEST
static uint8_t camp_cap[8][1024];
static size_t camp_cap_len[8];
static unsigned camp_cap_n;
static ssize_t camp_capture_write(int fd, const void *buf, size_t len)
{
    (void)fd;
    if (camp_cap_n < 8 && len <= sizeof camp_cap[0]) {
        memcpy(camp_cap[camp_cap_n], buf, len);
        camp_cap_len[camp_cap_n] = len;
    }
    camp_cap_n++;
    return (ssize_t)len;
}

static int test_camp_dispatch(void)
{
    struct owner o;
    memset(&o, 0, sizeof o);
    o.phase = WAIT_7;
    o.ipc = 5;
    o.camp.owner_start_ms = 0;
    o.camp.radio_ready0_ms = 1000;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_advance(&o, 1000);
    host_sit_write_override = NULL;
    if (!o.camp.dispatched || camp_cap_n != 5) return 110;
    if (camp_cap_len[0] != SEQ_CONFIG_LEN ||
        little16(camp_cap[0] + 2) != SEQ_CONFIG_COMMAND) return 111;
    if (camp_cap_len[1] != SGC_LEN ||
        little16(camp_cap[1] + 2) != SGC_COMMAND) return 112;
    if (camp_cap_len[2] != 532 || little16(camp_cap[2] + 2) != 0x4600 ||
        memcmp(camp_cap[2] + 276, "\x29\x1d\x8c\x8f", 4) != 0) return 113;
    if (camp_cap_len[3] != 532 || little16(camp_cap[3] + 2) != 0x4600 ||
        memcmp(camp_cap[3] + 276, "\xd0\xc8\x11\x04", 4) != 0) return 113;
    if (camp_cap_len[4] != CAMP_POWER_LEN ||
        little16(camp_cap[4] + 2) != CAMP_POWER_COMMAND) return 113;
    /* A second advance must not re-dispatch. */
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_advance(&o, 1000);
    host_sit_write_override = NULL;
    if (camp_cap_n != 0) return 114;
    /* The matching ACK records success without re-sending. */
    uint8_t ack[12];
    memset(ack, 0, sizeof ack);
    ack[0] = 1;
    ack[2] = (uint8_t)CAMP_POWER_COMMAND;
    ack[3] = (uint8_t)(CAMP_POWER_COMMAND >> 8);
    ack[4] = 12;
    put_little32(ack + 6, o.camp.camp_token);
    camp_feed(&o.camp, ack, sizeof ack, 1100);
    if (!o.camp.camp_acked || o.camp.camp_error) return 115;
    /* An armed pair that misses the dispatch window invalidates, no send. */
    struct owner o2;
    memset(&o2, 0, sizeof o2);
    o2.phase = WAIT_7;
    o2.ipc = 5;
    o2.camp.radio_ready0_ms = 1000;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_advance(&o2, 1000 + CAMP_DISPATCH_MS + 1);
    host_sit_write_override = NULL;
    if (o2.camp.dispatched || camp_cap_n != 0 ||
        !o2.camp.radio_invalidated) return 116;
    /* A successful RFS terminal (final ACK sent) still permits camp dispatch;
     * a failed terminal (no final ACK) blocks it. */
    struct owner o3;
    memset(&o3, 0, sizeof o3);
    o3.phase = TERMINAL;
    o3.final_ack_sent = 1;
    o3.ipc = 5;
    o3.camp.radio_ready0_ms = 1000;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_advance(&o3, 1000);
    host_sit_write_override = NULL;
    if (!o3.camp.dispatched || camp_cap_n != 5) return 117;
    struct owner o4;
    memset(&o4, 0, sizeof o4);
    o4.phase = TERMINAL;
    o4.final_ack_sent = 0;
    o4.ipc = 5;
    o4.camp.radio_ready0_ms = 1000;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_advance(&o4, 1000);
    host_sit_write_override = NULL;
    if (o4.camp.dispatched || camp_cap_n != 0) return 118;
    return 0;
}

static int test_camp_prober(void)
{
    /* A signal reply shorter than the V4 minimum yields no mask. */
    uint8_t sig_short[PROBE_SIGNAL_MIN - 1];
    uint8_t sig[PROBE_SIGNAL_MIN];
    uint32_t mask = 0xffffffffu;
    host_sit_reply(sig_short, sizeof sig_short, 0x0900, 0x55u, 0);
    if (camp_signal_mask(sig_short, sizeof sig_short, 0x55u, &mask)) return 120;
    host_sit_reply(sig, sizeof sig, 0x0900, 0x55u, 0);
    sig[12] = 0x45; /* low-seven presence bits 0x45; high bits ignored */
    sig[13] = 0x80;
    if (!camp_signal_mask(sig, sizeof sig, 0x55u, &mask) || mask != 0x45u)
        return 121;
    if (camp_signal_mask(sig, sizeof sig, 0x56u, &mask)) return 122; /* token */
    host_sit_reply(sig, sizeof sig, 0x0900, 0x55u, 6);
    if (camp_signal_mask(sig, sizeof sig, 0x55u, &mask)) return 123;  /* error */

    /* The prober sends a GET once started and clears on the matching reply. */
    struct owner o;
    memset(&o, 0, sizeof o);
    o.phase = TERMINAL;
    o.final_ack_sent = 1;
    o.ipc = 5;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, PROBE_START_MS - 1); /* too early */
    if (camp_cap_n != 0 || o.camp.probe_pending) { host_sit_write_override = NULL; return 124; }
    camp_probe_advance(&o, PROBE_START_MS);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || !o.camp.probe_pending ||
        o.camp.probe_id != 0x0200 || camp_cap_len[0] != 12) return 125;

    /* A SIM reply with app_state READY(5) marks the SIM ready, no poison. */
    uint8_t sim[15 + SIT_SIM_APP_STRIDE];
    host_sit_reply(sim, sizeof sim, 0x0200, o.camp.probe_token, 0);
    sim[SIT_SIM_CARD] = 1;
    sim[SIT_SIM_APPS] = 1;
    sim[SIT_SIM_APP_STATE] = 5;
    sim[SIT_SIM_PIN1] = 3;
    camp_feed(&o.camp, sim, sizeof sim, PROBE_START_MS + 10);
    if (o.camp.probe_pending || !o.camp.sim_ready || o.camp.poisoned ||
        o.camp.probe_replies != 1) return 126;

    /* A reply timeout backs the prober off without poisoning. */
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, PROBE_START_MS + 2000);     /* next GET */
    if (camp_cap_n != 1 || !o.camp.probe_pending) { host_sit_write_override = NULL; return 127; }
    camp_probe_advance(&o, PROBE_START_MS + 2000 + PROBE_REPLY_MS + 1);
    host_sit_write_override = NULL;
    if (o.camp.probe_pending || o.camp.poisoned ||
        o.camp.probe_timeouts != 1) return 128;

    /* A 0x0210 indication re-arms a SIM GET only while the SIM is not ready. */
    struct camp_driver fresh;
    memset(&fresh, 0, sizeof fresh);
    uint8_t ind[8] = {2, 0, 0x10, 0x02, 8, 0, 0, 0};
    camp_feed(&fresh, ind, sizeof ind, 100);
    if (!fresh.sim_change_pending) return 129;
    fresh.sim_ready = 1;
    fresh.sim_change_pending = 0;
    camp_feed(&fresh, ind, sizeof ind, 200);
    if (fresh.sim_change_pending) return 130;
    return 0;
}

static int test_setup_data_call(void)
{
    struct owner o;
    uint8_t expect[sizeof cc_sit_0600];
    uint8_t ack[12];
    int64_t t = PROBE_START_MS;
    memset(&o, 0, sizeof o);
    o.phase = TERMINAL;
    o.final_ack_sent = 1;
    o.ipc = 5;
    o.camp.reg_complete = 1;
    o.camp.data_home = 1;
    o.camp.probe_token = 0x11;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != sizeof cc_sit_0625) return 140;
    if (little16(camp_cap[0] + 2) != 0x0625 || camp_cap[0][12] != 1) return 141;
    host_sit_reply(ack, sizeof ack, 0x0625, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.vonr_sent) return 142;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != sizeof cc_sit_0613) return 143;
    if (little16(camp_cap[0] + 2) != 0x0613) return 144;
    if (memcmp(camp_cap[0] + 16, "internet", 8) != 0) return 145;
    host_sit_reply(ack, sizeof ack, 0x0613, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.profile_sent) return 146;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != sizeof cc_sit_0600) return 147;
    memcpy(expect, cc_sit_0600, sizeof expect);
    put_little32(expect + 6, o.camp.probe_token);
    if (memcmp(camp_cap[0], expect, sizeof expect) != 0) return 148;
    host_sit_reply(ack, sizeof ack, 0x0600, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.setup_sent) return 149;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 12) return 169;
    if (little16(camp_cap[0] + 2) != 0x090c) return 170;
    host_sit_reply(ack, sizeof ack, 0x090c, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.activity_sent) return 171;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != sizeof cc_sit_0605) return 150;
    if (little16(camp_cap[0] + 2) != 0x0605) return 159;
    if (memcmp(camp_cap[0] + 12, "\x0a\x0a\x0a\x0a", 4) != 0) return 160;
    host_sit_reply(ack, sizeof ack, 0x0605, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.fd_sent) return 161;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != sizeof cc_sit_0613_ims) return 162;
    if (memcmp(camp_cap[0] + 16, "ims", 4) != 0) return 163;
    host_sit_reply(ack, sizeof ack, 0x0613, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.ims_sent) return 164;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != sizeof cc_sit_0613_sos) return 165;
    if (memcmp(camp_cap[0] + 16, "sos", 4) != 0) return 166;
    host_sit_reply(ack, sizeof ack, 0x0613, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.sos_sent) return 167;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 12 ||
        little16(camp_cap[0] + 2) != 0x073e)
        return 172;
    host_sit_reply(ack, sizeof ack, 0x073e, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.endc_sent) return 173;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 16 ||
        little16(camp_cap[0] + 2) != 0x0954)
        return 174;
    if (camp_cap[0][12] | camp_cap[0][13] | camp_cap[0][14] | camp_cap[0][15])
        return 175;
    host_sit_reply(ack, sizeof ack, 0x0954, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.vonrcapa_sent) return 176;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 12 ||
        little16(camp_cap[0] + 2) != 0x0718)
        return 177;
    host_sit_reply(ack, sizeof ack, 0x0718, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.rcnet_sent) return 178;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 21 ||
        little16(camp_cap[0] + 2) != 0x094d)
        return 179;
    if (memcmp(camp_cap[0] + 12, "\x00\x30\x75\x00\x00\x00\x00\x00\x00", 9) != 0)
        return 180;
    host_sit_reply(ack, sizeof ack, 0x094d, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.throttle_sent) return 181;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 16 ||
        little16(camp_cap[0] + 2) != 0x0928 ||
        little32(camp_cap[0] + 12) != 0xffu)
        return 182;
    host_sit_reply(ack, sizeof ack, 0x0928, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.unsol_wide_sent) return 183;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 16 ||
        little16(camp_cap[0] + 2) != 0x0928 ||
        little32(camp_cap[0] + 12) != 0x7fu)
        return 280;
    host_sit_reply(ack, sizeof ack, 0x0928, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.unsol_sent) return 281;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 16 ||
        little16(camp_cap[0] + 2) != 0x0902)
        return 185;
    if (little32(camp_cap[0] + 12) != 1u) return 186;
    host_sit_reply(ack, sizeof ack, 0x0902, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.screen_sent) return 187;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 12 ||
        little16(camp_cap[0] + 2) != 0x070c)
        return 188;
    host_sit_reply(ack, sizeof ack, 0x070c, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.cellinfo_sent) return 189;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 12 ||
        little16(camp_cap[0] + 2) != 0x0108)
        return 190;
    host_sit_reply(ack, sizeof ack, 0x0108, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.smsc_sent) return 191;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 12 ||
        little16(camp_cap[0] + 2) != 0x0953)
        return 192;
    host_sit_reply(ack, sizeof ack, 0x0953, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.vonrget_sent) return 193;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    {
        time_t wall = time(NULL);
        struct tm tm;
        uint8_t expect[6];
        host_sit_write_override = camp_capture_write;
        camp_probe_advance(&o, t);
        host_sit_write_override = NULL;
        if (camp_cap_n != 1 || camp_cap_len[0] != 18 ||
            little16(camp_cap[0] + 2) != 0x0949)
            return 210;
        if (!localtime_r(&wall, &tm)) return 211;
        camp_ap_time_bytes(expect, &tm);
        if (memcmp(camp_cap[0] + 12, expect, 6) != 0) {
            time_t wall2 = time(NULL);
            if (!localtime_r(&wall2, &tm)) return 211;
            camp_ap_time_bytes(expect, &tm);
            if (memcmp(camp_cap[0] + 12, expect, 6) != 0) return 212;
        }
    }
    host_sit_reply(ack, sizeof ack, 0x0949, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.aptime_sent) return 213;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 13 ||
        little16(camp_cap[0] + 2) != 0x090b || camp_cap[0][12] != 0)
        return 214;
    host_sit_reply(ack, sizeof ack, 0x090b, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.dbgtrace_sent) return 215;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 16 ||
        little16(camp_cap[0] + 2) != 0x0903 ||
        little32(camp_cap[0] + 12) != 0)
        return 216;
    host_sit_reply(ack, sizeof ack, 0x0903, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.tty_sent) return 217;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 12 ||
        little16(camp_cap[0] + 2) != 0x0711)
        return 218;
    host_sit_reply(ack, sizeof ack, 0x0711, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.pssvc_sent) return 219;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 13 ||
        little16(camp_cap[0] + 2) != 0x0740 || camp_cap[0][12] != 0)
        return 220;
    host_sit_reply(ack, sizeof ack, 0x0740, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.prefmodem_sent) return 221;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 12 ||
        little16(camp_cap[0] + 2) != 0x024d)
        return 223;
    host_sit_reply(ack, sizeof ack, 0x024d, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.slot_sent) return 224;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    {
        uint8_t expect[67];
        camp_fill_signal_criteria(expect);
        if (camp_cap_n != 1 || camp_cap_len[0] != 67 ||
            little16(camp_cap[0] + 2) != 0x0943 ||
            memcmp(camp_cap[0] + 12, expect + 12, 55) != 0)
            return 225;
    }
    host_sit_reply(ack, sizeof ack, 0x0943, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.sigcrit_sent) return 226;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 16 ||
        little16(camp_cap[0] + 2) != 0x0107 ||
        little32(camp_cap[0] + 12) != 0)
        return 227;
    host_sit_reply(ack, sizeof ack, 0x0107, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.smsact_sent) return 228;
    {
        int step;
        for (step = 0; step < 5; step++) {
            uint8_t expect[190];
            t += PROBE_GAP_MS;
            camp_cap_n = 0;
            host_sit_write_override = camp_capture_write;
            camp_probe_advance(&o, t);
            host_sit_write_override = NULL;
            camp_fill_link_criteria(expect, camp_link_access[step]);
            if (camp_cap_n != 1 || camp_cap_len[0] != 190 ||
                little16(camp_cap[0] + 2) != 0x0944 ||
                memcmp(camp_cap[0] + 12, expect + 12, 178) != 0)
                return 229;
            host_sit_reply(ack, sizeof ack, 0x0944, o.camp.probe_token, 0);
            camp_feed(&o.camp, ack, sizeof ack, t + 10);
            if (o.camp.probe_pending || o.camp.linkcrit_next != step + 1)
                return 230;
        }
    }
    {
        int which;
        for (which = 0; which < 2; which++) {
            uint8_t expect[76];
            size_t nsend = camp_fill_smscb(expect, which);
            t += PROBE_GAP_MS;
            camp_cap_n = 0;
            host_sit_write_override = camp_capture_write;
            camp_probe_advance(&o, t);
            host_sit_write_override = NULL;
            if (camp_cap_n != 1 || camp_cap_len[0] != nsend ||
                little16(camp_cap[0] + 2) != 0x0106 ||
                memcmp(camp_cap[0] + 12, expect + 12, nsend - 12) != 0)
                return 231;
            host_sit_reply(ack, sizeof ack, 0x0106, o.camp.probe_token, 0);
            camp_feed(&o.camp, ack, sizeof ack, t + 10);
            if (o.camp.probe_pending || o.camp.smscb_next != which + 1)
                return 232;
        }
    }
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 12 ||
        little16(camp_cap[0] + 2) != 0x0000)
        return 233;
    host_sit_reply(ack, sizeof ack, 0x0000, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.calllist_sent) return 234;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != OPX_VOICE_LEN ||
        little16(camp_cap[0] + 2) != OPX_VOICE_SET ||
        little32(camp_cap[0] + 12) != OPX_VOICE_MODE)
        return 235;
    host_sit_reply(ack, sizeof ack, OPX_VOICE_SET, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.voice_stock_sent) return 236;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != BBVER_LEN ||
        little16(camp_cap[0] + 2) != BBVER_GET ||
        camp_cap[0][12] != BBVER_MASK)
        return 237;
    {
        uint8_t bb[20];
        host_sit_reply(bb, sizeof bb, BBVER_GET, o.camp.probe_token, 0);
        memcpy(bb + BBVER_SWVER_OFF, "g5300q", 6);
        camp_feed(&o.camp, bb, sizeof bb, t + 10);
    }
    if (o.camp.probe_pending || !o.camp.bbver_sent || !o.camp.bbver_done)
        return 238;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 13 ||
        little16(camp_cap[0] + 2) != 0x0c20 || camp_cap[0][12] != 1)
        return 239;
    host_sit_reply(ack, sizeof ack, 0x0c20, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.gpslock_sent) return 240;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 13 ||
        little16(camp_cap[0] + 2) != 0x0c33 || camp_cap[0][12] != 0)
        return 241;
    host_sit_reply(ack, sizeof ack, 0x0c33, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.gpsnfw_sent) return 242;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 12 ||
        little16(camp_cap[0] + 2) != 0x0755)
        return 243;
    host_sit_reply(ack, sizeof ack, 0x0755, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.samode_sent) return 244;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 14 ||
        little16(camp_cap[0] + 2) != 0x0d3c ||
        camp_cap[0][12] != 1 || camp_cap[0][13] != 1)
        return 245;
    host_sit_reply(ack, sizeof ack, 0x0d3c, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.xcapstop_sent) return 246;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 14 ||
        little16(camp_cap[0] + 2) != 0x0d3a ||
        camp_cap[0][12] != 1 || camp_cap[0][13] != 1)
        return 247;
    host_sit_reply(ack, sizeof ack, 0x0d3a, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.aimstop_sent) return 248;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 14 ||
        little16(camp_cap[0] + 2) != 0x0d3b ||
        camp_cap[0][12] != 1 || camp_cap[0][13] != 1)
        return 249;
    host_sit_reply(ack, sizeof ack, 0x0d3b, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.xcapstart_sent) return 250;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 14 ||
        little16(camp_cap[0] + 2) != 0x0d3a ||
        camp_cap[0][12] != 0 || camp_cap[0][13] != 1)
        return 251;
    host_sit_reply(ack, sizeof ack, 0x0d3a, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.aimstop0_sent) return 252;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 14 ||
        little16(camp_cap[0] + 2) != 0x0d3c ||
        camp_cap[0][12] != 0 || camp_cap[0][13] != 1)
        return 253;
    host_sit_reply(ack, sizeof ack, 0x0d3c, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.xcapstop0_sent) return 254;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != RATBM_SET_LEN ||
        little16(camp_cap[0] + 2) != RATBM_SET ||
        little32(camp_cap[0] + 12) != RATBM_WIRE_STOCK)
        return 255;
    host_sit_reply(ack, sizeof ack, RATBM_SET, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.ratbm_set_sent) return 256;
    {
        int si;
        for (si = 0; si < (int)(sizeof camp_sig_more / sizeof camp_sig_more[0]);
             si++) {
            uint8_t expect[67];
            t += PROBE_GAP_MS;
            camp_cap_n = 0;
            host_sit_write_override = camp_capture_write;
            camp_probe_advance(&o, t);
            host_sit_write_override = NULL;
            camp_fill_signal_row(expect, &camp_sig_more[si]);
            if (camp_cap_n != 1 || camp_cap_len[0] != 67 ||
                memcmp(camp_cap[0] + 12, expect + 12, 55) != 0)
                return 257 + si;
            host_sit_reply(ack, sizeof ack, 0x0943, o.camp.probe_token, 0);
            camp_feed(&o.camp, ack, sizeof ack, t + 10);
            if (o.camp.probe_pending || o.camp.sigcrit_extra != si + 1)
                return 270 + si;
        }
    }
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 12 ||
        little16(camp_cap[0] + 2) != 0x0615 ||
        camp_cap[0][4] != 0 || camp_cap[0][5] != 0)
        return 282;
    host_sit_reply(ack, sizeof ack, 0x0615, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.phonecap_sent) return 283;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 14 ||
        little16(camp_cap[0] + 2) != 0x0d3c ||
        camp_cap[0][12] != 0 || camp_cap[0][13] != 2)
        return 284;
    host_sit_reply(ack, sizeof ack, 0x0d3c, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.xcapstop2_sent) return 285;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 12 ||
        little16(camp_cap[0] + 2) != 0x0212 ||
        camp_cap[0][4] != 12)
        return 286;
    host_sit_reply(ack, sizeof ack, 0x0212, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.atr_sent) return 287;
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || camp_cap_len[0] != 14 ||
        little16(camp_cap[0] + 2) != 0x4605 ||
        camp_cap[0][12] != 0x39 || camp_cap[0][13] != 0x34)
        return 288;
    host_sit_reply(ack, sizeof ack, 0x4605, o.camp.probe_token, 0);
    camp_feed(&o.camp, ack, sizeof ack, t + 10);
    if (o.camp.probe_pending || !o.camp.svn_sent) return 289;
    {
        uint8_t ind[24];
        memset(ind, 0, sizeof ind);
        ind[0] = 2;
        ind[2] = 0x06;
        ind[3] = 0x09;
        ind[4] = 12;
        ind[8] = 0x45;
        camp_feed(&o.camp, ind, 12, t);
        camp_feed(&o.camp, ind, 12, t);
        if (o.camp.ind_signal != 1 || o.camp.ind_signal_mask != 0x45)
            return 290;
        {
            uint8_t dc[301];
            memset(dc, 0, sizeof dc);
            dc[0] = 2;
            dc[2] = 0x04;
            dc[3] = 0x06;
            dc[4] = 0x2d;
            dc[5] = 0x01;
            dc[8] = 1;
            dc[11] = 2;
            dc[12] = 1;
            dc[13] = 3;
            camp_feed(&o.camp, dc, sizeof dc, t);
            if (o.camp.ind_datacall != 1 || o.camp.ind_dc_count != 1 ||
                o.camp.ind_dc_cid != 2 || o.camp.ind_dc_active != 1 ||
                o.camp.ind_dc_pdp != 3)
                return 291;
        }
        memset(ind, 0, sizeof ind);
        ind[0] = 2;
        ind[2] = 0x45;
        ind[3] = 0x09;
        ind[4] = 24;
        put_little32(ind + 8, 100);
        put_little32(ind + 12, 50);
        put_little32(ind + 16, 0);
        put_little32(ind + 20, 0xffffffffu);
        camp_feed(&o.camp, ind, 24, t);
        if (o.camp.ind_linkcap != 1 || o.camp.link_dl != 100 ||
            o.camp.link_ul != 50 || o.camp.link_dl2 != 0 ||
            o.camp.link_ul2 != -1)
            return 292;
        {
            uint8_t phy[74];
            memset(phy, 0, sizeof phy);
            phy[0] = 2;
            phy[2] = 0x42;
            phy[3] = 0x07;
            phy[4] = 74;
            put_little32(phy + 8, 1);
            phy[12] = 1;
            put_little32(phy + 13, 20000);
            phy[17] = 20;
            put_little32(phy + 19, 1234);
            put_little32(phy + 60, 100);
            put_little32(phy + 64, 5678);
            put_little32(phy + 68, 10000);
            phy[72] = 0xfe;
            phy[73] = 0xff;
            camp_feed(&o.camp, phy, sizeof phy, t);
            camp_feed(&o.camp, phy, sizeof phy, t);
            if (o.camp.ind_phy != 1 || o.camp.ind_phy_count != 1 ||
                o.camp.ind_phy_status != 1 || o.camp.ind_phy_rat != 14 ||
                o.camp.ind_phy_dl_bw != 20000 || o.camp.ind_phy_dl_ch != 1234 ||
                o.camp.ind_phy_ul_ch != 5678 || o.camp.ind_phy_ul_bw != 10000 ||
                o.camp.ind_phy_pci != 100 || o.camp.ind_phy_band != -2)
                return 293;
            {
                uint8_t ac[41];
                memset(ac, 0, sizeof ac);
                ac[0] = 2;
                ac[2] = 0x20;
                ac[3] = 0x07;
                ac[4] = 41;
                ac[8] = 1;
                ac[9] = 0;
                ac[17] = 1;
                ac[25] = 0;
                ac[33] = 1;
                camp_feed(&o.camp, ac, sizeof ac, t);
                camp_feed(&o.camp, ac, sizeof ac, t);
                if (o.camp.ind_acbar != 1 || o.camp.ac_emc != 1 ||
                    o.camp.ac_mosig != 0 || o.camp.ac_modata != 1 ||
                    o.camp.ac_voice != 0 || o.camp.ac_video != 1)
                    return 294;
                {
                    uint8_t bar[328];
                    memset(bar, 0, sizeof bar);
                    bar[0] = 2;
                    bar[2] = 0x4b;
                    bar[3] = 0x07;
                    bar[4] = 0x48;
                    bar[5] = 0x01;
                    bar[8] = 2;
                    put_little32(bar + 15, 0x1a2b3c);
                    put_little32(bar + 19, 114);
                    put_little32(bar + 23, 100);
                    put_little32(bar + 27, 1500);
                    put_little32(bar + 290, 2);
                    put_little32(bar + 294, 3);
                    put_little32(bar + 298, 1);
                    put_little32(bar + 302, 50);
                    put_little32(bar + 306, 30);
                    bar[310] = 1;
                    put_little32(bar + 311, 1);
                    put_little32(bar + 315, 2);
                    put_little32(bar + 319, 10);
                    put_little32(bar + 323, 20);
                    camp_feed(&o.camp, bar, sizeof bar, t);
                    camp_feed(&o.camp, bar, sizeof bar, t);
                    if (o.camp.ind_barring != 1 || o.camp.bar_cell != 2 ||
                        o.camp.bar_count != 2 || o.camp.bar_svc != 3 ||
                        o.camp.bar_kind != 1 || o.camp.bar_factor != 50 ||
                        o.camp.bar_time != 30 || o.camp.bar_barred != 1)
                        return 295;
                    if (camp_lte_id(0x10000000u, 0) != 0x7fffffff ||
                        camp_lte_id(504, 1) != 0x7fffffff ||
                        camp_lte_id(0x10000u, 2) != 0x7fffffff ||
                        camp_lte_id(0x40000u, 3) != 0x7fffffff ||
                        o.camp.bar_ci != 0x1a2b3c || o.camp.bar_pci != 114 ||
                        o.camp.bar_tac != 100 || o.camp.bar_earfcn != 1500)
                        return 296;
                    if (o.camp.bar_nlogged != 2 || o.camp.bar2_svc != 1 ||
                        o.camp.bar2_kind != 2 || o.camp.bar2_factor != 10 ||
                        o.camp.bar2_time != 20 || o.camp.bar2_barred != 0)
                        return 297;
                }
            }
        }
    }
    t += PROBE_GAP_MS;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, t);
    host_sit_write_override = NULL;
    if (camp_cap_n && (little16(camp_cap[0] + 2) == 0x0600 ||
                       little16(camp_cap[0] + 2) == 0x0625 ||
                       little16(camp_cap[0] + 2) == 0x0613 ||
                       little16(camp_cap[0] + 2) == 0x0605 ||
                       little16(camp_cap[0] + 2) == 0x073e ||
                       little16(camp_cap[0] + 2) == 0x0954 ||
                       little16(camp_cap[0] + 2) == 0x0718 ||
                       little16(camp_cap[0] + 2) == 0x094d ||
                       little16(camp_cap[0] + 2) == 0x0928 ||
                       little16(camp_cap[0] + 2) == 0x0902 ||
                       little16(camp_cap[0] + 2) == 0x070c ||
                       little16(camp_cap[0] + 2) == 0x0108 ||
                       little16(camp_cap[0] + 2) == 0x0953 ||
                       little16(camp_cap[0] + 2) == 0x0949 ||
                       little16(camp_cap[0] + 2) == 0x090b ||
                       little16(camp_cap[0] + 2) == 0x0903 ||
                       little16(camp_cap[0] + 2) == 0x0711 ||
                       little16(camp_cap[0] + 2) == 0x0740 ||
                       little16(camp_cap[0] + 2) == 0x024d ||
                       little16(camp_cap[0] + 2) == 0x0943 ||
                       little16(camp_cap[0] + 2) == 0x0615 ||
                       little16(camp_cap[0] + 2) == 0x0212 ||
                       little16(camp_cap[0] + 2) == 0x4605 ||
                       little16(camp_cap[0] + 2) == 0x0107 ||
                       little16(camp_cap[0] + 2) == 0x0944 ||
                       little16(camp_cap[0] + 2) == 0x0106 ||
                       little16(camp_cap[0] + 2) == 0x0000 ||
                       little16(camp_cap[0] + 2) == OPX_VOICE_SET ||
                       little16(camp_cap[0] + 2) == BBVER_GET ||
                       little16(camp_cap[0] + 2) == 0x0c20 ||
                       little16(camp_cap[0] + 2) == 0x0c33 ||
                       little16(camp_cap[0] + 2) == 0x0755 ||
                       little16(camp_cap[0] + 2) == 0x0d3c ||
                       little16(camp_cap[0] + 2) == 0x0d3a ||
                       little16(camp_cap[0] + 2) == 0x0d3b ||
                       little16(camp_cap[0] + 2) == RATBM_SET))
        return 168;
    {
        struct tm fixed;
        uint8_t packed[6];
        memset(&fixed, 0, sizeof fixed);
        fixed.tm_year = 126;
        fixed.tm_mon = 9;
        fixed.tm_mday = 5;
        fixed.tm_hour = 23;
        fixed.tm_min = 59;
        fixed.tm_sec = 58;
        camp_ap_time_bytes(packed, &fixed);
        if (memcmp(packed, "\x7e\x09\x05\x17\x3b\x3a", 6) != 0) return 222;
    }
    {
        uint8_t fr[64];
        uint8_t got[4];
        unsigned idx = 99;
        memset(fr, 0, sizeof fr);
        fr[14] = 2;
        fr[16] = 3;
        fr[17] = 10;
        fr[18] = 1;
        fr[19] = 2;
        fr[20] = 3;
        if (camp_data_call_v4(fr, sizeof fr, &idx, got) || idx != 1 ||
            memcmp(got, "\x0a\x01\x02\x03", 4) != 0)
            return 151;
        fr[16] = 2;
        if (camp_data_call_v4(fr, sizeof fr, &idx, got) == 0) return 152;
        memset(fr, 0, sizeof fr);
        fr[37] = 3;
        fr[38] = 8;
        fr[39] = 8;
        fr[40] = 8;
        fr[41] = 8;
        fr[58] = 1;
        fr[59] = 1;
        fr[60] = 1;
        fr[61] = 1;
        {
            uint8_t dns[2][4];
            int ndns = 0;
            if (camp_data_call_dns(fr, sizeof fr, dns, &ndns) || ndns != 2)
                return 153;
            if (memcmp(dns[0], "\x08\x08\x08\x08", 4) != 0) return 154;
            if (memcmp(dns[1], "\x01\x01\x01\x01", 4) != 0) return 155;
        }
        {
            uint8_t got6[16];
            unsigned idx6 = 99;
            static const uint8_t expect6[16] = {
                0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 1
            };
            memset(fr, 0, sizeof fr);
            fr[14] = 2;
            fr[16] = 3;
            memcpy(fr + 21, expect6, 16);
            if (camp_data_call_v6(fr, sizeof fr, &idx6, got6) || idx6 != 1 ||
                memcmp(got6, expect6, 16) != 0)
                return 156;
            fr[16] = 1;
            if (camp_data_call_v6(fr, sizeof fr, &idx6, got6) == 0) return 157;
            memset(fr + 21, 0, 16);
            fr[16] = 2;
            if (camp_data_call_v6(fr, sizeof fr, &idx6, got6) == 0) return 158;
        }
        {
            uint8_t wide[96];
            uint8_t dns6[2][16];
            int ndns6 = 0;
            static const uint8_t a[16] = {
                0x20, 0x01, 0x48, 0x60, 0x48, 0x60, 0, 0,
                0, 0, 0, 0, 0, 0, 0x88, 0x88
            };
            static const uint8_t b[16] = {
                0x20, 0x01, 0x48, 0x60, 0x48, 0x60, 0, 0,
                0, 0, 0, 0, 0, 0, 0x88, 0x44
            };
            memset(wide, 0, sizeof wide);
            wide[37] = 3;
            memcpy(wide + 42, a, 16);
            memcpy(wide + 62, b, 16);
            if (camp_data_call_dns6(wide, sizeof wide, dns6, &ndns6) ||
                ndns6 != 2)
                return 194;
            if (memcmp(dns6[0], a, 16) != 0) return 195;
            if (memcmp(dns6[1], b, 16) != 0) return 196;
            wide[37] = 1;
            if (camp_data_call_dns6(wide, sizeof wide, dns6, &ndns6) == 0)
                return 197;
        }
    }
    return 0;
}
#endif

static int test_camp_reg(void)
{
    /* Builders: exact factory byte shapes, no invented bytes. */
    uint8_t pref[REG_PREF_LEN];
    static const uint8_t pref_e[REG_PREF_LEN] =
        {0,0,0x0a,0x07,16,0,0x78,0x56,0x34,0x12,0,0,12,0,0,0};
    make_setpref_request(pref, RAT_LTE_WCDMA, 0x12345678);
    if (!same_bytes(pref, pref_e, sizeof pref)) return 131;
    uint8_t allow[REG_ALLOW_LEN];
    static const uint8_t allow_e[REG_ALLOW_LEN] =
        {0,0,0x10,0x07,13,0,0x78,0x56,0x34,0x12,0,0,1};
    make_allowdata_request(allow, 0x12345678);
    if (!same_bytes(allow, allow_e, sizeof allow)) return 132;

    /* Step machine: radio -> selection (read, bounded) -> auto SET ->
     * preferred (read, bounded) -> broaden SET -> allow_data -> done. */
    struct camp_driver c;
    memset(&c, 0, sizeof c);
    if (camp_reg_next(&c) != 0) return 133;          /* SIM not ready yet */
    c.sim_ready = 1;
    if (camp_reg_next(&c) != REG_RADIO_GET) return 134;
    c.radio_on = 1;
    if (camp_reg_next(&c) != REG_SEL_GET) return 135;
    c.sel_known = 1; c.sel_mode = 1;                 /* manual -> force auto */
    if (camp_reg_next(&c) != REG_SEL_AUTO_SET) return 136;
    c.sel_auto_sent = 1;
    if (camp_reg_next(&c) != REG_PREF_GET) return 137;
    c.pref_known = 1; c.preferred_raw = RAT_LTE_ONLY;
    if (camp_reg_next(&c) != REG_PREF_SET) return 138;
    c.pref_set_sent = 1;
    if (camp_reg_next(&c) != REG_ALLOW_DATA) return 139;
    c.allow_data_sent = 1;
    if (camp_reg_next(&c) != 0) return 140;

    /* With an APN configured, SET_INITIAL_ATTACH_APN precedes allow_data. */
    struct camp_driver iac;
    memset(&iac, 0, sizeof iac);
    iac.sim_ready = 1; iac.radio_on = 1;
    iac.sel_auto_sent = 1; iac.pref_set_sent = 1;
    memcpy(iac.apn, "internet", 9);
    if (camp_reg_next(&iac) != REG_INIT_ATTACH_APN) return 170;
    iac.ia_apn_sent = 1;
    if (camp_reg_next(&iac) != REG_ALLOW_DATA) return 171;
    iac.allow_data_sent = 1;
    if (camp_reg_next(&iac) != 0) return 172;

    /* Already-auto selection skips 0x0704; already-LTE_WCDMA preferred skips
     * 0x070a; any other preferred value (e.g. raw 16) is broadened. */
    struct camp_driver d;
    memset(&d, 0, sizeof d);
    d.sim_ready = 1; d.radio_on = 1; d.sel_known = 1; d.sel_mode = 0;
    if (camp_reg_next(&d) != REG_PREF_GET) return 141;
    d.pref_known = 1; d.preferred_raw = RAT_LTE_WCDMA;
    if (camp_reg_next(&d) != REG_ALLOW_DATA) return 142;
    d.allow_data_sent = 1;
    if (camp_reg_next(&d) != 0) return 143;

    struct camp_driver d16;
    memset(&d16, 0, sizeof d16);
    d16.sim_ready = 1; d16.radio_on = 1; d16.sel_auto_sent = 1;
    d16.pref_known = 1; d16.preferred_raw = 16;      /* observed settled value */
    if (camp_reg_next(&d16) != REG_PREF_SET) return 144;

    /* Bounded GET retries: unanswered 0x0703/0x070b still advance to the SETs. */
    struct camp_driver e;
    memset(&e, 0, sizeof e);
    e.sim_ready = 1; e.radio_on = 1;
    e.sel_get_tries = REG_GET_MAX;                   /* selection never answered */
    if (camp_reg_next(&e) != REG_SEL_AUTO_SET) return 145;
    e.sel_auto_sent = 1;
    e.pref_get_tries = REG_GET_MAX;                  /* preferred never answered */
    if (camp_reg_next(&e) != REG_PREF_SET) return 146;
    e.pref_set_sent = 1;
    if (camp_reg_next(&e) != REG_ALLOW_DATA) return 147;

    /* Unanswered radio GET gives up rather than looping forever. */
    struct camp_driver f;
    memset(&f, 0, sizeof f);
    f.sim_ready = 1; f.radio_get_tries = REG_GET_MAX;
    if (camp_reg_next(&f) != 0) return 148;

    /* reg_complete gate: ACK of allow_data stops all further steps. */
    struct camp_driver g;
    memset(&g, 0, sizeof g);
    g.sim_ready = 1; g.reg_complete = 1;
    if (camp_reg_next(&g) != 0) return 149;

    /* OPX experiment: after reg_complete the three GETs are read, then one SET
     * per the selected step. GET-only when no step is selected. */
    if (camp_opx_next(&g) != OPX_STACK_GET) return 150;
    g.opx_stack_get_sent = 1;
    if (camp_opx_next(&g) != OPX_VOICE_GET) return 151;
    g.opx_voice_get_sent = 1;
    if (camp_opx_next(&g) != OPX_DEVSVC_GET) return 152;
    g.opx_devsvc_get_sent = 1;
    if (camp_opx_next(&g) != 0) return 153;           /* no SET step selected */
    g.opx_step = OPX_STEP_VOICE;
    if (camp_opx_next(&g) != OPX_VOICE_SET) return 154;
    g.opx_set_sent = 1; g.opx_done = 1;
    if (camp_opx_next(&g) != 0) return 155;           /* done after one SET */

    struct camp_driver h;
    memset(&h, 0, sizeof h);
    h.sim_ready = 1; h.reg_complete = 1; h.opx_step = OPX_STEP_INTPS;
    h.opx_stack_get_sent = h.opx_voice_get_sent = h.opx_devsvc_get_sent = 1;
    if (camp_opx_next(&h) != OPX_INTPS_SET) return 156;
    struct camp_driver k;
    memset(&k, 0, sizeof k);
    k.sim_ready = 1; k.reg_complete = 1; k.opx_step = OPX_STEP_STACK;
    k.opx_stack_get_sent = k.opx_voice_get_sent = k.opx_devsvc_get_sent = 1;
    if (camp_opx_next(&k) != OPX_STACK_SET) return 157;
    struct camp_driver dv;
    memset(&dv, 0, sizeof dv);
    dv.sim_ready = 1; dv.reg_complete = 1; dv.opx_step = OPX_STEP_DEVSVC;
    dv.opx_stack_get_sent = dv.opx_voice_get_sent = dv.opx_devsvc_get_sent = 1;
    if (camp_opx_next(&dv) != OPX_DEVSVC_SET) return 161;
    struct camp_driver du;
    memset(&du, 0, sizeof du);
    du.sim_ready = 1; du.reg_complete = 1; du.opx_step = OPX_STEP_DUAL;
    du.opx_stack_get_sent = du.opx_voice_get_sent = du.opx_devsvc_get_sent = 1;
    if (camp_opx_next(&du) != OPX_DUAL_SET) return 163;

    /* Frame byte-exactness: id@+2, len@+4, token@+6, body@+12. */
    uint8_t vs[OPX_VOICE_LEN];
    static const uint8_t vs_e[OPX_VOICE_LEN] =
        {0,0,0x1a,0x09,16,0,0x78,0x56,0x34,0x12,0,0,3,0,0,0};
    make_opx_u32_request(vs, OPX_VOICE_SET, OPX_VOICE_LEN, 0x12345678,
                         OPX_VOICE_MODE);
    if (!same_bytes(vs, vs_e, sizeof vs)) return 158;
    uint8_t is[OPX_INTPS_LEN];
    static const uint8_t is_e[OPX_INTPS_LEN] =
        {0,0,0x33,0x09,16,0,0x78,0x56,0x34,0x12,0,0,1,0,0,0};
    make_opx_u32_request(is, OPX_INTPS_SET, OPX_INTPS_LEN, 0x12345678,
                         OPX_INTPS_MODE);
    if (!same_bytes(is, is_e, sizeof is)) return 159;
    uint8_t ss[OPX_STACK_LEN];
    static const uint8_t ss_e[OPX_STACK_LEN] =
        {0,0,0x0f,0x08,13,0,0x78,0x56,0x34,0x12,0,0,1};
    make_opx_byte_request(ss, OPX_STACK_SET, OPX_STACK_LEN, 0x12345678,
                          OPX_STACK_MODE_ENABLE);
    if (!same_bytes(ss, ss_e, sizeof ss)) return 160;
    uint8_t ds[OPX_DEVSVC_LEN];
    static const uint8_t ds_e[OPX_DEVSVC_LEN] =
        {0,0,0x56,0x09,16,0,0x78,0x56,0x34,0x12,0,0,2,0,0,0};
    make_opx_u32_request(ds, OPX_DEVSVC_SET, OPX_DEVSVC_LEN, 0x12345678,
                         OPX_DEVSVC_MODE_DATA);
    if (!same_bytes(ds, ds_e, sizeof ds)) return 162;
    uint8_t du2[OPX_DUAL_LEN];
    static const uint8_t du_e[OPX_DUAL_LEN] =
        {0,0,0x2b,0x07,28,0,0x78,0x56,0x34,0x12,0,0,
         12,0,0,0, 12,0,0,0, 1,0,0,0, 1,0,0,0};
    make_opx_dual_request(du2, 0x12345678);
    if (!same_bytes(du2, du_e, sizeof du2)) return 164;
    /* SET_INITIAL_ATTACH_APN (0x0603): the CP2A stock frame for APN "internet"
     * has exactly these nonzero body bytes: [12]=1 [13]=0x0e APN@16 [218]=3
     * [248]=3. */
    uint8_t ia[REG_IA_LEN];
    make_initial_attach_apn_request(ia, "internet", 0x12345678);
    if (ia[0] || ia[1] || ia[2] != 0x03 || ia[3] != 0x06 ||
        ia[4] != 251 || ia[5]) return 165;
    if (little32(ia + 6) != 0x12345678 || ia[10] || ia[11]) return 166;
    if (ia[12] != REG_IA_CID || ia[13] != REG_IA_CONST13 ||
        ia[14] || ia[15]) return 167;
    if (memcmp(ia + REG_IA_APN_OFF, "internet", 9)) return 168; /* incl NUL */
    for (unsigned i = 12; i < REG_IA_LEN; i++) {
        unsigned expect = i == 12 ? 1 : i == 13 ? 0x0e : i == 218 || i == 248 ? 3 :
                          i >= 16 && i < 24 ? (unsigned)(uint8_t)"internet"[i - 16] : 0;
        if (ia[i] != expect) return 169;
    }
    /* Reg-state decode: RAT map recovered from libsitril .rodata @0xd8afc,
     * and the fixed field offsets for voice (0x0700) / data (0x0701). */
    if (sit_net_rat_map(3) != 3 || sit_net_rat_map(14) != 14 ||
        sit_net_rat_map(16) != 16 || sit_net_rat_map(1) != 1 ||
        sit_net_rat_map(21) != 20) return 173;
    if (sit_net_rat_map(0) != 0 || sit_net_rat_map(22) != 0) return 174;
    if (SIT_NET_REG_STATE_OFFSET != 12 || SIT_NET_REJECT_OFFSET != 13)
        return 175;
    if (SIT_NET_VOICE_TECH_OFFSET != 14 || SIT_NET_VOICE_LAC_OFFSET != 15 ||
        SIT_NET_VOICE_CID_OFFSET != 19 || SIT_NET_VOICE_PSC_OFFSET != 23)
        return 176;
    if (SIT_NET_DATA_TECH_OFFSET != 15 || SIT_NET_DATA_LAC_OFFSET != 16 ||
        SIT_NET_DATA_CID_OFFSET != 20 || SIT_NET_DATA_PSC_OFFSET != 24)
        return 177;
    /* Preferred-RAT target override + LTE-only builder (SIT value 0x0b=11,
     * recovered from BuildSetPreferredNetworkType table @0xd8c5c[11]). */
    if (SIT_NET_OPERATOR != 0x0702) return 178;
    struct camp_driver pt;
    memset(&pt, 0, sizeof pt);
    if (camp_pref_target(&pt) != (unsigned)RAT_LTE_WCDMA) return 179;
    pt.pref_target = RAT_LTE_ONLY;
    if (camp_pref_target(&pt) != 11u) return 180;
    uint8_t lo[REG_PREF_LEN];
    make_setpref_request(lo, RAT_LTE_ONLY, 0x12345678);
    if (lo[2] != 0x0a || lo[3] != 0x07 || lo[4] != REG_PREF_LEN ||
        little32(lo + 12) != 11u) return 181;
    /* pref_target=LTE_ONLY: once radio/sel known and preferred_raw!=11, the next
     * step is a preferred SET; after a matching readback it advances past it. */
    struct camp_driver po;
    memset(&po, 0, sizeof po);
    po.sim_ready = 1; po.radio_on = 1; po.sel_known = 1; po.sel_mode = 0;
    po.sel_auto_sent = 1; po.pref_target = RAT_LTE_ONLY;
    po.pref_known = 1; po.preferred_raw = RAT_LTE_WCDMA; /* 12 != target 11 */
    if (camp_reg_next(&po) != REG_PREF_SET) return 182;
    po.preferred_raw = RAT_LTE_ONLY;
    if (camp_reg_next(&po) != REG_ALLOW_DATA) return 183;

    /* Available-networks scan (0x0706) + manual selection (0x0705), recovered
     * from libsitril ProtocolNetAvailableNetworkAdapter and
     * BuildSetNetworkSelectionManual. */
    if (SIT_NET_AVAILABLE_NETWORKS != 0x0706 ||
        SIT_NET_SELECTION_MANUAL != 0x0705) return 184;
    /* Scan-RAT remap: 0x11..0x15 remapped, everything else pass-through. */
    if (sit_scan_rat_map(0x11) != 0 || sit_scan_rat_map(0x12) != 17 ||
        sit_scan_rat_map(0x13) != 15 || sit_scan_rat_map(0x14) != 14 ||
        sit_scan_rat_map(0x15) != 20) return 185;
    if (sit_scan_rat_map(3) != 3 || sit_scan_rat_map(16) != 16 ||
        sit_scan_rat_map(0) != 0) return 186;
    /* Parser: a 2-entry 0x0706 reply (lifecell 25506 LTE current, foreign
     * 25501 UMTS available). count@+12, list@+16 stride 14. */
    {
        uint8_t fr[16 + 2 * 14] = {0};
        fr[0] = 1; fr[2] = 0x06; fr[3] = 0x07;
        put_little32(fr + 4, sizeof fr);
        put_little32(fr + 12, 2);           /* count */
        /* entry 0: RAT raw 0x14 (LTE), PLMN "25506", status 2 (current) */
        put_little32(fr + 16 + 0, 0x14);
        memcpy(fr + 16 + 4, "25506", 5); fr[16 + 9] = 0x23;
        put_little32(fr + 16 + 10, 2);
        /* entry 1: RAT raw 3 (UMTS), PLMN "25501", status 1 (available) */
        put_little32(fr + 30 + 0, 3);
        memcpy(fr + 30 + 4, "25501", 5); fr[30 + 9] = 0x23;
        put_little32(fr + 30 + 10, 1);
        int seen = 0;
        if (sit_net_log_available("selftest", fr, sizeof fr, "25506", &seen) != 2)
            return 187;
        if (!seen) return 195;
        {
            int seen2 = 1;
            if (sit_net_log_available("selftest", fr, sizeof fr, "25503",
                                      &seen2) != 2 || seen2) return 196;
        }
    }
    /* Manual-select builder bytes: id@+2, len 22@+4, token@+6, RAT any=0@+12,
     * PLMN ASCII@+16 with '#' filler at +21 for a 5-digit PLMN. */
    {
        uint8_t ms[SIT_NET_SEL_MANUAL_LEN];
        make_manual_select_request(ms, "25506", 0x12345678);
        if (ms[0] || ms[1] || ms[2] != 0x05 || ms[3] != 0x07 ||
            ms[4] != SIT_NET_SEL_MANUAL_LEN || ms[5]) return 188;
        if (little32(ms + 6) != 0x12345678 ||
            little32(ms + SIT_NET_SEL_MANUAL_RAT_OFFSET) != 0) return 189;
        if (memcmp(ms + SIT_NET_SEL_MANUAL_PLMN_OFFSET, "25506#", 6))
            return 190;
    }
    /* camp_reg_next: scan armed after radio ON precedes selection; a manual PLMN
     * replaces automatic selection; both are one-shot. */
    {
        struct camp_driver sc;
        memset(&sc, 0, sizeof sc);
        sc.sim_ready = 1; sc.radio_on = 1; sc.scan_enabled = 1;
        if (camp_reg_next(&sc) != REG_SCAN) return 191;
        sc.scan_sent = 1;
        /* no manual PLMN, selection unknown -> falls to selection GET */
        if (camp_reg_next(&sc) != REG_SEL_GET) return 192;
        struct camp_driver mn;
        memset(&mn, 0, sizeof mn);
        mn.sim_ready = 1; mn.radio_on = 1;
        memcpy(mn.manual_plmn, "25506", 6);
        if (camp_reg_next(&mn) != REG_MANUAL_SEL) return 193;
        mn.manual_sel_sent = 1;
        mn.pref_known = 1; mn.preferred_raw = camp_pref_target(&mn);
        if (camp_reg_next(&mn) != REG_ALLOW_DATA) return 194;
    }
    /* Deregister-then-scan (VERDICT 15): the sub-sequence only runs post-reg and
     * armed; it walks OFF -> confirm -> ON -> scan -> (manual if target seen). */
    {
        struct camp_driver dr;
        memset(&dr, 0, sizeof dr);
        /* Not armed: no dereg step even when reg complete. */
        dr.reg_complete = 1;
        if (camp_dereg_next(&dr) != DRG_NONE) return 197;
        dr.dereg_enabled = 1;
        if (camp_dereg_next(&dr) != DRG_OFF) return 198;
        dr.drg_off_sent = 1;
        if (camp_dereg_next(&dr) != DRG_NONE) return 199;  /* awaiting ack */
        dr.drg_off_ack = 1;
        if (camp_dereg_next(&dr) != DRG_OFF_GET) return 200;
        dr.drg_off_confirmed = 1;
        if (camp_dereg_next(&dr) != DRG_ON) return 201;
        dr.drg_on_sent = 1;
        if (camp_dereg_next(&dr) != DRG_NONE) return 202;  /* awaiting ack */
        dr.drg_on_ack = 1;
        if (camp_dereg_next(&dr) != DRG_SCAN) return 203;
        dr.drg_scan_sent = 1;
        if (camp_dereg_next(&dr) != DRG_NONE) return 204;  /* awaiting reply */
        /* Scan parsed, target not visible, no manual PLMN -> sequence idle. */
        dr.drg_scan_got = 1;
        if (camp_dereg_next(&dr) != DRG_NONE) return 205;
        /* With a visible target PLMN -> manual-select fires once. */
        memcpy(dr.manual_plmn, "25506", 6);
        dr.drg_target_visible = 1;
        if (camp_dereg_next(&dr) != DRG_MANUAL) return 206;
        dr.drg_manual_sent = 1; dr.drg_manual_ack = 1; dr.drg_done = 1;
        if (camp_dereg_next(&dr) != DRG_NONE) return 207;
        /* camp_dereg_reply: OFF/ON acks advance; a scan error ends the run. */
        struct camp_driver dq;
        memset(&dq, 0, sizeof dq);
        dq.dereg_enabled = 1; dq.drg_off_sent = 1;
        if (!camp_dereg_reply(&dq, CAMP_POWER_COMMAND, 0) || !dq.drg_off_ack ||
            dq.drg_done) return 208;
        dq.drg_on_sent = 1;
        if (!camp_dereg_reply(&dq, CAMP_POWER_COMMAND, 0) || !dq.drg_on_ack)
            return 209;
        dq.drg_scan_sent = 1;
        if (!camp_dereg_reply(&dq, REG_SCAN, 2) || !dq.drg_scan_got ||
            !dq.drg_done) return 210;
    }
    /* VERDICT 16: PIN1-verify / dial / hangup frame byte-exactness and the
     * activation-call sub-sequence walk. */
    {
        /* PIN verify (0x0201, len 38): [12]=len, [13..]=PIN ASCII. The test
         * vector is an arbitrary non-secret digit string. */
        uint8_t pv[CALL_PIN_LEN];
        static const uint8_t pv_e[CALL_PIN_LEN] = {
            0,0,0x01,0x02,38,0,0x78,0x56,0x34,0x12,0,0,
            4,'4','7','2','9'};
        make_pin_verify_request(pv, "4729", 0x12345678);
        if (!same_bytes(pv, pv_e, sizeof pv)) return 211;
        /* PIN longer than the 8-char cap is truncated to 8 in [12] and body. */
        uint8_t pv2[CALL_PIN_LEN];
        make_pin_verify_request(pv2, "123456789", 0x12345678);
        if (pv2[CALL_PIN_LEN_OFF] != 8 || pv2[CALL_PIN_OFF + 7] != '8' ||
            pv2[CALL_PIN_OFF + 8] != 0) return 212;
        /* DIAL (0x0001, len 104): voice=0, numlen, number, TOA intl, present=1. */
        uint8_t dl[CALL_DIAL_LEN];
        make_dial_request(dl, "+380953444757", 0x12345678);
        if (dl[0] || dl[1] || dl[2] != 0x01 || dl[3] != 0x00 ||
            dl[4] != 104 || dl[CALL_DIAL_TYPE_OFF] != 0 ||
            dl[CALL_DIAL_NUMLEN_OFF] != 13 ||
            dl[CALL_DIAL_NUM_OFF] != '+' || dl[CALL_DIAL_NUM_OFF + 12] != '7' ||
            dl[CALL_DIAL_TOA_OFF] != CALL_DIAL_TOA_INTL ||
            dl[CALL_DIAL_PRESENT_OFF] != 1 ||
            dl[CALL_DIAL_CLIR_OFF] != 0) return 213;
        /* A national (no '+') number gets the national TOA. */
        uint8_t dl2[CALL_DIAL_LEN];
        make_dial_request(dl2, "0953444757", 0x12345678);
        if (dl2[CALL_DIAL_TOA_OFF] != CALL_DIAL_TOA_NATL ||
            dl2[CALL_DIAL_NUMLEN_OFF] != 10) return 214;
        /* HANGUP (0x0008, len 20): [12]=index, [16]=1. */
        uint8_t hu[CALL_HANGUP_LEN];
        static const uint8_t hu_e[CALL_HANGUP_LEN] = {
            0,0,0x08,0x00,20,0,0x78,0x56,0x34,0x12,0,0,
            1,0,0,0, 1,0,0,0};
        make_hangup_request(hu, 1, 0x12345678);
        if (!same_bytes(hu, hu_e, sizeof hu)) return 215;
        /* camp_call_next walk: gated on reg_complete + armed + CS registered. */
        struct camp_driver cc;
        memset(&cc, 0, sizeof cc);
        cc.reg_complete = 1;
        memcpy(cc.call_number, "+380953444757", 13);
        cc.call_enabled = 1;
        if (camp_call_next(&cc) != CLL_NONE) return 216;  /* no voice reg yet */
        cc.voice_reg_known = 1; cc.voice_reg_state = 3;    /* denied */
        if (camp_call_next(&cc) != CLL_NONE) return 217;
        cc.voice_reg_state = 5;                             /* roaming */
        if (camp_call_next(&cc) != CLL_DIAL) return 218;
        cc.call_dial_sent = 1;
        if (camp_call_next(&cc) != CLL_NONE) return 219;    /* awaiting dial ack */
        cc.call_dialed_ok = 1;
        if (camp_call_next(&cc) != CLL_POLL) return 220;
        cc.call_poll_count = CALL_POLL_MAX;
        if (camp_call_next(&cc) != CLL_HANGUP) return 221;
        cc.call_hangup_sent = 1;
        if (camp_call_next(&cc) != CLL_NONE) return 222;
        /* camp_call_reply: PIN ok -> re-read SIM; dial error -> call done;
         * list parse -> count/state/index; hangup ack -> done. */
        struct camp_driver cr;
        memset(&cr, 0, sizeof cr);
        cr.pin_verify_sent = 1;
        if (!camp_call_reply(&cr, NULL, 0, CALL_PIN_VERIFY, 0) ||
            !cr.pin_verified || !cr.sim_change_pending) return 223;
        struct camp_driver ce;
        memset(&ce, 0, sizeof ce);
        ce.call_dial_sent = 1;
        if (!camp_call_reply(&ce, NULL, 0, CALL_DIAL, 2) || !ce.call_done)
            return 224;
        struct camp_driver cl;
        memset(&cl, 0, sizeof cl);
        cl.call_dial_sent = 1; cl.call_dialed_ok = 1;
        uint8_t lst[CALL_LIST_ENTRY_OFF + 8];
        memset(lst, 0, sizeof lst);
        put_little32(lst + CALL_LIST_COUNT_OFF, 1);
        lst[CALL_LIST_ENTRY_OFF + CALL_LIST_STATE_OFF] = 3;   /* dialing */
        put_little32(lst + CALL_LIST_ENTRY_OFF + CALL_LIST_INDEX_OFF, 1);
        if (!camp_call_reply(&cl, lst, sizeof lst, CALL_LIST, 0) ||
            cl.call_seen_count != 1 || cl.call_last_state != 3 ||
            cl.call_index != 1) return 225;
    }
    /* VERDICT 17: len-16 scanType sweep frame byte-exactness + sweep walk. */
    {
        /* scanType 2 passes through at [12]; header opcode 0x0706 len 16. */
        uint8_t s16[REG_SCAN16_LEN];
        static const uint8_t s16_e[REG_SCAN16_LEN] = {
            0,0,0x06,0x07,16,0,0x78,0x56,0x34,0x12,0,0, 2,0,0,0};
        make_scan16_request(s16, 2, 0x12345678);
        if (!same_bytes(s16, s16_e, sizeof s16)) return 226;
        /* VERDICT 26: StartNetworkScan 0x0734, one EUTRAN specifier, empty
         * band and channel lists (scan every EUTRAN band). 100-byte frame. */
        uint8_t sns[SCAN734_LEN];
        static const uint8_t sns_e[SCAN734_LEN] = {
            0,0,0x34,0x07,100,0,0x78,0x56,0x34,0x12,0,0, /* hdr: op 0x734 len100 */
            0,          /* +12 scanType ONE_SHOT */
            0,0,        /* +13 interval */
            60,0,       /* +15 maxSearchTime=60 */
            0,          /* +17 incrementalResults */
            3,0,        /* +18 periodicity=3 */
            1,          /* +20 numSpecifiers */
            0,          /* +21 numMccMncs */
            3           /* +22 RAN=EUTRAN; bands_length and channels stay 0 */
            /* remaining specifier bytes are zero via the initializer */ };
        make_startscan_request(sns, 0x12345678);
        if (!same_bytes(sns, sns_e, sizeof sns)) return 241;
        /* One 0x0734, only from the post-radio-on scan step. Not while camped. */
        struct camp_driver ss;
        memset(&ss, 0, sizeof ss);
        if (camp_scan734_next(&ss) != 0 || camp_dereg_next(&ss) != DRG_NONE)
            return 242;
        ss.reg_complete = 1; ss.scan734 = 1;
        if (camp_scan734_next(&ss) != 0) return 243;
        if (camp_dereg_next(&ss) != DRG_OFF) return 244;
        ss.drg_off_sent = 1; ss.drg_off_ack = 1; ss.drg_off_confirmed = 1;
        if (camp_dereg_next(&ss) != DRG_ON) return 245;
        ss.drg_on_sent = 1; ss.drg_on_ack = 1;
        if (camp_dereg_next(&ss) != DRG_SCAN) return 246;
        ss.drg_scan_sent = 1; ss.scan734_sent = 1;
        if (!camp_scan734_reply(&ss, SCAN734_GET, 0) || !ss.scan734_done ||
            !ss.drg_done) return 247;
        if (camp_scan734_next(&ss) != 0 || camp_dereg_next(&ss) != DRG_NONE)
            return 242;
        /* Out-of-range scanType clamps to 0 (stock sub/cmp/csel). */
        uint8_t s16b[REG_SCAN16_LEN];
        make_scan16_request(s16b, 9, 0x12345678);
        if (little32(s16b + SCAN16_MODE_OFF) != 0) return 227;
        make_scan16_request(s16b, 5, 0x12345678);
        if (little32(s16b + SCAN16_MODE_OFF) != 5) return 228;
        /* Sweep: fires only post-reg + armed; one mode per call; stops on list. */
        struct camp_driver sc;
        memset(&sc, 0, sizeof sc);
        sc.reg_complete = 1;
        if (camp_scan16_next(&sc) != 0) return 229;      /* not armed */
        sc.scan16_enabled = 1;
        if (camp_scan16_next(&sc) != REG_SCAN16) return 230;
        sc.scan16_sent = 1; sc.scan16_mode = 0;
        if (camp_scan16_next(&sc) != 0) return 231;      /* awaiting reply */
        /* Each error reply advances to the next mode, up to SCAN16_MODE_COUNT. */
        int guard;
        for (guard = 0; guard < SCAN16_MODE_COUNT; guard++) {
            if (!camp_scan16_reply(&sc, NULL, 0, REG_SCAN16, 2)) return 232;
            if (sc.scan16_done) break;
            if (camp_scan16_next(&sc) != REG_SCAN16) return 233;
            sc.scan16_sent = 1;
        }
        if (!sc.scan16_done || sc.scan16_got_list ||
            sc.scan16_fired != SCAN16_MODE_COUNT) return 234;
        /* A success reply with a list parses and ends the sweep immediately. */
        struct camp_driver sg;
        memset(&sg, 0, sizeof sg);
        sg.reg_complete = 1; sg.scan16_enabled = 1; sg.scan16_sent = 1;
        uint8_t avn[SIT_NET_AVN_LIST_OFFSET + SIT_NET_AVN_ENTRY_STRIDE];
        memset(avn, 0, sizeof avn);
        put_little32(avn + SIT_NET_AVN_COUNT_OFFSET, 1);
        memcpy(avn + SIT_NET_AVN_LIST_OFFSET + SIT_NET_AVN_ENTRY_PLMN, "25503#", 6);
        put_little32(avn + SIT_NET_AVN_LIST_OFFSET + SIT_NET_AVN_ENTRY_STATUS, 1);
        if (!camp_scan16_reply(&sg, avn, sizeof avn, REG_SCAN16, 0) ||
            !sg.scan16_got_list || !sg.scan16_done) return 235;
    }
    /* VERDICT 20: RAT-gate bitmap transform, frame, and dispatch sequencing. */
    {
        if (raf_to_sit_ratbm((uint32_t)RAF_LTE) != 0x80u) return 240;
        if (raf_to_sit_ratbm((uint32_t)RAF_GSM) != 0x200u) return 241;
        if (raf_to_sit_ratbm((uint32_t)RAF_UMTS) != 0x8u) return 242;
        if (raf_to_sit_ratbm((uint32_t)RATBM_RAF_LTE_WCDMA_GSM) !=
                (uint32_t)RATBM_WIRE_LTE_WCDMA_GSM ||
            RATBM_WIRE_LTE_WCDMA_GSM != 0x3fe) return 243;

        if (raf_to_sit_ratbm((uint32_t)RATBM_RAF_STOCK) != (uint32_t)RATBM_WIRE_STOCK)
            return 243;
        uint8_t rb[RATBM_SET_LEN];
        make_opx_u32_request(rb, RATBM_SET, RATBM_SET_LEN, 0x12345678u,
                             raf_to_sit_ratbm(RATBM_RAF_STOCK));
        if (rb[0] || rb[2] != 0x4f || rb[3] != 0x07 ||
            rb[4] != RATBM_SET_LEN || rb[5]) return 244;
        if (little32(rb + 6) != 0x12345678u) return 245;
        static const uint8_t rb_stock[4] = { 0xfe, 0x03, 0x04, 0x00 };
        if (!same_bytes(rb + RATBM_OFF, rb_stock, sizeof rb_stock)) return 246;

        if (RATBM_GET_LEN != 12 || RATBM_BANDMODE_LEN != 12 ||
            RATBM_GET != 0x0750 || RATBM_BANDMODE_GET != 0x0709 ||
            RATBM_SET != 0x074f) return 247;

        struct camp_driver rr;
        memset(&rr, 0, sizeof rr);
        if (camp_ratbm_next(&rr) != 0) return 248;          /* not armed */
        rr.reg_complete = 1; rr.ratbm_enabled = 1;
        if (camp_ratbm_next(&rr) != RATBM_GET) return 248;
        rr.ratbm_get1_sent = 1;
        if (camp_ratbm_next(&rr) != RATBM_BANDMODE_GET) return 248;
        rr.ratbm_band_sent = 1;
        if (camp_ratbm_next(&rr) != RATBM_SET) return 248;
        rr.ratbm_set_sent = 1;
        if (camp_ratbm_next(&rr) != RATBM_GET) return 248;
        rr.ratbm_get2_sent = 1;
        uint8_t gr[RATBM_OFF + 4];
        memset(gr, 0, sizeof gr);
        put_little32(gr + RATBM_OFF, 0x3feu);
        if (!camp_ratbm_reply(&rr, gr, sizeof gr, RATBM_GET, 0) ||
            !rr.ratbm_done) return 249;
        if (camp_ratbm_next(&rr) != 0) return 249;          /* done */
    }
    /* VERDICT 21: baseband-version GET frame + dispatch sequencing. */
    {
        uint8_t bf[BBVER_LEN];
        make_opx_byte_request(bf, BBVER_GET, BBVER_LEN, 0x12345678u, BBVER_MASK);
        if (bf[0] || bf[2] != 0x01 || bf[3] != 0x09 || bf[4] != BBVER_LEN ||
            bf[5]) return 250;
        if (little32(bf + 6) != 0x12345678u || bf[12] != 0xFF) return 251;

        struct camp_driver bb;
        memset(&bb, 0, sizeof bb);
        if (camp_bbver_next(&bb) != 0) return 252;          /* not armed */
        bb.reg_complete = 1; bb.bbver_enabled = 1;
        if (camp_bbver_next(&bb) != BBVER_GET) return 252;
        bb.bbver_sent = 1;
        uint8_t br[BBVER_SWVER_OFF + 8];
        memset(br, 0, sizeof br);
        memcpy(br + BBVER_SWVER_OFF, "g5300q", 6);
        if (!camp_bbver_reply(&bb, br, sizeof br, BBVER_GET, 0) ||
            !bb.bbver_done) return 253;
        if (camp_bbver_next(&bb) != 0) return 253;          /* done */
    }
    /* SMS-SUBMIT: recovered 271-byte 0x0100, empty-SMSC fallback, TPDU of a
     * short test number that is not a live destination. */
    {
        uint8_t sm[SMS_SEND_LEN];
        if (!make_sms_request(sm, "+100", 0x12345678u)) return 260;
        if (sm[2] != 0x00 || sm[3] != 0x01 || sm[4] != 0x0f || sm[5] != 0x01)
            return 261;
        if (little32(sm + 6) != 0x12345678u || sm[SMS_PROP_OFF] != 0 ||
            sm[SMS_SMSC_OFF] != 1 || sm[SMS_SMSC_OFF + 1] != 0)
            return 262;
        if (sm[SMS_PDU_LEN_OFF] != 11 || sm[SMS_PDU_OFF] != 0x01 ||
            sm[SMS_PDU_OFF + 1] != 0x00 || sm[SMS_PDU_OFF + 2] != 3 ||
            sm[SMS_PDU_OFF + 3] != 0x91 || sm[SMS_PDU_OFF + 4] != 0x01 ||
            sm[SMS_PDU_OFF + 5] != 0xf0 || sm[SMS_PDU_OFF + 8] != 2 ||
            sm[SMS_PDU_OFF + 9] != 0xef || sm[SMS_PDU_OFF + 10] != 0x35)
            return 263;
        if (sm[SMS_PDU_OFF + 11] != 0) return 264;
        if (make_sms_request(sm, "", 1) || make_sms_request(sm, "+12a", 1))
            return 265;
        struct camp_driver sx;
        memset(&sx, 0, sizeof sx);
        sx.reg_complete = 1;
        if (camp_sms_next(&sx) != 0) return 266;
        memcpy(sx.sms_number, "+100", 4);
        sx.sms_enabled = 1;
        if (camp_sms_next(&sx) != 0) return 267;
        sx.voice_reg_known = 1; sx.voice_reg_state = 3;
        if (camp_sms_next(&sx) != 0) return 268;
        sx.voice_reg_state = 1;
        if (camp_sms_next(&sx) != SMS_SEND) return 269;
        sx.capquery = 1;
        if (camp_sms_next(&sx) != 0) return 270;
        sx.capquery = 0; sx.sms_sent = 1;
        if (camp_sms_next(&sx) != 0) return 271;
        if (!camp_sms_reply(&sx, SMS_SEND, 0) || !sx.sms_done) return 272;
        if (camp_sms_next(&sx) != 0) return 273;
    }
    {
        struct camp_driver vx;
        memset(&vx, 0, sizeof vx);
        if (camp_volteprov_next(&vx) != 0) return 274;
        vx.reg_complete = 1;
        if (camp_volteprov_next(&vx) != VOLTE_PROV_GET) return 275;
        vx.capquery = 1;
        if (camp_volteprov_next(&vx) != 0) return 276;
        vx.capquery = 0;
        vx.volteprov_sent = 1;
        if (camp_volteprov_next(&vx) != 0) return 277;
        uint8_t vr[13];
        memset(vr, 0, sizeof vr);
        vr[VOLTE_PROV_STATUS_OFF] = 0x04;
        if (!camp_volteprov_reply(&vx, vr, sizeof vr, VOLTE_PROV_GET, 0) ||
            !vx.volteprov_done) return 278;
        if (camp_volteprov_next(&vx) != 0) return 279;
        struct camp_driver ve;
        memset(&ve, 0, sizeof ve);
        ve.volteprov_sent = 1;
        if (!camp_volteprov_reply(&ve, NULL, 0, VOLTE_PROV_GET, 2) ||
            !ve.volteprov_done) return 280;
    }
    {
        uint8_t eq[EM_QUERY_LEN];
        static const uint8_t eq_e[EM_QUERY_LEN] = {
            0,0,0x12,0x07,14,0,0x78,0x56,0x34,0x12,0,0, 1,0xff};
        make_emquery_request(eq, 0x12345678u);
        if (!same_bytes(eq, eq_e, sizeof eq)) return 281;
        struct camp_driver ex;
        memset(&ex, 0, sizeof ex);
        if (camp_emquery_next(&ex) != 0) return 282;
        ex.reg_complete = 1;
        if (camp_emquery_next(&ex) != EM_QUERY) return 283;
        ex.capquery = 1;
        if (camp_emquery_next(&ex) != 0) return 284;
        ex.capquery = 0;
        ex.emquery_sent = 1;
        if (camp_emquery_next(&ex) != 0) return 285;
        if (!camp_emquery_reply(&ex, EM_QUERY, 0) || !ex.emquery_done)
            return 286;
        if (camp_emquery_next(&ex) != 0) return 287;
    }
    {
        struct camp_driver dx;
        uint8_t body[13];
        memset(&dx, 0, sizeof dx);
        if (DCALL_LIST != 0x0602) return 288;
        if (camp_dcall_next(&dx) != 0) return 289;
        dx.reg_complete = 1;
        if (camp_dcall_next(&dx) != DCALL_LIST) return 290;
        dx.capquery = 1;
        if (camp_dcall_next(&dx) != 0) return 291;
        dx.capquery = 0;
        dx.dcall_sent = 1;
        if (camp_dcall_next(&dx) != 0) return 292;
        memset(body, 0, sizeof body);
        body[DCALL_COUNT_OFF] = 2;
        if (!camp_dcall_reply(&dx, body, sizeof body, DCALL_LIST, 0) ||
            !dx.dcall_done || dx.dcall_count != 2) return 293;
        if (camp_dcall_next(&dx) != 0) return 294;
        memset(&dx, 0, sizeof dx);
        dx.dcall_sent = 1;
        if (!camp_dcall_reply(&dx, NULL, 0, DCALL_LIST, 6) ||
            !dx.dcall_done || dx.dcall_count != -1) return 295;
    }
    return 0;
}
#endif

static int test_cc(void)
{
    char dir[] = "/tmp/cc-test-XXXXXX";
    char path[256];
    uint8_t open_f[128];
    uint8_t rd[24];
    uint8_t got[64];
    const char *rel = "/vendor/firmware/carrierconfig/manifests/hi";
    size_t rel_n = strlen(rel);
    int p[2];
    int file;
    ssize_t n;
    if (!mkdtemp(dir)) return 1;
    snprintf(path, sizeof path, "%s/manifests", dir);
    if (mkdir(path, 0700)) return 1;
    snprintf(path, sizeof path, "%s/manifests/hi", dir);
    file = open(path, O_WRONLY | O_CREAT | O_EXCL | O_CLOEXEC, 0600);
    if (file < 0 || write(file, "hello", 5) != 5) return 1;
    close(file);
    if (cc_root_fd >= 0) close(cc_root_fd);
    cc_root_fd = -1;
    memset(cc_slot, 0, sizeof cc_slot);
    cc_root_path = dir;
    if (pipe(p)) return 1;
    memset(open_f, 0, sizeof open_f);
    open_f[0] = 4;
    open_f[2] = 7;
    put_little32(open_f + 4, (uint32_t)(8 + rel_n + 1));
    put_little32(open_f + 8, 9);
    memcpy(open_f + 16, rel, rel_n + 1);
    if (!cc_serve(p[1], open_f, 16 + rel_n + 1)) return 1;
    n = read(p[0], got, sizeof got);
    if (n != 20 || got[0] != 3 || little32(got + 8) != 0 ||
        little32(got + 12) != 9 || little32(got + 16) != 5)
        return 1;
    memset(rd, 0, sizeof rd);
    rd[0] = 6;
    rd[2] = 8;
    put_little32(rd + 4, 16);
    put_little32(rd + 8, 9);
    put_little32(rd + 16, 5);
    put_little32(rd + 20, 1);
    if (!cc_serve(p[1], rd, sizeof rd)) return 1;
    n = read(p[0], got, sizeof got);
    if (n != 25 || got[0] != 1 || little32(got + 16) != 5 ||
        memcmp(got + 20, "hello", 5) != 0)
        return 1;
    rd[0] = 6;
    put_little32(rd + 20, 2);
    if (!cc_serve(p[1], rd, sizeof rd)) return 1;
    n = read(p[0], got, 16);
    if (n != 16 || little32(got + 8) != 3) return 1;
    {
        struct owner ow;
        int64_t now = monotonic_ms();
        memset(&ow, 0, sizeof ow);
        ow.phase = WAIT_7;
        ow.rfs = p[1];
        ow.deadline_ms = ow.total_deadline_ms = now + 10000;
        if (now < 0 || feed_rfs(&ow, open_f, 16 + rel_n + 1, now) ||
            ow.phase != WAIT_7) return 1;
        n = read(p[0], got, sizeof got);
        if (n != 20 || little32(got + 8) != 0) return 1;
        memset(open_f, 0, sizeof open_f);
        memcpy(open_f + 16, "/config/other", 14);
        open_f[0] = 4;
        put_little32(open_f + 4, 8u + 14u);
        put_little32(open_f + 8, 4);
        now = monotonic_ms();
        if (feed_rfs(&ow, open_f, 30, now) || ow.phase != WAIT_7) return 1;
        n = read(p[0], got, 16);
        if (n != 16 || little32(got + 8) != 3) return 1;
        memset(open_f, 0, 20);
        open_f[0] = 3;
        put_little32(open_f + 4, 12);
        put_little32(open_f + 12, 9);
        now = monotonic_ms();
        if (feed_rfs(&ow, open_f, 20, now) || ow.phase != WAIT_7) return 1;
        if (fcntl(p[0], F_SETFL, O_NONBLOCK) < 0) return 1;
        n = read(p[0], got, sizeof got);
        if (n > 0) return 1;
        memset(open_f, 0, 16);
        open_f[0] = 1;
        open_f[2] = 1;
        put_little32(open_f + 4, 8);
        put_little32(open_f + 12, 9);
        now = monotonic_ms();
        if (feed_rfs(&ow, open_f, 16, now) || ow.phase != WAIT_7) return 1;
        n = read(p[0], got, 16);
        if (n != 16 || got[0] != 3 || little32(got + 8) != 0 ||
            little32(got + 12) != 9) return 1;
        memset(open_f, 0, 12);
        open_f[0] = 5;
        put_little32(open_f + 4, 4);
        put_little32(open_f + 8, 9);
        now = monotonic_ms();
        if (feed_rfs(&ow, open_f, 12, now) || ow.phase != WAIT_7) return 1;
        n = read(p[0], got, 16);
        if (n != 16 || little32(got + 8) != 0) return 1;
        memset(open_f, 0, 20);
        open_f[0] = 3;
        put_little32(open_f + 4, 12);
        put_little32(open_f + 12, 9);
        now = monotonic_ms();
        if (feed_rfs(&ow, open_f, 20, now) || ow.phase != WAIT_7) return 1;
        n = read(p[0], got, sizeof got);
        if (n > 0) return 1;
    }
    close(p[0]);
    close(p[1]);
    if (cc_root_fd >= 0) close(cc_root_fd);
    cc_root_fd = -1;
    cc_root_path = CC_ROOT;
    return 0;
}

static int self_test(void)
{
    static const uint8_t abc[] = {'a', 'b', 'c'};
    static const uint8_t abc_sha256[32] = {
        0xba,0x78,0x16,0xbf,0x8f,0x01,0xcf,0xea,
        0x41,0x41,0x40,0xde,0x5d,0xae,0x22,0x23,
        0xb0,0x03,0x61,0xa3,0x96,0x17,0x7a,0x9c,
        0xb4,0x10,0xff,0x61,0xf2,0x00,0x15,0xad
    };
    struct sha256 hash;
    struct owner o = {.phase = WAIT_7};
    uint8_t digest[32], data[RFS_FRAME_MAX] = {0};
    sha_init(&hash);
    sha_update(&hash, abc, sizeof abc);
    sha_final(&hash, digest);
    if (!same_bytes(digest, abc_sha256, sizeof digest)) return 11;
    if (classify(&o, request_7, sizeof request_7) != STATUS_7) return 12;
    o.phase = WAIT_3;
    if (classify(&o, request_3, sizeof request_3) != NO_REPLY)
        return 2;
    o.phase = WAIT_6;
    if (classify(&o, request_6, sizeof request_6) != GRANT_1 ||
        RFS_TRANSFER_BYTES != 94 * FIRST_CHUNK + 318 ||
        test_framing())
        return 3;
    o.phase = WAIT_DATA;
    o.grant_attempted = RFS_GRANTS_MAX;
    o.chunks_stored = RFS_GRANTS_MAX - 1;
    o.received_bytes = 94 * FIRST_CHUNK;
    o.expected_chunk = 318;
    data[0] = 2; data[2] = 1;
    put_little32(data + 4, 12 + 318);
    put_little32(data + 12, 3);
    put_little32(data + 16, 318);
    if (valid_frame_length(data, 338) != 338 ||
        classify(&o, data, 338) != STORE_CHUNK)
        return 4;
    data[2] = 2;
    if (classify(&o, data, 338) != BAD_FRAME)
        return 5;
    data[2] = 1;
    put_little32(data + 16, 319);
    if (classify(&o, data, 338) != BAD_FRAME)
        return 6;
    o.phase = TERMINAL;
    if (classify(&o, data, 338) != BAD_FRAME)
        return 7;
#ifdef RFS_HOST_TEST
    if (test_host_storage_faults() || test_host_finish_candidate(0, 0) ||
        test_host_finish_candidate(1, 0) ||
        test_host_finish_candidate(2, 0) ||
        test_host_finish_candidate(1, 1) ||
        test_host_finish_candidate(2, 1) ||
        test_host_transcript(0) || test_host_transcript(1) ||
        test_sit_observer() || test_sit_event_trace() ||
        test_host_failure_diagnostics() || test_host_final_frame_shape() ||
        test_host_padded_final_refusals())
        return 8;
#endif
#ifdef SAAIOS_RFS_CAMP
    if (test_cc() || test_camp_builders() || test_camp_observer() || test_camp_reg()
#ifdef RFS_HOST_TEST
        || test_camp_dispatch() || test_camp_prober() || test_setup_data_call()
#endif
       )
        return 9;
#endif
    zero_bytes(data, sizeof data);
    zero_bytes(digest, sizeof digest);
#ifdef SAAIOS_RFS_CAMP
    puts("PASS combined RFS quarantine + camp dispatch self-test");
#else
    puts("PASS full-RFS quarantine, SIT observer, and SHA-256 self-test");
#endif
    return 0;
}

int saaios_run_camp_owner(int ipc, int rfs, int ready)
{
    return run_owner(ipc, rfs, ready);
}

#ifndef SAAIOS_EMBEDDED_OWNER
int main(int argc, char **argv)
{
    if (argc == 2 && (!strcmp(argv[1], "--mode") ||
                      !strcmp(argv[1], "--mode=rfs-full-quarantine"))) {
#ifdef SAAIOS_RFS_CAMP
        puts("rfs-camp-combined");
#else
        puts("rfs-full-quarantine");
#endif
        return 0;
    }
    if (argc == 2 && !strcmp(argv[1], "self-test"))
        return self_test();
    int ipc, rfs, ready;
    if (parse_args(argc, argv, &ipc, &rfs, &ready)) {
        fputs("usage: modem-rfs-full-quarantine-owner self-test | --mode | "
              "--ipc-fd N --rfs-fd N --ready-fd N\n", stderr);
        return 64;
    }
    return run_owner(ipc, rfs, ready);
}
#endif
