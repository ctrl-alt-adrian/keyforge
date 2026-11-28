use crate::{
    app::Keyforge,
    components::{build_status, caption},
};
use gpui::{prelude::*, *};
use gpui_component::{ActiveTheme, Disableable, WindowExt, button::Button};
use keyforge_core::{BuildStatus, Command, build_environment::State};

impl Keyforge {
    pub(crate) fn build_indicator(&self, cx: &App) -> Div {
        if self.core.build.status != BuildStatus::Idle && !self.core.build_environment.busy() {
            return build_status(self.core.build.status, cx);
        }
        div()
            .text_size(px(12.))
            .text_color(if self.core.build_environment.ready() {
                cx.theme().muted_foreground
            } else {
                cx.theme().warning_foreground
            })
            .child(self.core.build_environment.label())
    }
    pub(crate) fn build_setup_panel(&self, cx: &mut Context<Self>) -> Div {
        let state = &self.core.build_environment;
        let message = match state {
            State::Unavailable { reason, .. } => reason.clone(),
            State::NeedsImage { .. } => {
                "Download the ZMK build environment to compile firmware locally.".into()
            }
            State::Ready { engine } => {
                format!("{engine} and the ZMK build environment are available.")
            }
            State::Installing { .. } => {
                "Downloading and checking the build environment. You can keep editing.".into()
            }
            _ => "Checking the local build runtime…".into(),
        };
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(caption("LOCAL BUILDS", cx))
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(if state.ready() {
                        cx.theme().success_foreground
                    } else {
                        cx.theme().warning_foreground
                    })
                    .child(state.label()),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(cx.theme().muted_foreground)
                    .child(message),
            )
            .child(
                Button::new("check-build-setup")
                    .label(if state.ready() {
                        "Recheck build setup"
                    } else {
                        "Set up local builds…"
                    })
                    .disabled(state.busy())
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.command(Command::CheckBuildEnvironment, window, cx)
                    })),
            )
    }
    pub(crate) fn present_build_setup(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.build_setup_prompted {
            return;
        }
        let (title,detail,confirm,action)=match &self.core.build_environment {
            State::NeedsImage{..}=>("Set up local firmware builds?","Keyforge needs to download the official ZMK build image before it can compile firmware locally. Set it up now? You can keep editing while it downloads.".to_string(),"Set up",0),
            State::Unavailable{installed:true,reason}=>("Local build runtime unavailable",reason.clone(),"Retry",1),
            State::Unavailable{installed:false,reason}=>("Install Docker or Podman",reason.clone(),"Open setup guide",2),
            _=>return,
        };
        self.build_setup_prompted = true;
        let confirm_view = cx.entity().downgrade();
        let cancel_view = confirm_view.clone();
        window.open_dialog(cx, move |dialog, _, cx| {
            let confirm_view = confirm_view.clone();
            let cancel_view = cancel_view.clone();
            crate::components::confirmation(
                dialog,
                crate::components::Confirmation {
                    title,
                    detail: &detail,
                    confirm,
                    cancel: "Continue editing",
                    destructive: false,
                },
                cx,
            )
            .on_ok(move |_, window, cx| {
                let _ = confirm_view.update(cx, |this, cx| match action {
                    0 => this.command(Command::PrepareBuildEnvironment, window, cx),
                    1 => this.command(Command::CheckBuildEnvironment, window, cx),
                    _ => {
                        cx.open_url(
                            "https://zmk.dev/docs/development/local-toolchain/setup/container",
                        );
                        this.command(Command::DismissBuildSetup, window, cx);
                    }
                });
                true
            })
            .on_cancel(move |_, window, cx| {
                let _ = cancel_view.update(cx, |this, cx| {
                    this.command(Command::DismissBuildSetup, window, cx)
                });
                true
            })
        });
    }
}
