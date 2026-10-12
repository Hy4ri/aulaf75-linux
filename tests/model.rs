use aulaf75_linux::model::{Catalogue, Workspace};

#[test]
fn bundled_catalogue_uses_original_first_ledopt_field() {
    let cat = Catalogue::bundled().unwrap();
    assert_eq!(cat.keys.len(), 81);
    assert_eq!(cat.effects.len(), 20);
    assert_eq!(cat.effects[3].hardware_id, 4);
    assert_eq!(cat.effects[18].hardware_id, 21);
    assert_eq!(cat.effects[19].hardware_id, 0);
}

#[test]
fn local_profile_roundtrip_preserves_edits() {
    let cat = Catalogue::bundled().unwrap();
    let mut ws = Workspace::new(&cat);
    ws.profiles[0].brightness = 3;
    ws.profiles[0].speed = 2;
    let json = ws.to_json().unwrap();
    let restored = Workspace::from_json(&json, &cat).unwrap();
    assert_eq!(restored.profiles[0].brightness, 3);
    assert_eq!(restored.profiles[0].speed, 2);
}

#[test]
fn lighting_patch_preserves_all_unedited_profile_bytes() {
    let cat = Catalogue::bundled().unwrap();
    let mut ws = Workspace::new(&cat);
    let p = &mut ws.profiles[0];
    p.effect_slot = 3;
    p.brightness = 3;
    p.speed = 2;
    p.color = 7;
    // Synthetic preservation fixture, NOT captured hardware data.
    let mut original = [0x55; 128];
    original[126] = 0x5a;
    original[127] = 0xa5;
    let patched = p.patch_lighting(&original, &cat).unwrap();
    assert_eq!(patched[10], 4);
    assert_eq!(patched[0x3a + 3 * 2], 3);
    assert_eq!(patched[0x3b + 3 * 2], 0x27);
    for i in 0..128 {
        if ![9, 10, 0x3a + 3 * 2, 0x3b + 3 * 2].contains(&i) {
            assert_eq!(patched[i], original[i], "byte {i} unexpectedly changed");
        }
    }
}

#[test]
fn invalid_state_or_missing_profile_never_gets_encoded() {
    let cat = Catalogue::bundled().unwrap();
    let mut ws = Workspace::new(&cat);
    assert!(ws.profiles[0].patch_lighting(&[0; 128], &cat).is_err());
    ws.profiles[0].brightness = 10;
    assert!(ws.validate(&cat).is_err());
    assert!(Workspace::from_json("{\"version\":999}", &cat).is_err());
}
