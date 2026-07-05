use read_fonts::{tables::gvar::Gvar, FontRead, FontRef, TableProvider};
use serde_json::{json, Value};

use super::reader::parsed_field;

pub fn parse(font: &FontRef<'_>) -> Result<Value, String> {
    let gvar = font.gvar().map_err(|e| e.to_string())?;

    let axis_count = gvar.axis_count() as usize;
    let mut parsed_shared_tuples = Vec::new();
    if let Ok(shared_tuples) = gvar.shared_tuples() {
        let tuples_array = shared_tuples.tuples();
        let base_offset = gvar.shared_tuples_offset().to_u32() as usize;
        for i in 0..tuples_array.len() {
            if let Ok(tuple) = tuples_array.get(i) {
                let tuple_offset = base_offset + i * axis_count * 2;
                let values: Vec<Value> = (0..tuple.len())
                    .filter_map(|j| {
                        tuple.get(j).map(|v| {
                            json!({
                                "type": "F2DOT14",
                                "value": v.to_f32(),
                                "offset": tuple_offset + j * 2,
                                "length": 2
                            })
                        })
                    })
                    .collect();
                parsed_shared_tuples.push(json!({
                    "type": "Tuple",
                    "value": values,
                    "offset": tuple_offset,
                    "length": axis_count * 2
                }));
            }
        }
    }

    let glyph_count = gvar.glyph_count();
    let version = gvar.version();

    let is_long = gvar
        .flags()
        .contains(read_fonts::tables::gvar::GvarFlags::LONG_OFFSETS);
    let byte_len = if is_long { 4 } else { 2 };
    let type_str = if is_long { "Offset32" } else { "Offset16" };

    let mut offsets_json = Vec::with_capacity(glyph_count as usize + 1);
    let offsets_array = gvar.glyph_variation_data_offsets();
    let offsets_start = 20;

    for i in 0..=glyph_count as usize {
        if let Ok(offset) = offsets_array.get(i) {
            offsets_json.push(json!({
                "type": type_str,
                "value": offset.get(),
                "offset": offsets_start + i * byte_len,
                "length": byte_len
            }));
        } else {
            break;
        }
    }

    let variation_data_length = if let Ok(last_offset) = offsets_array.get(glyph_count as usize) {
        last_offset.get() as usize
    } else {
        0
    };

    Ok(json!({
        "majorVersion": parsed_field("uint16", version.major, 0, 2),
        "minorVersion": parsed_field("uint16", version.minor, 2, 2),
        "axisCount": parsed_field("uint16", gvar.axis_count(), 4, 2),
        "sharedTupleCount": parsed_field("uint16", gvar.shared_tuple_count(), 6, 2),
        "sharedTuplesOffset": parsed_field("Offset32", gvar.shared_tuples_offset().to_u32(), 8, 4),
        "glyphCount": parsed_field("uint16", glyph_count, 12, 2),
        "flags": {
            "type": "uint16",
            "value": gvar.flags().bits(),
            "summary": format_gvar_flags(gvar.flags().bits()),
            "offset": 14,
            "length": 2
        },
        "glyphVariationDataArrayOffset": parsed_field("Offset32", gvar.glyph_variation_data_array_offset(), 16, 4),
        "glyphVariationDataOffsets": {
            "type": format!("{}[]", type_str),
            "value": offsets_json,
            "offset": offsets_start,
            "length": (glyph_count as usize + 1) * byte_len
        },
        "sharedTuples": {
            "type": "Tuple[]",
            "value": parsed_shared_tuples,
            "offset": gvar.shared_tuples_offset().to_u32(),
            "length": parsed_shared_tuples.len() * (gvar.axis_count() as usize) * 2
        },
        "glyphVariationData": {
            "type": "GlyphVariationData[]",
            "value": {
                "_is_lazy_array": true,
                "lazy_command": "parse_gvar_batch",
                "total": glyph_count,
                "loaded": parse_batch(font, 0, 100, None, None)?["items"].clone()
            },
            "offset": gvar.glyph_variation_data_array_offset(),
            "length": variation_data_length,
            "summary": format!("[{} records]", glyph_count)
        }
    }))
}

