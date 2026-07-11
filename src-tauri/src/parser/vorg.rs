use super::reader::Reader;
use read_fonts::{FontRef, TableProvider};
use serde_json::{json, Value};

pub fn parse(font: &FontRef<'_>) -> Result<Value, String> {
    let table = font.vorg().map_err(|e| e.to_string())?;
    let mut t = Reader::new();

    let major = t.read_as(table.version().major, 2, "uint16");
    let minor = t.read_as(table.version().minor, 2, "uint16");
    let default_y = t.read(table.default_vert_origin_y(), 2);
    let num_metrics = t.read(table.num_vert_origin_y_metrics(), 2);

    let metrics_offset = t.current_offset();
    let mut metrics_array = Vec::new();
    for metric in table.vert_origin_y_metrics() {
        metrics_array.push(json!({
            "glyphIndex": t.read_as(metric.glyph_index(), 2, "uint16"),
            "vertOriginY": t.read(metric.vert_origin_y(), 2),
        }));
    }
    let metrics_length = t.current_offset() - metrics_offset;

    Ok(json!({
        "majorVersion": major,
        "minorVersion": minor,
        "defaultVertOriginY": default_y,
        "numVertOriginYMetrics": num_metrics,
        "vertOriginYMetrics": {
            "type": "VertOriginYMetrics[]",
            "value": metrics_array,
            "offset": metrics_offset,
            "length": metrics_length
        }
    }))
}
