<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { ConsoleIssue } from "./console";

const props = defineProps<{
  issues: ConsoleIssue[];
}>();

const emit = defineEmits<{
  jump: [issue: ConsoleIssue];
}>();

const collapsed = ref(false);
const userCollapsed = ref(false);

const errorCount = computed(
  () => props.issues.filter((issue) => issue.severity === "error").length,
);
const warningCount = computed(
  () => props.issues.filter((issue) => issue.severity === "warning").length,
);
const visible = computed(() => props.issues.length > 0);

watch(
  () => props.issues.length,
  (count, previous) => {
    if (count > 0 && (previous === 0 || previous === undefined) && !userCollapsed.value) {
      collapsed.value = false;
    }
    if (count === 0) {
      userCollapsed.value = false;
      collapsed.value = false;
    }
  },
);

function toggle(): void {
  collapsed.value = !collapsed.value;
  userCollapsed.value = collapsed.value;
}

function severityClass(severity: ConsoleIssue["severity"]): string {
  if (severity === "error") {
    return "text-red-400";
  }
  if (severity === "warning") {
    return "text-amber-400";
  }
  return "text-sky-400";
}
</script>

<template>
  <section
    v-if="visible"
    class="flex shrink-0 flex-col border-t border-slate-800 bg-slate-950"
  >
    <button
      type="button"
      class="flex h-8 w-full items-center gap-2 px-3 text-left text-xs hover:bg-slate-900"
      @click="toggle"
    >
      <span class="font-semibold uppercase tracking-wide text-gray-400">Console</span>
      <span v-if="errorCount" class="rounded bg-red-500/20 px-1.5 py-0.5 text-red-300">
        {{ errorCount }} error{{ errorCount === 1 ? "" : "s" }}
      </span>
      <span v-if="warningCount" class="rounded bg-amber-500/20 px-1.5 py-0.5 text-amber-300">
        {{ warningCount }} warning{{ warningCount === 1 ? "" : "s" }}
      </span>
      <span class="ml-auto text-gray-500">{{ collapsed ? "Show" : "Hide" }}</span>
    </button>
    <ul
      v-show="!collapsed"
      class="max-h-36 min-h-0 overflow-auto border-t border-slate-800 px-2 py-1 font-mono text-xs"
    >
      <li v-for="issue in issues" :key="issue.id">
        <button
          type="button"
          class="flex w-full items-start gap-2 rounded px-2 py-1 text-left hover:bg-slate-900"
          @click="emit('jump', issue)"
        >
          <span class="shrink-0 uppercase" :class="severityClass(issue.severity)">
            {{ issue.severity }}
          </span>
          <span class="min-w-0 flex-1 text-gray-200">
            {{ issue.message }}
            <span v-if="issue.location" class="text-gray-500"> · {{ issue.location }}</span>
            <span v-else-if="issue.line" class="text-gray-500"> · line {{ issue.line }}</span>
          </span>
        </button>
      </li>
    </ul>
  </section>
</template>