pub fn parse_batch(
    font: &FontRef<'_>,
    offset: usize,
    limit: usize,
    search_target: Option<String>,
    search_query: Option<String>,
) -> Result<Value, String> {
    let gvar = font.gvar().map_err(|e| e.to_string())?;
    let num_glyphs = gvar.glyph_count() as usize;

    let mut glyphs = Vec::new();

    if let (Some(target), Some(query)) = (&search_target, &search_query) {
        if target == "index" {
            if let Ok(idx) = query.parse::<usize>() {
                if idx >= offset && idx < num_glyphs {
                    let data = parse_single_glyph(&gvar, idx);
                    glyphs.push(json!({
                        "rowIndex": idx,
                        "data": data
                    }));
                }
            }
            return Ok(json!({
                "items": glyphs,
                "next_offset": num_glyphs
            }));
        }
    }

    let query_lower = search_query.as_ref().map(|q| q.to_lowercase());
    let mut i = offset;
    let mut scanned = 0;

    while i < num_glyphs && glyphs.len() < limit && scanned < 1000 {
        let data = parse_single_glyph(&gvar, i);
        scanned += 1;

        let mut matches = true;
        if let Some(q) = &query_lower {
            let json_str = data.to_string().to_lowercase();
            if !json_str.contains(q) {
                matches = false;
            }
        }

        if matches {
            glyphs.push(json!({
                "rowIndex": i,
                "data": data
            }));
        }

        i += 1;
    }

    Ok(json!({
        "items": glyphs,
        "next_offset": i
    }))
}

fn extract_search_text(value: &Value) -> String {
    if let Some(obj) = value.as_object() {
        if obj.contains_key("value") && obj.contains_key("offset") && obj.contains_key("length") {
            let mut text = String::new();
            if let Some(summary) = obj.get("summary") {
                if let Some(s) = summary.as_str() {
                    text.push_str(s);
                    text.push(' ');
                }
            }
            if let Some(val) = obj.get("value") {
                text.push_str(&extract_search_text(val));
            }
            return text;
        } else {
            let mut text = String::new();
            for val in obj.values() {
                text.push_str(&extract_search_text(val));
                text.push(' ');
            }
            return text;
        }
    }
    if let Some(arr) = value.as_array() {
        let mut text = String::new();
        for val in arr {
            text.push_str(&extract_search_text(val));
            text.push(' ');
        }
        return text;
    }
    if let Some(s) = value.as_str() {
        return s.to_string();
    }
    value.to_string()
}

pub fn search_index(
    font: &FontRef<'_>,
    offset: usize,
    search_target: Option<String>,
    search_query: Option<String>,
    forward: bool,
) -> Result<Value, String> {
    let gvar = font.gvar().map_err(|e| e.to_string())?;
    let num_glyphs = gvar.glyph_count() as usize;

    let query_opt = search_query.as_ref().map(|q| q.to_lowercase());
    if query_opt.is_none() {
        return Ok(json!({ "index": None::<usize>, "next_offset": offset }));
    }
    let query = query_opt.unwrap();
    let target_str = search_target.as_deref().unwrap_or("all");
    let target_key = if let Some(stripped) = target_str.strip_prefix("column:") {
        stripped
    } else {
        target_str
    };

    if target_key == "index" {
        if let Ok(idx) = query.parse::<usize>() {
            if idx < num_glyphs {
                return Ok(json!({ "index": idx, "next_offset": idx }));
            }
        }
        return Ok(json!({ "index": None::<usize>, "next_offset": offset }));
    }

    let mut scanned = 0;
    let mut current = offset;

    while scanned < 2000 {
        if current >= num_glyphs {
            break;
        }

        let data = parse_single_glyph(&gvar, current);
        let search_data = if target_key == "all" {
            Some(&data)
        } else {
            data.get(target_key)
        };

        if let Some(search_val) = search_data {
            let text = extract_search_text(search_val).to_lowercase();
            let is_match = if target_key == "all" {
                text.contains(&query) || current.to_string().contains(&query)
            } else {
                text.contains(&query)
            };

            if is_match {
                return Ok(json!({ "index": current, "next_offset": current }));
            }
        }

        scanned += 1;
        if forward {
            current += 1;
        } else {
            if current == 0 {
                break;
            }
            current -= 1;
        }
    }

    Ok(json!({ "index": None::<usize>, "next_offset": current }))
}

