<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { FwbButton } from "flowbite-vue";
import JsonTree from "./JsonTree.vue";
import { defaultPath, previewTable, readFields, recordPath, valueAt, type ReadField } from "./dataRows";
import type { Field } from "./report";

const props = defineProps<{
  documentPath?: string;
  active?: boolean;
  datasets?: { id: string; name: string; listPath: string | null; fields: Field[] }[];
}>();
const emit = defineEmits<{
  pickDocument: [];
  appendFields: [id: string, fields: ReadField[], query?: string | null];
  changeField: [id: string, index: number, patch: { name?: string; className?: string; jsonPath?: string | null }];
  addField: [id: string];
  removeField: [id: string, index: number];
}>();

const document = ref<unknown>(null);
const error = ref("");
const path = ref("");
const loadedPath = ref("");
const datasetId = ref("report");
const readMessage = ref("");
const fieldHeight = ref(208);
const rootEl = ref<HTMLElement | null>(null);
const fieldListEl = ref<HTMLElement | null>(null);
let fieldDrag: { startY: number; startHeight: number } | null = null;

const datasets = computed(() => props.datasets ?? []);
const current = computed(() => datasets.value.find((item) => item.id === datasetId.value) ?? null);
const readTarget = computed(() => {
  const chosen = current.value;
  if (!chosen) {
    return "";
  }
  if (chosen.id === "report") {
    return chosen.listPath ? `Reads ${chosen.listPath}` : "Reads the root record";
  }
  if (chosen.listPath) {
    return `Reads ${chosen.listPath}`;
  }
  return "Set the list on this dataset’s table or chart";
});
const page = ref(0);
const table = computed(() => (document.value === null ? null : previewTable(document.value, path.value, page.value)));

const summary = computed(() => {
  const rows = table.value;
  if (!rows || rows.missing || rows.total === 0) {
    return "";
  }
  if (rows.pageCount <= 1) {
    return rows.total === 1 ? "1 record" : `${rows.total} records`;
  }
  const start = rows.page * rows.pageSize + 1;
  const end = start + rows.rows.length - 1;
  return `${start}–${end} of ${rows.total}`;
});

const fileLabel = computed(() => fileName(props.documentPath ?? ""));

watch(
  () => [props.documentPath, props.active] as const,
  () => {
    if (props.active) {
      void load();
    }
  },
  { immediate: true },
);

async function load(): Promise<void> {
  const file = props.documentPath ?? "";
  if (!file) {
    document.value = null;
    error.value = "";
    path.value = "";
    loadedPath.value = "";
    return;
  }
  try {
    const text = await invoke<string>("read_template", { path: file });
    const parsed = JSON.parse(text) as unknown;
    const sameFile = loadedPath.value === file;
    document.value = parsed;
    error.value = "";
    loadedPath.value = file;
    if (!sameFile) {
      path.value = defaultPath(parsed);
    }
  } catch (err) {
    document.value = null;
    error.value = err instanceof SyntaxError ? "The file is not JSON." : err instanceof Error ? err.message : String(err);
  }
}

watch(path, () => {
  page.value = 0;
});

watch(datasetId, () => {
  const chosen = current.value;
  if (!chosen || chosen.id === "report" || !chosen.listPath || document.value === null) {
    return;
  }
  const record = Array.isArray(document.value) ? document.value[0] : document.value;
  if (valueAt(record, chosen.listPath) === undefined) {
    return;
  }
  path.value = Array.isArray(document.value) ? `0.${chosen.listPath}` : chosen.listPath;
});

watch(
  datasets,
  (list) => {
    if (!list.some((item) => item.id === datasetId.value)) {
      datasetId.value = list[0]?.id ?? "report";
    }
  },
  { immediate: true },
);

async function onReadFields(): Promise<void> {
  readMessage.value = "";
  const chosen = current.value;
  const doc = document.value;
  if (doc === null || !chosen) {
    return;
  }
  if (chosen.id !== "report" && !chosen.listPath) {
    readMessage.value = "Set the data path on this dataset’s table or chart.";
    return;
  }
  const record = Array.isArray(doc) ? doc[0] : doc;
  if (chosen.id !== "report") {
    const found = readFields(record, chosen.listPath ?? "");
    if (!found) {
      readMessage.value = `The list ${chosen.listPath} is not on the current record.`;
      return;
    }
    emit("appendFields", chosen.id, found);
    if (found.length === 0) {
      readMessage.value = `${chosen.listPath} has no values.`;
    }
    return;
  }
  const query = chosen.listPath;
  const queryFields = query && valueAt(doc, query) !== undefined ? readFields(doc, query) : null;
  if (query && queryFields && queryFields.length > 0) {
    emit("appendFields", chosen.id, queryFields);
    return;
  }
  const tablePath = path.value || defaultPath(doc);
  const located =
    recordPath(doc, tablePath) ?? (tablePath !== defaultPath(doc) ? recordPath(doc, defaultPath(doc)) : null);
  if (!located) {
    readMessage.value = "The root record has no values.";
    return;
  }
  const found = readFields(doc, located);
  if (!found || found.length === 0) {
    readMessage.value = "The root record has no values.";
    return;
  }
  const nextQuery = located === "" || located === "." ? null : located;
  const queryChanged = (nextQuery ?? "") !== (query ?? "");
  const before = chosen.fields.length;
  emit("appendFields", chosen.id, found, queryChanged ? nextQuery : undefined);
  await nextTick();
  if ((current.value?.fields.length ?? 0) > before && fieldListEl.value) {
    fieldListEl.value.scrollTop = fieldListEl.value.scrollHeight;
  }
}

