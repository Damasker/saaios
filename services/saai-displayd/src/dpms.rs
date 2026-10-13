//! Connector DPMS for panther scanout (ADR-565).
//!
//! A successful modeset that leaves `card0-DSI-1/dpms` at Off keeps the
//! panel black. drm-splash's `SETCRTC` implies On. displayd must write
//! On whenever it presents. Sysfs `dpms` is not writable from ash.

pub const DPMS_PROPERTY: &str = "DPMS";

/// `DRM_MODE_DPMS_ON`. Off is 3; do not write that from present.
pub const DRM_MODE_DPMS_ON: u64 = 0;

pub fn is_dpms_property(name: &str) -> bool {
    name == DPMS_PROPERTY
}

#[cfg(test)]
mod tests {
    #[test]
    fn dpms_property_name_is_the_kernel_name() {
        assert!(super::is_dpms_property("DPMS"));
        assert!(!super::is_dpms_property("CRTC_ID"));
        assert!(!super::is_dpms_property("dpms"));
    }

    #[test]
    fn present_writes_on_not_off() {
        assert_eq!(super::DRM_MODE_DPMS_ON, 0);
        assert_ne!(super::DRM_MODE_DPMS_ON, 3);
    }
}
