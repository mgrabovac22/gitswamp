<script setup lang="ts">
import { onBeforeUnmount, ref, shallowRef, watch } from "vue";
import { ExternalLink, Loader2, MessageSquare, RotateCw } from "lucide-vue-next";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { DiscussionKind, RemoteComment, RemoteDiscussionClient } from "./githubRemote";

const props = defineProps<{ client?: RemoteDiscussionClient; number: number; kind: DiscussionKind; author?: string; createdAt?: string; body?: string }>();
const comments = shallowRef<RemoteComment[]>([]);
const loading = ref(false);
const sending = ref(false);
const loadError = ref("");
const postError = ref("");
const draft = ref("");
const more = ref(false);
let page = 0;
let loadController: AbortController | undefined;
let postController: AbortController | undefined;

function combine(previous: RemoteComment[], next: RemoteComment[]) {
  return [...new Map([...previous, ...next].map(comment => [comment.id, comment])).values()]
    .sort((a, b) => a.createdAt.localeCompare(b.createdAt) || a.id - b.id);
}

async function load(reset = false) {
  const client = props.client;
  if (!client || loading.value || sending.value) return;
  const controller = new AbortController();
  loadController = controller;
  const nextPage = reset ? 1 : page + 1;
  loading.value = true;
  loadError.value = "";
  try {
    const result = await client.load(props.number, props.kind, nextPage, controller.signal);
    if (controller.signal.aborted) return;
    comments.value = combine(reset ? [] : comments.value, result.comments);
    page = nextPage;
    more.value = result.more;
  } catch (error) {
    if (!controller.signal.aborted) loadError.value = error instanceof Error ? error.message : "Could not load comments.";
  } finally {
    if (loadController === controller) loading.value = false;
  }
}

async function post() {
  const body = draft.value.trim();
  const client = props.client;
  if (!client?.canWrite || !body || sending.value || loading.value) return;
  const controller = new AbortController();
  postController = controller;
  sending.value = true;
  postError.value = "";
  try {
    const comment = await client.post(props.number, body, controller.signal);
    if (controller.signal.aborted) return;
    comments.value = combine(comments.value, [comment]);
    draft.value = "";
  } catch (error) {
    if (!controller.signal.aborted) postError.value = error instanceof Error ? error.message : "Could not post comment.";
  } finally {
    if (postController === controller) sending.value = false;
  }
}

function abort() {
  loadController?.abort();
  postController?.abort();
  loadController = postController = undefined;
  loading.value = sending.value = false;
}

watch([() => props.client?.key, () => props.number, () => props.kind], () => {
  abort();
  comments.value = [];
  page = 0;
  more.value = false;
  loadError.value = postError.value = draft.value = "";
  void load();
}, { immediate: true });
watch(() => props.client?.canWrite, (canWrite, previous) => {
  if (canWrite && previous === false && loadError.value) void load(true);
});
onBeforeUnmount(abort);

function dateLabel(value?: string) {
  if (!value) return "";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "" : date.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
}
function open(url: string) { if (url) void openUrl(url).catch(() => {}); }
</script>

