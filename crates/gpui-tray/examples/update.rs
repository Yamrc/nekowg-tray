//! Update example - dynamically changing tray properties with UI controls.

use gpui::{
    App, Application, Context, Div, Image, ImageFormat, MenuItem, Stateful, Window, WindowOptions,
    actions, div, prelude::*,
};
use gpui_tray::{Tray, TrayAppContext};
use gpui_tray_core::{ClickEvent, DoubleClickEvent};
use log::info;

actions!(
    update_example,
    [ToggleVisible, UpdateTooltip, ToggleIcon, Quit]
);

struct AppState {
    tray: Tray,
    counter: i32,
    use_png: bool,
}

impl gpui::Global for AppState {}

struct Example;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    Application::new().run(|cx: &mut App| {
        let png_icon = Image::from_bytes(
            ImageFormat::Png,
            include_bytes!("image/app-icon.png").to_vec(),
        );

        let tray = Tray::new()
            .tooltip("Click to update")
            .icon(png_icon)
            .menu(|| {
                vec![
                    MenuItem::action("Update Tooltip", UpdateTooltip),
                    MenuItem::action("Toggle Visibility", ToggleVisible),
                    MenuItem::action("Toggle Icon", ToggleIcon),
                    MenuItem::separator(),
                    MenuItem::action("Quit", Quit),
                ]
            });

        cx.set_global(AppState {
            tray,
            counter: 0,
            use_png: true,
        });

        cx.activate(true);
        cx.on_action(quit);
        cx.on_action(toggle_visible);
        cx.on_action(update_tooltip);
        cx.on_action(toggle_icon);
        cx.on_action(on_tray_click);
        cx.on_action(on_tray_double_click);

        cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| Example))
            .unwrap();

        let app_state = cx.global::<AppState>();
        cx.set_tray(app_state.tray.clone()).unwrap();
    });
}

fn quit(_: &Quit, cx: &mut App) {
    info!("Quit action received");
    cx.quit();
}

fn toggle_visible(_: &ToggleVisible, cx: &mut App) {
    let app_state = cx.global_mut::<AppState>();
    app_state.tray.visible = !app_state.tray.visible;

    let tray = app_state.tray.clone();
    cx.set_tray(tray).unwrap();
    cx.refresh_windows();
}

fn update_tooltip(_: &UpdateTooltip, cx: &mut App) {
    let app_state = cx.global_mut::<AppState>();
    app_state.counter += 1;
    let count = app_state.counter;
    app_state.tray.tooltip = Some(format!("Updated {} times", count).into());

    let tray = app_state.tray.clone();
    cx.set_tray(tray).unwrap();
    cx.refresh_windows();
}

fn toggle_icon(_: &ToggleIcon, cx: &mut App) {
    let app_state = cx.global_mut::<AppState>();
    app_state.use_png = !app_state.use_png;

    let new_icon = if app_state.use_png {
        Image::from_bytes(
            ImageFormat::Png,
            include_bytes!("image/app-icon.png").to_vec(),
        )
    } else {
        Image::from_bytes(ImageFormat::Jpeg, include_bytes!("image/icon.jpg").to_vec())
    };

    app_state.tray.icon = Some(new_icon);

    let tray = app_state.tray.clone();
    cx.set_tray(tray).unwrap();
    cx.refresh_windows();
}

fn on_tray_click(event: &ClickEvent, _cx: &mut App) {
    info!(
        "Tray clicked: button={:?}, position={:?}",
        event.button, event.position
    );
}

fn on_tray_double_click(_event: &DoubleClickEvent, _cx: &mut App) {
    info!("Tray double-clicked!");
}

impl Render for Example {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        fn button(id: &'static str) -> Stateful<Div> {
            div()
                .id(id)
                .py_0p5()
                .px_3()
                .bg(gpui::black())
                .rounded_xs()
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
            .child("Tray Update Example")
            .child(format!("Tooltip updates: {}", app_state.counter))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_3()
                    .child(button("update-tooltip").child("Update Tooltip").on_click(
                        |_, window, cx| {
                            window.dispatch_action(Box::new(UpdateTooltip), cx);
                        },
                    ))
                    .child(
                        button("toggle-visible")
                            .child(format!("Visible: {}", app_state.tray.visible))
                            .on_click(|_, window, cx| {
                                window.dispatch_action(Box::new(ToggleVisible), cx);
                            }),
                    )
                    .child(
                        button("toggle-icon")
                            .child(format!(
                                "Icon: {}",
                                if app_state.use_png { "PNG" } else { "JPG" }
                            ))
                            .on_click(|_, window, cx| {
                                window.dispatch_action(Box::new(ToggleIcon), cx);
                            }),
                    ),
            )
    }
}
