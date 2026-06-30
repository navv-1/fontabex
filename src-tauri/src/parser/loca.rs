use read_fonts::{FontRef, TableProvider};
use serde_json::{json, Value};

pub fn parse(font: &FontRef<'_>) -> Result<Value, String> {
    let loca = font.loca(None).map_err(|e| e.to_string())?;
    let head = font.head().map_err(|e| e.to_string())?;

    let is_long = head.index_to_loc_format() == 1;
    let byte_len = if is_long { 4 } else { 2 };
    let start_offset = 0; // loca offsets are relative to loca table start

    // loca length is numGlyphs + 1
    let num_glyphs = font.maxp().map_err(|e| e.to_string())?.num_glyphs() as usize;

    let mut offsets = Vec::with_capacity(num_glyphs + 1);
    let type_str = if is_long { "Offset32" } else { "Offset16" };

    for i in 0..=num_glyphs {
        if let Some(offset) = loca.get_raw(i) {
            offsets.push(json!({
                "type": type_str,
                "value": offset,
                "offset": start_offset + (i * byte_len) as u32,
                "length": byte_len
            }));
        } else {
            break;
        }
    }

    Ok(json!({
        "offsets": {
            "type": format!("{}[]", type_str),
            "value": offsets,
            "offset": 0,
            "length": (num_glyphs + 1) * byte_len
        }
    }))
}
