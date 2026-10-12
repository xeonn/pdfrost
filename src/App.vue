<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  FwbAlert,
  FwbBadge,
  FwbButton,
  FwbButtonGroup,
  FwbP,
  FwbSpinner,
} from "flowbite-vue";
import ConsolePanel from "./Console.vue";
import DataView from "./DataView.vue";
import Designer from "./Designer.vue";
import Icon from "./Icon.vue";
import Source from "./Source.vue";
import { type ConsoleIssue, lineFromMessage } from "./console";
import { FitMode, PdfStage } from "./pdf";
import { blankReport } from "./report";
import type { Rendered } from "./types";

type Mode = "design" | "source" | "data" | "preview";

const mode = ref<Mode>("preview");
const modes: { id: Mode; label: string }[] = [
  { id: "design", label: "Design" },
  { id: "source", label: "Source" },
  { id: "preview", label: "Preview" },
];
const templatePath = ref("");
const documentPath = ref("");
const error = ref("");
const loading = ref(false);
const hasDocument = ref(false);
const consoleIssues = ref<ConsoleIssue[]>([]);
const sourceRef = ref<{
  confirmDiscardIfDirty: (message?: string) => boolean;
  jumpTo: (issue: ConsoleIssue) => void;
  isDirty: () => boolean;
  discardChanges: () => Promise<void>;
  reloadFromDisk: () => Promise<void>;
  save: () => Promise<void>;
  saveAs: () => Promise<void>;
} | null>(null);
const designRef = ref<{
  confirmDiscardIfDirty: (message?: string) => boolean;
  discardChanges: () => void;
  isDirty: () => boolean;
  save: () => Promise<void>;
  saveAs: () => Promise<void>;
  appendFields: (id: string, fields: { name: string; className: string; jsonPath: string }[], query?: string | null) => void;
  changeField: (
    id: string,
    index: number,
    patch: { name?: string; className?: string; jsonPath?: string | null },
  ) => void;
  addField: (id: string) => void;
  removeField: (id: string, index: number) => void;
} | null>(null);
const fileMenuOpen = ref(false);
const fileCanSave = ref(false);
const fileMenuEl = ref<HTMLElement | null>(null);
const datasets = ref<{ id: string; name: string; listPath: string | null; fields: { name: string; className: string; description: string | null; jsonPath: string | null }[] }[]>([]);
let renderGeneration = 0;

const stage = new PdfStage();
const stageEl = ref<HTMLElement | null>(null);
const pageNumber = ref(1);
const pageCount = ref(0);
const scale = ref(1);
const fit = ref<FitMode>("none");
const ZOOM_STEPS = [0.5, 0.75, 1, 1.25, 1.5, 2];

const loaded = computed(() => hasDocument.value && !error.value);
const dataBadge = computed(() => (documentPath.value ? "Filled" : "Empty data"));
const zoomOptions = ZOOM_STEPS.map((step) => ({
  name: `${Math.round(step * 100)}%`,
  value: String(step),
}));
const zoomModel = computed({
  get: () => (fit.value === "none" ? String(scale.value) : String(scale.value)),
  set: (value: string) => {
    const next = Number(value);
    if (!Number.isFinite(next) || next <= 0) {
      return;
    }
    fit.value = "none";
    scale.value = next;
  },
});

async function chooseTemplate(): Promise<void> {
  if (
    sourceRef.value &&
    !sourceRef.value.confirmDiscardIfDirty(
      "Discard unsaved Source changes and open another template?",
    )
  ) {
    return;
  }
  if (
    designRef.value &&
    !designRef.value.confirmDiscardIfDirty(
      "Discard unsaved Design changes and open another template?",
    )
  ) {
    return;
  }
  const picked = await chooseFile("JRXML", ["jrxml"]);
  if (!picked) {
    return;
  }
  templatePath.value = picked;
  documentPath.value = "";
  mode.value = "design";
  clearPreview();
}

