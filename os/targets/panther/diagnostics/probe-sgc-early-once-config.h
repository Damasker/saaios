/* Separate early-SGC experiment; never substitute the passive owner. The
 * early variant dispatches the one factory 0x0404 on the early radio trigger
 * (0x0803 then 0x0802 raw 0), before the settled READY/ON baseline, mutually
 * exclusive with the late SGC and the active scan. */
#ifndef PROBE_OWNER_HANDOFF
#error "SGC probe requires owner handoff"
#endif
#ifdef PROBE_QUERY_SIM
#error "SGC probe cannot run a competing SIM reader"
#endif
#define PROBE_OWNER_EXEC "/data/saaios/bin/modem-channel-owner-sgc-early-once"
#define PROBE_OWNER_LOG "/data/saaios/var/modem-channel-owner-sgc-early-once.log"
#define PROBE_OWNER_HOLD_ON_FAILURE 1
