use std::path::Path;

use hdi_campaign::{run_campaign, verify, Hunter};
use hdi_contract::Claim;

fn pixel() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pixel-graphics")
}

fn pixel_key() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pixel-key.txt")
}

#[test]
fn pixel_key_accuracy_is_one_and_stays_out_of_the_bundle() {
    let campaign = tempfile::tempdir().expect("campaign");
    let mut hunter = Hunter::new();
    let report = run_campaign(
        pixel().as_path(),
        campaign.path(),
        &["pci.txt"],
        &mut hunter,
    )
    .unwrap();
    assert!(report
        .inputs
        .iter()
        .all(|input| !input.contains("expected_model")));
    let verification = verify(pixel().as_path(), &pixel_key()).unwrap();
    let scored = report
        .metrics
        .with_stand_key(&report.claims, &verification.confirmed_for("pixel-7"));
    assert_eq!(scored.identification_accuracy, 1.0);
    assert_eq!(
        report
            .metrics
            .with_stand_key(&report.claims, &[])
            .identification_accuracy,
        0.0
    );
    let macbook = campaign.path().join("macbook-key.txt");
    std::fs::write(
        &macbook,
        "machine_id: machine-a\nexpected_product: MacBookPro11,3\nigd_id: 8086:0d26\n",
    )
    .unwrap();
    let other = verify(pixel().as_path(), &macbook).unwrap();
    let confirmed: Vec<Claim> = other.confirmed_for("machine-a");
    assert!(!confirmed.is_empty());
    assert_eq!(
        report
            .metrics
            .with_stand_key(&report.claims, &confirmed)
            .identification_accuracy,
        0.0
    );
}
