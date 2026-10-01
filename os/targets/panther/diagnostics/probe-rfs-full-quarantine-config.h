/* Injected only for the separate, manual full-RFS quarantine probe. */
#ifndef PROBE_OWNER_HANDOFF
#error "RFS probe requires the pre-FIN owner handoff"
#endif
#ifdef PROBE_QUERY_SIM
#error "RFS probe cannot run a competing SIM reader"
#endif
#define PROBE_OWNER_EXEC "/data/saaios/bin/modem-rfs-full-quarantine-owner"
#define PROBE_OWNER_LOG "/data/saaios/var/modem-rfs-full-quarantine-owner.log"
/* Candidate clone/fsync/source reread precede READY, not the 95-frame run. */
#define PROBE_OWNER_READY_TIMEOUT_MS 15000
#define PROBE_OWNER_HOLD_ON_FAILURE 1
