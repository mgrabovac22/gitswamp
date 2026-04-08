<script setup lang="ts">
const props = defineProps<{
  showBranchDialog: boolean;
  newBranchName: string;
  showStashDialog: boolean;
  stashMessage: string;
  showTagDialog: boolean;
  tagName: string;
  showAnnotatedTagDialog: boolean;
  annotatedTagName: string;
  annotatedTagMessage: string;
  showEditMessageDialog: boolean;
  editMessageText: string;
  showRenameDialog: boolean;
  renameBranchOld: string;
  renameBranchNew: string;
  showRebaseConflictDialog: boolean;
  rebaseConflictSource: string;
  rebaseConflictTarget: string;
  rebaseConflictBusy: boolean;
}>();

const emit = defineEmits<{
  "update:newBranchName": [value: string];
  "update:stashMessage": [value: string];
  "update:tagName": [value: string];
  "update:annotatedTagName": [value: string];
  "update:annotatedTagMessage": [value: string];
  "update:editMessageText": [value: string];
  "update:renameBranchNew": [value: string];
  "close:branch": [];
  "submit:branch": [];
  "close:stash": [];
  "submit:stash": [];
  "close:tag": [];
  "submit:tag": [];
  "close:annotatedTag": [];
  "submit:annotatedTag": [];
  "close:editMessage": [];
  "submit:editMessage": [];
  "close:rename": [];
  "submit:rename": [];
  "close:rebaseConflict": [];
  "submit:rebaseContinue": [];
  "submit:rebaseSkip": [];
  "submit:rebaseAbort": [];
}>();
</script>

