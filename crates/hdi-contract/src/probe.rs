use serde::Deserialize;
use serde_json::{Map, Value};

use crate::error::ContractError;

const PCI_PREFIX: &str = "/sys/bus/pci/devices/";
const DRM_PREFIX: &str = "/sys/class/drm/";
const DMI_PREFIX: &str = "/sys/class/dmi/id/";
const DMI_NAMES: &[&str] = &[
    "product_name",
    "product_version",
    "board_name",
    "bios_version",
    "bios_date",
    "sys_vendor",
];

/// Read-only probe names from ADR-426. `apple_gmux` is intentionally absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeName {
    PciId,
    PciDrivers,
    PciTree,
    PciSlot,
    UsbBrief,
    UsbTree,
    Modules,
    DrmClass,
    PciSysfsList,
    DmiAllowlist,
    PlatformNodes,
    PlatformDrivers,
    KernelWarningsRedacted,
    SysfsRead,
}

impl ProbeName {
    fn expects_arguments(self) -> &'static [&'static str] {
        match self {
            Self::PciSlot => &["slot"],
            Self::SysfsRead => &["path"],
            Self::PciId
            | Self::PciDrivers
            | Self::PciTree
            | Self::UsbBrief
            | Self::UsbTree
            | Self::Modules
            | Self::DrmClass
            | Self::PciSysfsList
            | Self::DmiAllowlist
            | Self::PlatformNodes
            | Self::PlatformDrivers
            | Self::KernelWarningsRedacted => &[],
        }
    }
}

/// JSON shape before the probe name is checked against the whitelist.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProbe {
    campaign_id: String,
    pass: u32,
    #[serde(default)]
    device: Option<String>,
    #[serde(default)]
    hypothesis: Option<String>,
    requested_probe: String,
    arguments: Map<String, Value>,
    risk: String,
}

/// Structured investigator request. The executor is not invoked here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeRequest {
    pub campaign_id: String,
    pub pass: u32,
    pub device: Option<String>,
    pub hypothesis: Option<String>,
    pub requested_probe: ProbeName,
    pub arguments: Map<String, Value>,
    pub risk: String,
}

impl ProbeName {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PciId => "pci_id",
            Self::PciDrivers => "pci_drivers",
            Self::PciTree => "pci_tree",
            Self::PciSlot => "pci_slot",
            Self::UsbBrief => "usb_brief",
            Self::UsbTree => "usb_tree",
            Self::Modules => "modules",
            Self::DrmClass => "drm_class",
            Self::PciSysfsList => "pci_sysfs_list",
            Self::DmiAllowlist => "dmi_allowlist",
            Self::PlatformNodes => "platform_nodes",
            Self::PlatformDrivers => "platform_drivers",
            Self::KernelWarningsRedacted => "kernel_warnings_redacted",
            Self::SysfsRead => "sysfs_read",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "pci_id" => Self::PciId,
            "pci_drivers" => Self::PciDrivers,
            "pci_tree" => Self::PciTree,
            "pci_slot" => Self::PciSlot,
            "usb_brief" => Self::UsbBrief,
            "usb_tree" => Self::UsbTree,
            "modules" => Self::Modules,
            "drm_class" => Self::DrmClass,
            "pci_sysfs_list" => Self::PciSysfsList,
            "dmi_allowlist" => Self::DmiAllowlist,
            "platform_nodes" => Self::PlatformNodes,
            "platform_drivers" => Self::PlatformDrivers,
            "kernel_warnings_redacted" => Self::KernelWarningsRedacted,
            "sysfs_read" => Self::SysfsRead,
            _ => return None,
        })
    }
}

impl ProbeRequest {
    pub fn parse(raw: &str) -> Result<Self, ContractError> {
        let raw: RawProbe = serde_json::from_str(raw).map_err(|_| ContractError::InvalidProbe)?;
        let requested_probe =
            ProbeName::from_name(&raw.requested_probe).ok_or(ContractError::UnknownProbe)?;
        let request = Self {
            campaign_id: raw.campaign_id,
            pass: raw.pass,
            device: raw.device,
            hypothesis: raw.hypothesis,
            requested_probe,
            arguments: raw.arguments,
            risk: raw.risk,
        };
        request.validate()?;
        Ok(request)
    }

