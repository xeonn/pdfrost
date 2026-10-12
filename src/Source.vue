<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { xml } from "@codemirror/lang-xml";
import { bracketMatching, foldGutter, foldKeymap, indentOnInput } from "@codemirror/language";
import { linter, lintGutter, type Diagnostic } from "@codemirror/lint";
import { gotoLine, highlightSelectionMatches, search, searchKeymap } from "@codemirror/search";
import { EditorState } from "@codemirror/state";
import { oneDark } from "@codemirror/theme-one-dark";
import { EditorView, keymap, lineNumbers } from "@codemirror/view";
import { FwbAlert, FwbButton, FwbSpinner } from "flowbite-vue";
import {
  issuesFromValidate,
  lineFromMessage,
  type ConsoleIssue,
  type ValidateResult,
} from "./console";
import Icon from "./Icon.vue";
import { formatXml } from "./xmlFormat";

const props = defineProps<{ templatePath: string; active?: boolean }>();
const emit = defineEmits<{
  saved: [path: string];
  issues: [issues: ConsoleIssue[]];
}>();

const hostEl = ref<HTMLElement | null>(null);
const source = ref("");
const savedSource = ref("");
const error = ref("");
const status = ref("");
const loading = ref(false);
const saving = ref(false);
const editorReady = ref(false);
let view: EditorView | null = null;
let latestIssues: ConsoleIssue[] = [];

const dirty = computed(() => source.value !== savedSource.value);
const canSave = computed(
  () => dirty.value && !!props.templatePath && !loading.value && !saving.value && editorReady.value,
);

function confirmDiscardIfDirty(messageText = "Discard unsaved Source changes?"): boolean {
  syncFromView();
  if (!dirty.value) {
    return true;
  }
  return window.confirm(messageText);
}

function jumpTo(issue: ConsoleIssue): void {
  if (!view) {
    return;
  }
  if (issue.line) {
    const line = view.state.doc.line(Math.min(issue.line, view.state.doc.lines));
    view.dispatch({
      selection: { anchor: line.from },
      scrollIntoView: true,
    });
    view.focus();
    return;
  }
  if (issue.location) {
    const leaf = issue.location.split("/").filter(Boolean).pop() ?? issue.location;
    const tag = leaf.replace(/\[\d+\]$/, "");
    const needle = `<${tag}`;
    const text = view.state.doc.toString();
    const index = text.indexOf(needle);
    if (index >= 0) {
      view.dispatch({
        selection: { anchor: index, head: index + needle.length },
        scrollIntoView: true,
      });
      view.focus();
    }
  }
}

function isDirty(): boolean {
  return dirty.value;
}

async function reloadFromDisk(): Promise<void> {
  await load();
}

defineExpose({
  confirmDiscardIfDirty,
  dirty,
  jumpTo,
  isDirty,
  reloadFromDisk,
  discardChanges: reloadFromDisk,
  save,
  saveAs,
});

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

function syncFromView(): void {
  if (!view) {
    return;
  }
  source.value = view.state.doc.toString();
}

function setEditorText(text: string): void {
  if (!view) {
    return;
  }
  view.dispatch({
    changes: { from: 0, to: view.state.doc.length, insert: text },
  });
}

function destroyEditor(): void {
  view?.destroy();
  view = null;
  editorReady.value = false;
}

function publishIssues(issues: ConsoleIssue[]): void {
  latestIssues = issues;
  emit("issues", issues);
}

async function runValidate(text: string): Promise<Diagnostic[]> {
  const result = await invoke<ValidateResult>("validate_template", { contents: text });
  const issues = issuesFromValidate(result);
  publishIssues(issues);
  const diagnostics: Diagnostic[] = [];
  if (result.error) {
    const line = lineFromMessage(result.error) ?? 1;
    const docLine = Math.min(Math.max(line, 1), text.split(/\r?\n/).length || 1);
    let from = 0;
    let to = Math.min(1, text.length);
    if (view) {
      const cmLine = view.state.doc.line(docLine);
      from = cmLine.from;
      to = cmLine.to;
    } else {
      const lines = text.split(/\r?\n/);
      let offset = 0;
      for (let i = 0; i < docLine - 1; i += 1) {
        offset += (lines[i]?.length ?? 0) + 1;
      }
      from = offset;
      to = offset + (lines[docLine - 1]?.length ?? 0);
    }
    diagnostics.push({
      from,
      to: Math.max(to, from + 1),
      severity: "error",
      message: result.error,
      source: "jrxml",
    });
  }
  for (const warning of result.warnings) {
    const leaf = warning.location.split("/").filter(Boolean).pop() ?? warning.tag;
    const tag = leaf.replace(/\[\d+\]$/, "");
    const needle = `<${tag}`;
    const index = text.indexOf(needle);
    const from = index >= 0 ? index : 0;
    const to = index >= 0 ? index + needle.length : Math.min(text.length, 1);
    diagnostics.push({
      from,
      to: Math.max(to, from + 1),
      severity: "warning",
      message: `${warning.tag} at ${warning.location}`,
      source: "jrxml",
    });
  }
  return diagnostics;
}

