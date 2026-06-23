use read_fonts::FontRef;
use serde_json::Value;

pub mod avar;
pub mod cmap;
pub mod fvar;
pub mod head;
pub mod hhea;
pub mod hmtx;
pub mod maxp;
pub mod mvar;
pub mod name;
pub mod os2;
pub mod post;
pub mod reader;
pub mod stat;
pub mod table_directory;
pub mod variations;

#[tauri::command]
pub fn parse_specific_table(path: String, tag: String) -> Result<Value, String> {
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let font = FontRef::new(&bytes).map_err(|e| e.to_string())?;

    match tag.as_str() {
        "head" => head::parse(&font),
        "maxp" => maxp::parse(&font),
        "OS/2" => os2::parse(&font),
        "Table Directory" => table_directory::parse(&font),
        "hhea" => hhea::parse(&font),
        "hmtx" => hmtx::parse(&font),
        "post" => post::parse(&font),
        "name" => name::parse(&font),
        "cmap" => cmap::parse(&font),
        "avar" => avar::parse(&font),
        "fvar" => fvar::parse(&font),
        "MVAR" => mvar::parse(&font),
        "STAT" => stat::parse(&font),
        _ => Err(format!(
            "Parsing for table '{}' is not implemented yet.",
            tag
        )),
    }
}
