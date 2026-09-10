use std::path::PathBuf;

use clap::Parser;
use saai_appd::{run_daemon, DaemonConfig};

#[derive(Debug, Parser)]
#[command(name = "saai-appd")]
struct Args {
    #[arg(long, default_value = "/data/saaios")]
    data_root: PathBuf,
    #[arg(long, default_value = "/run/saaios/appd.sock")]
    socket: PathBuf,
    #[arg(long, env = "XDG_RUNTIME_DIR", default_value = "/run/wayland")]
    runtime_dir: PathBuf,
    #[arg(long, env = "WAYLAND_DISPLAY", default_value = "wayland-1")]
    wayland_display: String,
    /// Explicit non-root bypass for host integration tests only.
    #[arg(long, hide = true)]
    allow_unsandboxed: bool,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = Args::parse();
    let config = DaemonConfig {
        data_root: args.data_root,
        socket_path: args.socket,
        runtime_dir: args.runtime_dir,
        wayland_display: args.wayland_display,
        allow_unsandboxed: args.allow_unsandboxed,
    };
    if let Err(error) = run_daemon(config).await {
        eprintln!("saai-appd: {error}");
        std::process::exit(1);
    }
}
