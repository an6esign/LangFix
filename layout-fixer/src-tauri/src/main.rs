#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::Mutex,
    thread,
    time::Duration,
};

use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use serde::{Deserialize, Serialize};
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, State, WindowEvent,
};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{
    GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
};
use tauri_plugin_updater::{Updater, UpdaterExt};

const MAC_KEYCODE_C: u16 = 8;
const MAC_KEYCODE_V: u16 = 9;
const TRAY_SHOW_ID: &str = "tray_show";
const TRAY_QUIT_ID: &str = "tray_quit";
const FIRST_RUN_MARKER: &str = "onboarding-complete";
const SETTINGS_FILE: &str = "settings.json";
const DEFAULT_SHORTCUT: &str = "Command+Shift+KeyK";
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    shortcut: String,
    launch_at_login: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            shortcut: DEFAULT_SHORTCUT.into(),
            launch_at_login: true,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsPayload {
    shortcut: String,
    shortcut_label: String,
    launch_at_login: bool,
    first_launch: bool,
}

struct SettingsState {
    current: Mutex<AppSettings>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdatePayload {
    version: String,
    current_version: String,
    date: Option<String>,
    body: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateCheckPayload {
    configured: bool,
    available: bool,
    update: Option<UpdatePayload>,
    message: String,
}

#[tauri::command]
fn fix_selected_text() -> Result<(), String> {
    thread::sleep(Duration::from_millis(120));

    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    let old_clip = clipboard.get_text().ok();

    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| format!("Enigo init error: {e:?}"))?;

    enigo
        .key(Key::Meta, Direction::Press)
        .map_err(|e| format!("{e:?}"))?;
    enigo
        .raw(MAC_KEYCODE_C, Direction::Click)
        .map_err(|e| format!("{e:?}"))?;
    enigo
        .key(Key::Meta, Direction::Release)
        .map_err(|e| format!("{e:?}"))?;

    thread::sleep(Duration::from_millis(300));

    let text = clipboard.get_text().map_err(|e| e.to_string())?;
    if text.trim().is_empty() {
        return Err("Не удалось получить выделенный текст".into());
    }

    let converted = convert_layout(&text);
    clipboard.set_text(converted).map_err(|e| e.to_string())?;

    thread::sleep(Duration::from_millis(150));

    enigo
        .key(Key::Meta, Direction::Press)
        .map_err(|e| format!("{e:?}"))?;
    enigo
        .raw(MAC_KEYCODE_V, Direction::Click)
        .map_err(|e| format!("{e:?}"))?;
    enigo
        .key(Key::Meta, Direction::Release)
        .map_err(|e| format!("{e:?}"))?;

    if let Some(old) = old_clip {
        thread::sleep(Duration::from_millis(250));
        let _ = clipboard.set_text(old);
    }

    Ok(())
}

#[tauri::command]
fn open_accessibility_settings() -> Result<(), String> {
    Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn complete_onboarding(app: AppHandle) -> Result<(), String> {
    let marker = onboarding_marker_path(&app)?;

    if let Some(parent) = marker.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    fs::write(&marker, b"done").map_err(|e| e.to_string())?;

    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }

    Ok(())
}

#[tauri::command]
fn is_first_launch(app: AppHandle) -> Result<bool, String> {
    Ok(!onboarding_marker_path(&app)?.exists())
}

#[tauri::command]
fn get_settings(app: AppHandle, state: State<'_, SettingsState>) -> Result<SettingsPayload, String> {
    let settings = state
        .current
        .lock()
        .map_err(|_| "Не удалось прочитать настройки".to_string())?
        .clone();

    settings_payload(&app, &settings)
}

#[tauri::command]
fn save_settings(
    app: AppHandle,
    state: State<'_, SettingsState>,
    shortcut: String,
    launch_at_login: bool,
) -> Result<SettingsPayload, String> {
    let normalized_shortcut = normalize_shortcut(&shortcut)?;
    let previous_settings = state
        .current
        .lock()
        .map_err(|_| "Не удалось обновить настройки".to_string())?
        .clone();

    let new_settings = AppSettings {
        shortcut: normalized_shortcut,
        launch_at_login,
    };

    let shortcut_changed = new_settings.shortcut != previous_settings.shortcut;
    if shortcut_changed {
        replace_shortcut(&app, &previous_settings.shortcut, &new_settings.shortcut)?;
    }

    if let Err(error) = apply_autostart(&app, new_settings.launch_at_login) {
        if shortcut_changed {
            let _ = replace_shortcut(&app, &new_settings.shortcut, &previous_settings.shortcut);
        }
        return Err(error);
    }

    if let Err(error) = save_settings_to_disk(&app, &new_settings) {
        if shortcut_changed {
            let _ = replace_shortcut(&app, &new_settings.shortcut, &previous_settings.shortcut);
        }
        let _ = apply_autostart(&app, previous_settings.launch_at_login);
        return Err(error);
    }

    *state
        .current
        .lock()
        .map_err(|_| "Не удалось сохранить настройки".to_string())? = new_settings.clone();

    settings_payload(&app, &new_settings)
}

#[tauri::command]
fn reset_settings(app: AppHandle, state: State<'_, SettingsState>) -> Result<SettingsPayload, String> {
    let previous_settings = state
        .current
        .lock()
        .map_err(|_| "Не удалось сбросить настройки".to_string())?
        .clone();
    let default_settings = AppSettings::default();

    let shortcut_changed = previous_settings.shortcut != default_settings.shortcut;
    if shortcut_changed {
        replace_shortcut(&app, &previous_settings.shortcut, &default_settings.shortcut)?;
    }

    if let Err(error) = apply_autostart(&app, default_settings.launch_at_login) {
        if shortcut_changed {
            let _ = replace_shortcut(&app, &default_settings.shortcut, &previous_settings.shortcut);
        }
        return Err(error);
    }

    if let Err(error) = save_settings_to_disk(&app, &default_settings) {
        if shortcut_changed {
            let _ = replace_shortcut(&app, &default_settings.shortcut, &previous_settings.shortcut);
        }
        let _ = apply_autostart(&app, previous_settings.launch_at_login);
        return Err(error);
    }

    let marker = onboarding_marker_path(&app)?;
    let _ = fs::remove_file(marker);

    *state
        .current
        .lock()
        .map_err(|_| "Не удалось сохранить настройки".to_string())? = default_settings.clone();

    show_main_window(&app);
    settings_payload(&app, &default_settings)
}

#[tauri::command]
async fn check_for_updates(app: AppHandle) -> Result<UpdateCheckPayload, String> {
    let update = build_updater(&app)?
        .check()
        .await
        .map_err(|e| format!("Не удалось проверить обновления: {e}"))?;

    let payload = update.map(|update| UpdatePayload {
        version: update.version.clone(),
        current_version: app.package_info().version.to_string(),
        date: update.date.map(|date| date.to_string()),
        body: update.body.clone(),
    });
    let available = payload.is_some();

    Ok(UpdateCheckPayload {
        configured: true,
        available,
        update: payload,
        message: if available {
            "Доступна новая версия.".into()
        } else {
            "Сейчас установлена актуальная версия.".into()
        },
    })
}

#[tauri::command]
async fn install_update(app: AppHandle) -> Result<String, String> {
    let Some(update) = build_updater(&app)?
        .check()
        .await
        .map_err(|e| format!("Не удалось получить обновление: {e}"))?
    else {
        return Ok("Новых обновлений нет.".into());
    };

    update
        .download_and_install(
            |_chunk_length, _content_length| {},
            || {},
        )
        .await
        .map_err(|e| format!("Не удалось установить обновление: {e}"))?;

    app.restart();
}

fn convert_layout(input: &str) -> String {
    let en = "`qwertyuiop[]asdfghjkl;'zxcvbnm,./~QWERTYUIOP{}ASDFGHJKL:\"ZXCVBNM<>?";
    let ru = "ёйцукенгшщзхъфывапролджэячсмитьбю.ËЙЦУКЕНГШЩЗХЪФЫВАПРОЛДЖЭЯЧСМИТЬБЮ,";

    let latin = input.chars().filter(|c| c.is_ascii_alphabetic()).count();
    let cyrillic = input
        .chars()
        .filter(|c| ('а'..='я').contains(c) || ('А'..='Я').contains(c) || *c == 'ё' || *c == 'Ё')
        .count();

    let en_to_ru = latin >= cyrillic;
    let (from, to) = if en_to_ru { (en, ru) } else { (ru, en) };

    input
        .chars()
        .map(|c| {
            from.chars()
                .position(|x| x == c)
                .and_then(|idx| to.chars().nth(idx))
                .unwrap_or(c)
        })
        .collect()
}

fn build_updater(app: &AppHandle) -> Result<Updater, String> {
    app.updater()
        .map_err(|e| format!("Не удалось создать updater: {e}"))
}

fn settings_payload(app: &AppHandle, settings: &AppSettings) -> Result<SettingsPayload, String> {
    Ok(SettingsPayload {
        shortcut: settings.shortcut.clone(),
        shortcut_label: shortcut_to_label(&settings.shortcut),
        launch_at_login: settings.launch_at_login,
        first_launch: !onboarding_marker_path(app)?.exists(),
    })
}

fn shortcut_to_label(shortcut: &str) -> String {
    shortcut
        .split('+')
        .map(|token| match token.to_uppercase().as_str() {
            "COMMAND" | "CMD" | "SUPER" | "COMMANDORCONTROL" | "COMMANDORCTRL" | "CMDORCTRL"
            | "CMDORCONTROL" => "⌘".to_string(),
            "CONTROL" | "CTRL" => "⌃".to_string(),
            "SHIFT" => "⇧".to_string(),
            "ALT" | "OPTION" => "⌥".to_string(),
            "SPACE" => "Space".to_string(),
            "ESCAPE" => "Esc".to_string(),
            "ENTER" => "Enter".to_string(),
            "TAB" => "Tab".to_string(),
            "ARROWUP" => "↑".to_string(),
            "ARROWDOWN" => "↓".to_string(),
            "ARROWLEFT" => "←".to_string(),
            "ARROWRIGHT" => "→".to_string(),
            token if token.starts_with("KEY") && token.len() == 4 => token[3..4].to_string(),
            token if token.starts_with("DIGIT") && token.len() == 6 => token[5..6].to_string(),
            "MINUS" => "-".to_string(),
            "EQUAL" => "=".to_string(),
            "COMMA" => ",".to_string(),
            "PERIOD" => ".".to_string(),
            "SLASH" => "/".to_string(),
            "BACKQUOTE" => "`".to_string(),
            "SEMICOLON" => ";".to_string(),
            "QUOTE" => "'".to_string(),
            "BRACKETLEFT" => "[".to_string(),
            "BRACKETRIGHT" => "]".to_string(),
            "BACKSLASH" => "\\".to_string(),
            other => other.to_string(),
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn onboarding_marker_path(app: &AppHandle) -> Result<PathBuf, String> {
    let base_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(base_dir.join(FIRST_RUN_MARKER))
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let base_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(base_dir.join(SETTINGS_FILE))
}

fn load_settings(app: &AppHandle) -> Result<AppSettings, String> {
    let path = settings_path(app)?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }

    let contents = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&contents).map_err(|e| e.to_string())
}

fn save_settings_to_disk(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = settings_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let contents = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(path, contents).map_err(|e| e.to_string())
}

fn normalize_shortcut(input: &str) -> Result<String, String> {
    let shortcut: Shortcut = input
        .trim()
        .parse()
        .map_err(|_| "Некорректный формат хоткея".to_string())?;

    if shortcut.mods.is_empty() {
        return Err("Хоткей должен содержать хотя бы один модификатор".into());
    }

    let mut parts = Vec::new();
    if shortcut.mods.contains(Modifiers::SUPER) {
        parts.push("Command".to_string());
    }
    if shortcut.mods.contains(Modifiers::CONTROL) {
        parts.push("Control".to_string());
    }
    if shortcut.mods.contains(Modifiers::ALT) {
        parts.push("Alt".to_string());
    }
    if shortcut.mods.contains(Modifiers::SHIFT) {
        parts.push("Shift".to_string());
    }
    parts.push(shortcut.key.to_string());

    Ok(parts.join("+"))
}

fn apply_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    if enabled {
        app.autolaunch().enable().map_err(|e| e.to_string())
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())
    }
}

