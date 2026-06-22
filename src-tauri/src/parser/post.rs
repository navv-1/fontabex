use super::reader::Reader;
use read_fonts::{FontRef, TableProvider};
use serde_json::{json, Map, Value};

pub fn parse(font: &FontRef<'_>) -> Result<Value, String> {
    let table = font.post().map_err(|e| e.to_string())?;
    let mut t = Reader::new();
    let mut fields = Map::new();

    fields.insert(
        "version".into(),
        t.read_as(table.version().to_string(), 4, "Version16Dot16"),
    );
    fields.insert(
        "italicAngle".into(),
        t.read_as(table.italic_angle().to_f64(), 4, "Fixed"),
    );
    fields.insert(
        "underlinePosition".into(),
        t.read_as(table.underline_position(), 2, "FWORD"),
    );
    fields.insert(
        "underlineThickness".into(),
        t.read_as(table.underline_thickness(), 2, "FWORD"),
    );
    fields.insert("isFixedPitch".into(), t.read(table.is_fixed_pitch(), 4));
    fields.insert("minMemType42".into(), t.read(table.min_mem_type42(), 4));
    fields.insert("maxMemType42".into(), t.read(table.max_mem_type42(), 4));
    fields.insert("minMemType1".into(), t.read(table.min_mem_type1(), 4));
    fields.insert("maxMemType1".into(), t.read(table.max_mem_type1(), 4));

    if let Some(num_glyphs) = table.num_glyphs() {
        fields.insert("numGlyphs".into(), t.read(num_glyphs, 2));
    }

    if let Some(glyph_name_index) = table.glyph_name_index() {
        fields.insert(
            "glyphNameIndex".into(),
            read_glyph_name_index(&mut t, glyph_name_index),
        );
    }

    if let Some(string_data) = table.string_data() {
        let start_offset = t.current_offset();
        fields.insert(
            "stringData".into(),
            read_string_data(&mut t, start_offset, string_data.iter()),
        );
    }

    Ok(Value::Object(fields))
}

fn read_glyph_name_index(
    reader: &mut Reader,
    glyph_name_index: &[read_fonts::types::BigEndian<u16>],
) -> Value {
    let start_offset = reader.current_offset();
    let values = glyph_name_index
        .iter()
        .enumerate()
        .map(|(index, value)| {
            json!({
                "type": "uint16",
                "value": value.get(),
                "offset": start_offset + index * 2,
                "length": 2
            })
        })
        .collect::<Vec<_>>();

    reader.read_as(Value::Array(values), glyph_name_index.len() * 2, "uint16[]")
}

fn read_string_data<'a>(
    reader: &mut Reader,
    start_offset: usize,
    strings: impl Iterator<Item = Result<read_fonts::tables::post::PString<'a>, read_fonts::ReadError>>,
) -> Value {
    let mut offset = start_offset;
    let values = strings
        .filter_map(|string| {
            let string = string.ok()?;
            let value = string.as_str();
            let length = 1 + value.len();
            let entry = json!({
                "type": "PString",
                "value": value,
                "offset": offset,
                "length": length
            });
            offset += length;
            Some(entry)
        })
        .collect::<Vec<_>>();

    reader.read_as(Value::Array(values), offset - start_offset, "PString[]")
}
