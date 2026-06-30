use super::reader::Reader;
use read_fonts::{FontRef, TableProvider};
use serde_json::{Map, Value};

pub fn parse(font: &FontRef<'_>) -> Result<Value, String> {
    let table = font.vhea().map_err(|e| e.to_string())?;
    let mut t = Reader::new();
    let mut fields = Map::new();

    let version = table.version().to_string();
    let (ascender_name, descender_name, line_gap_name) = if version == "1.1" {
        ("vertTypoAscender", "vertTypoDescender", "vertTypoLineGap")
    } else {
        ("ascender", "descender", "lineGap")
    };

    fields.insert("version".into(), t.read_as(version, 4, "Version16Dot16"));
    fields.insert(
        ascender_name.into(),
        t.read_as(table.ascender(), 2, "FWORD"),
    );
    fields.insert(
        descender_name.into(),
        t.read_as(table.descender(), 2, "FWORD"),
    );
    fields.insert(
        line_gap_name.into(),
        t.read_as(table.line_gap(), 2, "FWORD"),
    );
    fields.insert(
        "advanceHeightMax".into(),
        t.read_as(table.advance_height_max(), 2, "UFWORD"),
    );
    fields.insert(
        "minTopSideBearing".into(),
        t.read_as(table.min_top_side_bearing(), 2, "FWORD"),
    );
    fields.insert(
        "minBottomSideBearing".into(),
        t.read_as(table.min_bottom_side_bearing(), 2, "FWORD"),
    );
    fields.insert(
        "yMaxExtent".into(),
        t.read_as(table.y_max_extent(), 2, "FWORD"),
    );
    fields.insert("caretSlopeRise".into(), t.read(table.caret_slope_rise(), 2));
    fields.insert("caretSlopeRun".into(), t.read(table.caret_slope_run(), 2));
    fields.insert("caretOffset".into(), t.read(table.caret_offset(), 2));
    fields.insert("reserved0".into(), t.read(0i16, 2));
    fields.insert("reserved1".into(), t.read(0i16, 2));
    fields.insert("reserved2".into(), t.read(0i16, 2));
    fields.insert("reserved3".into(), t.read(0i16, 2));
    fields.insert(
        "metricDataFormat".into(),
        t.read(table.metric_data_format(), 2),
    );
    fields.insert(
        "numberOfLongVerMetrics".into(),
        t.read(table.number_of_long_ver_metrics(), 2),
    );

    Ok(Value::Object(fields))
}