fn parse_single_glyph(gvar: &Gvar<'_>, i: usize) -> Value {
    let gid = read_fonts::types::GlyphId::new(i as u32);

    // We need the absolute offset of this glyph's data from the start of the gvar table
    let data_start = gvar.glyph_variation_data_array_offset();
    let glyph_offset = gvar
        .glyph_variation_data_offsets()
        .get(i)
        .map(|o| o.get())
        .unwrap_or(0);
    let next_glyph_offset = gvar
        .glyph_variation_data_offsets()
        .get(i + 1)
        .map(|o| o.get())
        .unwrap_or(glyph_offset);
    let absolute_offset = (data_start + glyph_offset) as usize;
    let length = (next_glyph_offset - glyph_offset) as usize;

    let empty_result = json!({
        "type": "GlyphVariationData",
        "value": {},
        "offset": absolute_offset,
        "length": length
    });

    let Ok(Some(raw_data)) = gvar.data_for_gid(gid) else {
        return empty_result;
    };

    let Ok(header) = read_fonts::tables::gvar::GlyphVariationDataHeader::read(raw_data) else {
        return empty_result.clone();
    };

    let Ok(Some(var_data)) = gvar.glyph_variation_data(gid) else {
        return empty_result.clone();
    };

    let axis_count = gvar.axis_count() as usize;

    // Start after the tuple variation count and data offset fields (4 bytes)
    let mut cursor = 4;
    let mut tuples_json = Vec::new();

    let tuples = var_data.tuples();
    for tuple in tuples {
        let header_start = absolute_offset + cursor;
        // Read the two header fields manually since they aren't exposed cleanly
        let variation_data_size = raw_data.read_at::<u16>(cursor).unwrap_or(0);
        let tuple_index = raw_data.read_at::<u16>(cursor + 2).unwrap_or(0);

        let has_peak = (tuple_index & 0x8000) != 0;
        let has_inter = (tuple_index & 0x4000) != 0;

        let mut t_obj = serde_json::Map::new();

        t_obj.insert(
            "variationDataSize".to_string(),
            json!({
                "type": "uint16",
                "value": variation_data_size,
                "offset": header_start,
                "length": 2
            }),
        );
        t_obj.insert(
            "tupleIndex".to_string(),
            json!({
                "type": "uint16",
                "value": tuple_index,
                "summary": format_tuple_index(tuple_index),
                "offset": header_start + 2,
                "length": 2
            }),
        );

        let parse_tuple =
            |t: read_fonts::tables::variations::Tuple<'_>, offset: usize, length: usize| {
                let values: Vec<Value> = t
                    .values()
                    .iter()
                    .enumerate()
                    .map(|(j, v)| {
                        json!({
                            "type": "F2DOT14",
                            "value": v.get().to_f32(),
                            "offset": offset + j * 2,
                            "length": 2
                        })
                    })
                    .collect();
                json!({
                    "type": "Tuple",
                    "value": values,
                    "offset": offset,
                    "length": length
                })
            };

        let peak = tuple.peak();
        let tuple_byte_len = axis_count * 2;

        let mut current_tuple_offset = header_start + 4;
        let mut peak_offset = current_tuple_offset;
        if has_peak {
            current_tuple_offset += tuple_byte_len;
        } else {
            // It's a shared tuple! The bytes are in the sharedTuples array.
            let shared_tuples_start = gvar.shared_tuples_offset().to_u32() as usize;
            let shared_idx = (tuple_index & 0x0FFF) as usize;
            peak_offset = shared_tuples_start + shared_idx * tuple_byte_len;
        }

        let inter_start_offset = current_tuple_offset;
        let inter_end_offset = inter_start_offset + tuple_byte_len;
        if has_inter {
            current_tuple_offset += tuple_byte_len * 2;
        }

        if !peak.is_empty() {
            t_obj.insert(
                "peakTuple".to_string(),
                parse_tuple(peak, peak_offset, tuple_byte_len),
            );
        }
        if let Some(start) = tuple.intermediate_start() {
            t_obj.insert(
                "intermediateStartTuple".to_string(),
                parse_tuple(start, inter_start_offset, tuple_byte_len),
            );
        }
        if let Some(end) = tuple.intermediate_end() {
            t_obj.insert(
                "intermediateEndTuple".to_string(),
                parse_tuple(end, inter_end_offset, tuple_byte_len),
            );
        }

        tuples_json.push(json!({
            "type": "TupleVariationHeader",
            "value": t_obj,
            "offset": header_start,
            "length": current_tuple_offset - header_start
        }));

        cursor = current_tuple_offset - absolute_offset;
    }

    let serialized_data_start = absolute_offset + header.serialized_data_offset().to_u32() as usize;
    let serialized_data_length = if length > 0
        && serialized_data_start >= absolute_offset
        && (absolute_offset + length) > serialized_data_start
    {
        (absolute_offset + length) - serialized_data_start
    } else {
        0
    };

    let has_shared_points = (header.tuple_variation_count().bits() & 0x8000) != 0;
    let mut shared_pts_len = 0;
    if has_shared_points {
        let serialized_data_start_rel = header.serialized_data_offset().to_u32() as usize;
        if let Ok(raw_bytes) = raw_data.read_array::<u8>(
            serialized_data_start_rel..serialized_data_start_rel + serialized_data_length,
        ) {
            shared_pts_len = get_packed_point_numbers_len(raw_bytes);
        }
    }

    // Build per-tuple serialized data using variationDataSize from each header
    let mut serialized_cursor = serialized_data_start + shared_pts_len;

    let mut per_tuple_data_json = Vec::new();
    let mut tuple_iter = var_data.tuples();

    // Read variationDataSize for each tuple from raw data to get exact byte sizes
    let mut header_cursor = 4usize; // skip tupleVariationCount + dataOffset
    for _ in 0..tuples_json.len() {
        let var_data_size = raw_data.read_at::<u16>(header_cursor).unwrap_or(0) as usize;
        let tuple_idx_val = raw_data.read_at::<u16>(header_cursor + 2).unwrap_or(0);

        let has_peak = (tuple_idx_val & 0x8000) != 0;
        let has_inter = (tuple_idx_val & 0x4000) != 0;

        // Advance header cursor past this TupleVariationHeader
        header_cursor += 4; // variationDataSize + tupleIndex
        if has_peak {
            header_cursor += axis_count * 2;
        }
        if has_inter {
            header_cursor += axis_count * 4; // start + end
        }

        let tuple_data_offset = serialized_cursor;

        // Get number of points for this tuple to parse the deltas
        let num_points = if let Some(tuple) = tuple_iter.next() {
            tuple.deltas().count()
        } else {
            0
        };

        let mut per_tuple_obj = serde_json::Map::new();

        let has_private_points = (tuple_idx_val & 0x2000) != 0;
        let mut local_cursor = 0;

        let tuple_data_offset_rel = tuple_data_offset - absolute_offset;
        if let Ok(raw_tuple_data) =
            raw_data.read_array::<u8>(tuple_data_offset_rel..tuple_data_offset_rel + var_data_size)
        {
            let raw_slice = raw_tuple_data;

            if has_private_points {
                let private_pts_len = get_packed_point_numbers_len(raw_slice);
                let parsed_private_pts: Vec<Value> = raw_slice
                    [local_cursor..local_cursor + private_pts_len]
                    .iter()
                    .enumerate()
                    .map(|(j, &b)| {
                        json!({
                            "type": "uint8",
                            "value": b,
                            "offset": tuple_data_offset + local_cursor + j,
                            "length": 1
                        })
                    })
                    .collect();

                per_tuple_obj.insert(
                    "privatePointNumbers".to_string(),
                    json!({
                        "type": "PackedPointNumbers",
                        "value": parsed_private_pts,
                        "offset": tuple_data_offset,
                        "length": private_pts_len
                    }),
                );
                local_cursor += private_pts_len;
            } else {
                per_tuple_obj.insert("privatePointNumbers".to_string(), Value::Null);
            }

            let x_len = get_packed_deltas_len(&raw_slice[local_cursor..], num_points);
            let parsed_x: Vec<Value> = raw_slice[local_cursor..local_cursor + x_len]
                .iter()
                .enumerate()
                .map(|(j, &b)| {
                    json!({
                        "type": "uint8",
                        "value": b,
                        "offset": tuple_data_offset + local_cursor + j,
                        "length": 1
                    })
                })
                .collect();

            per_tuple_obj.insert(
                "xPackedDeltas".to_string(),
                json!({
                    "type": "PackedDeltas",
                    "value": parsed_x,
                    "offset": tuple_data_offset + local_cursor,
                    "length": x_len
                }),
            );
            local_cursor += x_len;

            let y_len = get_packed_deltas_len(&raw_slice[local_cursor..], num_points);
            let parsed_y: Vec<Value> = raw_slice[local_cursor..local_cursor + y_len]
                .iter()
                .enumerate()
                .map(|(j, &b)| {
                    json!({
                        "type": "uint8",
                        "value": b,
                        "offset": tuple_data_offset + local_cursor + j,
                        "length": 1
                    })
                })
                .collect();

            per_tuple_obj.insert(
                "yPackedDeltas".to_string(),
                json!({
                    "type": "PackedDeltas",
                    "value": parsed_y,
                    "offset": tuple_data_offset + local_cursor,
                    "length": y_len
                }),
            );
        }
        per_tuple_data_json.push(json!({
            "type": "TupleVariationData",
            "value": per_tuple_obj,
            "offset": tuple_data_offset,
            "length": var_data_size
        }));

        serialized_cursor += var_data_size;
    }

    // Build serialized data object
    let mut serialized_obj = serde_json::Map::new();

    if has_shared_points {
        let serialized_data_start_rel = header.serialized_data_offset().to_u32() as usize;
        let packed_value = if let Ok(raw_bytes) = raw_data
            .read_array::<u8>(serialized_data_start_rel..serialized_data_start_rel + shared_pts_len)
        {
            let slice: &[u8] = raw_bytes;
            let parsed_shared: Vec<Value> = slice
                .iter()
                .enumerate()
                .map(|(j, &b)| {
                    json!({
                        "type": "uint8",
                        "value": b,
                        "offset": serialized_data_start + j,
                        "length": 1
                    })
                })
                .collect();
            json!(parsed_shared)
        } else {
            json!([])
        };

        serialized_obj.insert(
            "sharedPointNumbers".to_string(),
            json!({
                "type": "PackedPointNumbers",
                "value": packed_value,
                "offset": serialized_data_start,
                "length": shared_pts_len
            }),
        );
    }

    serialized_obj.insert("perTupleData".to_string(), json!({
        "type": "TupleVariationData[]",
        "value": per_tuple_data_json,
        "offset": if has_shared_points && serialized_obj.contains_key("sharedPointNumbers") {
            serialized_obj.get("sharedPointNumbers").unwrap().get("offset").unwrap().as_u64().unwrap() as usize +
            serialized_obj.get("sharedPointNumbers").unwrap().get("length").unwrap().as_u64().unwrap() as usize
        } else {
            serialized_data_start
        },
        "length": per_tuple_data_json.iter().map(|t| t.get("length").unwrap().as_u64().unwrap() as usize).sum::<usize>()
    }));

    json!({
        "type": "GlyphVariationData",
        "value": {
            "header": {
                "type": "GlyphVariationDataHeader",
                "value": {
                    "tupleVariationCount": {
                        "type": "uint16",
                        "value": header.tuple_variation_count().bits(),
                        "summary": format_tuple_variation_count(header.tuple_variation_count().bits()),
                        "offset": absolute_offset,
                        "length": 2
                    },
                    "dataOffset": parsed_field("Offset16", header.serialized_data_offset().to_u32(), absolute_offset + 2, 2),
                    "tupleVariationHeaders": {
                        "type": "TupleVariationHeader[]",
                        "value": tuples_json,
                        "offset": absolute_offset + 4,
                        "length": cursor - 4
                    }
                },
                "offset": absolute_offset,
                "length": cursor
            },
            "serializedData": {
                "type": "SerializedData",
                "value": serialized_obj,
                "offset": serialized_data_start,
                "length": serialized_data_length
            }
        },
        "offset": absolute_offset,
        "length": length
    })
}

