//! Interactive 81-key layout view and 4-layer draft remapping editor.
//!
//! Note on hardware boundary:
//! Transport and matrix encoding for key remappings and macros are not validated
//! on this firmware revision yet. All remap and macro changes made here update
//! local draft profile state only. Saving writes to local JSON workspaces; no hardware
//! remapping commands are sent to the device.

use crate::{
    theme,
    ui::{button, heading, hint, Action, AppView},
};
use aulaf75_linux::model::{Binding, KeySpec};
use gpui::{prelude::*, *};

/// 4 Layer names corresponding to firmware/software layer designations.
const LAYER_NAMES: [&str; 4] = ["Base", "Fn1", "Fn2", "Fn3"];

/// Standard HID usage lookup table for common keys to display user-friendly names.
fn hid_usage_name(usage: u8) -> &'static str {
    match usage {
        0x04 => "A",
        0x05 => "B",
        0x06 => "C",
        0x07 => "D",
        0x08 => "E",
        0x09 => "F",
        0x0a => "G",
        0x0b => "H",
        0x0c => "I",
        0x0d => "J",
        0x0e => "K",
        0x0f => "L",
        0x10 => "M",
        0x11 => "N",
        0x12 => "O",
        0x13 => "P",
        0x14 => "Q",
        0x15 => "R",
        0x16 => "S",
        0x17 => "T",
        0x18 => "U",
        0x19 => "V",
        0x1a => "W",
        0x1b => "X",
        0x1c => "Y",
        0x1d => "Z",
        0x1e => "1",
        0x1f => "2",
        0x20 => "3",
        0x21 => "4",
        0x22 => "5",
        0x23 => "6",
        0x24 => "7",
        0x25 => "8",
        0x26 => "9",
        0x27 => "0",
        0x28 => "Enter",
        0x29 => "Escape",
        0x2a => "Backspace",
        0x2b => "Tab",
        0x2c => "Space",
        0x2d => "- _",
        0x2e => "= +",
        0x2f => "[ {",
        0x30 => "] }",
        0x31 => "\\ |",
        0x33 => "; :",
        0x34 => "' \"",
        0x35 => "` ~",
        0x36 => ", <",
        0x37 => ". >",
        0x38 => "/ ?",
        0x39 => "CapsLock",
        0x3a => "F1",
        0x3b => "F2",
        0x3c => "F3",
        0x3d => "F4",
        0x3e => "F5",
        0x3f => "F6",
        0x40 => "F7",
        0x41 => "F8",
        0x42 => "F9",
        0x43 => "F10",
        0x44 => "F11",
        0x45 => "F12",
        0x46 => "PrintScreen",
        0x47 => "ScrollLock",
        0x48 => "Pause",
        0x49 => "Insert",
        0x4a => "Home",
        0x4b => "PageUp",
        0x4c => "Delete",
        0x4d => "End",
        0x4e => "PageDown",
        0x4f => "Right",
        0x50 => "Left",
        0x51 => "Down",
        0x52 => "Up",
        0x53 => "NumLock",
        0xe0 => "LCtrl",
        0xe1 => "LShift",
        0xe2 => "LAlt",
        0xe3 => "LMeta",
        0xe4 => "RCtrl",
        0xe5 => "RShift",
        0xe6 => "RAlt",
        0xe7 => "RMeta",
        _ => "Unknown",
    }
}

/// Consumer media usage display labels.
fn media_usage_name(usage: u16) -> &'static str {
    match usage {
        0x00cd => "Play/Pause",
        0x00e9 => "Volume Up",
        0x00ea => "Volume Down",
        0x00e2 => "Mute",
        0x00b5 => "Next Track",
        0x00b6 => "Prev Track",
        0x00b7 => "Stop",
        0x018a => "Mail",
        0x0192 => "Calculator",
        0x0223 => "Browser Home",
        _ => "Media Key",
    }
}

/// Clean VK or raw labels for display.
fn human_label(raw: &str) -> &str {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix("//") {
        rest.trim()
    } else if let Some(rest) = trimmed.strip_prefix(';') {
        rest.trim()
    } else {
        trimmed
    }
}

/// Format modifier mask into human-readable string.
fn format_modifiers(mods: u8) -> String {
    let mut parts = Vec::new();
    if mods & 1 != 0 {
        parts.push("Ctrl");
    }
    if mods & 2 != 0 {
        parts.push("Shift");
    }
    if mods & 4 != 0 {
        parts.push("Alt");
    }
    if mods & 8 != 0 {
        parts.push("Super");
    }
    if parts.is_empty() {
        "None".to_string()
    } else {
        parts.join(" + ")
    }
}

