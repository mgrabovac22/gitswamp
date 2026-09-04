import { createApp } from "vue"

import AppView from "@/view/AppView.vue"

import "@/styles/index.css"

function renderStartupError(error: unknown) {
  const appRoot = document.getElementById("app")
  if (!appRoot) return

  const reason = String(error)
  appRoot.innerHTML = `
    <div style="min-height:100%;display:flex;align-items:center;justify-content:center;background:#020617;color:#e2e8f0;font-family:'Segoe UI',Arial,sans-serif;padding:24px;box-sizing:border-box;">
      <div style="max-width:720px;width:100%;border:1px solid rgba(148,163,184,0.25);border-radius:14px;background:rgba(2,6,23,0.86);padding:20px 18px;box-shadow:0 14px 32px rgba(2,6,23,0.45);">
        <h1 style="margin:0 0 10px;font-size:20px;line-height:1.2;font-weight:700;">GitSwamp failed to start</h1>
        <p style="margin:0 0 10px;font-size:14px;line-height:1.45;color:#cbd5e1;">The app hit a startup error. This prevents an empty white screen and keeps the app responsive while you collect logs.</p>
        <pre style="margin:0;padding:12px;border-radius:10px;background:rgba(15,23,42,0.9);border:1px solid rgba(148,163,184,0.22);white-space:pre-wrap;word-break:break-word;font-size:12px;line-height:1.4;color:#f8fafc;">${reason}</pre>
      </div>
    </div>
  `
}

function dismissBootSplash() {
  const splash = document.getElementById("boot-splash")
  if (!splash) return

  splash.classList.add("boot-splash-hidden")
  globalThis.setTimeout(() => {
    splash.remove()
  }, 320)
}

export function mountApp() {
  try {
    createApp(AppView).mount("#app")
  } catch (error) {
    dismissBootSplash()
    renderStartupError(error)
    return
  }

  globalThis.requestAnimationFrame(() => {
    globalThis.requestAnimationFrame(() => {
      dismissBootSplash()
    })
  })

  globalThis.setTimeout(() => {
    dismissBootSplash()
  }, 2000)
}
