use aulaf75_linux::model::{Binding, Catalogue, Macro, MacroEvent, Workspace};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

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
        let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
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

fn valid_minimal_macro(name: &str) -> Macro {
    Macro {
        name: name.to_string(),
        repeat: 1,
        events: vec![MacroEvent {
            usage: 4, // 'a' key usage
            pressed: true,
            delay_ms: 10,
        }],
    }
}

#[test]
fn default_workspace_validates_against_catalogue() {
    let cat = Catalogue::bundled().expect("bundled catalogue must load");
    let ws = Workspace::new(&cat);
    assert_eq!(ws.version, 1);
    assert!(ws.validate(&cat).is_ok());
    assert_eq!(ws.current().name, ws.profiles[ws.selected].name);
}

#[test]
fn validate_rejects_empty_profiles_and_out_of_bounds_selected() {
    let cat = Catalogue::bundled().unwrap();
    let mut ws = Workspace::new(&cat);

    ws.selected = ws.profiles.len();
    assert!(
        ws.validate(&cat).is_err(),
        "selected index out of bounds must fail validation"
    );

    ws.profiles.clear();
    ws.selected = 0;
    assert!(
        ws.validate(&cat).is_err(),
        "workspace with no profiles must fail validation"
    );
}

#[test]
fn validate_rejects_version_mismatch() {
    let cat = Catalogue::bundled().unwrap();
    let mut ws = Workspace::new(&cat);
    ws.version = 0;
    assert!(ws.validate(&cat).is_err(), "version 0 must fail validation");
    ws.version = 2;
    assert!(ws.validate(&cat).is_err(), "version 2 must fail validation");
}

#[test]
fn validate_enforces_lighting_parameter_bounds() {
    let cat = Catalogue::bundled().unwrap();

    // brightness: 0..=9
    let mut ws = Workspace::new(&cat);
    ws.profiles[0].brightness = 10;
    assert!(ws.validate(&cat).is_err(), "brightness 10 must be rejected");

    // speed: 0..=4
    let mut ws = Workspace::new(&cat);
    ws.profiles[0].speed = 5;
    assert!(ws.validate(&cat).is_err(), "speed 5 must be rejected");

    // color: 0..=7
    let mut ws = Workspace::new(&cat);
    ws.profiles[0].color = 8;
    assert!(ws.validate(&cat).is_err(), "color 8 must be rejected");

    // effect_slot: must exist in catalogue
    let mut ws = Workspace::new(&cat);
    ws.profiles[0].effect_slot = cat.effects.len();
    assert!(
        ws.validate(&cat).is_err(),
        "out-of-bounds effect_slot must be rejected"
    );

    // sleep_seconds: must be a supported value in catalogue.sleep_seconds
    let mut ws = Workspace::new(&cat);
    ws.profiles[0].sleep_seconds = 777; // not in catalogue sleep list [60..1200]
    assert!(
        ws.validate(&cat).is_err(),
        "unsupported sleep_seconds must be rejected"
    );
}

#[test]
fn validate_enforces_profile_and_macro_names_nonempty() {
    let cat = Catalogue::bundled().unwrap();

    let mut ws = Workspace::new(&cat);
    ws.profiles[0].name = "".to_string();
    assert!(
        ws.validate(&cat).is_err(),
        "empty profile name must be rejected"
    );

    let mut ws2 = Workspace::new(&cat);
    ws2.profiles[0].macros.push(Macro {
        name: "".to_string(),
        repeat: 1,
        events: vec![MacroEvent {
            usage: 4,
            pressed: true,
            delay_ms: 10,
        }],
    });
    assert!(
        ws2.validate(&cat).is_err(),
        "empty macro name must be rejected"
    );
}

#[test]
fn validate_enforces_macro_repeat_and_events() {
    let cat = Catalogue::bundled().unwrap();

    // repeat must be > 0
    let mut ws = Workspace::new(&cat);
    let mut m = valid_minimal_macro("M1");
    m.repeat = 0;
    ws.profiles[0].macros.push(m);
    assert!(
        ws.validate(&cat).is_err(),
        "macro repeat=0 must be rejected"
    );

    // macro events usage must be > 0
    let mut ws2 = Workspace::new(&cat);
    let mut m2 = valid_minimal_macro("M2");
    m2.events[0].usage = 0;
    ws2.profiles[0].macros.push(m2);
    assert!(
        ws2.validate(&cat).is_err(),
        "macro event usage=0 must be rejected"
    );

    // macro event delay must be <= 0xfffff
    let mut ws3 = Workspace::new(&cat);
    let mut m3 = valid_minimal_macro("M3");
    m3.events[0].delay_ms = 0x100000;
    ws3.profiles[0].macros.push(m3);
    assert!(
        ws3.validate(&cat).is_err(),
        "macro event delay > 0xfffff must be rejected"
    );
}

