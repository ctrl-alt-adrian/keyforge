use crate::app::Keyforge;
use crate::keycap;
use gpui::{prelude::*, *};
use gpui_component::ActiveTheme;
use keyforge_core::{Binding, layout::Point as WorldPoint};

pub fn view(app: &Keyforge, cx: &mut Context<Keyforge>) -> AnyElement {
    let theme = cx.theme().clone();
    let layers = app.core.project.keymap.layers.clone();
    let reduced = app.core.workspace.preferences.reduced_motion;
    let keyboard = app.core.project.keyboard.clone();
    let camera = app.camera;
    let selected = app.core.selected_key;
    let combo_positions = app
        .core
        .editor
        .as_ref()
        .filter(|d| {
            d.section == keyforge_core::editor::Section::Combos
                && app.core.destination == keyforge_core::Destination::Combos
        })
        .map(|d| d.positions())
        .unwrap_or_default();
    let hovered = app.hovered;
    let behaviors = app
        .core
        .project
        .zmk
        .as_ref()
        .and_then(|doc| doc.hold_tap_behaviors().ok())
        .unwrap_or_default();
    let bindings: Vec<_> = keyboard
        .keys
        .iter()
        .map(|key| {
            let binding = app
                .core
                .binding(app.core.active_layer, key.position)
                .cloned()
                .expect("Validated key binding");
            let label = keycap::for_binding(&binding, &layers, &behaviors);
            (
                binding,
                app.core.modified(app.core.active_layer, key.position),
                label,
            )
        })
        .collect();
    let bounds_state = app.canvas_bounds.clone();
    let description = app
        .hovered
        .and_then(|key| {
            bindings.get(key.0).map(|(binding, _, label)| {
                format!(
                    "Position {} · {}\n{}",
                    key.0,
                    label.description,
                    binding.zmk()
                )
            })
        })
        .unwrap_or_else(|| "Keyboard canvas · click to select · arrow keys to navigate".into());
    let element = div()
        .id("keyboard-canvas")
        .track_focus(&app.canvas_focus)
        .key_context("KeyforgeCanvas")
        .cursor_pointer()
        .tooltip(move |w, cx| {
            gpui_component::tooltip::Tooltip::new(description.clone()).build(w, cx)
        })
        .focus(|s| s.border_1().border_color(theme.ring))
        .flex_1()
        .min_h_0()
        .w_full()
        .px_10()
        .overflow_hidden()
        .child(
            canvas(
                move |bounds, _, _| {
                    bounds_state.set(bounds);
                },
                move |bounds, _, window, cx| {
                    let transform = transform(
                        camera,
                        &keyboard,
                        bounds.size.width.into(),
                        bounds.size.height.into(),
                    );
                    for (key, (binding, modified, label)) in keyboard.keys.iter().zip(&bindings) {
                        let points = key.corners(0.045).map(|p| {
                            let p = transform.screen(p);
                            point(bounds.origin.x + px(p.x), bounds.origin.y + px(p.y))
                        });
                        let mut fill = PathBuilder::fill();
                        rounded_outline(&mut fill, points, px((transform.scale * 0.11).min(7.)));
                        if let Ok(path) = fill.build() {
                            window.paint_path(
                                path,
                                if selected == Some(key.position)
                                    || combo_positions.contains(&key.position.0)
                                {
                                    theme.selection
                                } else if *modified {
                                    theme.warning
                                } else if matches!(
                                    binding,
                                    Binding::MomentaryLayer(_) | Binding::LayerTap { .. }
                                ) {
                                    theme.success
                                } else {
                                    theme.secondary
                                },
                            );
                        }
                        let is_selected = selected == Some(key.position)
                            || combo_positions.contains(&key.position.0);
                        let mut stroke =
                            PathBuilder::stroke(px(if is_selected { 2.5 } else { 1. }));
                        rounded_outline(&mut stroke, points, px((transform.scale * 0.11).min(7.)));
                        if let Ok(path) = stroke.build() {
                            window.paint_path(
                                path,
                                if is_selected || hovered == Some(key.position) {
                                    theme.primary
                                } else if *modified {
                                    theme.warning_foreground
                                } else if matches!(
                                    binding,
                                    Binding::MomentaryLayer(_) | Binding::LayerTap { .. }
                                ) {
                                    theme.success_foreground
                                } else {
                                    theme.border
                                },
                            );
                        }
                        let center = transform.screen(key.center());
                        if *modified {
                            window.paint_quad(gpui::fill(
                                Bounds::new(
                                    point(
                                        bounds.origin.x + px(center.x),
                                        bounds.origin.y
                                            + px(center.y - transform.scale * key.height * 0.34),
                                    ),
                                    size(px(3.), px(3.)),
                                ),
                                theme.warning_foreground,
                            ));
                        }
                        let has_secondary = !label.secondary.empty();
                        let color = if matches!(binding, Binding::None | Binding::Transparent) {
                            theme.muted_foreground
                        } else {
                            theme.foreground
                        };
                        let center = point(
                            bounds.origin.x + px(center.x),
                            bounds.origin.y + px(center.y),
                        );
                        keycap::paint(
                            &label.primary,
                            point(
                                center.x,
                                center.y
                                    - px(if has_secondary {
                                        transform.scale * 0.13
                                    } else {
                                        0.
                                    }),
                            ),
                            transform.scale * key.width * 0.82,
                            (transform.scale * 0.28).clamp(10., 20.),
                            (transform.scale * 0.43).clamp(14., 28.),
                            color,
                            FontWeight::SEMIBOLD,
                            window,
                            cx,
                        );
                        if has_secondary {
                            keycap::paint(
                                &label.secondary,
                                point(center.x, center.y + px(transform.scale * 0.23)),
                                transform.scale * key.width * 0.84,
                                (transform.scale * 0.17).clamp(8., 12.),
                                12.,
                                if *modified {
                                    theme.warning_foreground
                                } else {
                                    theme.muted_foreground
                                },
                                FontWeight::MEDIUM,
                                window,
                                cx,
                            );
                        }
                    }
                },
            )
            .size_full(),
        )
        .on_mouse_down(MouseButton::Left, cx.listener(Keyforge::canvas_down))
        .on_mouse_down(
            MouseButton::Middle,
            cx.listener(|this, event, _, cx| {
                this.pan_down(event);
                cx.stop_propagation();
            }),
        )
        .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
            let delta = event.delta.pixel_delta(px(20.));
            this.camera
                .zoom_by(if delta.y > px(0.) { 1.1 } else { 1. / 1.1 });
            cx.notify();
            cx.stop_propagation();
        }))
        .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
            let bounds = this.canvas_bounds.get();
            let transform = transform(
                this.camera,
                &this.core.project.keyboard,
                bounds.size.width.into(),
                bounds.size.height.into(),
            );
            let world = transform.world(WorldPoint {
                x: (event.position.x - bounds.origin.x).into(),
                y: (event.position.y - bounds.origin.y).into(),
            });
            let hovered = this.core.project.keyboard.hit_test(world);
            if this.hovered != hovered {
                this.hovered = hovered;
                cx.notify();
            }
        }))
        .on_hover(cx.listener(|this, hovered, _, cx| {
            if !hovered && this.hovered.take().is_some() {
                cx.notify();
            }
        }));
    if reduced {
        element.into_any_element()
    } else {
        element
            .with_animation(
                ("key-selection", app.selection_revision),
                Animation::new(crate::motion::SELECTION_DURATION),
                |el, delta| el.opacity(0.88 + 0.12 * delta),
            )
            .into_any_element()
    }
}

