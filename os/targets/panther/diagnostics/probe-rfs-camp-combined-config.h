/* Injected only for the separate, manual combined RFS-quarantine + camp probe.
 * The owner serves the protected-NV cmd7/cmd3/cmd6 sequence into the quarantine
 * copy (original EFS is never mounted read-write) and dispatches the stock
 * stage-1 0x093f -> 0x0404 -> 0x0800 on the early radio edge. */
#ifndef PROBE_OWNER_HANDOFF
#error "RFS probe requires the pre-FIN owner handoff"
#endif
#ifdef PROBE_QUERY_SIM
#error "RFS probe cannot run a competing SIM reader"
#endif
#define PROBE_OWNER_EXEC "/data/saaios/bin/modem-rfs-camp-combined-owner"
#define PROBE_OWNER_LOG "/data/saaios/var/modem-rfs-camp-combined-owner.log"
/* Candidate clone/fsync/source reread precede READY, not the frame run. */
#define PROBE_OWNER_READY_TIMEOUT_MS 15000
#define PROBE_OWNER_HOLD_ON_FAILURE 1
