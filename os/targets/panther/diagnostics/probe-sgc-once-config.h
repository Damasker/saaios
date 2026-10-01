/* Separate late-SGC experiment; never substitute the passive owner. */
#ifndef PROBE_OWNER_HANDOFF
#error "SGC probe requires owner handoff"
#endif
#ifdef PROBE_QUERY_SIM
#error "SGC probe cannot run a competing SIM reader"
#endif
#define PROBE_OWNER_EXEC "/data/saaios/bin/modem-channel-owner-sgc-once"
#define PROBE_OWNER_LOG "/data/saaios/var/modem-channel-owner-sgc-once.log"
#define PROBE_OWNER_HOLD_ON_FAILURE 1
