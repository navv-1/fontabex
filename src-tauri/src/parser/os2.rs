use super::reader::Reader;
use read_fonts::tables::os2::SelectionFlags;
use read_fonts::{FontRef, TableProvider};
use serde::Serialize;
use serde_json::{json, Map, Value};

pub fn parse(font: &FontRef<'_>) -> Result<Value, String> {
    let table = font.os2().map_err(|e| e.to_string())?;
    let mut t = Reader::new();
    let mut fields = Map::new();

    fields.insert("version".into(), t.read(table.version(), 2));
    fields.insert("xAvgCharWidth".into(), t.read(table.x_avg_char_width(), 2));
    fields.insert("usWeightClass".into(), t.read(table.us_weight_class(), 2));
    fields.insert("usWidthClass".into(), t.read(table.us_width_class(), 2));
    fields.insert(
        "fsType".into(),
        t.read_as(format_fs_type(table.fs_type()), 2, "uint16"),
    );
    fields.insert(
        "ySubscriptXSize".into(),
        t.read(table.y_subscript_x_size(), 2),
    );
    fields.insert(
        "ySubscriptYSize".into(),
        t.read(table.y_subscript_y_size(), 2),
    );
    fields.insert(
        "ySubscriptXOffset".into(),
        t.read(table.y_subscript_x_offset(), 2),
    );
    fields.insert(
        "ySubscriptYOffset".into(),
        t.read(table.y_subscript_y_offset(), 2),
    );
    fields.insert(
        "ySuperscriptXSize".into(),
        t.read(table.y_superscript_x_size(), 2),
    );
    fields.insert(
        "ySuperscriptYSize".into(),
        t.read(table.y_superscript_y_size(), 2),
    );
    fields.insert(
        "ySuperscriptXOffset".into(),
        t.read(table.y_superscript_x_offset(), 2),
    );
    fields.insert(
        "ySuperscriptYOffset".into(),
        t.read(table.y_superscript_y_offset(), 2),
    );
    fields.insert("yStrikeoutSize".into(), t.read(table.y_strikeout_size(), 2));
    fields.insert(
        "yStrikeoutPosition".into(),
        t.read(table.y_strikeout_position(), 2),
    );
    fields.insert("sFamilyClass".into(), t.read(table.s_family_class(), 2));
    fields.insert("panose".into(), read_panose(&mut t, table.panose_10()));
    fields.insert(
        "ulUnicodeRange1".into(),
        t.read_as(format_u32_hex(table.ul_unicode_range_1()), 4, "uint32"),
    );
    fields.insert(
        "ulUnicodeRange2".into(),
        t.read_as(format_u32_hex(table.ul_unicode_range_2()), 4, "uint32"),
    );
    fields.insert(
        "ulUnicodeRange3".into(),
        t.read_as(format_u32_hex(table.ul_unicode_range_3()), 4, "uint32"),
    );
    fields.insert(
        "ulUnicodeRange4".into(),
        t.read_as(format_u32_hex(table.ul_unicode_range_4()), 4, "uint32"),
    );
    fields.insert("achVendId".into(), t.read_as(table.ach_vend_id(), 4, "Tag"));
    fields.insert(
        "fsSelection".into(),
        t.read_as(format_selection_flags(table.fs_selection()), 2, "uint16"),
    );
    fields.insert(
        "usFirstCharIndex".into(),
        t.read(table.us_first_char_index(), 2),
    );
    fields.insert(
        "usLastCharIndex".into(),
        t.read(table.us_last_char_index(), 2),
    );
    fields.insert("sTypoAscender".into(), t.read(table.s_typo_ascender(), 2));
    fields.insert("sTypoDescender".into(), t.read(table.s_typo_descender(), 2));
    fields.insert("sTypoLineGap".into(), t.read(table.s_typo_line_gap(), 2));
    fields.insert("usWinAscent".into(), t.read(table.us_win_ascent(), 2));
    fields.insert("usWinDescent".into(), t.read(table.us_win_descent(), 2));

    insert_optional(
        &mut fields,
        &mut t,
        "ulCodePageRange1",
        table.ul_code_page_range_1().map(format_u32_hex),
        4,
        "uint32",
    );
    insert_optional(
        &mut fields,
        &mut t,
        "ulCodePageRange2",
        table.ul_code_page_range_2().map(format_u32_hex),
        4,
        "uint32",
    );
    insert_optional(
        &mut fields,
        &mut t,
        "sxHeight",
        table.sx_height(),
        2,
        "int16",
    );
    insert_optional(
        &mut fields,
        &mut t,
        "sCapHeight",
        table.s_cap_height(),
        2,
        "int16",
    );
    insert_optional(
        &mut fields,
        &mut t,
        "usDefaultChar",
        table.us_default_char(),
        2,
        "uint16",
    );
    insert_optional(
        &mut fields,
        &mut t,
        "usBreakChar",
        table.us_break_char(),
        2,
        "uint16",
    );
    insert_optional(
        &mut fields,
        &mut t,
        "usMaxContext",
        table.us_max_context(),
        2,
        "uint16",
    );
    insert_optional(
        &mut fields,
        &mut t,
        "usLowerOpticalPointSize",
        table.us_lower_optical_point_size(),
        2,
        "uint16",
    );
    insert_optional(
        &mut fields,
        &mut t,
        "usUpperOpticalPointSize",
        table.us_upper_optical_point_size(),
        2,
        "uint16",
    );

    Ok(Value::Object(fields))
}

