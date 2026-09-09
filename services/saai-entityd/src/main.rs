use clap::Parser;
use saai_entityd::{run_daemon, DaemonConfig};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "saai-entityd")]
struct Args {
    #[arg(long, default_value = "/data/saaios/var/entities")]
    store_root: PathBuf,
    #[arg(long, default_value = "/data/saaios/var/ui/active-space")]
    legacy_active_space: PathBuf,
    #[arg(long, default_value = "/run/saaios/entityd.sock")]
    socket: PathBuf,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = Args::parse();
    let config = DaemonConfig {
        store_root: args.store_root,
        legacy_active_space: args.legacy_active_space,
        socket_path: args.socket,
    };
    if let Err(error) = run_daemon(config).await {
        eprintln!("saai-entityd: {error}");
        std::process::exit(1);
    }
}