<template>
  <section :aria-label="kind === 'conversation' ? 'Conversation' : 'Review comments'" class="conversation">
    <article v-if="kind === 'conversation'" class="thread">
      <span class="avatar" aria-hidden="true">{{ (author || '?').slice(0, 1).toUpperCase() }}</span>
      <div class="entry"><header><strong>{{ author || 'GitHub user' }}</strong> opened this <time v-if="createdAt" :datetime="createdAt">· {{ dateLabel(createdAt) }}</time></header>
        <div class="body" :class="{ muted: !body }">{{ body || 'No description provided.' }}</div>
      </div>
    </article>
    <div class="activity">
      <div><MessageSquare :size="15" /><h3>{{ kind === 'conversation' ? 'Comments' : 'Comments on changed lines' }}</h3></div>
      <button v-if="client" type="button" class="refresh" :disabled="loading || sending" aria-label="Refresh comments" title="Refresh comments" @click="load(true)"><RotateCw :size="15" /></button>
    </div>
    <p v-if="loading && !comments.length" class="muted loading" role="status"><Loader2 :size="15" class="animate-spin" /> Loading comments</p>
    <p v-else-if="!client" class="muted">GitHub discussion is unavailable.</p>
    <p v-else-if="!comments.length && !loadError" class="muted">No comments yet.</p>
    <article v-for="comment in comments" :key="comment.id" class="thread">
      <span class="avatar" aria-hidden="true">{{ comment.author.slice(0, 1).toUpperCase() }}</span>
      <div class="entry">
        <header><strong>{{ comment.author }}</strong><time :datetime="comment.createdAt">{{ dateLabel(comment.createdAt) }}</time>
          <button v-if="comment.url" type="button" class="refresh" aria-label="Open comment on GitHub" title="Open comment on GitHub" @click="open(comment.url)"><ExternalLink :size="14" /></button>
        </header>
        <code v-if="comment.path" class="file">{{ comment.path }}{{ comment.line ? ':' + comment.line : '' }}</code>
        <div class="body">{{ comment.body }}</div>
      </div>
    </article>
    <p v-if="loadError" class="error" role="alert">{{ loadError }}</p>
    <button v-if="more" type="button" class="next" :disabled="loading || sending" @click="load()">{{ loading ? 'Loading comments…' : 'Load more comments' }}</button>
    <form v-if="kind === 'conversation' && client" class="composer" @submit.prevent="post">
      <label for="remote-comment-body">Add a comment</label>
      <textarea id="remote-comment-body" v-model="draft" :disabled="sending" maxlength="65536" rows="4" placeholder="Leave a comment" />
      <p v-if="postError" class="error" role="alert">{{ postError }}</p>
      <div class="send">
        <button v-if="!client.canWrite" type="button" class="next" @click="client.authenticate()">Connect GitHub to comment</button>
        <button v-else class="primary" type="submit" :disabled="!draft.trim() || sending || loading">
          <Loader2 v-if="sending" :size="15" class="animate-spin" />{{ sending ? 'Posting…' : 'Comment' }}
        </button>
      </div>
    </form>
  </section>
</template>

<style scoped>
.conversation { display: grid; min-width: 0; gap: 13px; }
.thread { display: grid; grid-template-columns: 30px minmax(0, 1fr); gap: 9px; }
.avatar { display: grid; place-items: center; width: 30px; height: 30px; border-radius: 50%; background: var(--secondary); color: var(--foreground); font-weight: 700; }
.entry { border: 1px solid var(--border); border-radius: 7px; overflow: hidden; background: var(--background); }
.entry header { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; min-height: 38px; padding: 5px 10px; background: var(--secondary); border-bottom: 1px solid var(--border); color: var(--muted-foreground); overflow-wrap: anywhere; }
.entry header strong { color: var(--foreground); }
.entry .refresh { margin-inline-start: auto; }
.body { white-space: pre-wrap; overflow-wrap: anywhere; padding: 11px; line-height: 1.55; }
.file { display: block; padding: 8px 11px 0; overflow-wrap: anywhere; color: var(--muted-foreground); }
.activity, .activity > div, .loading, .send { display: flex; align-items: center; gap: 7px; }
.activity { justify-content: space-between; border-bottom: 1px solid var(--border); padding-bottom: 4px; }
.activity h3 { font-weight: 600; }
.refresh { display: grid; place-items: center; width: 27px; height: 27px; border-radius: 4px; color: var(--muted-foreground); }
.refresh:hover { background: var(--secondary); color: var(--foreground); }
.muted { color: var(--muted-foreground); }
.error { color: var(--destructive); }
.next { justify-self: start; border: 1px solid var(--border); padding: 6px 9px; border-radius: 6px; }
.next:hover { background: var(--secondary); }
.composer { display: grid; gap: 7px; padding-top: 4px; }
.composer label { font-weight: 600; }
.composer textarea { width: 100%; min-height: 92px; padding: 8px; background: var(--input-background); border: 1px solid var(--border); border-radius: 6px; resize: vertical; }
.send { justify-content: end; }
.primary { display: inline-flex; align-items: center; justify-content: center; gap: 6px; min-height: 34px; padding: 5px 12px; background: var(--primary); color: var(--primary-foreground); border-radius: 6px; }
button:disabled { opacity: .55; }
</style>
