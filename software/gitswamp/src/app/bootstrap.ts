import { createApp } from "vue"

import AppView from "@/view/AppView.vue"

import "@/styles/index.css"

export function mountApp() {
  createApp(AppView).mount("#app")
}
