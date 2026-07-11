use serde_json::{json, Value};

use super::reader::parsed_field;

#[derive(Default)]
pub struct DictOffsets {
    pub fd_array: Option<usize>,
    pub fd_select: Option<usize>,
    pub charset: Option<usize>,
    pub charstrings: Option<usize>,
    pub local_subrs: Option<usize>,
    pub vstore: Option<usize>,
}

pub type ParseDictFn<'a> = &'a dyn Fn(
    &[u8],
    usize,
    &mut DictOffsets,
) -> (
    serde_json::Map<String, Value>,
    Option<std::ops::Range<usize>>,
);

pub type ParsePrivateDictFn<'a> = &'a dyn Fn(
    std::ops::Range<usize>,
    &[u8],
    ParseDictFn<'_>,
    bool,
) -> Option<serde_json::Map<String, Value>>;

pub fn parse_index(
    cff_table: &[u8],
    index_offset: usize,
    index_size: usize,
    data_list: Value,
    data_type: &str,
    is_cff2: bool,
) -> Value {
    let mut map = serde_json::Map::new();
    let index_bytes = if index_offset + index_size <= cff_table.len() {
        &cff_table[index_offset..index_offset + index_size]
    } else {
        &cff_table[index_offset..]
    };

    let count_len = if is_cff2 { 4 } else { 2 };
    let count_type = if is_cff2 { "uint32" } else { "uint16" };

    if index_bytes.len() < count_len {
        map.insert(
            "count".to_string(),
            parsed_field(count_type, json!(0), index_offset, count_len),
        );
        return json!(map);
    }

    let count = if is_cff2 {
        u32::from_be_bytes([
            index_bytes[0],
            index_bytes[1],
            index_bytes[2],
            index_bytes[3],
        ]) as usize
    } else {
        u16::from_be_bytes([index_bytes[0], index_bytes[1]]) as usize
    };

    map.insert(
        "count".to_string(),
        parsed_field(count_type, json!(count), index_offset, count_len),
    );

    if count == 0 {
        return json!(map);
    }

    if index_bytes.len() < count_len + 1 {
        return json!(map);
    }

    let off_size = index_bytes[count_len];
    map.insert(
        "offsetSize".to_string(),
        parsed_field("uint8", json!(off_size), index_offset + count_len, 1),
    );

    let offsets_count = count + 1;
    let offsets_byte_len = offsets_count * off_size as usize;
    let mut offsets_array = Vec::new();

    let offsets_start = count_len + 1;
    if index_bytes.len() >= offsets_start + offsets_byte_len {
        for i in 0..offsets_count {
            let offset_pos = offsets_start + i * off_size as usize;
            let mut offset_val: u32 = 0;
            for j in 0..off_size as usize {
                offset_val = (offset_val << 8) | index_bytes[offset_pos + j] as u32;
            }
            offsets_array.push(parsed_field(
                &format!("Offset{}", off_size * 8),
                json!(offset_val),
                index_offset + offset_pos,
                off_size as usize,
            ));
        }

        map.insert(
            "offsets".to_string(),
            parsed_field(
                &format!("Offset{}[]", off_size * 8),
                json!(offsets_array),
                index_offset + offsets_start,
                offsets_byte_len,
            ),
        );

        let data_start = offsets_start + offsets_byte_len;
        let data_len = index_size.saturating_sub(data_start);

        map.insert(
            "data".to_string(),
            parsed_field(
                &format!("{}[]", data_type),
                data_list,
                index_offset + data_start,
                data_len,
            ),
        );
    }

    json!(map)
}

pub fn get_dict_entry_offsets(data: &[u8]) -> Vec<(usize, usize)> {
    let mut offsets = Vec::new();
    let mut pos = 0;
    let mut entry_start = 0;

    while pos < data.len() {
        let b0 = data[pos];
        let is_operator = b0 <= 27 || b0 == 31;
        let token_len = if is_operator {
            if b0 == 12 && pos + 1 < data.len() {
                2
            } else {
                1
            }
        } else if b0 == 28 {
            3
        } else if b0 == 29 {
            5
        } else if b0 == 30 {
            let mut len = 1;
            let mut current_pos = pos + 1;
            while current_pos < data.len() {
                let b = data[current_pos];
                len += 1;
                current_pos += 1;
                if (b & 0x0F) == 0x0F || (b >> 4) == 0x0F {
                    break;
                }
            }
            len
        } else if (32..=246).contains(&b0) {
            1
        } else if (247..=254).contains(&b0) {
            2
        } else {
            1
        };

        pos += token_len;

        if is_operator {
            offsets.push((entry_start, pos - entry_start));
            entry_start = pos;
        }
    }
    offsets
}

