#[derive(Clone, Debug)]
pub struct OverlayView {
    pub lines: Vec<String>,
    pub selected: Option<usize>,
    pub scroll: usize,
    pub height: usize,
}

impl OverlayView {
    pub fn new(lines: Vec<String>, height: usize) -> Self {
        Self {
            lines,
            selected: None,
            scroll: 0,
            height,
        }
    }

    pub fn move_selection(&mut self, delta: isize) {
        if self.lines.is_empty() {
            self.selected = None;
            return;
        }
        let maximum = self.lines.len().saturating_sub(1) as isize;
        let selected = match self.selected {
            Some(current) => current as isize + delta,
            None => 0,
        };
        self.selected = Some(selected.clamp(0, maximum) as usize);
        self.keep_visible();
    }

    pub fn page(&mut self, direction: isize) {
        let step = self.height.max(1) as isize;
        self.move_selection(direction.saturating_mul(step));
    }

    pub fn select(&mut self, index: usize) {
        if self.lines.is_empty() {
            self.selected = None;
            return;
        }
        self.selected = Some(index.min(self.lines.len() - 1));
        self.keep_visible();
    }

    pub fn go_top(&mut self) {
        self.select(0);
    }

    pub fn go_bottom(&mut self) {
        if !self.lines.is_empty() {
            self.select(self.lines.len() - 1);
        }
    }

    fn keep_visible(&mut self) {
        // Scroll only when the selection leaves the window. Otherwise the
        // view chases its own tail.
        let Some(selected) = self.selected else {
            return;
        };
        if selected < self.scroll {
            self.scroll = selected;
        } else if selected >= self.scroll.saturating_add(self.height) {
            self.scroll = selected.saturating_add(1).saturating_sub(self.height);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> OverlayView {
        OverlayView::new((0..10).map(|index| format!("line {index}")).collect(), 3)
    }

    #[test]
    fn shows_the_first_page() {
        let view = view();
        assert_eq!(view.lines.len(), 10);
    }

    #[test]
    fn selection_scrolls_into_view() {
        let mut view = view();
        view.move_selection(1);
        assert_eq!(view.selected, Some(0));
        view.move_selection(5);
        assert_eq!(view.selected, Some(5));
        assert_eq!(view.scroll, 3);
    }

    #[test]
    fn clamps_at_both_ends() {
        let mut view = view();
        view.move_selection(-5);
        assert_eq!(view.selected, Some(0));
        view.go_bottom();
        assert_eq!(view.selected, Some(9));
        assert_eq!(view.scroll, 7);
        view.move_selection(100);
        assert_eq!(view.selected, Some(9));
    }

    #[test]
    fn pages_by_the_visible_height() {
        let mut view = view();
        view.go_top();
        view.page(1);
        assert_eq!(view.selected, Some(3));
        view.page(-1);
        assert_eq!(view.selected, Some(0));
    }

    #[test]
    fn handles_empty_and_zero_height_views() {
        let mut empty = OverlayView::new(Vec::new(), 0);
        empty.move_selection(1);
        empty.go_bottom();
        assert_eq!(empty.selected, None);

        let mut view = view();
        view.height = 0;
        view.select(2);
        assert_eq!(view.selected, Some(2));
    }
}
