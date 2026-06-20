use super::reader::Reader;
use read_fonts::{FontRef, TableProvider};
use serde_json::{Map, Value};

pub fn parse(font: &FontRef<'_>) -> Result<Value, String> {
    let table = font.maxp().map_err(|e| e.to_string())?;
    let mut t = Reader::new();
    let mut fields = Map::new();

    fields.insert(
        "version".into(),
        t.read_as(table.version().to_string(), 4, "Version16Dot16"),
    );
    fields.insert("numGlyphs".into(), t.read(table.num_glyphs(), 2));
    insert_optional(&mut fields, &mut t, "maxPoints", table.max_points());
    insert_optional(&mut fields, &mut t, "maxContours", table.max_contours());
    insert_optional(
        &mut fields,
        &mut t,
        "maxCompositePoints",
        table.max_composite_points(),
    );
    insert_optional(
        &mut fields,
        &mut t,
        "maxCompositeContours",
        table.max_composite_contours(),
    );
    insert_optional(&mut fields, &mut t, "maxZones", table.max_zones());
    insert_optional(
        &mut fields,
        &mut t,
        "maxTwilightPoints",
        table.max_twilight_points(),
    );
    insert_optional(&mut fields, &mut t, "maxStorage", table.max_storage());
    insert_optional(
        &mut fields,
        &mut t,
        "maxFunctionDefs",
        table.max_function_defs(),
    );
    insert_optional(
        &mut fields,
        &mut t,
        "maxInstructionDefs",
        table.max_instruction_defs(),
    );
    insert_optional(
        &mut fields,
        &mut t,
        "maxStackElements",
        table.max_stack_elements(),
    );
    insert_optional(
        &mut fields,
        &mut t,
        "maxSizeOfInstructions",
        table.max_size_of_instructions(),
    );
    insert_optional(
        &mut fields,
        &mut t,
        "maxComponentElements",
        table.max_component_elements(),
    );
    insert_optional(
        &mut fields,
        &mut t,
        "maxComponentDepth",
        table.max_component_depth(),
    );

    Ok(Value::Object(fields))
}

fn insert_optional(
    fields: &mut Map<String, Value>,
    reader: &mut Reader,
    key: &'static str,
    value: Option<u16>,
) {
    if let Some(value) = value {
        fields.insert(key.into(), reader.read(value, 2));
    }
}
