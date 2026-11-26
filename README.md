# Keyforge

A desktop editor for [ZMK](https://zmk.dev/) keymaps, with local firmware builds.

<img width="1411" height="991" alt="image" src="https://github.com/user-attachments/assets/93f86c3c-94f1-4ffb-b126-c01b82867f74" />

## Features

### Editing

- Edit keys and layers on a keyboard layout.
- Search ZMK keycodes or use custom behavior references.
- Add and edit combos and macros.
- Create hold-tap, tap-dance and mod-morph behaviors.
- Edit conditional layers and existing encoder bindings.
- Undo changes and check for errors before saving.

### Files and builds

- Open local keymaps or load a configuration from GitHub.
- Save a local copy of a GitHub configuration.
- Build firmware with Docker or Podman.
- View compiler output, errors and warnings in the app.
- Save to GitHub and run an existing firmware workflow.
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

Select a layer, click a key and choose its behavior. Click **Apply binding** to keep the change.

Combos, macros, behaviors, conditional layers and encoders have their own editing screens. Click **Apply changes** in those screens before saving.

### Save your changes

Use **Save file** under **Open configuration** to save without building.

For a configuration opened from GitHub, **Save copy…** creates a local copy. Use **Save to GitHub** if you want to write changes back to the repository.

### Build firmware

Install Docker or Podman and make sure your user can run it. Your configuration needs a `build.yaml` and either `config/west.yml` or a root `west.yml`.

Click **Save and build**. If you opened the configuration from GitHub, Keyforge offers to make a local copy first. It also asks before downloading a missing build image.

Compiler output appears in the bottom pane. Errors and warnings are listed under **Problems**. When the build finishes successfully, click **Open firmware folder** to find the files. Later builds reuse cached dependencies and build files.

To build on GitHub instead, choose **Save and build on GitHub** under **Open configuration**. This uses your repository's existing workflow. The workflow needs `workflow_dispatch` support to run without a new commit.

## Current limits

Keyforge is still in development.

- Corne 42 is the only supported keyboard layout.
- The app has been run on Linux with Wayland. macOS and Windows haven't been verified.
- Keymaps that need C preprocessing or expansion across multiple files aren't supported yet.
- Flashing firmware isn't included.

## Inspiration

Keyforge was inspired by [Keymap Editor](https://github.com/nickcoutsos/keymap-editor), created by [Nick Coutsos](https://github.com/nickcoutsos).

## License

[MIT License](LICENSE). Copyright (c) 2026 Adrian Thomas.

Fonts, icons and ZMK keycode data keep their upstream licenses and credits in [assets](assets/).
