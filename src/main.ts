import { createApp } from "vue";
import App from "./App.vue";
import { installContextMenu } from "./contextMenu";
import "./styles.css";
import "flowbite-vue/index.css";

// Capture phase, before the webview treats Cmd/Ctrl+S as "save this page" and reloads.
window.addEventListener(
  "keydown",
  (event) => {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") {
      event.preventDefault();
    }
  },
  true,
);

installContextMenu();
createApp(App).mount("#app");