fn insert_optional<T: Serialize>(
    fields: &mut Map<String, Value>,
    reader: &mut Reader,
    name: &str,
    value: Option<T>,
    size: usize,
    data_type: &'static str,
) {
    if let Some(value) = value {
        fields.insert(name.into(), reader.read_as(value, size, data_type));
    }
}

fn format_u32_hex(value: u32) -> String {
    format!("0x{value:08X}")
}

fn read_panose(reader: &mut Reader, bytes: &[u8]) -> Value {
    let offset = reader.current_offset();
    let field_names = [
        "bFamilyType",
        "bSerifStyle",
        "bWeight",
        "bProportion",
        "bContrast",
        "bStrokeVariation",
        "bArmStyle",
        "bLetterform",
        "bMidline",
        "bXHeight",
    ];
    let value = field_names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let byte = bytes.get(index).copied().unwrap_or_default();
            json!({
                "name": name,
                "type": "uint8",
                "value": byte,
                "offset": offset + index,
                "length": 1
            })
        })
        .collect::<Vec<_>>();

    reader.read_as(Value::Array(value), 10, "uint8[]")
}

fn format_fs_type(bits: u16) -> String {
    let mut names = Vec::new();
    for (mask, name) in [
        (0x0002, "Restricted License Embedding"),
        (0x0004, "Preview & Print Embedding"),
        (0x0008, "Editable Embedding"),
        (0x0100, "No Subsetting"),
        (0x0200, "Bitmap Embedding Only"),
    ] {
        if bits & mask != 0 {
            names.push(name);
        }
    }

    let label = if names.is_empty() {
        "Installable Embedding".to_string()
    } else {
        names.join(", ")
    };
    format!("0x{bits:04X} ({label})")
}

fn format_selection_flags(flags: SelectionFlags) -> String {
    let mut names = Vec::new();
    for (flag, name) in [
        (SelectionFlags::ITALIC, "Italic"),
        (SelectionFlags::UNDERSCORE, "Underscore"),
        (SelectionFlags::NEGATIVE, "Negative"),
        (SelectionFlags::OUTLINED, "Outlined"),
        (SelectionFlags::STRIKEOUT, "Strikeout"),
        (SelectionFlags::BOLD, "Bold"),
        (SelectionFlags::REGULAR, "Regular"),
        (SelectionFlags::USE_TYPO_METRICS, "Use typo metrics"),
        (SelectionFlags::WWS, "WWS"),
        (SelectionFlags::OBLIQUE, "Oblique"),
    ] {
        if flags.contains(flag) {
            names.push(name);
        }
    }

    let bits = flags.bits();
    let label = if names.is_empty() {
        "None".to_string()
    } else {
        names.join(", ")
    };
    format!("0x{bits:04X} ({label})")
}