#[test]
fn validate_enforces_remap_key_format_and_ranges() {
    let cat = Catalogue::bundled().unwrap();

    // Valid remap
    let mut ws = Workspace::new(&cat);
    ws.profiles[0].remaps.insert(
        "0:1".to_string(),
        Binding::Key {
            usage: 4,
            modifiers: 0,
        },
    );
    assert!(ws.validate(&cat).is_ok());

    // Layer bounds 0..=3: layer 4 is invalid
    let mut ws_bad_layer = Workspace::new(&cat);
    ws_bad_layer.profiles[0].remaps.insert(
        "4:1".to_string(),
        Binding::Key {
            usage: 4,
            modifiers: 0,
        },
    );
    assert!(
        ws_bad_layer.validate(&cat).is_err(),
        "layer 4 must be rejected"
    );

    // Key id bounds 1..=81: key id 0 is invalid
    let mut ws_bad_key0 = Workspace::new(&cat);
    ws_bad_key0.profiles[0].remaps.insert(
        "0:0".to_string(),
        Binding::Key {
            usage: 4,
            modifiers: 0,
        },
    );
    assert!(
        ws_bad_key0.validate(&cat).is_err(),
        "key id 0 must be rejected"
    );

    // Key id bounds 1..=81: key id 82 is invalid
    let mut ws_bad_key82 = Workspace::new(&cat);
    ws_bad_key82.profiles[0].remaps.insert(
        "0:82".to_string(),
        Binding::Key {
            usage: 4,
            modifiers: 0,
        },
    );
    assert!(
        ws_bad_key82.validate(&cat).is_err(),
        "key id 82 must be rejected"
    );

    // Malformed key format string
    let bad_formats = ["abc", "0", "0:", ":1", "layer0:key1", "0:1:2"];
    for bad in bad_formats {
        let mut ws_malformed = Workspace::new(&cat);
        ws_malformed.profiles[0].remaps.insert(
            bad.to_string(),
            Binding::Key {
                usage: 4,
                modifiers: 0,
            },
        );
        assert!(
            ws_malformed.validate(&cat).is_err(),
            "remap key format '{bad}' must be rejected"
        );
    }
}

#[test]
fn validate_enforces_macro_reference_index_within_bounds() {
    let cat = Catalogue::bundled().unwrap();
    let mut ws = Workspace::new(&cat);

    // Profile has 1 macro at index 0
    ws.profiles[0].macros.push(valid_minimal_macro("MyMacro"));

    // Reference to index 0 is valid
    ws.profiles[0]
        .remaps
        .insert("0:1".to_string(), Binding::Macro { index: 0 });
    assert!(ws.validate(&cat).is_ok());

    // Reference to index 1 is out of bounds (macros len is 1)
    let mut ws_oob = Workspace::new(&cat);
    ws_oob.profiles[0]
        .macros
        .push(valid_minimal_macro("MyMacro"));
    ws_oob.profiles[0]
        .remaps
        .insert("0:1".to_string(), Binding::Macro { index: 1 });
    assert!(
        ws_oob.validate(&cat).is_err(),
        "macro reference index >= macros.len() must be rejected"
    );
}

#[test]
fn json_deserialization_rejects_corrupted_or_invalid_inputs() {
    let cat = Catalogue::bundled().unwrap();

    let invalid_json_samples = [
        "",
        "{",
        "{\"version\": 1}", // missing fields
        "{\"version\": 2, \"selected\": 0, \"profiles\": []}", // version != 1
        "not json at all",
        "{\"version\": 1, \"selected\": 0, \"profiles\": [{\"name\": \"\", \"effect_slot\": 0, \"brightness\": 15, \"speed\": 0, \"color\": 0, \"sleep_seconds\": 60, \"debounce_ms\": 5, \"remaps\": {}, \"macros\": []}]}", // invalid brightness
    ];

    for (i, json) in invalid_json_samples.iter().enumerate() {
        assert!(
            Workspace::from_json(json, &cat).is_err(),
            "json sample {i} should fail deserialization or validation: {json}"
        );
    }
}

