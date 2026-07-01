use read_fonts::{FontRef, TableProvider};
use serde_json::{json, Value};

pub fn parse(font: &FontRef<'_>) -> Result<Value, String> {
    let maxp = font.maxp().map_err(|e| e.to_string())?;
    let num_glyphs = maxp.num_glyphs() as usize;

    // Return the lazy array wrapper for the frontend
    Ok(json!({
        "_is_lazy_array": true,
        "lazy_command": "parse_glyf_batch",
        "total": num_glyphs,
        "loaded": parse_batch(font, 0, 100, None, None)?["items"].clone()
    }))
}

pub fn parse_batch(
    font: &FontRef<'_>,
    offset: usize,
    limit: usize,
    search_target: Option<String>,
    search_query: Option<String>,
) -> Result<Value, String> {
    let loca = font.loca(None).map_err(|e| e.to_string())?;
    let glyf = font.glyf().map_err(|e| e.to_string())?;
    let maxp = font.maxp().map_err(|e| e.to_string())?;
    let num_glyphs = maxp.num_glyphs() as usize;

    let mut glyphs = Vec::new();

    // Fast path for index search
    if let (Some(target), Some(query)) = (&search_target, &search_query) {
        if target == "index" {
            if let Ok(idx) = query.parse::<usize>() {
                if idx >= offset && idx < num_glyphs {
                    let data = parse_single_glyph(&loca, &glyf, idx);
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
        let data = parse_single_glyph(&loca, &glyf, i);
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
    let loca = font.loca(None).map_err(|e| e.to_string())?;
    let glyf = font.glyf().map_err(|e| e.to_string())?;
    let maxp = font.maxp().map_err(|e| e.to_string())?;
    let num_glyphs = maxp.num_glyphs() as usize;

    let query_opt = search_query.as_ref().map(|q| q.to_lowercase());
    if query_opt.is_none() {
        return Ok(json!({ "index": None::<usize>, "next_offset": offset }));
    }
    let query = query_opt.unwrap();
    let target_str = search_target.as_deref().unwrap_or("all");
    let target_key = if target_str.starts_with("column:") {
        &target_str["column:".len()..]
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

        let data = parse_single_glyph(&loca, &glyf, current);
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

fn parse_single_glyph(
    loca: &read_fonts::tables::loca::Loca<'_>,
    glyf: &read_fonts::tables::glyf::Glyf<'_>,
    i: usize,
) -> Value {
    let gid = read_fonts::types::GlyphId::new(i as u32);
    let glyph_res = loca.get_glyf(gid, glyf);

    let start_offset = loca.get_raw(i).unwrap_or(0);
    let end_offset = loca.get_raw(i + 1).unwrap_or(start_offset);
    let total_length = end_offset.saturating_sub(start_offset);

    match glyph_res {
        Ok(Some(glyph)) => {
            let mut curr = start_offset;

            let header_obj = json!({
                "numberOfContours": {
                    "type": "int16",
                    "value": match glyph {
                        read_fonts::tables::glyf::Glyph::Simple(ref s) => s.number_of_contours(),
                        read_fonts::tables::glyf::Glyph::Composite(_) => -1,
                    },
                    "offset": curr,
                    "length": 2
                },
                "xMin": { "type": "int16", "value": glyph.x_min(), "offset": curr + 2, "length": 2 },
                "yMin": { "type": "int16", "value": glyph.y_min(), "offset": curr + 4, "length": 2 },
                "xMax": { "type": "int16", "value": glyph.x_max(), "offset": curr + 6, "length": 2 },
                "yMax": { "type": "int16", "value": glyph.y_max(), "offset": curr + 8, "length": 2 },
            });

            let header = json!({
                "type": "GlyphHeader",
                "value": header_obj,
                "offset": curr,
                "length": std::cmp::min(10, total_length)
            });

            curr += 10;

            let mut glyph_data = json!({
                "header": header
            });

            match glyph {
                read_fonts::tables::glyf::Glyph::Simple(simple) => {
                    let num_contours = simple.number_of_contours() as u32;
                    let end_pts: Vec<u16> = simple
                        .end_pts_of_contours()
                        .iter()
                        .map(|p| p.get())
                        .collect();
                    let pts_length = num_contours * 2;
                    let inst_len = simple.instruction_length();

                    let data = simple.glyph_data();
                    let n_points = end_pts.last().map(|last| *last as u16 + 1).unwrap_or(0);
                    let mut flags_bytes = Vec::new();
                    let mut x_coords = Vec::new();
                    let mut y_coords = Vec::new();

                    let flags_offset = curr + pts_length + 2 + inst_len as u32;
                    let mut x_offset = flags_offset;
                    let mut y_offset = flags_offset;

                    if let Ok((f_len, x_len, y_len)) = resolve_coords_len(data, n_points) {
                        if f_len as usize + x_len as usize + y_len as usize <= data.len() {
                            flags_bytes = data[..f_len as usize].to_vec();
                            x_coords = data[f_len as usize..(f_len + x_len) as usize].to_vec();
                            y_coords = data
                                [(f_len + x_len) as usize..(f_len + x_len + y_len) as usize]
                                .to_vec();
                            x_offset = flags_offset + f_len;
                            y_offset = x_offset + x_len;
                        }
                    }

                    let end_pts_array: Vec<_> = end_pts
                        .iter()
                        .enumerate()
                        .map(|(idx, &val)| {
                            json!({
                                "type": "uint16",
                                "value": val,
                                "offset": curr + (idx * 2) as u32,
                                "length": 2
                            })
                        })
                        .collect();

                    let inst_array: Vec<_> = simple
                        .instructions()
                        .iter()
                        .enumerate()
                        .map(|(idx, &val)| {
                            json!({
                                "type": "uint8",
                                "value": val,
                                "offset": curr + pts_length + 2 + idx as u32,
                                "length": 1
                            })
                        })
                        .collect();

                    let flags_array: Vec<_> = flags_bytes
                        .iter()
                        .enumerate()
                        .map(|(idx, &val)| {
                            json!({
                                "type": "uint8",
                                "value": val,
                                "summary": format_simple_flag(val),
                                "offset": flags_offset + idx as u32,
                                "length": 1
                            })
                        })
                        .collect();

                    let x_coords_array =
                        parse_coordinate_array(&flags_bytes, &x_coords, true, x_offset);
                    let y_coords_array =
                        parse_coordinate_array(&flags_bytes, &y_coords, false, y_offset);

                    let simple_obj = json!({
                        "endPtsOfContours": {
                            "type": "uint16[]",
                            "value": end_pts_array,
                            "offset": curr,
                            "length": pts_length
                        },
                        "instructionLength": {
                            "type": "uint16",
                            "value": inst_len,
                            "offset": curr + pts_length,
                            "length": 2
                        },
                        "instructions": {
                            "type": "uint8[]",
                            "value": inst_array,
                            "offset": curr + pts_length + 2,
                            "length": inst_len as u32
                        },
                        "flags": {
                            "type": "uint8[]",
                            "value": flags_array,
                            "offset": flags_offset,
                            "length": flags_bytes.len()
                        },
                        "xCoordinates": {
                            "type": "(uint8|int16)[]",
                            "value": x_coords_array,
                            "offset": x_offset,
                            "length": x_coords.len()
                        },
                        "yCoordinates": {
                            "type": "(uint8|int16)[]",
                            "value": y_coords_array,
                            "offset": y_offset,
                            "length": y_coords.len()
                        },
                    });

                    let simple_len = total_length.saturating_sub(10);
                    let simple_val = json!({
                        "type": "SimpleGlyph",
                        "value": simple_obj,
                        "offset": curr,
                        "length": simple_len
                    });

                    glyph_data
                        .as_object_mut()
                        .unwrap()
                        .insert("simple".to_string(), simple_val);
                }
                read_fonts::tables::glyf::Glyph::Composite(composite) => {
                    let data = composite.component_data();
                    let (components, comp_len, has_instructions) =
                        parse_composite_components(data, curr + 10);
                    let mut composite_obj_map = serde_json::Map::new();
                    composite_obj_map.insert(
                        "components".to_string(),
                        json!({
                            "type": "ComponentGlyph[]",
                            "value": components,
                            "offset": curr + 10,
                            "length": comp_len
                        }),
                    );

                    if has_instructions {
                        let inst_offset = comp_len as usize;
                        if inst_offset + 2 <= data.len() {
                            let num_instr =
                                u16::from_be_bytes([data[inst_offset], data[inst_offset + 1]]);
                            composite_obj_map.insert(
                                "instructionLength".to_string(),
                                json!({
                                    "type": "uint16",
                                    "value": num_instr,
                                    "offset": curr + 10 + inst_offset as u32,
                                    "length": 2
                                }),
                            );

                            let instr_len = num_instr as usize;
                            if inst_offset + 2 + instr_len <= data.len() {
                                let instr_data =
                                    &data[inst_offset + 2..inst_offset + 2 + instr_len];
                                let instr_array: Vec<_> = instr_data.iter().enumerate().map(|(idx, &val)| {
                                    json!({
                                        "type": "uint8",
                                        "value": val,
                                        "offset": curr + 10 + inst_offset as u32 + 2 + idx as u32,
                                        "length": 1
                                    })
                                }).collect();

                                composite_obj_map.insert(
                                    "instructions".to_string(),
                                    json!({
                                        "type": "uint8[]",
                                        "value": instr_array,
                                        "offset": curr + 10 + inst_offset as u32 + 2,
                                        "length": instr_len
                                    }),
                                );
                            }
                        }
                    }

                    let composite_obj = json!(composite_obj_map);

                    let composite_len = total_length.saturating_sub(10);
                    let composite = json!({
                        "type": "CompositeGlyph",
                        "value": composite_obj,
                        "offset": curr,
                        "length": composite_len
                    });

                    glyph_data
                        .as_object_mut()
                        .unwrap()
                        .insert("composite".to_string(), composite);
                }
            }

            glyph_data
        }
        Ok(None) => {
            json!({})
        }
        Err(e) => {
            json!({
                "error": e.to_string(),
            })
        }
    }
}

fn resolve_coords_len(data: &[u8], points_total: u16) -> Result<(u32, u32, u32), ()> {
    let mut offset = 0;
    let mut flags_left = points_total as u32;
    let mut x_coords_len = 0;
    let mut y_coords_len = 0;

    while flags_left > 0 {
        if offset >= data.len() {
            return Err(());
        }
        let flag = data[offset];
        offset += 1;

        let repeats = if (flag & 0x08) != 0 {
            if offset >= data.len() {
                return Err(());
            }
            let r = data[offset];
            offset += 1;
            r as u32 + 1
        } else {
            1
        };

        if repeats > flags_left {
            return Err(());
        }

        let x_short = (flag & 0x02) != 0;
        let x_same_pos = (flag & 0x10) != 0;
        if x_short {
            x_coords_len += repeats;
        } else if !x_same_pos {
            x_coords_len += repeats * 2;
        }

        let y_short = (flag & 0x04) != 0;
        let y_same_pos = (flag & 0x20) != 0;
        if y_short {
            y_coords_len += repeats;
        } else if !y_same_pos {
            y_coords_len += repeats * 2;
        }

        flags_left -= repeats;
    }

    Ok((offset as u32, x_coords_len, y_coords_len))
}

fn format_simple_flag(val: u8) -> String {
    let mut meanings = Vec::new();
    if (val & 0x01) != 0 {
        meanings.push("ON_CURVE");
    }
    if (val & 0x02) != 0 {
        meanings.push("X_SHORT");
    }
    if (val & 0x04) != 0 {
        meanings.push("Y_SHORT");
    }
    if (val & 0x08) != 0 {
        meanings.push("REPEAT");
    }
    if (val & 0x10) != 0 {
        meanings.push("X_SAME/POS");
    }
    if (val & 0x20) != 0 {
        meanings.push("Y_SAME/POS");
    }
    if (val & 0x40) != 0 {
        meanings.push("OVERLAP");
    }

    if meanings.is_empty() {
        format!("0x{:02X}", val)
    } else {
        format!("0x{:02X} ({})", val, meanings.join(", "))
    }
}

fn format_component_flag(val: u16) -> String {
    let mut meanings = Vec::new();
    if (val & 0x0001) != 0 {
        meanings.push("ARGS_WORDS");
    }
    if (val & 0x0002) != 0 {
        meanings.push("ARGS_XY");
    }
    if (val & 0x0004) != 0 {
        meanings.push("ROUND_XY");
    }
    if (val & 0x0008) != 0 {
        meanings.push("SCALE");
    }
    if (val & 0x0020) != 0 {
        meanings.push("MORE_COMPONENTS");
    }
    if (val & 0x0040) != 0 {
        meanings.push("XY_SCALE");
    }
    if (val & 0x0080) != 0 {
        meanings.push("2X2");
    }
    if (val & 0x0100) != 0 {
        meanings.push("INSTR");
    }
    if (val & 0x0200) != 0 {
        meanings.push("USE_METRICS");
    }
    if (val & 0x0400) != 0 {
        meanings.push("OVERLAP");
    }
    if (val & 0x0800) != 0 {
        meanings.push("SCALED_OFFSET");
    }
    if (val & 0x1000) != 0 {
        meanings.push("UNSCALED_OFFSET");
    }

    if meanings.is_empty() {
        format!("0x{:04X}", val)
    } else {
        format!("0x{:04X} ({})", val, meanings.join(", "))
    }
}

fn parse_coordinate_array(flags: &[u8], coords: &[u8], is_x: bool, base_offset: u32) -> Vec<Value> {
    let mut fields = Vec::new();
    let mut offset = 0;
    let mut i = 0;
    while i < flags.len() {
        let flag = flags[i];
        i += 1;
        let repeats = if (flag & 0x08) != 0 {
            if i < flags.len() {
                let r = flags[i];
                i += 1;
                r as u32 + 1
            } else {
                1
            }
        } else {
            1
        };

        let short_mask = if is_x { 0x02 } else { 0x04 };
        let same_mask = if is_x { 0x10 } else { 0x20 };

        let is_short = (flag & short_mask) != 0;
        let is_same = (flag & same_mask) != 0;

        for _ in 0..repeats {
            if is_short {
                if offset < coords.len() {
                    let mut val = coords[offset] as i16;
                    if !is_same {
                        val = -val;
                    }
                    fields.push(json!({
                        "type": "uint8",
                        "value": val,
                        "offset": base_offset + offset as u32,
                        "length": 1
                    }));
                    offset += 1;
                }
            } else if !is_same {
                if offset + 1 < coords.len() {
                    let val = i16::from_be_bytes([coords[offset], coords[offset + 1]]);
                    fields.push(json!({
                        "type": "int16",
                        "value": val,
                        "offset": base_offset + offset as u32,
                        "length": 2
                    }));
                    offset += 2;
                }
            }
        }
    }
    fields
}

fn parse_composite_components(data: &[u8], base_offset: u32) -> (Vec<Value>, u32, bool) {
    let mut fields = Vec::new();
    let mut offset = 0;
    let mut more = true;
    let mut has_instructions = false;

    while more && offset < data.len() {
        let start_offset = offset;

        if offset + 4 > data.len() {
            break;
        }
        let flags = u16::from_be_bytes([data[offset], data[offset + 1]]);
        let flags_val = json!({
            "type": "uint16",
            "value": flags,
            "summary": format_component_flag(flags),
            "offset": base_offset + offset as u32,
            "length": 2
        });
        offset += 2;

        let glyph_index = u16::from_be_bytes([data[offset], data[offset + 1]]);
        let glyph_index_val = json!({
            "type": "uint16",
            "value": glyph_index,
            "offset": base_offset + offset as u32,
            "length": 2
        });
        offset += 2;

        let args_are_words = (flags & 0x0001) != 0;
        let args_are_xy = (flags & 0x0002) != 0;

        let arg1_val;
        let arg2_val;

        if args_are_words {
            if offset + 4 > data.len() {
                break;
            }
            let arg1 = i16::from_be_bytes([data[offset], data[offset + 1]]);
            let arg2 = i16::from_be_bytes([data[offset + 2], data[offset + 3]]);
            let t = if args_are_xy { "int16" } else { "uint16" };
            arg1_val = json!({ "type": t, "value": arg1, "offset": base_offset + offset as u32, "length": 2 });
            arg2_val = json!({ "type": t, "value": arg2, "offset": base_offset + offset as u32 + 2, "length": 2 });
            offset += 4;
        } else {
            if offset + 2 > data.len() {
                break;
            }
            let arg1 = data[offset];
            let arg2 = data[offset + 1];
            let (t, v1, v2) = if args_are_xy {
                ("int8", json!(arg1 as i8), json!(arg2 as i8))
            } else {
                ("uint8", json!(arg1), json!(arg2))
            };
            arg1_val = json!({ "type": t, "value": v1, "offset": base_offset + offset as u32, "length": 1 });
            arg2_val = json!({ "type": t, "value": v2, "offset": base_offset + offset as u32 + 1, "length": 1 });
            offset += 2;
        }

        let mut comp_obj = serde_json::Map::new();
        comp_obj.insert("flags".to_string(), flags_val);
        comp_obj.insert("glyphIndex".to_string(), glyph_index_val);
        comp_obj.insert("argument1".to_string(), arg1_val);
        comp_obj.insert("argument2".to_string(), arg2_val);

        let mut transform_obj = serde_json::Map::new();
        let transform_offset = base_offset + offset as u32;
        let mut transform_len = 0;

        if (flags & 0x0008) != 0 {
            if offset + 2 > data.len() {
                break;
            }
            let scale = i16::from_be_bytes([data[offset], data[offset + 1]]);
            transform_obj.insert("scale".to_string(), json!({ "type": "F2DOT14", "value": scale as f32 / 16384.0, "offset": base_offset + offset as u32, "length": 2 }));
            offset += 2;
            transform_len = 2;
        } else if (flags & 0x0040) != 0 {
            if offset + 4 > data.len() {
                break;
            }
            let xscale = i16::from_be_bytes([data[offset], data[offset + 1]]);
            let yscale = i16::from_be_bytes([data[offset + 2], data[offset + 3]]);
            transform_obj.insert("xscale".to_string(), json!({ "type": "F2DOT14", "value": xscale as f32 / 16384.0, "offset": base_offset + offset as u32, "length": 2 }));
            transform_obj.insert("yscale".to_string(), json!({ "type": "F2DOT14", "value": yscale as f32 / 16384.0, "offset": base_offset + offset as u32 + 2, "length": 2 }));
            offset += 4;
            transform_len = 4;
        } else if (flags & 0x0080) != 0 {
            if offset + 8 > data.len() {
                break;
            }
            let xscale = i16::from_be_bytes([data[offset], data[offset + 1]]);
            let scale01 = i16::from_be_bytes([data[offset + 2], data[offset + 3]]);
            let scale10 = i16::from_be_bytes([data[offset + 4], data[offset + 5]]);
            let yscale = i16::from_be_bytes([data[offset + 6], data[offset + 7]]);
            transform_obj.insert("xscale".to_string(), json!({ "type": "F2DOT14", "value": xscale as f32 / 16384.0, "offset": base_offset + offset as u32, "length": 2 }));
            transform_obj.insert("scale01".to_string(), json!({ "type": "F2DOT14", "value": scale01 as f32 / 16384.0, "offset": base_offset + offset as u32 + 2, "length": 2 }));
            transform_obj.insert("scale10".to_string(), json!({ "type": "F2DOT14", "value": scale10 as f32 / 16384.0, "offset": base_offset + offset as u32 + 4, "length": 2 }));
            transform_obj.insert("yscale".to_string(), json!({ "type": "F2DOT14", "value": yscale as f32 / 16384.0, "offset": base_offset + offset as u32 + 6, "length": 2 }));
            offset += 8;
            transform_len = 8;
        }

        if !transform_obj.is_empty() {
            comp_obj.insert(
                "transform".to_string(),
                json!({
                    "type": "Transform",
                    "value": transform_obj,
                    "offset": transform_offset,
                    "length": transform_len
                }),
            );
        }

        let comp_len = offset - start_offset;
        fields.push(json!({
            "type": "ComponentGlyph",
            "value": comp_obj,
            "offset": base_offset + start_offset as u32,
            "length": comp_len as u32
        }));

        if (flags & 0x0100) != 0 {
            has_instructions = true;
        }

        more = (flags & 0x0020) != 0;
    }

    (fields, offset as u32, has_instructions)
}
