use super::reader::Reader;
use read_fonts::FontRef;
use serde_json::{json, Value};

pub fn parse(font: &FontRef<'_>) -> Result<Value, String> {
    let td = &font.table_directory;
    let mut t = Reader::new();

    let sfnt_version = t.read_as(format!("0x{:08X}", td.sfnt_version()), 4, "uint32");
    let num_tables = t.read(td.num_tables(), 2);
    let search_range = t.read(td.search_range(), 2);
    let entry_selector = t.read(td.entry_selector(), 2);
    let range_shift = t.read(td.range_shift(), 2);

    let start_offset = t.current_offset();
    let mut records: Vec<Value> = Vec::new();
    for rec in td.table_records().iter() {
        records.push(json!({
            "tag": t.read_as(rec.tag(), 4, "Tag"),
            "checksum": t.read_as(format!("0x{:08X}", rec.checksum()), 4, "uint32"),
            "offset": t.read(rec.offset(), 4),
            "length": t.read(rec.length(), 4)
        }));
    }

    let records_length = t.current_offset() - start_offset;
    let records_val = json!({
        "type": "TableRecord[]",
        "value": records,
        "offset": start_offset,
        "length": records_length
    });

    Ok(json!({
        "sfntVersion": sfnt_version,
        "numTables": num_tables,
        "searchRange": search_range,
        "entrySelector": entry_selector,
        "rangeShift": range_shift,
        "tableRecords": records_val
    }))
}
