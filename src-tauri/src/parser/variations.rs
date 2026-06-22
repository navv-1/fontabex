use read_fonts::{
    tables::variations::{
        DeltaSetIndexMap, DeltaSetIndexMapFormat0, DeltaSetIndexMapFormat1, ItemVariationData,
        ItemVariationStore, RegionAxisCoordinates, VariationRegion, VariationRegionList,
    },
    MinByteRange,
};
use serde_json::{json, Value};

const DELTA_SET_INDEX_MAP_FORMAT_0_HEADER_SIZE: usize = 4;
const DELTA_SET_INDEX_MAP_FORMAT_1_HEADER_SIZE: usize = 6;
const ITEM_VARIATION_STORE_HEADER_SIZE: usize = 8;
const ITEM_VARIATION_DATA_HEADER_SIZE: usize = 6;
const REGION_AXIS_COORDINATES_SIZE: usize = 6;

pub fn parse_delta_set_index_map(
    index_map: &DeltaSetIndexMap<'_>,
    offset: usize,
) -> Result<Value, String> {
    match index_map {
        DeltaSetIndexMap::Format0(table) => parse_delta_set_index_map_format0(table, offset),
        DeltaSetIndexMap::Format1(table) => parse_delta_set_index_map_format1(table, offset),
    }
}

pub fn parse_item_variation_store(
    store: &ItemVariationStore<'_>,
    offset: usize,
) -> Result<Value, String> {
    let region_list_offset = offset + store.variation_region_list_offset().to_u32() as usize;
    let region_list = store.variation_region_list().map_err(|e| e.to_string())?;
    let data_offsets = store.item_variation_data_offsets();
    let item_variation_data = store.item_variation_data();

    Ok(json!({
        "format": parsed_field("uint16", store.format(), offset, 2),
        "variationRegionListOffset": parsed_field(
            "Offset32",
            store.variation_region_list_offset().to_u32(),
            offset + 2,
            4,
        ),
        "itemVariationDataCount": parsed_field("uint16", store.item_variation_data_count(), offset + 6, 2),
        "itemVariationDataOffsets": {
            "type": "Offset32[]",
            "value": data_offsets
                .iter()
                .enumerate()
                .map(|(index, data_offset)| {
                    parsed_field(
                        "Offset32",
                        data_offset.get().offset().to_u32(),
                        offset + ITEM_VARIATION_STORE_HEADER_SIZE + index * 4,
                        4,
                    )
                })
                .collect::<Vec<_>>(),
            "offset": offset + ITEM_VARIATION_STORE_HEADER_SIZE,
            "length": data_offsets.len() * 4
        },
        "variationRegionList": parsed_field(
            "VariationRegionList",
            parse_variation_region_list(&region_list, region_list_offset),
            region_list_offset,
            region_list.min_table_bytes().len(),
        ),
        "itemVariationData": {
            "type": "ItemVariationData[]",
            "value": data_offsets
                .iter()
                .enumerate()
                .map(|(index, data_offset)| {
                    let data_offset_value = data_offset.get();
                    if data_offset_value.is_null() {
                        return Ok(Value::Null);
                    }
                    let Some(data) = item_variation_data.get(index) else {
                        return Ok(Value::Null);
                    };
                    let data = data.map_err(|e| e.to_string())?;
                    let data_table_offset = offset + data_offset_value.offset().to_u32() as usize;
                    Ok(parse_item_variation_data(&data, data_table_offset))
                })
                .collect::<Result<Vec<_>, String>>()?,
            "offset": offset,
            "length": item_variation_store_length(store)
        }
    }))
}

pub fn item_variation_store_length(store: &ItemVariationStore<'_>) -> usize {
    let mut length = store.min_table_bytes().len();

    if let Ok(region_list) = store.variation_region_list() {
        length = length.max(
            store.variation_region_list_offset().to_u32() as usize
                + region_list.min_table_bytes().len(),
        );
    }

    let item_variation_data = store.item_variation_data();
    for (index, data_offset) in store.item_variation_data_offsets().iter().enumerate() {
        let data_offset = data_offset.get();
        if data_offset.is_null() {
            continue;
        }
        if let Some(Ok(data)) = item_variation_data.get(index) {
            length =
                length.max(data_offset.offset().to_u32() as usize + data.min_table_bytes().len());
        }
    }

    length
}

fn parse_delta_set_index_map_format0(
    table: &DeltaSetIndexMapFormat0<'_>,
    offset: usize,
) -> Result<Value, String> {
    let entry_format = table.entry_format();
    Ok(json!({
        "format": parsed_field("uint8", table.format(), offset, 1),
        "entryFormat": entry_format_field(entry_format, offset + 1),
        "mapCount": parsed_field("uint16", table.map_count(), offset + 2, 2),
        "mapData": parse_delta_set_index_map_data(
            entry_format,
            table.map_data(),
            offset + DELTA_SET_INDEX_MAP_FORMAT_0_HEADER_SIZE,
        )?
    }))
}

fn parse_delta_set_index_map_format1(
    table: &DeltaSetIndexMapFormat1<'_>,
    offset: usize,
) -> Result<Value, String> {
    let entry_format = table.entry_format();
    Ok(json!({
        "format": parsed_field("uint8", table.format(), offset, 1),
        "entryFormat": entry_format_field(entry_format, offset + 1),
        "mapCount": parsed_field("uint32", table.map_count(), offset + 2, 4),
        "mapData": parse_delta_set_index_map_data(
            entry_format,
            table.map_data(),
            offset + DELTA_SET_INDEX_MAP_FORMAT_1_HEADER_SIZE,
        )?
    }))
}

