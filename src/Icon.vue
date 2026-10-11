<script setup lang="ts">
withDefaults(
  defineProps<{
    name:
      | "folder"
      | "file"
      | "layers"
      | "type"
      | "field"
      | "image"
      | "minus"
      | "square"
      | "table"
      | "chevrons-left"
      | "chevron-left"
      | "chevron-right"
      | "chevrons-right"
      | "zoom-out"
      | "zoom-in"
      | "save"
      | "refresh"
      | "print";
  }>(),
  {},
);
</script>

<template>
  <svg
    xmlns="http://www.w3.org/2000/svg"
    fill="none"
    viewBox="0 0 24 24"
    stroke-width="1.5"
    stroke="currentColor"
    class="h-4 w-4 shrink-0"
    aria-hidden="true"
  >
    <path
      v-if="name === 'folder'"
      stroke-linecap="round"
      stroke-linejoin="round"
      d="M3.75 6.75A2.25 2.25 0 016 4.5h3.879a1.5 1.5 0 011.06.44l1.122 1.12A1.5 1.5 0 0013.122 6.5H18a2.25 2.25 0 012.25 2.25v8.25A2.25 2.25 0 0118 19.25H6a2.25 2.25 0 01-2.25-2.25V6.75z"
    />
    <path
      v-else-if="name === 'file'"
      stroke-linecap="round"
      stroke-linejoin="round"
      d="M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9z"
    />
    <path
      v-else-if="name === 'layers'"
      stroke-linecap="round"
      stroke-linejoin="round"
      d="M6.429 9.75L2.25 12l4.179 2.25m0-4.5l5.571 3 5.571-3m-11.142 0L2.25 7.5 12 2.25l9.75 5.25-4.179 2.25m0 0L21.75 12l-4.179 2.25m-11.142 0l5.571 3 5.571-3m-11.142 0L2.25 16.5 12 21.75l9.75-5.25-4.179-2.25"
    />
    <path
      v-else-if="name === 'type'"
      stroke-linecap="round"
      stroke-linejoin="round"
      d="M7.5 3.75h9m-4.5 0v16.5m-5.25 0h10.5"
    />
    <path
      v-else-if="name === 'field'"
      stroke-linecap="round"
      stroke-linejoin="round"
      d="M3.75 6.75h16.5M3.75 12h16.5m-16.5 5.25H12"
    />
    <path
      v-else-if="name === 'image'"
      stroke-linecap="round"
      stroke-linejoin="round"
      d="M2.25 15.75l5.159-5.159a2.25 2.25 0 013.182 0l5.159 5.159m-1.5-1.5l1.409-1.41a2.25 2.25 0 013.182 0l2.909 2.91M3.75 19.5h16.5a1.5 1.5 0 001.5-1.5V6a1.5 1.5 0 00-1.5-1.5H3.75A1.5 1.5 0 002.25 6v12a1.5 1.5 0 001.5 1.5zM8.25 8.25h.008v.008H8.25V8.25z"
    />
    <path
      v-else-if="name === 'minus'"
      stroke-linecap="round"
      stroke-linejoin="round"
      d="M5 12h14"
    />
    <path
      v-else-if="name === 'square'"
      stroke-linecap="round"
      stroke-linejoin="round"
      d="M5.25 5.25h13.5v13.5H5.25z"
    />
    <path
      v-else-if="name === 'table'"
      stroke-linecap="round"
      stroke-linejoin="round"
      d="M3.375 6.75h17.25M3.375 12h17.25m-17.25 5.25h17.25M9 3.75v16.5m6-16.5v16.5"
    />
    <path
      v-else-if="name === 'chevrons-left'"
      stroke-linecap="round"
      stroke-linejoin="round"
      d="M18.75 19.5l-7.5-7.5 7.5-7.5m-6 15L5.25 12l7.5-7.5"
    />
    <path
      v-else-if="name === 'chevron-left'"
      stroke-linecap="round"
      stroke-linejoin="round"
      d="M15.75 19.5L8.25 12l7.5-7.5"
    />
    <path
      v-else-if="name === 'chevron-right'"
      stroke-linecap="round"
      stroke-linejoin="round"
      d="M8.25 4.5l7.5 7.5-7.5 7.5"
    />
    <path
      v-else-if="name === 'chevrons-right'"
      stroke-linecap="round"
      stroke-linejoin="round"
      d="M5.25 4.5l7.5 7.5-7.5 7.5m6-15l7.5 7.5-7.5 7.5"
    />
    <path
      v-else-if="name === 'zoom-out'"
      stroke-linecap="round"
      stroke-linejoin="round"
      d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607zM13.5 10.5h-6"
    />
    <path
      v-else-if="name === 'zoom-in'"
      stroke-linecap="round"
      stroke-linejoin="round"
      d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607zM10.5 7.5v6m3-3h-6"
    />
    <path
      v-else-if="name === 'save'"
      stroke-linecap="round"
      stroke-linejoin="round"
      d="M17.25 6.75v12.75A2.25 2.25 0 0115 21.75H6.75A2.25 2.25 0 014.5 19.5V4.5A2.25 2.25 0 016.75 2.25h7.5L17.25 6.75zM9 15.75h6"
    />
    <path
      v-else-if="name === 'refresh'"
      stroke-linecap="round"
      stroke-linejoin="round"
      d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182"
    />
    <path
      v-else-if="name === 'print'"
      stroke-linecap="round"
      stroke-linejoin="round"
      d="M6.72 13.829V21h10.56v-7.171M6.72 9V3.75h10.56V9M3.75 9h16.5v7.5H3.75V9z"
    />
  </svg>
</template>
