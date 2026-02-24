// #![cfg(target_os = "macos")]

// use gpui::App;
// use gpui_tray_core::{PlatformTray, Result, Tray};
// use log::debug;

// mod tray;

// pub use tray::MacosTray;

// pub fn create() -> Result<Box<dyn PlatformTray>> {
//     debug!("Creating macOS tray implementation");
//     Ok(Box::new(MacosTray::new()))
// }

// impl PlatformTray for MacosTray {
//     fn set_tray(&mut self, _cx: &mut App, _tray: &Tray) -> Result<()> {
//         debug!("MacosTray::set_tray called");
//         Ok(())
//     }

//     fn update_tray(&mut self, _cx: &mut App, _tray: &Tray) -> Result<()> {
//         debug!("MacosTray::update_tray called");
//         Ok(())
//     }

//     fn remove_tray(&mut self, _cx: &mut App) -> Result<()> {
//         debug!("MacosTray::remove_tray called");
//         Ok(())
//     }
// }
