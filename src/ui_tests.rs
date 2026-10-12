//! Real GPUI TestAppContext / VisualTestContext event-handler interaction tests.
//! Exercises lighting controls, navigation, profiles, persistence readback,
//! and macro editor recording guard against the public contract.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use aulaf75_linux::model::{Catalogue, Workspace};
use gpui::{point, px, size, AvailableSpace, Modifiers, TestAppContext, VisualTestContext};
use crate::ui::{AppView, Page};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

struct TempDirGuard {
    path: PathBuf,
}

impl TempDirGuard {
    fn create(prefix: &str) -> Self {
        let mut base = std::env::temp_dir();
        let pid = std::process::id();
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let seq = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        base.push(format!("{prefix}_{pid}_{ts}_{seq}"));
        fs::create_dir_all(&base).expect("failed to create temporary scratch directory");
        Self { path: base }
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Helper that drives a full GPUI frame draw of the root AppView and finds the center of a debug selector.
fn find_center(
    cx: &mut VisualTestContext,
    view: &gpui::Entity<AppView>,
    selector: &'static str,
) -> gpui::Point<gpui::Pixels> {
    let cloned_view = view.clone();
    cx.draw(
        point(px(0.), px(0.)),
        size(AvailableSpace::Definite(px(1200.)), AvailableSpace::Definite(px(800.))),
        |_window, _app_cx| cloned_view.clone(),
    );
    let bounds = cx
        .debug_bounds(selector)
        .unwrap_or_else(|| panic!("debug selector '{selector}' was not found in rendered frame"));
    bounds.center()
}

/// Helper to click on an element by its debug selector.
fn click_selector(
    cx: &mut VisualTestContext,
    view: &gpui::Entity<AppView>,
    selector: &'static str,
) {
    let pt = find_center(cx, view, selector);
    cx.simulate_click(pt, Modifiers::default());
}

#[gpui::test]
fn test_admin_connect_button_does_not_prompt_while_busy(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_window, cx| AppView::new(cx));
    view.update(cx, |app, _| {
        app.busy = true;
        app.status = "Existing device operation".into();
    });
    click_selector(cx, &view, "connect-admin");
    view.update(cx, |app, _| {
        assert!(app.busy);
        assert_eq!(app.status, "Existing device operation");
        assert!(app.hardware.is_none());
    });
}

#[gpui::test]
fn test_ui_navigation_via_rendered_clicks(cx: &mut TestAppContext) {
    let temp = TempDirGuard::create("aula_ui_nav");
    let (view, cx) = cx.add_window_view(|_window, app_cx| AppView::new(app_cx));

    // Reset workspace to clean isolated state with custom storage path
    view.update(cx, |app, _cx| {
        app.workspace = Workspace::new(&app.catalogue);
        app.storage_path = temp.path.join("workspace.json");
        app.page = Page::Lighting;
    });

    // 1. Click Keyboard tab
    click_selector(cx, &view, "nav-Keyboard");
    view.update(cx, |app, _cx| {
        assert_eq!(app.page, Page::Keyboard, "navigation to Keyboard page failed");
    });

    // 2. Click Macros tab
    click_selector(cx, &view, "nav-Macros");
    view.update(cx, |app, _cx| {
        assert_eq!(app.page, Page::Macros, "navigation to Macros page failed");
    });

    // 3. Click Profiles tab
    click_selector(cx, &view, "nav-Profiles");
    view.update(cx, |app, _cx| {
        assert_eq!(app.page, Page::Profiles, "navigation to Profiles page failed");
    });

    // 4. Click Lighting tab
    click_selector(cx, &view, "nav-Lighting");
    view.update(cx, |app, _cx| {
        assert_eq!(app.page, Page::Lighting, "navigation back to Lighting page failed");
    });
}

#[gpui::test]
fn test_ui_lighting_controls_modify_draft(cx: &mut TestAppContext) {
    let temp = TempDirGuard::create("aula_ui_lighting");
    let (view, cx) = cx.add_window_view(|_window, app_cx| AppView::new(app_cx));

    view.update(cx, |app, _cx| {
        app.workspace = Workspace::new(&app.catalogue);
        app.storage_path = temp.path.join("workspace.json");
        app.page = Page::Lighting;
        let profile = app.workspace.current_mut();
        profile.effect_slot = 0;
        profile.brightness = 2;
        profile.speed = 2;
        profile.color = 0;
    });

    // Click effect-btn-3 to select slot 3
    click_selector(cx, &view, "effect-btn-3");
    view.update(cx, |app, _cx| {
        assert_eq!(
            app.workspace.current().effect_slot,
            3,
            "clicking effect-btn-3 should update profile effect_slot"
        );
    });

    // Click bright-inc
    click_selector(cx, &view, "bright-inc");
    view.update(cx, |app, _cx| {
        assert_eq!(
            app.workspace.current().brightness,
            3,
            "clicking bright-inc should increment brightness"
        );
    });

    // Click speed-inc
    click_selector(cx, &view, "speed-inc");
    view.update(cx, |app, _cx| {
        assert_eq!(
            app.workspace.current().speed,
            3,
            "clicking speed-inc should increment speed"
        );
    });

    // Click color-btn-5
    click_selector(cx, &view, "color-btn-5");
    view.update(cx, |app, _cx| {
        assert_eq!(
            app.workspace.current().color,
            5,
            "clicking color-btn-5 should set color to 5"
        );
    });
}

#[gpui::test]
fn test_ui_profiles_workflow_and_save_readback(cx: &mut TestAppContext) {
    let temp = TempDirGuard::create("aula_ui_profiles");
    let storage_file = temp.path.join("workspace.json");
    let (view, cx) = cx.add_window_view(|_window, app_cx| AppView::new(app_cx));

    view.update(cx, |app, _cx| {
        app.workspace = Workspace::new(&app.catalogue);
        app.storage_path = storage_file.clone();
        app.page = Page::Profiles;
    });

    // Initial count is 1
    view.update(cx, |app, _cx| {
        assert_eq!(app.workspace.profiles.len(), 1);
    });

    // Click profile-new-btn
    click_selector(cx, &view, "profile-new-btn");
    view.update(cx, |app, _cx| {
        assert_eq!(app.workspace.profiles.len(), 2, "profile-new-btn should add a profile");
        assert_eq!(app.workspace.selected, 1, "new profile should be selected");
    });

    // Rename the selected profile: set text in name_input, then click profile-rename-btn
    view.update(cx, |app, cx| {
        app.name_input.update(cx, |input, cx| {
            input.set_text("Gaming Rig".to_string(), cx);
        });
    });
    click_selector(cx, &view, "profile-rename-btn");
    view.update(cx, |app, _cx| {
        assert_eq!(
            app.workspace.current().name,
            "Gaming Rig",
            "profile-rename-btn should apply new name to current profile"
        );
    });

    // Save to local disk via save-local button
    click_selector(cx, &view, "save-local");

    // Read back and verify from disk model
    let loaded = Workspace::load(&storage_file, &Catalogue::bundled().unwrap())
        .expect("saved workspace should reload cleanly");
    assert_eq!(loaded.profiles.len(), 2);
    assert_eq!(loaded.profiles[1].name, "Gaming Rig");

    // Duplicate current profile
    click_selector(cx, &view, "profile-duplicate-btn");
    view.update(cx, |app, _cx| {
        assert_eq!(app.workspace.profiles.len(), 3, "duplicate should increase count to 3");
        assert!(
            app.workspace.current().name.contains("Gaming Rig"),
            "duplicated profile should derive from original name"
        );
    });

    // Delete current profile
    click_selector(cx, &view, "profile-delete-btn");
    view.update(cx, |app, _cx| {
        assert_eq!(app.workspace.profiles.len(), 2, "delete should reduce count to 2");
    });
}

#[gpui::test]
fn test_ui_macros_creation_and_events(cx: &mut TestAppContext) {
    let temp = TempDirGuard::create("aula_ui_macros");
    let (view, cx) = cx.add_window_view(|_window, app_cx| AppView::new(app_cx));

    view.update(cx, |app, _cx| {
        app.workspace = Workspace::new(&app.catalogue);
        app.storage_path = temp.path.join("workspace.json");
        app.page = Page::Macros;
    });

    // Create a new macro
    click_selector(cx, &view, "macro-new-btn");
    view.update(cx, |app, _cx| {
        let profile = app.workspace.current();
        assert_eq!(profile.macros.len(), 1, "macro-new-btn should create a macro");
        assert_eq!(app.selected_macro, Some(0), "new macro should be selected");
    });

    // Set macro delay input and usage input
    view.update(cx, |app, cx| {
        app.macro_usage_input.update(cx, |input, cx| {
            input.set_text("4".to_string(), cx); // 'A' key usage
        });
        app.macro_delay_input.update(cx, |input, cx| {
            input.set_text("50".to_string(), cx);
        });
    });

    // Add press event
    click_selector(cx, &view, "macro-add-press");
    view.update(cx, |app, _cx| {
        let m = &app.workspace.current().macros[0];
        assert_eq!(m.events.len(), 1, "macro-add-press should append an event");
        assert_eq!(m.events[0].usage, 4);
        assert!(m.events[0].pressed);
        assert_eq!(m.events[0].delay_ms, 50);
    });

    // Add release event
    click_selector(cx, &view, "macro-add-release");
    view.update(cx, |app, _cx| {
        let m = &app.workspace.current().macros[0];
        assert_eq!(m.events.len(), 2, "macro-add-release should append a release event");
        assert_eq!(m.events[1].usage, 4);
        assert!(!m.events[1].pressed);
    });

    // Increment repeat count
    click_selector(cx, &view, "macro-repeat-inc");
    view.update(cx, |app, _cx| {
        let m = &app.workspace.current().macros[0];
        assert_eq!(m.repeat, 2, "macro-repeat-inc should increment repeat count");
    });
}

#[gpui::test]
fn test_ui_macro_recording_guard_on_text_field(cx: &mut TestAppContext) {
    let temp = TempDirGuard::create("aula_ui_guard");
    let (view, cx) = cx.add_window_view(|_window, app_cx| AppView::new(app_cx));

    view.update(cx, |app, _cx| {
        app.workspace = Workspace::new(&app.catalogue);
        app.storage_path = temp.path.join("workspace.json");
        app.page = Page::Macros;
    });

    // Create a new macro
    click_selector(cx, &view, "macro-new-btn");

    // Toggle recording on
    click_selector(cx, &view, "macro-record-toggle");
    view.update(cx, |app, _cx| {
        assert!(app.recording, "macro recording should be active");
    });

    // Typing into macro-name-input text field must stop recording and not append events
    click_selector(cx, &view, "macro-name-input");
    cx.simulate_input("ComboName");

    view.update(cx, |app, _cx| {
        assert!(
            !app.recording,
            "focusing or typing into macro-name-input textfield must stop recording"
        );
        let m = &app.workspace.current().macros[0];
        assert!(
            m.events.is_empty(),
            "typing into macro-name textfield must NOT append macro events"
        );
    });
}
