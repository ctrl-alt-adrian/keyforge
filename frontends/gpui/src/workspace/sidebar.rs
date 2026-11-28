use crate::{app::Keyforge, components::*};
use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme, IconName, Selectable,
    button::{Button, ButtonVariants},
    input::Input,
};
use keyforge_core::{Command, Destination, LayerId};

impl Keyforge {
    pub(crate) fn layer_controls(&self, cx: &mut Context<Self>) -> Div {
        div().flex().flex_wrap().gap_1().children(
            self.core
                .project
                .keymap
                .layers
                .iter()
                .enumerate()
                .map(|(index, layer)| {
                    self.button(
                        format!("layer-inline-{index}"),
                        format!("{index}  {}", layer.name),
                        Command::SelectLayer(LayerId(index)),
                        cx,
                    )
                    .selected(self.core.active_layer.0 == index)
                }),
        )
    }
    pub(crate) fn sidebar(&self, width: f32, cx: &mut Context<Self>) -> Stateful<Div> {
        if self.core.project.zmk.is_none() {
            return div()
                .id("workspace-sidebar")
                .w(px(width))
                .h_full()
                .p_3()
                .bg(cx.theme().sidebar)
                .border_r_1()
                .border_color(cx.theme().sidebar_border)
                .flex()
                .flex_col()
                .gap_3()
                .child(caption("CONFIGURATION", cx).mt_2())
                .child(
                    self.button(
                        "nav-open",
                        "Open keymap",
                        Command::Navigate(Destination::Repository),
                        cx,
                    )
                    .icon(IconName::FolderOpen),
                )
                .child(
                    self.button(
                        "nav-preferences",
                        "Preferences",
                        Command::Navigate(Destination::Settings),
                        cx,
                    )
                    .icon(IconName::Settings),
                );
        }
        let mut layers = div().flex().flex_col().gap(px(6.));
        for (index, layer) in self.core.project.keymap.layers.iter().enumerate() {
            let id = LayerId(index);
            if let Some(draft) = self
                .core
                .layer_name_draft
                .as_ref()
                .filter(|d| d.layer == id)
            {
                let mut row = div().flex().flex_col().gap_1().child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(Input::new(&self.layer_name_input)),
                        )
                        .child(
                            self.icon_button(
                                "apply-layer-name",
                                IconName::Check,
                                "Finish layer name · Enter",
                                Command::ApplyLayerRename,
                                cx,
                            )
                            .w(px(24.)),
                        )
                        .child(
                            self.icon_button(
                                "cancel-layer-name",
                                IconName::Close,
                                "Close rename · Escape",
                                Command::CancelLayerRename,
                                cx,
                            )
                            .w(px(24.)),
                        )
                        .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                            if event.keystroke.key == "escape" {
                                this.command(Command::CancelLayerRename, window, cx);
                                this.canvas_focus.focus(window);
                                cx.stop_propagation();
                            }
                        })),
                );
                if let Some(error) = &draft.error {
                    row = row.child(
                        div()
                            .text_size(px(11.))
                            .text_color(cx.theme().danger_foreground)
                            .child(error.clone()),
                    );
                }
                layers = layers.child(row);
            } else {
                layers = layers.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(
                            Button::new(SharedString::from(format!("layer-{index}")))
                                .ghost()
                                .label(format!("{index}    {}", layer.name))
                                .h(px(36.))
                                .text_size(px(13.))
                                .flex_1()
                                .min_w_0()
                                .justify_start()
                                .tooltip(layer.name.clone())
                                .selected(
                                    self.core.active_layer == id
                                        && self.core.destination == Destination::Keymap,
                                )
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.command(
                                        Command::Navigate(Destination::Keymap),
                                        window,
                                        cx,
                                    );
                                    if this.core.destination == Destination::Keymap {
                                        this.command(Command::SelectLayer(id), window, cx);
                                    }
                                })),
                        )
                        .child(
                            Button::new(("rename-layer", index))
                                .ghost()
                                .w(px(24.))
                                .h(px(30.))
                                .child(svg().path("keycaps/pencil.svg").size(px(13.)))
                                .tooltip("Rename layer · F2")
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.begin_layer_rename(id, window, cx)
                                })),
                        ),
                );
            }
        }
        let mut navigation = div().flex().flex_col().gap(px(6.));
        for (destination, icon) in [
            (Destination::Combos, IconName::Plus),
            (Destination::Macros, IconName::Menu),
            (Destination::Behaviors, IconName::Asterisk),
            (Destination::ConditionalLayers, IconName::Replace),
            (Destination::Encoders, IconName::Redo),
        ] {
            navigation = navigation.child(
                self.button(
                    format!("nav-{destination:?}"),
                    destination.to_string(),
                    Command::Navigate(destination),
                    cx,
                )
                .icon(icon)
                .w_full()
                .justify_start()
                .selected(self.core.destination == destination),
            );
        }
        div()
            .id("workspace-sidebar")
            .w(px(width))
            .h_full()
            .flex_shrink_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .px_3()
            .pt_5()
            .pb_4()
            .bg(cx.theme().sidebar)
            .border_r_1()
            .border_color(cx.theme().sidebar_border)
            .child(caption("LAYERS", cx).px_1().mb_5())
            .child(layers)
            .child(divider(cx).my_6())
            .child(caption("CUSTOMIZE", cx).px_1().mb_4())
            .child(navigation)
            .child(divider(cx).mt_6())
            .child(
                self.button(
                    "nav-open",
                    "Open configuration",
                    Command::Navigate(Destination::Repository),
                    cx,
                )
                .icon(IconName::FolderOpen)
                .mt_3(),
            )
            .child(div().flex_1().min_h(px(20.)))
            .child(
                div()
                    .px_1()
                    .text_size(px(12.))
                    .text_color(cx.theme().muted_foreground)
                    .child(format!(
                        "ZMK · {} keys",
                        self.core.project.keyboard.keys.len()
                    )),
            )
    }
}
