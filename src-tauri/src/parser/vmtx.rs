use super::reader::parsed_field;
use read_fonts::tables::vmtx::LongMetric;
use read_fonts::types::FixedSize;
use read_fonts::{FontRef, TableProvider};
use serde_json::{json, Value};

pub fn parse(font: &FontRef<'_>) -> Result<Value, String> {
    let table = font.vmtx().map_err(|e| e.to_string())?;

    let v_metrics = table
        .v_metrics()
        .iter()
        .enumerate()
        .map(|(index, metric)| parse_long_metric(index, metric))
        .collect::<Vec<_>>();
    let v_metrics_length = table.v_metrics().len() * LongMetric::RAW_BYTE_LEN;

    let top_side_bearings = table
        .top_side_bearings()
        .iter()
        .enumerate()
        .map(|(index, side_bearing)| {
            parsed_field("FWORD", side_bearing.get(), v_metrics_length + index * 2, 2)
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "vMetrics": {
            "type": "LongVerMetric[]",
            "value": v_metrics,
            "offset": 0,
            "length": v_metrics_length
        },
        "topSideBearings": {
            "type": "FWORD[]",
            "value": top_side_bearings,
            "offset": v_metrics_length,
            "length": table.top_side_bearings().len() * 2
        }
    }))
}

fn parse_long_metric(index: usize, metric: &LongMetric) -> Value {
    let offset = index * LongMetric::RAW_BYTE_LEN;
    json!({
        "advanceHeight": parsed_field("UFWORD", metric.advance(), offset, 2),
        "tsb": parsed_field("FWORD", metric.side_bearing(), offset + 2, 2)
    })
}
