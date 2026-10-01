/* Injected only when building the separate, opt-in scan handoff probe. */
#ifndef PROBE_OWNER_HANDOFF
#error "scan probe requires owner handoff"
#endif
#ifdef PROBE_QUERY_SIM
#error "scan probe cannot run a competing SIM reader"
#endif
#define PROBE_OWNER_EXEC "/data/saaios/bin/modem-channel-owner-scan-once"
#define PROBE_OWNER_LOG "/data/saaios/var/modem-channel-owner-scan-once.log"