function createEditor(text: string): void {
  if (!hostEl.value) {
    return;
  }
  destroyEditor();
  view = new EditorView({
    parent: hostEl.value,
    state: EditorState.create({
      doc: text,
      extensions: [
        lineNumbers(),
        lintGutter(),
        foldGutter(),
        history(),
        indentOnInput(),
        bracketMatching(),
        highlightSelectionMatches(),
        search({ top: true }),
        xml(),
        oneDark,
        linter(
          async (editor) => runValidate(editor.state.doc.toString()),
          { delay: 600 },
        ),
        EditorView.lineWrapping,
        keymap.of([
          indentWithTab,
          ...defaultKeymap,
          ...historyKeymap,
          ...searchKeymap,
          ...foldKeymap,
          {
            key: "Mod-s",
            run: () => {
              void save();
              return true;
            },
          },
          {
            key: "Mod-Shift-f",
            run: () => {
              formatDocument();
              return true;
            },
          },
          {
            key: "Mod-Alt-g",
            run: gotoLine,
          },
        ]),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            syncFromView();
            status.value = "";
          }
        }),
        EditorView.theme({
          "&": {
            height: "100%",
            fontSize: "13px",
          },
          ".cm-scroller": {
            fontFamily:
              'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
            lineHeight: "1.55",
          },
          "&.cm-focused": {
            outline: "none",
          },
          ".cm-panels": {
            fontSize: "13px",
            lineHeight: "1.4",
          },
          ".cm-panels.cm-panels-top": {
            borderBottom: "1px solid #334155",
          },
          ".cm-panel": {
            padding: "8px 10px",
            gap: "6px",
          },
          ".cm-panel input[type=checkbox]": {
            width: "14px",
            height: "14px",
            marginRight: "4px",
          },
          ".cm-textfield": {
            fontSize: "13px",
            padding: "6px 10px",
            minHeight: "32px",
            borderRadius: "6px",
          },
          ".cm-button": {
            fontSize: "13px",
            padding: "6px 12px",
            minHeight: "32px",
            borderRadius: "6px",
            cursor: "pointer",
          },
          ".cm-panel label": {
            fontSize: "13px",
            marginLeft: "4px",
            marginRight: "8px",
          },
        }),
      ],
    }),
  });
  source.value = text;
  savedSource.value = text;
  editorReady.value = true;
  void runValidate(text);
}

async function load(): Promise<void> {
  if (!props.templatePath) {
    source.value = "";
    savedSource.value = "";
    error.value = "";
    status.value = "";
    publishIssues([]);
    destroyEditor();
    return;
  }
  loading.value = true;
  error.value = "";
  status.value = "";
  try {
    const text = await invoke<string>("read_template", { path: props.templatePath });
    await nextTick();
    if (view) {
      setEditorText(text);
      source.value = text;
      savedSource.value = text;
      void runValidate(text);
    } else {
      createEditor(text);
    }
    view?.requestMeasure();
  } catch (err) {
    source.value = "";
    savedSource.value = "";
    error.value = message(err);
    publishIssues([
      {
        id: `load:${message(err)}`,
        severity: "error",
        message: message(err),
        source: "xml",
      },
    ]);
    destroyEditor();
  } finally {
    loading.value = false;
  }
}

