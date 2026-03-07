//! Menu example - shows how to create a context menu.

use gpui::{App, Application, Image, ImageFormat, MenuItem, actions};
use gpui_tray::{Tray, TrayAppContext};

actions!(menu_example, [Open, Settings, Quit]);

fn main() {
    Application::new().run(|cx: &mut App| {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
        cx.activate(true);

        cx.on_action(|_: &Quit, cx: &mut App| cx.quit());
        cx.on_action(|_: &Open, _| println!("Open clicked!"));
        cx.on_action(|_: &Settings, _| println!("Settings clicked!"));

        let icon = Image::from_bytes(
            ImageFormat::Png,
            include_bytes!("image/app-icon.png").to_vec(),
        );

        cx.set_tray(Tray::new().tooltip("Right-click me!").icon(icon).menu(|| {
            vec![
                MenuItem::action("Open", Open),
                MenuItem::action("Settings", Settings),
                MenuItem::separator(),
                MenuItem::action("Quit", Quit),
            ]
        }))
        .unwrap();
    });
}
