use keyforge_core::{
    Application, Behavior, Binding, BindingDraft, BuildStatus, Command, KeyCode, KeyPosition,
    LayerId, Pane, Severity,
    layout::Point,
    persistence::{LocalStorage, decode_project, encode_project},
    services::build_step,
    workspace::{BottomTab, Workspace},
};

fn press(code: &str) -> Binding {
    Binding::KeyPress(KeyCode(code.into()))
}
fn edit(app: &mut Application, layer: usize, key: usize, binding: Binding) {
    app.dispatch(Command::ChangeBinding {
        layer: LayerId(layer),
        key: KeyPosition(key),
        binding,
    })
    .unwrap();
}

#[test]
fn selection_and_inspector_are_addressed_by_layer_and_key() {
    let mut app = Application::sample().unwrap();
    app.dispatch(Command::SelectKey(Some(KeyPosition(16))))
        .unwrap();
    assert_eq!(app.draft.as_ref().unwrap().preview(), "&mt LSHFT F");
    app.dispatch(Command::SelectLayer(LayerId(1))).unwrap();
    assert_eq!(app.selected_key, Some(KeyPosition(16)));
    assert_eq!(app.draft.as_ref().unwrap().preview(), "&trans");
    assert!(app.dispatch(Command::SelectLayer(LayerId(8))).is_err());
    assert!(
        app.dispatch(Command::SelectKey(Some(KeyPosition(42))))
            .is_err()
    );
    assert_eq!(app.active_layer, LayerId(1));
}

#[test]
fn inspector_generates_and_validates_all_six_behaviors() {
    let cases = [
        (Behavior::KeyPress, ["F", ""], "&kp F"),
        (Behavior::ModTap, ["LSHFT", "F"], "&mt LSHFT F"),
        (Behavior::LayerTap, ["1", "F"], "&lt 1 F"),
        (Behavior::MomentaryLayer, ["2", ""], "&mo 2"),
        (Behavior::Transparent, ["", ""], "&trans"),
        (Behavior::None, ["", ""], "&none"),
    ];
    for (behavior, params, expected) in cases {
        let draft = BindingDraft {
            behavior,
            parameters: params.map(str::to_string).to_vec(),
            reference: None,
        };
        assert_eq!(draft.preview(), expected);
        let issues = keyforge_core::validate_binding(
            &draft.binding().unwrap(),
            LayerId(1),
            KeyPosition(16),
            4,
        );
        assert!(!issues.iter().any(|d| d.severity == Severity::Error));
    }
}

#[test]
fn invalid_drafts_do_not_change_bindings_or_history() {
    let mut app = Application::sample().unwrap();
    app.dispatch(Command::SelectKey(Some(KeyPosition(16))))
        .unwrap();
    let before = app.selected_binding().unwrap().clone();
    app.dispatch(Command::SetParameter {
        index: 0,
        value: "F".into(),
    })
    .unwrap();
    assert!(
        app.draft_diagnostics()
            .iter()
            .any(|d| d.severity == Severity::Error)
    );
    assert!(app.dispatch(Command::ApplyBinding).is_err());
    assert_eq!(app.selected_binding(), Some(&before));
    assert!(!app.can_undo());
    app.dispatch(Command::SetBehavior(Behavior::LayerTap))
        .unwrap();
    app.dispatch(Command::SetParameter {
        index: 0,
        value: "99".into(),
    })
    .unwrap();
    assert!(app.dispatch(Command::ApplyBinding).is_err());
}

#[test]
fn history_preserves_layers_and_invalidates_redo_after_new_edit() {
    let mut app = Application::sample().unwrap();
    let initial = app.project.keymap.clone();
    edit(&mut app, 0, 16, press("J"));
    edit(&mut app, 1, 16, press("K"));
    app.dispatch(Command::Undo).unwrap();
    assert_eq!(
        app.binding(LayerId(1), KeyPosition(16)),
        Some(&initial.layers[1].bindings[16])
    );
    assert_eq!(app.binding(LayerId(0), KeyPosition(16)), Some(&press("J")));
    app.dispatch(Command::Redo).unwrap();
    assert_eq!(app.binding(LayerId(1), KeyPosition(16)), Some(&press("K")));
    app.dispatch(Command::Undo).unwrap();
    edit(&mut app, 0, 16, press("L"));
    assert!(!app.can_redo());
    assert_eq!(app.changes().len(), 1);
}

#[test]
fn no_op_edits_do_not_enter_history_and_reverted_values_are_clean() {
    let mut app = Application::sample().unwrap();
    let original = app.binding(LayerId(0), KeyPosition(0)).unwrap().clone();
    edit(&mut app, 0, 0, original.clone());
    assert!(!app.can_undo());
    edit(&mut app, 0, 0, press("A"));
    assert_eq!(app.changes()[0].before, original);
    app.dispatch(Command::Undo).unwrap();
    assert!(app.changes().is_empty());
}

