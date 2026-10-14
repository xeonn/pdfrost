import { describe, expect, test } from "bun:test";
import { imageClassForExpression, imageExpressionForClass, imagePictureName, retainedImagePictures } from "./report";

describe("image expression class", () => {
  test("a path becomes a File or URL call, and String unwraps it", () => {
    expect(imageExpressionForClass('"logo.png"', "java.io.File")).toEqual({
      expression: 'new java.io.File("logo.png")',
      expressionClass: "java.io.File",
    });
    expect(imageExpressionForClass('new java.io.File("logo.png")', "java.net.URL")).toEqual({
      expression: 'new java.net.URL("logo.png")',
      expressionClass: "java.net.URL",
    });
    expect(imageExpressionForClass('new java.net.URL($F{logo})', "java.lang.String")).toEqual({
      expression: "$F{logo}",
      expressionClass: null,
    });
  });

  test("typing a constructor sets its class, and a path sets String", () => {
    expect(imageClassForExpression('new java.io.File("logo.png")')).toBe("java.io.File");
    expect(imageClassForExpression("new java.net.URL($F{logo})")).toBe("java.net.URL");
    expect(imageClassForExpression("$F{logo}")).toBeNull();
    expect(imageClassForExpression('"logo.png"')).toBeNull();
    expect(imageClassForExpression("new java.io.ByteArrayInputStream($F{logo})")).toBeUndefined();
  });

  test("a stream expression is left unchanged", () => {
    const stream = "new java.io.ByteArrayInputStream($F{logo})";
    expect(imageExpressionForClass(stream, "java.io.File")).toBeNull();
    expect(imageExpressionForClass(stream, "java.lang.String")).toBeNull();
  });

  test("a replaced picture drops the property the expression no longer names", () => {
    const pictures = { "pdfrost.logo": "aaa", "pdfrost.logo2": "bbb" };
    expect(retainedImagePictures("$F{pdfrost.logo2}", pictures)).toEqual({ "pdfrost.logo2": "bbb" });
    expect(retainedImagePictures('new java.io.File($F{pdfrost.logo})', pictures)).toEqual({ "pdfrost.logo": "aaa" });
    expect(retainedImagePictures('"logo.png"', pictures)).toBeUndefined();
    expect(imagePictureName('new java.io.File($F{pdfrost.logo})', pictures)).toBe("pdfrost.logo");
  });
});