async function openTemplate(path: string): Promise<void> {
  if (!path || path === templatePath.value) {
    return;
  }
  if (
    sourceRef.value &&
    !sourceRef.value.confirmDiscardIfDirty("Discard unsaved Source changes and open the subreport?")
  ) {
    return;
  }
  if (
    designRef.value &&
    !designRef.value.confirmDiscardIfDirty("Discard unsaved Design changes and open the subreport?")
  ) {
    return;
  }
  templatePath.value = path;
  mode.value = "design";
  if (documentPath.value) {
    await render();
  }
}

async function chooseDocument(stayOnDesign?: boolean): Promise<void> {
  if (!templatePath.value) {
    return;
  }
  const picked = await chooseFile("JSON", ["json"]);
  if (!picked) {
    return;
  }
  documentPath.value = picked;
  if (stayOnDesign !== true) {
    mode.value = "data";
  }
  await render();
}

async function chooseFile(name: string, extensions: string[]): Promise<string | null> {
  try {
    const picked = await open({ multiple: false, filters: [{ name, extensions }] });
    return typeof picked === "string" ? picked : null;
  } catch (err) {
    error.value = message(err);
    hasDocument.value = false;
    stage.clear();
    pageCount.value = 0;
    return null;
  }
}

function setConsoleFromRender(warnings: Rendered["warnings"], renderError?: string): void {
  const issues: ConsoleIssue[] = [];
  if (renderError) {
    issues.push({
      id: `render:${renderError}`,
      severity: "error",
      message: renderError,
      line: lineFromMessage(renderError),
      source: "render",
    });
  }
  for (const warning of warnings) {
    issues.push({
      id: `render-warn:${warning.tag}:${warning.location}`,
      severity: "warning",
      message: warning.tag,
      location: warning.location,
      source: "engine",
    });
  }
  consoleIssues.value = issues;
}

async function render(): Promise<void> {
  if (!templatePath.value) {
    return;
  }
  const generation = ++renderGeneration;
  loading.value = true;
  error.value = "";
  try {
    const rendered = await invoke<Rendered>("render_report", {
      templatePath: templatePath.value,
      documentPath: documentPath.value || null,
    });
    if (generation !== renderGeneration) {
      return;
    }
    const count = await stage.load(Uint8Array.from(rendered.pdf));
    if (generation !== renderGeneration) {
      return;
    }
    pageCount.value = count;
    pageNumber.value = 1;
    hasDocument.value = true;
    setConsoleFromRender(rendered.warnings);
    await nextTick();
    if (generation !== renderGeneration) {
      return;
    }
    await paint();
  } catch (err) {
    if (generation !== renderGeneration) {
      return;
    }
    stage.clear();
    hasDocument.value = false;
    pageCount.value = 0;
    error.value = message(err);
    setConsoleFromRender([], message(err));
  } finally {
    if (generation === renderGeneration) {
      loading.value = false;
    }
  }
}

async function selectMode(next: Mode): Promise<void> {
  if (next === mode.value) {
    return;
  }
  if (mode.value === "source" && next !== "source" && sourceRef.value) {
    if (!sourceRef.value.confirmDiscardIfDirty("Discard unsaved Source changes?")) {
      return;
    }
    if (sourceRef.value.isDirty()) {
      await sourceRef.value.discardChanges();
    }
  }
  if (mode.value === "design" && next === "source" && designRef.value) {
    if (!designRef.value.confirmDiscardIfDirty("Discard unsaved Design changes?")) {
      return;
    }
    if (designRef.value.isDirty()) {
      designRef.value.discardChanges();
    }
  }
  mode.value = next;
}