async function save(): Promise<void> {
  if (!canSave.value || !props.templatePath) {
    return;
  }
  syncFromView();
  saving.value = true;
  error.value = "";
  status.value = "";
  try {
    const diagnostics = await runValidate(source.value);
    const hasError = diagnostics.some((item) => item.severity === "error");
    if (hasError) {
      error.value = "Fix validation errors before saving.";
      return;
    }
    await invoke("write_template", {
      path: props.templatePath,
      contents: source.value,
    });
    savedSource.value = source.value;
    status.value = "Saved";
    emit("saved", props.templatePath);
  } catch (err) {
    error.value = message(err);
  } finally {
    saving.value = false;
  }
}

async function saveAs(): Promise<void> {
  if (!props.templatePath || saving.value) {
    return;
  }
  syncFromView();
  const diagnostics = await runValidate(source.value);
  if (diagnostics.some((item) => item.severity === "error")) {
    error.value = "Fix validation errors before saving.";
    return;
  }
  const picked = await saveDialog({
    filters: [{ name: "JRXML", extensions: ["jrxml"] }],
    defaultPath: props.templatePath.split(/[/\\]/).pop() || "Report.jrxml",
  });
  if (typeof picked !== "string") {
    return;
  }
  const path = picked.toLowerCase().endsWith(".jrxml") ? picked : `${picked}.jrxml`;
  saving.value = true;
  error.value = "";
  status.value = "";
  try {
    await invoke("write_template", { path, contents: source.value });
    savedSource.value = source.value;
    status.value = "Saved";
    emit("saved", path);
  } catch (err) {
    error.value = message(err);
  } finally {
    saving.value = false;
  }
}

async function reload(): Promise<void> {
  if (!confirmDiscardIfDirty("Discard unsaved changes and reload from disk?")) {
    return;
  }
  await load();
}

function formatDocument(): void {
  if (!view) {
    return;
  }
  syncFromView();
  try {
    const formatted = formatXml(source.value);
    if (formatted !== source.value) {
      setEditorText(formatted);
      syncFromView();
      status.value = "Formatted";
    } else {
      status.value = "Already formatted";
    }
    error.value = "";
    void runValidate(formatted);
  } catch (err) {
    error.value = message(err);
    publishIssues([
      {
        id: `format:${message(err)}`,
        severity: "error",
        message: message(err),
        line: lineFromMessage(message(err)),
        source: "xml",
      },
      ...latestIssues.filter((issue) => issue.severity !== "error"),
    ]);
  }
}

function openGotoLine(): void {
  if (!view) {
    return;
  }
  gotoLine(view);
  view.focus();
}

onMounted(() => {
  void load();
});

onBeforeUnmount(() => {
  destroyEditor();
});

watch(
  () => props.templatePath,
  (next, previous) => {
    if (next === previous) {
      return;
    }
    void load();
  },
);

watch(
  () => props.active,
  (active) => {
    if (active) {
      view?.requestMeasure();
      if (latestIssues.length) {
        emit("issues", latestIssues);
      }
    }
  },
);
</script>

<template>
  <section class="flex h-full min-h-0 flex-col bg-slate-950">
    <div
      class="flex shrink-0 items-center gap-2 border-b border-slate-800 bg-slate-900 px-3 py-2"
    >
      <FwbButton size="xs" color="alternative" :disabled="loading || saving" @click="reload">
        <span class="inline-flex items-center gap-1.5">
          <Icon name="refresh" />
          Reload
        </span>
      </FwbButton>
      <FwbButton size="xs" color="alternative" :disabled="!editorReady || loading" @click="formatDocument">
        Format
      </FwbButton>
      <FwbButton size="xs" color="alternative" :disabled="!editorReady || loading" @click="openGotoLine">
        Go to line
      </FwbButton>
      <span v-if="dirty" class="text-xs font-medium text-amber-400">Unsaved changes</span>
      <span v-else-if="status" class="text-xs text-gray-400">{{ status }}</span>
      <span class="ml-auto text-xs text-gray-500">⌘F · ⌥⌘G · ⌘⇧F · ⌘S</span>
    </div>

    <div class="relative min-h-0 flex-1 bg-slate-900">
      <div
        v-if="loading"
        class="absolute inset-0 z-10 flex items-center justify-center bg-slate-950/80"
      >
        <FwbSpinner size="8" color="blue" />
      </div>
      <FwbAlert
        v-if="error"
        type="danger"
        class="absolute inset-x-0 top-0 z-10 m-3"
      >
        {{ error }}
      </FwbAlert>
      <div ref="hostEl" class="h-full min-h-0 overflow-hidden" />
    </div>
  </section>
</template>
