//! Workspace views: Profiles, Macros, and Settings pages.
//! Functional native JSON workspace import/export, profile management, macro event editor & app-window recording,
//! and local hardware-uncommitted settings drafts.

use crate::{
    theme,
    ui::{button, heading, hint, Action, AppView},
};
use gpui::{prelude::*, *};

/// Helper to render a disabled button (styled like a button, but without pointer cursor or click listener).
fn disabled_button(label: impl Into<SharedString>) -> Div {
    let label: SharedString = label.into();
    div()
        .px_3()
        .py_2()
        .rounded_md()
        .text_sm()
        .border_1()
        .border_color(theme::border())
        .bg(theme::panel())
        .text_color(theme::muted())
        .cursor_not_allowed()
        .opacity(0.5)
        .child(label)
}

/// Profiles management and import/export page.
pub fn profiles(app: &mut AppView, _window: &mut Window, cx: &mut Context<AppView>) -> Div {
    let profile_count = app.workspace.profiles.len();
    let current_name = app.workspace.current().name.clone();
    let storage_path_str = app.storage_path.display().to_string();

    div()
        .flex()
        .flex_col()
        .gap_6()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(heading("Workspace Profiles"))
                .child(hint(
                    "Manage local profiles and export/import native workspace JSON files. Profiles are stored locally and are completely independent from device firmware state.",
                )),
        )
        // Storage location status panel
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .p_4()
                .rounded_md()
                .border_1()
                .border_color(theme::border())
                .bg(theme::panel())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(div().font_weight(FontWeight::SEMIBOLD).text_sm().child("Local Storage File"))
                        .child(
                            div()
                                .text_xs()
                                .px_2()
                                .py_1()
                                .rounded_sm()
                                .bg(theme::input_bg())
                                .text_color(theme::accent())
                                .child("Local filesystem only"),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::muted())
                        .child(storage_path_str),
                ),
        )
        // Profile list and switcher
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
                .child(div().font_weight(FontWeight::SEMIBOLD).text_sm().child("Active Profile Selection"))
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_2()
                        .children(app.workspace.profiles.iter().enumerate().map(|(idx, prof)| {
                            let is_active = idx == app.workspace.selected;
                            button(
                                cx,
                                format!("profile-select-{}", idx),
                                prof.name.clone(),
                                Action::Profile(idx),
                                is_active,
                            )
                        })),
                ),
        )
        // Profile actions (Rename, New, Duplicate, Delete)
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
                        .child(format!("Edit Profile: \"{}\"", current_name)),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .debug_selector(|| "profile-name-input".into())
                                .child(app.name_input.clone()),
                        )
                        .child(button(
                            cx,
                            "profile-rename-btn",
                            "Rename",
                            Action::RenameProfile,
                            false,
                        )),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap_2()
                        .pt_2()
                        .child(button(
                            cx,
                            "profile-new-btn",
                            "New Profile",
                            Action::NewProfile,
                            false,
                        ))
                        .child(button(
                            cx,
                            "profile-duplicate-btn",
                            "Duplicate",
                            Action::DuplicateProfile,
                            false,
                        ))
                        .child(if profile_count > 1 {
                            button(
                                cx,
                                "profile-delete-btn",
                                "Delete Profile",
                                Action::DeleteProfile,
                                false,
                            )
                            .into_any_element()
                        } else {
                            disabled_button("Delete Profile").into_any_element()
                        }),
                ),
        )
        // Import & Export section
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
                .child(div().font_weight(FontWeight::SEMIBOLD).text_sm().child("Export & Import Native Workspace JSON"))
                .child(hint(
                    "Imports and exports use native workspace JSON format (not vendor binary format). Validation failures preserve the existing draft.",
                ))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .debug_selector(|| "profile-path-input".into())
                                .child(app.path_input.clone()),
                        )
                        .child(button(
                            cx,
                            "profile-export-btn",
                            "Export",
                            Action::Export,
                            false,
                        ))
                        .child(button(
                            cx,
                            "profile-import-btn",
                            "Import",
                            Action::Import,
                            false,
                        )),
                ),
        )
}

