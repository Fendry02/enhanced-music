mod config;

use std::io::Read;
use std::sync::OnceLock;
use std::time::Duration;

use config::Config;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

// ─── HTTP agent (shared, with timeouts) ───────────────────────────────────────

fn http() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(5))
            .timeout(Duration::from_secs(20))
            .build()
    })
}

// ─── URL encoding ─────────────────────────────────────────────────────────────

fn url_encode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => c.to_string(),
            ' ' => "+".to_string(),
            c => format!("%{:02X}", c as u32),
        })
        .collect()
}

// ─── Anthropic helpers ────────────────────────────────────────────────────────

fn call_claude(api_key: &str, max_tokens: u64, prompt: &str) -> Option<serde_json::Value> {
    http()
        .post("https://api.anthropic.com/v1/messages")
        .set("x-api-key", api_key)
        .set("anthropic-version", "2023-06-01")
        .send_json(serde_json::json!({
            "model": "claude-haiku-4-5-20251001",
            "max_tokens": max_tokens,
            "messages": [{ "role": "user", "content": prompt }]
        }))
        .map_err(|e| eprintln!("[claude] request error: {e}"))
        .ok()?
        .into_json()
        .map_err(|e| eprintln!("[claude] JSON parse error: {e}"))
        .ok()
}

fn extract_claude_json(response: &serde_json::Value, ctx: &str) -> Option<serde_json::Value> {
    let text = response["content"][0]["text"].as_str().or_else(|| {
        eprintln!("[claude:{ctx}] unexpected response shape: {response}");
        None
    })?;
    let clean = strip_code_fences(text);
    serde_json::from_str(clean)
        .map_err(|e| eprintln!("[claude:{ctx}] JSON parse error: {e}\nRaw: {clean}"))
        .ok()
}

/// Removes ```json / ``` fences from a Claude response, exactly once each side.
fn strip_code_fences(s: &str) -> &str {
    let s = s.trim();
    let s = s.strip_prefix("```json").or_else(|| s.strip_prefix("```")).unwrap_or(s);
    let s = s.strip_suffix("```").unwrap_or(s);
    s.trim()
}

// ─── Genius helpers ───────────────────────────────────────────────────────────

fn genius_get(url: &str, token: &str) -> Option<serde_json::Value> {
    http()
        .get(url)
        .set("Authorization", &format!("Bearer {}", token))
        .call()
        .map_err(|e| eprintln!("[genius] GET {url} error: {e}"))
        .ok()?
        .into_json()
        .map_err(|e| eprintln!("[genius] JSON parse error: {e}"))
        .ok()
}

/// Fetches an album description from Genius via song search → song → album.
/// Uses an inner `Option`-returning function so `?` can be used freely.
fn genius_album_description(token: &str, artist: &str, album: &str) -> String {
    genius_album_description_inner(token, artist, album).unwrap_or_default()
}

fn genius_album_description_inner(token: &str, artist: &str, album: &str) -> Option<String> {
    let query  = url_encode(&format!("{} {}", artist, album));
    let search = genius_get(&format!("https://api.genius.com/search?q={}", query), token)?;

    let song_id = search["response"]["hits"][0]["result"]["id"]
        .as_i64()
        .or_else(|| { eprintln!("[genius] no hits for «{album}» by {artist}"); None })?;

    let song     = genius_get(&format!("https://api.genius.com/songs/{}", song_id), token)?;
    let album_id = song["response"]["song"]["album"]["id"].as_i64()?;
    let album_v  = genius_get(&format!("https://api.genius.com/albums/{}", album_id), token)?;

    album_v["response"]["album"]["description_preview"]
        .as_str()
        .filter(|s| !s.is_empty() && *s != "?")
        .map(|s| s.to_string())
}

// ─── iTunes helper ────────────────────────────────────────────────────────────

/// Returns (release_year, genre) from the iTunes Search API.
fn itunes_album_metadata(artist: &str, album: &str) -> (String, String) {
    let query = url_encode(&format!("{} {}", artist, album));
    let url = format!(
        "https://itunes.apple.com/search?term={}&media=music&entity=album&limit=10",
        query
    );

    let json: serde_json::Value = match http().get(&url).call().ok().and_then(|r| r.into_json().ok()) {
        Some(v) => v,
        None => {
            eprintln!("[itunes] request failed for «{album}» by {artist}");
            return (String::new(), String::new());
        }
    };

    let album_lc  = album.to_lowercase();
    let artist_lc = artist.to_lowercase();

    let hit = json["results"].as_array().and_then(|arr| {
        arr.iter().find(|r| {
            r["collectionName"].as_str().unwrap_or("").to_lowercase().contains(&album_lc)
                && r["artistName"].as_str().unwrap_or("").to_lowercase().contains(&artist_lc)
        })
    });

    let year = hit
        .and_then(|r| r["releaseDate"].as_str())
        .and_then(|d| d.get(..4))
        .unwrap_or("")
        .to_string();

    let genre = hit
        .and_then(|r| r["primaryGenreName"].as_str())
        .unwrap_or("")
        .to_string();

    (year, genre)
}

