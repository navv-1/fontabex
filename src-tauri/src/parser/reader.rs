use serde::Serialize;
use serde_json::{json, Value};
use std::any::type_name;

pub struct Reader {
    offset: usize,
}

impl Reader {
    pub fn new() -> Self {
        Self { offset: 0 }
    }

    /// Read a value of `size` bytes and return it wrapped with type, offset, and length.
    pub fn read<T: Serialize>(&mut self, value: T, size: usize) -> Value {
        self.read_as(value, size, font_type_name::<T>())
    }

    pub fn read_as<T: Serialize>(
        &mut self,
        value: T,
        size: usize,
        data_type: &'static str,
    ) -> Value {
        let current = self.offset;
        self.offset += size;
        json!({
            "type": data_type,
            "value": value,
            "offset": current,
            "length": size
        })
    }

    /// Return the current offset.
    pub fn current_offset(&self) -> usize {
        self.offset
    }
}

fn font_type_name<T>() -> &'static str {
    match type_name::<T>() {
        "u8" => "uint8",
        "i8" => "int8",
        "u16" => "uint16",
        "i16" => "int16",
        "u32" => "uint32",
        "i32" => "int32",
        "u64" => "uint64",
        "i64" => "int64",
        "bool" => "bool",
        _ => type_name::<T>().rsplit("::").next().unwrap_or("unknown"),
    }
}
