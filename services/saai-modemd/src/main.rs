use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use saai_modemd::boot_model;
use saai_modemd::post_edge;
use saai_modemd::soft_lock::{
    self, SoftLockSnapshot, APP_STATE_PIN, PIN1_DISABLED, PIN1_ENABLED_VERIFIED,
    TRAY_BEARER_CHASE_CMD, TRAY_BEARER_CHASE_ON_DEVICE,
};
use saai_modemd::rfs_policy;
use saai_modemd::runtime_model::{self, QueryAdmission};
use saai_modemd::supervise::{self, FirstAction, HoldNote};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_MODEM_STATE: &str = "/sys/devices/platform/cpif/modem_state";
const DEFAULT_BOOT_ARCHIVE: &str = "/data/saaios/var/boot-archive";
const DEFAULT_NET_CLASS: &str = "/sys/class/net";
const DEFAULT_MOUNTS: &str = "/proc/mounts";
const REVIEWED_CPIF_SHA256: &str =
    "8cdd21d771189af08035dc6b8fc2b90708a83a520ccb0a45570836a0bce1e79c";
const REVIEWED_B_MODEM_SHA256: &str =
    "449eeab3bf70fc4ed0793dce3a1f245447bf54a23b4e666df9881317bfc2344b";

#[derive(Debug, Parser)]
#[command(name = "saai-modemd")]
#[command(about = "Safe Pixel modem boundary; no hardware actions by default")]
struct Args {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Read modem state and bearer names without opening modem endpoints.
    Status {
        #[arg(long, default_value = DEFAULT_MODEM_STATE)]
        modem_state: PathBuf,
        #[arg(long, default_value = DEFAULT_NET_CLASS)]
        net_class: PathBuf,
    },
    /// Check prerequisites for a separately invoked diagnostic run.
    Preflight {
        #[arg(long, default_value = DEFAULT_MODEM_STATE)]
        modem_state: PathBuf,
        #[arg(long, default_value = DEFAULT_MOUNTS)]
        mounts: PathBuf,
        #[arg(long)]
        cpif_ko: Option<PathBuf>,
        #[arg(long)]
        b_modem_bin: Option<PathBuf>,
        #[arg(long, default_value = REVIEWED_CPIF_SHA256)]
        expected_cpif_sha256: String,
        #[arg(long, default_value = REVIEWED_B_MODEM_SHA256)]
        expected_b_modem_sha256: String,
    },
    /// Parse a modem image TOC and validate the reviewed Panther layout.
    /// This is host-safe: it reads one file and performs no hardware actions.
    InspectImage {
        #[arg(long)]
        image: PathBuf,
    },
    /// Detect MODEM-06 soft-lock from numeric fields or a tray/status text file.
    /// Host-safe: no modem endpoints opened; prints the tray-bearer chase command.
    SoftLock {
        /// app_state raw (2 = PIN).
        #[arg(long)]
        app: Option<u8>,
        /// pin1 raw (1 = NOT_VERIFIED, 2 = VERIFIED, 3 = DISABLED).
        #[arg(long)]
        pin1: Option<u8>,
        /// Optional independently measured CP Present byte; SIT status cannot infer it.
        #[arg(long)]
        present: Option<String>,
        /// Optional tray-watch / chase log snippet (no secrets expected).
        #[arg(long)]
        from_text: Option<PathBuf>,
    },
    /// Plan a guarded READY→registration→rmnet path (host-safe; no modem I/O).
    /// PIN verification and SetupDataCall are disarmed unless separately opted in.
    PostEdge {
        #[arg(long)]
        app: Option<u8>,
        #[arg(long)]
        pin1: Option<u8>,
        /// Optional independently measured CP Present byte; not used for the app gate.
        #[arg(long)]
        present: Option<String>,
        #[arg(long)]
        from_text: Option<PathBuf>,
        /// Operator APN hostname (never invent); does not itself arm a data call.
        #[arg(long)]
        apn: Option<String>,
        /// One-line APN file (`apn=…` or bare host). Default on device: /data/saaios/etc/apn.
        #[arg(long)]
        apn_file: Option<PathBuf>,
        /// Include a conditional PIN verification step in the host plan only.
        #[arg(long)]
        allow_pin_verify: bool,
        /// Include SetupDataCall in the host plan only (also needs APN and reg=1/5).
        #[arg(long)]
        allow_setup_data_call: bool,
        /// Optional live counters for bearer_verified check (host-side eval).
        #[arg(long)]
        rmnet_rx: Option<u64>,
        #[arg(long)]
        rmnet_tx: Option<u64>,
        #[arg(long)]
        ipv4: Option<String>,
    },
    /// Say whether this boot may start a camp or open the modem endpoint.
    /// Opens nothing and does not start a second camp.
    Lifecycle {
        #[arg(long, default_value = DEFAULT_MODEM_STATE)]
        modem_state: PathBuf,
        #[arg(long, default_value = "/proc")]
        proc: PathBuf,
    },
    /// Decide whether one reviewed GET may run. This command does not open
    /// the modem endpoint. A running owner, a held endpoint, or the status tool is busy.
    Query {
        #[arg(value_parser = ["sim-status", "radio-state", "data-registration"])]
        name: String,
        #[arg(long, default_value = DEFAULT_MODEM_STATE)]
        modem_state: PathBuf,
        #[arg(long, default_value = "/proc")]
        proc: PathBuf,
    },
    /// Replay eight reviewed headers and print the channel for each. Opens nothing.
    RfsDispatch,
    /// Say how one RFS command would be answered. Opens nothing.
    RfsPolicy {
        /// `nv` uses `--file`. `carrier-config` is the read-only copy path.
        #[arg(long, default_value = "nv", value_parser = ["nv", "carrier-config"])]
        channel: String,
        #[arg(long)]
        file: Option<u32>,
        #[arg(long)]
        cmd: u16,
        /// Carrier-config command 6: 1 reads the copy, 2 is a write and is denied.
        #[arg(long)]
        op: Option<u32>,
    },
    /// Stay up for this boot. Start the existing camp handoff only when the
    /// CP is OFFLINE (or not loaded) and the owner is not already running.
    /// A second launch in the same process never happens.
    Supervise {
        #[arg(long, default_value = DEFAULT_MODEM_STATE)]
        modem_state: PathBuf,
        #[arg(long, default_value = "/data/saaios/bin/owner-handoff-rfs-oemipc.sh")]
        handoff: PathBuf,
        #[arg(long, default_value = "/proc")]
        proc: PathBuf,
    },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("saai-modemd: {error:#}");
        std::process::exit(1);
    }
}

fn print_channel(decision: rfs_policy::ChannelDecision) {
    match decision {
        rfs_policy::ChannelDecision::Carrier(rfs_policy::CarrierDecision::Deny)
        | rfs_policy::ChannelDecision::Nv(rfs_policy::RfsDecision::Deny) => {
            println!("rfs=deny");
        }
        rfs_policy::ChannelDecision::Carrier(other) => {
            println!("rfs={} file=carrier-config", rfs_policy::carrier_word(other));
        }
        rfs_policy::ChannelDecision::Nv(other) => {
            let file_name = match other {
                rfs_policy::RfsDecision::OpenCopy(file)
                | rfs_policy::RfsDecision::QuarantineWrite(file)
                | rfs_policy::RfsDecision::QuarantineStatus(file) => rfs_policy::file_word(file),
                rfs_policy::RfsDecision::Deny => unreachable!(),
            };
            println!("rfs={} file={file_name}", rfs_policy::decision_word(other));
        }
    }
}

