/** Pretty-print XML/JRXML while preserving the XML declaration when present. */
export function formatXml(raw: string): string {
  const text = raw.replace(/^\uFEFF/, "");
  const declaration = text.match(/^\s*<\?xml[^?]*\?>\s*/i)?.[0]?.trim() ?? "";
  const body = declaration ? text.slice(text.indexOf(declaration) + declaration.length) : text;
  const parser = new DOMParser();
  const doc = parser.parseFromString(body.trim(), "application/xml");
  const parseError = doc.querySelector("parsererror");
  if (parseError) {
    throw new Error(parseError.textContent?.trim() || "invalid XML");
  }
  const root = doc.documentElement;
  if (!root) {
    throw new Error("invalid XML");
  }
  const pretty = serializeNode(root, 0).trimEnd() + "\n";
  return declaration ? `${declaration}\n${pretty}` : pretty;
}

function serializeNode(node: Node, depth: number): string {
  const pad = "  ".repeat(depth);
  if (node.nodeType === Node.TEXT_NODE) {
    const value = (node.textContent ?? "").trim();
    return value ? `${pad}${escapeText(value)}\n` : "";
  }
  if (node.nodeType === Node.COMMENT_NODE) {
    return `${pad}<!--${node.textContent ?? ""}-->\n`;
  }
  if (node.nodeType !== Node.ELEMENT_NODE) {
    return "";
  }
  const el = node as Element;
  const name = el.tagName;
  const attrs = Array.from(el.attributes)
    .map((attr) => ` ${attr.name}="${escapeAttr(attr.value)}"`)
    .join("");
  const children = Array.from(el.childNodes).filter((child) => {
    if (child.nodeType === Node.TEXT_NODE) {
      return (child.textContent ?? "").trim().length > 0;
    }
    return child.nodeType === Node.ELEMENT_NODE || child.nodeType === Node.COMMENT_NODE;
  });
  if (!children.length) {
    return `${pad}<${name}${attrs}/>\n`;
  }
  const onlyText =
    children.length === 1 && children[0]?.nodeType === Node.TEXT_NODE
      ? (children[0].textContent ?? "").trim()
      : "";
  if (onlyText && children.length === 1) {
    return `${pad}<${name}${attrs}>${escapeText(onlyText)}</${name}>\n`;
  }
  let out = `${pad}<${name}${attrs}>\n`;
  for (const child of children) {
    out += serializeNode(child, depth + 1);
  }
  out += `${pad}</${name}>\n`;
  return out;
}

function escapeText(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

function escapeAttr(value: string): string {
  return escapeText(value).replace(/"/g, "&quot;");
}
