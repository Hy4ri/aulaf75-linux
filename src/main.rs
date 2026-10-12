mod keyboard_view;
mod lighting_view;
mod text_input;
mod theme;
mod ui;
#[cfg(test)]
mod ui_tests;
mod workspace_views;

use gpui::*;
use ui::AppView;

fn main() {
    if std::env::args().any(|a| a == "--inspect") {
        let result = aulaf75_linux::model::Catalogue::bundled()
            .and_then(|c| aulaf75_linux::hardware::Hardware::connect(&c));
        match result {
            Ok(hw) => println!("{}", hw.description()),
            Err(e) => {
                eprintln!("Device inspection failed: {e:#}");
                std::process::exit(1);
            }
        }
        return;
    }
    Application::new().run(|cx: &mut App| {
        text_input::bind_keys(cx);
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.open_window(
            WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: Some("AULA F75 · Linux".into()),
                    appears_transparent: true,
                    traffic_light_position: None,
                }),
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point::new(px(100.), px(100.)),
                    size: Size {
                        width: px(1120.),
                        height: px(800.),
                    },
                })),
                ..Default::default()
            },
            |window, cx| {
                window.set_window_title("AULA F75 · Linux");
                window.set_app_id("aulaf75-linux");
                cx.new(AppView::new)
            },
        )
        .expect("Open native F75 window");
    });
}