pub fn get_dict_operand_offsets(data: &[u8]) -> Vec<(usize, usize)> {
    let mut offsets = Vec::new();
    let mut pos = 0;

    while pos < data.len() {
        let b0 = data[pos];
        if b0 <= 27 || b0 == 31 {
            break;
        }

        let token_len = if b0 == 28 {
            3
        } else if b0 == 29 {
            5
        } else if b0 == 30 {
            let mut len = 1;
            let mut current_pos = pos + 1;
            while current_pos < data.len() {
                let b = data[current_pos];
                len += 1;
                current_pos += 1;
                if (b & 0x0F) == 0x0F || (b >> 4) == 0x0F {
                    break;
                }
            }
            len
        } else if (32..=246).contains(&b0) {
            1
        } else if (247..=254).contains(&b0) {
            2
        } else {
            1
        };

        offsets.push((pos, token_len));
        pos += token_len;
    }
    offsets
}

pub fn parse_charstrings(
    offset: usize,
    cff_table_bytes: &[u8],
    is_cff2: bool,
) -> Option<(serde_json::Value, usize, usize)> {
    let charstrings =
        read_fonts::ps::cff::index::Index::new(cff_table_bytes.get(offset..)?, is_cff2).ok()?;
    let size = charstrings.size_in_bytes().unwrap_or(0);
    let count = charstrings.count() as usize;
    let mut charstrings_list = Vec::new();
    for i in 0..count {
        if let Ok(charstring_bytes) = charstrings.get(i) {
            let charstring_offset =
                charstring_bytes.as_ptr() as usize - cff_table_bytes.as_ptr() as usize;
            let length = charstring_bytes.len();
            charstrings_list.push(parsed_field(
                "Bytecode",
                json!(format!("[{} bytes]", length)),
                charstring_offset,
                length,
            ));
        }
    }
    let index_json = parse_index(
        cff_table_bytes,
        offset,
        size,
        json!(charstrings_list),
        "Bytecode",
        is_cff2,
    );
    Some((index_json, size, count))
}

