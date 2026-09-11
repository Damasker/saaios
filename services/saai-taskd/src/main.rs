use clap::Parser;
use saai_taskd::Daemon;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "saai-taskd")]
struct Args {
    #[arg(long, default_value = "/run/saaios/entityd.sock")]
    entityd_socket: PathBuf,
    /// Space this daemon watches for `saaios.intent` entities. S09
    /// Change 2 only needs one space -- multi-space workflows are out
    /// of scope until something actually asks for them.
    #[arg(long, default_value = "home")]
    space: String,
    /// `host:port` of the on-device `saaios-runtime` free-form intents
    /// are bridged to (ADR-033). Required, not defaulted -- it's
    /// specific to how this particular device's `saaios-runtime` is
    /// reached (a USB-NCM address of whatever host is connected right
    /// now), not something safe to guess.
    #[arg(long)]
    runtime_addr: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = Args::parse();
    let mut daemon =
        match Daemon::connect(&args.entityd_socket, args.space, args.runtime_addr).await {
            Ok(daemon) => daemon,
            Err(error) => {
                eprintln!("saai-taskd: failed to connect to saai-entityd: {error}");
                std::process::exit(1);
            }
        };
    match daemon.reconcile_existing_intents().await {
        Ok(0) => {}
        Ok(count) => eprintln!("saai-taskd: reconciled {count} pre-existing intent(s)"),
        Err(error) => {
            eprintln!("saai-taskd: reconcile failed: {error}");
            std::process::exit(1);
        }
    }
    // Change 3: a Task confirmed (moved to Running) while this daemon
    // wasn't running to see the event still needs its Action executed.
    // A Task still WaitingConfirmation is deliberately left untouched
    // here -- see Daemon::reconcile_confirmed_tasks's own doc comment.
    match daemon.reconcile_confirmed_tasks().await {
        Ok(0) => {}
        Ok(count) => eprintln!("saai-taskd: resumed {count} confirmed task(s)"),
        Err(error) => {
            eprintln!("saai-taskd: confirmed-task reconcile failed: {error}");
            std::process::exit(1);
        }
    }
    if let Err(error) = daemon.run().await {
        eprintln!("saai-taskd: {error}");
        std::process::exit(1);
    }
}
