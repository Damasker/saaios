use hdi_campaign::{Hunter, Investigator};
use hdi_contract::ClaimKind;

#[test]
fn missing_vga_class_is_one_absence() {
    let input = r#"{"files":{"pci.txt":"0000:00:00.0 PCI bridge [0604]: [1234:0001]\n"}}"#;
    let claims = Hunter::new().observe(input);
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].kind, ClaimKind::DeviceAbsent);
    assert_eq!(claims[0].subject, "pci:class:0300");
    assert_eq!(claims[0].value, "0300");
    assert_eq!(claims[0].evidence_ids, vec!["pci.txt".to_string()]);
}

#[test]
fn present_vga_class_adds_no_claim() {
    let input =
        r#"{"files":{"pci.txt":"0000:01:00.0 VGA compatible controller [0300]: [10de:0fe9]\n"}}"#;
    let claims = Hunter::new().observe(input);
    assert!(claims.is_empty());
}

#[test]
fn absence_does_not_name_a_gpu() {
    let input = r#"{"files":{"pci.txt":"0000:00:00.0 PCI bridge [0604]: [144d:ecec]\n"}}"#;
    let claims = Hunter::new().observe(input);
    let text = format!("{} {}", claims[0].subject, claims[0].value);
    assert!(!text.contains("8086"));
    assert!(!text.contains("Iris"));
    assert!(!text.contains(".ko"));
}
