import { readFileSync } from "node:fs";
import { describe, expect, test } from "bun:test";
import { defaultPath, previewTable, valueAt } from "./dataRows";

const receipts = JSON.parse(readFileSync("jrxmls/Receipts.json", "utf8")) as unknown;
const orders = JSON.parse(readFileSync("jrxmls/SubreportData.json", "utf8")) as unknown;

describe("previewTable", () => {
  test("a root list is one row per object", () => {
    expect(defaultPath(receipts)).toBe(".");
    const table = previewTable(receipts, ".");
    expect(table.missing).toBe(false);
    expect(table.total).toBe(10);
    expect(table.columns).toEqual(["receiptNo", "customer", "amount", "note"]);
    expect(table.rows[0]).toEqual(["RCP-1001", "Maria Anders", "120", "Counter sale"]);
    expect(table.rows[9]?.[0]).toBe("RCP-1010");
  });

  test("an object is one record until a list inside it is chosen", () => {
    expect(defaultPath(orders)).toBe("");
    const root = previewTable(orders, "");
    expect(root.total).toBe(1);
    expect(root.columns).toEqual(["company", "contact", "orders"]);
    expect(root.rows[0]?.[0]).toBe("Northwind Traders");
    expect(root.rows[0]?.[2]).toBe("10 items");

    const lines = previewTable(orders, "orders");
    expect(lines.total).toBe(10);
    expect(lines.columns).toEqual(["orderNo", "lines"]);
    expect(lines.rows[0]?.[0]).toBe("SO-1001");
    expect(lines.rows[0]?.[1]).toBe("4 items");

    const skus = previewTable(orders, "orders.0.lines");
    expect(skus.columns).toEqual(["sku", "qty"]);
    expect(skus.rows[0]).toEqual(["BOLT-1", "4"]);
  });

  test("a missing path is reported", () => {
    expect(valueAt(orders, "missing")).toBeUndefined();
    expect(previewTable(orders, "missing").missing).toBe(true);
  });
});
