import { describe, expect, test } from "bun:test";
import { blankReport, datasetListPath, fieldsFromDiscovery, type Field } from "./report";

function field(name: string, description: string | null = null): Field {
  return { name, className: "java.lang.String", description, jsonPath: name };
}

describe("fieldsFromDiscovery", () => {
  test("appends new names and keeps an existing field", () => {
    const previous = [field("customer", "buyer"), field("note")];
    const next = fieldsFromDiscovery(previous, [
      { name: "customer", className: "java.lang.Integer", jsonPath: "customer" },
      { name: "amount", className: "java.lang.Integer", jsonPath: "amount" },
    ]);
    expect(next.map((item) => item.name)).toEqual(["customer", "note", "amount"]);
    expect(next[0]).toBe(previous[0]);
    expect(next[2]?.className).toBe("java.lang.Integer");
    const refreshed = fieldsFromDiscovery(previous, [
      { name: "customer", className: "java.lang.String", jsonPath: "(root).Northwind.Customers.customer" },
    ]);
    expect(refreshed[0]?.jsonPath).toBe("(root).Northwind.Customers.customer");
    expect(refreshed[0]?.description).toBe("buyer");
  });

  test("drops a repeated name and leaves every other field", () => {
    const previous = [field("customer", "buyer"), field("customer", "copy"), field("note")];
    const next = fieldsFromDiscovery(previous, []);
    expect(next.map((item) => item.name)).toEqual(["customer", "note"]);
    expect(next[0]?.description).toBe("buyer");
  });

  test("a dataset reads the list named on its table or chart", () => {
    const report = blankReport();
    report.subdatasets.push({ name: "Lines", fields: [] });
    const box = { x: 0, y: 0, width: 100, height: 40, mode: null, backcolor: null, borders: null };
    report.bands[1]?.elements.push({
      type: "table",
      element: box,
      whenNoData: null,
      subDataset: "Lines",
      dataSource: "orders.lines",
      columns: [],
    });
    expect(datasetListPath(report, "Lines")).toBe("orders.lines");
    expect(datasetListPath(report, "Report")).toBeNull();
  });
});