#[test]
fn all_visibility_combinations_reclaim_space_and_preserve_editing() {
    for mask in 0..8 {
        let mut app = Application::sample().unwrap();
        app.dispatch(Command::SelectKey(Some(KeyPosition(16))))
            .unwrap();
        app.dispatch(Command::SetParameter {
            index: 1,
            value: "J".into(),
        })
        .unwrap();
        let draft = app.draft.clone();
        for (bit, pane) in [Pane::Left, Pane::Right, Pane::Bottom]
            .into_iter()
            .enumerate()
        {
            if mask & (1 << bit) == 0 {
                app.dispatch(Command::TogglePane(pane)).unwrap();
            }
        }
        let rendered = app.workspace.rendered(1440., 800.);
        assert_eq!(rendered.left > 0., mask & 1 != 0);
        assert_eq!(rendered.right > 0., mask & 2 != 0);
        assert_eq!(rendered.bottom > 0., mask & 4 != 0);
        assert_eq!(app.draft, draft);
        assert!(!app.can_undo());
        assert_eq!(app.selected_key, Some(KeyPosition(16)));
    }
}

#[test]
fn focus_restores_every_workspace_configuration_without_persisting_override() {
    for mask in 0..8 {
        let mut app = Application::sample().unwrap();
        for (bit, pane) in [Pane::Left, Pane::Right, Pane::Bottom]
            .into_iter()
            .enumerate()
        {
            if mask & (1 << bit) == 0 {
                app.dispatch(Command::TogglePane(pane)).unwrap();
            }
        }
        app.dispatch(Command::ResizePane {
            pane: Pane::Right,
            size: 387.,
        })
        .unwrap();
        app.dispatch(Command::SelectBottomTab(BottomTab::Changes))
            .unwrap();
        let saved = app.workspace.preferences.clone();
        app.dispatch(Command::ToggleFocusMode).unwrap();
        for pane in [Pane::Left, Pane::Right, Pane::Bottom] {
            assert!(!app.workspace.visible(pane));
        }
        assert_eq!(app.workspace.preferences, saved);
        app.dispatch(Command::ToggleFocusMode).unwrap();
        assert_eq!(app.workspace.preferences, saved);
    }
}

#[test]
fn window_constraints_do_not_change_preferred_pane_dimensions() {
    let mut workspace = Workspace::default();
    workspace.resize(Pane::Right, 500.);
    let saved = workspace.preferences.clone();
    let small = workspace.rendered(900., 500.);
    assert!(small.left + small.right <= 560.01);
    assert_eq!(workspace.preferences, saved);
    assert_eq!(workspace.rendered(1600., 900.).right, 500.);
    workspace.toggle(Pane::Right);
    workspace.toggle(Pane::Right);
    assert_eq!(workspace.preferences.right.preferred_size, 500.);
}

#[test]
fn prototype_and_preferences_round_trip_with_schema_validation() {
    let app = Application::sample().unwrap();
    let encoded = encode_project(&app.project).unwrap();
    assert_eq!(decode_project(&encoded).unwrap(), app.project);
    let preferences = app.workspace.preferences;
    let encoded = serde_json::to_string(&preferences).unwrap();
    let decoded = serde_json::from_str::<keyforge_core::workspace::Preferences>(&encoded).unwrap();
    assert_eq!(decoded, preferences);
    let broken = encoded.replace("\"schema_version\":1", "\"schema_version\":99");
    assert_ne!(broken, encoded);
    let mut project = app.project;
    project.keymap.layers[0].bindings.pop();
    assert!(encode_project(&project).is_err());
}

#[test]
fn geometry_uses_rotation_pivots_for_hit_testing_and_navigation() {
    let app = Application::sample().unwrap();
    let keyboard = &app.project.keyboard;
    assert_eq!(keyboard.keys.len(), 42);
    for key in &keyboard.keys {
        assert_eq!(keyboard.hit_test(key.center()), Some(key.position));
    }
    let thumb = &keyboard.keys[38];
    assert_eq!(thumb.height, 1.5);
    let corners = thumb.corners(0.);
    let max_x = corners
        .iter()
        .map(|p| p.x)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = corners.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
    assert!(!thumb.contains(Point { x: max_x, y: min_y }));
    assert_eq!(keyboard.navigate(KeyPosition(16), 1., 0.), KeyPosition(17));
    let camera = keyforge_core::layout::Camera::default();
    let t = camera.transform(keyboard, 800., 400.);
    let p = thumb.center();
    let round_trip = t.world(t.screen(p));
    assert!((p.x - round_trip.x).abs() < 0.0001);
    assert!((p.y - round_trip.y).abs() < 0.0001);
}

