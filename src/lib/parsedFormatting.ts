export interface ParsedField {
  type?: string;
  value: unknown;
  offset: number;
  length: number;
  summary?: unknown;
  name?: unknown;
}

export function isParsedField(value: unknown): value is ParsedField {
  return (
    value !== null &&
    typeof value === "object" &&
    "value" in value &&
    "offset" in value &&
    "length" in value
  );
}

export function getParsedRawValue(value: unknown) {
  return isParsedField(value) ? value.value : value;
}

function isRecordArray(value: unknown[]) {
  return value.every((item) => {
    const rawItem = getParsedRawValue(item);
    return (
      rawItem !== null && typeof rawItem === "object" && !Array.isArray(rawItem)
    );
  });
}

export function formatValue(value: unknown): string {
  if (value === null) return "null";
  if (typeof value === "object") {
    if (Array.isArray(value)) {
      if (value.length === 0) return "[]";
      return `[${value.length} ${isRecordArray(value) ? "records" : "items"}]`;
    }
    const keys = Object.keys(value);
    if (keys.length === 0) return "{}";
    const displayEntries = keys
      .slice(0, 2)
      .map((k) => `${k}: ${formatParsedValue((value as any)[k])}`);
    if (keys.length > 2) displayEntries.push("...");
    return `{${displayEntries.join(", ")}}`;
  }
  return String(value);
}

export function formatParsedValue(value: unknown) {
  if (isParsedField(value)) {
    if (value.summary) return String(value.summary);
  }

  return formatValue(getParsedRawValue(value));
}