fn run() -> Result<()> {
    match Args::parse().command {
        Cmd::Status {
            modem_state,
            net_class,
        } => {
            let state = read_trimmed_optional(&modem_state).unwrap_or_else(|| "missing".into());
            let bearers = cellular_ifaces(&net_class)?;
            let live = live_bearers(&net_class, &bearers);
            let epoch = supervise::latest_boot_epoch(Path::new(DEFAULT_BOOT_ARCHIVE));
            let owner_running = owner_is_running(Path::new("/proc")).unwrap_or(false);
            let boot_log = read_log_tail(Path::new("/run/modem-boot.log"), 256 * 1024);
            let supervisor = boot_log
                .as_deref()
                .and_then(saai_observation::last_supervisor_token);
            let owner_log = fs::read_to_string(
                "/data/saaios/var/modem-rfs-camp-combined-owner.log",
            )
            .ok();
            let registration = camp_registration(&state, owner_running, owner_log.as_deref());
            let voice = camp_voice(&state, owner_running, owner_log.as_deref());
            let radio = camp_radio(&state, owner_running, owner_log.as_deref());
            let selection = camp_selection(&state, owner_running, owner_log.as_deref());
            let stack = camp_stack(&state, owner_running, owner_log.as_deref());
            let device_service = camp_device_service(&state, owner_running, owner_log.as_deref());
            let voice_operation = camp_voice_operation(&state, owner_running, owner_log.as_deref());
            let allow_data = camp_allow_data(&state, owner_running, owner_log.as_deref());
            let initial_attach = camp_initial_attach(&state, owner_running, owner_log.as_deref());
            let dns = camp_dns(&state, owner_running, owner_log.as_deref());
            let dns6 = camp_dns6(&state, owner_running, owner_log.as_deref());
            let config = camp_modem_config(&state, owner_running, owner_log.as_deref());
            let sgc = camp_sgc(&state, owner_running, owner_log.as_deref());
            let power = camp_radio_power(&state, owner_running, owner_log.as_deref());
            let voice_set = camp_voice_set(&state, owner_running, owner_log.as_deref());
            let ipv4 = camp_ipv4(&state, owner_running, owner_log.as_deref());
            let ipv6 = camp_ipv6(&state, owner_running, owner_log.as_deref());
            let setup = camp_data_setup(&state, owner_running, owner_log.as_deref());
            let profile = camp_data_profile(&state, owner_running, owner_log.as_deref());
            let activity = camp_activity(&state, owner_running, owner_log.as_deref());
            let fastdorm = camp_fastdorm(&state, owner_running, owner_log.as_deref());
            let endc = camp_endc(&state, owner_running, owner_log.as_deref());
            let throttle = camp_throttle(&state, owner_running, owner_log.as_deref());
            let unsolff = camp_unsolff(&state, owner_running, owner_log.as_deref());
            let unsol = camp_unsol(&state, owner_running, owner_log.as_deref());
            let screen = camp_screen(&state, owner_running, owner_log.as_deref());
            let cellinfo = camp_cellinfo(&state, owner_running, owner_log.as_deref());
            let smsc = camp_smsc(&state, owner_running, owner_log.as_deref());
            let vonrget = camp_vonrget(&state, owner_running, owner_log.as_deref());
            let aptime = camp_aptime(&state, owner_running, owner_log.as_deref());
            let dbgtrace = camp_dbgtrace(&state, owner_running, owner_log.as_deref());
            let tty = camp_tty(&state, owner_running, owner_log.as_deref());
            let pssvc = camp_pssvc(&state, owner_running, owner_log.as_deref());
            let sim = camp_sim(&state, owner_running, owner_log.as_deref());
            for line in status_lines(
                &state,
                &bearers,
                &live,
                registration,
                voice,
                radio,
                selection,
                stack,
                device_service,
                voice_operation,
                allow_data,
                initial_attach,
                dns,
                dns6,
                config,
                sgc,
                power,
                voice_set,
                ipv4,
                ipv6,
                setup,
                profile,
                activity,
                fastdorm,
                endc,
                throttle,
                unsolff,
                unsol,
                screen,
                cellinfo,
                smsc,
                vonrget,
                aptime,
                dbgtrace,
                tty,
                pssvc,
                sim,
                epoch,
                endpoint_holder_at(Path::new("/proc")),
                supervisor,
                owner_running,
                process_argv0_ends_with(Path::new("/proc"), "/sit-sim-status").unwrap_or(false),
            ) {
                println!("{line}");
            }
        }
        Cmd::Preflight {
            modem_state,
            mounts,
            cpif_ko,
            b_modem_bin,
            expected_cpif_sha256,
            expected_b_modem_sha256,
        } => {
            let state = read_trimmed(&modem_state)?;
            if state != "OFFLINE" {
                return Err(anyhow!("requires modem_state OFFLINE, got {state}"));
            }
            if mounts_include_original_efs(&read_to_string_optional(&mounts)?) {
                return Err(anyhow!("original EFS appears mounted; refusing"));
            }
            if let Some(path) = cpif_ko {
                assert_sha256(&path, &expected_cpif_sha256).with_context(|| {
                    format!("cpif provenance check failed for {}", path.display())
                })?;
            }
            if let Some(path) = b_modem_bin {
                assert_sha256(&path, &expected_b_modem_sha256).with_context(|| {
                    format!("B modem image check failed for {}", path.display())
                })?;
            }
            println!("preflight=ok");
            println!("hardware_actions=none");
        }
        Cmd::InspectImage { image } => {
            let bytes = fs::read(&image).with_context(|| format!("reading {}", image.display()))?;
            let toc = boot_model::parse_toc(&bytes)?;
            boot_model::validate_reviewed_panther_layout(&toc, bytes.len())?;
            let plan = boot_model::reviewed_boot_plan(&toc, bytes.len())?;
            let actions = boot_model::plan_executor_actions(&plan)?;
            println!("image={}", image.display());
            println!("toc_entries={}", toc.len());
            println!("boot_plan_steps={}", plan.steps.len());
            println!("executor_actions={}", actions.len());
            for name in ["BOOT", "MAIN", "VSS", "APM", "NV_NORM", "NV_PROT"] {
                let entry = boot_model::find_stage(&toc, name)
                    .ok_or_else(|| anyhow!("missing stage {name}"))?;
                let plan = boot_model::stage_plan(entry)?;
                println!(
                    "stage={} idx={} size=0x{:x} crc_policy={:?} start=0x{:x} done=0x{:x}",
                    plan.name, plan.index, plan.size, plan.crc_policy, plan.start, plan.done
                );
            }
            println!("hardware_actions=none");
        }
        Cmd::SoftLock {
            app,
            pin1,
            present,
            from_text,
        } => {
            let snapshot = if let Some(path) = from_text {
                let text = fs::read_to_string(&path)
                    .with_context(|| format!("reading {}", path.display()))?;
                soft_lock::detect_from_status_text(&text).ok_or_else(|| {
                    anyhow!("no usable app=/pin1= fields in {}", path.display())
                })?
            } else {
                let app = app.ok_or_else(|| anyhow!("need --app or --from-text"))?;
                let pin1 = pin1.ok_or_else(|| anyhow!("need --pin1 or --from-text"))?;
                let present = match present.as_deref() {
                    None | Some("notin") | Some("notin_1_2_3") => None,
                    Some(s) => Some(
                        s.parse::<u8>()
                            .with_context(|| format!("bad --present {s}"))?,
                    ),
                };
                SoftLockSnapshot {
                    app_state: app,
                    pin1,
                    present,
                }
            };
            for line in soft_lock::advice_lines(snapshot) {
                println!("{line}");
            }
            if snapshot.is_modem06() {
                println!(
                    "hint_app={APP_STATE_PIN} hint_pin1={PIN1_ENABLED_VERIFIED}|{PIN1_DISABLED}"
                );
                println!("doc=docs/os/targets/panther/MODEM-BLOCKER.md");
                // Echo short forms for scripting.
                let _ = (TRAY_BEARER_CHASE_CMD, TRAY_BEARER_CHASE_ON_DEVICE);
            }
            println!("hardware_actions=none");
        }
        Cmd::PostEdge {
            app,
            pin1,
            present,
            from_text,
            apn,
            apn_file,
            allow_pin_verify,
            allow_setup_data_call,
            rmnet_rx,
            rmnet_tx,
            ipv4,
        } => {
            let snapshot = if let Some(path) = from_text {
                let text = fs::read_to_string(&path)
                    .with_context(|| format!("reading {}", path.display()))?;
                soft_lock::detect_from_status_text(&text).ok_or_else(|| {
                    anyhow!("no usable app=/pin1= fields in {}", path.display())
                })?
            } else {
                let app = app.ok_or_else(|| anyhow!("need --app or --from-text"))?;
                let pin1 = pin1.ok_or_else(|| anyhow!("need --pin1 or --from-text"))?;
                let present = match present.as_deref() {
                    None | Some("notin") | Some("notin_1_2_3") => None,
                    Some(s) => Some(
                        s.parse::<u8>()
                            .with_context(|| format!("bad --present {s}"))?,
                    ),
                };
                SoftLockSnapshot {
                    app_state: app,
                    pin1,
                    present,
                }
            };
            let apn_from_file = match apn_file {
                Some(path) => {
                    let text = fs::read_to_string(&path)
                        .with_context(|| format!("reading {}", path.display()))?;
                    post_edge::resolve_apn_from_text(&text)
                }
                None => None,
            };
            let apn_arg = apn.as_deref().filter(|s| post_edge::apn_is_usable(s));
            let apn_resolved = apn_arg.or(apn_from_file.as_deref());
            let plan = post_edge::plan_from_snapshot_with_opt_ins(
                snapshot,
                apn_resolved,
                post_edge::PostEdgeOptIns {
                    allow_pin_verify,
                    allow_setup_data_call,
                },
            );
            for line in post_edge::advice_lines(&plan) {
                println!("{line}");
            }
            if let (Some(rx), Some(tx)) = (rmnet_rx, rmnet_tx) {
                let ok = post_edge::bearer_verified(ipv4.as_deref(), rx, tx);
                println!(
                    "bearer_verified={}",
                    if ok { "yes" } else { "no" }
                );
            }
            println!("doc=docs/os/targets/panther/MODEM-BLOCKER.md");
            println!("hardware_actions=none");
        }
        Cmd::Lifecycle { modem_state, proc } => {
            let state = read_trimmed_optional(&modem_state).unwrap_or_else(|| "missing".into());
            let owner_running = owner_is_running(&proc).unwrap_or(false);
            let lock_busy = process_argv0_ends_with(&proc, "/sit-sim-status").unwrap_or(false);
            let holder = endpoint_holder_at(&proc);
            for line in lifecycle_lines(&state, owner_running, lock_busy, holder) {
                println!("{line}");
            }
        }
        Cmd::Query {
            name,
            modem_state,
            proc,
        } => {
            let Some(query) = runtime_model::runtime_query_from_name(&name) else {
                return Err(anyhow!("unknown query {name}"));
            };
            let cp = read_trimmed_optional(&modem_state);
            let owner_running = owner_is_running(&proc)?;
            let status_lock_busy = process_argv0_ends_with(&proc, "/sit-sim-status")?;
            let holder = endpoint_holder_at(&proc);
            let admission = runtime_model::query_admission(
                cp.as_deref(),
                owner_running,
                status_lock_busy,
                holder,
            );
            let epoch = supervise::latest_boot_epoch(std::path::Path::new(DEFAULT_BOOT_ARCHIVE));
            let epoch_suffix = match epoch {
                Some(epoch) => format!(" epoch={epoch}"),
                None => String::new(),
            };
            let endpoint_suffix = endpoint_suffix(holder);
            let mut hardware = "none";
            match admission {
                QueryAdmission::Refuse(reason) => {
                    println!(
                        "query=refuse name={name} reason={reason} token={}{epoch_suffix}{endpoint_suffix}",
                        runtime_model::query_token(query)
                    );
                }
                QueryAdmission::Ready => {
                    let again = runtime_model::query_admission(
                        cp.as_deref(),
                        owner_is_running(&proc)?,
                        process_argv0_ends_with(&proc, "/sit-sim-status")?,
                        endpoint_holder_at(&proc),
                    );
                    match again {
                        QueryAdmission::Refuse(reason) => {
                            println!(
                                "query=refuse name={name} reason={reason} token={}{epoch_suffix}{endpoint_suffix}",
                                runtime_model::query_token(query)
                            );
                        }
                        QueryAdmission::Ready => {
                            hardware = "query";
                            match exchange_one_query(query) {
                                Ok(QueryExchange::Answer(line)) => {
                                    println!("query=answer name={name} {line}{epoch_suffix}{endpoint_suffix}");
                                }
                                Ok(QueryExchange::Timeout { frames }) => {
                                    println!(
                                        "query=timeout name={name} token={} deadline_s={} frames={frames}{epoch_suffix}{endpoint_suffix}",
                                        runtime_model::query_token(query),
                                        runtime_model::QUERY_DEADLINE_SECS
                                    );
                                }
                                Err(error) => return Err(error),
                            }
                        }
                    }
                }
            }
            println!("hardware_actions={hardware}");
        }
        Cmd::RfsDispatch => {
            for decision in rfs_policy::reviewed_dispatch() {
                print_channel(decision);
            }
            println!("hardware_actions=none");
        }
        Cmd::RfsPolicy {
            channel,
            file,
            cmd,
            op,
        } => {
            if channel == "carrier-config" {
                let decision = rfs_policy::decide_carrier_config(cmd, op);
                if decision == rfs_policy::CarrierDecision::Deny {
                    println!("rfs=deny");
                } else {
                    println!(
                        "rfs={} file=carrier-config",
                        rfs_policy::carrier_word(decision)
                    );
                }
            } else {
                let Some(file) = file else {
                    return Err(anyhow!("nv channel needs --file"));
                };
                let decision = rfs_policy::decide_rfs(file, cmd);
                match decision {
                    rfs_policy::RfsDecision::Deny => {
                        println!("rfs=deny");
                    }
                    other => {
                        let file_name = match other {
                            rfs_policy::RfsDecision::OpenCopy(file)
                            | rfs_policy::RfsDecision::QuarantineWrite(file)
                            | rfs_policy::RfsDecision::QuarantineStatus(file) => {
                                rfs_policy::file_word(file)
                            }
                            rfs_policy::RfsDecision::Deny => unreachable!(),
                        };
                        println!(
                            "rfs={} file={file_name}",
                            rfs_policy::decision_word(other)
                        );
                    }
                }
            }
            println!("hardware_actions=none");
        }
        Cmd::Supervise {
            modem_state,
            handoff,
            proc,
        } => supervise_camp(&modem_state, &handoff, &proc)?,
    }
    Ok(())
}

