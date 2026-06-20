use super::reader::Reader;
use read_fonts::{FontRef, TableProvider};
use serde_json::{json, Value};

pub fn parse(font: &FontRef<'_>) -> Result<Value, String> {
    let table = font.hhea().map_err(|e| e.to_string())?;
    let mut t = Reader::new();

    Ok(json!({
        "version": t.read_as(table.version().to_string(), 4, "MajorMinor"),
        "ascender": t.read(table.ascender(), 2),
        "descender": t.read(table.descender(), 2),
        "lineGap": t.read(table.line_gap(), 2),
        "advanceWidthMax": t.read(table.advance_width_max(), 2),
        "minLeftSideBearing": t.read(table.min_left_side_bearing(), 2),
        "minRightSideBearing": t.read(table.min_right_side_bearing(), 2),
        "xMaxExtent": t.read(table.x_max_extent(), 2),
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
