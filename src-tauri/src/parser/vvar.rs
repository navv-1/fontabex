use super::{
    reader::parsed_field,
    variations::{
        item_variation_store_length, parse_delta_set_index_map, parse_item_variation_store,
    },
};
use read_fonts::{tables::variations::DeltaSetIndexMap, FontRef, MinByteRange, TableProvider};
use serde_json::{Map, Value};

pub fn parse(font: &FontRef<'_>) -> Result<Value, String> {
    let table = font.vvar().map_err(|e| e.to_string())?;
    let mut fields = Map::new();

    fields.insert(
        "majorVersion".into(),
        parsed_field("uint16", table.version().major, 0, 2),
    );
    fields.insert(
        "minorVersion".into(),
        parsed_field("uint16", table.version().minor, 2, 2),
    );

    let item_variation_store_offset = table.item_variation_store_offset();
    let item_variation_store_table_offset = item_variation_store_offset.to_u32() as usize;
    fields.insert(
        "itemVariationStoreOffset".into(),
        parsed_field("Offset32", item_variation_store_offset.to_u32(), 4, 4),
    );

    fields.insert(
        "advanceHeightMappingOffset".into(),
        parsed_field(
            "Offset32",
            table.advance_height_mapping_offset().offset().to_u32(),
            8,
            4,
        ),
    );
    fields.insert(
        "tsbMappingOffset".into(),
        parsed_field(
            "Offset32",
            table.tsb_mapping_offset().offset().to_u32(),
            12,
            4,
        ),
    );
    fields.insert(
        "bsbMappingOffset".into(),
        parsed_field(
            "Offset32",
            table.bsb_mapping_offset().offset().to_u32(),
            16,
            4,
        ),
    );
    fields.insert(
        "vOrgMappingOffset".into(),
        parsed_field(
            "Offset32",
            table.v_org_mapping_offset().offset().to_u32(),
            20,
            4,
        ),
    );

    let item_variation_store = table.item_variation_store().map_err(|e| e.to_string())?;
    fields.insert(
        "itemVariationStore".into(),
        parsed_field(
            "ItemVariationStore",
            parse_item_variation_store(&item_variation_store, item_variation_store_table_offset)?,
            item_variation_store_table_offset,
            item_variation_store_length(&item_variation_store),
        ),
    );

    insert_index_map(
        &mut fields,
        "advanceHeightMapping",
        table.advance_height_mapping(),
        table.advance_height_mapping_offset().offset().to_u32() as usize,
    )?;
    insert_index_map(
        &mut fields,
        "tsbMapping",
        table.tsb_mapping(),
        table.tsb_mapping_offset().offset().to_u32() as usize,
    )?;
    insert_index_map(
        &mut fields,
        "bsbMapping",
        table.bsb_mapping(),
        table.bsb_mapping_offset().offset().to_u32() as usize,
    )?;
    insert_index_map(
        &mut fields,
        "vOrgMapping",
        table.v_org_mapping(),
        table.v_org_mapping_offset().offset().to_u32() as usize,
    )?;

    Ok(Value::Object(fields))
}

fn insert_index_map(
    fields: &mut Map<String, Value>,
    name: &'static str,
    index_map: Option<Result<DeltaSetIndexMap<'_>, read_fonts::ReadError>>,
    offset: usize,
) -> Result<(), String> {
    let Some(index_map) = index_map else {
        return Ok(());
    };
    let index_map = index_map.map_err(|e| e.to_string())?;
    fields.insert(
        name.into(),
        parsed_field(
            "DeltaSetIndexMap",
            parse_delta_set_index_map(&index_map, offset)?,
            offset,
            index_map.min_table_bytes().len(),
        ),
    );
    Ok(())
}