// ─── Track ────────────────────────────────────────────────────────────────────

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
                        set t  to name of current track
                        set ar to artist of current track
                        set al to album of current track
                        if player state is playing then
                            set s to "playing"
                        else
                            set s to "paused"
                        end if
                        return t & "|||" & ar & "|||" & al & "|||" & s
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
    (parts.len() == 4).then(|| Track {
        title:      parts[0].to_string(),
        artist:     parts[1].to_string(),
        album:      parts[2].to_string(),
        is_playing: parts[3].trim() == "playing",
    })
}

// ─── Artwork ──────────────────────────────────────────────────────────────────

#[tauri::command]
fn get_artwork(title: String, artist: String) -> Option<String> {
    let query = url_encode(&format!("{} {}", artist, title));
    let json: serde_json::Value = http()
        .get(&format!(
            "https://itunes.apple.com/search?term={}&media=music&entity=song&limit=1",
            query
        ))
        .call()
        .ok()?
        .into_json()
        .ok()?;

    let artwork_url = json["results"][0]["artworkUrl100"].as_str()?.to_string();
    let hd_url = artwork_url.replace("100x100bb", "600x600bb");

    let mut bytes = Vec::new();
    http()
        .get(&hd_url)
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

// ─── Album info ───────────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct AlbumInfo {
    release_year: String,
    genre: String,
    context: String,
    notable_fact: String,
}

#[tauri::command]
fn get_album_info(
    album: String,
    artist: String,
    state: tauri::State<'_, Config>,
) -> Option<AlbumInfo> {
    if !state.has_keys() {
        eprintln!("[album_info] API keys missing — skipping");
        return None;
    }

    let (release_year, genre) = itunes_album_metadata(&artist, &album);
    let description           = genius_album_description(&state.api.genius_token, &artist, &album);
    let prompt                = build_album_prompt(&album, &artist, &release_year, &genre, &description);

    let response  = call_claude(&state.api.anthropic_key, 400, &prompt)?;
    let extracted = extract_claude_json(&response, "album")?;

    Some(AlbumInfo {
        release_year,
        genre,
        context:      extracted["context"].as_str().unwrap_or("").to_string(),
        notable_fact: extracted["notable_fact"].as_str().unwrap_or("").to_string(),
    })
}

fn build_album_prompt(album: &str, artist: &str, year: &str, genre: &str, description: &str) -> String {
    let meta = if year.is_empty() {
        String::new()
    } else {
        format!(" (sorti en {year}, genre : {genre})")
    };

    let base = if description.is_empty() {
        format!("En te basant sur tes connaissances, pour l'album \"{album}\" de {artist}{meta}, réponds en français.")
    } else {
        format!("Pour l'album \"{album}\" de {artist}{meta}, basé sur cette description :\n{description}\nRéponds en français.")
    };

    format!(
        "{base}\n\nRéponds UNIQUEMENT avec ce JSON valide (sans markdown) :\
         {{\"context\":\"2-3 phrases sur le contexte et la genèse de l'album\",\
         \"notable_fact\":\"Un fait marquant ou anecdote sur cet album\"}}"
    )
}

// ─── Lyrics analysis ──────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct LyricsAnalysis {
    interpretation: String,
}

#[tauri::command]
fn get_lyrics_analysis(
    title: String,
    artist: String,
    state: tauri::State<'_, Config>,
) -> Option<LyricsAnalysis> {
    if !state.has_keys() {
        return None;
    }

    let genius_token = &state.api.genius_token;
    let query        = url_encode(&format!("{} {}", artist, title));
    let search_url   = format!("https://api.genius.com/search?q={}", query);

    let search = genius_get(&search_url, genius_token)?;

    let song_url = search["response"]["hits"][0]["result"]["url"]
        .as_str()
        .or_else(|| { eprintln!("[lyrics] no hits for «{title}» by {artist}"); None })?;

    let lyrics = fetch_genius_lyrics(song_url);
    let prompt  = build_lyrics_prompt(&title, &artist, lyrics.as_deref());

    let response  = call_claude(&state.api.anthropic_key, 450, &prompt)?;
    let extracted = extract_claude_json(&response, "lyrics")?;

    Some(LyricsAnalysis {
        interpretation: extracted["interpretation"].as_str().unwrap_or("").to_string(),
    })
}