fn parse_delta_set_index_map_data(
    entry_format: read_fonts::tables::variations::EntryFormat,
    data: &[u8],
    offset: usize,
) -> Result<Value, String> {
    let entry_size = entry_format.entry_size() as usize;
    let bit_count = entry_format.bit_count();
    let entries = data
        .chunks_exact(entry_size)
        .enumerate()
        .map(|(index, bytes)| {
            let entry = read_uint_be(bytes)?;
            let inner_mask = (1u32 << bit_count) - 1;
            Ok(json!({
                "outerIndex": parsed_field("uint16", (entry >> bit_count) as u16, offset + index * entry_size, entry_size),
                "innerIndex": parsed_field("uint16", (entry & inner_mask) as u16, offset + index * entry_size, entry_size)
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(json!({
        "type": "DeltaSetIndex[]",
        "value": entries,
        "offset": offset,
        "length": data.len()
    }))
}

fn parse_variation_region_list(region_list: &VariationRegionList<'_>, offset: usize) -> Value {
    let regions_offset = offset + 4;
    json!({
        "axisCount": parsed_field("uint16", region_list.axis_count(), offset, 2),
        "regionCount": parsed_field("uint16", region_list.region_count(), offset + 2, 2),
        "variationRegions": {
            "type": "VariationRegion[]",
            "value": region_list
                .variation_regions()
                .iter()
                .enumerate()
                .filter_map(|(index, region)| {
                    region.ok().map(|region| {
                        parse_variation_region(
                            &region,
                            regions_offset
                                + index
                                    * region_list.axis_count() as usize
                                    * REGION_AXIS_COORDINATES_SIZE,
                        )
                    })
                })
                .collect::<Vec<_>>(),
            "offset": regions_offset,
            "length": region_list
                .region_count() as usize
                * region_list.axis_count() as usize
                * REGION_AXIS_COORDINATES_SIZE
        }
    })
}

fn parse_variation_region(region: &VariationRegion<'_>, offset: usize) -> Value {
    json!({
        "regionAxes": {
            "type": "RegionAxisCoordinates[]",
            "value": region
                .region_axes()
                .iter()
                .enumerate()
                .map(|(index, axis)| {
                    parse_region_axis_coordinates(
                        axis,
                        offset + index * REGION_AXIS_COORDINATES_SIZE,
                    )
                })
                .collect::<Vec<_>>(),
            "offset": offset,
            "length": region.region_axes().len() * REGION_AXIS_COORDINATES_SIZE
        }
    })
}

fn parse_region_axis_coordinates(axis: &RegionAxisCoordinates, offset: usize) -> Value {
    json!({
        "startCoord": parsed_field("F2DOT14", axis.start_coord().to_f64(), offset, 2),
        "peakCoord": parsed_field("F2DOT14", axis.peak_coord().to_f64(), offset + 2, 2),
        "endCoord": parsed_field("F2DOT14", axis.end_coord().to_f64(), offset + 4, 2)
    })
}

fn parse_item_variation_data(data: &ItemVariationData<'_>, offset: usize) -> Value {
    let region_indexes_offset = offset + ITEM_VARIATION_DATA_HEADER_SIZE;
    let delta_sets_offset = region_indexes_offset + data.region_indexes().len() * 2;
    json!({
        "itemCount": parsed_field("uint16", data.item_count(), offset, 2),
        "wordDeltaCount": parsed_field("uint16", data.word_delta_count(), offset + 2, 2),
        "regionIndexCount": parsed_field("uint16", data.region_index_count(), offset + 4, 2),
        "regionIndexes": {
            "type": "uint16[]",
            "value": data
                .region_indexes()
                .iter()
                .enumerate()
                .map(|(index, region_index)| {
                    parsed_field("uint16", region_index.get(), region_indexes_offset + index * 2, 2)
                })
                .collect::<Vec<_>>(),
            "offset": region_indexes_offset,
            "length": data.region_indexes().len() * 2
        },
        "deltaSets": {
            "type": "int32[]",
            "value": (0..data.item_count())
                .map(|index| {
                    json!({
                        "type": "int32[]",
                        "name": format!("Delta set {index}"),
                        "value": data.delta_set(index).collect::<Vec<_>>(),
                        "offset": delta_sets_offset + index as usize * data.get_delta_row_len(),
                        "length": data.get_delta_row_len()
                    })
                })
                .collect::<Vec<_>>(),
            "offset": delta_sets_offset,
            "length": data.delta_sets().len()
        }
    })
}

fn entry_format_field(
    entry_format: read_fonts::tables::variations::EntryFormat,
    offset: usize,
) -> Value {
    let bits = entry_format.bits();
    let mut field = parsed_field("uint8", bits, offset, 1);
    field["summary"] = json!(format!(
        "{:#02X} (entry size: {} bytes, inner index bits: {})",
        bits,
        entry_format.entry_size(),
        entry_format.bit_count()
    ));
    field
}

fn read_uint_be(bytes: &[u8]) -> Result<u32, String> {
    match bytes {
        [a] => Ok(u32::from(*a)),
        [a, b] => Ok(u32::from_be_bytes([0, 0, *a, *b])),
        [a, b, c] => Ok(u32::from_be_bytes([0, *a, *b, *c])),
        [a, b, c, d] => Ok(u32::from_be_bytes([*a, *b, *c, *d])),
        _ => Err(format!(
            "unsupported DeltaSetIndexMap entry size {}",
            bytes.len()
        )),
    }
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