<template>
  <div v-if="props.showBranchDialog" class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" @click.self="emit('close:branch')">
    <div class="bg-[var(--popover)] border border-[var(--border)] rounded-lg p-6 w-96 shadow-2xl">
      <h3 class="text-sm font-medium text-[var(--foreground)] mb-4">Create New Branch</h3>
      <input
        :value="props.newBranchName"
        @input="emit('update:newBranchName', ($event.target as HTMLInputElement).value)"
        placeholder="Branch name..."
        class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40 mb-4"
        @keyup.enter="emit('submit:branch')"
        autofocus
      />
      <div class="flex justify-end gap-2">
        <button @click="emit('close:branch')" class="px-3 py-1.5 text-xs text-[var(--muted-foreground)] hover:text-[var(--foreground)] rounded hover:bg-[var(--secondary)] transition-colors">Cancel</button>
        <button @click="emit('submit:branch')" :disabled="!props.newBranchName.trim()" class="px-3 py-1.5 text-xs text-white bg-[var(--primary)] hover:opacity-90 rounded disabled:opacity-50 transition-colors">Create</button>
      </div>
    </div>
  </div>

  <div v-if="props.showStashDialog" class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" @click.self="emit('close:stash')">
    <div class="bg-[var(--popover)] border border-[var(--border)] rounded-lg p-6 w-96 shadow-2xl">
      <h3 class="text-sm font-medium text-[var(--foreground)] mb-4">Stash Changes</h3>
      <input
        :value="props.stashMessage"
        @input="emit('update:stashMessage', ($event.target as HTMLInputElement).value)"
        placeholder="Stash message (optional)..."
        class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40 mb-4"
        @keyup.enter="emit('submit:stash')"
        autofocus
      />
      <div class="flex justify-end gap-2">
        <button @click="emit('close:stash')" class="px-3 py-1.5 text-xs text-[var(--muted-foreground)] hover:text-[var(--foreground)] rounded hover:bg-[var(--secondary)] transition-colors">Cancel</button>
        <button @click="emit('submit:stash')" class="px-3 py-1.5 text-xs text-white bg-[var(--primary)] hover:opacity-90 rounded transition-colors">Stash</button>
      </div>
    </div>
  </div>

  <div v-if="props.showTagDialog" class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" @click.self="emit('close:tag')">
    <div class="bg-[var(--popover)] border border-[var(--border)] rounded-lg p-6 w-96 shadow-2xl">
      <h3 class="text-sm font-medium text-[var(--foreground)] mb-4">Create Tag</h3>
      <input
        :value="props.tagName"
        @input="emit('update:tagName', ($event.target as HTMLInputElement).value)"
        placeholder="Tag name..."
        class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40 mb-4"
        @keyup.enter="emit('submit:tag')"
        autofocus
      />
      <div class="flex justify-end gap-2">
        <button @click="emit('close:tag')" class="px-3 py-1.5 text-xs text-[var(--muted-foreground)] hover:text-[var(--foreground)] rounded hover:bg-[var(--secondary)] transition-colors">Cancel</button>
        <button @click="emit('submit:tag')" :disabled="!props.tagName.trim()" class="px-3 py-1.5 text-xs text-white bg-[var(--primary)] hover:opacity-90 rounded disabled:opacity-50 transition-colors">Create</button>
      </div>
    </div>
  </div>

  <div v-if="props.showAnnotatedTagDialog" class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" @click.self="emit('close:annotatedTag')">
    <div class="bg-[var(--popover)] border border-[var(--border)] rounded-lg p-6 w-96 shadow-2xl">
      <h3 class="text-sm font-medium text-[var(--foreground)] mb-4">Create Annotated Tag</h3>
      <input
        :value="props.annotatedTagName"
        @input="emit('update:annotatedTagName', ($event.target as HTMLInputElement).value)"
        placeholder="Tag name..."
        class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40 mb-3"
        autofocus
      />
      <textarea
        :value="props.annotatedTagMessage"
        @input="emit('update:annotatedTagMessage', ($event.target as HTMLTextAreaElement).value)"
        placeholder="Tag message..."
        rows="3"
        class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40 resize-none mb-4"
      />
      <div class="flex justify-end gap-2">
        <button @click="emit('close:annotatedTag')" class="px-3 py-1.5 text-xs text-[var(--muted-foreground)] hover:text-[var(--foreground)] rounded hover:bg-[var(--secondary)] transition-colors">Cancel</button>
        <button @click="emit('submit:annotatedTag')" :disabled="!props.annotatedTagName.trim() || !props.annotatedTagMessage.trim()" class="px-3 py-1.5 text-xs text-white bg-[var(--primary)] hover:opacity-90 rounded disabled:opacity-50 transition-colors">Create</button>
      </div>
    </div>
  </div>

  <div v-if="props.showEditMessageDialog" class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" @click.self="emit('close:editMessage')">
    <div class="bg-[var(--popover)] border border-[var(--border)] rounded-lg p-6 w-[480px] shadow-2xl">
      <h3 class="text-sm font-medium text-[var(--foreground)] mb-4">Edit Commit Message</h3>
      <textarea
        :value="props.editMessageText"
        @input="emit('update:editMessageText', ($event.target as HTMLTextAreaElement).value)"
        rows="5"
        class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40 resize-none mb-4"
        autofocus
      />
      <div class="flex justify-end gap-2">
        <button @click="emit('close:editMessage')" class="px-3 py-1.5 text-xs text-[var(--muted-foreground)] hover:text-[var(--foreground)] rounded hover:bg-[var(--secondary)] transition-colors">Cancel</button>
        <button @click="emit('submit:editMessage')" :disabled="!props.editMessageText.trim()" class="px-3 py-1.5 text-xs text-white bg-[var(--primary)] hover:opacity-90 rounded disabled:opacity-50 transition-colors">Save</button>
      </div>
    </div>
  </div>

  <div v-if="props.showRenameDialog" class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" @click.self="emit('close:rename')">
    <div class="bg-[var(--popover)] border border-[var(--border)] rounded-lg p-6 w-96 shadow-2xl">
      <h3 class="text-sm font-medium text-[var(--foreground)] mb-4">Rename Branch</h3>
      <div class="text-[10px] text-[var(--muted-foreground)] mb-2">Rename "{{ props.renameBranchOld }}" to:</div>
      <input
        :value="props.renameBranchNew"
        @input="emit('update:renameBranchNew', ($event.target as HTMLInputElement).value)"
        placeholder="New branch name..."
        class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40 mb-4"
        @keyup.enter="emit('submit:rename')"
        autofocus
      />
      <div class="flex justify-end gap-2">
        <button @click="emit('close:rename')" class="px-3 py-1.5 text-xs text-[var(--muted-foreground)] hover:text-[var(--foreground)] rounded hover:bg-[var(--secondary)] transition-colors">Cancel</button>
        <button @click="emit('submit:rename')" :disabled="!props.renameBranchNew.trim()" class="px-3 py-1.5 text-xs text-white bg-[var(--primary)] hover:opacity-90 rounded disabled:opacity-50 transition-colors">Rename</button>
      </div>
    </div>
  </div>

  <div
    v-if="props.showRebaseConflictDialog"
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
    @click.self="!props.rebaseConflictBusy && emit('close:rebaseConflict')"
  >
    <div class="bg-[var(--popover)] border border-[var(--border)] rounded-lg p-6 w-[520px] max-w-[92vw] shadow-2xl">
      <h3 class="text-sm font-semibold text-[var(--foreground)] mb-2">Rebase Conflict</h3>
      <p class="text-xs text-[var(--muted-foreground)] leading-relaxed mb-2">
        Rebase paused while applying
        <span class="text-[var(--foreground)] font-medium">{{ props.rebaseConflictSource }}</span>
        onto
        <span class="text-[var(--foreground)] font-medium">{{ props.rebaseConflictTarget }}</span>.
      </p>
      <p class="text-xs text-[var(--muted-foreground)] leading-relaxed mb-4">
        Resolve conflicts in files, then choose Continue. Use Skip to drop the current patch, or Abort to cancel the rebase and restore the previous state.
      </p>

      <div class="flex items-center justify-end gap-2">
        <button
          @click="emit('submit:rebaseAbort')"
          :disabled="props.rebaseConflictBusy"
          class="px-3 py-1.5 text-xs rounded border border-[#ef4444]/40 text-[#ef4444] hover:bg-[#ef4444]/10 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          Abort
        </button>
        <button
          @click="emit('submit:rebaseSkip')"
          :disabled="props.rebaseConflictBusy"
          class="px-3 py-1.5 text-xs rounded border border-[var(--border)] text-[var(--foreground)] hover:bg-[var(--secondary)] disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          Skip
        </button>
        <button
          @click="emit('submit:rebaseContinue')"
          :disabled="props.rebaseConflictBusy"
          class="px-3 py-1.5 text-xs text-white bg-[var(--primary)] hover:opacity-90 rounded disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          Continue
        </button>
      </div>
    </div>
  </div>
</template>
