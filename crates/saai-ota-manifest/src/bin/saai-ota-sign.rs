//! Build-server-side tool for S12 Change 2: generates the ed25519 keypair
//! used to sign OTA manifests, and builds+signs a manifest from a set of
//! partition image files. Never runs on the device -- the signing key
//! this produces must stay on the build server (or an offline store), and
//! only the verifying (public) key is ever embedded in a device image
//! (S12 DoR's Threat/privacy impact).

use clap::{Parser, Subcommand};
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand_core::OsRng;
use saai_ota_manifest::{
    check_installable, sha256_hex, sign_manifest, verify_partition_bytes, ManifestBody,
    PartitionEntry, SignedManifest, OTA_MANIFEST_SCHEMA_V1,
};
use std::fs;
use std::path::PathBuf;

/// ADR-044's write allow-list -- the default here mirrors the one code
/// path Change 4 will actually use; `--allow-partition` exists only so
/// this tool can be tested against fixtures without editing the binary.
const DEFAULT_ALLOWED_PARTITIONS: &[&str] = &["init_boot_a", "vendor_boot_a"];

#[derive(Debug, Parser)]
#[command(name = "saai-ota-sign")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Generates a new ed25519 keypair for signing OTA manifests.
    Keygen {
        #[arg(long, default_value = "signing.key")]
        signing_key_out: PathBuf,
        #[arg(long, default_value = "verifying.pub")]
        verifying_key_out: PathBuf,
    },
    /// Builds and signs a manifest from a set of partition image files.
    Build {
        #[arg(long)]
        signing_key: PathBuf,
        #[arg(long)]
        target: String,
        #[arg(long)]
        version: String,
        #[arg(long)]
        min_supported_version: String,
        /// `name=path/to/image`, repeatable -- one per partition.
        #[arg(long = "partition", value_parser = parse_partition_arg)]
        partitions: Vec<(String, PathBuf)>,
        #[arg(long)]
        notes: Option<String>,
        #[arg(long, default_value = "manifest.json")]
        out: PathBuf,
    },
    /// Runs the full install-time gate against a manifest file -- the same
    /// checks a real device-side verifier would run before staging a
    /// download (S12 Change 3). Exits non-zero and prints the rejection
    /// reason on any failure; never touches a device.
    Verify {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        verifying_key: PathBuf,
        #[arg(long)]
        device_target: String,
        #[arg(long)]
        installed_version: String,
        /// Overrides the default allow-list, for testing only.
        #[arg(long = "allow-partition")]
        allow_partitions: Vec<String>,
        /// If given, also checks this partition's staged bytes against the
        /// manifest's declared hash/size.
        #[arg(long)]
        check_partition: Option<String>,
        #[arg(long)]
        partition_file: Option<PathBuf>,
    },
}

fn parse_partition_arg(raw: &str) -> Result<(String, PathBuf), String> {
    let (name, path) = raw
        .split_once('=')
        .ok_or_else(|| format!("expected name=path, got '{raw}'"))?;
    Ok((name.to_string(), PathBuf::from(path)))
}

fn fatal(msg: &str) -> ! {
    eprintln!("saai-ota-sign: {msg}");
    std::process::exit(1);
}

fn main() {
    let args = Args::parse();
    match args.command {
        Command::Keygen {
            signing_key_out,
            verifying_key_out,
        } => keygen(&signing_key_out, &verifying_key_out),
        Command::Build {
            signing_key,
            target,
            version,
            min_supported_version,
            partitions,
            notes,
            out,
        } => build(
            &signing_key,
            target,
            version,
            min_supported_version,
            partitions,
            notes,
            &out,
        ),
        Command::Verify {
            manifest,
            verifying_key,
            device_target,
            installed_version,
            allow_partitions,
            check_partition,
            partition_file,
        } => verify(
            &manifest,
            &verifying_key,
            &device_target,
            &installed_version,
            allow_partitions,
            check_partition,
            partition_file,
        ),
    }
}

fn keygen(signing_key_out: &PathBuf, verifying_key_out: &PathBuf) {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    write_private_key_file(signing_key_out, &signing_key);
    fs::write(verifying_key_out, hex::encode(verifying_key.to_bytes()))
        .unwrap_or_else(|err| fatal(&format!("writing {}: {err}", verifying_key_out.display())));

    println!(
        "signing key (PRIVATE, keep off-device, never commit): {}",
        signing_key_out.display()
    );
    println!(
        "verifying key (public, embed in device image): {}",
        verifying_key_out.display()
    );
    println!(
        "verifying key hex: {}",
        hex::encode(verifying_key.to_bytes())
    );
}

#[cfg(unix)]
fn write_private_key_file(path: &PathBuf, signing_key: &SigningKey) {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .unwrap_or_else(|err| fatal(&format!("creating {}: {err}", path.display())));
    file.write_all(hex::encode(signing_key.to_bytes()).as_bytes())
        .unwrap_or_else(|err| fatal(&format!("writing {}: {err}", path.display())));
}

