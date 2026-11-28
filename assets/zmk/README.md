# ZMK keycodes

Canonical keycode names and aliases extracted from ZMK `app/include/dt-bindings/zmk/keys.h` at `4de313367288aad5aeaa1243890e40dc813f7008`. Source: https://github.com/zmkfirmware/zmk/blob/4de313367288aad5aeaa1243890e40dc813f7008/app/include/dt-bindings/zmk/keys.h . MIT; upstream license retained. No firmware code is compiled.

`keycode-labels.tsv` contains all 644 object-like keycode defines from that header, paired with their upstream comment descriptions, including aliases and deprecated names retained for existing keymaps. The searchable picker combines this with the original `keycodes.txt` catalog. Single-letter and exact-code searches rank exact matches first.

`intrinsic-modifiers.tsv` records the 53 keycode aliases in that revision whose definitions already include a modifier, including primary modifier keys and shifted symbols. The editor retains their source names and prevents adding a duplicate wrapper for the same modifier. Explicit left/right wrappers on other keys remain independent.

The built-in key-position behavior catalog in `core/src/bindings.rs` follows `app/dts/behaviors.dtsi` and its included behavior definitions at the same revision. Action values and modifier functions follow the `bt.h`, `rgb.h`, `backlight.h`, `outputs.h`, `ext_power.h`, `pointing.h` and `modifiers.h` headers under `app/include/dt-bindings/zmk/`. Upstream MIT notices remain in `LICENSE.txt`. Source metadata was checked against upstream on 2026-11-27.

Loaded behavior definitions contribute their labels, display names, binding-cell counts, and parameter kinds inferred through their child behavior references. Module/included definitions that cannot be inspected directly remain editable through the binding expression field. The catalog describes bindings; availability still depends on the selected firmware revision, configuration and hardware.
