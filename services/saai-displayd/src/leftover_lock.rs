//! Dead lock-client policy for dest-no-lock (ADR-510).
//!
//! A gone `ext-session-lock-v1` client must not keep routing touch to
//! a stale lock surface when dest-no-lock asked to skip session lock.
//! Without the marker the session stays locked.

/// Same path as `saai-shell`. `/run` is tmpfs — reboot drops it.
pub const DEV_NO_LOCK_MARKER: &str = "/run/saaios/dev-no-lock";

pub fn dest_no_lock_marker_present() -> bool {
    std::path::Path::new(DEV_NO_LOCK_MARKER).exists()
}

pub fn should_unlock_dead_lock_client(locked: bool, dest_no_lock: bool) -> bool {
    locked && dest_no_lock
}

#[cfg(test)]
mod tests {
    #[test]
    fn dest_no_lock_clears_dead_lock_client() {
        assert!(super::should_unlock_dead_lock_client(true, true));
    }

    #[test]
    fn real_lock_survives_dead_client() {
        assert!(!super::should_unlock_dead_lock_client(true, false));
    }

    #[test]
    fn unlocked_session_stays_unlocked() {
        assert!(!super::should_unlock_dead_lock_client(false, true));
        assert!(!super::should_unlock_dead_lock_client(false, false));
    }

    #[test]
    fn dest_no_lock_marker_path_matches_shell() {
        assert_eq!(super::DEV_NO_LOCK_MARKER, "/run/saaios/dev-no-lock");
        let _ = super::dest_no_lock_marker_present();
    }
}
