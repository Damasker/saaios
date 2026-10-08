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
    /// Decide whether one reviewed GET may run. This command does not open
    /// the modem endpoint. A running owner or the status tool is busy.
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
            for line in status_lines(&state, &bearers, endpoint_holder_at(Path::new("/proc"))) {
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
            let admission = runtime_model::query_admission(cp.as_deref(), owner_running, status_lock_busy);
            let epoch = supervise::latest_boot_epoch(std::path::Path::new(DEFAULT_BOOT_ARCHIVE));
            let epoch_suffix = match epoch {
                Some(epoch) => format!(" epoch={epoch}"),
                None => String::new(),
            };
            let endpoint_suffix = endpoint_suffix(endpoint_holder_at(Path::new("/proc")));
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

fn status_lines(state: &str, bearers: &[String], holder: Option<&str>) -> Vec<String> {
    let mut lines = vec![format!("modem_state={state}")];
    if bearers.is_empty() {
        lines.push("cellular_bearers=none".to_string());
    } else {
        lines.push(format!("cellular_bearers={}", bearers.join(",")));
    }
    if let Some(holder) = endpoint_suffix(holder).strip_prefix(' ') {
        lines.push(holder.to_string());
    }
    lines.push("hardware_actions=none".to_string());
    lines
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
    fn status_names_the_endpoint_holder_without_a_hardware_action() {
        let lines = status_lines("ONLINE", &["rmnet1".into()], Some("owner"));
        assert_eq!(
            lines,
            vec![
                "modem_state=ONLINE".to_string(),
                "cellular_bearers=rmnet1".to_string(),
                "endpoint=owner".to_string(),
                "hardware_actions=none".to_string(),
            ]
        );
        let absent = status_lines("missing", &[], None);
        assert_eq!(
            absent,
            vec![
                "modem_state=missing".to_string(),
                "cellular_bearers=none".to_string(),
                "hardware_actions=none".to_string(),
            ]
        );
        assert!(!absent.iter().any(|line| line.contains("endpoint=")));
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
