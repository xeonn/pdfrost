# Images

An `image` element draws a PNG or JPEG in its `reportElement` box. The picture comes from `imageExpression`. This note is the load rule for that expression. The short contract stays in [JRXML subset](jrxml-subset.md).

JasperReports 6.20.1 is the behavior reference. A company logo on an operational form arrives as raw base64 on a JSON field, with no `data:` prefix. Studio decodes that string in Java. The expression pdfrost fills is the field itself, so the same bytes draw when the engine cannot open a file.

## Expression

| Form | What the text is |
| --- | --- |
| A quoted string | A file path, raw base64, or a `data:image/...;base64,` URI. |
| `$F{name}` | A `pdfrost.` property of that name on this image, when one is stored. Otherwise the field text, read the same way. |
| `new java.io.File(...)` | The path inside the call. The argument is a quoted string or `$F{name}`. |
| `new java.net.URL(...)` | The address inside the call, read as a location. `http` and `https` stay unloaded. |
| `new java.io.ByteArrayInputStream(decode(...))` | The decode argument, read as base64. `decode` is `Base64.getMimeDecoder().decode`, `Base64.getDecoder().decode`, `Base64.decodeBase64`, or `Base64Util.decode`. The argument is a quoted string or `$F{name}`, with an optional `.trim()` or `.getBytes()`. A `? null :` branch in front of the constructor is skipped. |

`read_image_bytes` resolves that text in this order:

1. A `data:` URI is decoded immediately. The payload after the comma is standard base64. Whitespace is removed first. The decoded bytes must be a PNG or JPEG.
2. An `http://` or `https://` location is left unloaded.
3. Any other string is a path. An absolute path is read as given. A relative path is read beside the template directory when the caller passed one, then from the process directory. The file must decode as a PNG or JPEG.
4. When no file decodes, the same string is tried as raw base64. Whitespace is removed. The alphabet is standard base64 (`A–Z`, `a–z`, `0–9`, `+`, `/`, `=`). The decoded bytes must be a PNG or JPEG.

A build with no filesystem still draws steps 1 and 4. The Go package is that build.

## Expression class

The designer writes one of three classes. An absent class is `java.lang.String`, and that name is left off the element. Choosing File or URL rewrites a quoted path or `$F{name}` into `new java.io.File(...)` or `new java.net.URL(...)`. Choosing String unwraps that call. Typing one of those constructors sets that class. Typing a quoted path or `$F{name}` sets String. An expression that is neither of those stays as written, and the class stays with it. The picture is still read as a path or as base64 once the call is reduced.

| Class | Studio | This engine |
| --- | --- | --- |
| `java.lang.String` | A path, URL, or classpath resource. | A path or base64. Base64 stays blank in Studio. |
| `java.io.File` | A `java.io.File` returned by the expression. | `new java.io.File(...)` reads that path. Other text is still a path or base64. |
| `java.net.URL` | A `java.net.URL` returned by the expression. | `new java.net.URL(...)` reads that address. An `http` or `https` address stays unloaded. |

Any other class, such as `java.io.InputStream` or `java.awt.Image`, is kept on the element and named in the properties as not supported. It does not change how the text is read. Choose String, File, or URL before saving so the expression stays one Studio accepts.

## Logo on a JSON field

Studio loads the logo with a Java expression and hides the element when the field is blank:

```java
$F{logoBase64} == null || $F{logoBase64}.trim().isEmpty()
  ? null
  : new java.io.ByteArrayInputStream(
      java.util.Base64.getMimeDecoder().decode($F{logoBase64}.trim()))
```

That constructor is evaluated. The decoded field text is the picture. `$F{logoBase64}` alone is the same bytes when the field holds raw base64. The field path for this logo is `company.logoBase64`.

```xml
<image scaleImage="RetainShape" hAlign="Left" vAlign="Top" onErrorType="Blank">
  <reportElement x="0" y="0" width="130" height="40"/>
  <imageExpression><![CDATA[$F{logoBase64}]]></imageExpression>
</image>
```

