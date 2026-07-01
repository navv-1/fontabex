use super::reader::parsed_field;
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

fn get_variation_region_list_length(region_list: &VariationRegionList<'_>) -> usize {
    4 + region_list.region_count() as usize
        * region_list.axis_count() as usize
        * REGION_AXIS_COORDINATES_SIZE
}

fn get_item_variation_data_length(data: &ItemVariationData<'_>) -> usize {
    ITEM_VARIATION_DATA_HEADER_SIZE
        + data.region_index_count() as usize * 2
        + data.item_count() as usize * data.get_delta_row_len()
}

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

    let mut data_start_offset = usize::MAX;
    let mut data_end_offset = 0;

    let parsed_item_variation_data = data_offsets
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

            data_start_offset = data_start_offset.min(data_table_offset);
            data_end_offset =
                data_end_offset.max(data_table_offset + get_item_variation_data_length(&data));

            Ok(parse_item_variation_data(&data, data_table_offset))
        })
        .collect::<Result<Vec<_>, String>>()?;

    if data_start_offset == usize::MAX {
        data_start_offset = offset + ITEM_VARIATION_STORE_HEADER_SIZE + data_offsets.len() * 4;
        data_end_offset = data_start_offset;
    }

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
            get_variation_region_list_length(&region_list),
        ),
        "itemVariationData": {
            "type": "ItemVariationData[]",
            "value": parsed_item_variation_data,
            "offset": data_start_offset,
            "length": data_end_offset - data_start_offset
        }
    }))
}

pub fn item_variation_store_length(store: &ItemVariationStore<'_>) -> usize {
    let mut length = store.min_table_bytes().len();

    if let Ok(region_list) = store.variation_region_list() {
        length = length.max(
            store.variation_region_list_offset().to_u32() as usize
                + get_variation_region_list_length(&region_list),
        );
    }

    let item_variation_data = store.item_variation_data();
    for (index, data_offset) in store.item_variation_data_offsets().iter().enumerate() {
        let data_offset = data_offset.get();
        if data_offset.is_null() {
            continue;
        }
        if let Some(Ok(data)) = item_variation_data.get(index) {
            length = length.max(
                data_offset.offset().to_u32() as usize + get_item_variation_data_length(&data),
            );
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
            table.map_data(),
            offset + DELTA_SET_INDEX_MAP_FORMAT_0_HEADER_SIZE,
        )
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
            table.map_data(),
            offset + DELTA_SET_INDEX_MAP_FORMAT_1_HEADER_SIZE,
        )
    }))
}

fn parse_delta_set_index_map_data(data: &[u8], offset: usize) -> Value {
    let entries = data
        .iter()
        .enumerate()
        .map(|(index, byte)| parsed_field("uint8", *byte, offset + index, 1))
        .collect::<Vec<_>>();

    json!({
        "type": "uint8[]",
        "value": entries,
        "offset": offset,
        "length": data.len()
    })
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
    let word_delta_count = data.word_delta_count();
    let long_words = (word_delta_count & 0x8000) != 0;
    let count = word_delta_count & 0x7FFF;
    let mut word_delta_field = parsed_field("uint16", word_delta_count, offset + 2, 2);
    word_delta_field["summary"] = json!(format!(
        "{:#06X} (LONG_WORDS: {}, count: {})",
        word_delta_count, long_words, count
    ));

    json!({
        "itemCount": parsed_field("uint16", data.item_count(), offset, 2),
        "wordDeltaCount": word_delta_field,
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
            "type": "DeltaSet[]",
            "value": (0..data.item_count())
                .map(|index| {
                    let row_len = data.get_delta_row_len();
                    let row_offset = delta_sets_offset + index as usize * row_len;
                    let delta_type = if long_words {
                        "(int32 | int16)[]"
                    } else {
                        "(int16 | int8)[]"
                    };

                    let mut current_item_offset = row_offset;
                    let parsed_deltas: Vec<Value> = data
                        .delta_set(index)
                        .enumerate()
                        .map(|(i, delta)| {
                            let is_long = i < count as usize;
                            let (item_type, item_len) = if long_words {
                                if is_long { ("int32", 4) } else { ("int16", 2) }
                            } else {
                                if is_long { ("int16", 2) } else { ("int8", 1) }
                            };

                            let field = parsed_field(item_type, delta, current_item_offset, item_len);
                            current_item_offset += item_len;
                            field
                        })
                        .collect();

                    json!({
                        "type": "DeltaSet",
                        "name": format!("Delta set {index}"),
                        "value": {
                            "deltaData": {
                                "type": delta_type,
                                "value": parsed_deltas,
                                "offset": row_offset,
                                "length": row_len
                            }
                        },
                        "offset": row_offset,
                        "length": row_len
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
