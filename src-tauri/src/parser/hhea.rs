use super::reader::Reader;
use read_fonts::{FontRef, TableProvider};
use serde_json::{json, Value};

pub fn parse(font: &FontRef<'_>) -> Result<Value, String> {
    let table = font.hhea().map_err(|e| e.to_string())?;
    let mut t = Reader::new();

    Ok(json!({
        "majorVersion": t.read_as(table.version().major, 2, "uint16"),
        "minorVersion": t.read_as(table.version().minor, 2, "uint16"),
        "ascender": t.read_as(table.ascender(), 2, "FWORD"),
        "descender": t.read_as(table.descender(), 2, "FWORD"),
        "lineGap": t.read_as(table.line_gap(), 2, "FWORD"),
        "advanceWidthMax": t.read_as(table.advance_width_max(), 2, "UFWORD"),
        "minLeftSideBearing": t.read_as(table.min_left_side_bearing(), 2, "FWORD"),
        "minRightSideBearing": t.read_as(table.min_right_side_bearing(), 2, "FWORD"),
        "xMaxExtent": t.read_as(table.x_max_extent(), 2, "FWORD"),
        "caretSlopeRise": t.read(table.caret_slope_rise(), 2),
        "caretSlopeRun": t.read(table.caret_slope_run(), 2),
        "caretOffset": t.read(table.caret_offset(), 2),
        "reserved0": t.read(0i16, 2),
        "reserved1": t.read(0i16, 2),
        "reserved2": t.read(0i16, 2),
        "reserved3": t.read(0i16, 2),
        "metricDataFormat": t.read(table.metric_data_format(), 2),
        "numberOfHMetrics": t.read(table.number_of_h_metrics(), 2)
    }))
}