pub fn parse_charset(
    offset: usize,
    cff_table_bytes: &[u8],
    num_glyphs: u32,
) -> Option<(serde_json::Map<String, Value>, usize)> {
    if offset <= 2 {
        return None;
    }
    let charset_data = cff_table_bytes.get(offset..)?;
    let charset = <read_fonts::ps::cff::charset::CustomCharset as read_fonts::FontRead>::read(
        charset_data.into(),
    )
    .ok()?;

    let mut charset_map = serde_json::Map::new();
    let size;
    let num_glyphs_to_parse = num_glyphs.saturating_sub(1);

    match charset {
        read_fonts::ps::cff::charset::CustomCharset::Format0(fmt) => {
            let glyphs = fmt.glyph();
            let glyphs_count = num_glyphs_to_parse as usize;
            size = 1 + glyphs_count * 2;
            charset_map.insert(
                "format".to_string(),
                parsed_field("uint8", json!(0), offset, 1),
            );

            let mut glyph_json = Vec::new();
            for i in 0..glyphs_count {
                if let Some(glyph) = glyphs.get(i) {
                    let glyph_id = glyph.get();
                    glyph_json.push(parsed_field("SID", json!(glyph_id), offset + 1 + i * 2, 2));
                }
            }
            charset_map.insert(
                "glyph".to_string(),
                parsed_field("SID[]", json!(glyph_json), offset + 1, glyphs_count * 2),
            );
        }
        read_fonts::ps::cff::charset::CustomCharset::Format1(fmt) => {
            let ranges = fmt.ranges();
            charset_map.insert(
                "format".to_string(),
                parsed_field("uint8", json!(1), offset, 1),
            );

            let mut ranges_json = Vec::new();
            let ranges_offset = offset + 1;
            let mut num_left_total = 0;
            for (i, range) in ranges.iter().enumerate() {
                let first = range.first();
                let n_left = range.n_left();

                let mut range_map = serde_json::Map::new();
                range_map.insert(
                    "first".to_string(),
                    parsed_field("SID", json!(first), ranges_offset + i * 3, 2),
                );
                range_map.insert(
                    "nLeft".to_string(),
                    parsed_field("uint8", json!(n_left), ranges_offset + i * 3 + 2, 1),
                );
                ranges_json.push(parsed_field(
                    "DICT",
                    json!(range_map),
                    ranges_offset + i * 3,
                    3,
                ));

                num_left_total += n_left as u32 + 1;
                if num_left_total >= num_glyphs_to_parse {
                    break;
                }
            }
            size = 1 + ranges_json.len() * 3;
            charset_map.insert(
                "ranges".to_string(),
                parsed_field(
                    "Range1[]",
                    json!(ranges_json),
                    ranges_offset,
                    ranges_json.len() * 3,
                ),
            );
        }
        read_fonts::ps::cff::charset::CustomCharset::Format2(fmt) => {
            let ranges = fmt.ranges();
            charset_map.insert(
                "format".to_string(),
                parsed_field("uint8", json!(2), offset, 1),
            );

            let mut ranges_json = Vec::new();
            let ranges_offset = offset + 1;
            let mut num_left_total = 0;
            for (i, range) in ranges.iter().enumerate() {
                let first = range.first();
                let n_left = range.n_left();

                let mut range_map = serde_json::Map::new();
                range_map.insert(
                    "first".to_string(),
                    parsed_field("SID", json!(first), ranges_offset + i * 4, 2),
                );
                range_map.insert(
                    "nLeft".to_string(),
                    parsed_field("uint16", json!(n_left), ranges_offset + i * 4 + 2, 2),
                );
                ranges_json.push(parsed_field(
                    "DICT",
                    json!(range_map),
                    ranges_offset + i * 4,
                    4,
                ));

                num_left_total += n_left as u32 + 1;
                if num_left_total >= num_glyphs_to_parse {
                    break;
                }
            }
            size = 1 + ranges_json.len() * 4;
            charset_map.insert(
                "ranges".to_string(),
                parsed_field(
                    "Range2[]",
                    json!(ranges_json),
                    ranges_offset,
                    ranges_json.len() * 4,
                ),
            );
        }
    }
    Some((charset_map, size))
}