/// Macros management, event editor, and app-window recording page.
pub fn macros(app: &mut AppView, _window: &mut Window, cx: &mut Context<AppView>) -> Div {
    let selected_macro_idx = app.selected_macro;
    let current_profile = app.workspace.current();
    let macro_count = current_profile.macros.len();
    let is_recording = app.recording;

    let selected_macro = selected_macro_idx.and_then(|i| current_profile.macros.get(i));

    div()
        .flex()
        .flex_col()
        .gap_6()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(heading("Macro Editor"))
                .child(hint(
                    "Record and edit key macros for the current profile. Macros are stored locally in the workspace draft; hardware encoding is not validated.",
                )),
        )
        // Macro list & Creation bar
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
                .child(div().font_weight(FontWeight::SEMIBOLD).text_sm().child("Macros in Profile"))
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_2()
                        .children(if macro_count == 0 {
                            vec![div()
                                .text_sm()
                                .text_color(theme::muted())
                                .child("No macros in this profile. Create one below.")
                                .into_any_element()]
                        } else {
                            current_profile
                                .macros
                                .iter()
                                .enumerate()
                                .map(|(idx, m)| {
                                    let is_active = Some(idx) == selected_macro_idx;
                                    button(
                                        cx,
                                        format!("macro-select-{}", idx),
                                        format!("{} ({} ev, x{})", m.name, m.events.len(), m.repeat),
                                        Action::MacroSelect(idx),
                                        is_active,
                                    )
                                    .into_any_element()
                                })
                                .collect()
                        }),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .pt_2()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .debug_selector(|| "macro-name-input".into())
                                .child(app.macro_name_input.clone()),
                        )
                        .child(button(
                            cx,
                            "macro-new-btn",
                            "New Macro",
                            Action::MacroNew,
                            false,
                        ))
                        .child(if selected_macro.is_some() {
                            button(
                                cx,
                                "macro-rename-btn",
                                "Rename",
                                Action::MacroRename,
                                false,
                            )
                            .into_any_element()
                        } else {
                            disabled_button("Rename").into_any_element()
                        })
                        .child(if selected_macro.is_some() {
                            button(
                                cx,
                                "macro-delete-btn",
                                "Delete Macro",
                                Action::MacroDelete,
                                false,
                            )
                            .into_any_element()
                        } else {
                            disabled_button("Delete Macro").into_any_element()
                        }),
                ),
        )
        // Selected Macro Details & Recording / Event Editing
        .child(if let Some(m) = selected_macro {
            div()
                .flex()
                .flex_col()
                .gap_4()
                .p_4()
                .rounded_md()
                .border_1()
                .border_color(theme::border())
                .bg(theme::panel())
                // Header with Repeat count and Record controls
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_sm()
                                .child(format!("Selected: {} ({} events)", m.name, m.events.len())),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(div().text_sm().child(format!("Repeat: {}", m.repeat)))
                                .child(button(
                                    cx,
                                    "macro-repeat-dec",
                                    "-1",
                                    Action::MacroRepeat(-1),
                                    false,
                                ))
                                .child(button(
                                    cx,
                                    "macro-repeat-inc",
                                    "+1",
                                    Action::MacroRepeat(1),
                                    false,
                                )),
                        ),
                )
                // Recording Banner / Trigger
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .p_3()
                        .rounded_sm()
                        .border_1()
                        .border_color(if is_recording { theme::accent() } else { theme::border() })
                        .bg(theme::input_bg())
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(if is_recording { theme::accent() } else { theme::foreground() })
                                        .child(if is_recording {
                                            "● Recording in Progress"
                                        } else {
                                            "In-App Window Key Recording"
                                        }),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme::muted())
                                        .child(
                                            "Captures a-z, 0-9, Enter, Esc, Tab, Backspace, Space, and Arrow keys only while this app window is focused. Not global.",
                                        ),
                                ),
                        )
                        .child(button(
                            cx,
                            "macro-record-toggle",
                            if is_recording { "Stop Recording" } else { "Start Recording" },
                            Action::MacroRecord,
                            is_recording,
                        )),
                )
                // Manual Event Adder
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .pt_2()
                        .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Add Event Manually"))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .w(px(140.))
                                        .debug_selector(|| "macro-usage-input".into())
                                .child(app.macro_usage_input.clone()),
                                )
                                .child(
                                    div()
                                        .w(px(180.))
                                        .debug_selector(|| "macro-delay-input".into())
                                .child(app.macro_delay_input.clone()),
                                )
                                .child(button(
                                    cx,
                                    "macro-add-press",
                                    "+ Press",
                                    Action::MacroEvent(true),
                                    false,
                                ))
                                .child(button(
                                    cx,
                                    "macro-add-release",
                                    "+ Release",
                                    Action::MacroEvent(false),
                                    false,
                                )),
                        ),
                )
                // Events List (bounded scroll)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .pt_2()
                        .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Event Sequence"))
                        .child(if m.events.is_empty() {
                            div()
                                .text_sm()
                                .text_color(theme::muted())
                                .p_3()
                                .child("No events recorded or added yet.")
                                .into_any_element()
                        } else {
                            div()
                                .id("macro-event-list")
                                .max_h(px(260.))
                                .overflow_y_scroll()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .children(m.events.iter().enumerate().map(|(e_idx, event)| {
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .px_3()
                                        .py_1p5()
                                        .rounded_sm()
                                        .bg(theme::input_bg())
                                        .text_sm()
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_3()
                                                .child(
                                                    div()
                                                        .w(px(32.))
                                                        .text_color(theme::muted())
                                                        .text_xs()
                                                        .child(format!("#{}", e_idx + 1)),
                                                )
                                                .child(
                                                    div()
                                                        .w(px(70.))
                                                        .font_weight(FontWeight::SEMIBOLD)
                                                        .text_color(if event.pressed {
                                                            theme::accent()
                                                        } else {
                                                            theme::muted()
                                                        })
                                                        .child(if event.pressed { "Press" } else { "Release" }),
                                                )
                                                .child(
                                                    div()
                                                        .w(px(110.))
                                                        .child(format!("HID: 0x{:02X}", event.usage)),
                                                )
                                                .child(
                                                    div()
                                                        .text_color(theme::muted())
                                                        .child(format!("Delay: {} ms", event.delay_ms)),
                                                ),
                                        )
                                        .child(button(
                                            cx,
                                            format!("macro-event-del-{}", e_idx),
                                            "Remove",
                                            Action::MacroRemove(e_idx),
                                            false,
                                        ))
                                }))
                                .into_any_element()
                        }),
                )
                .into_any_element()
        } else {
            div()
                .p_6()
                .rounded_md()
                .border_1()
                .border_color(theme::border())
                .bg(theme::panel())
                .text_sm()
                .text_color(theme::muted())
                .child("Select or create a macro above to view and edit its events.")
                .into_any_element()
        })
}