enum QueryExchange {
    Answer(String),
    Timeout { frames: u32 },
}

fn exchange_one_query(query: runtime_model::RuntimeQuery) -> Result<QueryExchange> {
    use std::io::{Read, Write};
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt};
    use std::time::{Duration, Instant};

    let identity = fs::read_to_string("/sys/class/cpif/umts_ipc0/dev")
        .context("umts_ipc0 identity")?;
    let mut parts = identity.trim().split(':');
    let expect_major: u64 = parts
        .next()
        .unwrap_or("")
        .parse()
        .context("umts_ipc0 major")?;
    let expect_minor: u64 = parts
        .next()
        .unwrap_or("")
        .parse()
        .context("umts_ipc0 minor")?;

    let mut lock_opts = fs::OpenOptions::new();
    lock_opts.read(true).write(true).create(true).mode(0o600);
    let lock = lock_opts
        .open("/run/saaios-sit-status.lock")
        .context("SIT lock")?;
    let locked = unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if locked != 0 {
        return Err(anyhow!("SIT lock busy"));
    }

    let mut dev_opts = fs::OpenOptions::new();
    dev_opts
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC | libc::O_NOFOLLOW);
    let mut dev = dev_opts.open("/dev/umts_ipc0").context("umts_ipc0 open")?;
    let meta = dev.metadata().context("umts_ipc0 stat")?;
    let major = libc::major(meta.rdev()) as u64;
    let minor = libc::minor(meta.rdev()) as u64;
    if !meta.file_type().is_char_device() || major != expect_major || minor != expect_minor {
        return Err(anyhow!("umts_ipc0 identity mismatch"));
    }

    let request = runtime_model::build_runtime_request(query);
    dev.write_all(&request.bytes).context("one-shot write")?;

    let deadline = Instant::now() + Duration::from_secs(runtime_model::QUERY_DEADLINE_SECS);
    let mut reader = runtime_model::RuntimeFrameReader::default();
    let mut tmp = [0u8; 4096];
    let mut frames = 0u32;
    while Instant::now() < deadline && frames < 128 {
        let mut pfd = libc::pollfd {
            fd: dev.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let wait = deadline
            .saturating_duration_since(Instant::now())
            .as_millis()
            .min(500) as i32;
        let ready = unsafe { libc::poll(&mut pfd, 1, wait) };
        if ready < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(err).context("SIT poll");
        }
        if ready == 0 {
            continue;
        }
        if pfd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
            return Err(anyhow!("SIT poll event"));
        }
        let n = match dev.read(&mut tmp) {
            Ok(0) => return Err(anyhow!("SIT EOF")),
            Ok(n) => n,
            Err(err)
                if err.kind() == std::io::ErrorKind::Interrupted
                    || err.kind() == std::io::ErrorKind::WouldBlock =>
            {
                continue;
            }
            Err(err) => return Err(err).context("SIT read"),
        };
        let parsed = reader.push(&tmp[..n]).context("runtime frame")?;
        for frame in parsed {
            frames += 1;
            if let Some(observation) = runtime_model::parse_matching_response(query, &frame) {
                return Ok(QueryExchange::Answer(runtime_model::report_line(
                    query,
                    &observation,
                )));
            }
        }
    }
    Ok(QueryExchange::Timeout { frames })
}

fn supervise_camp(modem_state: &Path, handoff: &Path, proc_root: &Path) -> Result<()> {
    let cp = read_trimmed_optional(modem_state);
    let owner_running = owner_is_running(proc_root)?;
    let action = supervise::first_action(cp.as_deref(), owner_running);
    match action {
        FirstAction::Attend => {
            println!(
                "supervise=attend cp={} owner={}",
                cp.as_deref().unwrap_or("missing"),
                if owner_running { "yes" } else { "no" }
            );
        }
        FirstAction::LaunchOnce => {
            println!(
                "supervise=launch-once cp={}",
                cp.as_deref().unwrap_or("missing")
            );
            let mut child = std::process::Command::new(handoff)
                .arg("rfs-camp-combined")
                .spawn()
                .with_context(|| format!("starting {}", handoff.display()))?;
            let status = child.wait().context("waiting for the camp handoff")?;
            println!("supervise=handoff-exit code={}", status.code().unwrap_or(-1));
        }
    }
    println!("supervise=hold");
    let cp_now = read_trimmed_optional(modem_state);
    let owner_now = owner_is_running(proc_root).unwrap_or(owner_running);
    let mut watch = supervise::HoldWatch::start(cp_now.as_deref(), owner_now);
    loop {
        std::thread::sleep(std::time::Duration::from_secs(30));
        let running = owner_is_running(proc_root).unwrap_or(false);
        let cp = read_trimmed_optional(modem_state);
        for note in watch.poll(cp.as_deref(), running) {
            match note {
                HoldNote::OwnerGone => println!("supervise=owner-gone"),
                HoldNote::CpLeft => println!("supervise=cp-left"),
            }
        }
    }
}

fn owner_is_running(proc_root: &Path) -> Result<bool> {
    process_argv0_ends_with(proc_root, "/modem-rfs-camp-combined-owner")
}

