#[derive(Debug, Default)]
pub struct PresentationState {
    dirty: bool,
    flip_pending: bool,
}

impl PresentationState {
    pub fn request_repaint(&mut self) {
        self.dirty = true;
    }

    pub fn can_render(&self) -> bool {
        self.dirty && !self.flip_pending
    }

    pub fn queued(&mut self) {
        debug_assert!(self.can_render());
        self.dirty = false;
        self.flip_pending = true;
    }

    pub fn vblank(&mut self) {
        self.flip_pending = false;
    }
}

#[cfg(test)]
mod tests {
    use super::PresentationState;

    #[test]
    fn coalesces_commits_while_flip_is_pending() {
        let mut state = PresentationState::default();
        state.request_repaint();
        assert!(state.can_render());
        state.queued();
        state.request_repaint();
        state.request_repaint();
        assert!(!state.can_render());
        state.vblank();
        assert!(state.can_render());
    }

    #[test]
    fn a_presented_frame_is_clean_after_vblank() {
        let mut state = PresentationState::default();
        state.request_repaint();
        state.queued();
        state.vblank();
        assert!(!state.can_render());
    }
}