fn format_tuple_variation_count(val: u16) -> String {
    let mut parts = Vec::new();
    if (val & 0x8000) != 0 {
        parts.push("SHARED_POINT_NUMBERS".to_string());
    }
    let count = val & 0x0FFF;
    parts.push(format!("count: {}", count));
    format!("0x{:04X} ({})", val, parts.join(", "))
}

fn format_gvar_flags(val: u16) -> String {
    let mut parts = Vec::new();
    if (val & 0x0001) != 0 {
        parts.push("LONG_OFFSETS".to_string());
    } else {
        parts.push("SHORT_OFFSETS".to_string());
    }

    format!("0x{:04X} ({})", val, parts.join(", "))
}

fn get_packed_point_numbers_len(data: &[u8]) -> usize {
    if data.is_empty() {
        return 0;
    }

    let first = data[0];
    let (n_points, mut n_bytes) = match first {
        0 => (0, 1),
        count @ 1..=127 => (count as u16, 1),
        _ => {
            if data.len() < 2 {
                return data.len();
            }
            let count = u16::from_be_bytes([data[0], data[1]]) & 0x7FFF;
            if count == 0 {
                (0, 2)
            } else {
                (count, 2)
            }
        }
    };

    if n_points == 0 {
        return n_bytes;
    }

    let mut n_seen = 0;
    while n_seen < n_points && n_bytes < data.len() {
        let control = data[n_bytes];
        n_bytes += 1;
        let count = (control & 0x7F) as u16 + 1;
        let two_bytes = (control & 0x80) != 0;

        let word_size = if two_bytes { 2 } else { 1 };
        let run_size = word_size * count as usize;

        n_bytes += run_size;
        n_seen += count;
    }

    n_bytes.min(data.len())
}

