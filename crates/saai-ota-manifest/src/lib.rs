//! Signed, versioned OTA manifest (S12 Change 2, ADR pending).
//!
//! A manifest names the partitions an update touches, their expected
//! content (size + SHA-256), the target device model and version, and is
//! itself signed with ed25519. The private signing key never ships on a
//! device or in this repository -- only `VerifyingKey` (public) is ever
//! embedded in a device image or passed to [`check_installable`].
//!
//! `check_installable` is the full install-time gate (signature, schema,
//! device model, anti-downgrade, partition allow-list). It does not check
//! partition *content* -- that is [`verify_partition_bytes`], run once an
//! artifact is staged (S12 Change 3), after `check_installable` has
//! already accepted the manifest itself.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Manifest schema version. Bumped only for a breaking format change --
/// `#[serde(deny_unknown_fields)]` below means an old verifier already
/// rejects a manifest carrying fields it does not know, so schema bumps
/// are for removing/renaming fields, not just adding them.
pub const OTA_MANIFEST_SCHEMA_V1: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartitionEntry {
    /// Real GPT `PARTNAME` (ADR-044) -- e.g. `init_boot_a`. Checked against
    /// an explicit allow-list at install time, never trusted on its own.
    pub name: String,
    /// Lowercase hex SHA-256, 64 characters.
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestBody {
    pub schema: u32,
    /// Must match the device's live `androidboot.hardware`
    /// (`system_identity`'s `target` field) -- e.g. `panther`.
    pub target: String,
    /// semver. Anti-downgrade requires this to be strictly greater than
    /// the device's currently installed version.
    pub version: String,
    /// semver floor. The device's currently installed version must be at
    /// least this -- a compatibility gate distinct from anti-downgrade,
    /// e.g. for updates that assume an earlier migration already ran.
    pub min_supported_version: String,
    pub partitions: Vec<PartitionEntry>,
    /// RFC3339 timestamp. Informational only, not checked.
    pub built_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedManifest {
    pub body: ManifestBody,
    /// Hex-encoded ed25519 signature (64 raw bytes -> 128 hex chars) over
    /// `body.canonical_bytes()`.
    pub signature: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ManifestError {
    #[error("manifest schema {0} is not supported")]
    UnsupportedSchema(u32),
    #[error("signature is not valid hex/length: {0}")]
    BadSignatureEncoding(String),
    #[error("signature does not verify against manifest body")]
    BadSignature,
    #[error("manifest version is not valid semver: {0}")]
    InvalidVersion(String),
    #[error("manifest target '{manifest}' does not match device target '{device}'")]
    WrongTarget { manifest: String, device: String },
    #[error("manifest version {manifest} is not newer than installed version {installed}")]
    Downgrade { manifest: String, installed: String },
    #[error(
        "installed version {installed} is older than manifest's minimum supported version {min}"
    )]
    BelowMinimumSupported { installed: String, min: String },
    #[error("partition '{0}' is not on the allowed write list")]
    PartitionNotAllowed(String),
    #[error("manifest does not declare partition '{0}'")]
    PartitionNotInManifest(String),
    #[error(
        "partition '{name}' content mismatch: expected sha256={expected_sha} size={expected_size}, got sha256={actual_sha} size={actual_size}"
    )]
    PartitionMismatch {
        name: String,
        expected_sha: String,
        expected_size: u64,
        actual_sha: String,
        actual_size: u64,
    },
}

impl ManifestBody {
    /// Deterministic bytes that get signed and re-checked at verify time.
    /// Signer and verifier always use this exact Rust type -- struct field
    /// order is fixed by the definition above, and every field is a
    /// `String`/`u64`/`Vec`/`Option`, never a `HashMap` -- so plain
    /// `serde_json` serialization is already deterministic without a
    /// separate canonicalization scheme.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("ManifestBody fields are all directly serializable")
    }
}

pub fn sign_manifest(body: ManifestBody, signing_key: &SigningKey) -> SignedManifest {
    let signature = signing_key.sign(&body.canonical_bytes());
    SignedManifest {
        body,
        signature: hex::encode(signature.to_bytes()),
    }
}

pub fn verify_signature(
    signed: &SignedManifest,
    verifying_key: &VerifyingKey,
) -> Result<(), ManifestError> {
    let sig_bytes = hex::decode(&signed.signature)
        .map_err(|err| ManifestError::BadSignatureEncoding(err.to_string()))?;
    let sig_bytes: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| ManifestError::BadSignatureEncoding("expected 64 bytes".to_string()))?;
    let signature = Signature::from_bytes(&sig_bytes);
    verifying_key
        .verify(&signed.body.canonical_bytes(), &signature)
        .map_err(|_| ManifestError::BadSignature)
}