async function createReport(): Promise<void> {
  if (
    sourceRef.value &&
    !sourceRef.value.confirmDiscardIfDirty("Discard unsaved Source changes and create a new report?")
  ) {
    return;
  }
  if (
    designRef.value &&
    !designRef.value.confirmDiscardIfDirty("Discard unsaved Design changes and create a new report?")
  ) {
    return;
  }
  const picked = await save({
    filters: [{ name: "JRXML", extensions: ["jrxml"] }],
    defaultPath: "Report.jrxml",
  });
  if (typeof picked !== "string") {
    return;
  }
  const path = picked.toLowerCase().endsWith(".jrxml") ? picked : `${picked}.jrxml`;
  try {
    await invoke("save_report", { path, report: blankReport() });
  } catch (err) {
    error.value = message(err);
    return;
  }
  templatePath.value = path;
  documentPath.value = "";
  mode.value = "design";
  clearPreview();
}

function onSourceSaved(path?: string): void {
  if (path && path !== templatePath.value) {
    templatePath.value = path;
  }
  if (documentPath.value) {
    void render();
  }
}

async function onDesignSaved(path: string): Promise<void> {
  const changed = path !== templatePath.value;
  templatePath.value = path;
  if (!changed) {
    await sourceRef.value?.reloadFromDisk();
  }
  if (documentPath.value) {
    await render();
  }
}

function clearPreview(): void {
  stage.clear();
  hasDocument.value = false;
  pageCount.value = 0;
  error.value = "";
  consoleIssues.value = [];
}

function toggleFileMenu(): void {
  if (fileMenuOpen.value) {
    fileMenuOpen.value = false;
    return;
  }
  fileCanSave.value =
    mode.value === "source" ? !!sourceRef.value?.isDirty() : !!designRef.value?.isDirty();
  fileMenuOpen.value = true;
}

function closeFileMenu(): void {
  fileMenuOpen.value = false;
}

async function saveFromMenu(): Promise<void> {
  closeFileMenu();
  if (mode.value === "source") {
    await sourceRef.value?.save();
    return;
  }
  await designRef.value?.save();
}

async function saveAsFromMenu(): Promise<void> {
  closeFileMenu();
  if (mode.value === "source") {
    await sourceRef.value?.saveAs();
    return;
  }
  await designRef.value?.saveAs();
}

function onWindowKeydown(event: KeyboardEvent): void {
  if (event.key === "Escape") {
    fileMenuOpen.value = false;
  }
}

function onWindowPointerDown(event: PointerEvent): void {
  if (!fileMenuOpen.value) {
    return;
  }
  const target = event.target;
  if (target instanceof Node && fileMenuEl.value?.contains(target)) {
    return;
  }
  fileMenuOpen.value = false;
}

function onSourceIssues(issues: ConsoleIssue[]): void {
  consoleIssues.value = issues;
}

function onConsoleJump(issue: ConsoleIssue): void {
  mode.value = "source";
  nextTick(() => {
    sourceRef.value?.jumpTo(issue);
  });
}

async function paint(): Promise<void> {
  if (!stageEl.value || !hasDocument.value) {
    return;
  }
  await stage.draw(stageEl.value, pageNumber.value, {
    scale: scale.value,
    fit: fit.value,
  });
}

function goFirst(): void {
  pageNumber.value = 1;
}

function goPrev(): void {
  pageNumber.value = Math.max(1, pageNumber.value - 1);
}

function goNext(): void {
  pageNumber.value = Math.min(pageCount.value, pageNumber.value + 1);
}

function goLast(): void {
  pageNumber.value = pageCount.value;
}

function zoomOut(): void {
  fit.value = "none";
  const index = ZOOM_STEPS.findIndex((step) => step >= scale.value - 0.001);
  const current = index === -1 ? ZOOM_STEPS.length - 1 : index;
  scale.value = ZOOM_STEPS[Math.max(0, current - 1)] ?? 0.5;
}

function zoomIn(): void {
  fit.value = "none";
  const index = ZOOM_STEPS.findIndex((step) => step > scale.value + 0.001);
  scale.value = ZOOM_STEPS[index === -1 ? ZOOM_STEPS.length - 1 : index] ?? 2;
}

