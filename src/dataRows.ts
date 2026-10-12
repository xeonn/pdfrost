/** How many records the Data tab draws. The rest stay in the file. */
export const DATA_ROW_LIMIT = 100;

const COLUMN_LIMIT = 40;

export type DataTable = {
  columns: string[];
  rows: string[][];
  total: number;
  /** The path does not exist in the document. */
  missing: boolean;
};

/** Walk a dotted path. `.` and an empty path are the document itself. */
export function valueAt(root: unknown, path: string): unknown {
  const trimmed = path.trim();
  if (!trimmed || trimmed === ".") {
    return root;
  }
  let current: unknown = root;
  for (const part of trimmed.split(".")) {
    if (!part) {
      return undefined;
    }
    if (Array.isArray(current)) {
      if (!/^\d+$/.test(part)) {
        return undefined;
      }
      current = current[Number(part)];
      continue;
    }
    if (current !== null && typeof current === "object") {
      current = (current as Record<string, unknown>)[part];
      continue;
    }
    return undefined;
  }
  return current;
}

/** A root list is the record set. An object is one record until a list is chosen. */
export function defaultPath(root: unknown): string {
  return Array.isArray(root) ? "." : "";
}

export function previewTable(root: unknown, path: string): DataTable {
  const value = valueAt(root, path);
  if (value === undefined) {
    return { columns: [], rows: [], total: 0, missing: true };
  }
  const records = Array.isArray(value) ? value : [value];
  const shown = records.slice(0, DATA_ROW_LIMIT);
  const objects = shown.map((record) =>
    record !== null && typeof record === "object" && !Array.isArray(record)
      ? (record as Record<string, unknown>)
      : null,
  );
  const columns: string[] = [];
  const seen = new Set<string>();
  if (objects.some((row) => row === null)) {
    columns.push("value");
    seen.add("value");
  }
  for (const row of objects) {
    if (!row) {
      continue;
    }
    for (const key of Object.keys(row)) {
      if (!key || seen.has(key)) {
        continue;
      }
      seen.add(key);
      columns.push(key);
      if (columns.length >= COLUMN_LIMIT) {
        break;
      }
    }
    if (columns.length >= COLUMN_LIMIT) {
      break;
    }
  }
  const rows = shown.map((record, index) => {
    const object = objects[index];
    return columns.map((column) => {
      if (column === "value" && !object) {
        return cellText(record);
      }
      if (!object || !(column in object)) {
        return "";
      }
      return cellText(object[column]);
    });
  });
  return { columns, rows, total: records.length, missing: false };
}

export type ReadField = {
  name: string;
  className: string;
  jsonPath: string;
};

/**
 * Columns at `path` become fields. A list or object is skipped.
 * A column is an Integer only when every value is a whole number.
 * `null` means the path is not in the document.
 */
export function readFields(root: unknown, path: string): ReadField[] | null {
  const value = valueAt(root, path);
  if (value === undefined) {
    return null;
  }
  const records = Array.isArray(value) ? value : [value];
  const shown = records.slice(0, DATA_ROW_LIMIT);
  const columns = previewTable(root, path).columns;
  const fields: ReadField[] = [];
  for (const column of columns) {
    const className = classNameFor(shown, column);
    if (!className) {
      continue;
    }
    fields.push({ name: column, className, jsonPath: column });
  }
  return fields;
}

function classNameFor(records: unknown[], column: string): string | null {
  let integer = true;
  let any = false;
  for (const record of records) {
    const value = valueOf(record, column);
    if (value === undefined || value === null || typeof value === "object") {
      continue;
    }
    any = true;
    if (typeof value !== "number" || !Number.isInteger(value)) {
      integer = false;
    }
  }
  if (!any) {
    return null;
  }
  return integer ? "java.lang.Integer" : "java.lang.String";
}

function valueOf(record: unknown, column: string): unknown {
  if (record !== null && typeof record === "object" && !Array.isArray(record)) {
    return (record as Record<string, unknown>)[column];
  }
  return column === "value" ? record : undefined;
}

function cellText(value: unknown): string {
  if (value === null || value === undefined) {
    return "";
  }
  if (typeof value === "string" || typeof value === "number" || typeof value === "boolean") {
    return String(value);
  }
  if (Array.isArray(value)) {
    return value.length === 1 ? "1 item" : `${value.length} items`;
  }
  if (typeof value === "object") {
    const count = Object.keys(value as object).length;
    return count === 1 ? "1 field" : `${count} fields`;
  }
  return "";
}