fn process_argv0_ends_with(proc_root: &Path, suffix: &str) -> Result<bool> {
    let Ok(entries) = fs::read_dir(proc_root) else {
        return Ok(false);
    };
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        let cmdline = fs::read(entry.path().join("cmdline")).unwrap_or_default();
        let argv0 = cmdline.split(|byte| *byte == 0).next().unwrap_or(b"");
        let text = String::from_utf8_lossy(argv0);
        if text.ends_with(suffix) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn read_trimmed(path: &Path) -> Result<String> {
    read_trimmed_optional(path).ok_or_else(|| anyhow!("reading {}", path.display()))
}

fn read_trimmed_optional(path: &Path) -> Option<String> {
    let value = fs::read_to_string(path).ok()?;
    Some(value.trim().to_string())
}

fn read_to_string_optional(path: &Path) -> Result<String> {
    Ok(fs::read_to_string(path).unwrap_or_default())
}

fn cellular_iface_name(name: &str) -> bool {
    let stem = name
        .split(|c: char| c == '.' || c == '@')
        .next()
        .unwrap_or(name);
    stem.starts_with("rmnet")
        || stem.starts_with("wwan")
        || stem.starts_with("qmimux")
        || stem.starts_with("ccmni")
}

fn cellular_ifaces(net_class: &Path) -> Result<Vec<String>> {
    let mut names = Vec::new();
    let Ok(entries) = fs::read_dir(net_class) else {
        return Ok(names);
    };
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if cellular_iface_name(&name) {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

fn mounts_include_original_efs(mounts: &str) -> bool {
    mounts.lines().any(|line| {
        let mut fields = line.split_whitespace();
        let source = fields.next().unwrap_or("");
        let target = fields.next().unwrap_or("");
        target == "/efs"
            || target == "/mnt/vendor/efs"
            || target.contains("/efs/")
            || source.contains("efs")
    })
}

fn live_bearers(net_class: &Path, names: &[String]) -> Vec<String> {
    names
        .iter()
        .filter(|name| {
            saai_observation::bearer_is_live(
                iface_has_ipv4(name),
                iface_counter(net_class, name, "rx_bytes"),
                iface_counter(net_class, name, "tx_bytes"),
            )
        })
        .cloned()
        .collect()
}

fn iface_counter(net_class: &Path, name: &str, which: &str) -> u64 {
    if !iface_name_ok(name) {
        return 0;
    }
    fs::read_to_string(net_class.join(name).join("statistics").join(which))
        .ok()
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(0)
}

fn iface_has_ipv4(name: &str) -> bool {
    if !iface_name_ok(name) {
        return false;
    }
    let Ok(out) = std::process::Command::new("/saaios/ip")
        .args(["-4", "-o", "addr", "show", "dev", name])
        .output()
    else {
        return false;
    };
    out.status.success() && out.stdout.windows(5).any(|window| window == b"inet ")
}

fn read_log_tail(path: &Path, max: u64) -> Option<String> {
    let mut file = fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    if len > max {
        use std::io::Seek;
        file.seek(std::io::SeekFrom::End(-(max as i64))).ok()?;
    }
    let mut text = String::new();
    use std::io::Read;
    file.read_to_string(&mut text).ok()?;
    Some(text)
}

fn iface_name_ok(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('/')
        && !name.contains('.')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn status_lines(
    state: &str,
    bearers: &[String],
    live: &[String],
    registration: Option<u32>,
    voice: Option<u32>,
    radio: Option<&str>,
    selection: Option<&str>,
    stack: Option<&str>,
    device_service: Option<&str>,
    voice_operation: Option<&str>,
    allow_data: Option<&str>,
    initial_attach: Option<&str>,
    dns: Option<&str>,
    dns6: Option<&str>,
    config: Option<&str>,
    sgc: Option<&str>,
    power: Option<&str>,
    voice_set: Option<&str>,
    ipv4: Option<&str>,
    ipv6: Option<&str>,
    setup: Option<&str>,
    profile: Option<&str>,
    activity: Option<&str>,
    fastdorm: Option<&str>,
    endc: Option<&str>,
    throttle: Option<&str>,
    unsolff: Option<&str>,
    unsol: Option<&str>,
    screen: Option<&str>,
    cellinfo: Option<&str>,
    smsc: Option<&str>,
    vonrget: Option<&str>,
    aptime: Option<&str>,
    dbgtrace: Option<&str>,
    tty: Option<&str>,
    pssvc: Option<&str>,
    sim: Option<&str>,
    epoch: Option<u64>,
    holder: Option<&str>,
    supervisor: Option<&str>,
    owner_running: bool,
    lock_busy: bool,
) -> Vec<String> {
    let mut lines = vec![format!("modem_state={state}")];
    if bearers.is_empty() {
        lines.push("cellular_bearers=none".to_string());
    } else {
        lines.push(format!("cellular_bearers={}", bearers.join(",")));
    }
    if !live.is_empty() {
        lines.push(format!("bearer={}", live.join(",")));
    }
    if let Some(raw) = registration {
        lines.push(format!("registration_raw={raw}"));
    }
    if let Some(raw) = voice {
        lines.push(format!("voice_registration_raw={raw}"));
    }
    if radio == Some("on") {
        lines.push("radio=on".to_string());
    }
    if let Some(token) = selection_word(selection) {
        lines.push(format!("selection={token}"));
    }
    if let Some(token) = stack_word(stack) {
        lines.push(format!("stack={token}"));
    }
    if let Some(token) = device_service_word(device_service) {
        lines.push(format!("device_service={token}"));
    }
    if voice_operation == Some("enabled") {
        lines.push("voice_operation=enabled".to_string());
    }
    if allow_data == Some("accepted") {
        lines.push("allow_data=accepted".to_string());
    }
    if initial_attach == Some("accepted") {
        lines.push("initial_attach=accepted".to_string());
    }
    if let Some(token) = dns_word(dns) {
        lines.push(format!("dns={token}"));
    }
    if let Some(token) = dns_word(dns6) {
        lines.push(format!("dns6={token}"));
    }
    if config == Some("accepted") {
        lines.push("config=accepted".to_string());
    }
    if sgc == Some("accepted") {
        lines.push("sgc=accepted".to_string());
    }
    if power == Some("accepted") {
        lines.push("power=accepted".to_string());
    }
    if voice_set == Some("accepted") {
        lines.push("voice_set=accepted".to_string());
    }
    if let Some(token) = dns_word(ipv4) {
        lines.push(format!("ipv4={token}"));
    }
    if let Some(token) = dns_word(ipv6) {
        lines.push(format!("ipv6={token}"));
    }
    if setup == Some("accepted") {
        lines.push("setup=accepted".to_string());
    }
    if profile == Some("accepted") {
        lines.push("profile=accepted".to_string());
    }
    if activity == Some("accepted") {
        lines.push("activity=accepted".to_string());
    }
    if fastdorm == Some("accepted") {
        lines.push("fastdorm=accepted".to_string());
    }
    if endc == Some("accepted") {
        lines.push("endc=accepted".to_string());
    }
    if throttle == Some("accepted") {
        lines.push("throttle=accepted".to_string());
    }
    if unsolff == Some("accepted") {
        lines.push("unsolff=accepted".to_string());
    }
    if unsol == Some("accepted") {
        lines.push("unsol=accepted".to_string());
    }
    if screen == Some("accepted") {
        lines.push("screen=accepted".to_string());
    }
    if cellinfo == Some("accepted") {
        lines.push("cellinfo=accepted".to_string());
    }
    if smsc == Some("accepted") {
        lines.push("smsc=accepted".to_string());
    }
    if vonrget == Some("accepted") {
        lines.push("vonrget=accepted".to_string());
    }
    if aptime == Some("accepted") {
        lines.push("aptime=accepted".to_string());
    }
    if dbgtrace == Some("accepted") {
        lines.push("dbgtrace=accepted".to_string());
    }
    if tty == Some("accepted") {
        lines.push("tty=accepted".to_string());
    }
    if pssvc == Some("accepted") {
        lines.push("pssvc=accepted".to_string());
    }
    if let Some(token) = sim_word(sim) {
        lines.push(format!("sim={token}"));
    }
    if let Some(epoch) = epoch {
        lines.push(format!("epoch={epoch}"));
    }
    if let Some(holder) = endpoint_suffix(holder).strip_prefix(' ') {
        lines.push(holder.to_string());
    }
    if let Some(token) = supervisor_word(supervisor) {
        lines.push(format!("supervisor={token}"));
    }
    lines.push(format!(
        "owner={}",
        if owner_running { "running" } else { "gone" }
    ));
    let cp = if state == "missing" { None } else { Some(state) };
    lines.push(format!(
        "action={}",
        saai_observation::camp_action(cp, owner_running)
    ));
    let open = saai_observation::camp_open(cp, owner_running, holder, lock_busy);
    lines.push(if open == "ready" {
        "open=ready".to_string()
    } else {
        format!("open=refuse reason={open}")
    });
    lines.push("hardware_actions=none".to_string());
    lines
}

fn lifecycle_lines(
    state: &str,
    owner_running: bool,
    lock_busy: bool,
    holder: Option<&str>,
) -> Vec<String> {
    let cp = if state == "missing" { None } else { Some(state) };
    let action = match supervise::first_action(cp, owner_running) {
        FirstAction::LaunchOnce => "launch-once",
        FirstAction::Attend => "attend",
    };
    let open = match runtime_model::query_admission(cp, owner_running, lock_busy, holder) {
        QueryAdmission::Ready => "open=ready".to_string(),
        QueryAdmission::Refuse(reason) => format!("open=refuse reason={reason}"),
    };
    let mut lines = vec![format!("action={action}"), open];
    if let Some(holder) = endpoint_suffix(holder).strip_prefix(' ') {
        lines.push(holder.to_string());
    }
    lines.push(format!(
        "owner={}",
        if owner_running { "running" } else { "gone" }
    ));
    lines.push(format!("cp={state}"));
    lines.push("hardware_actions=none".to_string());
    lines
}

fn camp_sim(state: &str, owner_running: bool, log: Option<&str>) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_sim_presence(&facts)
}

fn sim_word(token: Option<&str>) -> Option<&str> {
    match token {
        Some(token @ ("ready" | "present" | "absent")) => Some(token),
        _ => None,
    }
}

fn camp_radio(state: &str, owner_running: bool, log: Option<&str>) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_radio_token(&facts)
}

fn camp_registration(state: &str, owner_running: bool, log: Option<&str>) -> Option<u32> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_data_registration_raw(&facts)
}

fn dns_word(token: Option<&str>) -> Option<&str> {
    match token {
        Some("yes") | Some("no") => token,
        _ => None,
    }
}

fn camp_pssvc(state: &str, owner_running: bool, log: Option<&str>) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_pssvc(&facts)
}

