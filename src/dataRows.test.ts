import { readFileSync } from "node:fs";
import { describe, expect, test } from "bun:test";
import { defaultPath, previewTable, readFields, recordPath, valueAt } from "./dataRows";

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

  test("read fields keeps primitive columns and the integer class", () => {
    expect(readFields(receipts, ".")).toEqual([
      { name: "receiptNo", className: "java.lang.String", jsonPath: "(root).receiptNo" },
      { name: "customer", className: "java.lang.String", jsonPath: "(root).customer" },
      { name: "amount", className: "java.lang.Integer", jsonPath: "(root).amount" },
      { name: "note", className: "java.lang.String", jsonPath: "(root).note" },
    ]);
    expect(readFields(orders, "")).toEqual([
      { name: "company", className: "java.lang.String", jsonPath: "(root).company" },
      { name: "contact", className: "java.lang.String", jsonPath: "(root).contact" },
    ]);
    expect(readFields(orders, "orders.0.lines")).toEqual([
      { name: "sku", className: "java.lang.String", jsonPath: "(root).orders.lines.sku" },
      { name: "qty", className: "java.lang.Integer", jsonPath: "(root).orders.lines.qty" },
    ]);
    expect(readFields(JSON.parse(readFileSync("jrxmls/SampleData.json", "utf8")) as unknown, "Northwind.Customers")).toEqual(
      expect.arrayContaining([
        { name: "Address", className: "java.lang.String", jsonPath: "(root).Northwind.Customers.Address" },
      ]),
    );
    expect(readFields(orders, "missing")).toBeNull();
    expect(recordPath(JSON.parse(readFileSync("jrxmls/SampleData.json", "utf8")) as unknown, "")).toBe(
      "Northwind.Customers",
    );
    expect(recordPath({ itemsList: [{ sku: "A", qty: 1 }] }, "")).toBe("itemsList");
    expect(recordPath(receipts, ".")).toBe(".");
    const many = Array.from({ length: 120 }, (_, index) => ({ n: index }));
    const first = previewTable(many, ".", 0);
    expect(first.pageCount).toBe(2);
    expect(first.rows).toHaveLength(100);
    expect(first.rows[0]?.[0]).toBe("0");
    const second = previewTable(many, ".", 1);
    expect(second.rows).toHaveLength(20);
    expect(second.rows[0]?.[0]).toBe("100");
    expect(
      readFields(
        { docno: "1", audit: { created: "t" }, items: [{ sku: "A" }] },
        "",
      ),
    ).toEqual([
      { name: "docno", className: "java.lang.String", jsonPath: "(root).docno" },
      { name: "audit_created", className: "java.lang.String", jsonPath: "(root).audit.created" },
    ]);
  });

  test("a missing path is reported", () => {
    expect(valueAt(orders, "missing")).toBeUndefined();
    expect(previewTable(orders, "missing").missing).toBe(true);
  });
});
