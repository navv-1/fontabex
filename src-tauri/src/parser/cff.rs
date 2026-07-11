use read_fonts::{FontRef, TableProvider};
use serde_json::{json, Value};

use super::cff_common::*;
use super::reader::parsed_field;

pub fn parse(font: &FontRef<'_>) -> Result<Value, String> {
    let cff = font.cff().map_err(|e| e.to_string())?;
    let cff_table = font
        .table_data(read_fonts::types::Tag::new(b"CFF "))
        .ok_or_else(|| "CFF table not found".to_string())?;

    let mut cff_obj = serde_json::Map::new();

    let header = cff.header();
    let hdr_size = header.hdr_size() as usize;

    // Header fields
    cff_obj.insert(
        "majorVersion".to_string(),
        parsed_field("uint8", header.major(), 0, 1),
    );
    cff_obj.insert(
        "minorVersion".to_string(),
        parsed_field("uint8", header.minor(), 1, 1),
    );
    cff_obj.insert(
        "headerSize".to_string(),
        parsed_field("uint8", header.hdr_size(), 2, 1),
    );
    cff_obj.insert(
        "offsetSize".to_string(),
        parsed_field("uint8", header.off_size(), 3, 1),
    );

    let mut current_offset = hdr_size;

    // Name INDEX
    let names = cff.names();
    let names_size = names.size_in_bytes().unwrap_or(0);

    let mut names_list = Vec::new();
    for i in 0..names.count() {
        if let Ok(name_bytes) = names.get(i as usize) {
            let offset = name_bytes.as_ptr() as usize - cff_table.as_bytes().as_ptr() as usize;
            let length = name_bytes.len();
            names_list.push(parsed_field(
                "String",
                json!(String::from_utf8_lossy(name_bytes).to_string()),
                offset,
                length,
            ));
        }
    }

    cff_obj.insert(
        "nameIndex".to_string(),
        parsed_field(
            "INDEX",
            parse_index(
                cff_table.as_bytes(),
                current_offset,
                names_size,
                json!(names_list),
                "String",
                false,
            ),
            current_offset,
            names_size,
        ),
    );

    current_offset += names_size;

    // Top DICT INDEX
    let top_dicts = cff.top_dicts();
    let top_dicts_size = top_dicts.size_in_bytes().unwrap_or(0);

    let parse_dict = |dict_bytes: &[u8],
                      dict_absolute_offset: usize,
                      dict_offsets: &mut DictOffsets|
     -> (
        serde_json::Map<String, Value>,
        Option<std::ops::Range<usize>>,
    ) {
        let mut dict_entries = serde_json::Map::new();
        let mut priv_range = None;
        let offsets = get_dict_entry_offsets(dict_bytes);

        for (i, entry) in read_fonts::ps::cff::dict::entries(dict_bytes, None).enumerate() {
            if let Ok(e) = entry {
                let debug_str = format!("{:?}", e);
                let original_key = debug_str
                    .split(['(', '{'])
                    .next()
                    .unwrap_or("Unknown")
                    .trim()
                    .to_string();
                let mut key = original_key.clone();

                // Rename specific keys for clarity
                match key.as_str() {
                    "PrivateDictRange" => key = "PrivateDictOffset".to_string(),
                    "Charset" => key = "CharsetOffset".to_string(),
                    "Encoding" => key = "EncodingOffset".to_string(),
                    "SubrsOffset" => key = "LocalSubrsOffset".to_string(),
                    _ => {}
                }

                let (entry_rel_offset, entry_len) =
                    offsets.get(i).copied().unwrap_or((0, dict_bytes.len()));
                let abs_offset = dict_absolute_offset + entry_rel_offset;

                let entry_bytes = &dict_bytes[entry_rel_offset..entry_rel_offset + entry_len];
                let operand_offsets = get_dict_operand_offsets(entry_bytes);

                let (value, type_str): (serde_json::Value, &str) = match &e {
                    read_fonts::ps::cff::dict::Entry::Version(sid)
                    | read_fonts::ps::cff::dict::Entry::Notice(sid)
                    | read_fonts::ps::cff::dict::Entry::FullName(sid)
                    | read_fonts::ps::cff::dict::Entry::FamilyName(sid)
                    | read_fonts::ps::cff::dict::Entry::Weight(sid)
                    | read_fonts::ps::cff::dict::Entry::Copyright(sid)
                    | read_fonts::ps::cff::dict::Entry::PostScript(sid)
                    | read_fonts::ps::cff::dict::Entry::BaseFontName(sid)
                    | read_fonts::ps::cff::dict::Entry::FontName(sid) => {
                        let val = if let Some(s) = cff.string(*sid) {
                            format!("{} ({})", sid.to_u16(), String::from_utf8_lossy(s))
                        } else {
                            format!("{}", sid.to_u16())
                        };
                        (json!(val), "SID")
                    }
                    read_fonts::ps::cff::dict::Entry::Ros {
                        registry,
                        ordering,
                        supplement,
                    } => {
                        let reg_str = cff
                            .string(*registry)
                            .map(|b| String::from_utf8_lossy(b).to_string())
                            .unwrap_or_else(|| "".to_string());
                        let ord_str = cff
                            .string(*ordering)
                            .map(|b| String::from_utf8_lossy(b).to_string())
                            .unwrap_or_else(|| "".to_string());
                        let reg_disp = if reg_str.is_empty() {
                            format!("{}", registry.to_u16())
                        } else {
                            format!("{} ({})", registry.to_u16(), reg_str)
                        };
                        let ord_disp = if ord_str.is_empty() {
                            format!("{}", ordering.to_u16())
                        } else {
                            format!("{} ({})", ordering.to_u16(), ord_str)
                        };
                        (
                            json!(format!("{} {} {}", reg_disp, ord_disp, supplement.to_f32())),
                            "SID SID number",
                        )
                    }
                    read_fonts::ps::cff::dict::Entry::FontBbox(bbox) => {
                        let start_idx = operand_offsets.len().saturating_sub(4);
                        let json_arr: Vec<_> = [
                            bbox[0].to_f32(),
                            bbox[1].to_f32(),
                            bbox[2].to_f32(),
                            bbox[3].to_f32(),
                        ]
                        .into_iter()
                        .enumerate()
                        .map(|(idx, f)| {
                            let (rel_op_offset, op_len) = operand_offsets
                                .get(start_idx + idx)
                                .copied()
                                .unwrap_or((0, entry_len));
                            parsed_field("number", json!(f), abs_offset + rel_op_offset, op_len)
                        })
                        .collect();
                        (json!(json_arr), "number[]")
                    }
                    read_fonts::ps::cff::dict::Entry::FontMatrix(sfm) => {
                        let scale = sfm.scale as f64;
                        let m = sfm.matrix;
                        let xx = (m.xx.to_f32() as f64) / scale;
                        let yx = (m.yx.to_f32() as f64) / scale;
                        let xy = (m.xy.to_f32() as f64) / scale;
                        let yy = (m.yy.to_f32() as f64) / scale;
                        let dx = (m.dx.to_f32() as f64) / scale;
                        let dy = (m.dy.to_f32() as f64) / scale;
                        let start_idx = operand_offsets.len().saturating_sub(6);
                        let json_arr: Vec<_> = [xx, yx, xy, yy, dx, dy]
                            .into_iter()
                            .enumerate()
                            .map(|(idx, f)| {
                                let (rel_op_offset, op_len) = operand_offsets
                                    .get(start_idx + idx)
                                    .copied()
                                    .unwrap_or((0, entry_len));
                                parsed_field("number", json!(f), abs_offset + rel_op_offset, op_len)
                            })
                            .collect();
                        (json!(json_arr), "number[]")
                    }
                    read_fonts::ps::cff::dict::Entry::BlueValues(blues)
                    | read_fonts::ps::cff::dict::Entry::OtherBlues(blues)
                    | read_fonts::ps::cff::dict::Entry::FamilyBlues(blues)
                    | read_fonts::ps::cff::dict::Entry::FamilyOtherBlues(blues) => {
                        let mut arr = Vec::new();
                        for (a, b) in blues.values() {
                            arr.push(a.to_f32());
                            arr.push(b.to_f32());
                        }
                        for i in (1..arr.len()).rev() {
                            arr[i] -= arr[i - 1];
                        }
                        let start_idx = operand_offsets.len().saturating_sub(arr.len());
                        let json_arr: Vec<_> = arr
                            .into_iter()
                            .enumerate()
                            .map(|(idx, f)| {
                                let (rel_op_offset, op_len) = operand_offsets
                                    .get(start_idx + idx)
                                    .copied()
                                    .unwrap_or((0, entry_len));
                                parsed_field("delta", json!(f), abs_offset + rel_op_offset, op_len)
                            })
                            .collect();
                        (json!(json_arr), "delta[]")
                    }
                    read_fonts::ps::cff::dict::Entry::StemSnapH(snaps)
                    | read_fonts::ps::cff::dict::Entry::StemSnapV(snaps) => {
                        let mut arr: Vec<f32> = snaps.values().iter().map(|f| f.to_f32()).collect();
                        for i in (1..arr.len()).rev() {
                            arr[i] -= arr[i - 1];
                        }
                        let start_idx = operand_offsets.len().saturating_sub(arr.len());
                        let json_arr: Vec<_> = arr
                            .into_iter()
                            .enumerate()
                            .map(|(idx, f)| {
                                let (rel_op_offset, op_len) = operand_offsets
                                    .get(start_idx + idx)
                                    .copied()
                                    .unwrap_or((0, entry_len));
                                parsed_field("delta", json!(f), abs_offset + rel_op_offset, op_len)
                            })
                            .collect();
                        (json!(json_arr), "delta[]")
                    }
                    read_fonts::ps::cff::dict::Entry::PrivateDictRange(range) => {
                        let size = range.end - range.start;
                        let offset = range.start;
                        priv_range = Some(range.clone());
                        (json!(format!("{} {}", size, offset)), "number number")
                    }
                    read_fonts::ps::cff::dict::Entry::FdArrayOffset(offset) => {
                        dict_offsets.fd_array = Some(*offset);
                        (json!(offset), "number")
                    }
                    read_fonts::ps::cff::dict::Entry::FdSelectOffset(offset) => {
                        dict_offsets.fd_select = Some(*offset);
                        (json!(offset), "number")
                    }
                    read_fonts::ps::cff::dict::Entry::Charset(offset) => {
                        dict_offsets.charset = Some(*offset);
                        (json!(offset), "number")
                    }
                    read_fonts::ps::cff::dict::Entry::CharstringsOffset(offset) => {
                        dict_offsets.charstrings = Some(*offset);
                        (json!(offset), "number")
                    }
                    read_fonts::ps::cff::dict::Entry::SubrsOffset(offset) => {
                        dict_offsets.local_subrs = Some(*offset);
                        (json!(offset), "number")
                    }
                    _ => {
                        let mut t = "number";
                        if key == "isFixedPitch" {
                            t = "boolean";
                        } else if key == "XUID" {
                            t = "number[]";
                        } else if key == "BaseFontBlend" {
                            t = "delta[]";
                        }

                        let val_str = if let Some(inner) =
                            debug_str.strip_prefix(&format!("{}(", original_key))
                        {
                            inner.strip_suffix(")").unwrap_or(&debug_str)
                        } else {
                            &debug_str
                        };

                        let val_json = if t == "boolean" {
                            json!(val_str == "true" || val_str == "1")
                        } else if let Ok(num) = val_str.parse::<f64>() {
                            json!(num)
                        } else {
                            json!(val_str)
                        };
                        (val_json, t)
                    }
                };

                dict_entries.insert(key, parsed_field(type_str, value, abs_offset, entry_len));
            }
        }
        (dict_entries, priv_range)
    };

    let parse_private_dict = |range: std::ops::Range<usize>,
                              cff_table_bytes: &[u8],
                              parse_dict_ref: ParseDictFn<'_>,
                              is_cff2: bool|
     -> Option<serde_json::Map<String, Value>> {
        let priv_bytes = cff_table_bytes.get(range.clone())?;
        let mut dict_offsets = DictOffsets::default();
        let (mut priv_entries, _) = parse_dict_ref(priv_bytes, range.start, &mut dict_offsets);

        if let Some(subrs_off) = dict_offsets.local_subrs {
            let abs_subrs_off = range.start + subrs_off;
            if let Ok(local_subrs) = read_fonts::ps::cff::index::Index::new(
                cff_table_bytes.get(abs_subrs_off..).unwrap_or(&[]),
                is_cff2,
            ) {
                let size = local_subrs.size_in_bytes().unwrap_or(0);
                let mut local_subrs_list = Vec::new();
                for i in 0..local_subrs.count() {
                    if let Ok(subr_bytes) = local_subrs.get(i as usize) {
                        let offset =
                            subr_bytes.as_ptr() as usize - cff_table_bytes.as_ptr() as usize;
                        let length = subr_bytes.len();
                        local_subrs_list.push(parsed_field(
                            "Bytecode",
                            json!(format!("[{} bytes]", length)),
                            offset,
                            length,
                        ));
                    }
                }

                priv_entries.insert(
                    "localSubrsIndex".to_string(),
                    parsed_field(
                        "INDEX",
                        parse_index(
                            cff_table_bytes,
                            abs_subrs_off,
                            size,
                            json!(local_subrs_list),
                            "Bytecode",
                            false,
                        ),
                        abs_subrs_off,
                        size,
                    ),
                );
            }
        }
        Some(priv_entries)
    };

    let mut top_dict_array = Vec::new();

    for i in 0..top_dicts.count() {
        if let Ok(dict_bytes) = top_dicts.get(i as usize) {
            let dict_absolute_offset =
                dict_bytes.as_ptr() as usize - cff_table.as_bytes().as_ptr() as usize;
            let mut dict_offsets = DictOffsets::default();
            let (mut dict_entries, priv_range) =
                parse_dict(dict_bytes, dict_absolute_offset, &mut dict_offsets);
            let fd_array_offset = dict_offsets.fd_array;
            let fd_select_offset = dict_offsets.fd_select;
            let charset_offset = dict_offsets.charset;
            let charstrings_offset = dict_offsets.charstrings;

            let priv_parsed = priv_range.as_ref().and_then(|r| {
                parse_private_dict(r.clone(), cff_table.as_bytes(), &parse_dict, false)
            });
            let charstrings_parsed = charstrings_offset
                .and_then(|off| parse_charstrings(off, cff_table.as_bytes(), false));

            let mut num_glyphs = 0;
            if let Some((_, _, count)) = &charstrings_parsed {
                num_glyphs = *count as u32;
            }

            let charset_parsed =
                charset_offset.and_then(|off| parse_charset(off, cff_table.as_bytes(), num_glyphs));
            let fd_array_parsed = fd_array_offset.and_then(|off| {
                parse_fd_array(
                    off,
                    cff_table.as_bytes(),
                    &parse_dict,
                    &parse_private_dict,
                    false,
                )
            });
            let fd_select_parsed = fd_select_offset
                .and_then(|off| parse_fd_select(off, cff_table.as_bytes(), num_glyphs));

            let mut new_entries = serde_json::Map::new();
            for (k, v) in dict_entries {
                new_entries.insert(k.clone(), v);
                if k == "PrivateDictOffset" {
                    if let (Some(priv_entries), Some(range)) = (&priv_parsed, &priv_range) {
                        new_entries.insert(
                            "PrivateDict".to_string(),
                            parsed_field(
                                "DICT",
                                json!(priv_entries),
                                range.start,
                                range.end - range.start,
                            ),
                        );
                    }
                } else if k == "CharstringsOffset" {
                    if let (Some((index_json, size, _)), Some(offset)) =
                        (&charstrings_parsed, charstrings_offset)
                    {
                        new_entries.insert(
                            "CharstringsIndex".to_string(),
                            parsed_field("INDEX", index_json.clone(), offset, *size),
                        );
                    }
                } else if k == "CharsetOffset" {
                    if let (Some((charset_map, size)), Some(offset)) =
                        (&charset_parsed, charset_offset)
                    {
                        new_entries.insert(
                            "Charset".to_string(),
                            parsed_field("DICT", json!(charset_map), offset, *size),
                        );
                    }
                } else if k == "FdArrayOffset" {
                    if let (Some((fd_array_json, size)), Some(offset)) =
                        (&fd_array_parsed, fd_array_offset)
                    {
                        new_entries.insert(
                            "FdArray".to_string(),
                            parsed_field("INDEX", fd_array_json.clone(), offset, *size),
                        );
                    }
                } else if k == "FdSelectOffset" {
                    if let (Some((fd_select_map, size)), Some(offset)) =
                        (&fd_select_parsed, fd_select_offset)
                    {
                        new_entries.insert(
                            "FdSelect".to_string(),
                            parsed_field("DICT", json!(fd_select_map), offset, *size),
                        );
                    }
                }
            }
            dict_entries = new_entries;

            top_dict_array.push(json!(dict_entries));
        }
    }

    cff_obj.insert(
        "topDictIndex".to_string(),
        parsed_field(
            "INDEX",
            parse_index(
                cff_table.as_bytes(),
                current_offset,
                top_dicts_size,
                json!(top_dict_array),
                "DICT",
                false,
            ),
            current_offset,
            top_dicts_size,
        ),
    );

    current_offset += top_dicts_size;

    // String INDEX
    let strings = cff.strings();
    let strings_size = strings.size_in_bytes().unwrap_or(0);
    let mut strings_list = Vec::new();
    for i in 0..strings.count() {
        if let Ok(str_bytes) = strings.get(i as usize) {
            let offset = str_bytes.as_ptr() as usize - cff_table.as_bytes().as_ptr() as usize;
            let length = str_bytes.len();
            strings_list.push(parsed_field(
                "String",
                json!(String::from_utf8_lossy(str_bytes).to_string()),
                offset,
                length,
            ));
        }
    }

    cff_obj.insert(
        "stringIndex".to_string(),
        parsed_field(
            "INDEX",
            parse_index(
                cff_table.as_bytes(),
                current_offset,
                strings_size,
                json!(strings_list),
                "String",
                false,
            ),
            current_offset,
            strings_size,
        ),
    );

    current_offset += strings_size;

    // Global Subrs INDEX
    let global_subrs = cff.global_subrs();
    let global_subrs_size = global_subrs.size_in_bytes().unwrap_or(0);

    let mut global_subrs_list = Vec::new();
    for i in 0..global_subrs.count() {
        if let Ok(subr_bytes) = global_subrs.get(i as usize) {
            let offset = subr_bytes.as_ptr() as usize - cff_table.as_bytes().as_ptr() as usize;
            let length = subr_bytes.len();
            global_subrs_list.push(parsed_field(
                "Bytecode",
                json!(format!("[{} bytes]", length)),
                offset,
                length,
            ));
        }
    }

    cff_obj.insert(
        "globalSubrsIndex".to_string(),
        parsed_field(
            "INDEX",
            parse_index(
                cff_table.as_bytes(),
                current_offset,
                global_subrs_size,
                json!(global_subrs_list),
                "Bytecode",
                false,
            ),
            current_offset,
            global_subrs_size,
        ),
    );

    Ok(Value::Object(cff_obj))
}
