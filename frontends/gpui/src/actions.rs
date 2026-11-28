use gpui::*;

actions!(
    keyforge,
    [
        SaveBuild,
        SaveLocalCopy,
        RenameLayer,
        Undo,
        Redo,
        ToggleLeft,
        ToggleRight,
        ToggleBottom,
        FocusMode,
        Deselect,
        KeyLeft,
        KeyRight,
        KeyUp,
        KeyDown,
        FitKeyboard,
        Preferences,
        OpenKeymap,
        OpenCombos,
        OpenMacros,
        OpenBehaviors,
        OpenConditionalLayers,
        OpenEncoders,
        CloseConfirmationEnter
    ]
);

pub fn register(cx: &mut App) {
    cx.bind_keys([KeyBinding::new(
        "enter",
        CloseConfirmationEnter,
        Some("KeyforgeCloseConfirmation"),
    )]);
    let primary = if cfg!(target_os = "macos") {
        "cmd"
    } else {
        "ctrl"
    };
    cx.bind_keys([
        KeyBinding::new("f2", RenameLayer, Some("Keyforge")),
        KeyBinding::new(&format!("{primary}-s"), SaveBuild, Some("Keyforge")),
        KeyBinding::new(
            &format!("{primary}-shift-s"),
            SaveLocalCopy,
            Some("Keyforge"),
        ),
        KeyBinding::new(&format!("{primary}-z"), Undo, Some("Keyforge")),
        KeyBinding::new(&format!("{primary}-shift-z"), Redo, Some("Keyforge")),
        KeyBinding::new(&format!("{primary}-b"), ToggleLeft, Some("Keyforge")),
        KeyBinding::new(&format!("{primary}-shift-b"), ToggleRight, Some("Keyforge")),
        KeyBinding::new(&format!("{primary}-j"), ToggleBottom, Some("Keyforge")),
        KeyBinding::new(&format!("{primary}-shift-f"), FocusMode, Some("Keyforge")),
        KeyBinding::new(&format!("{primary}-,"), Preferences, Some("Keyforge")),
    ]);
    cx.bind_keys([
        KeyBinding::new(&format!("{primary}-1"), OpenKeymap, Some("Keyforge")),
        KeyBinding::new(&format!("{primary}-2"), OpenCombos, Some("Keyforge")),
        KeyBinding::new(&format!("{primary}-3"), OpenMacros, Some("Keyforge")),
        KeyBinding::new(&format!("{primary}-4"), OpenBehaviors, Some("Keyforge")),
        KeyBinding::new(
            &format!("{primary}-5"),
            OpenConditionalLayers,
            Some("Keyforge"),
        ),
        KeyBinding::new(&format!("{primary}-6"), OpenEncoders, Some("Keyforge")),
        KeyBinding::new("escape", Deselect, Some("KeyforgeCanvas")),
        KeyBinding::new("left", KeyLeft, Some("KeyforgeCanvas")),
        KeyBinding::new("right", KeyRight, Some("KeyforgeCanvas")),
        KeyBinding::new("up", KeyUp, Some("KeyforgeCanvas")),
        KeyBinding::new("down", KeyDown, Some("KeyforgeCanvas")),
        KeyBinding::new("0", FitKeyboard, Some("KeyforgeCanvas")),
    ]);
}