#[test]
fn build_lifecycle_cancellation_restart_and_stale_results() {
    let mut app = Application::sample().unwrap();
    app.dispatch(Command::StartBuild { fail: false }).unwrap();
    let old = app.build.id;
    assert_eq!(app.build.status, BuildStatus::Queued);
    app.dispatch(Command::CancelBuild).unwrap();
    app.dispatch(Command::BuildResult(build_step(old, 0, false)))
        .unwrap();
    assert_eq!(app.build.status, BuildStatus::Cancelled);
    app.dispatch(Command::StartBuild { fail: false }).unwrap();
    let current = app.build.id;
    assert!(current > old);
    app.dispatch(Command::BuildResult(build_step(old, 5, false)))
        .unwrap();
    assert_eq!(app.build.status, BuildStatus::Queued);
    for step in 0..6 {
        app.dispatch(Command::BuildResult(build_step(current, step, false)))
            .unwrap();
    }
    assert_eq!(app.build.status, BuildStatus::Successful);
    assert_eq!(app.build.artifacts.len(), 2);
    app.dispatch(Command::BuildResult(build_step(current, 0, false)))
        .unwrap();
    assert_eq!(app.build.status, BuildStatus::Successful);
    app.dispatch(Command::StartBuild { fail: true }).unwrap();
    for step in 0..6 {
        app.dispatch(Command::BuildResult(build_step(app.build.id, step, true)))
            .unwrap();
    }
    assert_eq!(app.build.status, BuildStatus::Failed);
}

#[test]
fn local_storage_is_prototype_only_and_preserves_saved_baseline() {
    let unique = format!(
        "keyforge-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let directory = std::env::temp_dir().join(unique);
    let storage = LocalStorage {
        directory: directory.clone(),
    };
    let mut app = Application::sample().unwrap();
    edit(&mut app, 0, 16, press("J"));
    storage.save_project(&app.project).unwrap();
    storage
        .save_preferences(&app.workspace.preferences)
        .unwrap();
    assert_eq!(storage.load_project().unwrap(), app.project);
    assert_eq!(
        storage.load_preferences().unwrap(),
        app.workspace.preferences
    );
    let mut reloaded = Application::from_project(storage.load_project().unwrap()).unwrap();
    assert!(reloaded.changes().is_empty());
    edit(&mut reloaded, 0, 16, press("K"));
    reloaded.dispatch(Command::Undo).unwrap();
    assert!(reloaded.changes().is_empty());
    std::fs::remove_dir_all(directory).unwrap();
}

fn temporary_storage() -> LocalStorage {
    let unique = format!(
        "keyforge-worker-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    LocalStorage {
        directory: std::env::temp_dir().join(unique),
    }
}
fn drain_services(app: &mut Application) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while app.needs_poll() && std::time::Instant::now() < deadline {
        app.poll();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(!app.needs_poll(), "Storage worker did not complete");
}

#[test]
fn save_acknowledges_written_snapshot_and_retains_later_edits() {
    let storage = temporary_storage();
    let mut app = Application::with_storage(storage.clone()).unwrap();
    edit(&mut app, 0, 16, press("J"));
    app.dispatch(Command::SaveDocument).unwrap();
    edit(&mut app, 0, 16, press("K"));
    drain_services(&mut app);
    assert_eq!(
        storage.load_project().unwrap().keymap.layers[0].bindings[16],
        press("J")
    );
    let changes = app.changes();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].before, press("J"));
    assert_eq!(changes[0].after, press("K"));
    app.dispatch(Command::Undo).unwrap();
    assert!(app.changes().is_empty());
    app.dispatch(Command::Undo).unwrap();
    assert!(
        !app.changes().is_empty(),
        "Undo across save must become dirty"
    );
    drop(app);
    std::fs::remove_dir_all(storage.directory).unwrap();
}

#[test]
fn save_failure_preserves_dirty_state_and_old_project_results_are_ignored() {
    let storage = temporary_storage();
    let mut app = Application::with_storage(storage.clone()).unwrap();
    std::fs::write(&storage.directory, "not a directory").unwrap();
    edit(&mut app, 0, 16, press("J"));
    app.dispatch(Command::SaveDocument).unwrap();
    drain_services(&mut app);
    assert_eq!(app.changes().len(), 1);
    assert!(app.status.contains("Storage"));
    drop(app);
    std::fs::remove_file(&storage.directory).unwrap();

    let mut app = Application::with_storage(storage.clone()).unwrap();
    edit(&mut app, 0, 16, press("J"));
    app.dispatch(Command::SaveDocument).unwrap();
    app.dispatch(Command::OpenSample).unwrap();
    drain_services(&mut app);
    assert!(
        app.changes().is_empty(),
        "Stale save must not replace a reopened document baseline"
    );
    drop(app);
    std::fs::remove_dir_all(storage.directory).unwrap();
}

#[test]
fn reset_layout_keeps_actual_window_dimensions_and_exits_focus() {
    let mut app = Application::sample().unwrap();
    app.dispatch(Command::WindowResized {
        width: 950.,
        height: 650.,
    })
    .unwrap();
    app.dispatch(Command::ResizePane {
        pane: Pane::Right,
        size: 450.,
    })
    .unwrap();
    app.dispatch(Command::ToggleFocusMode).unwrap();
    app.dispatch(Command::ResetWorkspaceLayout).unwrap();
    assert!(!app.workspace.focus_mode);
    assert_eq!(app.workspace.preferences.window_width, 950.);
    assert_eq!(app.workspace.preferences.window_height, 650.);
    assert_eq!(app.workspace.preferences.right.preferred_size, 320.);
}
