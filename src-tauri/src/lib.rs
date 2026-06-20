// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use read_fonts::FontRef;
use serde::Serialize;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            parse_font_tables,
            parser::parse_specific_table,
            get_table_bytes
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