fn build_lyrics_prompt(title: &str, artist: &str, lyrics: Option<&str>) -> String {
    let intro = format!(
        "Tu es un expert en musique et en analyse de textes. \
         Pour le morceau \"{title}\" de {artist}"
    );

    let body = match lyrics {
        Some(lyr) => format!(
            "{intro}, voici les paroles :\n\n{lyr}\n\n\
             Basé sur ces paroles, explique en 3-4 phrases en français"
        ),
        None => format!(
            "{intro}, explique en 3-4 phrases en français \
             (en te basant sur tes connaissances)"
        ),
    };

    format!(
        "{body} : le thème principal, l'émotion portée, et ce que l'artiste \
         cherche à exprimer. Sois précis et va au-delà du simple résumé.\n\n\
         Réponds UNIQUEMENT avec ce JSON (sans markdown) : \
         {{\"interpretation\": \"...\"}}"
    )
}

// ─── Lyrics scraping ──────────────────────────────────────────────────────────

fn fetch_genius_lyrics(url: &str) -> Option<String> {
    let html = http()
        .get(url)
        .set(
            "User-Agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
             AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        )
        .call()
        .ok()?
        .into_string()
        .ok()?;

    let lyrics = extract_lyrics_from_html(&html);
    if lyrics.trim().is_empty() {
        None
    } else {
        Some(lyrics.chars().take(3000).collect())
    }
}

/// Extracts plain-text lyrics from `data-lyrics-container="true"` divs.
fn extract_lyrics_from_html(html: &str) -> String {
    let mut result = String::new();
    let mut pos    = 0;

    while let Some(rel) = html[pos..].find("data-lyrics-container=\"true\"") {
        let tag_start = pos + rel;

        let Some(open_end) = html[tag_start..].find('>') else { break };
        let content_start = tag_start + open_end + 1;

        let mut depth = 1usize;
        let mut scan  = content_start;

        while depth > 0 && scan < html.len() {
            if html[scan..].starts_with("<div") {
                depth += 1;
                scan  += 4;
            } else if html[scan..].starts_with("</div>") {
                depth -= 1;
                if depth == 0 { break; }
                scan += 6;
            } else {
                scan += html[scan..].chars().next().map_or(1, |c| c.len_utf8());
            }
        }

        let section = html_to_text(&html[content_start..scan]);
        if !result.is_empty() { result.push('\n'); }
        result.push_str(section.trim());
        pos = scan;
    }

    result
}

/// Strips HTML tags, converts `<br>` to newlines, and decodes common entities.
fn html_to_text(fragment: &str) -> String {
    const ENTITIES: &[(&str, char)] = &[
        ("&amp;",  '&'), ("&lt;",   '<'), ("&gt;",   '>'),
        ("&quot;", '"'), ("&apos;", '\''),("&#x27;", '\''),
        ("&#39;",  '\''),
    ];

    let mut out = String::with_capacity(fragment.len());
    let mut pos = 0;

    while pos < fragment.len() {
        let rest = &fragment[pos..];

        if rest.starts_with('<') {
            if let Some(end) = rest.find('>') {
                if rest[1..end].trim_start().to_lowercase().starts_with("br") {
                    out.push('\n');
                }
                pos += end + 1;
            } else {
                pos += 1;
            }
        } else if rest.starts_with('&') {
            match ENTITIES.iter().find(|&&(e, _)| rest.starts_with(e)) {
                Some(&(entity, ch)) => { out.push(ch); pos += entity.len(); }
                None                => { out.push('&'); pos += 1; }
            }
        } else {
            let c = rest.chars().next().unwrap();
            out.push(c);
            pos += c.len_utf8();
        }
    }

    out
}

// ─── Window positioning ───────────────────────────────────────────────────────

fn as_physical(pos: tauri::Position) -> (f64, f64) {
    match pos {
        tauri::Position::Physical(p) => (p.x as f64, p.y as f64),
        tauri::Position::Logical(p)  => (p.x, p.y),
    }
}

fn as_physical_size(size: tauri::Size) -> (f64, f64) {
    match size {
        tauri::Size::Physical(s) => (s.width as f64, s.height as f64),
        tauri::Size::Logical(s)  => (s.width, s.height),
    }
}

// ─── App entry ────────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = Config::load();

    tauri::Builder::default()
        .manage(config)
        .invoke_handler(tauri::generate_handler![
            get_current_track,
            get_artwork,
            get_album_info,
            get_lyrics_analysis,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            if let Some(window) = app.get_webview_window("main") {
                #[cfg(target_os = "macos")]
                {
                    use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
                    if let Err(e) = apply_vibrancy(&window, NSVisualEffectMaterial::Popover, None, Some(22.0)) {
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
                let img   = image::load_from_memory(bytes)
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
