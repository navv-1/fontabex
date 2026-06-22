use super::reader::Reader;
use encoding_rs::{BIG5, EUC_KR, GBK, MACINTOSH, SHIFT_JIS, UTF_16BE, WINDOWS_1252};
use read_fonts::{tables::name::NameRecord, types::NameId, FontRef, TableProvider};
use serde_json::{json, Value};

pub fn parse(font: &FontRef<'_>) -> Result<Value, String> {
    let table = font.name().map_err(|e| e.to_string())?;
    let mut t = Reader::new();
    let string_data = table.string_data();
    let decoded_lang_tags = table
        .lang_tag_record()
        .unwrap_or_default()
        .iter()
        .map(|record| {
            record
                .lang_tag(string_data)
                .map(|value| value.to_string())
                .unwrap_or_else(|error| format!("<{}>", error))
        })
        .collect::<Vec<_>>();

    let version = t.read(table.version(), 2);
    let count = t.read(table.count(), 2);
    let storage_offset = t.read(table.storage_offset(), 2);

    let records_start = t.current_offset();
    let mut records = Vec::new();
    for record in table.name_record() {
        records.push(parse_name_record(record, &mut t, &decoded_lang_tags));
    }

    let records = json!({
        "type": "NameRecord[]",
        "value": records,
        "offset": records_start,
        "length": table.name_record().len() * 12
    });

    let mut result = json!({
        "version": version,
        "count": count,
        "storageOffset": storage_offset,
        "nameRecord": records
    });

    if table.version() == 1 {
        let lang_tag_count_offset = t.current_offset();
        let lang_tag_count = table.lang_tag_count().unwrap_or_default();
        result["langTagCount"] = parsed_field("uint16", lang_tag_count, lang_tag_count_offset, 2);

        let lang_tag_records_start = lang_tag_count_offset + 2;
        let lang_tag_records = table
            .lang_tag_record()
            .unwrap_or_default()
            .iter()
            .enumerate()
            .map(|(index, record)| {
                let record_offset = lang_tag_records_start + index * 4;
                let string_offset =
                    table.storage_offset() as usize + record.lang_tag_offset().to_u32() as usize;
                json!({
                    "length": parsed_field("uint16", record.length(), record_offset, 2),
                    "langTagOffset": parsed_field("Offset16", record.lang_tag_offset().to_u32(), record_offset + 2, 2),
                    "languageTag": parsed_field(
                        "NameString",
                        record
                            .lang_tag(string_data)
                            .map(|value| value.to_string())
                            .unwrap_or_else(|error| format!("<{}>", error)),
                        string_offset,
                        record.length() as usize
                    )
                })
            })
            .collect::<Vec<_>>();

        result["langTagRecord"] = json!({
            "type": "LangTagRecord[]",
            "value": lang_tag_records,
            "offset": lang_tag_records_start,
            "length": lang_tag_count as usize * 4
        });
    }

    if let Some(records_array) = result["nameRecord"]["value"].as_array_mut() {
        for (record_value, record) in records_array.iter_mut().zip(table.name_record()) {
            let string_offset =
                table.storage_offset() as usize + record.string_offset().to_u32() as usize;
            record_value["string"] = parsed_field(
                "NameString",
                decode_name_string(record, string_data),
                string_offset,
                record.length() as usize,
            );
        }
    }

    Ok(result)
}

fn parse_name_record(
    record: &NameRecord,
    reader: &mut Reader,
    decoded_lang_tags: &[String],
) -> Value {
    let mut platform_id = reader.read_as(record.platform_id(), 2, "uint16");
    add_summary(&mut platform_id, platform_label(record.platform_id()));

    let mut encoding_id = reader.read_as(record.encoding_id(), 2, "uint16");
    add_summary(
        &mut encoding_id,
        encoding_label(record.platform_id(), record.encoding_id()),
    );

    let mut language_id = reader.read_as(record.language_id(), 2, "uint16");
    add_summary(
        &mut language_id,
        language_label(
            record.platform_id(),
            record.language_id(),
            decoded_lang_tags,
        ),
    );

    let mut name_id = reader.read_as(record.name_id().to_u16(), 2, "NameId");
    add_summary(&mut name_id, name_id_label(record.name_id()));

    let length = reader.read(record.length(), 2);
    let string_offset = reader.read_as(record.string_offset().to_u32(), 2, "Offset16");

    json!({
        "platformID": platform_id,
        "encodingID": encoding_id,
        "languageID": language_id,
        "nameID": name_id,
        "length": length,
        "stringOffset": string_offset
    })
}