#[test]
fn local_file_save_and_load_roundtrip_integrity() {
    let temp = TempDirGuard::create("aula_editor_state_test");
    let file_path = temp.path.join("workspace.json");

    let cat = Catalogue::bundled().unwrap();
    let mut ws = Workspace::new(&cat);

    // Modify profile fields
    let profile = ws.current_mut();
    profile.name = "Custom Gaming".to_string();
    profile.effect_slot = 4;
    profile.brightness = 7;
    profile.speed = 3;
    profile.color = 5;
    profile.sleep_seconds = 300;
    profile.debounce_ms = 8;

    profile.macros.push(Macro {
        name: "Combo".to_string(),
        repeat: 3,
        events: vec![
            MacroEvent {
                usage: 4,
                pressed: true,
                delay_ms: 50,
            },
            MacroEvent {
                usage: 4,
                pressed: false,
                delay_ms: 25,
            },
        ],
    });

    profile.remaps.insert(
        "0:1".to_string(),
        Binding::Key {
            usage: 20,
            modifiers: 2,
        },
    );
    profile.remaps.insert(
        "1:14".to_string(),
        Binding::Media { usage: 0xe9 }, // volume up
    );
    profile.remaps.insert("2:2".to_string(), Binding::Disabled);
    profile
        .remaps
        .insert("0:5".to_string(), Binding::Macro { index: 0 });

    // Save
    ws.save(&file_path).expect("workspace save should succeed");
    assert!(file_path.exists(), "saved file must exist on disk");

    // Load and verify
    let loaded = Workspace::load(&file_path, &cat).expect("workspace load should succeed");
    assert_eq!(loaded.version, ws.version);
    assert_eq!(loaded.selected, ws.selected);
    assert_eq!(loaded.profiles.len(), ws.profiles.len());

    let loaded_p = loaded.current();
    assert_eq!(loaded_p.name, "Custom Gaming");
    assert_eq!(loaded_p.effect_slot, 4);
    assert_eq!(loaded_p.brightness, 7);
    assert_eq!(loaded_p.speed, 3);
    assert_eq!(loaded_p.color, 5);
    assert_eq!(loaded_p.sleep_seconds, 300);
    assert_eq!(loaded_p.debounce_ms, 8);
    assert_eq!(loaded_p.macros.len(), 1);
    assert_eq!(loaded_p.macros[0].name, "Combo");
    assert_eq!(loaded_p.macros[0].repeat, 3);
    assert_eq!(loaded_p.macros[0].events.len(), 2);
    assert_eq!(loaded_p.remaps.len(), 4);
}

#[test]
fn save_creates_missing_profile_directory_and_roundtrips() {
    let temp = TempDirGuard::create("aula_editor_err_test");
    let nonexistent_target = temp.path.join("does_not_exist_dir").join("workspace.json");

    let cat = Catalogue::bundled().unwrap();
    let ws = Workspace::new(&cat);

    ws.save(&nonexistent_target)
        .expect("First save must create the application profile directory");
    let loaded = Workspace::load(&nonexistent_target, &cat).unwrap();
    assert_eq!(loaded.current().name, ws.current().name);
    let blocker = temp.path.join("not_a_directory");
    fs::write(&blocker, "existing file").unwrap();
    assert!(ws.save(&blocker.join("workspace.json")).is_err());
    assert_eq!(fs::read_to_string(&blocker).unwrap(), "existing file");
}

#[test]
fn load_fails_on_missing_or_malformed_file() {
    let temp = TempDirGuard::create("aula_editor_load_err_test");
    let missing_path = temp.path.join("missing.json");
    let bad_path = temp.path.join("bad.json");

    let cat = Catalogue::bundled().unwrap();

    // Missing file
    assert!(Workspace::load(&missing_path, &cat).is_err());

    // Corrupted content file
    fs::write(&bad_path, "not a valid json").unwrap();
    assert!(Workspace::load(&bad_path, &cat).is_err());
}
