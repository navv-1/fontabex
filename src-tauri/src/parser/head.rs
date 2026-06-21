use super::reader::Reader;
use read_fonts::tables::head::{Flags, MacStyle};
use read_fonts::{FontRef, MinByteRange, TableProvider};
use serde_json::{json, Value};
use std::ops::Range;

pub fn parse(font: &FontRef<'_>) -> Result<Value, String> {
    let table = font.head().map_err(|e| e.to_string())?;
    let mut t = Reader::new();

    Ok(json!({
        "version": t.read_as(table.version().to_string(), 4, "MajorMinor"),
        "fontRevision": t.read_as(table.font_revision().to_string(), 4, "Fixed"),
        "checksumAdjustment": t.read(table.checksum_adjustment(), 4),
        "magicNumber": t.read(format!("0x{:08X}", table.magic_number()), 4),
        "flags": t.read_as(format_head_flags(raw_head_flags(&table)?), 2, "uint16"),
        "unitsPerEm": t.read(table.units_per_em(), 2),
        "created": t.read_as(format_head_long_date_time(&table, table.created_byte_range())?, 8, "LongDateTime"),
        "modified": t.read_as(format_head_long_date_time(&table, table.modified_byte_range())?, 8, "LongDateTime"),
        "xMin": t.read(table.x_min(), 2),
        "yMin": t.read(table.y_min(), 2),
        "xMax": t.read(table.x_max(), 2),
        "yMax": t.read(table.y_max(), 2),
        "macStyle": t.read_as(format_mac_style(table.mac_style()), 2, "uint16"),
        "lowestRecPpem": t.read(table.lowest_rec_ppem(), 2),
        "fontDirectionHint": t.read(table.font_direction_hint(), 2),
        "indexToLocFormat": t.read(table.index_to_loc_format(), 2),
        "glyphDataFormat": t.read(table.glyph_data_format(), 2)
    }))
}

const SECS_BETWEEN_1904_AND_1970: i64 = 2_082_844_800;
const MIN_REASONABLE_LONG_DATE_TIME_YEAR: i64 = 1904;
const MAX_REASONABLE_LONG_DATE_TIME_YEAR: i64 = 2100;

struct DateTimeParts {
    year: i64,
    month: u32,
    day: u32,
    hour: i64,
    minute: i64,
    second: i64,
}

impl DateTimeParts {
    fn to_iso_string(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

fn format_head_long_date_time(
    table: &read_fonts::tables::head::Head<'_>,
    range: Range<usize>,
) -> Result<String, String> {
    let raw = raw_head_long_date_time(table, range)?;
    let secs = i64::from_be_bytes(raw);
    let parts = long_date_time_parts(secs);
    if is_reasonable_long_date_time(&parts) {
        return Ok(parts.to_iso_string());
    }

    let low_secs = u32::from_be_bytes([raw[4], raw[5], raw[6], raw[7]]) as i64;
    let low_parts = long_date_time_parts(low_secs);
    if is_reasonable_long_date_time(&low_parts) {
        return Ok(format!(
            "{} (low 32 bits; raw 0x{})",
            low_parts.to_iso_string(),
            format_raw_bytes(&raw)
        ));
    }

    Ok(format!("Invalid date (raw 0x{})", format_raw_bytes(&raw)))
}

fn raw_head_long_date_time(
    table: &read_fonts::tables::head::Head<'_>,
    range: Range<usize>,
) -> Result<[u8; 8], String> {
    let bytes = table.min_table_bytes();
    let date_bytes = bytes
        .get(range)
        .ok_or_else(|| "head LongDateTime byte range is outside the table".to_string())?;
    date_bytes
        .try_into()
        .map_err(|_| "head LongDateTime field is not 8 bytes".to_string())
}

fn long_date_time_parts(secs_since_1904: i64) -> DateTimeParts {
    let unix_secs = secs_since_1904 - SECS_BETWEEN_1904_AND_1970;
    let days = unix_secs.div_euclid(86_400);
    let seconds_of_day = unix_secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;

    DateTimeParts {
        year,
        month,
        day,
        hour,
        minute,
        second,
    }
}

fn is_reasonable_long_date_time(parts: &DateTimeParts) -> bool {
    (MIN_REASONABLE_LONG_DATE_TIME_YEAR..=MAX_REASONABLE_LONG_DATE_TIME_YEAR).contains(&parts.year)
}

fn format_raw_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join("")
}

fn civil_from_days(days_since_unix_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = year + i64::from(month <= 2);

    (year, month as u32, day as u32)
}

fn format_mac_style(mac_style: MacStyle) -> String {
    let mut names = Vec::new();
    for (flag, name) in [
        (MacStyle::BOLD, "Bold"),
        (MacStyle::ITALIC, "Italic"),
        (MacStyle::UNDERLINE, "Underline"),
        (MacStyle::OUTLINE, "Outline"),
        (MacStyle::SHADOW, "Shadow"),
        (MacStyle::CONDENSED, "Condensed"),
        (MacStyle::EXTENDED, "Extended"),
    ] {
        if mac_style.contains(flag) {
            names.push(name);
        }
    }

    let bits = mac_style.bits();
    let label = if names.is_empty() {
        "None".to_string()
    } else {
        names.join(", ")
    };
    format!("0x{bits:04X} ({label})")
}

fn raw_head_flags(table: &read_fonts::tables::head::Head<'_>) -> Result<u16, String> {
    let range = table.flags_byte_range();
    let bytes = table.min_table_bytes();
    let flags_bytes = bytes
        .get(range)
        .ok_or_else(|| "head.flags byte range is outside the table".to_string())?;
    Ok(u16::from_be_bytes([flags_bytes[0], flags_bytes[1]]))
}

fn format_head_flags(bits: u16) -> String {
    let mut names = Vec::new();
    for (flag, name) in [
        (Flags::BASELINE_AT_Y_0.bits(), "Baseline at y=0"),
        (Flags::LSB_AT_X_0.bits(), "LSB at x=0"),
        (
            Flags::INSTRUCTIONS_MAY_DEPEND_ON_POINT_SIZE.bits(),
            "Instructions may depend on point size",
        ),
        (Flags::FORCE_INTEGER_PPEM.bits(), "Force integer PPEM"),
        (
            Flags::INSTRUCTIONS_MAY_ALTER_ADVANCE_WIDTH.bits(),
            "Instructions may alter advance width",
        ),
        (
            Flags::LOSSLESS_TRANSFORMED_FONT_DATA.bits(),
            "Lossless transformed font data",
        ),
        (Flags::CONVERTED_FONT.bits(), "Converted font"),
        (
            Flags::OPTIMIZED_FOR_CLEARTYPE.bits(),
            "Optimized for ClearType",
        ),
        (Flags::LAST_RESORT_FONT.bits(), "Last Resort font"),
    ] {
        if bits & flag != 0 {
            names.push(name);
        }
    }

    let label = if names.is_empty() {
        "None".to_string()
    } else {
        names.join(", ")
    };
    format!("0x{bits:04X} ({label})")
}
