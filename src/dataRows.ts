/** How many records one page of the Data tab draws. */
export const DATA_PAGE_SIZE = 100;

/** How many records Read Fields inspects when choosing a class. */
export const DATA_ROW_LIMIT = DATA_PAGE_SIZE;

const COLUMN_LIMIT = 40;

export type DataTable = {
  columns: string[];
  rows: string[][];
  total: number;
  page: number;
  pageSize: number;
  pageCount: number;
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

export function previewTable(root: unknown, path: string, page = 0): DataTable {
  const value = valueAt(root, path);
  if (value === undefined) {
    return { columns: [], rows: [], total: 0, page: 0, pageSize: DATA_PAGE_SIZE, pageCount: 1, missing: true };
  }
  const records = Array.isArray(value) ? value : [value];
  const pageCount = Math.max(1, Math.ceil(records.length / DATA_PAGE_SIZE));
  const safePage = Math.min(Math.max(0, page), pageCount - 1);
  const start = safePage * DATA_PAGE_SIZE;
  const shown = records.slice(start, start + DATA_PAGE_SIZE);
  const objects = records.map((record) =>
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
  const pageObjects = objects.slice(start, start + DATA_PAGE_SIZE);
  const rows = shown.map((record, index) => {
    const object = pageObjects[index];
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
  return {
    columns,
    rows,
    total: records.length,
    page: safePage,
    pageSize: DATA_PAGE_SIZE,
    pageCount,
    missing: false,
  };
}

export type ReadField = {
  name: string;
  className: string;
  jsonPath: string;
};

/**
 * Leaf values at `path` become fields. A nested object is a dotted path,
 * such as `product.id`, stored under the name `product_id`.
 * An array is a list, not a field. `null` means the path is not in the document.
 */
export function readFields(root: unknown, path: string): ReadField[] | null {
  const value = valueAt(root, path);
  if (value === undefined) {
    return null;
  }
  const records = Array.isArray(value) ? value : [value];
  const leaves = new Map<string, unknown[]>();
  collectLeaves(records.slice(0, DATA_ROW_LIMIT), "", 0, leaves);
  const fields: ReadField[] = [];
  for (const [jsonPath, values] of leaves) {
    if (fields.length >= COLUMN_LIMIT) {
      break;
    }
    fields.push({
      name: fieldName(jsonPath, fields.map((field) => field.name)),
      className: integerValues(values) ? "java.lang.Integer" : "java.lang.String",
      jsonPath: studioFieldPath(path, jsonPath),
    });
  }
  return fields;
}

/** Studio's JSON field expression. `(root)` is the document. Array indexes are omitted. */
export function studioFieldPath(recordPath: string, relative: string): string {
  const base = recordPath
    .split(".")
    .map((part) => part.trim())
    .filter((part) => part && part !== "." && !/^\d+$/.test(part))
    .join(".");
  const tail = relative
    .split(".")
    .map((part) => part.trim())
    .filter((part) => part.length > 0)
    .join(".");
  if (!base) {
    return tail ? `(root).${tail}` : "(root)";
  }
  return tail ? `(root).${base}.${tail}` : `(root).${base}`;
}

/**
 * The record path Read Fields should use.
 * A node with its own values is that path. Otherwise the first list of
 * objects under it, in key order, such as `Northwind.Customers`.
 */
export function recordPath(root: unknown, start: string): string | null {
  if (valueAt(root, start) === undefined) {
    return null;
  }
  const direct = readFields(root, start);
  if (direct && direct.length > 0) {
    return start;
  }
  const value = valueAt(root, start);
  if (Array.isArray(value)) {
    const first = value.find(isRecord);
    if (!first) {
      return null;
    }
    const prefix = start === "." || start === "" ? "0" : `${start}.0`;
    return firstList(root, first, prefix, 0);
  }
  if (isRecord(value)) {
    return firstList(root, value, start === "." ? "" : start, 0);
  }
  return null;
}

function firstList(root: unknown, value: Record<string, unknown>, path: string, depth: number): string | null {
  if (depth > 8) {
    return null;
  }
  for (const [key, child] of Object.entries(value)) {
    const childPath = path ? `${path}.${key}` : key;
    if (Array.isArray(child)) {
      const fields = readFields(root, childPath);
      if (fields && fields.length > 0) {
        return childPath;
      }
      const first = child.find(isRecord);
      if (first) {
        const nested = firstList(root, first, `${childPath}.0`, depth + 1);
        if (nested) {
          return nested;
        }
      }
      continue;
    }
    if (isRecord(child)) {
      const nested = firstList(root, child, childPath, depth + 1);
      if (nested) {
        return nested;
      }
    }
  }
  return null;
}

function collectLeaves(records: unknown[], prefix: string, depth: number, out: Map<string, unknown[]>): void {
  if (depth > 5 || out.size >= COLUMN_LIMIT) {
    return;
  }
  const grouped = new Map<string, unknown[]>();
  for (const record of records) {
    if (!isRecord(record)) {
      continue;
    }
    for (const [key, value] of Object.entries(record)) {
      if (!key) {
        continue;
      }
      const path = prefix ? `${prefix}.${key}` : key;
      const values = grouped.get(path);
      if (values) {
        values.push(value);
      } else {
        grouped.set(path, [value]);
      }
    }
  }
  for (const [path, values] of grouped) {
    if (out.size >= COLUMN_LIMIT) {
      return;
    }
    const present = values.filter((value) => value !== null && value !== undefined);
    const primitives = present.filter((value) => !isRecord(value) && !Array.isArray(value));
    if (primitives.length > 0) {
      out.set(path, primitives);
      continue;
    }
    const nested = present.filter(isRecord);
    if (nested.length > 0) {
      collectLeaves(nested, path, depth + 1, out);
    }
  }
}

function fieldName(path: string, used: string[]): string {
  const base = path.replace(/\./g, "_");
  let name = base;
  let suffix = 2;
  while (used.includes(name)) {
    name = `${base}_${suffix}`;
    suffix += 1;
  }
  return name;
}

function integerValues(values: unknown[]): boolean {
  return values.every((value) => typeof value === "number" && Number.isInteger(value));
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
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
