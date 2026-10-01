#ifndef SAAIOS_SIT_SIM_LAYOUT_H
#define SAAIOS_SIT_SIM_LAYOUT_H

/* Factory ProtocolSimStatusAdapter copies wire bytes at object +16.
 * BuildRilCardStatusApplications uses x27=object+84, then:
 * ldurb -53 -> RIL app_type; -52 -> app_state; -51 -> perso_substate.
 * Thus wire type=15, state=16, perso=17 (NOT state=17).
 * GetPinState(0,1) reads object+88 => wire+72. App stride is 63.
 * No identity fields are exposed here. */
enum {
    SIT_SIM_CARD = 12,
    SIT_SIM_APPS = 14,
    SIT_SIM_APP_TYPE = 15,
    SIT_SIM_APP_STATE = 16,
    SIT_SIM_PERSO_STATE = 17,
    SIT_SIM_PIN1 = 72,
    SIT_SIM_PIN1_REMAIN = 74,
    SIT_SIM_APP_STRIDE = 63
};

#endif
