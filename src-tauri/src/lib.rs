// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use read_fonts::FontRef;
use serde::Serialize;

#[cfg(windows)]
use windows::Win32::{
    Foundation::{HWND, LPARAM, POINT, WPARAM},
    UI::WindowsAndMessaging::{
        EnableMenuItem, GetCursorPos, GetSystemMenu, IsZoomed, PostMessageW, SetForegroundWindow,
        SetMenuDefaultItem, TrackPopupMenu, MF_BYCOMMAND, MF_ENABLED, MF_GRAYED, SC_MAXIMIZE,
        SC_MOVE, SC_RESTORE, SC_SIZE, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_NULL, WM_SYSCOMMAND,
    },
};

#[derive(Serialize)]
pub struct FontTableInfo {
    pub tag: String,
    pub length: u32,
    pub offset: u32,
}

#[derive(Serialize)]
pub struct FontInfo {
    pub tables: Vec<FontTableInfo>,
    pub num_tables: u16,
}

#[tauri::command]
fn parse_font_tables(path: String) -> Result<FontInfo, String> {
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let font = FontRef::new(&bytes).map_err(|e| e.to_string())?;

    let table_directory = font.table_directory;

    let mut tables = Vec::new();

    for record in table_directory.table_records() {
        tables.push(FontTableInfo {
            tag: record.tag().to_string(),
            length: record.length(),
            offset: record.offset(),
        });
    }

    Ok(FontInfo {
        tables,
        num_tables: table_directory.num_tables(),
    })
}

mod parser;

#[tauri::command]
fn get_table_bytes(path: String, offset: u32, length: u32) -> Result<Vec<u8>, String> {
    use std::io::{Read, Seek};
    let mut file = std::fs::File::open(&path).map_err(|e| e.to_string())?;
    file.seek(std::io::SeekFrom::Start(offset as u64))
        .map_err(|e| e.to_string())?;
    let mut buffer = vec![0; length as usize];
    file.read_exact(&mut buffer).map_err(|e| e.to_string())?;
    Ok(buffer)
}

#[cfg(windows)]
#[tauri::command]
fn show_native_window_menu(window: tauri::Window) -> Result<bool, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    let menu_window = window.clone();
    window
        .run_on_main_thread(move || {
            let _ = tx.send(show_native_window_menu_on_main_thread(menu_window));
        })
        .map_err(|e| e.to_string())?;

    rx.recv().map_err(|e| e.to_string())?
}

#[cfg(windows)]
fn show_native_window_menu_on_main_thread(window: tauri::Window) -> Result<bool, String> {
    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
    let hwnd = HWND(hwnd.0 as isize as _);
    let menu = unsafe { GetSystemMenu(hwnd, false) };

    if menu.0.is_null() {
        return Err("Failed to read native window system menu".into());
    }

    let mut cursor = POINT::default();
    unsafe {
        GetCursorPos(&mut cursor).map_err(|e| e.to_string())?;
        let is_maximized = IsZoomed(hwnd).as_bool();
        let enabled = MF_BYCOMMAND | MF_ENABLED;
        let disabled = MF_BYCOMMAND | MF_GRAYED;

        let _ = EnableMenuItem(
            menu,
            SC_RESTORE,
            if is_maximized { enabled } else { disabled },
        );
        let _ = EnableMenuItem(
            menu,
            SC_MAXIMIZE,
            if is_maximized { disabled } else { enabled },
        );
        let _ = EnableMenuItem(menu, SC_MOVE, if is_maximized { disabled } else { enabled });
        let _ = EnableMenuItem(menu, SC_SIZE, if is_maximized { disabled } else { enabled });
        SetMenuDefaultItem(
            menu,
            if is_maximized {
                SC_RESTORE
            } else {
                SC_MAXIMIZE
            },
            0,
        )
        .map_err(|e| e.to_string())?;

        let _ = SetForegroundWindow(hwnd);

        let command = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            cursor.x,
            cursor.y,
            None,
            hwnd,
            None,
        );

        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));

        if command.0 != 0 {
            PostMessageW(
                Some(hwnd),
                WM_SYSCOMMAND,
                WPARAM(command.0 as usize),
                LPARAM(0),
            )
            .map_err(|e| e.to_string())?;
        }
    }

    Ok(true)
}

#[cfg(not(windows))]
#[tauri::command]
fn show_native_window_menu(_window: tauri::Window) -> Result<bool, String> {
    Ok(false)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            parse_font_tables,
            parser::parse_specific_table,
            get_table_bytes,
            show_native_window_menu
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
