mod connection;
mod definitions;
mod inspector;
mod output;
mod sidebar;
mod toolbar;

use crate::{app::Keyforge, components::*};
use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme, IconName, Selectable,
    button::{Button, ButtonVariants},
};
use keyforge_core::{Command, Destination, Pane};

impl Keyforge {
    pub(crate) fn button(
        &self,
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        command: Command,
        cx: &mut Context<Self>,
    ) -> Button {
        Button::new(id.into())
            .ghost()
            .label(label)
            .h(px(36.))
            .text_size(px(13.))
            .on_click(cx.listener(move |this, _, w, cx| this.command(command.clone(), w, cx)))
    }
    pub(crate) fn icon_button(
        &self,
        id: &'static str,
        icon: IconName,
        label: &'static str,
        command: Command,
        cx: &mut Context<Self>,
    ) -> Button {
        Button::new(id)
            .ghost()
            .icon(icon)
            .tooltip(label)
            .w(px(36.))
            .h(px(34.))
            .on_click(cx.listener(move |this, _, w, cx| this.command(command.clone(), w, cx)))
    }
    pub(crate) fn workspace(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        #[cfg(feature = "gallery")]
        if self.gallery {
            return self.gallery_view(cx).into_any_element();
        }
        let reduced = self.core.workspace.preferences.reduced_motion;
        let viewport = window.viewport_size();
        let sizes = self
            .core
            .workspace
            .rendered(viewport.width.into(), f32::from(viewport.height) - 92.);
        let compact = f32::from(viewport.width) - sizes.left - sizes.right < 600.;
        let mut center = div().flex_1().min_w_0().min_h_0().flex().flex_col();
        if self.core.destination == Destination::Keymap {
            center = center
                .child(self.keymap_header(cx))
                .child(crate::keyboard::view(self, cx))
                .child(
                    div()
                        .h(px(44.))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(12.))
                        .text_color(cx.theme().muted_foreground)
                        .child(if compact && self.canvas_focus.is_focused(window) {
                            "Arrows to navigate · Esc to deselect · 0 to fit"
                        } else if compact {
                            "Scroll to zoom · middle drag to pan"
                        } else if self.canvas_focus.is_focused(window) {
                            "Arrow keys to navigate · Esc to deselect · 0 to fit"
                        } else {
                            "Select a key to edit its binding · Middle drag to pan · Scroll to zoom"
                        }),
                );
        } else if self.core.destination == Destination::Settings {
            center = center.child(self.preferences(cx));
        } else if self.core.destination == Destination::Repository {
            center = center.child(self.connection_screen(cx));
        } else if let Some(section) = keyforge_core::live::section(self.core.destination) {
            center = center.child(self.definitions_screen(section, compact, cx));
        } else {
            center = center.child(self.connection_screen(cx));
        }

