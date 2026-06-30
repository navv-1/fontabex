use super::{
    reader::{parsed_field, read_u16_at},
    variations::{item_variation_store_length, parse_item_variation_store},
};
use read_fonts::{tables::mvar::ValueRecord, FontRef, TableProvider};
use serde_json::{json, Map, Value};

const MVAR_HEADER_SIZE: usize = 12;
const VALUE_RECORD_SIZE: usize = 8;

pub fn parse(font: &FontRef<'_>) -> Result<Value, String> {
    let table = font.mvar().map_err(|e| e.to_string())?;
    let mut fields = Map::new();

    fields.insert(
        "majorVersion".into(),
        parsed_field("uint16", table.version().major, 0, 2),
    );
    fields.insert(
        "minorVersion".into(),
        parsed_field("uint16", table.version().minor, 2, 2),
    );
    fields.insert(
        "reserved".into(),
        parsed_field(
            "uint16",
            read_u16_at(table.offset_data().as_bytes(), 4),
            4,
            2,
        ),
    );
    fields.insert(
        "valueRecordSize".into(),
        parsed_field("uint16", table.value_record_size(), 6, 2),
    );
    fields.insert(
        "valueRecordCount".into(),
        parsed_field("uint16", table.value_record_count(), 8, 2),
    );

    let item_variation_store_offset = table.item_variation_store_offset();
    fields.insert(
        "itemVariationStoreOffset".into(),
        parsed_field(
            "Offset16",
            item_variation_store_offset.offset().to_u32(),
            10,
            2,
        ),
    );

    let value_record_size = table.value_record_size() as usize;
    fields.insert(
        "valueRecords".into(),
        json!({
            "type": "ValueRecord[]",
            "value": table
                .value_records()
                .iter()
                .enumerate()
                .map(|(index, record)| {
                    parse_value_record(
                        record,
                        MVAR_HEADER_SIZE + index * value_record_size.max(VALUE_RECORD_SIZE),
                    )
                })
                .collect::<Vec<_>>(),
            "offset": MVAR_HEADER_SIZE,
            "length": table.value_record_count() as usize * value_record_size
        }),
    );

    if !item_variation_store_offset.is_null() {
        let store_offset = item_variation_store_offset.offset().to_u32() as usize;
        if let Some(store) = table.item_variation_store() {
            let store = store.map_err(|e| e.to_string())?;
            fields.insert(
                "itemVariationStore".into(),
                parsed_field(
                    "ItemVariationStore",
                    parse_item_variation_store(&store, store_offset)?,
                    store_offset,
                    item_variation_store_length(&store),
                ),
            );
        }
    }

    Ok(Value::Object(fields))
}

fn parse_value_record(record: &ValueRecord, offset: usize) -> Value {
    json!({
        "valueTag": parsed_field("Tag", record.value_tag().to_string(), offset, 4),
        "deltaSetOuterIndex": parsed_field(
            "uint16",
            record.delta_set_outer_index(),
            offset + 4,
            2
        ),
        "deltaSetInnerIndex": parsed_field(
            "uint16",
            record.delta_set_inner_index(),
            offset + 6,
            2
        )
    })
}
