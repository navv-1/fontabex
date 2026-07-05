use read_fonts::{FontRef, TableProvider};
use serde_json::{json, Value};

use super::reader::parsed_field;

pub fn parse(font: &FontRef<'_>) -> Result<Value, String> {
    let cvar = font.cvar().map_err(|e| e.to_string())?;

    let header = cvar.clone();
    let raw_data = cvar.offset_data();
    let has_shared_points = header.tuple_variation_count().shared_point_numbers();

    let mut shared_pts_len = 0;
    if has_shared_points {
        let serialized_data_start_rel = header.data_offset().to_u32() as usize;
        if let Ok(raw_bytes) =
            raw_data.read_array::<u8>(serialized_data_start_rel..serialized_data_start_rel + 4096)
        {
            shared_pts_len = get_packed_point_numbers_len(raw_bytes);
        }
    }

    let fvar = font.fvar().map_err(|e| e.to_string())?;
    let axis_count = fvar.axis_count() as usize;

    let var_data = cvar
        .variation_data(axis_count as u16)
        .map_err(|e| e.to_string())?;
    let tuples = var_data.tuples();

    let mut cursor = header.tuple_variation_headers_byte_range().start;
    let mut parsed_headers = Vec::new();
    let mut per_tuple_data_json = Vec::new();

    let mut serialized_cursor = 0;

    for tuple in tuples {
        let header_start = cursor;
        let variation_data_size = raw_data.read_at::<u16>(cursor).unwrap_or(0) as usize;
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
        let peak_offset = current_tuple_offset;

        // Wait, cvar has no shared tuples array in its header.
        // A tuple variation header with EMBEDDED_PEAK_TUPLE = 0 is considered an error in cvar according to spec,
        // or uses a shared tuple from gvar? No, "In a cvar table, all tuples must have an embedded peak tuple."
        if has_peak {
            current_tuple_offset += tuple_byte_len;
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

        parsed_headers.push(json!({
            "type": "TupleVariationHeader",
            "value": t_obj,
            "offset": header_start,
            "length": current_tuple_offset - header_start
        }));

        cursor = current_tuple_offset;

        // Process TupleVariationData
        let tuple_data_offset =
            header.data_offset().to_u32() as usize + shared_pts_len + serialized_cursor;

        let num_points = tuple.deltas().count();

        let mut per_tuple_obj = serde_json::Map::new();
        let has_private_points = (tuple_index & 0x2000) != 0;
        let mut local_cursor = 0;

        if let Ok(raw_tuple_data) =
            raw_data.read_array::<u8>(tuple_data_offset..tuple_data_offset + variation_data_size)
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

            let deltas_len = get_packed_deltas_len(&raw_slice[local_cursor..], num_points);
            let parsed_deltas: Vec<Value> = raw_slice[local_cursor..local_cursor + deltas_len]
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
                "cvtPackedDeltas".to_string(),
                json!({
                    "type": "PackedDeltas",
                    "value": parsed_deltas,
                    "offset": tuple_data_offset + local_cursor,
                    "length": deltas_len
                }),
            );
        }

        per_tuple_data_json.push(json!({
            "type": "TupleVariationData",
            "value": per_tuple_obj,
            "offset": tuple_data_offset,
            "length": variation_data_size
        }));

        serialized_cursor += variation_data_size;
    }

    let mut serialized_obj = serde_json::Map::new();

    if has_shared_points {
        let serialized_data_start_rel = header.data_offset().to_u32() as usize;
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
                        "offset": serialized_data_start_rel + j,
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
                "offset": serialized_data_start_rel,
                "length": shared_pts_len
            }),
        );
    }
    let serialized_data_start_rel = header.data_offset().to_u32() as usize;
    serialized_obj.insert("perTupleData".to_string(), json!({
        "type": "TupleVariationData[]",
        "value": per_tuple_data_json,
        "offset": if has_shared_points && serialized_obj.contains_key("sharedPointNumbers") {
            serialized_obj.get("sharedPointNumbers").unwrap().get("offset").unwrap().as_u64().unwrap() as usize +
            serialized_obj.get("sharedPointNumbers").unwrap().get("length").unwrap().as_u64().unwrap() as usize
        } else {
            serialized_data_start_rel
        },
        "length": per_tuple_data_json.iter().map(|t| t.get("length").unwrap().as_u64().unwrap() as usize).sum::<usize>()
    }));

    let mut cvar_obj = serde_json::Map::new();
    cvar_obj.insert(
        "majorVersion".to_string(),
        parsed_field("uint16", header.version().major, 0, 2),
    );
    cvar_obj.insert(
        "minorVersion".to_string(),
        parsed_field("uint16", header.version().minor, 2, 2),
    );

    let tvc_offset = header.tuple_variation_count_byte_range().start;
    cvar_obj.insert(
        "tupleVariationCount".to_string(),
        json!({
            "type": "uint16",
            "value": header.tuple_variation_count().bits(),
            "summary": format_tuple_variation_count(header.tuple_variation_count().bits()),
            "offset": tvc_offset,
            "length": 2
        }),
    );

    let data_offset_val = header.data_offset().to_u32();
    cvar_obj.insert(
        "dataOffset".to_string(),
        parsed_field(
            "Offset16",
            data_offset_val,
            header.data_offset_byte_range().start,
            2,
        ),
    );

    let tvh_offset = header.tuple_variation_headers_byte_range().start;
    cvar_obj.insert(
        "tupleVariationHeaders".to_string(),
        json!({
            "type": "TupleVariationHeader[]",
            "value": parsed_headers,
            "offset": tvh_offset,
            "length": cursor - tvh_offset
        }),
    );

    cvar_obj.insert(
        "serializedData".to_string(),
        json!({
            "type": "SerializedData",
            "value": serialized_obj,
            "offset": data_offset_val as usize,
            "length": shared_pts_len + serialized_cursor
        }),
    );

    Ok(json!(cvar_obj))
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

    if !has_embedded_peak {
        let index = val & 0x0FFF;
        parts.push(format!("index: {}", index));
    }

    format!("0x{:04X} ({})", val, parts.join(", "))
}
