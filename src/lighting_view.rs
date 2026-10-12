use crate::{
    theme,
    ui::{button, heading, hint, Action, AppView},
};
use aulaf75_linux::model::EffectSpec;
use gpui::{prelude::*, *};

const COLOR_NAMES: [&str; 8] = [
    "Red (0)",
    "Orange (1)",
    "Yellow (2)",
    "Green (3)",
    "Cyan (4)",
    "Blue (5)",
    "Purple (6)",
    "Colorful (7)",
];

pub fn render(app: &mut AppView, _window: &mut Window, cx: &mut Context<AppView>) -> Div {
    let current_profile = app.workspace.current();
    let current_slot = current_profile.effect_slot;
    let current_brightness = current_profile.brightness;
    let current_speed = current_profile.speed;
    let current_color = current_profile.color;

    let selected_effect: Option<&EffectSpec> = app
        .catalogue
        .effects
        .iter()
        .find(|e| e.slot == current_slot)
        .or_else(|| app.catalogue.effects.get(current_slot));

    let effect_label = selected_effect
        .map(|e| e.label.as_str())
        .unwrap_or("Unknown effect");
    let hardware_id = selected_effect.map(|e| e.hardware_id).unwrap_or(0);

    let has_brightness = selected_effect.map(|e| e.has_brightness).unwrap_or(false);
    let has_speed = selected_effect.map(|e| e.has_speed).unwrap_or(false);
    let has_color = selected_effect.map(|e| e.has_color).unwrap_or(false);

    let is_custom_slot = current_slot == 18;
    let has_hw = app.hardware.is_some();
    let can_apply = !app.busy && has_hw && !is_custom_slot;

    let disabled_reason: Option<&'static str> = if app.busy {
        Some("Busy communicating with hardware…")
    } else if !has_hw {
        Some("Inspect and connect device first before applying lighting.")
    } else if is_custom_slot {
        Some("Custom per-key hardware colors (slot 18) are unvalidated and rejected by firmware patch.")
    } else {
        None
    };

    let effects_grid = div()
        .flex()
        .flex_wrap()
        .gap_2()
        .children(app.catalogue.effects.iter().map(|eff| {
            let is_active = eff.slot == current_slot;
            let id_str: SharedString = format!("effect-btn-{}", eff.slot).into();
            let label_str: SharedString = format!("{}. {}", eff.slot + 1, eff.label).into();
            button(cx, id_str, label_str, Action::Effect(eff.slot), is_active)
        }));

    let metadata_card = div()
        .flex()
        .flex_col()
        .gap_2()
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
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_sm()
                        .child(format!("Selected Mode: {effect_label} (Slot {current_slot})")),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::muted())
                        .child(format!("Byte 0x0a / Mode ID: {hardware_id} (0x{hardware_id:02x})")),
                ),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_4()
                .text_xs()
                .text_color(theme::muted())
                .child(format!("Hardware ID (LedOpt[0]): {hardware_id}"))
                .child(format!(
                    "Offset: 0x{:02x}",
                    0x3a + current_slot * 2
                ))
                .child(format!(
                    "Encoded Pair: [0x{:02x}, 0x{:02x}]",
                    current_brightness,
                    ((current_speed as u16) << 4) | (current_color as u16)
                ))
                .child(format!(
                    "Flags: Speed={}, Brightness={}, Color={}",
                    if has_speed { "Yes" } else { "No" },
                    if has_brightness { "Yes" } else { "No" },
                    if has_color { "Yes" } else { "No" },
                )),
        )
        .child(
            div()
                .text_xs()
                .text_color(if is_custom_slot {
                    gpui::rgb(0xf59e0b)
                } else {
                    theme::muted()
                })
                .child(if is_custom_slot {
                    "Notice: Custom per-key mode requires a separate matrix color operation. Firmware patching rejects slot 18."
                } else {
                    "Source-backed profile encoding: Profile packer 0x48d680 maps slot to byte 0x0a and dynamic parameter pair 0x3a+2*slot."
                }),
        );

    let brightness_controls = div()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .p_2()
        .rounded_md()
        .border_1()
        .border_color(theme::border())
        .bg(theme::panel())
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .child("Brightness (0..9)"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::muted())
                        .child(if has_brightness {
                            format!("Level {} / 9 (active in this mode)", current_brightness)
                        } else {
                            format!(
                                "Level {} / 9 (ignored by current effect)",
                                current_brightness
                            )
                        }),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(button(cx, "bright-dec", "-", Action::Brightness(-1), false))
                .child(
                    div()
                        .w(px(32.))
                        .text_center()
                        .text_sm()
                        .font_weight(FontWeight::BOLD)
                        .child(format!("{current_brightness}")),
                )
                .child(button(cx, "bright-inc", "+", Action::Brightness(1), false)),
        );

    let speed_controls = div()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .p_2()
        .rounded_md()
        .border_1()
        .border_color(theme::border())
        .bg(theme::panel())
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .child("Speed (0..4)"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::muted())
                        .child(if has_speed {
                            format!("Step {} / 4 (active in this mode)", current_speed)
                        } else {
                            format!("Step {} / 4 (ignored by current effect)", current_speed)
                        }),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(button(cx, "speed-dec", "-", Action::Speed(-1), false))
                .child(
                    div()
                        .w(px(32.))
                        .text_center()
                        .text_sm()
                        .font_weight(FontWeight::BOLD)
                        .child(format!("{current_speed}")),
                )
                .child(button(cx, "speed-inc", "+", Action::Speed(1), false)),
        );

    let color_controls = div()
        .flex()
        .flex_col()
        .gap_2()
        .p_2()
        .rounded_md()
        .border_1()
        .border_color(theme::border())
        .bg(theme::panel())
        .child(
            div()
                .flex()
                .justify_between()
                .items_center()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .child("Color Selection"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::muted())
                        .child(if has_color {
                            format!(
                                "Current: {} (active in this mode)",
                                COLOR_NAMES
                                    .get(current_color as usize)
                                    .unwrap_or(&"Unknown")
                            )
                        } else {
                            format!(
                                "Current: {} (ignored by current effect)",
                                COLOR_NAMES
                                    .get(current_color as usize)
                                    .unwrap_or(&"Unknown")
                            )
                        }),
                ),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_1p5()
                .children((0..8u8).map(|c| {
                    let is_active = c == current_color;
                    let id_str: SharedString = format!("color-btn-{c}").into();
                    let name = COLOR_NAMES.get(c as usize).copied().unwrap_or("Color");
                    button(cx, id_str, name, Action::Color(c), is_active)
                })),
        );

    let apply_section = div()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(theme::border())
        .bg(theme::panel())
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Apply to Hardware"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(if can_apply {
                            theme::muted()
                        } else {
                            gpui::rgb(0xf59e0b)
                        })
                        .child(if let Some(reason) = disabled_reason {
                            format!("Apply disabled: {reason}")
                        } else {
                            "Writes patched 128-byte profile and verifies readback. No unvalidated writes.".to_string()
                        }),
                ),
        )
        .child(if can_apply {
            button(
                cx,
                "apply-lighting-btn",
                "Apply Lighting",
                Action::ApplyLighting,
                true,
            )
        } else {
            div()
                .id("apply-lighting-disabled")
                .px_3()
                .py_2()
                .rounded_md()
                .text_sm()
                .border_1()
                .border_color(theme::border())
                .bg(theme::panel())
                .text_color(theme::muted())
                .child("Apply Lighting")
        });

    div()
        .flex()
        .flex_col()
        .gap_4()
        .max_w(px(900.))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(heading("Lighting & Backlight Effects"))
                .child(hint(
                    "All 20 factory effect slots with verified LedOpt[0] mode IDs. Custom per-key colors (slot 18) are unvalidated and excluded from hardware writes.",
                )),
        )
        .child(effects_grid)
        .child(metadata_card)
        .child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(brightness_controls)
                .child(speed_controls)
                .child(color_controls),
        )
        .child(apply_section)
}
