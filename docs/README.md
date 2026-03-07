# GPUI Tray

A cross-platform system tray library for [GPUI](https://github.com/zed-industries/zed).

> **Heads up!** I don't have the bandwidth to thoroughly test this library, so there might be all sorts of weird bugs lurking around. If you run into any issues, please file an issue and I'll try to fix it when I can :3

## Platform Support

| Platform | Status | Notes |
|----------|--------|-------|
| Windows | Full support | Uses `windows` crate 0.62 |
| Linux | Full support | Uses `zbus` 5.14.0, implements StatusNotifierItem spec |
| macOS | Stub only | No hardware available for development. Has a placeholder that won't crash, but doesn't actually show a tray icon. PRs welcome! |

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
gpui-tray = { git = "https://github.com/Yamrc/gpui-tray.git" }
```

Basic usage:

```rust
use gpui_tray::{Tray, TrayAppContext};

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.activate(true);
        
        cx.set_tray(
            Tray::new()
                .tooltip("My Awesome App")
                .title("Tray App")
                .icon(icon)
                .menu(|| vec![
                    MenuItem::action("Open", OpenAction),
                    MenuItem::separator(),
                    MenuItem::action("Quit", QuitAction),
                ])
        ).unwrap();
    });
}
```

## API Overview

The library provides a simple builder pattern for configuring tray icons:

```rust
let tray = Tray::new()
    .tooltip("Hover text")           // Text shown on hover
    .title("Tray Title")             // Platform-specific title
    .icon(image)                      // GPUI Image for the icon
    .visible(true)                   // Show/hide the tray icon
    .menu(|| vec![...]);             // Context menu builder
```

Control the tray through the `TrayAppContext` extension trait on `App`:

```rust
// Set or replace the tray
cx.set_tray(tray)?;

// Get current tray config (if any)
if let Some(tray) = cx.tray() {
    println!("Tooltip: {:?}", tray.tooltip);
}

// Update the tray
cx.update_tray(|tray| {
    tray.tooltip = Some("Updated!".into());
})?;

// Remove the tray
cx.remove_tray()?;
```

More [examples](../crates/gpui-tray/examples/)

## Contributing

I'm not a professional developer, so there's probably a lot of stuff I didn't think through properly. If you see something that could be done better, feel free to open an issue or PR!

### macOS Help Wanted

I don't have a Mac to develop on, so the macOS implementation is currently just a stub that compiles but doesn't actually show anything in the menu bar. If you have macOS experience and want to help implement proper NSStatusBar support, that would be amazing!

## License

[MPL-2.0](../LICENSE)