    fn validate(&self) -> Result<(), ContractError> {
        if self.campaign_id.is_empty() {
            return Err(ContractError::InvalidProbe);
        }
        if self.risk != "read_only" {
            return Err(ContractError::RiskNotReadOnly);
        }
        self.arguments.values().try_for_each(reject_shell_values)?;
        if let Some(device) = &self.device {
            reject_shell_text(device)?;
        }
        validate_arguments(self.requested_probe, &self.arguments)?;
        Ok(())
    }
}

fn validate_arguments(
    probe: ProbeName,
    arguments: &Map<String, Value>,
) -> Result<(), ContractError> {
    let expected = probe.expects_arguments();
    if arguments.len() != expected.len() || expected.iter().any(|key| !arguments.contains_key(*key))
    {
        return Err(ContractError::InvalidArguments);
    }
    match probe {
        ProbeName::PciSlot => {
            let slot = expect_string(arguments.get("slot"))?;
            if slot.contains("..") {
                return Err(ContractError::PathEscape);
            }
            if !is_pci_slot(slot) {
                return Err(ContractError::InvalidArguments);
            }
        }
        ProbeName::SysfsRead => {
            let path = expect_string(arguments.get("path"))?;
            validate_sysfs_path(path)?;
        }
        _ => {}
    }
    Ok(())
}

fn expect_string(value: Option<&Value>) -> Result<&str, ContractError> {
    value
        .and_then(Value::as_str)
        .ok_or(ContractError::InvalidArguments)
}

fn is_pci_slot(slot: &str) -> bool {
    let mut parts = slot.split(':');
    let Some(domain) = parts.next() else {
        return false;
    };
    let Some(bus) = parts.next() else {
        return false;
    };
    let Some(rest) = parts.next() else {
        return false;
    };
    if parts.next().is_some() {
        return false;
    }
    let mut dev_fn = rest.split('.');
    let Some(device) = dev_fn.next() else {
        return false;
    };
    let Some(function) = dev_fn.next() else {
        return false;
    };
    dev_fn.next().is_none()
        && is_hex(domain, 4)
        && is_hex(bus, 2)
        && is_hex(device, 2)
        && function.len() == 1
        && function.as_bytes()[0].is_ascii_digit()
        && function.as_bytes()[0] <= b'7'
}

fn is_hex(text: &str, len: usize) -> bool {
    text.len() == len && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_sysfs_path(path: &str) -> Result<(), ContractError> {
    if path.contains("..") || path.contains('\\') || path.contains('\0') {
        return Err(ContractError::PathEscape);
    }
    if let Some(name) = path.strip_prefix(DMI_PREFIX) {
        if DMI_NAMES.contains(&name) {
            return Ok(());
        }
        return Err(ContractError::PathEscape);
    }
    if under_prefix(path, PCI_PREFIX) || under_prefix(path, DRM_PREFIX) {
        return Ok(());
    }
    Err(ContractError::PathEscape)
}

fn under_prefix(path: &str, prefix: &str) -> bool {
    path.starts_with(prefix)
        && !path[prefix.len()..].is_empty()
        && !path[prefix.len()..].starts_with('/')
}

fn reject_shell_values(value: &Value) -> Result<(), ContractError> {
    match value {
        Value::String(text) => reject_shell_text(text),
        Value::Array(items) => items.iter().try_for_each(reject_shell_values),
        Value::Object(map) => map.values().try_for_each(reject_shell_values),
        Value::Null | Value::Bool(_) | Value::Number(_) => Ok(()),
    }
}

fn reject_shell_text(text: &str) -> Result<(), ContractError> {
    if text.contains(';')
        || text.contains("&&")
        || text.contains('\n')
        || text.contains('\r')
        || text.contains('"')
        || text.contains('\'')
    {
        return Err(ContractError::ShellMetacharacters);
    }
    Ok(())
}
