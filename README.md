# Keyforge

A desktop editor for [ZMK](https://zmk.dev/) keymaps, with local firmware builds.

<img width="1425" height="872" alt="image" src="https://github.com/user-attachments/assets/26f45ee5-ee93-49da-a5a0-75055a43038c" />


## Features

### Editing

- Edit keys and layers on a keyboard layout.
- Search ZMK behaviors and keycodes, including names and descriptions.
- Edit separate hold/tap parameters and combine left/right modifiers.
- Assign custom behaviors and macros from your configuration, or edit a binding expression directly.
- Add and edit combos and macros.
- Create hold-tap, tap-dance and mod-morph behaviors.
- Edit conditional layers and existing encoder bindings.
- Undo changes and check for errors before saving.

### Files and builds

- Open local keymaps or load a configuration from GitHub.
- Save a local copy of a GitHub configuration.
- Build firmware with Docker or Podman.
- View compiler output, errors and warnings in the app.
- Save changes back to GitHub explicitly.
- Reopen the last saved keymap when the app starts.

## Getting started

Install [Rust](https://rustup.rs/) and the native build dependencies for your system. On Linux, you'll need a C/C++ toolchain, pkg-config, Wayland/X11, xkbcommon, Fontconfig/FreeType, OpenSSL and Vulkan support.

```sh
git clone https://github.com/ctrl-alt-adrian/keyforge.git
cd keyforge
cargo app --release
```

The first run takes longer because Cargo has to compile the app and its dependencies.

## Usage

### Open a local keymap

Open a `.keymap` file or a folder containing one keymap. You can also pass the file when starting Keyforge:

```sh
cargo app --release -- --open /path/to/zmk-config/config/corne.keymap
```

### Open a GitHub configuration

Install the [GitHub CLI](https://cli.github.com/) and sign in:

```sh
gh auth login --web
```

Choose **Connect GitHub**, select your configuration repository, branch and keymap, then click **Open keymap**.

### Edit your keymap

Select a layer, click a key and choose its behavior. Expand an action to search its keycode or layer. Modifiers appear for the keycode being edited. Valid changes stage immediately and participate in undo/redo.

Custom behaviors defined in the loaded keymap appear alongside the built-in behaviors. Expand **Binding expression** to inspect, copy or edit ZMK directly. Invalid drafts stay in the editor until corrected or explicitly discarded. Features such as mouse movement or lighting also need support enabled in your firmware configuration.

Combos, macros, behaviors, conditional layers and encoders reuse the same contextual editing controls. Shared behavior definitions own their timing and child behaviors; each key assignment owns its arguments.

### Save your changes

Use **Save file** under **Open configuration** to save without building.

For a configuration opened from GitHub, **Save copy…** creates a local copy. Use **Save to GitHub** if you want to write changes back to the repository.

Set **Preferences → Local keymap save directory** to choose where local configuration copies are saved. Changing the directory takes effect on the next local save.

### Build firmware

Install Docker or Podman and make sure your user can run it. Your configuration needs a `build.yaml` and either `config/west.yml` or a root `west.yml`.

Click **Save and build**. If you opened the configuration from GitHub, Keyforge offers to make a local copy first. It also asks before downloading a missing build image.

Compiler output appears under **Output**, with errors and warnings under **Problems**. Successful firmware appears under **Artifacts**, grouped by build and target, with export and folder controls. Previous successes remain distinguishable from the latest build. Later builds reuse cached dependencies and build files.

## Current limits

Keyforge is still in development.

- Corne 42 is the only supported keyboard layout.
- The app has been run on Linux with Wayland. macOS and Windows haven't been verified.
- Keymaps that need C preprocessing or expansion across multiple files aren't supported yet.
- Flashing firmware isn't included.

## Inspiration

Keyforge was inspired by [Keymap Editor](https://github.com/nickcoutsos/keymap-editor), created by [Nick Coutsos](https://github.com/nickcoutsos).

## License

Keyforge is free for personal, professional and commercial use under the
[Keyforge Source-Available License 1.0](LICENSE). You may modify and share
free copies, but selling Keyforge or modified versions, charging for access
to them, or bundling them with paid products requires prior written permission.

Copyright (c) 2026 Adrian Thomas.

Previously MIT-licensed versions remain available under their original terms.
Third-party dependencies and vendored code retain their upstream licenses.
Fonts, icons and ZMK keycode data keep their upstream licenses and credits in
[assets](assets/).
