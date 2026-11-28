mod action_editor;
mod actions;
mod app;
mod assets;
mod binding_controls;
mod build_setup;
mod components;
mod editor_controls;
#[cfg(feature = "gallery")]
mod gallery;
mod keyboard;
mod keycap;
mod motion;
mod parameter_picker;
mod theme;
mod workspace;

use gpui::{prelude::*, *};

fn main() {
    Application::new()
        .with_assets(assets::Assets)
        .run(|cx: &mut App| {
            gpui_component::init(cx);
            theme::load_fonts(cx);
            actions::register(cx);
            let mut core = keyforge_core::Application::for_frontend("gpui")
                .expect("Unable to open Keyforge storage");
            core.initialize_live_ui();
            core.execute(keyforge_core::Command::CheckBuildEnvironment);
            if core.project.zmk.is_some() {
                core.status = "Restored last keymap".into();
            }
            let mut arguments = std::env::args_os().skip(1);
            while let Some(argument) = arguments.next() {
                if argument == "--open"
                    && let Some(path) = arguments.next()
                {
                    core.execute(keyforge_core::Command::OpenLocal(path.into()));
                } else if argument == "--repository"
                    && let Some(repository) = arguments.next()
                {
                    core.execute(keyforge_core::Command::Navigate(
                        keyforge_core::Destination::Repository,
                    ));
                    core.execute(keyforge_core::Command::SetConnection {
                        field: "repository".into(),
                        value: repository.to_string_lossy().into_owned(),
                    });
                    core.execute(keyforge_core::Command::BrowseRepository);
                }
            }
            let preferences = core.workspace.preferences.clone();
            theme::configure(preferences.appearance, None, cx);
            let bounds = Bounds::centered(
                None,
                size(px(preferences.window_width), px(preferences.window_height)),
                cx,
            );
            cx.open_window(
                WindowOptions {
                    app_id: Some("dev.keyforge.Keyforge".into()),
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(900.), px(600.))),
                    titlebar: Some(TitlebarOptions {
                        title: Some("Keyforge".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |window, cx| {
                    window.set_window_title("Keyforge");
                    theme::configure(preferences.appearance, Some(window), cx);
                    let view = cx.new(|cx| app::Keyforge::new(core, window, cx));
                    let weak = view.downgrade();
                    window.on_window_should_close(cx, move |window, cx| {
                        weak.update(cx, |this, cx| this.request_close(window, cx))
                            .unwrap_or(true)
                    });
                    cx.new(|cx| gpui_component::Root::new(view, window, cx))
                },
            )
            .expect("Unable to open GPUI window");
            cx.activate(true);
            cx.on_window_closed(|cx| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
        });
}
