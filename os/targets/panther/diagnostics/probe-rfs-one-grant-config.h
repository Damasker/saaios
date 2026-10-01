/* Injected only for the separate, explicit one-grant RFS handoff probe. */
#ifndef PROBE_OWNER_HANDOFF
#error "RFS probe requires the pre-FIN owner handoff"
#endif
#ifdef PROBE_QUERY_SIM
#error "RFS probe cannot run a competing SIM reader"
#endif
#define PROBE_OWNER_EXEC "/data/saaios/bin/modem-rfs-one-grant-owner"
#define PROBE_OWNER_LOG "/data/saaios/var/modem-rfs-one-grant-owner.log"
/* Candidate fsync and source reread happen before the owner signals READY. */
#define PROBE_OWNER_READY_TIMEOUT_MS 15000
#define PROBE_OWNER_HOLD_ON_FAILURE 1