function onPreviewWheel(event: WheelEvent): void {
  if (mode.value !== "preview" || !loaded.value || (!event.ctrlKey && !event.metaKey)) {
    return;
  }
  event.preventDefault();
  if (event.deltaY === 0) {
    return;
  }
  if (event.deltaY > 0) {
    zoomOut();
  } else {
    zoomIn();
  }
}

function fitWidth(): void {
  fit.value = "width";
}

function fitPage(): void {
  fit.value = "page";
}

function printPreview(): void {
  if (!loaded.value) {
    return;
  }
  stage.print();
}

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

function fileName(path: string): string {
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}

watch([pageNumber, scale, fit], () => {
  void paint();
});

watch(mode, async (next) => {
  if (next === "preview" && hasDocument.value) {
    await nextTick();
    await paint();
  }
});

onMounted(() => {
  window.addEventListener("pointerdown", onWindowPointerDown);
  window.addEventListener("keydown", onWindowKeydown);
  const observer = new ResizeObserver(() => {
    if (fit.value !== "none") {
      void paint();
    }
  });
  watch(
    stageEl,
    (el, previous) => {
      if (previous) {
        observer.unobserve(previous);
        previous.removeEventListener("wheel", onPreviewWheel);
      }
      if (el) {
        observer.observe(el);
        el.addEventListener("wheel", onPreviewWheel, { passive: false });
      }
    },
    { immediate: true },
  );
});

onBeforeUnmount(() => {
  window.removeEventListener("pointerdown", onWindowPointerDown);
  window.removeEventListener("keydown", onWindowKeydown);
});
</script>

