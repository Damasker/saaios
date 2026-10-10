use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use saai_modemd::boot_model;
use saai_modemd::os_gate::OsGate;
use saai_modemd::post_edge;
use saai_modemd::soft_lock::{
    self, SoftLockSnapshot, APP_STATE_PIN, PIN1_DISABLED, PIN1_ENABLED_VERIFIED,
    TRAY_BEARER_CHASE_CMD, TRAY_BEARER_CHASE_ON_DEVICE,
};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_MODEM_STATE: &str = "/sys/devices/platform/cpif/modem_state";
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
    /// OS boot/shell contract: cellular unavailable; continue without SIT.
    /// Host-safe: no modem endpoints opened.
    OsGate,
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
}

fn main() {
    if let Err(error) = run() {
        eprintln!("saai-modemd: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    match Args::parse().command {
        Cmd::OsGate => {
            for line in OsGate::default().lines() {
                println!("{line}");
            }
        }
        Cmd::Status {
            modem_state,
            net_class,
        } => {
            let state = read_trimmed_optional(&modem_state).unwrap_or_else(|| "missing".into());
            let bearers = cellular_ifaces(&net_class)?;
            println!("modem_state={state}");
            if bearers.is_empty() {
                println!("cellular_bearers=none");
            } else {
                println!("cellular_bearers={}", bearers.join(","));
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
    }
    Ok(())
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