`RetainShape` scales the picture uniformly inside the box. `Left` and `Top` place it in that box. `130` by `40` is the Studio slot. `onErrorType="Blank"` draws nothing when the field is empty or the bytes are not a PNG or JPEG.

A missing or null field still adds a warning for that field name, then the blank error type draws nothing. `onErrorType="Error"`, the default, adds a warning tagged `image:` plus the expression text when the bytes cannot be loaded, and draws nothing. `Icon` draws a small mark. JasperReports stops the fill when the error type is `Error`.

An `image` with no `imageExpression` has nothing to decode, so the box stays empty. Declaring `logoBase64` does not attach it to that element.

## Embedded picture

Browsing for a PNG or JPEG in the designer stores the bytes on the image and points the expression at that property. Browsing again replaces that property. A property the expression no longer names is dropped. A field of the same name is added so Studio can compile `$F{name}`. The field has no JSON path. Studio keeps the property and draws the field when its value is a path. The base64 value stays blank in Studio.

```xml
<image>
  <reportElement x="0" y="0" width="50" height="50">
    <property name="pdfrost.logo" value="iVBORw0KGgo..."/>
  </reportElement>
  <imageExpression><![CDATA[$F{pdfrost.logo}]]></imageExpression>
</image>
```

`$F{pdfrost.logo}` reads the property before any JSON field of that name. A property whose name does not start with `pdfrost.` is unchanged: Studio metadata is still dropped, and any other property still warns.

## Fit

One pixel is one point. That matches JasperReports at its default of 72 DPI.

| `scaleImage` | Result |
| --- | --- |
| `Clip` | Natural size, clipped to the box. |
| `FillFrame` | Stretched to the box. |
| `RetainShape` | Scaled uniformly so the whole picture fits in the box. This is the default. |
| `RealHeight` | Width stays inside the box. A taller picture grows the element, and the band grows with it. |
| `RealSize` | The same fit as `RealHeight`. The element width stays inside the box. |

`rotation` of `Left`, `Right`, or `UpsideDown` turns the bitmap around the center of the box.

`isUsingCache`, `isLazy`, `evaluationTime`, and `evaluationGroup` are stored and written back. Fill loads the picture while the page is drawn and does not consult them.

## File paths in WebAssembly

The Go package runs `go/engine.wasm`, a `wasm32-unknown-unknown` build of `crates/pdfrost-wasm`. The export `render` calls `pdfrost_engine::render`. That call has no template directory. The module is `wasm32-unknown-unknown`: it has no WASI, and wazero is not given a directory to read. `std::fs::read` fails.

These locations fail in that build:

- A quoted path such as `"logo.png"`.
- A relative path beside the `.jrxml`.
- An absolute path.
- `$F{name}` when the field text is a path.

The failed path is then tried as base64. A path string does not decode to a PNG or JPEG, so the image is missing. `onErrorType="Blank"` leaves the box empty. The default `Error` adds a warning and still leaves the box empty.

The desktop app and a Rust caller of `render_with_assets` can read a file. Tauri passes the template's directory as the asset directory. Those callers are native. The Go package receives the `.jrxml` text and one JSON value. It cannot pass a logo file into the module.

Put the PNG or JPEG in the JSON field, or in the expression, as raw base64 or a `data:image/...;base64,` URI. `http` and `https` locations stay unloaded in the native builds as well.

## Left out

| Studio behavior | In this engine |
| --- | --- |
| Any other Java image expression | Warned and left blank. The constructors in [Expression](#expression) are the subset that runs. |
| `printWhenExpression` | The element is still visited. A blank field with `onErrorType="Blank"` draws nothing. |
| `onErrorType="Error"` fails the fill | A warning is recorded and the rest of the page is drawn. |
| GIF, WebP, TIFF, SVG | PNG and JPEG only. |
| An image DPI other than 72 | One pixel is one point. |
| A hyperlink on the image | Warned and omitted. |
