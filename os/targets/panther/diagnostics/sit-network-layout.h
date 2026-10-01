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
    SIT_NET_DATA_TECH_OFFSET = 15
};

static inline int sit_net_is_registration(unsigned id) {
    return id == SIT_NET_VOICE_REG || id == SIT_NET_DATA_REG;
}

#endif