async function onAddField(): Promise<void> {
  if (!current.value) {
    return;
  }
  const before = current.value.fields.length;
  emit("addField", current.value.id);
  await nextTick();
  if ((current.value?.fields.length ?? 0) <= before) {
    return;
  }
  const list = fieldListEl.value;
  if (!list) {
    return;
  }
  list.scrollTop = list.scrollHeight;
  const name = list.querySelector("tbody tr:last-child input");
  if (name instanceof HTMLInputElement) {
    name.focus();
    name.select();
  }
}

function clampFieldHeight(height: number): number {
  const host = rootEl.value?.clientHeight ?? 600;
  const max = Math.max(120, host - 120);
  return Math.min(max, Math.max(120, Math.round(height)));
}

function onFieldResizeDown(event: PointerEvent): void {
  fieldDrag = { startY: event.clientY, startHeight: fieldHeight.value };
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
}

function onFieldResizeMove(event: PointerEvent): void {
  if (!fieldDrag) {
    return;
  }
  fieldHeight.value = clampFieldHeight(fieldDrag.startHeight + (fieldDrag.startY - event.clientY));
}

function onFieldResizeUp(): void {
  fieldDrag = null;
}

const fieldInput =
  "min-w-0 w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-xs text-gray-100 outline-none focus:border-blue-500";

function fileName(file: string): string {
  const index = Math.max(file.lastIndexOf("/"), file.lastIndexOf("\\"));
  return index >= 0 ? file.slice(index + 1) : file;
}
</script>

