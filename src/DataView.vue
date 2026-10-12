<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { FwbButton } from "flowbite-vue";
import JsonTree from "./JsonTree.vue";
import { defaultPath, previewTable } from "./dataRows";

const props = defineProps<{ documentPath?: string; active?: boolean }>();
const emit = defineEmits<{ pickDocument: [] }>();

const document = ref<unknown>(null);
const error = ref("");
const path = ref("");
const loadedPath = ref("");

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
  </div>
</template>
