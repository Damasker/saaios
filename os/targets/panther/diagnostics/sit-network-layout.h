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
    SIT_NET_PREFERRED_SET = 0x070a,
    SIT_NET_PREFERRED_GET = 0x070b,
    SIT_NET_ALLOW_DATA = 0x0710,
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
