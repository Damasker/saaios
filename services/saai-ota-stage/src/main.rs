//! S12 Change 3: staged, verified OTA download -- run manually via serial
//! for now, same "prove before wiring into `native-init.c`" precedent as
//! `saai-taskd` (ADR-030). Fetches a signed manifest and its declared
//! partition images over HTTP (USB-NCM in practice), verifies the
//! manifest (signature, schema, device model, anti-downgrade, allow-list)
//! before downloading anything, then streams and hashes each partition
//! into `--staging-dir`, verifying size+SHA-256 before the file is
//! considered staged. Never opens, resolves, or writes any real GPT
//! partition -- that is Change 4, gated separately.

use clap::Parser;
use ed25519_dalek::VerifyingKey;
use futures::StreamExt;
use saai_ota_manifest::{
    check_installable, verify_partition_digest, ManifestBody, PartitionEntry, SignedManifest,
};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};
use system_tools::{system_identity, ToolsMode};

/// ADR-044's write allow-list. `--allow-partition` exists only so this
/// tool can be tested against fixtures without editing the binary --
/// Change 4's real writer is the actual enforcement point, not this list.
const DEFAULT_ALLOWED_PARTITIONS: &[&str] = &["init_boot_a", "vendor_boot_a"];

#[derive(Debug, Parser)]
#[command(name = "saai-ota-stage")]
struct Args {
    /// URL of the signed manifest JSON. Partition images are fetched from
    /// sibling files named `<partition-name>.img` next to it.
    #[arg(long)]
    manifest_url: String,
    #[arg(long)]
    verifying_key: PathBuf,
    #[arg(long, default_value = "/data/saaios/system/installed-version")]
    installed_version_file: PathBuf,
    #[arg(long, default_value = "/data/saaios/update-staging")]
    staging_dir: PathBuf,
    #[arg(long = "allow-partition")]
    allow_partitions: Vec<String>,
}

fn fatal(msg: impl std::fmt::Display) -> ! {
    eprintln!("saai-ota-stage: {msg}");
    std::process::exit(1);
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = Args::parse();

    let device_target = detect_device_target();
    let installed_version = read_installed_version(&args.installed_version_file);
    let verifying_key = read_verifying_key(&args.verifying_key);
    let allow_list: Vec<&str> = if args.allow_partitions.is_empty() {
        DEFAULT_ALLOWED_PARTITIONS.to_vec()
    } else {
        args.allow_partitions.iter().map(String::as_str).collect()
    };

    println!("saai-ota-stage: device_target={device_target} installed_version={installed_version}");
    println!("saai-ota-stage: fetching manifest {}", args.manifest_url);

    let client = reqwest::Client::new();
    let manifest_bytes = fetch_bytes(&client, &args.manifest_url).await;
    let signed: SignedManifest = match serde_json::from_slice(&manifest_bytes) {
        Ok(signed) => signed,
        Err(err) => fatal(format!(
            "manifest is not valid JSON/schema ({} bytes received): {err}",
            manifest_bytes.len()
        )),
    };

    if let Err(err) = check_installable(
        &signed,
        &verifying_key,
        &device_target,
        &installed_version,
        &allow_list,
    ) {
        fatal(format!("manifest rejected, nothing staged: {err}"));
    }
    println!(
        "saai-ota-stage: manifest accepted (version={}, {} partition(s) declared)",
        signed.body.version,
        signed.body.partitions.len()
    );

    std::fs::create_dir_all(&args.staging_dir)
        .unwrap_or_else(|err| fatal(format!("creating {}: {err}", args.staging_dir.display())));

    let manifest_base = manifest_base_url(&args.manifest_url);
    let mut all_ok = true;
    for entry in &signed.body.partitions {
        let url = format!("{manifest_base}/{}.img", entry.name);
        match stage_partition(&client, &url, &args.staging_dir, &signed.body, entry).await {
            Ok(bytes) => println!(
                "saai-ota-stage: staged and verified '{}' ({bytes} bytes)",
                entry.name
            ),
            Err(err) => {
                eprintln!("saai-ota-stage: '{}' rejected: {err}", entry.name);
                all_ok = false;
            }
        }
    }

    if !all_ok {
        fatal("one or more partitions failed staging/verification -- see above, nothing written outside the staging directory");
    }
    println!(
        "saai-ota-stage: all partitions staged and verified in {}",
        args.staging_dir.display()
    );
}

