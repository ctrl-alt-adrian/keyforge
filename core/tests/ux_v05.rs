use keyforge_core::{
    Application, BuildStatus, Command, Destination, KeyPosition, LayerId,
    bindings::{self, ParameterKind},
    editing::{BindingEdit, BindingTarget},
    editor::{self, Section},
    persistence::{decode_project, encode_project, export_artifact},
    services::build_step,
    workspace::Appearance,
    zmk::{self, Document, Origin},
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

const DEFINITIONS: &str = r#"
    behaviors {
        HML: home_left {
            compatible = "zmk,behavior-hold-tap";
            #binding-cells = <2>;
            bindings = <&kp>, <&kp>;
            flavor = "balanced";
            tapping-term-ms = <210>;
            vendor-property = <731>;
        };
        HMR: misleading_name {
            compatible = "zmk,behavior-hold-tap";
            #binding-cells = <2>;
            bindings = <&mo>, <&kp>;
        };
        dance: dance {
            compatible = "zmk,behavior-tap-dance";
            #binding-cells = <0>;
            bindings = <&kp ESC>, <&kp CAPSLOCK>;
            vendor-property = "do not replace";
        };
        rotate_custom: rotate_custom {
            compatible = "zmk,behavior-sensor-rotate-var";
            #sensor-binding-cells = <2>;
            bindings = <&mo>, <&kp>;
        };
    };
    combos {
        compatible = "zmk,combos";
        escape_combo {
            key-positions = <0 1>;
            bindings = <&kp ESC>;
            timeout-ms = <50>;
            slow-release;
            vendor-property = <918>;
        };
    };
    macros {
        copy: copy {
            compatible = "zmk,behavior-macro-one-param";
            #binding-cells = <1>;
            wait-ms = <15>;
            bindings = <&macro_press &kp LCTRL>,
                       <&kp C>,
                       <&macro_pause_for_release>,
                       <&macro_release &kp LCTRL>,
                       <&macro_param_1to1>,
                       <&kp MACRO_PLACEHOLDER>,
                       <&macro_wait_time 10>,
                       <&vendor FUNC(1, 2)>;
            vendor-property = "keep macro metadata";
        };
    };
    conditional_layers {
        compatible = "zmk,conditional-layers";
        adjust { if-layers = <1 2>; then-layer = <3>; vendor-property = <54>; };
    };
"#;

fn app(binding: &str) -> Application {
    let keyboard = Application::sample().unwrap().project.keyboard;
    let mut bindings = vec!["&kp A".to_string(); keyboard.keys.len()];
    bindings[0] = binding.to_string();
    let layers = (0..4).map(|i| format!("layer_{i} {{ display-name = \"Layer {i}\"; bindings = <{}>; sensor-bindings = <&rotate_custom 1 A>; }};", bindings.join(" "))).collect::<Vec<_>>().join("\n");
    let source = format!(
        "#include <behaviors.dtsi>\n#include <dt-bindings/zmk/keys.h>\n/ {{ {DEFINITIONS} keymap {{ compatible = \"zmk,keymap\"; {layers} }}; }};"
    );
    let document = Document {
        origin: Origin::Local {
            path: PathBuf::from("/not-written/corne.keymap"),
            saved_source: source.clone(),
        },
        source,
    };
    let mut app = Application::from_project(document.project(keyboard).unwrap()).unwrap();
    app.initialize_live_ui();
    app
}
fn key(app: &mut Application) {
    app.dispatch(Command::SelectKey(Some(KeyPosition(0))))
        .unwrap();
}
fn assignment(app: &mut Application, target: BindingTarget, edit: BindingEdit) {
    app.dispatch(Command::EditBinding { target, edit }).unwrap();
}
fn argument(app: &mut Application, target: BindingTarget, index: usize, value: &str) {
    assignment(
        app,
        target,
        BindingEdit::Argument {
            index,
            value: value.into(),
            selected: true,
        },
    );
}
fn definition(app: &mut Application, destination: Destination, section: Section, name: &str) {
    app.dispatch(Command::Navigate(destination)).unwrap();
    let id = editor::entries(&app.project, section, 0)
        .unwrap()
        .into_iter()
        .find(|entry| entry.name == name)
        .unwrap()
        .id;
    app.dispatch(Command::SelectEditor(id)).unwrap();
}
fn property(app: &Application, name: &str) -> String {
    let document = app.project.zmk.as_ref().unwrap();
    let tree = document.tree().unwrap();
    tree.find(app.editor.as_ref().unwrap().node.as_ref().unwrap())
        .unwrap()
        .value(&document.source, name)
        .unwrap()
        .to_string()
}
fn field(app: &mut Application, name: &str, value: &str) {
    app.dispatch(Command::SetEditorField {
        field: name.into(),
        value: value.into(),
    })
    .unwrap();
}

#[test]
fn metadata_controls_parameter_types_and_original_serialization_order() {
    let mut app = app("&HMR 1 K");
    key(&mut app);
    let catalog = bindings::catalog(app.project.zmk.as_ref());
    let spec = catalog
        .iter()
        .find(|spec| spec.reference == "&HMR")
        .unwrap();
    assert_eq!(spec.parameters[0].kind, ParameterKind::Layer);
    assert_eq!(spec.parameters[1].kind, ParameterKind::KeyCode);
    assert_eq!(
        spec.presentation_order(&["1".into(), "K".into()]),
        vec![1, 0]
    );
    argument(&mut app, BindingTarget::Key, 1, "C");
    assert_eq!(app.selected_binding().unwrap().zmk(), "&HMR 1 C");
    argument(&mut app, BindingTarget::Key, 0, "2");
    assert_eq!(app.selected_binding().unwrap().zmk(), "&HMR 2 C");
    assert!(app.has_changes());
}

#[test]
fn tap_and_hold_modifiers_are_independent_and_undoable() {
    let mut app = app("&HML RIGHT_GUI K");
    key(&mut app);
    assignment(
        &mut app,
        BindingTarget::Key,
        BindingEdit::Modifier {
            index: 1,
            modifier: "LC".into(),
        },
    );
    assignment(
        &mut app,
        BindingTarget::Key,
        BindingEdit::Modifier {
            index: 1,
            modifier: "LS".into(),
        },
    );
    assert_eq!(
        app.selected_binding().unwrap().zmk(),
        "&HML RIGHT_GUI LC(LS(K))"
    );
    assignment(
        &mut app,
        BindingTarget::Key,
        BindingEdit::Modifier {
            index: 0,
            modifier: "LS".into(),
        },
    );
    assert_eq!(
        app.selected_binding().unwrap().zmk(),
        "&HML LS(RIGHT_GUI) LC(LS(K))"
    );
    app.dispatch(Command::Undo).unwrap();
    assert_eq!(
        app.selected_binding().unwrap().zmk(),
        "&HML RIGHT_GUI LC(LS(K))"
    );
    app.dispatch(Command::Redo).unwrap();
    assert_eq!(
        app.selected_binding().unwrap().zmk(),
        "&HML LS(RIGHT_GUI) LC(LS(K))"
    );
}

#[test]
fn primary_and_intrinsic_modifiers_are_not_duplicated_or_normalized() {
    let mut app = app("&kp A");
    key(&mut app);
    argument(&mut app, BindingTarget::Key, 0, "RIGHT_GUI");
    let before = app.project.clone();
    assignment(
        &mut app,
        BindingTarget::Key,
        BindingEdit::Modifier {
            index: 0,
            modifier: "RG".into(),
        },
    );
    assert_eq!(app.project, before);
    assignment(
        &mut app,
        BindingTarget::Key,
        BindingEdit::Modifier {
            index: 0,
            modifier: "LS".into(),
        },
    );
    assert_eq!(app.selected_binding().unwrap().zmk(), "&kp LS(RIGHT_GUI)");
    argument(&mut app, BindingTarget::Key, 0, "DOLLAR");
    assert_eq!(app.selected_binding().unwrap().zmk(), "&kp DOLLAR");
    let before = app.project.clone();
    assignment(
        &mut app,
        BindingTarget::Key,
        BindingEdit::Modifier {
            index: 0,
            modifier: "LS".into(),
        },
    );
    assert_eq!(app.project, before);
}

#[test]
fn left_and_right_modifiers_survive_a_base_key_change() {
    let mut app = app("&kp LC(RA(K))");
    key(&mut app);
    argument(&mut app, BindingTarget::Key, 0, "B");
    assert_eq!(app.selected_binding().unwrap().zmk(), "&kp LC(RA(B))");
    assert!(!app.has_unapplied_draft());
}

#[test]
fn unknown_expressions_survive_selection_navigation_and_json_round_trips() {
    let source = "&vendor FUNC(1, 2) (VALUE << 1)";
    let mut app = app(source);
    let before = app.project.clone();
    key(&mut app);
    app.dispatch(Command::Navigate(Destination::Settings))
        .unwrap();
    app.dispatch(Command::Navigate(Destination::Keymap))
        .unwrap();
    assert_eq!(app.assignment_source(&BindingTarget::Key).unwrap(), source);
    assert_eq!(app.project, before);
    assert!(!app.has_changes());
    assert_eq!(
        decode_project(&encode_project(&app.project).unwrap()).unwrap(),
        before
    );
}

#[test]
fn invalid_expressions_and_empty_arguments_do_not_replace_staged_bindings() {
    let mut app = app("&HML RIGHT_GUI K");
    key(&mut app);
    let before = app.project.clone();
    argument(&mut app, BindingTarget::Key, 0, "");
    assert_eq!(app.project, before);
    assert_eq!(app.binding_form().unwrap().arguments, vec!["", "K"]);
    assert!(!app.assignment_errors(&BindingTarget::Key).is_empty());
    assert!(
        app.dispatch(Command::SelectKey(Some(KeyPosition(1))))
            .is_err()
    );
    assert_eq!(app.selected_key, Some(KeyPosition(0)));
    assert!(app.dispatch(Command::Undo).is_err());
    app.dispatch(Command::DiscardBindingDraft).unwrap();
    assignment(
        &mut app,
        BindingTarget::Key,
        BindingEdit::Expression("&kp LC(".into()),
    );
    assert_eq!(app.project, before);
    assert_eq!(
        app.assignment_source(&BindingTarget::Key).unwrap(),
        "&kp LC("
    );
    assignment(
        &mut app,
        BindingTarget::Key,
        BindingEdit::Expression("&kp LC(K)".into()),
    );
    assert_eq!(app.selected_binding().unwrap().zmk(), "&kp LC(K)");
    assert!(!app.has_unapplied_draft());
}

#[test]
fn invalid_nested_bindings_keep_their_editor_and_recover_after_preferences() {
    let mut app = app("&kp A");
    definition(
        &mut app,
        Destination::Combos,
        Section::Combos,
        "escape_combo",
    );
    let before = app.project.clone();
    let target = BindingTarget::Field {
        field: "bindings".into(),
        binding: 0,
    };
    assignment(
        &mut app,
        target.clone(),
        BindingEdit::Expression("&kp".into()),
    );
    assert!(
        app.dispatch(Command::Navigate(Destination::Keymap))
            .is_err()
    );
    assert_eq!(app.destination, Destination::Combos);
    assert!(
        app.dispatch(Command::RevealDiagnostic {
            layer: LayerId(0),
            key: KeyPosition(1)
        })
        .is_err()
    );
    app.dispatch(Command::Navigate(Destination::Settings))
        .unwrap();
    app.dispatch(Command::Navigate(Destination::Combos))
        .unwrap();
    assert_eq!(app.assignment_source(&target).as_deref(), Some("&kp"));
    assert_eq!(app.project, before);
    assignment(&mut app, target, BindingEdit::Expression("&kp B".into()));
    assert_eq!(property(&app, "bindings"), "<&kp B>");
    assert!(!app.has_unapplied_draft());
    app.dispatch(Command::Navigate(Destination::Keymap))
        .unwrap();
}

#[test]
fn shared_definition_rename_updates_references_and_preserves_unknown_properties() {
    let mut app = app("&HML RIGHT_GUI K");
    key(&mut app);
    let before = app.project.clone();
    app.dispatch(Command::OpenDefinition("&HML".into()))
        .unwrap();
    assert_eq!(app.destination, Destination::Behaviors);
    field(&mut app, "name", "home_left_renamed");
    assert_eq!(
        app.project.keymap.layers[0].bindings[0].zmk(),
        "&home_left_renamed RIGHT_GUI K"
    );
    assert_eq!(property(&app, "vendor-property"), "<731>");
    assert!(app.dispatch(Command::DeleteDefinition).is_err());
    app.dispatch(Command::Undo).unwrap();
    assert_eq!(app.project, before);
}

#[test]
fn shared_child_behaviors_have_no_assignment_arguments() {
    let mut app = app("&HML RIGHT_GUI K");
    definition(&mut app, Destination::Behaviors, Section::Behaviors, "HML");
    assignment(
        &mut app,
        BindingTarget::Child {
            field: "bindings".into(),
            binding: 0,
        },
        BindingEdit::Behavior("&mo".into()),
    );
    let references = property(&app, "bindings");
    let values = zmk::binding_spans(&references)
        .unwrap()
        .iter()
        .map(|span| &references[span.clone()])
        .collect::<Vec<_>>();
    assert_eq!(values, vec!["&mo", "&kp"]);
    assert_eq!(
        app.project.keymap.layers[0].bindings[0].zmk(),
        "&HML RIGHT_GUI K"
    );
    assert!(
        app.assignment_errors(&BindingTarget::Child {
            field: "bindings".into(),
            binding: 0
        })
        .is_empty()
    );
}

#[test]
fn combo_positions_toggle_physically_and_result_uses_the_same_binding_editor() {
    let mut app = app("&kp A");
    definition(
        &mut app,
        Destination::Combos,
        Section::Combos,
        "escape_combo",
    );
    app.dispatch(Command::ToggleComboPosition(2)).unwrap();
    assert_eq!(property(&app, "key-positions"), "<0 1 2>");
    let target = BindingTarget::Field {
        field: "bindings".into(),
        binding: 0,
    };
    argument(&mut app, target.clone(), 0, "C");
    assignment(
        &mut app,
        target,
        BindingEdit::Modifier {
            index: 0,
            modifier: "LC".into(),
        },
    );
    assert_eq!(property(&app, "bindings"), "<&kp LC(C)>");
    assert_eq!(property(&app, "vendor-property"), "<918>");
    assert!(
        app.project
            .zmk
            .as_ref()
            .unwrap()
            .source
            .contains("slow-release;")
    );
    assert!(!app.has_unapplied_draft());
}

#[test]
fn macro_timing_edits_leave_imported_sequences_byte_preserved() {
    let mut app = app("&kp A");
    definition(&mut app, Destination::Macros, Section::Macros, "copy");
    let before = property(&app, "bindings");
    field(&mut app, "wait-ms", "22");
    assert_eq!(property(&app, "bindings"), before);
    assert_eq!(property(&app, "vendor-property"), "\"keep macro metadata\"");
    assert_eq!(app.editor.as_ref().unwrap().get("step-mode-1"), "Raw");
}

#[test]
fn macro_action_edits_preserve_inherited_modes_pause_forwarding_and_unknown_calls() {
    let mut app = app("&kp A");
    definition(&mut app, Destination::Macros, Section::Macros, "copy");
    let original = zmk::cell_groups(&property(&app, "bindings")).unwrap();
    argument(
        &mut app,
        BindingTarget::Field {
            field: "step-bindings-1".into(),
            binding: 0,
        },
        0,
        "B",
    );
    let mut expected = original;
    expected[1] = "&kp B".into();
    assert_eq!(
        zmk::cell_groups(&property(&app, "bindings")).unwrap(),
        expected
    );
    app.dispatch(Command::MoveMacroStep { index: 1, delta: 1 })
        .unwrap();
    expected.swap(1, 2);
    assert_eq!(
        zmk::cell_groups(&property(&app, "bindings")).unwrap(),
        expected
    );
    app.dispatch(Command::Undo).unwrap();
    expected.swap(1, 2);
    assert_eq!(
        zmk::cell_groups(&property(&app, "bindings")).unwrap(),
        expected
    );
}

#[test]
fn macro_expression_edits_validate_locally_and_preserve_other_groups() {
    let mut app = app("&kp A");
    definition(&mut app, Destination::Macros, Section::Macros, "copy");
    let original = zmk::cell_groups(&property(&app, "bindings")).unwrap();
    app.dispatch(Command::SetMacroExpression {
        index: 1,
        source: "&kp".into(),
    })
    .unwrap();
    assert_eq!(
        zmk::cell_groups(&property(&app, "bindings")).unwrap(),
        original
    );
    assert!(app.editor.as_ref().unwrap().error.is_some());
    assert_eq!(app.editor.as_ref().unwrap().sequence_group(1), "&kp");
    app.dispatch(Command::SetMacroExpression {
        index: 1,
        source: "&macro_release &kp RC(C)".into(),
    })
    .unwrap();
    let mut expected = original.clone();
    expected[1] = "&macro_release &kp RC(C)".into();
    assert_eq!(
        zmk::cell_groups(&property(&app, "bindings")).unwrap(),
        expected
    );
    assert!(!app.has_unapplied_draft());
    app.dispatch(Command::Undo).unwrap();
    assert_eq!(
        zmk::cell_groups(&property(&app, "bindings")).unwrap(),
        original
    );
}

#[test]
fn inserted_macro_controls_do_not_introduce_implicit_tap_mode() {
    let mut app = app("&kp A");
    definition(&mut app, Destination::Macros, Section::Macros, "copy");
    let original = zmk::cell_groups(&property(&app, "bindings")).unwrap();
    app.dispatch(Command::InsertMacroStep(1)).unwrap();
    assignment(
        &mut app,
        BindingTarget::Field {
            field: "step-bindings-1".into(),
            binding: 0,
        },
        BindingEdit::Behavior("&macro_wait_time".into()),
    );
    let groups = zmk::cell_groups(&property(&app, "bindings")).unwrap();
    assert_eq!(groups[1], "&macro_wait_time 0");
    assert_eq!(groups[0], original[0]);
    assert_eq!(groups[2..], original[1..]);
    app.dispatch(Command::RemoveMacroStep(1)).unwrap();
    assert_eq!(
        zmk::cell_groups(&property(&app, "bindings")).unwrap(),
        original
    );
}

#[test]
fn conditional_rules_validate_duplicate_missing_and_self_referencing_layers() {
    let mut app = app("&kp A");
    definition(
        &mut app,
        Destination::ConditionalLayers,
        Section::ConditionalLayers,
        "adjust",
    );
    let before = app.project.clone();
    for value in ["1 1", "", "1 9", "1 3"] {
        field(&mut app, "if-layers", value);
        assert_eq!(app.project, before);
        assert!(app.editor.as_ref().unwrap().error.is_some());
        assert!(app.dispatch(Command::Undo).is_err());
        app.dispatch(Command::RevertDefinition).unwrap();
    }
    app.dispatch(Command::ToggleEditorLayer {
        field: "if-layers".into(),
        layer: LayerId(0),
    })
    .unwrap();
    assert_eq!(property(&app, "if-layers"), "<1 2 0>");
    assert_eq!(property(&app, "vendor-property"), "<54>");
}

#[test]
fn encoder_parameter_kinds_and_direction_order_follow_sensor_metadata() {
    let mut app = app("&kp A");
    definition(
        &mut app,
        Destination::Encoders,
        Section::Encoders,
        "Encoder 0",
    );
    let catalog = bindings::sensor_catalog(app.project.zmk.as_ref());
    let rotation = catalog
        .iter()
        .find(|spec| spec.reference == "&rotate_custom")
        .unwrap();
    assert_eq!(rotation.parameters[0].kind, ParameterKind::Layer);
    assert_eq!(rotation.parameters[1].kind, ParameterKind::KeyCode);
    argument(&mut app, BindingTarget::Sensor, 0, "2");
    assignment(
        &mut app,
        BindingTarget::Sensor,
        BindingEdit::Modifier {
            index: 1,
            modifier: "RA".into(),
        },
    );
    assert_eq!(
        app.assignment_source(&BindingTarget::Sensor).unwrap(),
        "&rotate_custom 2 RA(A)"
    );
    let document = app.project.zmk.as_ref().unwrap();
    assert!(
        document
            .source
            .contains("sensor-bindings = <&rotate_custom 2 RA(A)>")
    );
    assert!(!document.source.contains("\n    sensor-binding ="));
}

#[test]
fn selection_and_device_preferences_do_not_stage_configuration_changes() {
    let mut app = app("&dance");
    let before = app.project.clone();
    key(&mut app);
    assert_eq!(app.binding_form().unwrap().arguments.len(), 0);
    app.dispatch(Command::SetAppearance(Appearance::Light))
        .unwrap();
    app.dispatch(Command::Navigate(Destination::Combos))
        .unwrap();
    app.dispatch(Command::Navigate(Destination::Keymap))
        .unwrap();
    assert_eq!(app.project, before);
    assert!(!app.has_changes());
    assert!(app.build.output.is_empty());
}

#[test]
fn valid_layer_renames_stage_without_a_second_apply_operation() {
    let mut app = app("&kp A");
    app.dispatch(Command::BeginLayerRename(LayerId(1))).unwrap();
    app.dispatch(Command::SetLayerName("Symbols".into()))
        .unwrap();
    assert_eq!(app.project.keymap.layers[1].name, "Symbols");
    let before = app.project.clone();
    app.dispatch(Command::SetLayerName("".into())).unwrap();
    assert_eq!(app.project, before);
    assert!(app.layer_name_draft.as_ref().unwrap().error.is_some());
    app.dispatch(Command::CancelLayerRename).unwrap();
    app.dispatch(Command::Undo).unwrap();
    assert_eq!(app.project.keymap.layers[1].name, "Layer 1");
}

#[test]
fn build_results_keep_previous_successes_separate_from_failures_and_stale_events() {
    let mut app = Application::sample().unwrap();
    app.dispatch(Command::StartBuild { fail: false }).unwrap();
    let first = app.build.id;
    app.dispatch(Command::BuildResult(build_step(first, 0, false)))
        .unwrap();
    app.dispatch(Command::BuildResult(build_step(first, 5, false)))
        .unwrap();
    assert_eq!(app.artifact_history.len(), 1);
    app.dispatch(Command::StartBuild { fail: true }).unwrap();
    let second = app.build.id;
    assert!(app.build.artifacts.is_empty());
    assert_eq!(app.artifact_history[0].build, first);
    app.dispatch(Command::BuildResult(build_step(second, 5, true)))
        .unwrap();
    app.dispatch(Command::BuildResult(build_step(first, 5, false)))
        .unwrap();
    assert_eq!(app.build.status, BuildStatus::Failed);
    assert_eq!(app.artifact_history.len(), 1);
    assert_ne!(app.artifact_history[0].build, app.build.id);
}

#[test]
fn streaming_diagnostics_retain_earlier_errors_and_target_context() {
    let mut app = Application::sample().unwrap();
    app.dispatch(Command::StartBuild { fail: false }).unwrap();
    let id = app.build.id;
    let mut event = build_step(id, 0, false);
    event.line = "Building target 1/2: corne_left\ncorne.keymap:12: error: bad binding".into();
    app.dispatch(Command::BuildResult(event)).unwrap();
    let mut event = build_step(id, 1, false);
    event.line = "later compiler output\n".repeat(1000);
    app.dispatch(Command::BuildResult(event)).unwrap();
    assert_eq!(app.build.problems[0].target.as_deref(), Some("corne_left"));
    assert!(
        app.build
            .output
            .iter()
            .any(|line| line.contains("bad binding"))
    );
}

#[test]
fn a_running_local_build_cannot_be_replaced_by_save_and_build() {
    let mut app = app("&kp A");
    app.build.id = 77;
    app.build.status = BuildStatus::Running;
    assert!(app.dispatch(Command::SaveAndBuild).is_err());
    assert_eq!(app.build.id, 77);
    assert_eq!(app.build.status, BuildStatus::Running);
}

#[test]
fn artifact_export_is_explicit_and_never_overwrites_existing_firmware() {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "keyforge-v05-export-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(root.join("export")).unwrap();
    let source = root.join("corne_left.uf2");
    fs::write(&source, b"firmware snapshot").unwrap();
    let target = export_artifact(&source, &root.join("export")).unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"firmware snapshot");
    fs::write(&target, b"existing user firmware").unwrap();
    assert!(export_artifact(&source, &root.join("export")).is_err());
    assert_eq!(fs::read(&target).unwrap(), b"existing user firmware");
    assert_eq!(fs::read(&source).unwrap(), b"firmware snapshot");
    fs::remove_dir_all(root).unwrap();
}
