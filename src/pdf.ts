import * as pdfjs from "pdfjs-dist";
import type { PDFDocumentProxy } from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";

pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

export type FitMode = "none" | "width" | "page";

export class PdfStage {
  private document: PDFDocumentProxy | null = null;
  private bytes: Uint8Array | null = null;
  private renderToken = 0;

  get pageCount(): number {
    return this.document?.numPages ?? 0;
  }

  async load(bytes: Uint8Array): Promise<number> {
    await this.document?.cleanup();
    this.bytes = bytes;
    // Keep embedded fonts as FontFace so glyph edges anti-alias in the canvas.
    this.document = await pdfjs.getDocument({
      data: bytes,
      disableFontFace: false,
      useSystemFonts: false,
    }).promise;
    return this.document.numPages;
  }

  clear(): void {
    void this.document?.cleanup();
    this.document = null;
    this.bytes = null;
    this.renderToken += 1;
  }

  /** Open the system print dialog for the loaded PDF. */
  print(): void {
    if (!this.bytes) {
      return;
    }
    const copy = new Uint8Array(this.bytes);
    const url = URL.createObjectURL(new Blob([copy], { type: "application/pdf" }));
    const frame = window.document.createElement("iframe");
    frame.style.position = "fixed";
    frame.style.right = "0";
    frame.style.bottom = "0";
    frame.style.width = "0";
    frame.style.height = "0";
    frame.style.border = "0";
    frame.src = url;
    const cleanup = (): void => {
      frame.remove();
      URL.revokeObjectURL(url);
    };
    frame.addEventListener("load", () => {
      try {
        frame.contentWindow?.focus();
        frame.contentWindow?.print();
      } finally {
        window.setTimeout(cleanup, 60_000);
      }
    });
    window.document.body.append(frame);
  }

  async draw(
    host: HTMLElement,
    pageNumber: number,
    options: { scale: number; fit: FitMode },
  ): Promise<void> {
    const document = this.document;
    if (!document || pageNumber < 1 || pageNumber > document.numPages) {
      host.replaceChildren();
      return;
    }
    const token = ++this.renderToken;
    const page = await document.getPage(pageNumber);
    if (token !== this.renderToken) {
      return;
    }
    const base = page.getViewport({ scale: 1 });
    const availableWidth = Math.max(host.clientWidth - 48, 120);
    const availableHeight = Math.max(host.clientHeight - 48, 120);
    let scale = options.scale;
    if (options.fit === "width") {
      scale = availableWidth / base.width;
    } else if (options.fit === "page") {
      scale = Math.min(availableWidth / base.width, availableHeight / base.height);
    }
    // Supersample so both type and strokes anti-alias like Acrobat (min 2×).
    const outputScale = Math.max(window.devicePixelRatio || 1, 2);
    const viewport = page.getViewport({ scale: scale * outputScale });
    const cssWidth = viewport.width / outputScale;
    const cssHeight = viewport.height / outputScale;
    const canvas = window.document.createElement("canvas");
    canvas.width = Math.max(1, Math.floor(viewport.width));
    canvas.height = Math.max(1, Math.floor(viewport.height));
    canvas.style.width = `${cssWidth}px`;
    canvas.style.height = `${cssHeight}px`;
    canvas.style.imageRendering = "auto";
    canvas.className = "pdf-page bg-white shadow-sm";
    const context = canvas.getContext("2d", {
      alpha: false,
      colorSpace: "srgb",
    });
    if (!context) {
      return;
    }
    context.imageSmoothingEnabled = true;
    context.imageSmoothingQuality = "high";
    await page.render({
      canvasContext: context,
      viewport,
      canvas,
      intent: "display",
    }).promise;
    if (token !== this.renderToken) {
      return;
    }
    const frame = window.document.createElement("div");
    frame.className =
      "flex min-h-full min-w-full items-center justify-center p-6 box-border";
    frame.append(canvas);
    host.replaceChildren(frame);
  }
}

export function zoomPercent(scale: number): number {
  return Math.round(scale * 100);
}
