use read_fonts::{FontRef, TableProvider};
use serde_json::{json, Value};

use super::cff_common::*;
use super::reader::parsed_field;

pub fn parse(font: &FontRef<'_>) -> Result<Value, String> {
    let cff = font.cff2().map_err(|e| e.to_string())?;
    let cff_table = font
        .table_data(read_fonts::types::Tag::new(b"CFF2"))
        .ok_or_else(|| "CFF2 table not found".to_string())?;

    let mut cff_obj = serde_json::Map::new();

    let header = cff.header();
    let _hdr_size = header.header_size() as usize;

    // Header fields
    cff_obj.insert(
        "majorVersion".to_string(),
        parsed_field("uint8", header.major_version(), 0, 1),
    );
    cff_obj.insert(
        "minorVersion".to_string(),
        parsed_field("uint8", header.minor_version(), 1, 1),
    );
    cff_obj.insert(
        "headerSize".to_string(),
        parsed_field("uint8", header.header_size(), 2, 1),
    );
    cff_obj.insert(
        "topDictSize".to_string(),
        parsed_field("uint16", header.top_dict_length(), 3, 2),
    );

    let mut current_offset = header.header_size() as usize;

    // Top DICT Data
    let top_dict_bytes = cff.top_dict_data();
    let top_dict_length = header.top_dict_length() as usize;

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
                    "CharstringsOffset" => key = "CharStringIndexOffset".to_string(),
                    "FdArrayOffset" => key = "FontDictIndexOffset".to_string(),
                    "FdSelectOffset" => key = "FontDictSelectOffset".to_string(),
                    "PrivateDictRange" => key = "PrivateDictOffset".to_string(),
                    "SubrsOffset" => key = "LocalSubrIndexOffset".to_string(),
                    _ => {}
                }

                let (entry_rel_offset, entry_len) =
                    offsets.get(i).copied().unwrap_or((0, dict_bytes.len()));
                let abs_offset = dict_absolute_offset + entry_rel_offset;

                let entry_bytes = &dict_bytes[entry_rel_offset..entry_rel_offset + entry_len];
                let operand_offsets = get_dict_operand_offsets(entry_bytes);

                let (value, type_str): (serde_json::Value, &str) = match &e {
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
                    read_fonts::ps::cff::dict::Entry::CharstringsOffset(offset) => {
                        dict_offsets.charstrings = Some(*offset);
                        (json!(offset), "number")
                    }
                    read_fonts::ps::cff::dict::Entry::SubrsOffset(offset) => {
                        dict_offsets.local_subrs = Some(*offset);
                        (json!(offset), "number")
                    }
                    read_fonts::ps::cff::dict::Entry::VariationStoreOffset(offset) => {
                        dict_offsets.vstore = Some(*offset);
                        (json!(offset), "number")
                    }
                    _ => {
                        let val_str = if let Some(inner) =
                            debug_str.strip_prefix(&format!("{}(", original_key))
                        {
                            inner.strip_suffix(")").unwrap_or(&debug_str)
                        } else {
                            &debug_str
                        };

                        let val_json = if let Ok(num) = val_str.parse::<f64>() {
                            json!(num)
                        } else {
                            json!(val_str)
                        };
                        (val_json, "number")
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
                    "LocalSubrIndex".to_string(),
                    parsed_field(
                        "INDEX",
                        parse_index(
                            cff_table_bytes,
                            abs_subrs_off,
                            size,
                            json!(local_subrs_list),
                            "Bytecode",
                            true,
                        ),
                        abs_subrs_off,
                        size,
                    ),
                );
            }
        }
        Some(priv_entries)
    };

    let dict_absolute_offset = current_offset;
    let mut dict_offsets = DictOffsets::default();

    let (dict_entries, priv_range) =
        parse_dict(top_dict_bytes, dict_absolute_offset, &mut dict_offsets);
    let fd_array_offset = dict_offsets.fd_array;
    let fd_select_offset = dict_offsets.fd_select;
    let charstrings_offset = dict_offsets.charstrings;
    let vstore_offset = dict_offsets.vstore;

    let priv_parsed = priv_range
        .as_ref()
        .and_then(|r| parse_private_dict(r.clone(), cff_table.as_bytes(), &parse_dict, true));
    let charstrings_parsed =
        charstrings_offset.and_then(|off| parse_charstrings(off, cff_table.as_bytes(), true));

    let num_glyphs = charstrings_parsed
        .as_ref()
        .map(|(_, _, count)| *count as u32)
        .unwrap_or(0);

    let fd_array_parsed = fd_array_offset.and_then(|off| {
        parse_fd_array(
            off,
            cff_table.as_bytes(),
            &parse_dict,
            &parse_private_dict,
            true,
        )
    });
    let fd_select_parsed =
        fd_select_offset.and_then(|off| parse_fd_select(off, cff_table.as_bytes(), num_glyphs));
    let vstore_parsed = vstore_offset.and_then(|off| {
        let bytes = cff_table.as_bytes().get(off..)?;
        if bytes.len() < 2 {
            return None;
        }
        use read_fonts::FontRead;
        let font_data = read_fonts::FontData::new(bytes.get(2..)?);
        let vstore = read_fonts::tables::variations::ItemVariationStore::read(font_data).ok()?;
        super::variations::parse_item_variation_store(&vstore, off + 2).ok()
    });

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
        } else if k == "CharStringIndexOffset" {
            if let (Some((index_json, size, _)), Some(offset)) =
                (&charstrings_parsed, charstrings_offset)
            {
                new_entries.insert(
                    "CharStringIndex".to_string(),
                    parsed_field("INDEX", index_json.clone(), offset, *size),
                );
            }
        } else if k == "FontDictIndexOffset" {
            if let (Some((fd_array_json, size)), Some(offset)) = (&fd_array_parsed, fd_array_offset)
            {
                new_entries.insert(
                    "FontDictIndex".to_string(),
                    parsed_field("INDEX", fd_array_json.clone(), offset, *size),
                );
            }
        } else if k == "FontDictSelectOffset" {
            if let (Some((fd_select_map, size)), Some(offset)) =
                (&fd_select_parsed, fd_select_offset)
            {
                new_entries.insert(
                    "FontDictSelect".to_string(),
                    parsed_field("DICT", json!(fd_select_map), offset, *size),
                );
            }
        } else if k == "VariationStoreOffset" {
            if let (Some(vstore_json), Some(offset)) = (&vstore_parsed, vstore_offset) {
                let bytes = cff_table.as_bytes().get(offset..).unwrap_or(&[]);
                let length = if bytes.len() >= 2 {
                    u16::from_be_bytes([bytes[0], bytes[1]]) as usize
                } else {
                    0
                };

                let mut vstore_map = serde_json::Map::new();
                vstore_map.insert(
                    "length".to_string(),
                    parsed_field("uint16", json!(length), offset, 2),
                );
                vstore_map.insert(
                    "data".to_string(),
                    parsed_field(
                        "ItemVariationStore",
                        vstore_json.clone(),
                        offset + 2,
                        length,
                    ),
                );

                new_entries.insert(
                    "VariationStore".to_string(),
                    parsed_field("VariationStore", json!(vstore_map), offset, length + 2),
                );
            }
        }
    }

    cff_obj.insert(
        "TopDict".to_string(),
        parsed_field("DICT", json!(new_entries), current_offset, top_dict_length),
    );

    current_offset += top_dict_length;

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
        "GlobalSubrIndex".to_string(),
        parsed_field(
            "INDEX",
            parse_index(
                cff_table.as_bytes(),
                current_offset,
                global_subrs_size,
                json!(global_subrs_list),
                "Bytecode",
                true,
            ),
            current_offset,
            global_subrs_size,
        ),
    );

    Ok(Value::Object(cff_obj))
}
