use hdi_campaign::{Hunter, Investigator};
use hdi_contract::ClaimKind;

#[test]
fn driver_module_is_a_separate_fact() {
    let input = r#"{"files":{"platform.txt":"28000000.mali\n19000000.aoc\n100b0000.TPU\n","modules.txt":"mali_kbase\naoc_core\n","platform_drivers.txt":"28000000.mali mali mali_kbase\n19000000.aoc aoc aoc_core\n100b0000.TPU unbound\n28000000.mali mali mali_pixel\n"}}"#;
    let claims = Hunter::new().observe(input);
    assert!(claims.iter().any(|claim| {
        claim.kind == ClaimKind::DriverBound
            && claim.subject == "platform:mali"
            && claim.value == "mali"
            && claim.evidence_ids == ["platform_drivers.txt"]
    }));
    assert!(claims.iter().any(|claim| {
        claim.kind == ClaimKind::DriverBound
            && claim.subject == "driver:mali"
            && claim.value == "mali_kbase"
            && claim.evidence_ids == ["platform_drivers.txt", "modules.txt"]
    }));
    assert!(claims.iter().any(|claim| {
        claim.kind == ClaimKind::DriverBound
            && claim.subject == "driver:aoc"
            && claim.value == "aoc_core"
    }));
    assert!(claims
        .iter()
        .all(|claim| claim.subject != "driver:mali" || claim.value != "mali_pixel"));
    assert!(claims
        .iter()
        .all(|claim| claim.subject != "platform:mali" || claim.value != "mali_kbase"));
}