/// Settings drafts (sleep timeout and debounce time) page.
pub fn settings(app: &mut AppView, _window: &mut Window, cx: &mut Context<AppView>) -> Div {
    let current_profile = app.workspace.current();
    let current_sleep = current_profile.sleep_seconds;
    let current_debounce = current_profile.debounce_ms;

    div()
        .flex()
        .flex_col()
        .gap_6()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(heading("Keyboard Settings (Local Draft)"))
                .child(hint(
                    "Configure sleep timeout and debounce interval. These values are saved to your local profile draft only. Device hardware encoding is unvalidated — no 'Apply Settings' or fake hardware write is performed.",
                )),
        )
        // Sleep timeout selection
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
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(div().font_weight(FontWeight::SEMIBOLD).text_sm().child("Deep Sleep Timeout"))
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme::accent())
                                .child(format_sleep(current_sleep)),
                        ),
                )
                .child(hint(
                    "Select keyboard idle duration before entering deep sleep mode. Recovered from vendor catalogue options.",
                ))
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_2()
                        .children(app.catalogue.sleep_seconds.iter().copied().map(|sec| {
                            let is_active = sec == current_sleep;
                            button(
                                cx,
                                format!("sleep-option-{}", sec),
                                format_sleep(sec),
                                Action::Sleep(sec),
                                is_active,
                            )
                        })),
                ),
        )
        // Debounce interval
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
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(div().font_weight(FontWeight::SEMIBOLD).text_sm().child("Key Debounce Interval"))
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(theme::accent())
                                .child(format!("{} ms", current_debounce)),
                        ),
                )
                .child(hint(
                    "Debounce filter delay for key matrix switches (0..255 ms).",
                ))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(button(
                            cx,
                            "debounce-dec-5",
                            "-5 ms",
                            Action::Debounce(-5),
                            false,
                        ))
                        .child(button(
                            cx,
                            "debounce-dec-1",
                            "-1 ms",
                            Action::Debounce(-1),
                            false,
                        ))
                        .child(
                            div()
                                .px_4()
                                .py_2()
                                .rounded_md()
                                .bg(theme::input_bg())
                                .text_sm()
                                .font_weight(FontWeight::BOLD)
                                .child(format!("{} ms", current_debounce)),
                        )
                        .child(button(
                            cx,
                            "debounce-inc-1",
                            "+1 ms",
                            Action::Debounce(1),
                            false,
                        ))
                        .child(button(
                            cx,
                            "debounce-inc-5",
                            "+5 ms",
                            Action::Debounce(5),
                            false,
                        )),
                ),
        )
        // Hardware Protocol Notice Banner
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .p_4()
                .rounded_md()
                .border_1()
                .border_color(theme::border())
                .bg(theme::input_bg())
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme::muted())
                        .child("Hardware Protocol Boundary"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::muted())
                        .child(
                            "The lighting backend requires a validated identity, a live profile read, acknowledged writes and matching readback. That path has not yet been validated on this keyboard. Sleep/debounce register encoding is also unvalidated; these settings remain local drafts.",
                        ),
                ),
        )
}

fn format_sleep(seconds: u16) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds.is_multiple_of(60) {
        format!("{} min", seconds / 60)
    } else {
        format!("{}m {}s", seconds / 60, seconds % 60)
    }
}
