//! Embedded Keyforge domain icons alongside the existing Component asset source.
use gpui::{AssetSource, Result, SharedString};
use std::borrow::Cow;

pub struct Assets;
impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        match path {
            "keycaps/pencil.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/pencil.svg")
                    .as_slice()
                    .into(),
            )),
            "keycaps/corner-down-left.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/corner-down-left.svg")
                    .as_slice()
                    .into(),
            )),
            "keycaps/space.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/space.svg")
                    .as_slice()
                    .into(),
            )),
            "keycaps/arrow-big-up.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/arrow-big-up.svg")
                    .as_slice()
                    .into(),
            )),
            "keycaps/command.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/command.svg")
                    .as_slice()
                    .into(),
            )),
            "keycaps/bluetooth.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/bluetooth.svg")
                    .as_slice()
                    .into(),
            )),
            "keycaps/usb.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/usb.svg")
                    .as_slice()
                    .into(),
            )),
            "keycaps/volume-1.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/volume-1.svg")
                    .as_slice()
                    .into(),
            )),
            "keycaps/volume-2.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/volume-2.svg")
                    .as_slice()
                    .into(),
            )),
            "keycaps/volume-x.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/volume-x.svg")
                    .as_slice()
                    .into(),
            )),
            "keycaps/skip-back.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/skip-back.svg")
                    .as_slice()
                    .into(),
            )),
            "keycaps/skip-forward.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/skip-forward.svg")
                    .as_slice()
                    .into(),
            )),
            "keycaps/play.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/play.svg")
                    .as_slice()
                    .into(),
            )),
            "keycaps/power.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/power.svg")
                    .as_slice()
                    .into(),
            )),
            "keycaps/lock-keyhole.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/lock-keyhole.svg")
                    .as_slice()
                    .into(),
            )),
            "keycaps/calculator.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/calculator.svg")
                    .as_slice()
                    .into(),
            )),
            "keycaps/printer.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/printer.svg")
                    .as_slice()
                    .into(),
            )),
            "keycaps/mail.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/mail.svg")
                    .as_slice()
                    .into(),
            )),
            "keycaps/calendar.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/calendar.svg")
                    .as_slice()
                    .into(),
            )),
            "keycaps/file-text.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/file-text.svg")
                    .as_slice()
                    .into(),
            )),
            "keycaps/keyboard.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/keyboard.svg")
                    .as_slice()
                    .into(),
            )),
            "keycaps/monitor.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/monitor.svg")
                    .as_slice()
                    .into(),
            )),
            "keycaps/repeat-2.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/repeat-2.svg")
                    .as_slice()
                    .into(),
            )),
            "keycaps/pause.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/pause.svg")
                    .as_slice()
                    .into(),
            )),
            "keycaps/camera.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/camera.svg")
                    .as_slice()
                    .into(),
            )),
            "keycaps/layers.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/layers.svg")
                    .as_slice()
                    .into(),
            )),
            "keycaps/arrow-down-to-line.svg" => Ok(Some(
                include_bytes!("../../../assets/icons/arrow-down-to-line.svg")
                    .as_slice()
                    .into(),
            )),
            _ => gpui_component_assets::Assets.load(path),
        }
    }
    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut entries = gpui_component_assets::Assets.list(path)?;
        entries.extend(
            [
                "keycaps/pencil.svg",
                "keycaps/corner-down-left.svg",
                "keycaps/space.svg",
                "keycaps/arrow-big-up.svg",
                "keycaps/command.svg",
                "keycaps/bluetooth.svg",
                "keycaps/usb.svg",
                "keycaps/volume-1.svg",
                "keycaps/volume-2.svg",
                "keycaps/volume-x.svg",
                "keycaps/skip-back.svg",
                "keycaps/skip-forward.svg",
                "keycaps/play.svg",
                "keycaps/power.svg",
                "keycaps/lock-keyhole.svg",
                "keycaps/calculator.svg",
                "keycaps/printer.svg",
                "keycaps/mail.svg",
                "keycaps/calendar.svg",
                "keycaps/file-text.svg",
                "keycaps/keyboard.svg",
                "keycaps/monitor.svg",
                "keycaps/repeat-2.svg",
                "keycaps/pause.svg",
                "keycaps/camera.svg",
                "keycaps/layers.svg",
                "keycaps/arrow-down-to-line.svg",
            ]
            .into_iter()
            .filter(|entry| entry.starts_with(path))
            .map(SharedString::from),
        );
        Ok(entries)
    }
}