fn camp_tty(state: &str, owner_running: bool, log: Option<&str>) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_tty(&facts)
}

fn camp_dbgtrace(state: &str, owner_running: bool, log: Option<&str>) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_dbgtrace(&facts)
}

fn camp_aptime(state: &str, owner_running: bool, log: Option<&str>) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_aptime(&facts)
}

fn camp_vonrget(state: &str, owner_running: bool, log: Option<&str>) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_vonrget(&facts)
}

fn camp_smsc(state: &str, owner_running: bool, log: Option<&str>) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_smsc(&facts)
}

fn camp_cellinfo(state: &str, owner_running: bool, log: Option<&str>) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_cellinfo(&facts)
}

fn camp_screen(state: &str, owner_running: bool, log: Option<&str>) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_screen(&facts)
}

fn camp_unsol(state: &str, owner_running: bool, log: Option<&str>) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_unsol(&facts)
}

fn camp_unsolff(state: &str, owner_running: bool, log: Option<&str>) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_unsolff(&facts)
}

fn camp_throttle(state: &str, owner_running: bool, log: Option<&str>) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_throttle(&facts)
}

fn camp_endc(state: &str, owner_running: bool, log: Option<&str>) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_endc(&facts)
}

fn camp_fastdorm(state: &str, owner_running: bool, log: Option<&str>) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_fastdorm(&facts)
}

fn camp_activity(state: &str, owner_running: bool, log: Option<&str>) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_activity(&facts)
}

fn camp_data_profile(state: &str, owner_running: bool, log: Option<&str>) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_data_profile(&facts)
}

fn camp_data_setup(state: &str, owner_running: bool, log: Option<&str>) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_data_setup(&facts)
}

fn camp_ipv6(state: &str, owner_running: bool, log: Option<&str>) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_ipv6(&facts)
}

fn camp_ipv4(state: &str, owner_running: bool, log: Option<&str>) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_ipv4(&facts)
}

fn camp_voice_set(
    state: &str,
    owner_running: bool,
    log: Option<&str>,
) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_voice_set(&facts)
}

fn camp_radio_power(
    state: &str,
    owner_running: bool,
    log: Option<&str>,
) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_radio_power(&facts)
}

fn camp_sgc(state: &str, owner_running: bool, log: Option<&str>) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_sgc(&facts)
}

fn camp_modem_config(
    state: &str,
    owner_running: bool,
    log: Option<&str>,
) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_modem_config(&facts)
}

fn camp_dns6(state: &str, owner_running: bool, log: Option<&str>) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_dns6(&facts)
}

fn camp_dns(state: &str, owner_running: bool, log: Option<&str>) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_dns(&facts)
}

fn camp_initial_attach(
    state: &str,
    owner_running: bool,
    log: Option<&str>,
) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_initial_attach(&facts)
}

fn camp_allow_data(
    state: &str,
    owner_running: bool,
    log: Option<&str>,
) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_allow_data(&facts)
}

fn camp_voice_operation(
    state: &str,
    owner_running: bool,
    log: Option<&str>,
) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_voice_operation(&facts)
}

fn device_service_word(token: Option<&str>) -> Option<&str> {
    match token {
        Some("voice-centric" | "data-centric") => token,
        _ => None,
    }
}

fn camp_device_service(
    state: &str,
    owner_running: bool,
    log: Option<&str>,
) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_device_service(&facts)
}

fn stack_word(token: Option<&str>) -> Option<&str> {
    match token {
        Some("enabled" | "disabled") => token,
        _ => None,
    }
}

fn camp_stack(state: &str, owner_running: bool, log: Option<&str>) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_stack_mode(&facts)
}

fn selection_word(token: Option<&str>) -> Option<&str> {
    match token {
        Some("automatic" | "manual") => token,
        _ => None,
    }
}

fn camp_selection(state: &str, owner_running: bool, log: Option<&str>) -> Option<&'static str> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_selection_mode(&facts)
}

fn camp_voice(state: &str, owner_running: bool, log: Option<&str>) -> Option<u32> {
    if !owner_running || state.trim() != "ONLINE" {
        return None;
    }
    let facts = saai_observation::owner_fact_lines(log?);
    saai_observation::last_voice_registration_raw(&facts)
}

fn supervisor_word(token: Option<&str>) -> Option<&str> {
    match token {
        Some(token @ ("hold" | "owner-gone" | "cp-left" | "attend" | "launch-once")) => Some(token),
        _ => None,
    }
}

fn endpoint_suffix(holder: Option<&str>) -> String {
    match holder {
        Some(holder @ ("owner" | "modemd" | "shared")) => format!(" endpoint={holder}"),
        _ => String::new(),
    }
}

/// Who holds `umts_ipc0` or `umts_rfs0`, from directory symlinks. The
/// devices themselves stay closed.
fn endpoint_holder_at(proc_root: &Path) -> Option<&'static str> {
    let Ok(entries) = fs::read_dir(proc_root) else {
        return None;
    };
    let mut owner_has = false;
    let mut modemd_has = false;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        let cmdline = fs::read(entry.path().join("cmdline")).unwrap_or_default();
        let argv0 = cmdline.split(|byte| *byte == 0).next().unwrap_or(b"");
        let argv0 = String::from_utf8_lossy(argv0);
        let kind = if argv0.ends_with("/modem-rfs-camp-combined-owner") {
            "owner"
        } else if argv0.ends_with("/saai-modemd") {
            "modemd"
        } else {
            continue;
        };
        let Ok(fds) = fs::read_dir(entry.path().join("fd")) else {
            continue;
        };
        let holds = fds.flatten().any(|fd| {
            fs::read_link(fd.path())
                .ok()
                .and_then(|target| target.to_str().map(str::to_string))
                .is_some_and(|target| saai_observation::link_is_modem_endpoint(&target))
        });
        if holds {
            match kind {
                "owner" => owner_has = true,
                _ => modemd_has = true,
            }
        }
    }
    saai_observation::endpoint_holder(owner_has, modemd_has)
}

