import { createApp } from "vue"

import AppView from "@/view/AppView.vue"

import "@/styles/index.css"

function dismissBootSplash() {
  const splash = document.getElementById("boot-splash")
  if (!splash) return

  splash.classList.add("boot-splash-hidden")
  globalThis.setTimeout(() => {
    splash.remove()
  }, 320)
}

export function mountApp() {
  createApp(AppView).mount("#app")

  globalThis.requestAnimationFrame(() => {
    globalThis.requestAnimationFrame(() => {
      dismissBootSplash()
    })
  })

  globalThis.setTimeout(() => {
    dismissBootSplash()
  }, 2000)
}
