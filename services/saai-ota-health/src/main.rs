//! S12 Change 5: boot-attempt counting, health confirmation, and the
//! trigger for an automatic rollback -- run manually via serial for now,
//! same "prove before wiring into `native-init.c`" precedent as
//! `saai-taskd`/`saai-ota-stage`/`saai-ota-write` (ADR-030/046/047).
//! `record-attempt` and `check` simulate what a future `native-init.c`
//! would call at boot start and once core services should be up,
//! respectively -- this Change does not wire either call in.
//!
//! `maybe-rollback` never touches a partition itself: it shells out to
//! the already-proven `saai-ota-write restore` for each partition, so
//! this tool carries no partition-write code of its own.

mod counter;
mod health;

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

const DEFAULT_COUNTER_FILE: &str = "/data/saaios/system/boot-attempts";

#[derive(Debug, Parser)]
#[command(name = "saai-ota-health")]
struct Args {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Increments the persisted boot-attempt counter.
    RecordAttempt {
        #[arg(long, default_value = DEFAULT_COUNTER_FILE)]
        counter_file: PathBuf,
    },
    /// Polls core service presence for up to `--budget-secs`. On success,
    /// resets the counter (this boot is healthy) and exits 0. On timeout,
    /// leaves the counter untouched, reports what's missing, exits 1.
    Check {
        #[arg(long, default_value = DEFAULT_COUNTER_FILE)]
        counter_file: PathBuf,
        #[arg(long, default_value_t = 15)]
        budget_secs: u64,
        #[arg(long = "process")]
        processes: Vec<String>,
        #[arg(long, default_value = "/proc")]
        proc_root: PathBuf,
    },
    /// If the counter has reached `--limit` without an intervening
    /// successful `check`, restores each `--partition name=backup_path`
    /// via `saai-ota-write restore` and resets the counter. Does not
    /// reboot -- that stays a separate, explicit step.
    MaybeRollback {
        #[arg(long, default_value = DEFAULT_COUNTER_FILE)]
        counter_file: PathBuf,
        #[arg(long, default_value_t = 3)]
        limit: u32,
        #[arg(long)]
        saai_ota_write: PathBuf,
        #[arg(long = "partition", value_parser = parse_partition_arg)]
        partitions: Vec<(String, PathBuf)>,
        #[arg(long, default_value = "/sys/class/block")]
        sys_class_block: PathBuf,
    },
}

fn parse_partition_arg(raw: &str) -> Result<(String, PathBuf), String> {
    let (name, path) = raw
        .split_once('=')
        .ok_or_else(|| format!("expected name=path, got '{raw}'"))?;
    Ok((name.to_string(), PathBuf::from(path)))
}

fn fatal(msg: impl std::fmt::Display) -> ! {
    eprintln!("saai-ota-health: {msg}");
    std::process::exit(1);
}

fn main() {
    let args = Args::parse();
    match args.command {
        Cmd::RecordAttempt { counter_file } => cmd_record_attempt(&counter_file),
        Cmd::Check {
            counter_file,
            budget_secs,
            processes,
            proc_root,
        } => cmd_check(&counter_file, budget_secs, processes, &proc_root),
        Cmd::MaybeRollback {
            counter_file,
            limit,
            saai_ota_write,
            partitions,
            sys_class_block,
        } => cmd_maybe_rollback(
            &counter_file,
            limit,
            &saai_ota_write,
            partitions,
            &sys_class_block,
        ),
    }
}

fn cmd_record_attempt(counter_file: &Path) {
    match counter::increment(counter_file) {
        Ok(count) => println!("saai-ota-health: boot attempt recorded, count={count}"),
        Err(err) => fatal(format!(
            "recording attempt in {}: {err}",
            counter_file.display()
        )),
    }
}

fn cmd_check(counter_file: &Path, budget_secs: u64, processes: Vec<String>, proc_root: &Path) {
    let processes = if processes.is_empty() {
        health::default_processes()
    } else {
        processes
    };
    let deadline = Instant::now() + Duration::from_secs(budget_secs);

    loop {
        let missing = health::missing_processes(proc_root, &processes);
        if missing.is_empty() {
            if let Err(err) = counter::reset(counter_file) {
                fatal(format!("resetting {}: {err}", counter_file.display()));
            }
            println!(
                "saai-ota-health: healthy -- all {} core process(es) present, counter reset",
                processes.len()
            );
            return;
        }
        if Instant::now() >= deadline {
            fatal(format!(
                "UNHEALTHY after {budget_secs}s -- missing: {}",
                missing.join(", ")
            ));
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

fn cmd_maybe_rollback(
    counter_file: &Path,
    limit: u32,
    saai_ota_write: &Path,
    partitions: Vec<(String, PathBuf)>,
    sys_class_block: &Path,
) {
    let count = counter::read(counter_file);
    if count < limit {
        println!("saai-ota-health: attempt count {count} below limit {limit}, no rollback");
        return;
    }

    println!(
        "saai-ota-health: attempt count {count} >= limit {limit} -- rolling back {} partition(s)",
        partitions.len()
    );
    let mut all_ok = true;
    for (name, backup_path) in &partitions {
        let result = Command::new(saai_ota_write)
            .arg("restore")
            .args(["--partition", name])
            .arg("--sys-class-block")
            .arg(sys_class_block)
            .arg("--backup")
            .arg(backup_path)
            .status();
        match result {
            Ok(status) if status.success() => println!("saai-ota-health: '{name}' restored"),
            Ok(status) => {
                eprintln!("saai-ota-health: restoring '{name}' failed, exit status {status}");
                all_ok = false;
            }
            Err(err) => {
                eprintln!(
                    "saai-ota-health: launching {} for '{name}' failed: {err}",
                    saai_ota_write.display()
                );
                all_ok = false;
            }
        }
    }

    if all_ok {
        if let Err(err) = counter::reset(counter_file) {
            fatal(format!(
                "resetting {} after rollback: {err}",
                counter_file.display()
            ));
        }
        println!("saai-ota-health: rollback complete, counter reset -- reboot to apply");
    } else {
        fatal("one or more partitions failed to restore -- counter NOT reset, manual intervention needed");
    }
}
