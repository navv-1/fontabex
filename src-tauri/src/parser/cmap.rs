use super::reader::Reader;
use read_fonts::tables::cmap::{
    CmapSubtable, ConstantMapGroup, EncodingRecord, PlatformId, SequentialMapGroup,
    VariationSelector,
};
use read_fonts::types::BigEndian;
use read_fonts::{FontRef, TableProvider};
use serde::Serialize;
use serde_json::{json, Map, Value};

pub fn parse(font: &FontRef<'_>) -> Result<Value, String> {
    let table = font.cmap().map_err(|e| e.to_string())?;
    let mut t = Reader::new();

    let version = t.read(table.version(), 2);
    let num_tables = t.read(table.num_tables(), 2);

    let records_start = t.current_offset();
    let records = table
        .encoding_records()
        .iter()
        .map(|record| parse_encoding_record(record, &mut t, table.offset_data()))
        .collect::<Vec<_>>();

    Ok(json!({
        "version": version,
        "numTables": num_tables,
        "encodingRecords": {
            "type": "EncodingRecord[]",
            "value": records,
            "offset": records_start,
            "length": table.encoding_records().len() * 8
        }
    }))
}

fn parse_encoding_record(
    record: &EncodingRecord,
    reader: &mut Reader,
    offset_data: read_fonts::FontData<'_>,
) -> Value {
    let platform_id = reader.read_as(format_platform_id(record.platform_id()), 2, "PlatformId");
    let mut encoding_id = reader.read_as(record.encoding_id(), 2, "uint16");
    add_summary(
        &mut encoding_id,
        encoding_label(record.platform_id(), record.encoding_id()),
    );
    let subtable_offset_value = record.subtable_offset().to_u32();
    let subtable_offset = reader.read_as(subtable_offset_value, 4, "Offset32");

    let subtable = match record.subtable(offset_data) {
        Ok(subtable) => parse_subtable(subtable, subtable_offset_value as usize),
        Err(error) => parsed_field(
            "CmapSubtable",
            format!("<{}>", error),
            subtable_offset_value as usize,
            0,
        ),
    };

    json!({
        "platformID": platform_id,
        "encodingID": encoding_id,
        "subtableOffset": subtable_offset,
        "subtable": subtable
    })
}