pub fn parse_fd_select(
    offset: usize,
    cff_table_bytes: &[u8],
    num_glyphs: u32,
) -> Option<(serde_json::Map<String, Value>, usize)> {
    let fd_select_data = cff_table_bytes.get(offset..)?;
    let fd_select = <read_fonts::ps::cff::fd_select::FdSelect as read_fonts::FontRead>::read(
        fd_select_data.into(),
    )
    .ok()?;
    let mut fd_select_map = serde_json::Map::new();
    let size;
    match fd_select {
        read_fonts::ps::cff::fd_select::FdSelect::Format0(fds) => {
            let fds_all = fds.fds();
            let n = num_glyphs as usize;
            let fds_arr = &fds_all[..n.min(fds_all.len())];
            size = 1 + fds_arr.len();

            fd_select_map.insert(
                "format".to_string(),
                parsed_field("uint8", json!(0), offset, 1),
            );

            let fds_vec: Vec<u8> = fds_arr.to_vec();
            fd_select_map.insert(
                "fontDictIDs".to_string(),
                parsed_field("uint8[]", json!(fds_vec), offset + 1, fds_arr.len()),
            );
        }
        read_fonts::ps::cff::fd_select::FdSelect::Format3(fds) => {
            let ranges = fds.ranges();
            size = 1 + 2 + ranges.len() * 3 + 2;

            fd_select_map.insert(
                "format".to_string(),
                parsed_field("uint8", json!(3), offset, 1),
            );
            fd_select_map.insert(
                "numRanges".to_string(),
                parsed_field("uint16", json!(ranges.len()), offset + 1, 2),
            );

            let mut ranges_json = Vec::new();
            let ranges_offset = offset + 3;
            for i in 0..ranges.len() {
                let range = ranges.get(i).unwrap();
                let first = range.first();
                let fd = range.fd();
                let mut range_map = serde_json::Map::new();
                range_map.insert(
                    "first".to_string(),
                    parsed_field("uint16", json!(first), ranges_offset + i * 3, 2),
                );
                range_map.insert(
                    "fontDictID".to_string(),
                    parsed_field("uint8", json!(fd), ranges_offset + i * 3 + 2, 1),
                );
                ranges_json.push(parsed_field(
                    "DICT",
                    json!(range_map),
                    ranges_offset + i * 3,
                    3,
                ));
            }

            fd_select_map.insert(
                "ranges".to_string(),
                parsed_field(
                    "Range3[]",
                    json!(ranges_json),
                    ranges_offset,
                    ranges.len() * 3,
                ),
            );
            fd_select_map.insert(
                "sentinel".to_string(),
                parsed_field(
                    "uint16",
                    json!(fds.sentinel()),
                    ranges_offset + ranges.len() * 3,
                    2,
                ),
            );
        }
        read_fonts::ps::cff::fd_select::FdSelect::Format4(fds) => {
            let ranges = fds.ranges();
            size = 1 + 4 + ranges.len() * 6 + 4;

            fd_select_map.insert(
                "format".to_string(),
                parsed_field("uint8", json!(4), offset, 1),
            );
            fd_select_map.insert(
                "numRanges".to_string(),
                parsed_field("uint32", json!(ranges.len()), offset + 1, 4),
            );

            let mut ranges_json = Vec::new();
            let ranges_offset = offset + 5;
            for i in 0..ranges.len() {
                let range = ranges.get(i).unwrap();
                let first = range.first();
                let fd = range.fd();
                let mut range_map = serde_json::Map::new();
                range_map.insert(
                    "first".to_string(),
                    parsed_field("uint32", json!(first), ranges_offset + i * 6, 4),
                );
                range_map.insert(
                    "fontDictID".to_string(),
                    parsed_field("uint16", json!(fd), ranges_offset + i * 6 + 4, 2),
                );
                ranges_json.push(parsed_field(
                    "DICT",
                    json!(range_map),
                    ranges_offset + i * 6,
                    6,
                ));
            }

            fd_select_map.insert(
                "ranges".to_string(),
                parsed_field(
                    "Range4[]",
                    json!(ranges_json),
                    ranges_offset,
                    ranges.len() * 6,
                ),
            );
            fd_select_map.insert(
                "sentinel".to_string(),
                parsed_field(
                    "uint32",
                    json!(fds.sentinel()),
                    ranges_offset + ranges.len() * 6,
                    4,
                ),
            );
        }
    }
    Some((fd_select_map, size))
}

pub fn parse_fd_array(
    offset: usize,
    cff_table_bytes: &[u8],
    parse_dict_ref: ParseDictFn<'_>,
    parse_private_dict_ref: ParsePrivateDictFn<'_>,
    is_cff2: bool,
) -> Option<(serde_json::Value, usize)> {
    let fd_array =
        read_fonts::ps::cff::index::Index::new(cff_table_bytes.get(offset..)?, is_cff2).ok()?;
    let mut fd_dict_array = Vec::new();
    for i in 0..fd_array.count() {
        if let Ok(dict_bytes) = fd_array.get(i as usize) {
            let dict_absolute_offset =
                dict_bytes.as_ptr() as usize - cff_table_bytes.as_ptr() as usize;
            let mut offsets = DictOffsets::default();
            let (mut dict_entries, priv_range) =
                parse_dict_ref(dict_bytes, dict_absolute_offset, &mut offsets);

            if let Some(range) = priv_range {
                if let Some(priv_entries) =
                    parse_private_dict_ref(range.clone(), cff_table_bytes, parse_dict_ref, is_cff2)
                {
                    let mut new_entries = serde_json::Map::new();
                    for (k, v) in dict_entries {
                        new_entries.insert(k.clone(), v);
                        if k == "PrivateDictOffset" {
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
                    }
                    dict_entries = new_entries;
                }
            }

            fd_dict_array.push(json!(dict_entries));
        }
    }
    let size = fd_array.size_in_bytes().unwrap_or(0);
    let index_json = parse_index(
        cff_table_bytes,
        offset,
        size,
        json!(fd_dict_array),
        "DICT",
        is_cff2,
    );
    Some((index_json, size))
}