/// Full install-time gate: signature, schema, device model, anti-downgrade,
/// partition allow-list. Run this before staging any download; run
/// [`verify_partition_bytes`] per-partition once content is staged.
pub fn check_installable(
    signed: &SignedManifest,
    verifying_key: &VerifyingKey,
    device_target: &str,
    installed_version: &semver::Version,
    allowed_partitions: &[&str],
) -> Result<(), ManifestError> {
    verify_signature(signed, verifying_key)?;

    if signed.body.schema != OTA_MANIFEST_SCHEMA_V1 {
        return Err(ManifestError::UnsupportedSchema(signed.body.schema));
    }

    if signed.body.target != device_target {
        return Err(ManifestError::WrongTarget {
            manifest: signed.body.target.clone(),
            device: device_target.to_string(),
        });
    }

    let manifest_version = semver::Version::parse(&signed.body.version)
        .map_err(|err| ManifestError::InvalidVersion(err.to_string()))?;
    let min_supported = semver::Version::parse(&signed.body.min_supported_version)
        .map_err(|err| ManifestError::InvalidVersion(err.to_string()))?;

    if manifest_version <= *installed_version {
        return Err(ManifestError::Downgrade {
            manifest: manifest_version.to_string(),
            installed: installed_version.to_string(),
        });
    }
    if *installed_version < min_supported {
        return Err(ManifestError::BelowMinimumSupported {
            installed: installed_version.to_string(),
            min: min_supported.to_string(),
        });
    }

    for entry in &signed.body.partitions {
        if !allowed_partitions.contains(&entry.name.as_str()) {
            return Err(ManifestError::PartitionNotAllowed(entry.name.clone()));
        }
    }

    Ok(())
}

pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Checks staged artifact bytes for one partition against the manifest's
/// declared hash and size. Call only after `check_installable` accepted
/// the manifest itself, and before any partition write.
pub fn verify_partition_bytes(
    body: &ManifestBody,
    partition_name: &str,
    data: &[u8],
) -> Result<(), ManifestError> {
    let entry = body
        .partitions
        .iter()
        .find(|entry| entry.name == partition_name)
        .ok_or_else(|| ManifestError::PartitionNotInManifest(partition_name.to_string()))?;

    let actual_size = data.len() as u64;
    let actual_sha = sha256_hex(data);

    if actual_size != entry.size_bytes || actual_sha != entry.sha256 {
        return Err(ManifestError::PartitionMismatch {
            name: partition_name.to_string(),
            expected_sha: entry.sha256.clone(),
            expected_size: entry.size_bytes,
            actual_sha,
            actual_size,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand_core::OsRng;

    fn test_keypair() -> (SigningKey, VerifyingKey) {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        (signing_key, verifying_key)
    }

    fn sample_body() -> ManifestBody {
        let artifact = b"fake init_boot_a image bytes";
        ManifestBody {
            schema: OTA_MANIFEST_SCHEMA_V1,
            target: "panther".to_string(),
            version: "0.12.0".to_string(),
            min_supported_version: "0.10.0".to_string(),
            partitions: vec![PartitionEntry {
                name: "init_boot_a".to_string(),
                sha256: sha256_hex(artifact),
                size_bytes: artifact.len() as u64,
            }],
            built_at: "2026-09-12T00:00:00Z".to_string(),
            notes: None,
        }
    }

    fn installed_version() -> semver::Version {
        semver::Version::parse("0.11.0").unwrap()
    }

    const ALLOWED: &[&str] = &["init_boot_a", "vendor_boot_a"];

    #[test]
    fn valid_manifest_is_accepted() {
        let (signing_key, verifying_key) = test_keypair();
        let signed = sign_manifest(sample_body(), &signing_key);
        assert!(check_installable(
            &signed,
            &verifying_key,
            "panther",
            &installed_version(),
            ALLOWED,
        )
        .is_ok());
    }

    #[test]
    fn tampered_signature_is_rejected() {
        let (signing_key, verifying_key) = test_keypair();
        let mut signed = sign_manifest(sample_body(), &signing_key);
        // Flip one hex character -- still valid hex, invalid signature.
        let mut chars: Vec<char> = signed.signature.chars().collect();
        chars[0] = if chars[0] == '0' { '1' } else { '0' };
        signed.signature = chars.into_iter().collect();

        let result = check_installable(
            &signed,
            &verifying_key,
            "panther",
            &installed_version(),
            ALLOWED,
        );
        assert_eq!(result, Err(ManifestError::BadSignature));
    }

    #[test]
    fn tampered_body_is_rejected_even_with_valid_signature_field() {
        let (signing_key, verifying_key) = test_keypair();
        let mut signed = sign_manifest(sample_body(), &signing_key);
        // Attacker edits the body after signing but leaves the (now
        // stale) signature bytes in place.
        signed.body.version = "9.9.9".to_string();

        let result = check_installable(
            &signed,
            &verifying_key,
            "panther",
            &installed_version(),
            ALLOWED,
        );
        assert_eq!(result, Err(ManifestError::BadSignature));
    }

    #[test]
    fn signed_by_wrong_key_is_rejected() {
        let (_, verifying_key) = test_keypair();
        let (attacker_key, _) = test_keypair();
        let signed = sign_manifest(sample_body(), &attacker_key);

        let result = check_installable(
            &signed,
            &verifying_key,
            "panther",
            &installed_version(),
            ALLOWED,
        );
        assert_eq!(result, Err(ManifestError::BadSignature));
    }

    #[test]
    fn wrong_device_model_is_rejected() {
        let (signing_key, verifying_key) = test_keypair();
        let mut body = sample_body();
        body.target = "some_other_device".to_string();
        let signed = sign_manifest(body, &signing_key);

        let result = check_installable(
            &signed,
            &verifying_key,
            "panther",
            &installed_version(),
            ALLOWED,
        );
        assert_eq!(
            result,
            Err(ManifestError::WrongTarget {
                manifest: "some_other_device".to_string(),
                device: "panther".to_string(),
            })
        );
    }

    #[test]
    fn downgrade_is_rejected() {
        let (signing_key, verifying_key) = test_keypair();
        let mut body = sample_body();
        body.version = "0.5.0".to_string(); // older than installed 0.11.0
        let signed = sign_manifest(body, &signing_key);

        let result = check_installable(
            &signed,
            &verifying_key,
            "panther",
            &installed_version(),
            ALLOWED,
        );
        assert_eq!(
            result,
            Err(ManifestError::Downgrade {
                manifest: "0.5.0".to_string(),
                installed: "0.11.0".to_string(),
            })
        );
    }

    #[test]
    fn reinstalling_same_version_is_rejected_as_downgrade() {
        let (signing_key, verifying_key) = test_keypair();
        let mut body = sample_body();
        body.version = "0.11.0".to_string(); // equal to installed
        let signed = sign_manifest(body, &signing_key);

        let result = check_installable(
            &signed,
            &verifying_key,
            "panther",
            &installed_version(),
            ALLOWED,
        );
        assert!(matches!(result, Err(ManifestError::Downgrade { .. })));
    }

    #[test]
    fn below_minimum_supported_is_rejected() {
        let (signing_key, verifying_key) = test_keypair();
        let mut body = sample_body();
        body.min_supported_version = "1.0.0".to_string(); // above installed 0.11.0
        body.version = "1.1.0".to_string();
        let signed = sign_manifest(body, &signing_key);

        let result = check_installable(
            &signed,
            &verifying_key,
            "panther",
            &installed_version(),
            ALLOWED,
        );
        assert_eq!(
            result,
            Err(ManifestError::BelowMinimumSupported {
                installed: "0.11.0".to_string(),
                min: "1.0.0".to_string(),
            })
        );
    }

    #[test]
    fn disallowed_partition_is_rejected() {
        let (signing_key, verifying_key) = test_keypair();
        let mut body = sample_body();
        body.partitions.push(PartitionEntry {
            name: "abl_a".to_string(), // bootloader chain, ADR-044 deny list
            sha256: sha256_hex(b"whatever"),
            size_bytes: 8,
        });
        let signed = sign_manifest(body, &signing_key);

        let result = check_installable(
            &signed,
            &verifying_key,
            "panther",
            &installed_version(),
            ALLOWED,
        );
        assert_eq!(
            result,
            Err(ManifestError::PartitionNotAllowed("abl_a".to_string()))
        );
    }

    #[test]
    fn unknown_schema_field_is_rejected_at_parse_time() {
        let json = r#"{
            "schema": 1,
            "target": "panther",
            "version": "0.12.0",
            "min_supported_version": "0.10.0",
            "partitions": [],
            "built_at": "2026-09-12T00:00:00Z",
            "unexpected_field_from_a_newer_or_hostile_schema": true
        }"#;
        let result: Result<ManifestBody, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn unsupported_schema_version_is_rejected() {
        let (signing_key, verifying_key) = test_keypair();
        let mut body = sample_body();
        body.schema = 2;
        let signed = sign_manifest(body, &signing_key);

        let result = check_installable(
            &signed,
            &verifying_key,
            "panther",
            &installed_version(),
            ALLOWED,
        );
        assert_eq!(result, Err(ManifestError::UnsupportedSchema(2)));
    }

    #[test]
    fn corrupted_artifact_bytes_are_rejected() {
        let body = sample_body();
        let corrupted = b"fake init_boot_a image BYTES-CORRUPTED";
        let result = verify_partition_bytes(&body, "init_boot_a", corrupted);
        assert!(matches!(
            result,
            Err(ManifestError::PartitionMismatch { .. })
        ));
    }

    #[test]
    fn intact_artifact_bytes_are_accepted() {
        let body = sample_body();
        let intact = b"fake init_boot_a image bytes";
        assert!(verify_partition_bytes(&body, "init_boot_a", intact).is_ok());
    }

    #[test]
    fn partition_missing_from_manifest_is_rejected() {
        let body = sample_body();
        let result = verify_partition_bytes(&body, "vendor_boot_a", b"anything");
        assert_eq!(
            result,
            Err(ManifestError::PartitionNotInManifest(
                "vendor_boot_a".to_string()
            ))
        );
    }

    #[test]
    fn canonical_bytes_are_stable_across_serialize_round_trips() {
        let body = sample_body();
        let once = body.canonical_bytes();
        let round_tripped: ManifestBody = serde_json::from_slice(&once).unwrap();
        let twice = round_tripped.canonical_bytes();
        assert_eq!(once, twice);
    }
}
