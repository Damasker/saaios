//! SHM slot policy for lock (ADR-532), OSK, and the status bar
//! (ADR-534).
//!
//! `9551b854` died SIGTRAP after the first lock frame because the pixel7
//! build uses `panic_immediate_abort` and the lock `SlotPool` was sized
//! for one 1080×2400 buffer. OSK and the status-bar SHM fallback used
//! the same one-slot pool and cleared the buffer on every configure.
//! dest-no-lock stays; this module does not lift it.

/// Two slots so a busy first lock frame cannot exhaust the pool.
pub const LOCK_SHM_SLOTS: usize = 2;

const PAGE: usize = 4096;

pub fn lock_shm_frame_bytes(width: u32, height: u32) -> usize {
    (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(4)
}

/// Page-align each slot. Kernel mmap length is rounded up; a bare
/// `2 * width * height * 4` is short of two aligned frames.
pub fn lock_shm_pool_bytes(width: u32, height: u32) -> usize {
    lock_shm_frame_bytes(width, height)
        .next_multiple_of(PAGE)
        .saturating_mul(LOCK_SHM_SLOTS)
}

/// Drop the SHM pool and buffer only when the lock surface size changes.
/// A same-size configure must keep the live buffer so the second frame
/// can paint or wait instead of allocating into a one-slot pool.
pub fn lock_size_replaced(
    old_width: u32,
    old_height: u32,
    width: u32,
    height: u32,
) -> bool {
    old_width != width || old_height != height
}

#[cfg(test)]
mod tests {
    #[test]
    fn pool_holds_two_page_aligned_frames() {
        let frame = super::lock_shm_frame_bytes(1080, 2400);
        assert_eq!(frame, 10_368_000);
        let pool = super::lock_shm_pool_bytes(1080, 2400);
        assert_eq!(super::LOCK_SHM_SLOTS, 2);
        assert!(pool >= frame * 2);
        assert_eq!(pool, frame.next_multiple_of(4096) * 2);
        assert!(pool > frame * 2);
    }

    #[test]
    fn first_configure_replaces_zero_size() {
        assert!(super::lock_size_replaced(0, 0, 1080, 2400));
    }

    #[test]
    fn same_size_configure_keeps_the_live_buffer() {
        assert!(!super::lock_size_replaced(1080, 2400, 1080, 2400));
    }

    #[test]
    fn new_size_replaces_pool_and_buffer() {
        assert!(super::lock_size_replaced(1080, 2400, 1080, 120));
        assert!(super::lock_size_replaced(720, 2400, 1080, 2400));
    }

    #[test]
    fn status_bar_pool_holds_two_page_aligned_frames() {
        let frame = super::lock_shm_frame_bytes(1080, 120);
        assert_eq!(frame, 518_400);
        let pool = super::lock_shm_pool_bytes(1080, 120);
        assert!(pool >= frame * 2);
        assert_eq!(pool, frame.next_multiple_of(4096) * 2);
    }
}