pub fn render(app: &mut AppView, window: &mut Window, cx: &mut Context<AppView>) -> Div {
    let selected_id = app.selected_key;
    let selected_layer = app.layer as usize;
    let pending_usage = app.pending_usage;
    let pending_mods = app.pending_modifiers;

    let profile = app.workspace.current();
    let current_remap_key = format!("{selected_layer}:{selected_id}");
    let current_binding = profile.remaps.get(&current_remap_key).cloned();

    let selected_spec: Option<KeySpec> = app
        .catalogue
        .keys
        .iter()
        .find(|k| k.id == selected_id)
        .cloned();

    // 1. Layer selection tabs (Base, Fn1, Fn2, Fn3)
    let layer_selector = div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .text_sm()
                .mr_2()
                .child("Layer:"),
        )
        .children((0..4).map(|layer_idx| {
            let active = layer_idx == selected_layer;
            let id: SharedString = format!("layer-btn-{layer_idx}").into();
            let label: SharedString = LAYER_NAMES[layer_idx].into();
            button(cx, id, label, Action::Layer(layer_idx as u8), active)
        }));

    // 2. Hardware boundary warning banner
    let warning_banner = div()
        .flex()
        .items_center()
        .justify_between()
        .px_3()
        .py_2()
        .rounded_md()
        .bg(theme::panel())
        .border_1()
        .border_color(theme::border())
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::BOLD)
                        .text_color(theme::accent())
                        .child("LOCAL DRAFT MODE:"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::muted())
                        .child("Key remap & macro hardware transport/matrix encoder not yet validated. All changes save to local profile JSON workspaces; no hardware writes performed."),
                ),
        );

    // 3. Interactive keyboard layout rendered from catalogue coordinates
    // Source coords: min_x = 30.0, max_x_right = 630.0 (span = 600.0)
    // min_y = 30.0, max_y_bottom = 263.0 (span = 233.0)
    // Scale factor: 720.0 / 600.0 = 1.20 so container width is ~730px, fitting comfortably in 800px max
    let canvas_width = (f32::from(window.viewport_size().width) - 218.0).clamp(300.0, 730.0);
    let scale: f32 = (canvas_width - 12.0) / 600.0;
    let min_x: f32 = 30.0;
    let min_y: f32 = 30.0;

    let mut regular_keys = Vec::new();
    let mut wheel_key: Option<KeySpec> = None;

    for k in &app.catalogue.keys {
        if k.id == 81 || (k.width == 0.0 && k.height == 0.0) {
            wheel_key = Some(k.clone());
        } else {
            regular_keys.push(k.clone());
        }
    }

    let keyboard_canvas = div()
        .relative()
        .w(px(canvas_width))
        .h(px(233.0 * scale + 12.0))
        .bg(theme::panel())
        .rounded_md()
        .border_1()
        .border_color(theme::border())
        .children(regular_keys.into_iter().map(|k| {
            let left_px = (k.x - min_x) * scale + 6.0;
            let top_px = (k.y - min_y) * scale + 6.0;
            let width_px = (k.width * scale - 2.0).max(18.0);
            let height_px = (k.height * scale - 2.0).max(18.0);

            let is_selected = k.id == selected_id;
            let remap_key = format!("{selected_layer}:{}", k.id);
            let has_remap = profile.remaps.contains_key(&remap_key);

            let id_str: SharedString = format!("kkey-{}", k.id).into();
            let label_str = human_label(&k.label);

            let bg_color = if is_selected {
                theme::accent()
            } else if has_remap {
                theme::input_bg()
            } else {
                theme::panel()
            };

            let text_color = if is_selected {
                theme::background()
            } else if has_remap {
                theme::accent()
            } else {
                theme::foreground()
            };

            let border_color = if is_selected || has_remap {
                theme::accent()
            } else {
                theme::border()
            };

            div()
                .id(id_str)
                .absolute()
                .left(px(left_px))
                .top(px(top_px))
                .w(px(width_px))
                .h(px(height_px))
                .rounded_sm()
                .border_1()
                .border_color(border_color)
                .bg(bg_color)
                .text_color(text_color)
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(|s| s.bg(theme::input_bg()))
                .on_click(cx.listener({
                    let kid = k.id;
                    move |this, _, window, cx| this.dispatch(Action::SelectKey(kid), window, cx)
                }))
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::BOLD)
                        .overflow_hidden()
                        .child(label_str.to_string()),
                )
        }))
        // Place key 81 (Rotary Wheel) in top right knob slot
        .children(wheel_key.map(|k| {
            let is_selected = k.id == selected_id;
            let remap_key = format!("{selected_layer}:{}", k.id);
            let has_remap = profile.remaps.contains_key(&remap_key);

            let id_str: SharedString = format!("kkey-{}", k.id).into();
            let label_str = human_label(&k.label);

            let bg_color = if is_selected {
                theme::accent()
            } else if has_remap {
                theme::input_bg()
            } else {
                theme::panel()
            };

            let text_color = if is_selected {
                theme::background()
            } else if has_remap {
                theme::accent()
            } else {
                theme::foreground()
            };

            let border_color = if is_selected || has_remap {
                theme::accent()
            } else {
                theme::border()
            };

            div()
                .id(id_str)
                .absolute()
                .right(px(10.0))
                .top(px(6.0))
                .w(px(34.0))
                .h(px(30.0))
                .rounded_full()
                .border_1()
                .border_color(border_color)
                .bg(bg_color)
                .text_color(text_color)
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(|s| s.bg(theme::input_bg()))
                .on_click(cx.listener({
                    let kid = k.id;
                    move |this, _, window, cx| this.dispatch(Action::SelectKey(kid), window, cx)
                }))
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::BOLD)
                        .child(label_str.to_string()),
                )
        }));

    // 4. Selected Key Information Card
    let selected_info_card = if let Some(spec) = &selected_spec {
        let binding_str = match &current_binding {
            Some(Binding::Key { usage, modifiers }) => {
                format!(
                    "HID Key: {} (0x{:02x}) | Mods: {}",
                    hid_usage_name(*usage),
                    usage,
                    format_modifiers(*modifiers)
                )
            }
            Some(Binding::Media { usage }) => {
                format!("Media: {} (0x{:04x})", media_usage_name(*usage), usage)
            }
            Some(Binding::Disabled) => "Disabled".to_string(),
            Some(Binding::Macro { index }) => {
                let macro_name = profile
                    .macros
                    .get(*index)
                    .map(|m| m.name.as_str())
                    .unwrap_or("Unknown Macro");
                format!("Macro [{}]: {}", index + 1, macro_name)
            }
            None => "Default Hardware Binding (No local remap)".to_string(),
        };

        div()
            .flex()
            .flex_col()
            .gap_1()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(theme::border())
            .bg(theme::panel())
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!(
                                "Selected Key: #{} \"{}\" (Matrix Slot: {}, VK: {})",
                                spec.id,
                                human_label(&spec.label),
                                spec.matrix_index,
                                spec.virtual_key
                            )),
                    )
                    .child(div().text_xs().text_color(theme::muted()).child(format!(
                        "Target: Layer {} ({})",
                        selected_layer, LAYER_NAMES[selected_layer]
                    ))),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_xs()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme::muted())
                            .child("Current Binding:"),
                    )
                    .child(
                        div()
                            .font_weight(FontWeight::BOLD)
                            .text_color(if current_binding.is_some() {
                                theme::accent()
                            } else {
                                theme::foreground()
                            })
                            .child(binding_str),
                    ),
            )
    } else {
        div()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(theme::border())
            .bg(theme::panel())
            .child("No key selected")
    };

    // 5. Remapping Editor Controls
    // Section A: HID Standard Key Selection
    let quick_keys: [(&str, u8); 16] = [
        ("A", 0x04),
        ("B", 0x05),
        ("C", 0x06),
        ("D", 0x07),
        ("E", 0x08),
        ("F", 0x09),
        ("1", 0x1e),
        ("2", 0x1f),
        ("3", 0x20),
        ("Space", 0x2c),
        ("Enter", 0x28),
        ("Esc", 0x29),
        ("Tab", 0x2b),
        ("Bksp", 0x2a),
        ("Del", 0x4c),
        ("Play/P", 0xcd), // Note: media usage handled separately, but included for reference
    ];

    let quick_key_row = div().flex().flex_wrap().gap_1().children(
        quick_keys
            .iter()
            .filter(|(_, u)| *u != 0xcd)
            .map(|(name, target_u)| {
                let delta = (*target_u as i16) - (pending_usage as i16);
                let is_current = pending_usage == *target_u;
                let id: SharedString = format!("quick-hid-{target_u}").into();
                button(cx, id, *name, Action::Usage(delta), is_current)
            }),
    );

    // Fine usage increments (Action::Usage)
    let usage_stepper = div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .child("Fine Adjust:"),
        )
        .child(button(cx, "hid-sub-10", "-10", Action::Usage(-10), false))
        .child(button(cx, "hid-sub-1", "-1", Action::Usage(-1), false))
        .child(
            div()
                .px_3()
                .py_1()
                .rounded_md()
                .bg(theme::input_bg())
                .border_1()
                .border_color(theme::border())
                .font_weight(FontWeight::BOLD)
                .text_sm()
                .child(format!(
                    "{} (0x{:02x} / {})",
                    hid_usage_name(pending_usage),
                    pending_usage,
                    pending_usage
                )),
        )
        .child(button(cx, "hid-add-1", "+1", Action::Usage(1), false))
        .child(button(cx, "hid-add-10", "+10", Action::Usage(10), false));

    // Section B: Modifiers toggles (Ctrl 1, Shift 2, Alt 4, Super 8)
    let modifier_toggles = div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .child("Modifiers:"),
        )
        .child(button(
            cx,
            "mod-ctrl",
            "Ctrl",
            Action::Modifier(1),
            pending_mods & 1 != 0,
        ))
        .child(button(
            cx,
            "mod-shift",
            "Shift",
            Action::Modifier(2),
            pending_mods & 2 != 0,
        ))
        .child(button(
            cx,
            "mod-alt",
            "Alt",
            Action::Modifier(4),
            pending_mods & 4 != 0,
        ))
        .child(button(
            cx,
            "mod-super",
            "Super",
            Action::Modifier(8),
            pending_mods & 8 != 0,
        ))
        .child(
            div()
                .ml_2()
                .text_xs()
                .text_color(theme::muted())
                .child(format!("Active: {}", format_modifiers(pending_mods))),
        );

    // Apply Key remap button
    let apply_key_row = div()
        .flex()
        .items_center()
        .gap_3()
        .child(button(
            cx,
            "action-bind-key",
            "Assign Key + Modifiers",
            Action::BindKey,
            false,
        ))
        .child(button(
            cx,
            "action-disable-key",
            "Disable Key",
            Action::DisableKey,
            matches!(current_binding, Some(Binding::Disabled)),
        ))
        .child(button(
            cx,
            "action-reset-key",
            "Reset to Default",
            Action::ResetKey,
            current_binding.is_none(),
        ));

    // Section C: Media Key Shortcuts
    let media_buttons = div()
        .flex()
        .flex_wrap()
        .gap_2()
        .child(button(
            cx,
            "media-playpause",
            "Play/Pause (0xCD)",
            Action::BindMedia(0x00cd),
            matches!(current_binding, Some(Binding::Media { usage: 0x00cd })),
        ))
        .child(button(
            cx,
            "media-volup",
            "Volume Up (0xE9)",
            Action::BindMedia(0x00e9),
            matches!(current_binding, Some(Binding::Media { usage: 0x00e9 })),
        ))
        .child(button(
            cx,
            "media-voldown",
            "Volume Down (0xEA)",
            Action::BindMedia(0x00ea),
            matches!(current_binding, Some(Binding::Media { usage: 0x00ea })),
        ))
        .child(button(
            cx,
            "media-mute",
            "Mute (0xE2)",
            Action::BindMedia(0x00e2),
            matches!(current_binding, Some(Binding::Media { usage: 0x00e2 })),
        ))
        .child(button(
            cx,
            "media-next",
            "Next Track (0xB5)",
            Action::BindMedia(0x00b5),
            matches!(current_binding, Some(Binding::Media { usage: 0x00b5 })),
        ))
        .child(button(
            cx,
            "media-prev",
            "Prev Track (0xB6)",
            Action::BindMedia(0x00b6),
            matches!(current_binding, Some(Binding::Media { usage: 0x00b6 })),
        ));

    // Section D: Macro Assignment
    let macro_assignment = if profile.macros.is_empty() {
        div().text_xs().text_color(theme::muted()).child(
            "No macros configured in current profile. Create macros in the Macros tab first.",
        )
    } else {
        div()
            .flex()
            .flex_wrap()
            .gap_2()
            .children(profile.macros.iter().enumerate().map(|(idx, m)| {
                let is_assigned =
                    matches!(current_binding, Some(Binding::Macro { index }) if index == idx);
                let id: SharedString = format!("bind-macro-{idx}").into();
                let label: SharedString = format!("{}. {}", idx + 1, m.name).into();
                button(cx, id, label, Action::BindMacro(idx), is_assigned)
            }))
    };

    // Container assembling all sections
    div()
        .flex()
        .flex_col()
        .gap_4()
        .w_full()
        .max_w(px(800.0))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(heading("Key Mapping Editor"))
                .child(layer_selector),
        )
        .child(hint(
            "Select any key in the layout to configure draft bindings for the active layer.",
        ))
        .child(warning_banner)
        .child(keyboard_canvas)
        .child(selected_info_card)
        .child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .p_4()
                .rounded_md()
                .border_1()
                .border_color(theme::border())
                .bg(theme::panel())
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_sm()
                        .child("1. Standard HID Key Assignment"),
                )
                .child(quick_key_row)
                .child(usage_stepper)
                .child(modifier_toggles)
                .child(apply_key_row),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .p_4()
                .rounded_md()
                .border_1()
                .border_color(theme::border())
                .bg(theme::panel())
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_sm()
                        .child("2. Media Control Binding"),
                )
                .child(media_buttons),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .p_4()
                .rounded_md()
                .border_1()
                .border_color(theme::border())
                .bg(theme::panel())
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_sm()
                        .child("3. Macro Assignment"),
                )
                .child(macro_assignment),
        )
}
