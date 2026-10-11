mod protocol;
mod driver;

use gpui::*;
use driver::AulaDevice;
use protocol::LIGHTING_EFFECTS;

struct AulaApp {
    device: AulaDevice,
    selected_effect: usize,
    status_text: SharedString,
}

impl AulaApp {
    fn new(_cx: &mut Context<Self>) -> Self {
        let mut dev = AulaDevice::new();
        let status = match dev.connect() {
            Ok(true) => format!("Connected ({:?})", dev.mode),
            Ok(false) => "AULA F75 Not Found".to_string(),
            Err(e) => format!("Error: {}", e),
        };

        Self {
            device: dev,
            selected_effect: 0,
            status_text: status.into(),
        }
    }
}

impl Render for AulaApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x18181b)) // zinc-900
            .text_color(rgb(0xf4f4f5))
            .p_6()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .pb_4()
                    .border_b_1()
                    .border_color(rgb(0x27272a))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_xl()
                                    .font_weight(FontWeight::BOLD)
                                    .child("AULA F75 Control"),
                            )
                    )
                    .child(
                        div()
                            .text_sm()
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .bg(rgb(0x27272a))
                            .child(self.status_text.clone()),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .py_4()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(div().font_weight(FontWeight::SEMIBOLD).text_sm().child("Lighting Effects"))
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .gap_2()
                                    .children(LIGHTING_EFFECTS.iter().enumerate().map(|(idx, effect)| {
                                        let is_selected = idx == self.selected_effect;
                                        div()
                                            .px_3()
                                            .py_1p5()
                                            .rounded_md()
                                            .cursor_pointer()
                                            .bg(if is_selected { rgb(0x3b82f6) } else { rgb(0x27272a) })
                                            .hover(|s| s.bg(rgb(0x3f3f46)))
                                            .child(effect.name)
                                    }))
                            )
                    )
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.open_window(
            WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: Some("AULA F75 Control".into()),
                    appears_transparent: true,
                    traffic_light_position: None,
                }),
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point::new(px(100.0), px(100.0)),
                    size: Size {
                        width: px(960.0),
                        height: px(600.0),
                    },
                })),
                ..Default::default()
            },
            |_, cx| cx.new(AulaApp::new),
        ).unwrap();
    });
}
