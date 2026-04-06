#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{thread, time::Duration};

use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use tauri_plugin_global_shortcut::{
    Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
};

const MAC_KEYCODE_C: u16 = 8;
const MAC_KEYCODE_V: u16 = 9;

#[tauri::command]
fn fix_selected_text() -> Result<(), String> {
    thread::sleep(Duration::from_millis(120));

    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    let old_clip = clipboard.get_text().ok();

    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| format!("Enigo init error: {e:?}"))?;

    // Cmd + C
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

    clipboard
        .set_text(converted)
        .map_err(|e| e.to_string())?;

    thread::sleep(Duration::from_millis(150));

    // Cmd + V
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

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![fix_selected_text])
        .setup(|app| {
            let shortcut = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyK);
            let app_handle = app.handle().clone();

            match app
                .global_shortcut()
                .on_shortcut(shortcut, move |_app, _shortcut, event| {
                    if event.state() == ShortcutState::Released {
                        let handle = app_handle.clone();
                        let _ = handle.run_on_main_thread(|| {
                            if let Err(e) = fix_selected_text() {
                                eprintln!("fix_selected_text error: {e}");
                            }
                        });
                    }
                })
            {
                Ok(_) => {
                    println!("Hotkey registered: Cmd+Shift+K");
                }
                Err(e) => {
                    eprintln!("Не удалось зарегистрировать хоткей: {e}");
                    eprintln!("Приложение продолжит работу без глобального хоткея.");
                    return Ok(());
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
