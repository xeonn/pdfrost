<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { FwbButton } from "flowbite-vue";
import JsonTree from "./JsonTree.vue";
import { defaultPath, previewTable, readFields, type ReadField } from "./dataRows";
import type { Field } from "./report";

const props = defineProps<{
  documentPath?: string;
  active?: boolean;
  datasets?: { id: string; name: string; fields: Field[] }[];
}>();
const emit = defineEmits<{
  pickDocument: [];
  appendFields: [id: string, fields: ReadField[]];
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

const datasets = computed(() => props.datasets ?? []);
const current = computed(() => datasets.value.find((item) => item.id === datasetId.value) ?? null);
const table = computed(() => (document.value === null ? null : previewTable(document.value, path.value)));

const summary = computed(() => {
  const rows = table.value;
  if (!rows || rows.missing) {
    return "";
  }
  if (rows.total > rows.rows.length) {
    return `Showing ${rows.rows.length} of ${rows.total} records`;
  }
  return rows.total === 1 ? "1 record" : `${rows.total} records`;
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

watch(
  datasets,
  (list) => {
    if (!list.some((item) => item.id === datasetId.value)) {
      datasetId.value = list[0]?.id ?? "report";
    }
  },
  { immediate: true },
);

function onReadFields(): void {
  readMessage.value = "";
  if (document.value === null || !current.value) {
    return;
  }
  const found = readFields(document.value, path.value);
  if (!found) {
    readMessage.value = "That path is not in this file.";
    return;
  }
  emit("appendFields", current.value.id, found);
  if (found.length === 0) {
    readMessage.value = "No fields were found at this path.";
  }
}

const fieldInput =
  "min-w-0 w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-xs text-gray-100 outline-none focus:border-blue-500";

function fileName(file: string): string {
  const index = Math.max(file.lastIndexOf("/"), file.lastIndexOf("\\"));
  return index >= 0 ? file.slice(index + 1) : file;
}
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col bg-slate-950">
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
              <td class="border-b border-slate-800 px-2 py-1 text-gray-500">{{ index + 1 }}</td>
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

    <section class="flex h-52 shrink-0 flex-col border-t border-slate-800 bg-slate-900">
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
        <FwbButton size="xs" color="alternative" :disabled="!current" @click="current && emit('addField', current.id)">
          Add field
        </FwbButton>
        <span v-if="readMessage" class="truncate text-xs text-amber-400">{{ readMessage }}</span>
      </div>
      <div class="min-h-0 flex-1 overflow-auto">
        <p v-if="!current" class="p-3 text-xs text-gray-500">Open a report to edit its fields.</p>
        <p v-else-if="current.fields.length === 0" class="p-3 text-xs text-gray-500">
          This dataset has no fields. Read Fields adds the columns at the data path.
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