fn parsed_field<T: serde::Serialize>(
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

fn decode_name_string(record: &NameRecord, string_data: read_fonts::FontData<'_>) -> String {
    let raw = match name_string_bytes(record, string_data) {
        Some(raw) => raw,
        None => {
            return record
                .string(string_data)
                .map(|value| value.to_string())
                .unwrap_or_else(|error| format!("<{}>", error));
        }
    };

    let decoded = record
        .string(string_data)
        .map(|value| value.to_string())
        .unwrap_or_default();
    if !decoded.is_empty() || raw.is_empty() {
        return decoded;
    }

    decode_name_string_fallback(record.platform_id(), record.encoding_id(), raw)
}

fn name_string_bytes<'a>(
    record: &NameRecord,
    string_data: read_fonts::FontData<'a>,
) -> Option<&'a [u8]> {
    let start = record.string_offset().to_u32() as usize;
    let end = start.checked_add(record.length() as usize)?;
    string_data.as_bytes().get(start..end)
}

fn decode_name_string_fallback(platform_id: u16, encoding_id: u16, raw: &[u8]) -> String {
    if should_decode_as_utf16be(platform_id, encoding_id, raw) {
        return decode_with_encoding(UTF_16BE, raw);
    }

    let encoding = match (platform_id, encoding_id) {
        (1, 0) => Some(MACINTOSH),
        (1, 1) => Some(SHIFT_JIS),
        (1, 2) => Some(BIG5),
        (1, 3) => Some(EUC_KR),
        (1, 25) => Some(GBK),
        (3, 2) => Some(SHIFT_JIS),
        (3, 3) => Some(GBK),
        (3, 4) => Some(BIG5),
        (3, 5) => Some(EUC_KR),
        _ => None,
    };

    encoding
        .map(|encoding| decode_with_encoding(encoding, raw))
        .unwrap_or_else(|| decode_with_encoding(WINDOWS_1252, raw))
}

fn should_decode_as_utf16be(platform_id: u16, encoding_id: u16, raw: &[u8]) -> bool {
    if raw.len() < 2 || !raw.len().is_multiple_of(2) {
        return false;
    }

    if platform_id == 0 || (platform_id == 3 && matches!(encoding_id, 0 | 1 | 10)) {
        return true;
    }

    let even_zeroes = raw.iter().step_by(2).filter(|byte| **byte == 0).count();
    let pairs = raw.len() / 2;
    even_zeroes * 2 >= pairs
}

fn decode_with_encoding(encoding: &'static encoding_rs::Encoding, raw: &[u8]) -> String {
    let (decoded, _, _) = encoding.decode(raw);
    decoded.into_owned()
}

fn platform_label(platform_id: u16) -> Option<String> {
    let label = match platform_id {
        0 => "Unicode",
        1 => "Macintosh",
        3 => "Windows",
        _ => return None,
    };
    Some(format!("{platform_id} ({label})"))
}

fn encoding_label(platform_id: u16, encoding_id: u16) -> Option<String> {
    let label = match (platform_id, encoding_id) {
        (0, 0) => "Unicode 1.0 semantics",
        (0, 1) => "Unicode 1.1 semantics",
        (0, 2) => "ISO/IEC 10646 semantics",
        (0, 3) => "Unicode 2.0 and onwards semantics, BMP only",
        (0, 4) => "Unicode 2.0 and onwards semantics, full repertoire",
        (1, 0) => "Roman",
        (1, 1) => "Japanese",
        (1, 2) => "Chinese (Traditional)",
        (1, 3) => "Korean",
        (1, 4) => "Arabic",
        (1, 5) => "Hebrew",
        (1, 6) => "Greek",
        (1, 7) => "Russian",
        (1, 8) => "RSymbol",
        (1, 9) => "Devanagari",
        (1, 10) => "Gurmukhi",
        (1, 11) => "Gujarati",
        (1, 12) => "Odia",
        (1, 13) => "Bangla",
        (1, 14) => "Tamil",
        (1, 15) => "Telugu",
        (1, 16) => "Kannada",
        (1, 17) => "Malayalam",
        (1, 18) => "Sinhalese",
        (1, 19) => "Burmese",
        (1, 20) => "Khmer",
        (1, 21) => "Thai",
        (1, 22) => "Laotian",
        (1, 23) => "Georgian",
        (1, 24) => "Armenian",
        (1, 25) => "Chinese (Simplified)",
        (1, 26) => "Tibetan",
        (1, 27) => "Mongolian",
        (1, 28) => "Geez",
        (1, 29) => "Slavic",
        (1, 30) => "Vietnamese",
        (1, 31) => "Sindhi",
        (1, 32) => "Uninterpreted",
        (3, 0) => "Symbol",
        (3, 1) => "Unicode BMP",
        (3, 2) => "ShiftJIS",
        (3, 3) => "PRC",
        (3, 4) => "Big5",
        (3, 5) => "Wansung",
        (3, 6) => "Johab",
        (3, 7) => "Reserved",
        (3, 8) => "Reserved",
        (3, 9) => "Reserved",
        (3, 10) => "Unicode full repertoire",
        _ => return None,
    };
    Some(format!("{encoding_id} ({label})"))
}

