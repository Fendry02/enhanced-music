use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

#[derive(serde::Serialize)]
struct Track {
    title: String,
    artist: String,
    album: String,
    is_playing: bool,
}

#[tauri::command]
fn get_current_track() -> Option<Track> {
    let script = r#"
        if application "Music" is running then
            tell application "Music"
                if player state is not stopped then
                    try
                        set t to name of current track
                        set ar to artist of current track
                        set al to album of current track
                        if player state is playing then
                            set playState to "playing"
                        else
                            set playState to "paused"
                        end if
                        return t & "|||" & ar & "|||" & al & "|||" & playState
                    end try
                end if
            end tell
        end if
        return ""
    "#;

    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .ok()?;

    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if raw.is_empty() {
        return None;
    }

    let parts: Vec<&str> = raw.splitn(4, "|||").collect();
    if parts.len() == 4 {
        Some(Track {
            title: parts[0].to_string(),
            artist: parts[1].to_string(),
            album: parts[2].to_string(),
            is_playing: parts[3].trim() == "playing",
        })
    } else {
        None
    }
}

#[tauri::command]
fn get_artwork(title: String, artist: String) -> Option<String> {
    use std::io::Read;

    let query = format!("{} {}", artist, title)
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => c.to_string(),
            ' ' => "+".to_string(),
            c => format!("%{:02X}", c as u32),
        })
        .collect::<String>();

    let search_url = format!(
        "https://itunes.apple.com/search?term={}&media=music&entity=song&limit=1",
        query
    );

    let json: serde_json::Value = ureq::get(&search_url)
        .call()
        .ok()?
        .into_json()
        .ok()?;

    let artwork_url = json["results"][0]["artworkUrl100"].as_str()?.to_string();
    let hd_url = artwork_url.replace("100x100bb", "600x600bb");

    let mut bytes = Vec::new();
    ureq::get(&hd_url)
        .call()
        .ok()?
        .into_reader()
        .read_to_end(&mut bytes)
        .ok()?;

    if bytes.is_empty() {
        return None;
    }

    use base64::{engine::general_purpose::STANDARD, Engine};
    Some(format!("data:image/jpeg;base64,{}", STANDARD.encode(&bytes)))
}

fn as_physical(pos: tauri::Position) -> (f64, f64) {
    match pos {
        tauri::Position::Physical(p) => (p.x as f64, p.y as f64),
        tauri::Position::Logical(p) => (p.x, p.y),
    }
}

fn as_physical_size(size: tauri::Size) -> (f64, f64) {
    match size {
        tauri::Size::Physical(s) => (s.width as f64, s.height as f64),
        tauri::Size::Logical(s) => (s.width, s.height),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_current_track, get_artwork])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            if let Some(window) = app.get_webview_window("main") {
                #[cfg(target_os = "macos")]
                {
                    use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
                    if let Err(e) = apply_vibrancy(
                        &window,
                        NSVisualEffectMaterial::Popover,
                        None,
                        Some(22.0),
                    ) {
                        eprintln!("Vibrancy unavailable: {e}");
                    }
                }

                let win = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::Focused(false) = event {
                        let _ = win.hide();
                    }
                });
            }

            let icon = {
                let bytes = include_bytes!("../icons/icon.png");
                let img = image::load_from_memory(bytes)
                    .expect("failed to decode tray icon")
                    .into_rgba8();
                let (w, h) = img.dimensions();
                tauri::image::Image::new_owned(img.into_raw(), w, h)
            };

            let quit = MenuItem::with_id(app, "quit", "Quit Enhanced Music", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit])?;

            TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    if event.id() == "quit" {
                        app.exit(0);
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        rect,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                if let Ok(win_size) = window.outer_size() {
                                    let (px, py) = as_physical(rect.position);
                                    let (sw, sh) = as_physical_size(rect.size);
                                    let x = (px + sw / 2.0 - win_size.width as f64 / 2.0) as i32;
                                    let y = (py + sh) as i32;
                                    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
                                }
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