<template>
  <div class="flex h-screen flex-col bg-slate-950 text-gray-100">
    <header
      class="titlebar relative z-20 flex h-14 shrink-0 items-center border-b border-slate-800 bg-slate-900 pr-4 pl-28"
    >
      <!--
        data-tauri-drag-region does not inherit to children. A full-size layer
        receives drags; controls sit above it with pointer-events-auto.
      -->
      <div data-tauri-drag-region class="absolute inset-0 z-0"></div>
      <div
        class="relative z-10 flex h-full min-w-0 flex-1 items-center gap-4 pointer-events-none"
      >
        <div class="flex h-full min-w-0 items-center gap-3 pointer-events-auto">
          <span
            class="select-none text-[15px] font-medium leading-none tracking-tight text-white"
          >
            pdfrost
          </span>
          <div class="flex items-center gap-2">
            <div ref="fileMenuEl" class="relative">
              <FwbButton size="xs" @click="toggleFileMenu">File</FwbButton>
              <div
                v-if="fileMenuOpen"
                class="absolute top-full left-0 z-50 mt-1 w-44 rounded-md border border-slate-700 bg-slate-900 py-1 shadow-lg"
              >
                <button type="button" class="block w-full px-3 py-1.5 text-left text-sm text-gray-100 hover:bg-slate-800" @click="closeFileMenu(); createReport()">
                  New Report
                </button>
                <button type="button" class="block w-full px-3 py-1.5 text-left text-sm text-gray-100 hover:bg-slate-800" @click="closeFileMenu(); chooseTemplate()">
                  Open Report…
                </button>
                <button
                  type="button"
                  class="block w-full px-3 py-1.5 text-left text-sm text-gray-100 hover:bg-slate-800 disabled:cursor-not-allowed disabled:text-gray-500 disabled:hover:bg-transparent"
                  :disabled="!fileCanSave"
                  @click="saveFromMenu"
                >
                  Save
                </button>
                <button
                  type="button"
                  class="block w-full px-3 py-1.5 text-left text-sm text-gray-100 hover:bg-slate-800 disabled:cursor-not-allowed disabled:text-gray-500 disabled:hover:bg-transparent"
                  :disabled="!templatePath"
                  @click="saveAsFromMenu"
                >
                  Save As…
                </button>
                <div class="my-1 border-t border-slate-800"></div>
                <button
                  type="button"
                  class="block w-full px-3 py-1.5 text-left text-sm text-gray-100 hover:bg-slate-800 disabled:cursor-not-allowed disabled:text-gray-500 disabled:hover:bg-transparent"
                  :disabled="!loaded"
                  @click="closeFileMenu(); printPreview()"
                >
                  Print
                </button>
              </div>
            </div>
            <FwbButton
              size="xs"
              :color="mode === 'data' ? 'default' : 'alternative'"
              :disabled="!templatePath"
              @click="selectMode('data')"
            >
              Dataset
            </FwbButton>
            <FwbBadge v-if="templatePath" size="xs" :type="documentPath ? 'green' : 'indigo'">
              {{ dataBadge }}
            </FwbBadge>
            <FwbSpinner v-if="loading" size="4" color="blue" />
          </div>
        </div>

        <div class="flex min-h-8 min-w-0 flex-1 items-center justify-center px-2">
          <span
            v-if="templatePath"
            class="max-w-md truncate rounded-md bg-slate-800 px-3 py-1 text-xs font-medium text-gray-200 ring-1 ring-slate-700"
            :title="templatePath"
          >
            {{ fileName(templatePath) }}
          </span>
        </div>

        <div class="flex shrink-0 items-center gap-1.5 pointer-events-auto">
          <button
            v-for="tab in modes"
            :key="tab.id"
            type="button"
            class="rounded-md px-3 py-1.5 text-sm font-medium text-white disabled:cursor-not-allowed disabled:opacity-40"
            :class="
              mode === tab.id
                ? 'bg-orange-500 hover:bg-orange-400'
                : 'bg-teal-700 hover:bg-teal-600'
            "
            :disabled="!templatePath"
            @click="selectMode(tab.id)"
          >
            {{ tab.label }}
          </button>
        </div>
      </div>
    </header>

    <main class="flex min-h-0 flex-1 flex-col bg-slate-950">
      <div
        v-if="!templatePath"
        class="flex h-full flex-col items-center justify-center gap-4 px-6"
      >
        <FwbP class="text-center text-gray-400">
          Open a report from the File menu, or create a new one.
        </FwbP>
        <div class="flex items-center gap-2">
          <FwbButton @click="chooseTemplate">
            <span class="inline-flex items-center gap-1.5">
              <Icon name="folder" />
              Open Report
            </span>
          </FwbButton>
          <FwbButton color="alternative" @click="createReport">New Report</FwbButton>
        </div>
      </div>

      <template v-else>
        <div
          v-show="mode === 'preview' && documentPath"
          class="flex h-10 shrink-0 items-center justify-center gap-2 border-b border-slate-800 bg-slate-900 px-3"
          :class="{ 'opacity-40': !loaded }"
        >
          <FwbButtonGroup>
            <FwbButton
              size="sm"
              color="alternative"
              title="First page"
              :disabled="!loaded || pageNumber <= 1"
              @click="goFirst"
            >
              <Icon name="chevrons-left" />
            </FwbButton>
            <FwbButton
              size="sm"
              color="alternative"
              title="Previous page"
              :disabled="!loaded || pageNumber <= 1"
              @click="goPrev"
            >
              <Icon name="chevron-left" />
            </FwbButton>
            <FwbButton
              size="sm"
              color="alternative"
              class="pointer-events-none min-w-28"
              :disabled="!loaded"
            >
              {{ loaded ? `Page ${pageNumber} of ${pageCount}` : "Page —" }}
            </FwbButton>
            <FwbButton
              size="sm"
              color="alternative"
              title="Next page"
              :disabled="!loaded || pageNumber >= pageCount"
              @click="goNext"
            >
              <Icon name="chevron-right" />
            </FwbButton>
            <FwbButton
              size="sm"
              color="alternative"
              title="Last page"
              :disabled="!loaded || pageNumber >= pageCount"
              @click="goLast"
            >
              <Icon name="chevrons-right" />
            </FwbButton>
          </FwbButtonGroup>

          <FwbButtonGroup>
            <FwbButton
              size="sm"
              color="alternative"
              title="Zoom out"
              :disabled="!loaded"
              @click="zoomOut"
            >
              <Icon name="zoom-out" />
            </FwbButton>
            <select
              class="h-8 min-w-20 border border-gray-600 bg-gray-700 px-2 text-sm text-white outline-none disabled:cursor-not-allowed disabled:opacity-50"
              :value="fit === 'none' ? String(scale) : ''"
              :disabled="!loaded"
              @change="zoomModel = ($event.target as HTMLSelectElement).value"
            >
              <option v-if="fit !== 'none'" value="" disabled>
                {{ fit === "width" ? "Width" : "Page" }}
              </option>
              <option v-for="option in zoomOptions" :key="option.value" :value="option.value">
                {{ option.name }}
              </option>
            </select>
            <FwbButton
              size="sm"
              color="alternative"
              title="Zoom in"
              :disabled="!loaded"
              @click="zoomIn"
            >
              <Icon name="zoom-in" />
            </FwbButton>
          </FwbButtonGroup>

          <FwbButtonGroup>
            <FwbButton
              size="sm"
              :color="fit === 'width' ? 'default' : 'alternative'"
              :disabled="!loaded"
              @click="fitWidth"
            >
              Width
            </FwbButton>
            <FwbButton
              size="sm"
              :color="fit === 'page' ? 'default' : 'alternative'"
              :disabled="!loaded"
              @click="fitPage"
            >
              Page
            </FwbButton>
          </FwbButtonGroup>

          <FwbButton
            size="sm"
            color="alternative"
            title="Print"
            :disabled="!loaded"
            @click="printPreview"
          >
            <span class="inline-flex items-center gap-1.5">
              <Icon name="print" />
              Print
            </span>
          </FwbButton>
        </div>

        <Designer
          ref="designRef"
          v-show="mode === 'design'"
          :template-path="templatePath"
          :document-path="documentPath"
          :active="mode === 'design'"
          class="min-h-0 flex-1"
          @saved="onDesignSaved"
          @open-template="openTemplate"
          @open-dataset="selectMode('data')"
          @datasets="datasets = $event"
        />
        <Source
          ref="sourceRef"
          v-show="mode === 'source'"
          :template-path="templatePath"
          :active="mode === 'source'"
          class="min-h-0 flex-1"
          @saved="onSourceSaved"
          @issues="onSourceIssues"
        />
        <DataView
          v-show="mode === 'data'"
          :document-path="documentPath"
          :datasets="datasets"
          :active="mode === 'data'"
          class="min-h-0 flex-1"
          @pick-document="chooseDocument(true)"
          @append-fields="(id, fields, query) => designRef?.appendFields(id, fields, query)"
          @change-field="(id, index, patch) => designRef?.changeField(id, index, patch)"
          @add-field="(id) => designRef?.addField(id)"
          @remove-field="(id, index) => designRef?.removeField(id, index)"
        />
        <div
          v-show="mode === 'preview'"
          class="flex min-h-0 flex-1 flex-col"
        >
          <div
            v-if="!documentPath"
            class="flex flex-1 flex-col items-center justify-center gap-3 bg-slate-950 px-6"
          >
            <FwbP class="text-center text-gray-400">
              Choose a JSON file in Dataset. Preview uses that file.
            </FwbP>
            <FwbButton @click="selectMode('data')">Dataset</FwbButton>
          </div>
          <div
            v-else-if="error && !hasDocument"
            class="flex flex-1 items-center justify-center bg-slate-950 p-6"
          >
            <FwbAlert type="danger" class="max-w-xl">{{ error }}</FwbAlert>
          </div>
          <div
            v-else
            ref="stageEl"
            class="min-h-0 flex-1 overflow-auto bg-slate-900"
          />
        </div>
        <ConsolePanel :issues="consoleIssues" @jump="onConsoleJump" />
      </template>
    </main>
  </div>
</template>
