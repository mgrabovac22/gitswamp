<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import { ShieldAlert, X, FileMinus, GitCommitHorizontal, HardDrive } from "lucide-vue-next";
import { secretConfirmation, finishSecretConfirmation } from "@/shared/notifications/secretConfirmation";

const props = defineProps<{ repoPath: string | null }>();
const emit = defineEmits<{ openLfs: [request: { path: string; files: string[] }] }>();
const cancelButton = ref<HTMLButtonElement | null>(null);
const dialog = ref<HTMLElement | null>(null);
let previousFocus: HTMLElement | null = null;
const report = computed(() => secretConfirmation.value?.report);
const canIgnore = computed(() => report.value?.findings.some(f => !f.tracked));
const largeFiles = computed(() => report.value?.large_files ?? []);
function sizeLabel(size: number) { return `${(size / 1024 / 1024).toFixed(1)} MiB`; }
function openLfsManager() {
  const path = secretConfirmation.value?.repoPath;
  if (!path) return;
  const files = largeFiles.value.map(file => file.path);
  finishSecretConfirmation();
  emit("openLfs", { path, files });
}
watch(() => props.repoPath, () => finishSecretConfirmation());
watch(secretConfirmation, async value => {
  if (value) {
    previousFocus = document.activeElement as HTMLElement | null;
    await nextTick(); cancelButton.value?.focus();
  } else previousFocus?.focus();
});
function onKey(event: KeyboardEvent) {
  if (event.key === "Escape") finishSecretConfirmation();
  if (event.key !== "Tab") return;
  const controls = [...(dialog.value?.querySelectorAll<HTMLButtonElement>('button:not(:disabled)') ?? [])];
  const first = controls[0], last = controls[controls.length - 1];
  if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last?.focus(); }
  else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); }
}
onBeforeUnmount(() => finishSecretConfirmation());
</script>

<template>
  <Teleport to="body">
    <div v-if="report" class="secret-overlay" @click.self="finishSecretConfirmation()" @keydown.stop="onKey">
      <section ref="dialog" role="alertdialog" aria-modal="true" aria-labelledby="secret-title" aria-describedby="secret-description" class="secret-dialog">
        <header><ShieldAlert :size="20" /><h2 id="secret-title">{{ largeFiles.length ? 'Large files need Git LFS' : 'Review staged secrets' }}</h2>
          <button title="Cancel commit" aria-label="Cancel commit" @click="finishSecretConfirmation()"><X :size="18" /></button>
        </header>
        <p id="secret-description" v-if="largeFiles.length">GitHub blocks regular Git files larger than 100 MiB. Convert the staged files to Git LFS or unstage them. No commit has been created.</p>
        <p v-if="report.findings.length || report.skipped.length" :id="largeFiles.length ? undefined : 'secret-description'">{{ report.findings.length ? 'Possible credentials found in staged files.' : 'Some staged files could not be fully checked.' }} {{ largeFiles.length ? '' : 'No commit has been created.' }}</p>
        <ul class="secret-findings">
          <li v-for="file in largeFiles" :key="'large:'+file.path"><strong>{{ file.path }}</strong><span>{{ sizeLabel(file.size) }} · exceeds GitHub's 100 MiB limit</span></li>
          <li v-for="(finding, index) in report.findings" :key="index"><strong>{{ finding.path }}:{{ finding.line }}</strong><span>{{ finding.kind }}{{ finding.tracked ? ' · tracked file' : '' }}</span></li>
          <li v-for="path in report.skipped" :key="path"><strong>{{ path }}</strong><span>Not fully scanned</span></li>
        </ul>
        <p v-if="canIgnore" class="secret-note">Ignore removes new flagged files from staging and adds their paths to .gitignore. Local files stay on disk. The commit is cancelled.</p>
        <p v-if="report.findings.some(f => f.tracked)" class="secret-note">Tracked files remain tracked even with .gitignore. Remove their credentials before staging again.</p>
        <footer>
          <button ref="cancelButton" @click="finishSecretConfirmation()">Cancel</button>
          <button v-if="canIgnore" @click="finishSecretConfirmation('ignore')"><FileMinus :size="15" />Add to .gitignore</button>
          <button v-if="largeFiles.length" class="secret-confirm" @click="openLfsManager"><HardDrive :size="15" />Open Git LFS Manager</button>
          <button v-else class="secret-confirm" @click="finishSecretConfirmation('commit')"><GitCommitHorizontal :size="15" />Commit anyway</button>
        </footer>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.secret-overlay { position: fixed; inset: 0; z-index: 10000; display: grid; place-items: center; padding: 16px; background: #0008; }
.secret-dialog { width: min(100%, 520px); max-height: calc(100dvh - 32px); overflow: auto; background: var(--card); color: var(--foreground); border: 1px solid var(--border); border-radius: 8px; padding: 18px; box-shadow: 0 16px 48px #0005; font-size: 13px; }
header { display: flex; align-items: center; gap: 8px; color: var(--warning, #d99723); } h2 { flex: 1; font-size: 15px; font-weight: 600; color: var(--foreground); }
p { margin-top: 12px; line-height: 1.5; } .secret-note { font-size: 12px; color: var(--muted-foreground); }
.secret-findings { max-height: 240px; overflow: auto; margin-top: 12px; padding: 0; list-style: none; border-block: 1px solid var(--border); }
li { display: flex; flex-direction: column; gap: 3px; padding: 8px 0; overflow-wrap: anywhere; } li strong { font-size: 12px; } li span { color: var(--muted-foreground); font-size: 11px; }
footer { display: flex; justify-content: flex-end; gap: 8px; flex-wrap: wrap; margin-top: 16px; }
button { display: inline-flex; align-items: center; justify-content: center; gap: 5px; padding: 6px 9px; border: 1px solid var(--border); border-radius: 4px; background: var(--secondary); color: var(--foreground); }
button:focus-visible { outline: 2px solid var(--primary); outline-offset: 2px; }.secret-confirm { border-color: var(--warning, #d99723); } header button { border: 0; padding: 4px; background: none; }
</style>