fn replace_shortcut(app: &AppHandle, old_shortcut: &str, new_shortcut: &str) -> Result<(), String> {
    unregister_shortcut(app, old_shortcut);
    thread::sleep(Duration::from_millis(60));

    if let Err(error) = register_shortcut(app, new_shortcut) {
        if !old_shortcut.is_empty() {
            let _ = register_shortcut(app, old_shortcut);
        }
        return Err(error);
    }

    Ok(())
}

fn unregister_shortcut(app: &AppHandle, shortcut_str: &str) {
    if let Ok(shortcut) = shortcut_str.parse::<Shortcut>() {
        let _ = app.global_shortcut().unregister(shortcut);
    }
}

fn register_shortcut(app: &AppHandle, shortcut_str: &str) -> Result<(), String> {
    let shortcut: Shortcut = shortcut_str
        .parse()
        .map_err(|_| "Некорректный формат хоткея".to_string())?;
    let app_handle = app.clone();

    for attempt in 0..3 {
        let handle = app_handle.clone();
        let result = app.global_shortcut().on_shortcut(shortcut, move |_app, _shortcut, event| {
            if event.state() == ShortcutState::Released {
                let handle = handle.clone();
                let _ = handle.run_on_main_thread(|| {
                    if let Err(e) = fix_selected_text() {
                        eprintln!("fix_selected_text error: {e}");
                    }
                });
            }
        });

        match result {
            Ok(_) => return Ok(()),
            Err(_) if attempt < 2 => thread::sleep(Duration::from_millis(80)),
            Err(_) => {
                return Err(
                    "Не удалось зарегистрировать хоткей. Возможно, он уже занят системой или другим приложением."
                        .to_string(),
                )
            }
        }
    }

    Err("Не удалось зарегистрировать хоткей.".to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::Builder::new().app_name("LangFix").build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            fix_selected_text,
            open_accessibility_settings,
            complete_onboarding,
            is_first_launch,
            get_settings,
            save_settings,
            reset_settings,
            check_for_updates,
            install_update
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let loaded_settings = load_settings(app.handle()).unwrap_or_default();
            save_settings_to_disk(app.handle(), &loaded_settings)?;
            app.manage(SettingsState {
                current: Mutex::new(loaded_settings.clone()),
            });

            let show_item = MenuItemBuilder::with_id(TRAY_SHOW_ID, "Показать LangFix").build(app)?;
            let quit_item = MenuItemBuilder::with_id(TRAY_QUIT_ID, "Выйти").build(app)?;
            let tray_menu = MenuBuilder::new(app)
                .item(&show_item)
                .separator()
                .item(&quit_item)
                .build()?;

            let mut tray = TrayIconBuilder::with_id("menu_bar")
                .menu(&tray_menu)
                .tooltip("LangFix")
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    TRAY_SHOW_ID => show_main_window(app),
                    TRAY_QUIT_ID => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                    }
                });

            if let Some(icon) = app.default_window_icon().cloned() {
                tray = tray.icon(icon).icon_as_template(true);
            }

            let _tray = tray.build(app)?;

            if loaded_settings.launch_at_login {
                if let Err(e) = app.autolaunch().enable() {
                    eprintln!("Не удалось включить автозапуск: {e}");
                }
            } else if let Err(e) = app.autolaunch().disable() {
                eprintln!("Не удалось выключить автозапуск: {e}");
            }

            match register_shortcut(app.handle(), &loaded_settings.shortcut) {
                Ok(_) => {
                    println!("Hotkey registered: {}", loaded_settings.shortcut);
                }
                Err(e) => {
                    eprintln!("{e}");
                    eprintln!("Приложение продолжит работу без глобального хоткея.");
                }
            }

            let first_launch = !onboarding_marker_path(app.handle())?.exists();
            if first_launch {
                show_main_window(app.handle());
            } else if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
