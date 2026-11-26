use crate::motion::PaneMotion;
use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme, WindowExt,
    input::{InputEvent, InputState},
    select::{SearchableVec, SelectEvent, SelectState},
};
use keyforge_core::{Application as Core, Behavior, Command, KeyPosition, Pane, layout::Camera};
use std::{cell::Cell, rc::Rc, time::Duration};

pub type Choices = SelectState<SearchableVec<String>>;

pub struct Keyforge {
    pub core: Core,
    pub camera: Camera,
    pub canvas_bounds: Rc<Cell<Bounds<Pixels>>>,
    pub hovered: Option<KeyPosition>,
    pub(crate) focus: FocusHandle,
    pub(crate) canvas_focus: FocusHandle,
    pub(crate) inputs: [Entity<InputState>; 2],
    pub(crate) behavior_select: Entity<Choices>,
    pub(crate) parameter_selects: [Entity<Choices>; 2],
    parameter_behavior: Option<Behavior>,
    pub(crate) _subscriptions: Vec<Subscription>,
    pub(crate) repository_input: Entity<InputState>,
    pub(crate) layer_name_input: Entity<InputState>,
    pub(crate) repository_choices: [Entity<Choices>; 4],
    pub(crate) definition_inputs: std::collections::BTreeMap<String, Entity<InputState>>,
    pub(crate) definition_choices: std::collections::BTreeMap<String, Entity<Choices>>,
    controls_dirty: bool,
    pub(crate) closing_prompt: bool,
    pub(crate) closing_storage: bool,
    pub(crate) build_setup_prompted: bool,
    polling: bool,
    pub(crate) build_output_list: ListState,
    build_output_identity: (u64, Option<String>),
    pub(crate) resize: Option<(Pane, Point<Pixels>, f32)>,
    pub(crate) pan: Option<Point<Pixels>>,
    pub(crate) panes: [PaneMotion; 3],
    pub(crate) selection_revision: usize,
    #[cfg(feature = "gallery")]
    pub(crate) gallery: bool,
}
impl Keyforge {
    pub fn new(core: Core, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let inputs = [
            cx.new(|cx| InputState::new(window, cx)),
            cx.new(|cx| InputState::new(window, cx)),
        ];
        let behavior_select = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(
                    Behavior::ALL
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>(),
                ),
                None,
                window,
                cx,
            )
            .searchable(true)
        });
        let parameter_selects = std::array::from_fn(|_| {
            cx.new(|cx| {
                SelectState::new(SearchableVec::new(Vec::<String>::new()), None, window, cx)
                    .searchable(true)
            })
        });
        let mut subscriptions = vec![cx.subscribe_in(
            &behavior_select,
            window,
            |this: &mut Self, _, event, window, cx| {
                if let SelectEvent::Confirm(Some(value)) = event
                    && let Some(behavior) =
                        Behavior::ALL.into_iter().find(|b| b.to_string() == *value)
                    && this
                        .core
                        .draft
                        .as_ref()
                        .is_some_and(|d| d.behavior != behavior)
                {
                    this.command(Command::SetBehavior(behavior), window, cx);
                }
            },
        )];
        for (index, input) in inputs.iter().enumerate() {
            subscriptions.push(cx.subscribe_in(
                input,
                window,
                move |this: &mut Self, input, event, window, cx| {
                    if matches!(event, InputEvent::Change) {
                        let value = input.read(cx).value().to_string();
                        if this
                            .core
                            .draft
                            .as_ref()
                            .is_some_and(|d| d.parameters[index] != value)
                        {
                            this.command(Command::SetParameter { index, value }, window, cx);
                        }
                    }
                    if matches!(event, InputEvent::PressEnter { .. }) {
                        this.command(Command::ApplyBinding, window, cx);
                    }
                },
            ));
        }
        for (index, select) in parameter_selects.iter().enumerate() {
            subscriptions.push(cx.subscribe_in(
                select,
                window,
                move |this: &mut Self, _, event, window, cx| {
                    if let SelectEvent::Confirm(Some(value)) = event
                        && this
                            .core
                            .draft
                            .as_ref()
                            .is_some_and(|d| d.parameters[index] != *value)
                    {
                        this.command(
                            Command::SetParameter {
                                index,
                                value: value.clone(),
                            },
                            window,
                            cx,
                        );
                    }
                },
            ));
        }
        subscriptions.push(
            cx.observe_window_bounds(window, |this: &mut Self, window, cx| {
                let size = window.viewport_size();
                this.command(
                    Command::WindowResized {
                        width: size.width.into(),
                        height: size.height.into(),
                    },
                    window,
                    cx,
                );
            }),
        );
        subscriptions.push(
            cx.observe_window_appearance(window, |this: &mut Self, window, cx| {
                if this.core.workspace.preferences.appearance
                    == keyforge_core::workspace::Appearance::System
                {
                    crate::theme::configure(
                        keyforge_core::workspace::Appearance::System,
                        Some(window),
                        cx,
                    );
                    cx.notify();
                }
            }),
        );
        let repository_input = cx.new(|cx| InputState::new(window, cx));
        let layer_name_input = cx.new(|cx| InputState::new(window, cx).placeholder("Layer name"));
        let repository_choices = std::array::from_fn(|_| {
            cx.new(|cx| {
                SelectState::new(SearchableVec::new(Vec::<String>::new()), None, window, cx)
                    .searchable(true)
            })
        });
        let focus = cx.focus_handle();
        focus.focus(window);
        let viewport = window.viewport_size();
        let sizes = core
            .workspace
            .rendered(viewport.width.into(), f32::from(viewport.height) - 92.);
        let mut app = Self {
            core,
            camera: Camera::default(),
            canvas_bounds: Rc::new(Cell::new(Bounds::default())),
            hovered: None,
            focus,
            canvas_focus: cx.focus_handle().tab_stop(true),
            inputs,
            behavior_select,
            parameter_selects,
            parameter_behavior: None,
            _subscriptions: subscriptions,
            repository_input,
            layer_name_input,
            repository_choices,
            definition_inputs: Default::default(),
            definition_choices: Default::default(),
            controls_dirty: true,
            closing_prompt: false,
            closing_storage: false,
            build_setup_prompted: false,
            polling: false,
            build_output_list: ListState::new(0, ListAlignment::Bottom, px(200.)),
            build_output_identity: (0, None),
            resize: None,
            pan: None,
            panes: [
                PaneMotion::new(sizes.left),
                PaneMotion::new(sizes.right),
                PaneMotion::new(sizes.bottom),
            ],
            selection_revision: 0,
            #[cfg(feature = "gallery")]
            gallery: std::env::args().any(|arg| arg == "--gallery"),
        };
        app.install_live_controls(window, cx);
        app.sync_inputs(window, cx);
        app.sync_live_controls(window, cx);
        app.controls_dirty = false;
        app.sync_build_output();
        app.schedule_poll(window, cx);
        app
    }
    pub fn command(&mut self, command: Command, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx)
            && !matches!(
                command,
                Command::CheckBuildEnvironment
                    | Command::PrepareBuildEnvironment
                    | Command::DismissBuildSetup
                    | Command::WindowResized { .. }
                    | Command::SetAppearance(_)
            )
        {
            return;
        }
        if matches!(command, Command::CheckBuildEnvironment) {
            self.build_setup_prompted = false;
        }
        let old_selection = (self.core.active_layer, self.core.selected_key);
        let pane_transition = matches!(command, Command::TogglePane(_) | Command::ToggleFocusMode);
        let appearance_change = matches!(command, Command::SetAppearance(_));
        let motion_change = matches!(command, Command::SetReducedMotion(_));
        if self.core.project.zmk.is_none()
            && matches!(
                command,
                Command::SaveDocument | Command::SaveAndBuild | Command::StartBuild { .. }
            )
        {
            self.core.status = "Open a configuration before saving or building".into();
            cx.notify();
            return;
        }
        self.core.execute(command);
        self.sync_build_output();
        if motion_change {
            for motion in &mut self.panes {
                motion.finish();
            }
        }
        if old_selection != (self.core.active_layer, self.core.selected_key) {
            self.selection_revision += 1;
        }
        if appearance_change {
            crate::theme::configure(self.core.workspace.preferences.appearance, Some(window), cx);
        }
        if pane_transition {
            // Return focus from disappearing controls, keeping canvas navigation available.
            self.canvas_focus.focus(window);
        }
        self.sync_panes(window, pane_transition);
        self.sync_inputs(window, cx);
        self.sync_live_controls(window, cx);
        self.controls_dirty = false;
        self.schedule_poll(window, cx);
        self.present_build_setup(window, cx);
        cx.notify();
    }
    pub(crate) fn sync_panes(&mut self, window: &Window, animate: bool) {
        let viewport = window.viewport_size();
        let size = self
            .core
            .workspace
            .rendered(viewport.width.into(), f32::from(viewport.height) - 92.);
        for (motion, size) in self
            .panes
            .iter_mut()
            .zip([size.left, size.right, size.bottom])
        {
            motion.retarget(
                size,
                animate && !self.core.workspace.preferences.reduced_motion,
            );
        }
    }
    fn sync_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(draft) = &self.core.draft {
            let refresh_choices = self.parameter_behavior != Some(draft.behavior);
            let value = draft.behavior.to_string();
            if self.behavior_select.read(cx).selected_value() != Some(&value) {
                self.behavior_select
                    .update(cx, |s, cx| s.set_selected_value(&value, window, cx));
            }
            for index in 0..2 {
                let value = draft.parameters[index].clone();
                if self.inputs[index].read(cx).value().as_ref() != value {
                    self.inputs[index]
                        .update(cx, |input, cx| input.set_value(value.clone(), window, cx));
                }
                let mut choices = match (draft.behavior, index) {
                    (Behavior::ModTap, 0) => keyforge_core::MODIFIERS
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                    (Behavior::LayerTap | Behavior::MomentaryLayer, 0) => {
                        (0..self.core.project.keymap.layers.len())
                            .map(|n| n.to_string())
                            .collect()
                    }
                    _ => keyforge_core::supported_key_codes(),
                };
                if !choices.contains(&value) {
                    choices.push(value.clone());
                }
                let select = &self.parameter_selects[index];
                if refresh_choices || select.read(cx).selected_value() != Some(&value) {
                    select.update(cx, |state, cx| {
                        state.set_items(SearchableVec::new(choices), window, cx);
                        state.set_selected_value(&value, window, cx);
                    });
                }
            }
            self.parameter_behavior = Some(draft.behavior);
        }
    }
    fn schedule_poll(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.polling || !self.core.needs_poll() {
            return;
        }
        self.polling = true;
        cx.spawn_in(window, async move |weak, cx| {
            loop {
                Timer::after(Duration::from_millis(100)).await;
                let keep = weak
                    .update_in(cx, |this, window, cx| {
                        let repository_busy = this.core.connection.busy;
                        if this.core.poll() {
                            // Log/progress events do not change editor controls.
                            this.controls_dirty |= repository_busy && !this.core.connection.busy;
                            this.sync_build_output();
                            cx.notify();
                        }
                        this.present_build_setup(window, cx);
                        this.polling = this.core.needs_poll();
                        this.polling
                    })
                    .unwrap_or(false);
                if !keep {
                    break;
                }
            }
        })
        .detach();
    }
    fn sync_build_output(&mut self) {
        // Caption and artifact/progress footer share the log's scroll surface.
        let count = self.core.build.output.len() + 2;
        let previous = self.build_output_list.item_count();
        let first = self.core.build.output.first();
        if previous < 2
            || self.build_output_identity.0 != self.core.build.id
            || self.build_output_identity.1.as_ref() != first
            || count < previous
        {
            self.build_output_identity = (self.core.build.id, first.cloned());
            self.build_output_list.reset(count);
        } else {
            if count > previous {
                // Insert before the footer, preserving the owner's scrollback.
                self.build_output_list
                    .splice(previous - 1..previous - 1, count - previous);
            }
            // Completion, cancellation and artifacts can change its height.
            self.build_output_list.splice(count - 1..count, 1);
        }
    }
    pub fn canvas_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.canvas_focus.focus(window);
        let bounds = self.canvas_bounds.get();
        let transform = crate::keyboard::transform(
            self.camera,
            &self.core.project.keyboard,
            bounds.size.width.into(),
            bounds.size.height.into(),
        );
        let point = transform.world(keyforge_core::layout::Point {
            x: (event.position.x - bounds.origin.x).into(),
            y: (event.position.y - bounds.origin.y).into(),
        });
        let key = self.core.project.keyboard.hit_test(point);
        if self.core.destination == keyforge_core::Destination::Combos {
            if let Some(key) = key {
                self.command(Command::ToggleComboPosition(key.0), window, cx);
            }
        } else {
            self.command(Command::SelectKey(key), window, cx);
        }
    }
    pub fn pan_down(&mut self, event: &MouseDownEvent) {
        self.pan = Some(event.position);
    }
}

