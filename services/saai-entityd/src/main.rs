use clap::Parser;
use saai_entityd::{run_daemon, DaemonConfig};
use std::path::PathBuf;

const USB_NCM_ENTITYD: &str = "172.31.7.1:38128";

#[derive(Debug, Parser)]
#[command(name = "saai-entityd")]
struct Args {
    #[arg(long, default_value = "/data/saaios/var/entities")]
    store_root: PathBuf,
    #[arg(long, default_value = "/data/saaios/var/ui/active-space")]
    legacy_active_space: PathBuf,
    #[arg(long, default_value = "/run/saaios/entityd.sock")]
    socket: PathBuf,
    /// Second surface over USB NCM. `none` disables. Default tries the
    /// gadget address and stays UDS-only if bind fails (ADR-307).
    #[arg(long)]
    tcp_bind: Option<String>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = Args::parse();
    let (tcp_bind, tcp_required) = match args.tcp_bind.as_deref() {
        Some("none") | Some("") => (None, false),
        Some(addr) => (Some(addr.to_string()), true),
        None => (Some(USB_NCM_ENTITYD.to_string()), false),
    };
    let config = DaemonConfig {
        store_root: args.store_root,
        legacy_active_space: args.legacy_active_space,
        socket_path: args.socket,
        tcp_bind,
        tcp_required,
    };
    if let Err(error) = run_daemon(config).await {
        eprintln!("saai-entityd: {error}");
        std::process::exit(1);
    }
}