#[cfg(not(unix))]
fn write_private_key_file(path: &PathBuf, signing_key: &SigningKey) {
    fs::write(path, hex::encode(signing_key.to_bytes()))
        .unwrap_or_else(|err| fatal(&format!("writing {}: {err}", path.display())));
}

fn read_signing_key(path: &PathBuf) -> SigningKey {
    let hex_str = fs::read_to_string(path)
        .unwrap_or_else(|err| fatal(&format!("reading {}: {err}", path.display())));
    let bytes = hex::decode(hex_str.trim())
        .unwrap_or_else(|err| fatal(&format!("{}: not valid hex: {err}", path.display())));
    let bytes: [u8; 32] = bytes
        .try_into()
        .unwrap_or_else(|_| fatal(&format!("{}: expected 32 bytes", path.display())));
    SigningKey::from_bytes(&bytes)
}

#[allow(clippy::too_many_arguments)]
fn build(
    signing_key_path: &PathBuf,
    target: String,
    version: String,
    min_supported_version: String,
    partitions: Vec<(String, PathBuf)>,
    notes: Option<String>,
    out: &PathBuf,
) {
    let signing_key = read_signing_key(signing_key_path);

    let mut entries = Vec::new();
    for (name, path) in partitions {
        let data = fs::read(&path)
            .unwrap_or_else(|err| fatal(&format!("reading {}: {err}", path.display())));
        entries.push(PartitionEntry {
            sha256: sha256_hex(&data),
            size_bytes: data.len() as u64,
            name,
        });
    }

    let body = ManifestBody {
        schema: OTA_MANIFEST_SCHEMA_V1,
        target,
        version,
        min_supported_version,
        partitions: entries,
        built_at: chrono::Utc::now().to_rfc3339(),
        notes,
    };

    let signed = sign_manifest(body, &signing_key);
    let json = serde_json::to_string_pretty(&signed).expect("SignedManifest is serializable");
    fs::write(out, json).unwrap_or_else(|err| fatal(&format!("writing {}: {err}", out.display())));
    println!("wrote signed manifest: {}", out.display());
}

fn read_verifying_key(path: &PathBuf) -> VerifyingKey {
    let hex_str = fs::read_to_string(path)
        .unwrap_or_else(|err| fatal(&format!("reading {}: {err}", path.display())));
    let bytes = hex::decode(hex_str.trim())
        .unwrap_or_else(|err| fatal(&format!("{}: not valid hex: {err}", path.display())));
    let bytes: [u8; 32] = bytes
        .try_into()
        .unwrap_or_else(|_| fatal(&format!("{}: expected 32 bytes", path.display())));
    VerifyingKey::from_bytes(&bytes).unwrap_or_else(|err| {
        fatal(&format!(
            "{}: not a valid ed25519 key: {err}",
            path.display()
        ))
    })
}

#[allow(clippy::too_many_arguments)]
fn verify(
    manifest_path: &PathBuf,
    verifying_key_path: &PathBuf,
    device_target: &str,
    installed_version: &str,
    allow_partitions: Vec<String>,
    check_partition: Option<String>,
    partition_file: Option<PathBuf>,
) {
    let manifest_json = fs::read_to_string(manifest_path)
        .unwrap_or_else(|err| fatal(&format!("reading {}: {err}", manifest_path.display())));
    let signed: SignedManifest = match serde_json::from_str(&manifest_json) {
        Ok(signed) => signed,
        Err(err) => fatal(&format!(
            "{}: malformed manifest: {err}",
            manifest_path.display()
        )),
    };

    let verifying_key = read_verifying_key(verifying_key_path);

    let installed_version = semver::Version::parse(installed_version)
        .unwrap_or_else(|err| fatal(&format!("--installed-version: {err}")));

    let allow_list: Vec<&str> = if allow_partitions.is_empty() {
        DEFAULT_ALLOWED_PARTITIONS.to_vec()
    } else {
        allow_partitions.iter().map(String::as_str).collect()
    };

    match check_installable(
        &signed,
        &verifying_key,
        device_target,
        &installed_version,
        &allow_list,
    ) {
        Ok(()) => println!("manifest accepted: {}", manifest_path.display()),
        Err(err) => fatal(&format!("manifest rejected: {err}")),
    }

    if let (Some(partition_name), Some(file_path)) = (check_partition, partition_file) {
        let data = fs::read(&file_path)
            .unwrap_or_else(|err| fatal(&format!("reading {}: {err}", file_path.display())));
        match verify_partition_bytes(&signed.body, &partition_name, &data) {
            Ok(()) => println!(
                "partition '{partition_name}' content accepted: {}",
                file_path.display()
            ),
            Err(err) => fatal(&format!("partition '{partition_name}' rejected: {err}")),
        }
    }
}
