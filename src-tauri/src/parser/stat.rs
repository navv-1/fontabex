use super::{
    name::name_id_label,
    reader::{parsed_field, Reader},
};
use read_fonts::{
    tables::stat::{
        AxisRecord, AxisValue, AxisValueFormat1, AxisValueFormat2, AxisValueFormat3,
        AxisValueFormat4, AxisValueRecord, AxisValueTableFlags,
    },
    types::NameId,
    FontRef, MinByteRange, TableProvider,
};
use serde_json::{json, Map, Value};

const AXIS_VALUE_FORMAT_4_BASE_SIZE: usize = 8;
const AXIS_VALUE_RECORD_SIZE: usize = 6;

pub fn parse(font: &FontRef<'_>) -> Result<Value, String> {
    let table = font.stat().map_err(|e| e.to_string())?;
    let mut reader = Reader::new();
    let mut fields = Map::new();

    fields.insert(
        "majorVersion".into(),
        reader.read_as(table.version().major, 2, "uint16"),
    );
    fields.insert(
        "minorVersion".into(),
        reader.read_as(table.version().minor, 2, "uint16"),
    );
    fields.insert(
        "designAxisSize".into(),
        reader.read(table.design_axis_size(), 2),
    );
    fields.insert(
        "designAxisCount".into(),
        reader.read(table.design_axis_count(), 2),
    );
    fields.insert(
        "designAxesOffset".into(),
        reader.read_as(table.design_axes_offset().to_u32(), 4, "Offset32"),
    );
    fields.insert(
        "axisValueCount".into(),
        reader.read(table.axis_value_count(), 2),
    );
    fields.insert(
        "offsetToAxisValueOffsets".into(),
        reader.read_as(
            table.offset_to_axis_value_offsets().offset().to_u32(),
            4,
            "Offset32",
        ),
    );

    if let Some(elided_fallback_name_id) = table.elided_fallback_name_id() {
        let mut field = reader.read_as(elided_fallback_name_id.to_u16(), 2, "uint16");
        if let Some(summary) = name_id_label(elided_fallback_name_id) {
            field["summary"] = json!(summary);
        }
        fields.insert("elidedFallbackNameID".into(), field);
    }

    let design_axes_offset = table.design_axes_offset().to_u32() as usize;
    let design_axes = table.design_axes().map_err(|e| e.to_string())?;
    fields.insert(
        "designAxes".into(),
        json!({
            "type": "AxisRecord[]",
            "value": design_axes
                .iter()
                .enumerate()
                .map(|(index, axis)| {
                    parse_axis_record(
                        axis,
                        design_axes_offset + index * table.design_axis_size() as usize,
                    )
                })
                .collect::<Vec<_>>(),
            "offset": design_axes_offset,
            "length": design_axes.len() * table.design_axis_size() as usize
        }),
    );

    if let Some(axis_values) = table.offset_to_axis_values() {
        let axis_values = axis_values.map_err(|e| e.to_string())?;
        let offsets_array_offset = table.offset_to_axis_value_offsets().offset().to_u32() as usize;
        let offsets = axis_values.axis_value_offsets();
        let mut parsed_axis_values = Vec::new();
        let mut axis_values_start: Option<usize> = None;
        let mut axis_values_end = offsets_array_offset;

        for (index, value) in axis_values.axis_values().iter().enumerate() {
            let value = value.map_err(|e| e.to_string())?;
            let offset = offsets_array_offset + offsets[index].get().to_u32() as usize;
            axis_values_start = Some(axis_values_start.map_or(offset, |start| start.min(offset)));
            axis_values_end = axis_values_end.max(offset + axis_value_length(&value));
            parsed_axis_values.push(parse_axis_value(&value, offset));
        }

        fields.insert(
            "axisValueOffsets".into(),
            json!({
                "type": "Offset16[]",
                "value": offsets
                    .iter()
                    .enumerate()
                    .map(|(index, offset)| {
                        parsed_field(
                            "Offset16",
                            offset.get().to_u32(),
                            offsets_array_offset + index * 2,
                            2,
                        )
                    })
                    .collect::<Vec<_>>(),
                "offset": offsets_array_offset,
                "length": offsets.len() * 2
            }),
        );

        fields.insert(
            "axisValues".into(),
            json!({
                "type": "AxisValue[]",
                "value": parsed_axis_values,
                "offset": axis_values_start.unwrap_or(offsets_array_offset),
                "length": axis_values_start
                    .map(|start| axis_values_end.saturating_sub(start))
                    .unwrap_or_default()
            }),
        );
    }

    Ok(Value::Object(fields))
}

fn parse_axis_record(axis: &AxisRecord, offset: usize) -> Value {
    json!({
        "axisTag": parsed_field("Tag", axis.axis_tag().to_string(), offset, 4),
        "axisNameID": name_id_field(axis.axis_name_id(), offset + 4, 2),
        "axisOrdering": parsed_field("uint16", axis.axis_ordering(), offset + 6, 2)
    })
}

