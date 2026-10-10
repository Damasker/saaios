/// Static read for one whitelist probe. Arguments are never interpolated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeTemplate {
    pub name: &'static str,
    pub argv: &'static [&'static str],
}

const TEMPLATES: &[ProbeTemplate] = &[
    ProbeTemplate {
        name: "pci_id",
        argv: &["read-fixture", "pci.txt"],
    },
    ProbeTemplate {
        name: "pci_drivers",
        argv: &["read-fixture", "pci_drivers.txt"],
    },
    ProbeTemplate {
        name: "pci_tree",
        argv: &["read-fixture", "pci_tree.txt"],
    },
    ProbeTemplate {
        name: "pci_slot",
        argv: &["read-fixture", "pci.txt"],
    },
    ProbeTemplate {
        name: "usb_brief",
        argv: &["read-fixture", "usb.txt"],
    },
    ProbeTemplate {
        name: "usb_tree",
        argv: &["read-fixture", "usb_tree.txt"],
    },
    ProbeTemplate {
        name: "modules",
        argv: &["read-fixture", "modules.txt"],
    },
    ProbeTemplate {
        name: "drm_class",
        argv: &["read-fixture", "drm.txt"],
    },
    ProbeTemplate {
        name: "pci_sysfs_list",
        argv: &["read-fixture", "pci_sysfs.txt"],
    },
    ProbeTemplate {
        name: "dmi_allowlist",
        argv: &["read-fixture", "dmi.txt"],
    },
    ProbeTemplate {
        name: "platform_nodes",
        argv: &["read-fixture", "platform.txt"],
    },
    ProbeTemplate {
        name: "kernel_warnings_redacted",
        argv: &["read-fixture", "kernel_warnings.txt"],
    },
    ProbeTemplate {
        name: "sysfs_read",
        argv: &["read-fixture-sysfs"],
    },
];

pub fn probe_templates() -> &'static [ProbeTemplate] {
    TEMPLATES
}