        let bottom = self.panes[2].extent();
        center = center.child(
            self.panes[2].view(
                "bottom-motion",
                true,
                reduced,
                div()
                    .relative()
                    .h(px(bottom))
                    .child(self.output(bottom, cx))
                    .child(self.separator(Pane::Bottom, cx)),
            ),
        );
        div()
            .flex()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .child(
                self.panes[0].view(
                    "left-motion",
                    false,
                    reduced,
                    div()
                        .relative()
                        .h_full()
                        .w(px(self.panes[0].extent()))
                        .child(self.sidebar(self.panes[0].extent(), cx))
                        .child(self.separator(Pane::Left, cx)),
                ),
            )
            .child(center)
            .child(
                self.panes[1].view(
                    "right-motion",
                    false,
                    reduced,
                    div()
                        .relative()
                        .h_full()
                        .w(px(self.panes[1].extent()))
                        .child(self.inspector(self.panes[1].extent(), cx))
                        .child(self.separator(Pane::Right, cx)),
                ),
            )
            .into_any_element()
    }
    fn separator(&self, pane: Pane, cx: &mut Context<Self>) -> Stateful<Div> {
        div()
            .id(SharedString::from(format!("resize-{pane:?}")))
            .absolute()
            .when(pane == Pane::Bottom, |el| {
                el.top_0().left_0().h(px(6.)).w_full().cursor_row_resize()
            })
            .when(pane != Pane::Bottom, |el| {
                el.top_0().h_full().w(px(6.)).cursor_col_resize()
            })
            .when(pane == Pane::Left, |el| el.right_0())
            .when(pane == Pane::Right, |el| el.left_0())
            .hover(|s| s.bg(cx.theme().primary.opacity(0.35)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, e: &MouseDownEvent, w, cx| {
                    let sizes = this.core.workspace.rendered(
                        w.viewport_size().width.into(),
                        f32::from(w.viewport_size().height) - 92.,
                    );
                    // Drag begins at the constrained on-screen edge, not an off-screen preference.
                    let size = match pane {
                        Pane::Left => sizes.left,
                        Pane::Right => sizes.right,
                        Pane::Bottom => sizes.bottom,
                    };
                    this.resize = Some((pane, e.position, size));
                    cx.stop_propagation();
                }),
            )
    }
    fn keymap_header(&self, cx: &mut Context<Self>) -> Div {
        let layer = &self.core.project.keymap.layers[self.core.active_layer.0];
        div()
            .px_7()
            .pt_6()
            .pb_2()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(26.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(self.core.project.keyboard.name.clone()),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("zoom-out")
                            .ghost()
                            .icon(IconName::Minus)
                            .tooltip("Zoom out")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.camera.zoom_by(1. / 1.15);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("fit")
                            .ghost()
                            .label(format!("{:.0}%", self.camera.zoom * 100.))
                            .tooltip("Fit keyboard · 0 when canvas focused")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.camera = Default::default();
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("zoom-in")
                            .ghost()
                            .icon(IconName::Plus)
                            .tooltip("Zoom in")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.camera.zoom_by(1.15);
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(cx.theme().muted_foreground)
                    .child(format!(
                        "{} layer · {} · {} keys",
                        layer.name,
                        self.core.project.name,
                        self.core.project.keyboard.keys.len()
                    )),
            )
            .when(!self.core.workspace.visible(Pane::Left), |el| {
                el.child(self.layer_controls(cx))
            })
    }
    fn preferences(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        use keyforge_core::workspace::Appearance;
        let appearance = div().flex().flex_wrap().gap_2().children(
            [
                (Appearance::System, "System"),
                (Appearance::Light, "Light"),
                (Appearance::Dark, "Dark"),
            ]
            .map(|(mode, label)| {
                self.button(
                    format!("theme-{label}"),
                    label,
                    Command::SetAppearance(mode),
                    cx,
                )
                .selected(self.core.workspace.preferences.appearance == mode)
            }),
        );
        let motion = self
            .button(
                "reduced-motion",
                if self.core.workspace.preferences.reduced_motion {
                    "On"
                } else {
                    "Off"
                },
                Command::SetReducedMotion(!self.core.workspace.preferences.reduced_motion),
                cx,
            )
            .with_variant(gpui_component::button::ButtonVariant::Secondary)
            .selected(self.core.workspace.preferences.reduced_motion)
            .w(px(100.));
        let folder = self
            .core
            .workspace
            .preferences
            .local_keymap_directory
            .as_ref();
        let choose = Button::new("local-keymap-directory")
            .w(px(180.))
            .h(px(36.))
            .label("Choose folder…")
            .icon(IconName::FolderOpen)
            .on_click(cx.listener(|this, _, window, cx| {
                this.browse_path(
                    crate::editor_controls::PathAction::LocalKeymapDirectory,
                    window,
                    cx,
                )
            }));
        let mut location = div().flex().flex_col().gap_3().child(
            div().text_size(px(13.)).child(
                folder
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "Use the opened keymap's location".into()),
            ),
        );
        if folder.is_some() {
            location = location.child(ActionRow::new(
                choose,
                self.button(
                    "use-source-location",
                    "Use source location",
                    Command::SetLocalKeymapDirectory(None),
                    cx,
                ),
            ));
        } else {
            location = location.child(choose);
        }
        div().id("preferences").flex_1().min_w_0().min_h_0().overflow_y_scroll().p_6().flex().flex_col().gap_3()
            .child(div().text_size(px(26.)).font_weight(FontWeight::SEMIBOLD).child("Preferences"))
            .child(div().text_color(cx.theme().muted_foreground).child("Device-local settings. These do not modify your keymap."))
            .child(setting_row("Theme", "Match the system or choose a fixed appearance.", appearance, cx))
            .child(setting_row("Reduced motion", "Disable pane and selection transitions.", motion, cx))
            .child(setting_row("Local keymap save directory", "Changing this folder creates a configuration copy on your next local save; the original stays intact.", location, cx))
            .child(self.build_setup_panel(cx))
            .child(div().mt_4().text_size(px(12.)).text_color(cx.theme().muted_foreground).child("Preferences save automatically on this device."))
    }
}