fn parse_axis_value(axis_value: &AxisValue<'_>, offset: usize) -> Value {
    match axis_value {
        AxisValue::Format1(table) => parse_axis_value_format1(table, offset),
        AxisValue::Format2(table) => parse_axis_value_format2(table, offset),
        AxisValue::Format3(table) => parse_axis_value_format3(table, offset),
        AxisValue::Format4(table) => parse_axis_value_format4(table, offset),
    }
}

fn parse_axis_value_format1(table: &AxisValueFormat1<'_>, offset: usize) -> Value {
    json!({
        "format": parsed_field("uint16", table.format(), offset, 2),
        "axisIndex": parsed_field("uint16", table.axis_index(), offset + 2, 2),
        "flags": flags_field(table.flags(), offset + 4),
        "valueNameID": name_id_field(table.value_name_id(), offset + 6, 2),
        "value": parsed_field("Fixed", table.value().to_f64(), offset + 8, 4)
    })
}

fn parse_axis_value_format2(table: &AxisValueFormat2<'_>, offset: usize) -> Value {
    json!({
        "format": parsed_field("uint16", table.format(), offset, 2),
        "axisIndex": parsed_field("uint16", table.axis_index(), offset + 2, 2),
        "flags": flags_field(table.flags(), offset + 4),
        "valueNameID": name_id_field(table.value_name_id(), offset + 6, 2),
        "nominalValue": parsed_field("Fixed", table.nominal_value().to_f64(), offset + 8, 4),
        "rangeMinValue": parsed_field("Fixed", table.range_min_value().to_f64(), offset + 12, 4),
        "rangeMaxValue": parsed_field("Fixed", table.range_max_value().to_f64(), offset + 16, 4)
    })
}

fn parse_axis_value_format3(table: &AxisValueFormat3<'_>, offset: usize) -> Value {
    json!({
        "format": parsed_field("uint16", table.format(), offset, 2),
        "axisIndex": parsed_field("uint16", table.axis_index(), offset + 2, 2),
        "flags": flags_field(table.flags(), offset + 4),
        "valueNameID": name_id_field(table.value_name_id(), offset + 6, 2),
        "value": parsed_field("Fixed", table.value().to_f64(), offset + 8, 4),
        "linkedValue": parsed_field("Fixed", table.linked_value().to_f64(), offset + 12, 4)
    })
}

fn parse_axis_value_format4(table: &AxisValueFormat4<'_>, offset: usize) -> Value {
    let axis_values_offset = offset + AXIS_VALUE_FORMAT_4_BASE_SIZE;
    json!({
        "format": parsed_field("uint16", table.format(), offset, 2),
        "axisCount": parsed_field("uint16", table.axis_count(), offset + 2, 2),
        "flags": flags_field(table.flags(), offset + 4),
        "valueNameID": name_id_field(table.value_name_id(), offset + 6, 2),
        "axisValues": {
            "type": "AxisValueRecord[]",
            "value": table
                .axis_values()
                .iter()
                .enumerate()
                .map(|(index, record)| {
                    parse_axis_value_record(
                        record,
                        axis_values_offset + index * AXIS_VALUE_RECORD_SIZE,
                    )
                })
                .collect::<Vec<_>>(),
            "offset": axis_values_offset,
            "length": table.axis_values().len() * AXIS_VALUE_RECORD_SIZE
        }
    })
}

fn parse_axis_value_record(record: &AxisValueRecord, offset: usize) -> Value {
    json!({
        "axisIndex": parsed_field("uint16", record.axis_index(), offset, 2),
        "value": parsed_field("Fixed", record.value().to_f64(), offset + 2, 4)
    })
}

fn flags_field(flags: AxisValueTableFlags, offset: usize) -> Value {
    let bits = flags.bits();
    let mut field = parsed_field("uint16", bits, offset, 2);
    let mut names = Vec::new();

    if flags.contains(AxisValueTableFlags::OLDER_SIBLING_FONT_ATTRIBUTE) {
        names.push("Older sibling font attribute");
    }
    if flags.contains(AxisValueTableFlags::ELIDABLE_AXIS_VALUE_NAME) {
        names.push("Elidable axis value name");
    }

    if !names.is_empty() {
        field["summary"] = json!(format!("0x{bits:04X} ({})", names.join(", ")));
    }

    field
}

fn axis_value_length(axis_value: &AxisValue<'_>) -> usize {
    match axis_value {
        AxisValue::Format1(table) => table.min_table_bytes().len(),
        AxisValue::Format2(table) => table.min_table_bytes().len(),
        AxisValue::Format3(table) => table.min_table_bytes().len(),
        AxisValue::Format4(table) => table.min_table_bytes().len(),
    }
}

fn name_id_field(name_id: NameId, offset: usize, length: usize) -> Value {
    let mut field = parsed_field("uint16", name_id.to_u16(), offset, length);
    if let Some(summary) = name_id_label(name_id) {
        field["summary"] = json!(summary);
    }
    field
}
