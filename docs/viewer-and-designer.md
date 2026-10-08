# Viewer and designer

The desktop app is a Tauri window over the engine. The viewer is built first. The designer is the same window later, and only for tags the engine already renders.

## Viewer (this pass)

The Preview-shaped viewer plan is in [Viewer](viewer.md): open a JRXML, render with empty data `{}`, show the PDF with page and zoom controls, list warnings, optionally choose JSON, and switch to a Design placeholder.

The page on screen is always PDF bytes from `render`. The UI does not interpret JRXML and does not keep a second layout.

## Designer

The Design tab is still a shell. The plan is in [Designer](designer.md): a Jaspersoft Studio-shaped band editor for the supported subset, editing the engine’s `Report` and saving with `write_report`. Preview still calls the same render command. Charts, images, subreports, and other unsupported tags stay off the canvas.

## Shared rule

Anything the window can create, the engine can parse. Anything the window can show as a finished page came from the engine as PDF bytes.
