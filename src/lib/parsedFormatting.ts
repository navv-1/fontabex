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
      return `[${value.length} ${isRecordArray(value) ? "records" : "items"}]`;
    }
    return `[Object(${Object.keys(value).length})]`;
  }
  return String(value);
}

export function formatParsedValue(value: unknown) {
  if (isParsedField(value)) {
    if (value.summary) return String(value.summary);
  }

  return formatValue(getParsedRawValue(value));
}
