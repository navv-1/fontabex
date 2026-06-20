use super::reader::Reader;
use read_fonts::tables::head::{Flags, MacStyle};
use read_fonts::{FontRef, TableProvider};
use serde_json::{json, Value};

pub fn parse(font: &FontRef<'_>) -> Result<Value, String> {
    let table = font.head().map_err(|e| e.to_string())?;
    let mut t = Reader::new();

    Ok(json!({
        "version": t.read_as(table.version().to_string(), 4, "MajorMinor"),
        "fontRevision": t.read_as(table.font_revision().to_string(), 4, "Fixed"),
        "checksumAdjustment": t.read(table.checksum_adjustment(), 4),
        "magicNumber": t.read(format!("0x{:08X}", table.magic_number()), 4),
        "flags": t.read_as(format_head_flags(table.flags()), 2, "Flags"),
        "unitsPerEm": t.read(table.units_per_em(), 2),
        "created": t.read_as(format_long_date_time(table.created().as_secs()), 8, "LongDateTime"),
        "modified": t.read_as(format_long_date_time(table.modified().as_secs()), 8, "LongDateTime"),
        "xMin": t.read(table.x_min(), 2),
        "yMin": t.read(table.y_min(), 2),
        "xMax": t.read(table.x_max(), 2),
        "yMax": t.read(table.y_max(), 2),
        "macStyle": t.read_as(format_mac_style(table.mac_style()), 2, "MacStyle"),
        "lowestRecPpem": t.read(table.lowest_rec_ppem(), 2),
        "fontDirectionHint": t.read(table.font_direction_hint(), 2),
        "indexToLocFormat": t.read(table.index_to_loc_format(), 2),
        "glyphDataFormat": t.read(table.glyph_data_format(), 2)
    }))
}

fn format_long_date_time(secs_since_1904: i64) -> String {
    const SECS_BETWEEN_1904_AND_1970: i64 = 2_082_844_800;
    let unix_secs = secs_since_1904 - SECS_BETWEEN_1904_AND_1970;
    let days = unix_secs.div_euclid(86_400);
    let seconds_of_day = unix_secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
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

fn format_head_flags(flags: Flags) -> String {
    let mut names = Vec::new();
    for (flag, name) in [
        (Flags::BASELINE_AT_Y_0, "Baseline at y=0"),
        (Flags::LSB_AT_X_0, "LSB at x=0"),
        (
            Flags::INSTRUCTIONS_MAY_DEPEND_ON_POINT_SIZE,
            "Instructions may depend on point size",
        ),
        (Flags::FORCE_INTEGER_PPEM, "Force integer PPEM"),
        (
            Flags::INSTRUCTIONS_MAY_ALTER_ADVANCE_WIDTH,
            "Instructions may alter advance width",
        ),
        (
            Flags::LOSSLESS_TRANSFORMED_FONT_DATA,
            "Lossless transformed font data",
        ),
        (Flags::CONVERTED_FONT, "Converted font"),
        (Flags::OPTIMIZED_FOR_CLEARTYPE, "Optimized for ClearType"),
        (Flags::LAST_RESORT_FONT, "Last Resort font"),
    ] {
        if flags.contains(flag) {
            names.push(name);
        }
    }

    let bits = flags.bits();
    let label = if names.is_empty() {
        "None".to_string()
    } else {
        names.join(", ")
    };
    format!("0x{bits:04X} ({label})")
}