fn assert_sha256(path: &Path, expected_hex: &str) -> Result<()> {
    let expected = hex::decode(expected_hex).context("expected digest is not hex")?;
    let data = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let actual = Sha256::digest(&data);
    if actual.as_slice() != expected.as_slice() {
        return Err(anyhow!(
            "sha256 mismatch: expected {expected_hex}, got {}",
            hex::encode(actual)
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_attends_a_live_camp_and_refuses_to_open() {
        assert_eq!(
            lifecycle_lines("ONLINE", true, false, Some("owner")),
            vec![
                "action=attend".to_string(),
                "open=refuse reason=owner".to_string(),
                "endpoint=owner".to_string(),
                "owner=running".to_string(),
                "cp=ONLINE".to_string(),
                "hardware_actions=none".to_string(),
            ]
        );
        assert_eq!(
            lifecycle_lines("OFFLINE", false, false, None),
            vec![
                "action=launch-once".to_string(),
                "open=refuse reason=cp".to_string(),
                "owner=gone".to_string(),
                "cp=OFFLINE".to_string(),
                "hardware_actions=none".to_string(),
            ]
        );
        assert_eq!(
            lifecycle_lines("ONLINE", false, false, Some("modemd")),
            vec![
                "action=attend".to_string(),
                "open=refuse reason=endpoint".to_string(),
                "endpoint=modemd".to_string(),
                "owner=gone".to_string(),
                "cp=ONLINE".to_string(),
                "hardware_actions=none".to_string(),
            ]
        );
        let ready = lifecycle_lines("ONLINE", false, false, None);
        assert_eq!(ready[0], "action=attend");
        assert_eq!(ready[1], "open=ready");
        assert_eq!(ready.last().map(String::as_str), Some("hardware_actions=none"));
        let missing = lifecycle_lines("missing", false, false, None);
        assert_eq!(missing[0], "action=launch-once");
        assert_eq!(missing[1], "open=refuse reason=cp");
    }

    #[test]
    fn status_names_the_endpoint_holder_without_a_hardware_action() {
        let lines = status_lines(
            "ONLINE",
            &["rmnet0".into(), "rmnet1".into()],
            &["rmnet1".into()],
            Some(1),
            Some(1),
            Some("on"),
            Some("automatic"),
            Some("enabled"),
            Some("voice-centric"),
            Some("enabled"),
            Some("accepted"),
            Some("accepted"),
            Some("yes"),
            Some("yes"),
            Some("accepted"),
            Some("accepted"),
            Some("accepted"),
            Some("accepted"),
            Some("yes"),
            Some("yes"),
            Some("accepted"),
            Some("accepted"),
            Some("accepted"),
            Some("accepted"),
            Some("accepted"),
            Some("accepted"),
            Some("accepted"),
            Some("accepted"),
            Some("accepted"),
            Some("accepted"),
            Some("accepted"),
            Some("accepted"),
            Some("accepted"),
            Some("accepted"),
            Some("accepted"),
            Some("accepted"),
            Some("present"),
            Some(250),
            Some("owner"),
            Some("hold"),
            true,
            false,
        );
        assert_eq!(
            lines,
            vec![
                "modem_state=ONLINE".to_string(),
                "cellular_bearers=rmnet0,rmnet1".to_string(),
                "bearer=rmnet1".to_string(),
                "registration_raw=1".to_string(),
                "voice_registration_raw=1".to_string(),
                "radio=on".to_string(),
                "selection=automatic".to_string(),
                "stack=enabled".to_string(),
                "device_service=voice-centric".to_string(),
                "voice_operation=enabled".to_string(),
                "allow_data=accepted".to_string(),
                "initial_attach=accepted".to_string(),
                "dns=yes".to_string(),
                "dns6=yes".to_string(),
                "config=accepted".to_string(),
                "sgc=accepted".to_string(),
                "power=accepted".to_string(),
                "voice_set=accepted".to_string(),
                "ipv4=yes".to_string(),
                "ipv6=yes".to_string(),
                "setup=accepted".to_string(),
                "profile=accepted".to_string(),
                "activity=accepted".to_string(),
                "fastdorm=accepted".to_string(),
                "endc=accepted".to_string(),
                "throttle=accepted".to_string(),
                "unsolff=accepted".to_string(),
                "unsol=accepted".to_string(),
                "screen=accepted".to_string(),
                "cellinfo=accepted".to_string(),
                "smsc=accepted".to_string(),
                "vonrget=accepted".to_string(),
                "aptime=accepted".to_string(),
                "dbgtrace=accepted".to_string(),
                "tty=accepted".to_string(),
                "pssvc=accepted".to_string(),
                "sim=present".to_string(),
                "epoch=250".to_string(),
                "endpoint=owner".to_string(),
                "supervisor=hold".to_string(),
                "owner=running".to_string(),
                "action=attend".to_string(),
                "open=refuse reason=owner".to_string(),
                "hardware_actions=none".to_string(),
            ]
        );
        let absent = status_lines(
            "missing",
            &[],
            &[],
            None,
            None,
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin"),
            Some("pin1"),
            None,
            None,
            Some("handoff-exit"),
            false,
            true,
        );
        assert_eq!(
            absent,
            vec![
                "modem_state=missing".to_string(),
                "cellular_bearers=none".to_string(),
                "owner=gone".to_string(),
                "action=launch-once".to_string(),
                "open=refuse reason=lock".to_string(),
                "hardware_actions=none".to_string(),
            ]
        );
        assert!(!absent.iter().any(|line| line.contains("endpoint=")));
        assert!(!absent.iter().any(|line| line.contains("epoch=")));
        assert!(!absent.iter().any(|line| line.starts_with("bearer=")));
        assert!(!absent.iter().any(|line| line.starts_with("supervisor=")));
        assert!(!absent.iter().any(|line| line.starts_with("registration_raw=")));
        assert!(!absent.iter().any(|line| line.starts_with("voice_registration_raw=")));
        assert!(!absent.iter().any(|line| line.starts_with("radio=")));
        assert!(!absent.iter().any(|line| line.starts_with("selection=")));
        assert!(!absent.iter().any(|line| line.starts_with("stack=")));
        assert!(!absent.iter().any(|line| line.starts_with("device_service=")));
        assert!(!absent.iter().any(|line| line.starts_with("voice_operation=")));
        assert!(!absent.iter().any(|line| line.starts_with("allow_data=")));
        assert!(!absent.iter().any(|line| line.starts_with("initial_attach=")));
        assert!(!absent.iter().any(|line| line.starts_with("dns=")));
        assert!(!absent.iter().any(|line| line.starts_with("dns6=")));
        assert!(!absent.iter().any(|line| line.starts_with("config=")));
        assert!(!absent.iter().any(|line| line.starts_with("sgc=")));
        assert!(!absent.iter().any(|line| line.starts_with("power=")));
        assert!(!absent.iter().any(|line| line.starts_with("voice_set=")));
        assert!(!absent.iter().any(|line| line.starts_with("ipv4=")));
        assert!(!absent.iter().any(|line| line.starts_with("ipv6=")));
        assert!(!absent.iter().any(|line| line.starts_with("setup=")));
        assert!(!absent.iter().any(|line| line.starts_with("profile=")));
        assert!(!absent.iter().any(|line| line.starts_with("activity=")));
        assert!(!absent.iter().any(|line| line.starts_with("fastdorm=")));
        assert!(!absent.iter().any(|line| line.starts_with("endc=")));
        assert!(!absent.iter().any(|line| line.starts_with("throttle=")));
        assert!(!absent.iter().any(|line| line.starts_with("unsolff=")));
        assert!(!absent.iter().any(|line| line.starts_with("unsol=")));
        assert!(!absent.iter().any(|line| line.starts_with("screen=")));
        assert!(!absent.iter().any(|line| line.starts_with("cellinfo=")));
        assert!(!absent.iter().any(|line| line.starts_with("smsc=")));
        assert!(!absent.iter().any(|line| line.starts_with("vonrget=")));
        assert!(!absent.iter().any(|line| line.starts_with("aptime=")));
        assert!(!absent.iter().any(|line| line.starts_with("dbgtrace=")));
        assert!(!absent.iter().any(|line| line.starts_with("tty=")));
        assert!(!absent.iter().any(|line| line.starts_with("pssvc=")));
        assert!(!absent.iter().any(|line| line.starts_with("sim=")));
        assert!(!absent.iter().any(|line| line.contains("pin")));
    }

    #[test]
    fn camp_sim_names_presence_and_keeps_pin_out() {
        let later = "camp_sim=ready\nfield=sim apps=1 pin1_raw=1\n";
        assert_eq!(camp_sim("ONLINE", true, Some(later)), Some("present"));
        assert_eq!(camp_sim("OFFLINE", true, Some(later)), None);
        assert_eq!(camp_sim("ONLINE", false, Some(later)), None);
        assert_eq!(
            camp_sim("ONLINE", true, Some("field=sim status=unknown_short apps=1\n")),
            None
        );
        assert_eq!(
            camp_sim("ONLINE", true, Some("field=sim apps=0\n")),
            Some("absent")
        );
    }

    #[test]
    fn camp_radio_is_on_only_for_the_stock_raw_value() {
        let on = "camp_reg field=radio radio_raw=10\n";
        assert_eq!(camp_radio("ONLINE", true, Some(on)), Some("on"));
        assert_eq!(camp_radio("OFFLINE", true, Some(on)), None);
        assert_eq!(camp_radio("ONLINE", false, Some(on)), None);
        assert_eq!(
            camp_radio("ONLINE", true, Some("camp_reg field=radio radio_raw=1\n")),
            None
        );
    }

    #[test]
    fn camp_registration_needs_a_live_owner_and_an_online_cp() {
        let log = "camp_reg field=voice registration_raw=3\n\
                   camp_reg field=data registration_raw=1 reject_raw=0 lac=0\n";
        assert_eq!(camp_registration("ONLINE", true, Some(log)), Some(1));
        assert_eq!(camp_voice("ONLINE", true, Some(log)), Some(3));
        assert_eq!(
            camp_selection(
                "ONLINE",
                true,
                Some("field=selection mode_raw=0\nfield=selection mode_raw=9\n")
            ),
            None
        );
        assert_eq!(
            camp_selection("ONLINE", true, Some("field=selection mode_raw=0\n")),
            Some("automatic")
        );
        assert_eq!(camp_selection("OFFLINE", true, Some("field=selection mode_raw=0\n")), None);
        assert_eq!(
            camp_stack("ONLINE", true, Some("camp_opx get=stack_status mode_raw=1\n")),
            Some("enabled")
        );
        assert_eq!(
            camp_stack("ONLINE", false, Some("camp_opx get=stack_status mode_raw=1\n")),
            None
        );
        assert_eq!(
            camp_stack("OFFLINE", true, Some("camp_opx get=stack_status mode_raw=1\n")),
            None
        );
        assert_eq!(
            camp_device_service(
                "ONLINE",
                true,
                Some("camp_opx get=device_service mode_raw=1\n")
            ),
            Some("voice-centric")
        );
        assert_eq!(
            camp_device_service(
                "ONLINE",
                false,
                Some("camp_opx get=device_service mode_raw=2\n")
            ),
            None
        );
        assert_eq!(
            camp_voice_operation(
                "ONLINE",
                true,
                Some("camp_opx get=voice_operation mode_raw=3\n")
            ),
            Some("enabled")
        );
        assert_eq!(
            camp_voice_operation(
                "ONLINE",
                true,
                Some("camp_opx get=voice_operation mode_raw=1\n")
            ),
            None
        );
        assert_eq!(
            camp_voice_operation("OFFLINE", true, Some("camp_opx get=voice_operation mode_raw=3\n")),
            None
        );
        assert_eq!(
            camp_allow_data(
                "ONLINE",
                true,
                Some("camp_reg set=allow_data response=yes error_raw=0\n")
            ),
            Some("accepted")
        );
        assert_eq!(
            camp_allow_data(
                "ONLINE",
                true,
                Some("camp_reg set=allow_data response=yes error_raw=2\n")
            ),
            None
        );
        assert_eq!(
            camp_allow_data(
                "OFFLINE",
                true,
                Some("camp_reg set=allow_data response=yes error_raw=0\n")
            ),
            None
        );
        assert_eq!(
            camp_initial_attach(
                "ONLINE",
                true,
                Some("camp_reg set=initial_attach_apn response=yes error_raw=0\n")
            ),
            Some("accepted")
        );
        assert_eq!(
            camp_initial_attach(
                "ONLINE",
                true,
                Some("camp_reg set=initial_attach_apn response=yes error_raw=2\n")
            ),
            None
        );
        assert_eq!(
            camp_initial_attach(
                "OFFLINE",
                true,
                Some("camp_reg set=initial_attach_apn response=yes error_raw=0\n")
            ),
            None
        );
        assert_eq!(
            camp_dns("ONLINE", true, Some("camp_setup dns=yes count=2\n")),
            Some("yes")
        );
        assert_eq!(camp_dns("ONLINE", true, Some("camp_setup dns=no\n")), Some("no"));
        assert_eq!(camp_dns("ONLINE", true, Some("camp_setup dns6=yes count=2\n")), None);
        assert_eq!(
            camp_dns("OFFLINE", true, Some("camp_setup dns=yes count=2\n")),
            None
        );
        assert_eq!(
            camp_dns6("ONLINE", true, Some("camp_setup dns6=yes count=2\n")),
            Some("yes")
        );
        assert_eq!(camp_dns6("ONLINE", true, Some("camp_setup dns6=no\n")), Some("no"));
        assert_eq!(camp_dns6("ONLINE", true, Some("camp_setup dns=yes count=2\n")), None);
        assert_eq!(
            camp_dns6("OFFLINE", true, Some("camp_setup dns6=yes count=2\n")),
            None
        );
        assert_eq!(
            camp_modem_config(
                "ONLINE",
                true,
                Some("camp_ack cmd=0x093f response=yes error_raw=0\n")
            ),
            Some("accepted")
        );
        assert_eq!(
            camp_modem_config(
                "ONLINE",
                true,
                Some("camp_ack cmd=0x093f response=yes error_raw=2\n")
            ),
            None
        );
        assert_eq!(
            camp_modem_config(
                "OFFLINE",
                true,
                Some("camp_ack cmd=0x093f response=yes error_raw=0\n")
            ),
            None
        );
        assert_eq!(
            camp_sgc(
                "ONLINE",
                true,
                Some("camp_ack cmd=0x0404 response=yes error_raw=0\n")
            ),
            Some("accepted")
        );
        assert_eq!(
            camp_sgc(
                "ONLINE",
                true,
                Some("camp_ack cmd=0x0404 response=yes error_raw=2\n")
            ),
            None
        );
        assert_eq!(
            camp_sgc(
                "ONLINE",
                true,
                Some("camp_ack cmd=0x093f response=yes error_raw=0\n")
            ),
            None
        );
        assert_eq!(
            camp_sgc(
                "OFFLINE",
                true,
                Some("camp_ack cmd=0x0404 response=yes error_raw=0\n")
            ),
            None
        );
        assert_eq!(
            camp_radio_power(
                "ONLINE",
                true,
                Some("camp_ack cmd=0x0800 response=yes error_raw=0\n")
            ),
            Some("accepted")
        );
        assert_eq!(
            camp_radio_power(
                "ONLINE",
                true,
                Some("camp_ack cmd=0x0800 response=yes error_raw=2\n")
            ),
            None
        );
        assert_eq!(
            camp_radio_power(
                "ONLINE",
                true,
                Some("camp_ack cmd=0x0404 response=yes error_raw=0\n")
            ),
            None
        );
        assert_eq!(
            camp_radio_power(
                "OFFLINE",
                true,
                Some("camp_ack cmd=0x0800 response=yes error_raw=0\n")
            ),
            None
        );
        assert_eq!(
            camp_voice_set(
                "ONLINE",
                true,
                Some("camp_opx set=set_voice_operation response=yes error_raw=0\n")
            ),
            Some("accepted")
        );
        assert_eq!(
            camp_voice_set(
                "ONLINE",
                true,
                Some("camp_opx set=set_voice_operation response=yes error_raw=2\n")
            ),
            None
        );
        assert_eq!(
            camp_voice_set(
                "ONLINE",
                true,
                Some("camp_opx get=voice_operation mode_raw=3\n")
            ),
            None
        );
        assert_eq!(
            camp_voice_set(
                "OFFLINE",
                true,
                Some("camp_opx set=set_voice_operation response=yes error_raw=0\n")
            ),
            None
        );
        assert_eq!(
            camp_ipv4(
                "ONLINE",
                true,
                Some("camp_setup if=rmnet1 ipv4=yes prefix=32 up=1 add=1 route=1\n")
            ),
            Some("yes")
        );
        assert_eq!(
            camp_ipv4(
                "ONLINE",
                true,
                Some("camp_setup if=rmnet1 ipv4=yes prefix=32 up=1 add=0 route=1\n")
            ),
            Some("no")
        );
        assert_eq!(
            camp_ipv4(
                "ONLINE",
                true,
                Some("camp_setup if=rmnet1 ipv6=yes prefix=64 up=1 add=1 route=1\n")
            ),
            None
        );
        assert_eq!(
            camp_ipv4(
                "OFFLINE",
                true,
                Some("camp_setup if=rmnet1 ipv4=yes prefix=32 up=1 add=1 route=1\n")
            ),
            None
        );
        assert_eq!(
            camp_ipv6(
                "ONLINE",
                true,
                Some("camp_setup if=rmnet1 ipv6=yes prefix=64 up=1 add=1 route=1\n")
            ),
            Some("yes")
        );
        assert_eq!(
            camp_ipv6(
                "ONLINE",
                true,
                Some("camp_setup if=rmnet1 ipv6=yes prefix=64 up=1 add=0 route=1\n")
            ),
            Some("no")
        );
        assert_eq!(
            camp_ipv6(
                "ONLINE",
                true,
                Some("camp_setup if=rmnet1 ipv4=yes prefix=32 up=1 add=1 route=1\n")
            ),
            None
        );
        assert_eq!(
            camp_ipv6(
                "OFFLINE",
                true,
                Some("camp_setup if=rmnet1 ipv6=yes prefix=64 up=1 add=1 route=1\n")
            ),
            None
        );
        assert_eq!(
            camp_data_setup(
                "ONLINE",
                true,
                Some("camp_setup response=yes error_raw=0 len=100\n")
            ),
            Some("accepted")
        );
        assert_eq!(
            camp_data_setup(
                "ONLINE",
                true,
                Some("camp_setup response=yes error_raw=2 len=100\n")
            ),
            None
        );
        assert_eq!(
            camp_data_setup(
                "ONLINE",
                true,
                Some("camp_setup if=rmnet1 ipv4=yes prefix=32 up=1 add=1 route=1\n")
            ),
            None
        );
        assert_eq!(
            camp_data_setup(
                "OFFLINE",
                true,
                Some("camp_setup response=yes error_raw=0 len=100\n")
            ),
            None
        );
        assert_eq!(
            camp_data_profile(
                "ONLINE",
                true,
                Some("camp_profile response=yes error_raw=0 len=40\n")
            ),
            Some("accepted")
        );
        assert_eq!(
            camp_data_profile(
                "ONLINE",
                true,
                Some("camp_profile response=yes error_raw=2 len=40\n")
            ),
            None
        );
        assert_eq!(
            camp_data_profile(
                "ONLINE",
                true,
                Some("camp_setup response=yes error_raw=0 len=100\n")
            ),
            None
        );
        assert_eq!(
            camp_data_profile(
                "OFFLINE",
                true,
                Some("camp_profile response=yes error_raw=0 len=40\n")
            ),
            None
        );
        assert_eq!(
            camp_activity(
                "ONLINE",
                true,
                Some("camp_activity response=yes error_raw=0 len=16\n")
            ),
            Some("accepted")
        );
        assert_eq!(
            camp_activity(
                "ONLINE",
                true,
                Some("camp_activity response=yes error_raw=2 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_activity(
                "ONLINE",
                true,
                Some("camp_fastdorm response=yes error_raw=0 len=12\n")
            ),
            None
        );
        assert_eq!(
            camp_activity(
                "OFFLINE",
                true,
                Some("camp_activity response=yes error_raw=0 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_fastdorm(
                "ONLINE",
                true,
                Some("camp_fastdorm response=yes error_raw=0 len=12\n")
            ),
            Some("accepted")
        );
        assert_eq!(
            camp_fastdorm(
                "ONLINE",
                true,
                Some("camp_fastdorm response=yes error_raw=2 len=12\n")
            ),
            None
        );
        assert_eq!(
            camp_fastdorm(
                "ONLINE",
                true,
                Some("camp_activity response=yes error_raw=0 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_fastdorm(
                "OFFLINE",
                true,
                Some("camp_fastdorm response=yes error_raw=0 len=12\n")
            ),
            None
        );
        assert_eq!(
            camp_endc(
                "ONLINE",
                true,
                Some("camp_endc response=yes error_raw=0 len=16\n")
            ),
            Some("accepted")
        );
        assert_eq!(
            camp_endc(
                "ONLINE",
                true,
                Some("camp_endc response=yes error_raw=2 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_endc(
                "ONLINE",
                true,
                Some("camp_vonrcapa response=yes error_raw=0 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_endc(
                "OFFLINE",
                true,
                Some("camp_endc response=yes error_raw=0 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_throttle(
                "ONLINE",
                true,
                Some("camp_throttle response=yes error_raw=0 len=16\n")
            ),
            Some("accepted")
        );
        assert_eq!(
            camp_throttle(
                "ONLINE",
                true,
                Some("camp_throttle response=yes error_raw=2 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_throttle(
                "ONLINE",
                true,
                Some("camp_screen response=yes error_raw=0 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_throttle(
                "OFFLINE",
                true,
                Some("camp_throttle response=yes error_raw=0 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_unsolff(
                "ONLINE",
                true,
                Some("camp_unsolff response=yes error_raw=0 len=16\n")
            ),
            Some("accepted")
        );
        assert_eq!(
            camp_unsolff(
                "ONLINE",
                true,
                Some("camp_unsolff response=yes error_raw=2 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_unsolff(
                "ONLINE",
                true,
                Some("camp_unsol response=yes error_raw=0 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_unsolff(
                "OFFLINE",
                true,
                Some("camp_unsolff response=yes error_raw=0 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_unsol(
                "ONLINE",
                true,
                Some("camp_unsol response=yes error_raw=0 len=16\n")
            ),
            Some("accepted")
        );
        assert_eq!(
            camp_unsol(
                "ONLINE",
                true,
                Some("camp_unsol response=yes error_raw=2 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_unsol(
                "ONLINE",
                true,
                Some("camp_unsolff response=yes error_raw=0 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_unsol(
                "OFFLINE",
                true,
                Some("camp_unsol response=yes error_raw=0 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_screen(
                "ONLINE",
                true,
                Some("camp_screen response=yes error_raw=0 len=16\n")
            ),
            Some("accepted")
        );
        assert_eq!(
            camp_screen(
                "ONLINE",
                true,
                Some("camp_screen response=yes error_raw=2 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_screen(
                "ONLINE",
                true,
                Some("camp_unsol response=yes error_raw=0 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_screen(
                "OFFLINE",
                true,
                Some("camp_screen response=yes error_raw=0 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_cellinfo(
                "ONLINE",
                true,
                Some("camp_cellinfo response=yes error_raw=0 len=16\n")
            ),
            Some("accepted")
        );
        assert_eq!(
            camp_cellinfo(
                "ONLINE",
                true,
                Some("camp_cellinfo response=yes error_raw=2 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_cellinfo(
                "ONLINE",
                true,
                Some("camp_smsc response=yes error_raw=0 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_cellinfo(
                "OFFLINE",
                true,
                Some("camp_cellinfo response=yes error_raw=0 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_smsc(
                "ONLINE",
                true,
                Some("camp_smsc response=yes error_raw=0 len=16\n")
            ),
            Some("accepted")
        );
        assert_eq!(
            camp_smsc(
                "ONLINE",
                true,
                Some("camp_smsc response=yes error_raw=2 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_smsc(
                "ONLINE",
                true,
                Some("camp_cellinfo response=yes error_raw=0 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_smsc(
                "OFFLINE",
                true,
                Some("camp_smsc response=yes error_raw=0 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_vonrget(
                "ONLINE",
                true,
                Some("camp_vonrget response=yes error_raw=0 len=16\n")
            ),
            Some("accepted")
        );
        assert_eq!(
            camp_vonrget(
                "ONLINE",
                true,
                Some("camp_vonrget response=yes error_raw=2 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_vonrget(
                "ONLINE",
                true,
                Some("camp_vonrcapa response=yes error_raw=0 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_vonrget(
                "OFFLINE",
                true,
                Some("camp_vonrget response=yes error_raw=0 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_aptime(
                "ONLINE",
                true,
                Some("camp_aptime response=yes error_raw=0 len=16\n")
            ),
            Some("accepted")
        );
        assert_eq!(
            camp_aptime(
                "ONLINE",
                true,
                Some("camp_aptime response=yes error_raw=2 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_aptime(
                "ONLINE",
                true,
                Some("camp_vonrget response=yes error_raw=0 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_aptime(
                "OFFLINE",
                true,
                Some("camp_aptime response=yes error_raw=0 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_dbgtrace(
                "ONLINE",
                true,
                Some("camp_dbgtrace response=yes error_raw=0 len=16\n")
            ),
            Some("accepted")
        );
        assert_eq!(
            camp_dbgtrace(
                "ONLINE",
                true,
                Some("camp_dbgtrace response=yes error_raw=2 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_dbgtrace(
                "ONLINE",
                true,
                Some("camp_aptime response=yes error_raw=0 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_dbgtrace(
                "OFFLINE",
                true,
                Some("camp_dbgtrace response=yes error_raw=0 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_tty(
                "ONLINE",
                true,
                Some("camp_tty response=yes error_raw=0 len=16\n")
            ),
            Some("accepted")
        );
        assert_eq!(
            camp_tty(
                "ONLINE",
                true,
                Some("camp_tty response=yes error_raw=2 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_tty(
                "ONLINE",
                true,
                Some("camp_dbgtrace response=yes error_raw=0 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_tty(
                "OFFLINE",
                true,
                Some("camp_tty response=yes error_raw=0 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_pssvc(
                "ONLINE",
                true,
                Some("camp_pssvc response=yes error_raw=0 len=16\n")
            ),
            Some("accepted")
        );
        assert_eq!(
            camp_pssvc(
                "ONLINE",
                true,
                Some("camp_pssvc response=yes error_raw=2 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_pssvc(
                "ONLINE",
                true,
                Some("camp_tty response=yes error_raw=0 len=16\n")
            ),
            None
        );
        assert_eq!(
            camp_pssvc(
                "OFFLINE",
                true,
                Some("camp_pssvc response=yes error_raw=0 len=16\n")
            ),
            None
        );
        assert_eq!(camp_voice("ONLINE", false, Some(log)), None);
        assert_eq!(
            camp_voice("ONLINE", true, Some("field=voice registration_raw=9\n")),
            None
        );
        assert_eq!(camp_registration("OFFLINE", true, Some(log)), None);
        assert_eq!(camp_registration("ONLINE", false, Some(log)), None);
        assert_eq!(
            camp_registration("ONLINE", true, Some("field=data registration_raw=9\n")),
            None
        );
        let line = format!(
            "registration_raw={}",
            camp_registration("ONLINE", true, Some(log)).unwrap()
        );
        assert!(!line.contains("lac"));
        assert!(!line.contains("reject"));
    }

    #[test]
    fn live_bearer_uses_both_directions_and_skips_a_bad_name() {
        assert!(saai_observation::bearer_is_live(true, 0, 0));
        assert!(saai_observation::bearer_is_live(false, 4, 4));
        assert!(!saai_observation::bearer_is_live(false, 0, 4));
        let root = std::env::temp_dir().join(format!("saai-bearer-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for (name, rx, tx) in [("rmnet0", "0", "10"), ("rmnet1", "4", "4")] {
            let stats = root.join(name).join("statistics");
            fs::create_dir_all(&stats).unwrap();
            fs::write(stats.join("rx_bytes"), rx).unwrap();
            fs::write(stats.join("tx_bytes"), tx).unwrap();
        }
        let names = vec!["rmnet0".into(), "rmnet1".into(), "../rmnet1".into()];
        assert_eq!(live_bearers(&root, &names), vec!["rmnet1".to_string()]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn endpoint_suffix_names_only_the_holder_words() {
        assert_eq!(endpoint_suffix(Some("owner")), " endpoint=owner");
        assert_eq!(endpoint_suffix(Some("modemd")), " endpoint=modemd");
        assert_eq!(endpoint_suffix(Some("shared")), " endpoint=shared");
        assert_eq!(endpoint_suffix(Some("pin")), "");
        assert_eq!(endpoint_suffix(None), "");
    }

    #[cfg(unix)]
    #[test]
    fn endpoint_holder_reads_symlinks_and_does_not_need_the_device() {
        let root = std::env::temp_dir().join(format!("saai-endpoint-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("42/fd")).unwrap();
        fs::write(
            root.join("42/cmdline"),
            b"/data/saaios/bin/modem-rfs-camp-combined-owner\0",
        )
        .unwrap();
        fs::create_dir_all(root.join("7/fd")).unwrap();
        fs::write(root.join("7/cmdline"), b"/data/saaios/bin/saai-modemd\0").unwrap();
        fs::create_dir_all(root.join("notes")).unwrap();
        std::os::unix::fs::symlink("/dev/umts_ipc0.bak", root.join("42/fd/3")).unwrap();
        assert_eq!(endpoint_holder_at(&root), None);
        std::os::unix::fs::symlink("/dev/umts_ipc0", root.join("42/fd/4")).unwrap();
        assert_eq!(endpoint_holder_at(&root), Some("owner"));
        std::os::unix::fs::symlink("/dev/umts_rfs0", root.join("7/fd/5")).unwrap();
        assert_eq!(endpoint_holder_at(&root), Some("shared"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn bearer_filter_is_live_iface_only() {
        assert!(cellular_iface_name("rmnet_data0"));
        assert!(cellular_iface_name("wwan0"));
        assert!(cellular_iface_name("qmimux0"));
        assert!(cellular_iface_name("ccmni0"));
        assert!(!cellular_iface_name("usb0"));
        assert!(!cellular_iface_name("wlan0"));
        assert!(!cellular_iface_name("google_modemctl"));
    }

    #[test]
    fn efs_mount_detection_is_conservative() {
        assert!(mounts_include_original_efs("/dev/sda5 /efs ext4 ro 0 0\n"));
        assert!(mounts_include_original_efs(
            "/dev/block/by-name/efs /mnt/vendor/efs ext4 ro 0 0\n"
        ));
        assert!(!mounts_include_original_efs(
            "/dev/sda31 /data f2fs rw 0 0\nproc /proc proc rw 0 0\n"
        ));
    }
}