fn parse_subtable(subtable: CmapSubtable<'_>, offset: usize) -> Value {
    match subtable {
        CmapSubtable::Format0(table) => parsed_subtable(
            offset,
            table.length() as usize,
            [
                ("format", parsed_field("uint16", table.format(), offset, 2)),
                (
                    "length",
                    parsed_field("uint16", table.length(), offset + 2, 2),
                ),
                (
                    "language",
                    parsed_field("uint16", table.language(), offset + 4, 2),
                ),
                (
                    "glyphIdArray",
                    parsed_array("uint8[]", table.glyph_id_array().to_vec(), offset + 6, 256),
                ),
            ],
        ),
        CmapSubtable::Format2(table) => parsed_subtable(
            offset,
            table.length() as usize,
            [
                ("format", parsed_field("uint16", table.format(), offset, 2)),
                (
                    "length",
                    parsed_field("uint16", table.length(), offset + 2, 2),
                ),
                (
                    "language",
                    parsed_field("uint16", table.language(), offset + 4, 2),
                ),
                (
                    "subHeaderKeys",
                    parsed_array(
                        "uint16[]",
                        be_array_values(table.sub_header_keys()),
                        offset + 6,
                        512,
                    ),
                ),
            ],
        ),
        CmapSubtable::Format4(table) => {
            let seg_count = usize::from(table.seg_count_x2() / 2);
            let end_code_offset = offset + 14;
            let reserved_pad_offset = end_code_offset + seg_count * 2;
            let start_code_offset = reserved_pad_offset + 2;
            let id_delta_offset = start_code_offset + seg_count * 2;
            let id_range_offsets_offset = id_delta_offset + seg_count * 2;
            let glyph_id_array_offset = id_range_offsets_offset + seg_count * 2;
            parsed_subtable(
                offset,
                table.length() as usize,
                [
                    ("format", parsed_field("uint16", table.format(), offset, 2)),
                    (
                        "length",
                        parsed_field("uint16", table.length(), offset + 2, 2),
                    ),
                    (
                        "language",
                        parsed_field("uint16", table.language(), offset + 4, 2),
                    ),
                    (
                        "segCountX2",
                        parsed_field("uint16", table.seg_count_x2(), offset + 6, 2),
                    ),
                    (
                        "searchRange",
                        parsed_field("uint16", table.search_range(), offset + 8, 2),
                    ),
                    (
                        "entrySelector",
                        parsed_field("uint16", table.entry_selector(), offset + 10, 2),
                    ),
                    (
                        "rangeShift",
                        parsed_field("uint16", table.range_shift(), offset + 12, 2),
                    ),
                    (
                        "endCode",
                        parsed_array(
                            "uint16[]",
                            be_array_values(table.end_code()),
                            end_code_offset,
                            seg_count * 2,
                        ),
                    ),
                    (
                        "reservedPad",
                        parsed_field("uint16", 0u16, reserved_pad_offset, 2),
                    ),
                    (
                        "startCode",
                        parsed_array(
                            "uint16[]",
                            be_array_values(table.start_code()),
                            start_code_offset,
                            seg_count * 2,
                        ),
                    ),
                    (
                        "idDelta",
                        parsed_array(
                            "int16[]",
                            table.id_delta().iter().map(|value| value.get()).collect(),
                            id_delta_offset,
                            seg_count * 2,
                        ),
                    ),
                    (
                        "idRangeOffsets",
                        parsed_array(
                            "uint16[]",
                            be_array_values(table.id_range_offsets()),
                            id_range_offsets_offset,
                            seg_count * 2,
                        ),
                    ),
                    (
                        "glyphIdArray",
                        parsed_array(
                            "uint16[]",
                            be_array_values(table.glyph_id_array()),
                            glyph_id_array_offset,
                            table.glyph_id_array().len() * 2,
                        ),
                    ),
                ],
            )
        }
        CmapSubtable::Format6(table) => parsed_subtable(
            offset,
            table.length() as usize,
            [
                ("format", parsed_field("uint16", table.format(), offset, 2)),
                (
                    "length",
                    parsed_field("uint16", table.length(), offset + 2, 2),
                ),
                (
                    "language",
                    parsed_field("uint16", table.language(), offset + 4, 2),
                ),
                (
                    "firstCode",
                    parsed_field("uint16", table.first_code(), offset + 6, 2),
                ),
                (
                    "entryCount",
                    parsed_field("uint16", table.entry_count(), offset + 8, 2),
                ),
                (
                    "glyphIdArray",
                    parsed_array(
                        "uint16[]",
                        be_array_values(table.glyph_id_array()),
                        offset + 10,
                        table.glyph_id_array().len() * 2,
                    ),
                ),
            ],
        ),
        CmapSubtable::Format8(table) => {
            let groups_offset = offset + 16 + 8192 + 4;
            parsed_subtable(
                offset,
                table.length() as usize,
                [
                    ("format", parsed_field("uint16", table.format(), offset, 2)),
                    ("reserved", parsed_field("uint16", 0u16, offset + 2, 2)),
                    (
                        "length",
                        parsed_field("uint32", table.length(), offset + 4, 4),
                    ),
                    (
                        "language",
                        parsed_field("uint32", table.language(), offset + 8, 4),
                    ),
                    (
                        "is32",
                        parsed_array("uint8[]", table.is32().to_vec(), offset + 12, 8192),
                    ),
                    (
                        "numGroups",
                        parsed_field("uint32", table.num_groups(), offset + 8204, 4),
                    ),
                    (
                        "groups",
                        parsed_array(
                            "SequentialMapGroup[]",
                            table
                                .groups()
                                .iter()
                                .map(parse_sequential_map_group)
                                .collect(),
                            groups_offset,
                            table.groups().len() * 12,
                        ),
                    ),
                ],
            )
        }
        CmapSubtable::Format10(table) => parsed_subtable(
            offset,
            table.length() as usize,
            [
                ("format", parsed_field("uint16", table.format(), offset, 2)),
                ("reserved", parsed_field("uint16", 0u16, offset + 2, 2)),
                (
                    "length",
                    parsed_field("uint32", table.length(), offset + 4, 4),
                ),
                (
                    "language",
                    parsed_field("uint32", table.language(), offset + 8, 4),
                ),
                (
                    "startCharCode",
                    parsed_field("uint32", table.start_char_code(), offset + 12, 4),
                ),
                (
                    "numChars",
                    parsed_field("uint32", table.num_chars(), offset + 16, 4),
                ),
                (
                    "glyphIdArray",
                    parsed_array(
                        "uint16[]",
                        be_array_values(table.glyph_id_array()),
                        offset + 20,
                        table.glyph_id_array().len() * 2,
                    ),
                ),
            ],
        ),
        CmapSubtable::Format12(table) => parsed_subtable(
            offset,
            table.length() as usize,
            [
                ("format", parsed_field("uint16", table.format(), offset, 2)),
                ("reserved", parsed_field("uint16", 0u16, offset + 2, 2)),
                (
                    "length",
                    parsed_field("uint32", table.length(), offset + 4, 4),
                ),
                (
                    "language",
                    parsed_field("uint32", table.language(), offset + 8, 4),
                ),
                (
                    "numGroups",
                    parsed_field("uint32", table.num_groups(), offset + 12, 4),
                ),
                (
                    "groups",
                    parsed_array(
                        "SequentialMapGroup[]",
                        table
                            .groups()
                            .iter()
                            .map(parse_sequential_map_group)
                            .collect(),
                        offset + 16,
                        table.groups().len() * 12,
                    ),
                ),
            ],
        ),
        CmapSubtable::Format13(table) => parsed_subtable(
            offset,
            table.length() as usize,
            [
                ("format", parsed_field("uint16", table.format(), offset, 2)),
                ("reserved", parsed_field("uint16", 0u16, offset + 2, 2)),
                (
                    "length",
                    parsed_field("uint32", table.length(), offset + 4, 4),
                ),
                (
                    "language",
                    parsed_field("uint32", table.language(), offset + 8, 4),
                ),
                (
                    "numGroups",
                    parsed_field("uint32", table.num_groups(), offset + 12, 4),
                ),
                (
                    "groups",
                    parsed_array(
                        "ConstantMapGroup[]",
                        table
                            .groups()
                            .iter()
                            .map(parse_constant_map_group)
                            .collect(),
                        offset + 16,
                        table.groups().len() * 12,
                    ),
                ),
            ],
        ),
        CmapSubtable::Format14(table) => parsed_subtable(
            offset,
            table.length() as usize,
            [
                ("format", parsed_field("uint16", table.format(), offset, 2)),
                (
                    "length",
                    parsed_field("uint32", table.length(), offset + 2, 4),
                ),
                (
                    "numVarSelectorRecords",
                    parsed_field("uint32", table.num_var_selector_records(), offset + 6, 4),
                ),
                (
                    "varSelector",
                    parsed_array(
                        "VariationSelector[]",
                        table
                            .var_selector()
                            .iter()
                            .map(parse_variation_selector)
                            .collect(),
                        offset + 10,
                        table.var_selector().len() * 11,
                    ),
                ),
            ],
        ),
    }
}