<template>
  <div ref="rootEl" class="flex min-h-0 flex-1 flex-col bg-slate-950">
    <div class="flex h-10 shrink-0 items-center gap-2 border-b border-slate-800 bg-slate-900 px-3">
      <span class="shrink-0 text-xs text-gray-400">JSON file</span>
      <span class="max-w-48 truncate text-xs text-gray-200" :title="documentPath">{{ fileLabel || "None" }}</span>
      <FwbButton size="xs" color="alternative" @click="emit('pickDocument')">Browse</FwbButton>
      <label class="flex min-w-0 flex-1 items-center gap-2 text-xs text-gray-400">
        Data path
        <input
          v-model="path"
          class="min-w-0 flex-1 rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-xs text-gray-100 outline-none focus:border-blue-500"
          placeholder="orders"
          :disabled="document === null"
        />
      </label>
      <FwbButton size="xs" color="alternative" :disabled="!documentPath" @click="load">Refresh</FwbButton>
      <span v-if="summary" class="shrink-0 text-xs text-gray-400">{{ summary }}</span>
      <FwbButton
        v-if="table && table.pageCount > 1"
        size="xs"
        color="alternative"
        :disabled="table.page === 0"
        @click="page = Math.max(0, page - 1)"
      >
        Previous
      </FwbButton>
      <FwbButton
        v-if="table && table.pageCount > 1"
        size="xs"
        color="alternative"
        :disabled="table.page >= table.pageCount - 1"
        @click="page = table.page + 1"
      >
        Next
      </FwbButton>
    </div>

    <div v-if="!documentPath" class="flex flex-1 items-center justify-center px-6 text-sm text-gray-400">
      Browse for a JSON file. A list shows one row per object. Click a list in the tree to use that list.
    </div>
    <div v-else-if="error" class="flex flex-1 items-center justify-center px-6 text-sm text-amber-400">
      {{ error }}
    </div>
    <div v-else class="flex min-h-0 flex-1">
      <aside class="w-64 shrink-0 overflow-auto border-r border-slate-800 p-2">
        <JsonTree :value="document" :selected="path" @select="path = $event" />
      </aside>
      <div class="min-w-0 flex-1 overflow-auto">
        <p v-if="table?.missing" class="p-4 text-sm text-amber-400">That path is not in this file.</p>
        <p v-else-if="table && table.total === 0" class="p-4 text-sm text-gray-400">This list has no records.</p>
        <p v-else-if="table && table.columns.length === 0" class="p-4 text-sm text-gray-400">This record has no fields.</p>
        <table v-else-if="table" class="w-full border-collapse text-left text-xs">
          <thead class="sticky top-0 bg-slate-900">
            <tr>
              <th class="border-b border-slate-700 px-2 py-1.5 font-medium text-gray-500">#</th>
              <th
                v-for="column in table.columns"
                :key="column"
                class="border-b border-slate-700 px-2 py-1.5 font-medium text-gray-200"
              >
                {{ column }}
              </th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(row, index) in table.rows" :key="index" class="odd:bg-slate-950 even:bg-slate-900/70">
              <td class="border-b border-slate-800 px-2 py-1 text-gray-500">{{ table.page * table.pageSize + index + 1 }}</td>
              <td
                v-for="(cell, cellIndex) in row"
                :key="cellIndex"
                class="max-w-xs truncate border-b border-slate-800 px-2 py-1 text-gray-100"
                :title="cell"
              >
                {{ cell }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <section
      class="relative flex shrink-0 flex-col border-t border-slate-800 bg-slate-900"
      :style="{ height: `${fieldHeight}px` }"
    >
      <div
        class="absolute -top-1 right-0 left-0 z-20 h-2 cursor-row-resize hover:bg-blue-500"
        role="separator"
        aria-orientation="horizontal"
        title="Resize fields"
        @pointerdown="onFieldResizeDown"
        @pointermove="onFieldResizeMove"
        @pointerup="onFieldResizeUp"
        @pointercancel="onFieldResizeUp"
      />
      <div class="flex h-10 shrink-0 items-center gap-2 border-b border-slate-800 px-3">
        <span class="text-xs font-medium text-gray-200">Fields</span>
        <select
          v-if="datasets.length > 1"
          v-model="datasetId"
          class="max-w-48 rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-xs text-gray-100 outline-none"
        >
          <option v-for="item in datasets" :key="item.id" :value="item.id">{{ item.name }}</option>
        </select>
        <span v-else-if="current" class="truncate text-xs text-gray-400">{{ current.name }}</span>
        <FwbButton size="xs" :disabled="document === null || !current" @click="onReadFields">Read Fields</FwbButton>
        <FwbButton size="xs" color="alternative" :disabled="!current" @click="onAddField">
          Add field
        </FwbButton>
        <span v-if="readTarget" class="truncate text-xs text-gray-500">{{ readTarget }}</span>
        <span v-if="readMessage" class="truncate text-xs text-amber-400">{{ readMessage }}</span>
      </div>
      <div ref="fieldListEl" class="min-h-0 flex-1 overflow-auto">
        <p v-if="!current" class="p-3 text-xs text-gray-500">Open a report to edit its fields.</p>
        <p v-else-if="current.fields.length === 0" class="p-3 text-xs text-gray-500">
          This dataset has no fields. Read Fields adds columns from the record this dataset fills.
        </p>
        <table v-else class="w-full border-collapse text-left text-xs">
          <thead class="sticky top-0 bg-slate-900">
            <tr>
              <th class="border-b border-slate-700 px-2 py-1.5 font-medium text-gray-400">Name</th>
              <th class="border-b border-slate-700 px-2 py-1.5 font-medium text-gray-400">Class</th>
              <th class="border-b border-slate-700 px-2 py-1.5 font-medium text-gray-400">JSON path</th>
              <th class="border-b border-slate-700 px-2 py-1.5"></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(field, index) in current.fields" :key="`${current.id}-${index}`">
              <td class="border-b border-slate-800 px-2 py-1">
                <input
                  :class="fieldInput"
                  :value="field.name"
                  @change="current && emit('changeField', current.id, index, { name: ($event.target as HTMLInputElement).value })"
                />
              </td>
              <td class="border-b border-slate-800 px-2 py-1">
                <select
                  :class="fieldInput"
                  :value="field.className"
                  @change="current && emit('changeField', current.id, index, { className: ($event.target as HTMLSelectElement).value })"
                >
                  <option value="java.lang.String">String</option>
                  <option value="java.lang.Integer">Integer</option>
                  <option
                    v-if="field.className !== 'java.lang.String' && field.className !== 'java.lang.Integer'"
                    :value="field.className"
                  >
                    {{ field.className }}
                  </option>
                </select>
              </td>
              <td class="border-b border-slate-800 px-2 py-1">
                <input
                  :class="fieldInput"
                  :value="field.jsonPath ?? ''"
                  @change="current && emit('changeField', current.id, index, { jsonPath: ($event.target as HTMLInputElement).value || null })"
                />
              </td>
              <td class="border-b border-slate-800 px-2 py-1 text-right">
                <FwbButton size="xs" color="alternative" @click="current && emit('removeField', current.id, index)">Remove</FwbButton>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>
  </div>
</template>
