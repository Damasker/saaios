/* Host fixture: the operator reply must never be decoded as registration. */
#include <stdio.h>
#include "sit-network-layout.h"

int main(void) {
    if (SIT_NET_VOICE_REG != 0x0700 || SIT_NET_DATA_REG != 0x0701 ||
        SIT_NET_OPERATOR != 0x0702 ||
        !sit_net_is_registration(SIT_NET_VOICE_REG) ||
        !sit_net_is_registration(SIT_NET_DATA_REG) ||
        sit_net_is_registration(SIT_NET_OPERATOR) ||
        SIT_NET_REG_STATE_OFFSET != 12 || SIT_NET_DATA_TECH_OFFSET != 15)
        return 1;
    puts("PASS SIT voice/data/operator ID fixture");
    return 0;
}