async fn fetch_bytes(client: &reqwest::Client, url: &str) -> Vec<u8> {
    let response = client
        .get(url)
        .send()
        .await
        .unwrap_or_else(|err| fatal(format!("fetching {url}: {err}")));
    if !response.status().is_success() {
        fatal(format!("fetching {url}: HTTP {}", response.status()));
    }
    response
        .bytes()
        .await
        .unwrap_or_else(|err| fatal(format!("reading response body from {url}: {err}")))
        .to_vec()
}

fn manifest_base_url(manifest_url: &str) -> &str {
    match manifest_url.rfind('/') {
        Some(idx) => &manifest_url[..idx],
        None => "",
    }
}

/// Downloads one partition image, hashing it incrementally so the whole
/// file is never held in memory twice. On any mismatch or transport
/// error, the partial `.tmp` file is deleted -- only a fully verified
/// file is ever renamed into its final, visible staging path. Returns
/// the verified byte count on success.
async fn stage_partition(
    client: &reqwest::Client,
    url: &str,
    staging_dir: &Path,
    body: &ManifestBody,
    entry: &PartitionEntry,
) -> Result<u64, String> {
    let tmp_path = staging_dir.join(format!("{}.img.tmp", entry.name));
    let final_path = staging_dir.join(format!("{}.img", entry.name));

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|err| format!("fetching {url}: {err}"))?;
    if !response.status().is_success() {
        return Err(format!("fetching {url}: HTTP {}", response.status()));
    }

    let mut file = std::fs::File::create(&tmp_path)
        .map_err(|err| format!("creating {}: {err}", tmp_path.display()))?;
    let mut hasher = Sha256::new();
    let mut total: u64 = 0;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(err) => {
                let _ = std::fs::remove_file(&tmp_path);
                return Err(format!(
                    "download interrupted after {total} of {} declared bytes: {err}",
                    entry.size_bytes
                ));
            }
        };
        hasher.update(&chunk);
        total += chunk.len() as u64;
        if let Err(err) = file.write_all(&chunk) {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(format!("writing {}: {err}", tmp_path.display()));
        }
    }
    drop(file);

    let actual_sha = hex::encode(hasher.finalize());
    if let Err(err) = verify_partition_digest(body, &entry.name, total, &actual_sha) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(err.to_string());
    }

    std::fs::rename(&tmp_path, &final_path).map_err(|err| {
        format!(
            "renaming {} -> {}: {err}",
            tmp_path.display(),
            final_path.display()
        )
    })?;
    Ok(total)
}

fn detect_device_target() -> String {
    let identity = system_identity(ToolsMode::RealLinux);
    identity
        .get("target")
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| {
            fatal("could not determine device target (androidboot.hardware) from /proc/bootconfig")
        })
}

fn read_installed_version(path: &Path) -> semver::Version {
    match std::fs::read_to_string(path) {
        Ok(text) => semver::Version::parse(text.trim())
            .unwrap_or_else(|err| fatal(format!("{}: not valid semver: {err}", path.display()))),
        // No recorded install yet -- treat as "nothing installed by OTA so
        // far" rather than refusing to run. Change 4 is responsible for
        // writing this file after a confirmed-healthy install.
        Err(_) => semver::Version::new(0, 0, 0),
    }
}

fn read_verifying_key(path: &Path) -> VerifyingKey {
    let hex_str = std::fs::read_to_string(path)
        .unwrap_or_else(|err| fatal(format!("reading {}: {err}", path.display())));
    let bytes = hex::decode(hex_str.trim())
        .unwrap_or_else(|err| fatal(format!("{}: not valid hex: {err}", path.display())));
    let bytes: [u8; 32] = bytes
        .try_into()
        .unwrap_or_else(|_| fatal(format!("{}: expected 32 bytes", path.display())));
    VerifyingKey::from_bytes(&bytes).unwrap_or_else(|err| {
        fatal(format!(
            "{}: not a valid ed25519 key: {err}",
            path.display()
        ))
    })
}