fn parsed_subtable<'a>(
    offset: usize,
    length: usize,
    fields: impl IntoIterator<Item = (&'a str, Value)>,
) -> Value {
    let value = fields
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect::<Map<_, _>>();

    json!({
        "type": "CmapSubtable",
        "value": Value::Object(value),
        "offset": offset,
        "length": length
    })
}

fn parse_sequential_map_group(group: &SequentialMapGroup) -> Value {
    json!({
        "startCharCode": parsed_field("uint32", format_codepoint(group.start_char_code()), 0, 4),
        "endCharCode": parsed_field("uint32", format_codepoint(group.end_char_code()), 4, 4),
        "startGlyphID": parsed_field("uint32", group.start_glyph_id(), 8, 4)
    })
}

fn parse_constant_map_group(group: &ConstantMapGroup) -> Value {
    json!({
        "startCharCode": parsed_field("uint32", format_codepoint(group.start_char_code()), 0, 4),
        "endCharCode": parsed_field("uint32", format_codepoint(group.end_char_code()), 4, 4),
        "glyphID": parsed_field("uint32", group.glyph_id(), 8, 4)
    })
}

fn parse_variation_selector(selector: &VariationSelector) -> Value {
    json!({
        "varSelector": parsed_field(
            "uint24",
            format_codepoint(selector.var_selector().to_u32()),
            0,
            3
        ),
        "defaultUvsOffset": parsed_field(
            "Offset32",
            selector.default_uvs_offset().offset().to_u32(),
            3,
            4
        ),
        "nonDefaultUvsOffset": parsed_field(
            "Offset32",
            selector.non_default_uvs_offset().offset().to_u32(),
            7,
            4
        )
    })
}

fn parsed_field<T: Serialize>(
    data_type: &'static str,
    value: T,
    offset: usize,
    length: usize,
) -> Value {
    json!({
        "type": data_type,
        "value": value,
        "offset": offset,
        "length": length
    })
}

fn add_summary(field: &mut Value, summary: Option<String>) {
    if let Some(summary) = summary {
        field["summary"] = json!(summary);
    }
}