fn language_label(
    platform_id: u16,
    language_id: u16,
    decoded_lang_tags: &[String],
) -> Option<String> {
    if language_id >= 0x8000 {
        let index = (language_id - 0x8000) as usize;
        if let Some(tag) = decoded_lang_tags.get(index) {
            return Some(format!("0x{language_id:04X} (Language tag {index}: {tag})"));
        }
        return Some(format!("0x{language_id:04X} (Language tag {index})"));
    }

    let label = match platform_id {
        1 => mac_language_label(language_id)?,
        3 => windows_language_label(language_id)?,
        _ => return None,
    };
    Some(format!("0x{language_id:04X} ({label})"))
}

fn mac_language_label(language_id: u16) -> Option<&'static str> {
    const MAC_LANGUAGE_CODES: &str = include_str!("data/macintosh-language-codes-comma.csv");

    MAC_LANGUAGE_CODES.lines().skip(1).find_map(|line| {
        let (id, language) = line.split_once(',')?;
        (id.parse::<u16>().ok()? == language_id).then_some(language.trim())
    })
}

fn windows_language_label(language_id: u16) -> Option<&'static str> {
    match language_id {
        0x0409 => Some("English (United States)"),
        0x0809 => Some("English (United Kingdom)"),
        0x0404 => Some("Chinese Traditional (Taiwan)"),
        0x0804 => Some("Chinese Simplified (PRC)"),
        0x0411 => Some("Japanese"),
        0x0412 => Some("Korean"),
        0x0407 => Some("German (Germany)"),
        0x0408 => Some("Greek (Greece)"),
        0x0419 => Some("Russian (Russia)"),
        0x0410 => Some("Italian (Italy)"),
        0x040C => Some("French (France)"),
        0x0C0A => Some("Spanish (Spain)"),
        _ => None,
    }
}

pub fn name_id_label(name_id: NameId) -> Option<String> {
    let label = match name_id {
        NameId::COPYRIGHT_NOTICE => "Copyright notice",
        NameId::FAMILY_NAME => "Font Family name",
        NameId::SUBFAMILY_NAME => "Font Subfamily name",
        NameId::UNIQUE_ID => "Unique font identifier",
        NameId::FULL_NAME => "Full font name",
        NameId::VERSION_STRING => "Version string",
        NameId::POSTSCRIPT_NAME => "PostScript name",
        NameId::TRADEMARK => "Trademark",
        NameId::MANUFACTURER => "Manufacturer name",
        NameId::DESIGNER => "Designer",
        NameId::DESCRIPTION => "Description",
        NameId::VENDOR_URL => "Vendor URL",
        NameId::DESIGNER_URL => "Designer URL",
        NameId::LICENSE_DESCRIPTION => "License description",
        NameId::LICENSE_URL => "License info URL",
        NameId::TYPOGRAPHIC_FAMILY_NAME => "Typographic Family name",
        NameId::TYPOGRAPHIC_SUBFAMILY_NAME => "Typographic Subfamily name",
        NameId::COMPATIBLE_FULL_NAME => "Compatible Full name",
        NameId::SAMPLE_TEXT => "Sample text",
        NameId::POSTSCRIPT_CID_NAME => "PostScript CID findfont name",
        NameId::WWS_FAMILY_NAME => "WWS Family name",
        NameId::WWS_SUBFAMILY_NAME => "WWS Subfamily name",
        NameId::LIGHT_BACKGROUND_PALETTE => "Light background palette",
        NameId::DARK_BACKGROUND_PALETTE => "Dark background palette",
        NameId::VARIATIONS_POSTSCRIPT_NAME_PREFIX => "Variations PostScript name prefix",
        _ => return None,
    };
    let name_id_u16 = name_id.to_u16();
    Some(format!("{name_id_u16} ({label})"))
}
