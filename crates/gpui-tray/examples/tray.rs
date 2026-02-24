use gpui::{
    App, Application, Context, Div, Global, Image, ImageFormat, MenuItem, Stateful, Window, WindowOptions, actions, div, prelude::*
};
use gpui_tray::{Tray, TrayAppContext};
use gpui_tray_core::{ClickEvent, DoubleClickEvent};
use log::{debug, info};

struct Example;

impl Render for Example {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        fn button(id: &'static str) -> Stateful<Div> {
            div()
                .id(id)
                .py_0p5()
                .px_3()
                .bg(gpui::black())
                .active(|this| this.bg(gpui::black().opacity(0.8)))
                .text_color(gpui::white())
        }

        let app_state = cx.global::<AppState>();

        div()
            .bg(gpui::white())
            .flex()
            .flex_col()
            .gap_4()
            .size_full()
            .justify_center()
            .items_center()
            .child("Example for set Tray Icon")
            .child(
                div().flex().flex_row().gap_3().child(
                    button("toggle-visible")
                        .child(format!("Visible: {}", app_state.tray.visible))
                        .on_click(|_, window, cx| {
                            debug!("Toggle visible button clicked");
                            window.dispatch_action(Box::new(ToggleVisible), cx);
                        }),
                ),
            )
    }
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    info!("Starting gpui-tray example application");

    Application::new().run(|cx: &mut App| {
        debug!("Setting up global application state");
        cx.set_global(AppState::new());

        cx.activate(true);
        cx.on_action(quit);
        cx.on_action(toggle_visible);
        cx.on_action(on_tray_click);
        cx.on_action(on_tray_double_click);

        debug!("Opening main window");
        cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| Example))
            .unwrap();

        debug!("Setting initial tray state");
        let app_state = cx.global::<AppState>();
        cx.set_tray(app_state.tray.clone()).unwrap();
        info!("Application ready!");
    });
}

struct AppState {
    tray: Tray,
}

impl AppState {
    fn new() -> Self {
        debug!("Creating AppState with default tray configuration");

        let icon_bytes = include_bytes!("image/app-icon.png");
        let icon = Image::from_bytes(ImageFormat::Png, icon_bytes.to_vec());

        Self {
            tray: Tray::new()
                .tooltip("🦀！？弱弱？！🦀")
                .title("Tray App")
                .icon(icon)
                .menu(Self::build_menus),
        }
    }

    fn build_menus(_cx: &mut App) -> Vec<MenuItem> {
        debug!("Building tray menu items");
        vec![
            MenuItem::action("Hide Tray Icon", ToggleVisible),
            MenuItem::separator(),
            MenuItem::action("Quit", Quit),
        ]
    }
}

impl Global for AppState {}

actions!(example, [Quit, ToggleVisible]);

fn quit(_: &Quit, cx: &mut App) {
    info!("Quit action received, shutting down gracefully");
    cx.quit();
}

fn toggle_visible(_: &ToggleVisible, cx: &mut App) {
    let app_state = cx.global_mut::<AppState>();
    let new_visible = !app_state.tray.visible;
    app_state.tray.visible = new_visible;

    debug!("Tray visibility toggled: visible={}", new_visible);

    let app_state = cx.global::<AppState>();
    cx.set_tray(app_state.tray.clone()).unwrap();
    cx.refresh_windows();
}

fn on_tray_click(event: &ClickEvent, _cx: &mut App) {
    info!(
        "Tray click event received: button={:?}, position={:?}",
        event.button, event.position
    );
}

fn on_tray_double_click(_event: &DoubleClickEvent, _cx: &mut App) {
    info!("Tray double click event received");
}