fn get_packed_deltas_len(data: &[u8], num_points: usize) -> usize {
    let mut pos = 0;
    let mut points_seen = 0;
    while points_seen < num_points && pos < data.len() {
        let control = data[pos];
        pos += 1;
        let run_count = (control & 0x3F) as usize + 1;
        let is_zero = (control & 0x80) != 0;
        let is_words = (control & 0x40) != 0;

        let width = if is_zero {
            0
        } else if is_words {
            2
        } else {
            1
        };

        pos += run_count * width;
        points_seen += run_count;
    }
    pos.min(data.len())
}

fn format_tuple_index(val: u16) -> String {
    let mut parts = Vec::new();
    let has_embedded_peak = (val & 0x8000) != 0;

    if has_embedded_peak {
        parts.push("EMBEDDED_PEAK_TUPLE".to_string());
    }
    if (val & 0x4000) != 0 {
        parts.push("INTERMEDIATE_REGION".to_string());
    }
    if (val & 0x2000) != 0 {
        parts.push("PRIVATE_POINT_NUMBERS".to_string());
    }

    // Only show the index if there is no embedded peak tuple.
    // If EMBEDDED_PEAK_TUPLE is set, the low 12 bits are ignored according to the spec.
    if !has_embedded_peak {
        let index = val & 0x0FFF;
        parts.push(format!("index: {}", index));
    }

    format!("0x{:04X} ({})", val, parts.join(", "))
}
