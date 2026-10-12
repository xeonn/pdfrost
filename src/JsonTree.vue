<script setup lang="ts">
import { computed } from "vue";
import JsonTree from "./JsonTree.vue";

const props = withDefaults(
  defineProps<{
    value: unknown;
    path?: string;
    label?: string;
    selected?: string;
    depth?: number;
  }>(),
  { path: "", label: "This object", selected: "", depth: 0 },
);

const emit = defineEmits<{ select: [path: string] }>();

const kind = computed(() =>
  Array.isArray(props.value) ? "array" : props.value !== null && typeof props.value === "object" ? "object" : "value",
);

const children = computed(() => childNodes(props.value, props.path, props.depth));

function childNodes(value: unknown, path: string, depth: number): { path: string; label: string; value: unknown }[] {
  if (depth >= 6) {
    return [];
  }
  if (Array.isArray(value)) {
    return value.slice(0, 40).map((item, index) => ({
      path: path ? `${path}.${index}` : String(index),
      label: String(index),
      value: item,
    }));
  }
  if (value !== null && typeof value === "object") {
    return Object.entries(value as Record<string, unknown>)
      .filter(([key]) => key && !key.includes("."))
      .slice(0, 40)
      .map(([key, item]) => ({
        path: path ? `${path}.${key}` : key,
        label: key,
        value: item,
      }));
  }
  return [];
}

const detail = computed(() => {
  if (Array.isArray(props.value)) {
    return `${props.value.length} items`;
  }
  if (props.value !== null && typeof props.value === "object") {
    return `${Object.keys(props.value as object).length} fields`;
  }
  if (props.value === null) {
    return "null";
  }
  if (typeof props.value === "string") {
    return props.value.length > 24 ? `${props.value.slice(0, 24)}…` : props.value;
  }
  return String(props.value);
});

function choose(): void {
  if (kind.value === "value") {
    return;
  }
  if (props.path === "" && Array.isArray(props.value)) {
    emit("select", ".");
    return;
  }
  emit("select", props.path);
}

const chosen = computed(
  () => props.selected === props.path || (props.selected === "." && props.path === "" && Array.isArray(props.value)),
);
</script>

<template>
  <div class="text-xs">
    <div class="flex items-center gap-1 py-0.5">
      <button
        type="button"
        class="min-w-0 flex-1 truncate rounded px-1 py-0.5 text-left"
        :class="chosen ? 'bg-blue-600/30 text-white' : kind === 'value' ? 'text-gray-500' : 'text-gray-200 hover:bg-slate-800'"
        :disabled="kind === 'value'"
        @click="choose"
      >
        {{ label }}
        <span class="text-gray-500"> {{ detail }}</span>
      </button>
    </div>
    <div v-if="children.length" class="ml-2 border-l border-slate-800 pl-2">
      <JsonTree
        v-for="child in children"
        :key="child.path"
        :value="child.value"
        :path="child.path"
        :label="child.label"
        :selected="selected"
        :depth="depth + 1"
        @select="emit('select', $event)"
      />
    </div>
  </div>
</template>
