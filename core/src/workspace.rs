use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pane {
    Left,
    Right,
    Bottom,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BottomTab {
    BuildOutput,
    Problems,
    Changes,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Appearance {
    #[default]
    System,
    Dark,
    Light,
}
impl BottomTab {
    pub const ALL: [Self; 3] = [Self::BuildOutput, Self::Problems, Self::Changes];
}
impl std::fmt::Display for BottomTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::BuildOutput => "Build Output",
            Self::Problems => "Problems",
            Self::Changes => "Changes",
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PanePreference {
    pub visible: bool,
    pub preferred_size: f32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Preferences {
    pub schema_version: u32,
    pub left: PanePreference,
    pub right: PanePreference,
    pub bottom: PanePreference,
    pub bottom_tab: BottomTab,
    pub window_width: f32,
    pub window_height: f32,
    #[serde(default)]
    pub appearance: Appearance,
    #[serde(default)]
    pub reduced_motion: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            schema_version: 1,
            left: PanePreference {
                visible: true,
                preferred_size: 208.,
            },
            right: PanePreference {
                visible: true,
                preferred_size: 320.,
            },
            bottom: PanePreference {
                visible: true,
                preferred_size: 222.,
            },
            bottom_tab: BottomTab::BuildOutput,
            window_width: 1440.,
            window_height: 900.,
            appearance: Appearance::System,
            reduced_motion: false,
        }
    }
}
impl Preferences {
    pub fn pane(&self, p: Pane) -> &PanePreference {
        match p {
            Pane::Left => &self.left,
            Pane::Right => &self.right,
            Pane::Bottom => &self.bottom,
        }
    }
    pub fn pane_mut(&mut self, p: Pane) -> &mut PanePreference {
        match p {
            Pane::Left => &mut self.left,
            Pane::Right => &mut self.right,
            Pane::Bottom => &mut self.bottom,
        }
    }
    pub fn sanitize(&mut self) {
        for p in [Pane::Left, Pane::Right, Pane::Bottom] {
            let v = self.pane_mut(p);
            let (min, max) = limits(p);
            v.preferred_size = if v.preferred_size.is_finite() {
                v.preferred_size.clamp(min, max)
            } else {
                min
            };
        }
        self.window_width = if self.window_width.is_finite() {
            self.window_width.clamp(900., 7680.)
        } else {
            1440.
        };
        self.window_height = if self.window_height.is_finite() {
            self.window_height.clamp(600., 4320.)
        } else {
            900.
        };
    }
}
pub fn limits(p: Pane) -> (f32, f32) {
    match p {
        Pane::Left => (140., 400.),
        Pane::Right => (240., 520.),
        Pane::Bottom => (100., 500.),
    }
}
#[derive(Debug, Clone, Default)]
pub struct Workspace {
    pub preferences: Preferences,
    pub focus_mode: bool,
}
#[derive(Debug, Clone, Copy)]
pub struct RenderedPanes {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
}
impl Workspace {
    pub fn visible(&self, p: Pane) -> bool {
        !self.focus_mode && self.preferences.pane(p).visible
    }
    pub fn toggle(&mut self, p: Pane) {
        self.focus_mode = false;
        let v = self.preferences.pane_mut(p);
        v.visible = !v.visible;
    }
    pub fn resize(&mut self, p: Pane, size: f32) {
        if !self.focus_mode && size.is_finite() {
            let (min, max) = limits(p);
            self.preferences.pane_mut(p).preferred_size = size.clamp(min, max);
        }
    }
    pub fn rendered(&self, width: f32, height: f32) -> RenderedPanes {
        let mut left = if self.visible(Pane::Left) {
            self.preferences.left.preferred_size
        } else {
            0.
        };
        let mut right = if self.visible(Pane::Right) {
            self.preferences.right.preferred_size
        } else {
            0.
        };
        let budget = (width - 340.).max(0.);
        if left + right > budget {
            let scale = budget / (left + right);
            left *= scale;
            right *= scale;
        }
        let bottom = if self.visible(Pane::Bottom) {
            self.preferences
                .bottom
                .preferred_size
                .min((height - 220.).max(0.))
        } else {
            0.
        };
        RenderedPanes {
            left,
            right,
            bottom,
        }
    }
    pub fn reset(&mut self) {
        let window_width = self.preferences.window_width;
        let window_height = self.preferences.window_height;
        self.preferences = Preferences {
            window_width,
            window_height,
            appearance: self.preferences.appearance,
            reduced_motion: self.preferences.reduced_motion,
            ..Preferences::default()
        };
        self.focus_mode = false;
    }
}