impl Render for Keyforge {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.controls_dirty {
            self.sync_inputs(window, cx);
            self.sync_live_controls(window, cx);
            self.controls_dirty = false;
        }
        self.sync_panes(window, false);
        let t = cx.theme().clone();
        let content = self.workspace(window, cx);
        div()
            .size_full()
            .flex()
            .flex_col()
            .font_family(t.font_family.clone())
            .text_size(px(14.))
            .bg(t.background)
            .text_color(t.foreground)
            .track_focus(&self.focus)
            .key_context("Keyforge")
            .on_action(
                cx.listener(|this, _: &crate::actions::SaveBuild, w, cx| this.save_primary(w, cx)),
            )
            .on_action(
                cx.listener(|this, _: &crate::actions::SaveLocalCopy, w, cx| {
                    this.browse_path(
                        crate::editor_controls::PathAction::SaveLocal { build: false },
                        w,
                        cx,
                    )
                }),
            )
            .on_action(cx.listener(|this, _: &crate::actions::Undo, w, cx| {
                this.command(Command::Undo, w, cx)
            }))
            .on_action(cx.listener(|this, _: &crate::actions::RenameLayer, w, cx| {
                this.begin_layer_rename(this.core.active_layer, w, cx)
            }))
            .on_action(cx.listener(|this, _: &crate::actions::Redo, w, cx| {
                this.command(Command::Redo, w, cx)
            }))
            .on_action(cx.listener(|this, _: &crate::actions::ToggleLeft, w, cx| {
                this.command(Command::TogglePane(Pane::Left), w, cx)
            }))
            .on_action(cx.listener(|this, _: &crate::actions::ToggleRight, w, cx| {
                this.command(Command::TogglePane(Pane::Right), w, cx)
            }))
            .on_action(
                cx.listener(|this, _: &crate::actions::ToggleBottom, w, cx| {
                    this.command(Command::TogglePane(Pane::Bottom), w, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &crate::actions::FocusMode, w, cx| {
                this.command(Command::ToggleFocusMode, w, cx)
            }))
            .on_action(cx.listener(|this, _: &crate::actions::Deselect, w, cx| {
                this.command(Command::SelectKey(None), w, cx)
            }))
            .on_action(cx.listener(|this, _: &crate::actions::KeyLeft, w, cx| {
                this.command(Command::NavigateKey { dx: -1., dy: 0. }, w, cx)
            }))
            .on_action(cx.listener(|this, _: &crate::actions::KeyRight, w, cx| {
                this.command(Command::NavigateKey { dx: 1., dy: 0. }, w, cx)
            }))
            .on_action(cx.listener(|this, _: &crate::actions::KeyUp, w, cx| {
                this.command(Command::NavigateKey { dx: 0., dy: -1. }, w, cx)
            }))
            .on_action(cx.listener(|this, _: &crate::actions::KeyDown, w, cx| {
                this.command(Command::NavigateKey { dx: 0., dy: 1. }, w, cx)
            }))
            .on_action(cx.listener(|this, _: &crate::actions::Preferences, w, cx| {
                this.command(
                    Command::Navigate(keyforge_core::Destination::Settings),
                    w,
                    cx,
                )
            }))
            .on_action(cx.listener(|this, _: &crate::actions::FitKeyboard, _, cx| {
                this.camera = Camera::default();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &crate::actions::OpenKeymap, w, cx| {
                this.command(Command::Navigate(keyforge_core::Destination::Keymap), w, cx)
            }))
            .on_action(cx.listener(|this, _: &crate::actions::OpenCombos, w, cx| {
                this.command(Command::Navigate(keyforge_core::Destination::Combos), w, cx)
            }))
            .on_action(cx.listener(|this, _: &crate::actions::OpenMacros, w, cx| {
                this.command(Command::Navigate(keyforge_core::Destination::Macros), w, cx)
            }))
            .on_action(
                cx.listener(|this, _: &crate::actions::OpenBehaviors, w, cx| {
                    this.command(
                        Command::Navigate(keyforge_core::Destination::Behaviors),
                        w,
                        cx,
                    )
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::actions::OpenConditionalLayers, w, cx| {
                    this.command(
                        Command::Navigate(keyforge_core::Destination::ConditionalLayers),
                        w,
                        cx,
                    )
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::actions::OpenEncoders, w, cx| {
                    this.command(
                        Command::Navigate(keyforge_core::Destination::Encoders),
                        w,
                        cx,
                    )
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, window, cx| {
                if let Some((pane, start, size)) = this.resize {
                    let delta = match pane {
                        Pane::Left => event.position.x - start.x,
                        Pane::Right => start.x - event.position.x,
                        Pane::Bottom => start.y - event.position.y,
                    };
                    this.command(
                        Command::ResizePane {
                            pane,
                            size: size + f32::from(delta),
                        },
                        window,
                        cx,
                    );
                }
                if let Some(last) = this.pan {
                    this.camera.pan.x += f32::from(event.position.x - last.x);
                    this.camera.pan.y += f32::from(event.position.y - last.y);
                    this.pan = Some(event.position);
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.resize = None),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.resize = None),
            )
            .on_mouse_up(
                MouseButton::Middle,
                cx.listener(|this, _, _, _| this.pan = None),
            )
            .on_mouse_up_out(
                MouseButton::Middle,
                cx.listener(|this, _, _, _| this.pan = None),
            )
            .child(self.toolbar(window, cx))
            .child(content)
            .child(self.status_bar(cx))
            .children(gpui_component::Root::render_dialog_layer(window, cx))
    }
}
