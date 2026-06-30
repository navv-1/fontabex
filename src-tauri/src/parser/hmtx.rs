use super::reader::parsed_field;
use read_fonts::tables::hmtx::LongMetric;
use read_fonts::types::FixedSize;
use read_fonts::{FontRef, TableProvider};
use serde_json::{json, Value};

pub fn parse(font: &FontRef<'_>) -> Result<Value, String> {
    let table = font.hmtx().map_err(|e| e.to_string())?;

    let h_metrics = table
        .h_metrics()
        .iter()
        .enumerate()
        .map(|(index, metric)| parse_long_metric(index, metric))
        .collect::<Vec<_>>();
    let h_metrics_length = table.h_metrics().len() * LongMetric::RAW_BYTE_LEN;

    let left_side_bearings = table
        .left_side_bearings()
        .iter()
        .enumerate()
        .map(|(index, side_bearing)| {
            parsed_field("FWORD", side_bearing.get(), h_metrics_length + index * 2, 2)
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "hMetrics": {
            "type": "LongHorMetric[]",
            "value": h_metrics,
            "offset": 0,
            "length": h_metrics_length
        },
        "leftSideBearings": {
            "type": "FWORD[]",
            "value": left_side_bearings,
            "offset": h_metrics_length,
            "length": table.left_side_bearings().len() * 2
        }
    }))
}

fn parse_long_metric(index: usize, metric: &LongMetric) -> Value {
    let offset = index * LongMetric::RAW_BYTE_LEN;
    json!({
        "advanceWidth": parsed_field("UFWORD", metric.advance(), offset, 2),
        "lsb": parsed_field("FWORD", metric.side_bearing(), offset + 2, 2)
    })
}
