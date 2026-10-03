#ifndef SAAIOS_SIT_NETWORK_LAYOUT_H
#define SAAIOS_SIT_NETWORK_LAYOUT_H

/* CP2A factory sit-stream: BuildNetworkRegistrationState(1/2) selects
 * 0x0700/0x0701. BuildOperator is 0x0702, not a registration response. */
enum {
    SIT_NET_VOICE_REG = 0x0700,
    SIT_NET_DATA_REG = 0x0701,
    SIT_NET_OPERATOR = 0x0702,
    SIT_NET_SELECTION_MODE = 0x0703,
    SIT_NET_SELECTION_AUTO = 0x0704,
    SIT_NET_SELECTION_MANUAL = 0x0705,
    SIT_NET_AVAILABLE_NETWORKS = 0x0706,
    SIT_NET_PREFERRED_SET = 0x070a,
    SIT_NET_PREFERRED_GET = 0x070b,
    SIT_NET_ALLOW_DATA = 0x0710,
    /* GET_AVAILABLE_NETWORKS (0x0706) response, recovered from libsitril.so
     * (efcca0d5) ProtocolNetAvailableNetworkAdapter::{GetCount,GetNetwork}:
     * count=int32 at payload[12]; the per-PLMN list starts at payload[16]
     * with a 14-byte stride. Within each entry: [0..3]=RAT raw, [4..9]=PLMN
     * numeric ASCII ([9]=='#' marks a 2-digit MNC), [10..13]=status
     * (1=available, 2=current, 3=forbidden). */
    SIT_NET_AVN_COUNT_OFFSET = 12,
    SIT_NET_AVN_LIST_OFFSET = 16,
    SIT_NET_AVN_ENTRY_STRIDE = 14,
    SIT_NET_AVN_ENTRY_RAT = 0,
    SIT_NET_AVN_ENTRY_PLMN = 4,
    SIT_NET_AVN_ENTRY_STATUS = 10,
    /* SET_NETWORK_SELECTION_MANUAL (0x0705), recovered from libsitril.so
     * ProtocolNetworkBuilder::BuildSetNetworkSelectionManual(int,char const*):
     * total frame len 22; payload[0]=RAT int32 (0=any), payload[4..9]=PLMN
     * numeric ASCII (5 or 6 chars, trailing byte preset to '#'). */
    SIT_NET_SEL_MANUAL_LEN = 22,
    SIT_NET_SEL_MANUAL_RAT_OFFSET = 12,
    SIT_NET_SEL_MANUAL_PLMN_OFFSET = 16,
    SIT_NET_SEL_MANUAL_PLMN_MAX = 6,
    SIT_NET_REG_STATE_OFFSET = 12,
    SIT_NET_REJECT_OFFSET = 13,
    SIT_NET_DATA_TECH_OFFSET = 15,
    /* Full reg-state field offsets recovered from libsitril.so (efcca0d5)
     * ProtocolNetVoiceRegStateAdapter (0x0700) / ProtocolNetDataRegStateAdapter
     * (0x0701) fixed-offset accessors. reg_state[12]/reject[13] are shared;
     * the remaining fields differ by one byte because data inserts MaxSDC[14].
     * These carry network topology only (RAT/LAC/cell/PSC) -- no subscriber id;
     * serving PLMN (MCC/MNC) is NOT in the reg-state frame (it is 0x0702). */
    SIT_NET_VOICE_TECH_OFFSET = 14,
    SIT_NET_VOICE_LAC_OFFSET = 15,
    SIT_NET_VOICE_CID_OFFSET = 19,
    SIT_NET_VOICE_PSC_OFFSET = 23,
    SIT_NET_DATA_MAXSDC_OFFSET = 14,
    SIT_NET_DATA_LAC_OFFSET = 16,
    SIT_NET_DATA_CID_OFFSET = 20,
    SIT_NET_DATA_PSC_OFFSET = 24
};

static inline int sit_net_is_registration(unsigned id) {
    return id == SIT_NET_VOICE_REG || id == SIT_NET_DATA_REG;
}

#endif
