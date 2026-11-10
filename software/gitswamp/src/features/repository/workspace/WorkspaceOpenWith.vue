<script lang="ts">
import { invoke } from "@tauri-apps/api/core";

const editorNames = {
  vscode: "VS Code",
  visualstudio: "Visual Studio",
  androidstudio: "Android Studio",
  intellij: "IntelliJ IDEA",
} as const;
type EditorId = keyof typeof editorNames;
type OpenTarget = EditorId | "explorer" | "application";

let detection: Promise<EditorId[]> | undefined;
function installedEditors() {
  return detection ??= invoke<string[]>("get_available_external_tools").then(ids =>
    (Object.keys(editorNames) as EditorId[]).filter(id => ids.includes(id)),
  ).catch(() => {
    detection = undefined;
    return [];
  });
}
</script>

<script setup lang="ts">
import { nextTick, ref, useId, watch } from "vue";
import { AppWindow, ChevronDown, CodeXml, FolderOpen, Loader2 } from "lucide-vue-next";
import { useToast } from "@/shared/notifications/useToast";

const props = defineProps<{ path: string; branch: string; disabled?: boolean }>();
const toast = useToast();
const menuId = useId();
const trigger = ref<HTMLButtonElement | null>(null);
const menu = ref<HTMLElement | null>(null);
const open = ref(false);
const scanning = ref(false);
const launching = ref(false);
const editors = ref<EditorId[]>([]);
const position = ref({ left: "0px", top: "0px" });

function reposition() {
  const anchor = trigger.value?.getBoundingClientRect();
  const panel = menu.value;
  if (!anchor || !panel) return;
  const bottom = anchor.bottom + 4;
  const top = bottom + panel.offsetHeight <= window.innerHeight - 8 ? bottom : anchor.top - panel.offsetHeight - 4;
  position.value = {
    left: `${Math.max(8, Math.min(anchor.right - panel.offsetWidth, window.innerWidth - panel.offsetWidth - 8))}px`,
    top: `${Math.max(8, top)}px`,
  };
}

function close(restoreFocus = false) {
  open.value = false;
  if (restoreFocus) trigger.value?.focus({ preventScroll: true });
}

async function toggle() {
  if (open.value) { close(); return; }
  open.value = true;
  scanning.value = true;
  const found = installedEditors();
  await nextTick();
  if (open.value) {
    reposition();
    menu.value?.querySelector<HTMLButtonElement>("button")?.focus({ preventScroll: true });
  }
  editors.value = await found;
  scanning.value = false;
  await nextTick();
  if (open.value) reposition();
}

watch(open, (visible, _, onCleanup) => {
  if (!visible) return;
  const outside = (event: PointerEvent) => {
    const target = event.target as Node;
    if (!menu.value?.contains(target) && !trigger.value?.contains(target)) close();
  };
  const dismiss = () => close();
  const onScroll = (event: Event) => {
    if (event.target instanceof Node && menu.value?.contains(event.target)) return;
    close();
  };
  window.addEventListener("pointerdown", outside);
  window.addEventListener("resize", dismiss);
  window.addEventListener("scroll", onScroll, true);
  onCleanup(() => {
    window.removeEventListener("pointerdown", outside);
    window.removeEventListener("resize", dismiss);
    window.removeEventListener("scroll", onScroll, true);
  });
});
watch(() => props.disabled, disabled => { if (disabled) close(); });

function onMenuKey(event: KeyboardEvent) {
  if (event.key === "Escape" || event.key === "Tab") {
    if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); }
    close(true);
    return;
  }
  const items = Array.from(menu.value?.querySelectorAll<HTMLButtonElement>("button") ?? []);
  if (!items.length || !["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
  event.preventDefault();
  const index = items.indexOf(document.activeElement as HTMLButtonElement);
  const next = event.key === "Home" ? 0 : event.key === "End" ? items.length - 1
    : (index + (event.key === "ArrowDown" ? 1 : -1) + items.length) % items.length;
  items[next].focus();
}

async function launch(tool: OpenTarget) {
  if (launching.value || props.disabled) return;
  close(true);
  launching.value = true;
  try {
    if (tool === "application") {
      await invoke("open_branch_workspace_with_application", { path: props.path, branch: props.branch });
    } else {
      await invoke("open_path_with_tool", { path: props.path, tool });
    }
  } catch (error) {
    toast.error(`Could not open ${props.branch}: ${String(error)}`);
  } finally { launching.value = false; }
}
</script>

<template>
  <button ref="trigger" type="button" class="open-with-trigger" title="Open with" :aria-label="`Open ${branch} with`"
    aria-haspopup="menu" :aria-expanded="open" :aria-controls="open ? menuId : undefined"
    :disabled="disabled || launching" @click="toggle" @keydown.down.prevent="!open && toggle()">
    <Loader2 v-if="launching" :size="14" class="animate-spin" />
    <template v-else><FolderOpen :size="15" /><ChevronDown :size="10" /></template>
  </button>
  <Teleport to="body">
    <div v-if="open" :id="menuId" ref="menu" role="menu" :aria-label="`Open ${branch} with`" class="open-with-menu"
      :style="position" @keydown="onMenuKey">
      <div class="open-with-title" :title="branch">{{ branch }}</div>
      <button type="button" role="menuitem" @click="launch('explorer')"><FolderOpen :size="15" />File Explorer</button>
      <div v-if="scanning" class="open-with-loading" role="status"><Loader2 :size="13" class="animate-spin" />Finding editors</div>
      <button v-for="editor in editors" :key="editor" type="button" role="menuitem" @click="launch(editor)">
        <CodeXml :size="15" />{{ editorNames[editor] }}
      </button>
      <button type="button" role="menuitem" class="open-with-application" @click="launch('application')"><AppWindow :size="15" />Choose application...</button>
    </div>
  </Teleport>
</template>

<style scoped>
.open-with-trigger { width: 33px; height: 27px; flex: 0 0 33px; display: flex; align-items: center; justify-content: center; gap: 2px; border-radius: 4px; color: var(--muted-foreground); }
.open-with-trigger:not(:disabled):hover, .open-with-trigger:focus-visible, .open-with-trigger[aria-expanded="true"] { background: var(--accent); color: var(--foreground); }
.open-with-trigger:disabled { opacity: .55; }
.open-with-menu { position: fixed; z-index: 300; width: min(215px, calc(100vw - 16px)); max-height: calc(100dvh - 16px); overflow: auto; padding: 4px; border: 1px solid var(--border); border-radius: 6px; background: var(--popover); color: var(--foreground); box-shadow: 0 8px 22px #0004; }
.open-with-title { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; padding: 4px 7px 6px; border-bottom: 1px solid var(--border); font-size: 11px; font-weight: 650; }
.open-with-menu button, .open-with-loading { width: 100%; display: flex; align-items: center; gap: 8px; padding: 6px 7px; text-align: left; font-size: 11px; border-radius: 4px; }
.open-with-menu button:hover, .open-with-menu button:focus-visible { background: var(--accent); outline: none; }
.open-with-menu svg { flex: none; }
.open-with-loading { color: var(--muted-foreground); }
.open-with-application { margin-top: 3px; border-top: 1px solid var(--border); }
</style>
