//! OrbyNode desktop shell (ADR 007): tray + wrapper. The daemon is the
//! runtime; closing this shell must never stop the daemon (Plan §109).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Tray menu per Plan §18. Quit Tray leaves the daemon running.
            let open = MenuItem::with_id(app, "open", "Open Dashboard", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Tray", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            TrayIconBuilder::with_id("orbynode-tray")
                .icon(app.default_window_icon().expect("window icon").clone())
                .tooltip("OrbyNode")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    "quit" => {
                        // Tray exit only: the daemon keeps running (Plan §18).
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running OrbyNode desktop shell");
}
