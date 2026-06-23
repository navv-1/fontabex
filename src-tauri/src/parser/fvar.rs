use super::{
    name::name_id_label,
    reader::{parsed_field, read_u16_at, Reader},
};
use read_fonts::{
    tables::fvar::{InstanceRecord, VariationAxisRecord},
    types::NameId,
    FontRef, TableProvider,
};
use serde_json::{json, Map, Value};

const FVAR_HEADER_SIZE: usize = 16;

pub fn parse(font: &FontRef<'_>) -> Result<Value, String> {
    let table = font.fvar().map_err(|e| e.to_string())?;
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
        "axesArrayOffset".into(),
        reader.read_as(table.axis_instance_arrays_offset().to_u32(), 2, "Offset16"),
    );
    fields.insert(
        "reserved".into(),
        reader.read_as(read_u16_at(table.offset_data().as_bytes(), 6), 2, "uint16"),
    );
    fields.insert("axisCount".into(), reader.read(table.axis_count(), 2));
    fields.insert("axisSize".into(), reader.read(table.axis_size(), 2));
    fields.insert(
        "instanceCount".into(),
        reader.read(table.instance_count(), 2),
    );
    fields.insert("instanceSize".into(), reader.read(table.instance_size(), 2));

    let axes_start = table.axis_instance_arrays_offset().to_u32() as usize;
    let axes = table.axes().map_err(|e| e.to_string())?;
    fields.insert(
        "axes".into(),
        json!({
            "type": "VariationAxisRecord[]",
            "value": axes
                .iter()
                .enumerate()
                .map(|(index, axis)| parse_axis(axis, axes_start + index * table.axis_size() as usize))
                .collect::<Vec<_>>(),
            "offset": axes_start,
            "length": axes.len() * table.axis_size() as usize
        }),
    );

    let instances_start = axes_start + table.axis_count() as usize * table.axis_size() as usize;
    let instances = table.instances().map_err(|e| e.to_string())?;
    let mut parsed_instances = Vec::new();
    for index in 0..table.instance_count() as usize {
        let instance = instances.get(index).map_err(|e| e.to_string())?;
        parsed_instances.push(parse_instance(
            &instance,
            axes,
            instances_start + index * table.instance_size() as usize,
            table.instance_size() as usize,
        ));
    }

    fields.insert(
        "instances".into(),
        json!({
            "type": "InstanceRecord[]",
            "value": parsed_instances,
            "offset": instances_start,
            "length": table.instance_count() as usize * table.instance_size() as usize
        }),
    );

    if reader.current_offset() < FVAR_HEADER_SIZE {
        fields.insert(
            "unreadHeaderBytes".into(),
            reader.read_as(
                "",
                FVAR_HEADER_SIZE - reader.current_offset(),
                "ReservedBytes",
            ),
        );
    }

    Ok(Value::Object(fields))
}

fn parse_axis(axis: &VariationAxisRecord, offset: usize) -> Value {
    let mut fields = Map::new();

    fields.insert(
        "axisTag".into(),
        parsed_field("Tag", axis.axis_tag().to_string(), offset, 4),
    );
    fields.insert(
        "minValue".into(),
        parsed_field("Fixed", axis.min_value().to_f64(), offset + 4, 4),
    );
    fields.insert(
        "defaultValue".into(),
        parsed_field("Fixed", axis.default_value().to_f64(), offset + 8, 4),
    );
    fields.insert(
        "maxValue".into(),
        parsed_field("Fixed", axis.max_value().to_f64(), offset + 12, 4),
    );

    let mut flags = parsed_field("uint16", axis.flags(), offset + 16, 2);
    if axis.flags() & 0x0001 != 0 {
        flags["summary"] = json!("Hidden axis");
    }
    fields.insert("flags".into(), flags);

    fields.insert(
        "axisNameID".into(),
        name_id_field(axis.axis_name_id(), offset + 18, 2),
    );

    Value::Object(fields)
}

fn parse_instance(
    instance: &InstanceRecord<'_>,
    axes: &[VariationAxisRecord],
    offset: usize,
    instance_size: usize,
) -> Value {
    let mut fields = Map::new();

    fields.insert(
        "subfamilyNameID".into(),
        name_id_field(instance.subfamily_name_id, offset, 2),
    );
    fields.insert(
        "flags".into(),
        parsed_field("uint16", instance.flags, offset + 2, 2),
    );

    let coordinates_offset = offset + 4;
    let coordinates = instance
        .coordinates
        .iter()
        .enumerate()
        .map(|(index, coordinate)| {
            let axis_tag = axes
                .get(index)
                .map(|axis| axis.axis_tag().to_string())
                .unwrap_or_else(|| format!("Axis {index}"));
            let value = coordinate.get().to_f64();

            json!({
                "type": "Fixed",
                "name": axis_tag,
                "value": value,
                "offset": coordinates_offset + index * 4,
                "length": 4
            })
        })
        .collect::<Vec<_>>();

    fields.insert(
        "coordinates".into(),
        json!({
            "type": "Fixed[]",
            "value": coordinates,
            "offset": coordinates_offset,
            "length": instance.coordinates.len() * 4
        }),
    );

    let post_script_name_id_offset = coordinates_offset + instance.coordinates.len() * 4;
    if instance_size >= post_script_name_id_offset - offset + 2 {
        fields.insert(
            "postScriptNameID".into(),
            parsed_field(
                "uint16",
                instance
                    .post_script_name_id
                    .map(|name_id| json!(name_id.to_u16()))
                    .unwrap_or(Value::Null),
                post_script_name_id_offset,
                2,
            ),
        );
    }

    Value::Object(fields)
}

fn name_id_field(name_id: NameId, offset: usize, length: usize) -> Value {
    let mut field = parsed_field("uint16", name_id.to_u16(), offset, length);
    if let Some(summary) = name_id_label(name_id) {
        field["summary"] = json!(summary);
    }
    field
}