/// Round corners in screen space after applying physical rotation and camera scale.
fn rounded_outline(builder: &mut PathBuilder, points: [Point<Pixels>; 4], radius: Pixels) {
    let toward = |a: Point<Pixels>, b: Point<Pixels>| {
        let dx = f32::from(b.x - a.x);
        let dy = f32::from(b.y - a.y);
        let ratio = (f32::from(radius) / dx.hypot(dy).max(1.)).min(0.45);
        point(a.x + px(dx * ratio), a.y + px(dy * ratio))
    };
    builder.move_to(toward(points[0], points[3]));
    for i in 0..4 {
        builder.line_to(toward(points[i], points[(i + 3) % 4]));
        builder.curve_to(toward(points[i], points[(i + 1) % 4]), points[i]);
    }
    builder.close();
}

/// Optical centering for the spacious base artboard; shared camera math still
/// determines fit, scale and inverse hit coordinates for every keyboard layout.
pub fn transform(
    camera: keyforge_core::layout::Camera,
    keyboard: &keyforge_core::layout::Keyboard,
    width: f32,
    height: f32,
) -> keyforge_core::layout::Transform {
    let mut transform = camera.transform(keyboard, width, height);
    transform.offset.y -= ((height - 500.) / 2.).clamp(0., 50.);
    transform
}
