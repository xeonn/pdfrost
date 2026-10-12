import { createApp } from "vue";
import App from "./App.vue";
import { installContextMenu } from "./contextMenu";
import "./styles.css";
import "flowbite-vue/index.css";

installContextMenu();
createApp(App).mount("#app");
