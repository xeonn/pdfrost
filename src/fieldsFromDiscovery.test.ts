import { describe, expect, test } from "bun:test";
import { fieldsFromDiscovery, type Field } from "./report";

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
  });

  test("drops a repeated name and leaves every other field", () => {
    const previous = [field("customer", "buyer"), field("customer", "copy"), field("note")];
    const next = fieldsFromDiscovery(previous, []);
    expect(next.map((item) => item.name)).toEqual(["customer", "note"]);
    expect(next[0]?.description).toBe("buyer");
  });
});
