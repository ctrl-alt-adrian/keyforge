# Keyforge

A desktop editor for [ZMK](https://zmk.dev/) keyboard configurations. Edit your keymap visually, manage layers and behaviors, and build firmware from the same app.

Keyforge is an independent desktop application inspired by [Keymap Editor](https://github.com/nickcoutsos/keymap-editor), created by [Nick Coutsos](https://github.com/nickcoutsos). Credit goes to Nick for his work making visual ZMK keymap editing accessible.

Keyforge is in early development. The currently supported keyboard layout is **Corne 42**, and the app has been run on Linux with Wayland. Other keyboard layouts are not yet supported; macOS and Windows have not been verified.

## Features

- Visual keymap editing with layer selection and renaming.
- Searchable ZMK keycodes and support for custom behavior references.
- Combo editing, including key positions, bindings, timeouts and active layers.
- Macro editing with timing controls and reorderable action sequences.
- Behavior editing for hold-tap, tap-dance and mod-morph definitions.
- Conditional layers and editing of existing rotary encoder bindings.
- Undo, redo, validation and a view of pending changes.
- Local `.keymap` files and GitHub configuration repositories.
- Local firmware builds with Docker or Podman, streaming build output, error diagnostics and firmware output folders.
- Optional GitHub saves, workflow builds and firmware artifact downloads.
- Last-keymap restoration, light and dark themes, and resizable workspace panes.

## Usage

### Run Keyforge

Install [Rust](https://rustup.rs/) and your platform's native build dependencies. On Linux, these include a C/C++ toolchain, pkg-config, Wayland/X11, xkbcommon, Fontconfig/FreeType, OpenSSL and Vulkan support.

Clone the repository and launch the app:

```sh
git clone https://github.com/ctrl-alt-adrian/keyforge.git
cd keyforge
cargo app --release
```

The first launch compiles the application and its dependencies.

### Open and edit a keymap

Open a local `.keymap` file or a configuration folder containing one keymap. You can also open a file directly when launching:

```sh
cargo app --release -- --open /path/to/zmk-config/config/corne.keymap
```

Select a layer and key, choose a behavior and its parameters, then select **Apply binding**. Use the Combos, Macros, Behaviors, Conditional layers and Encoders screens for the corresponding definitions. Apply changes in those forms before saving.

Keyforge preserves untouched keymap source. Configurations that require C preprocessing or expansion across multiple keymap files are not currently supported. See the [ZMK documentation](https://zmk.dev/docs) for the behavior and configuration options used by your keyboard.

### Save and build locally

Open a complete ZMK configuration containing `build.yaml` and `config/west.yml` or a root `west.yml`. A standalone `.keymap` file can be edited and saved, but needs the rest of its configuration to build firmware.

Install Docker or Podman and make sure it is running and accessible to your user. Keyforge checks the build environment and asks before downloading a missing build image.

Choose **Save and build** to save your configuration and compile its firmware. Build output streams into the bottom pane; errors and warnings also appear in **Problems**. Successful builds show the generated files and an **Open firmware folder** action. Dependencies and build files are cached for subsequent builds.

To save without compiling, choose **Save file** under **Open configuration**. Keyforge does not flash firmware to your keyboard.

### GitHub configurations

Install the [GitHub CLI](https://cli.github.com/) and sign in:

```sh
gh auth login --web
```

In Keyforge, choose **Connect GitHub**, select your ZMK configuration repository, branch and `.keymap` file, then choose **Open keymap**.

**Save copy…** creates a local configuration copy for editing and building. **Save and build** also offers this local-copy workflow for a configuration opened from GitHub.

To work directly with GitHub, use **Save to GitHub** or **Save and build on GitHub** under **Open configuration**. These actions write to your repository and use its existing firmware workflow. Starting a workflow without a new commit requires that workflow to support `workflow_dispatch`.

## License

Keyforge is licensed under the [MIT License](LICENSE).

Copyright (c) 2026 Adrian Thomas.

Bundled fonts, icons and ZMK keycode data retain their upstream licenses and attribution notices in [assets](assets/).