fn parsed_array<T: Serialize>(
    data_type: &'static str,
    value: Vec<T>,
    offset: usize,
    length: usize,
) -> Value {
    let item_type = data_type.strip_suffix("[]").unwrap_or(data_type);
    let item_length = if value.is_empty() {
        0
    } else {
        length / value.len()
    };
    let value = value
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            parsed_field(item_type, item, offset + index * item_length, item_length)
        })
        .collect::<Vec<_>>();

    json!({
        "type": data_type,
        "value": value,
        "offset": offset,
        "length": length
    })
}

fn be_array_values<T>(values: &[BigEndian<T>]) -> Vec<T>
where
    T: read_fonts::types::Scalar + Copy,
{
    values.iter().map(|value| value.get()).collect()
}

fn format_platform_id(platform_id: PlatformId) -> String {
    let raw = platform_id as u16;
    let label = match platform_id {
        PlatformId::Unicode => "Unicode",
        PlatformId::Macintosh => "Macintosh",
        PlatformId::ISO => "ISO",
        PlatformId::Windows => "Windows",
        PlatformId::Custom => "Custom",
        PlatformId::Unknown => "Unknown",
    };
    format!("{raw} ({label})")
}

fn encoding_label(platform_id: PlatformId, encoding_id: u16) -> Option<String> {
    let label = match platform_id {
        PlatformId::Unicode => unicode_encoding_label(encoding_id)?,
        PlatformId::Macintosh => macintosh_encoding_label(encoding_id)?,
        PlatformId::ISO => iso_encoding_label(encoding_id)?,
        PlatformId::Windows => windows_encoding_label(encoding_id)?,
        PlatformId::Custom | PlatformId::Unknown => return None,
    };
    Some(format!("{encoding_id} ({label})"))
}

fn unicode_encoding_label(encoding_id: u16) -> Option<&'static str> {
    match encoding_id {
        0 => Some("Unicode 1.0 semantics"),
        1 => Some("Unicode 1.1 semantics"),
        2 => Some("ISO/IEC 10646 semantics"),
        3 => Some("Unicode 2.0 and onwards semantics, BMP only"),
        4 => Some("Unicode 2.0 and onwards semantics, full repertoire"),
        5 => Some("Unicode Variation Sequences"),
        6 => Some("Unicode full repertoire"),
        _ => None,
    }
}

fn macintosh_encoding_label(encoding_id: u16) -> Option<&'static str> {
    match encoding_id {
        0 => Some("Roman"),
        1 => Some("Japanese"),
        2 => Some("Chinese (Traditional)"),
        3 => Some("Korean"),
        4 => Some("Arabic"),
        5 => Some("Hebrew"),
        6 => Some("Greek"),
        7 => Some("Russian"),
        8 => Some("RSymbol"),
        9 => Some("Devanagari"),
        10 => Some("Gurmukhi"),
        11 => Some("Gujarati"),
        12 => Some("Odia"),
        13 => Some("Bangla"),
        14 => Some("Tamil"),
        15 => Some("Telugu"),
        16 => Some("Kannada"),
        17 => Some("Malayalam"),
        18 => Some("Sinhalese"),
        19 => Some("Burmese"),
        20 => Some("Khmer"),
        21 => Some("Thai"),
        22 => Some("Laotian"),
        23 => Some("Georgian"),
        24 => Some("Armenian"),
        25 => Some("Chinese (Simplified)"),
        26 => Some("Tibetan"),
        27 => Some("Mongolian"),
        28 => Some("Geez"),
        29 => Some("Slavic"),
        30 => Some("Vietnamese"),
        31 => Some("Sindhi"),
        32 => Some("Uninterpreted"),
        _ => None,
    }
}

fn iso_encoding_label(encoding_id: u16) -> Option<&'static str> {
    match encoding_id {
        0 => Some("7-bit ASCII"),
        1 => Some("ISO/IEC 10646"),
        2 => Some("ISO/IEC 8859-1"),
        _ => None,
    }
}

fn windows_encoding_label(encoding_id: u16) -> Option<&'static str> {
    match encoding_id {
        0 => Some("Symbol"),
        1 => Some("Unicode BMP"),
        2 => Some("ShiftJIS"),
        3 => Some("PRC"),
        4 => Some("Big5"),
        5 => Some("Wansung"),
        6 => Some("Johab"),
        7 => Some("Reserved"),
        8 => Some("Reserved"),
        9 => Some("Reserved"),
        10 => Some("Unicode full repertoire"),
        _ => None,
    }
}

fn format_codepoint(codepoint: u32) -> String {
    format!("U+{codepoint:04X}")
}
