//! Presentation-only, finite transitions. Durable visibility always lives in core.
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc, time::Duration};

pub const PANE_DURATION: Duration = Duration::from_millis(180);
pub const SELECTION_DURATION: Duration = Duration::from_millis(120);

pub struct PaneMotion {
    from: f32,
    target: f32,
    generation: usize,
    current: Rc<Cell<f32>>,
}
impl PaneMotion {
    pub fn new(size: f32) -> Self {
        Self {
            from: size,
            target: size,
            generation: 0,
            current: Rc::new(Cell::new(size)),
        }
    }
    pub fn retarget(&mut self, size: f32, animate: bool) {
        if self.target == size {
            return;
        }
        self.from = if animate { self.current.get() } else { size };
        self.target = size;
        self.generation += 1;
        if !animate {
            self.current.set(size);
        }
    }
    pub fn extent(&self) -> f32 {
        self.from.max(self.target)
    }
    pub fn finish(&mut self) {
        self.from = self.target;
        self.current.set(self.target);
        self.generation += 1;
    }
    pub fn view(
        &self,
        id: &'static str,
        vertical: bool,
        reduced: bool,
        child: impl IntoElement,
    ) -> AnyElement {
        if self.target == 0. && self.current.get() < 0.01 {
            return div().into_any_element();
        }
        let from = self.from;
        let target = self.target;
        let current = self.current.clone();
        let base = div().id(id).flex_shrink_0().overflow_hidden().child(child);
        let size = move |el: Stateful<Div>, v: f32| {
            current.set(v);
            if vertical {
                el.h(px(v)).w_full()
            } else {
                el.w(px(v)).h_full()
            }
        };
        if reduced || (from - target).abs() < 0.1 {
            size(base, target).into_any_element()
        } else {
            base.with_animation(
                (id, self.generation),
                Animation::new(PANE_DURATION).with_easing(ease_in_out),
                move |el, delta| size(el, from + (target - from) * delta),
            )
            .into_any_element()
        }
    }
}
