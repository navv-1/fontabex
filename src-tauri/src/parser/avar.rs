use super::{
    reader::{parsed_field, read_u16_at, Reader},
    variations::{
        item_variation_store_length, parse_delta_set_index_map, parse_item_variation_store,
    },
};
use read_fonts::{
    tables::avar::{AxisValueMap, SegmentMaps},
    FontRef, MinByteRange, TableProvider,
};
use serde_json::{json, Map, Value};

const AVAR_HEADER_SIZE: usize = 8;
const AXIS_VALUE_MAP_SIZE: usize = 4;

pub fn parse(font: &FontRef<'_>) -> Result<Value, String> {
    let table = font.avar().map_err(|e| e.to_string())?;
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
        "reserved".into(),
        reader.read_as(read_u16_at(table.offset_data().as_bytes(), 4), 2, "uint16"),
    );
    fields.insert("axisCount".into(), reader.read(table.axis_count(), 2));

    let mut segment_maps = Vec::new();
    let mut offset = AVAR_HEADER_SIZE;
    for segment_map in table
        .axis_segment_maps()
        .iter()
        .take(table.axis_count() as usize)
    {
        let segment_map = segment_map.map_err(|e| e.to_string())?;
        let length = segment_map_length(&segment_map);
        segment_maps.push(parse_segment_map(&segment_map, offset));
        offset += length;
    }

    fields.insert(
        "axisSegmentMaps".into(),
        json!({
            "type": "SegmentMaps[]",
            "value": segment_maps,
            "offset": AVAR_HEADER_SIZE,
            "length": offset.saturating_sub(AVAR_HEADER_SIZE)
        }),
    );

    if let Some(axis_index_map_offset) = table.axis_index_map_offset() {
        let axis_index_map_table_offset = axis_index_map_offset.offset().to_u32() as usize;
        fields.insert(
            "axisIndexMapOffset".into(),
            parsed_field(
                "Offset32",
                axis_index_map_offset.offset().to_u32(),
                offset,
                4,
            ),
        );
        offset += 4;

        if let Some(axis_index_map) = table.axis_index_map() {
            let axis_index_map = axis_index_map.map_err(|e| e.to_string())?;
            fields.insert(
                "axisIndexMap".into(),
                parsed_field(
                    "DeltaSetIndexMap",
                    parse_delta_set_index_map(&axis_index_map, axis_index_map_table_offset)?,
                    axis_index_map_table_offset,
                    axis_index_map.min_table_bytes().len(),
                ),
            );
        }
    }

    if let Some(var_store_offset) = table.var_store_offset() {
        let var_store_table_offset = var_store_offset.offset().to_u32() as usize;
        fields.insert(
            "varStoreOffset".into(),
            parsed_field("Offset32", var_store_offset.offset().to_u32(), offset, 4),
        );

        if let Some(var_store) = table.var_store() {
            let var_store = var_store.map_err(|e| e.to_string())?;
            fields.insert(
                "varStore".into(),
                parsed_field(
                    "ItemVariationStore",
                    parse_item_variation_store(&var_store, var_store_table_offset)?,
                    var_store_table_offset,
                    item_variation_store_length(&var_store),
                ),
            );
        }
    }

    Ok(Value::Object(fields))
}

fn parse_segment_map(segment_map: &SegmentMaps<'_>, offset: usize) -> Value {
    let maps_offset = offset + 2;
    json!({
        "positionMapCount": parsed_field("uint16", segment_map.position_map_count(), offset, 2),
        "axisValueMaps": {
            "type": "AxisValueMap[]",
            "value": segment_map
                .axis_value_maps()
                .iter()
                .enumerate()
                .map(|(index, value_map)| {
                    parse_axis_value_map(
                        value_map,
                        maps_offset + index * AXIS_VALUE_MAP_SIZE,
                    )
                })
                .collect::<Vec<_>>(),
            "offset": maps_offset,
            "length": segment_map.axis_value_maps().len() * AXIS_VALUE_MAP_SIZE
        }
    })
}

fn parse_axis_value_map(value_map: &AxisValueMap, offset: usize) -> Value {
    json!({
        "fromCoordinate": parsed_field("F2DOT14", value_map.from_coordinate().to_f64(), offset, 2),
        "toCoordinate": parsed_field("F2DOT14", value_map.to_coordinate().to_f64(), offset + 2, 2)
    })
}

fn segment_map_length(segment_map: &SegmentMaps<'_>) -> usize {
    2 + segment_map.axis_value_maps().len() * AXIS_VALUE_MAP_SIZE
}
