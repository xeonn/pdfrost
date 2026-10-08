export type IssueSeverity = "error" | "warning" | "info";

export type ConsoleIssue = {
  id: string;
  severity: IssueSeverity;
  message: string;
  /** Engine path such as `title/staticText`, when available. */
  location?: string;
  /** 1-based line in Source, when known. */
  line?: number;
  source: "xml" | "engine" | "render";
};

export type ValidateResult = {
  ok: boolean;
  error: string | null;
  warnings: { tag: string; location: string }[];
};

/** Pull a 1-based line number from common XML/parser error text. */
export function lineFromMessage(text: string): number | undefined {
  const match = text.match(/(?:line|row)\s*[:=]?\s*(\d+)/i);
  if (!match) {
    return undefined;
  }
  const line = Number(match[1]);
  return Number.isFinite(line) && line > 0 ? line : undefined;
}

export function issuesFromValidate(result: ValidateResult): ConsoleIssue[] {
  const issues: ConsoleIssue[] = [];
  if (result.error) {
    issues.push({
      id: `error:${result.error}`,
      severity: "error",
      message: result.error,
      line: lineFromMessage(result.error),
      source: "xml",
    });
  }
  for (const warning of result.warnings) {
    issues.push({
      id: `warn:${warning.tag}:${warning.location}`,
      severity: "warning",
      message: warning.tag,
      location: warning.location,
      source: "engine",
    });
  }
  return issues;
}
