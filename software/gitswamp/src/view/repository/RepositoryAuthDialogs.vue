<script setup lang="ts">
const props = defineProps<{
  showPushUsernameDialog: boolean;
  pushPlatform: string;
  pushUsername: string;
  pushDomain: string;
  showAuthRequiredDialog: boolean;
  authProvider: "github" | "gitlab" | "gitlab-self" | "bitbucket" | "azure";
  authDomainInput: string;
  authTokenInput: string;
  authEmailInput: string;
  authKeyNameInput: string;
  authSubmitting: boolean;
}>();

const emit = defineEmits<{
  "update:showPushUsernameDialog": [value: boolean];
  "update:pushUsername": [value: string];
  "update:pushDomain": [value: string];
  "push": [];
  "update:showAuthRequiredDialog": [value: boolean];
  "update:authProvider": [value: "github" | "gitlab" | "gitlab-self" | "bitbucket" | "azure"];
  "update:authDomainInput": [value: string];
  "update:authTokenInput": [value: string];
  "update:authEmailInput": [value: string];
  "update:authKeyNameInput": [value: string];
  "saveAuthToken": [];
  "generateAndPushGitlabKey": [];
}>();

function onPushUsernameEnter() {
  emit("push");
}
</script>

<template>
  <div v-if="props.showPushUsernameDialog" class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" @click.self="emit('update:showPushUsernameDialog', false)">
    <div class="bg-[var(--popover)] border border-[var(--border)] rounded-lg p-6 w-96 shadow-2xl">
      <h3 class="text-sm font-medium text-[var(--foreground)] mb-4">Git {{ (props.pushPlatform === 'gitlab-self-hosted' || props.pushPlatform === 'gitlab-self') ? 'GitLab' : props.pushPlatform === 'github-enterprise' ? 'GitHub Enterprise' : props.pushPlatform === 'azure' ? 'Azure DevOps' : props.pushPlatform }} Credentials</h3>

      <div v-if="props.pushPlatform === 'gitlab-self-hosted' || props.pushPlatform === 'gitlab-self' || props.pushPlatform === 'github-enterprise' || props.pushPlatform === 'azure'" class="mb-4">
        <label for="push-domain" class="text-xs text-[var(--muted-foreground)] block mb-2">Domain (e.g., {{ props.pushPlatform === 'azure' ? 'dev.azure.com' : 'gitlab.company.com' }})</label>
        <input
          id="push-domain"
          :value="props.pushDomain"
          @input="emit('update:pushDomain', ($event.target as HTMLInputElement).value)"
          :placeholder="props.pushPlatform === 'azure' ? 'dev.azure.com' : 'gitlab.company.com'"
          class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40 mb-4"
        />
      </div>

      <div class="mb-4">
        <label for="push-username" class="text-xs text-[var(--muted-foreground)] block mb-2">Username</label>
        <input
          id="push-username"
          :value="props.pushUsername"
          @input="emit('update:pushUsername', ($event.target as HTMLInputElement).value)"
          :placeholder="`Your ${props.pushPlatform} username...`"
          class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
          @keyup.enter="onPushUsernameEnter"
          :autofocus="!(props.pushPlatform === 'gitlab-self-hosted' || props.pushPlatform === 'gitlab-self' || props.pushPlatform === 'github-enterprise' || props.pushPlatform === 'azure')"
        />
      </div>

      <div class="flex justify-end gap-2">
        <button @click="emit('update:showPushUsernameDialog', false)" class="px-3 py-1.5 text-xs text-[var(--muted-foreground)] hover:text-[var(--foreground)] rounded hover:bg-[var(--secondary)] transition-colors">Cancel</button>
        <button @click="emit('push')" :disabled="!props.pushUsername.trim() || (props.pushPlatform === 'gitlab-self-hosted' || props.pushPlatform === 'gitlab-self' || props.pushPlatform === 'github-enterprise' || props.pushPlatform === 'azure') && !props.pushDomain.trim()" class="px-3 py-1.5 text-xs text-white bg-[var(--primary)] hover:opacity-90 rounded disabled:opacity-50 transition-colors">Push</button>
      </div>
    </div>
  </div>

  <div v-if="props.showAuthRequiredDialog" class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" @click.self="emit('update:showAuthRequiredDialog', false)">
    <div class="bg-[var(--popover)] border border-[var(--border)] rounded-lg p-6 w-[460px] shadow-2xl">
      <h3 class="text-sm font-medium text-[var(--foreground)] mb-4">Authentication Required</h3>

      <div class="mb-3">
        <label for="auth-provider" class="text-xs text-[var(--muted-foreground)] block mb-2">Provider</label>
        <select
          id="auth-provider"
          :value="props.authProvider"
          @change="emit('update:authProvider', ($event.target as HTMLSelectElement).value as 'github' | 'gitlab' | 'gitlab-self' | 'bitbucket' | 'azure')"
          class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
        >
          <option value="github">GitHub</option>
          <option value="gitlab">GitLab.com</option>
          <option value="gitlab-self">GitLab self-hosted</option>
          <option value="bitbucket">Bitbucket</option>
          <option value="azure">Azure DevOps</option>
        </select>
      </div>

      <div v-if="props.authProvider === 'gitlab-self' || props.authProvider === 'azure'" class="mb-3">
        <label for="auth-domain" class="text-xs text-[var(--muted-foreground)] block mb-2">{{ props.authProvider === 'azure' ? 'Azure host domain' : 'GitLab domain' }}</label>
        <input
          id="auth-domain"
          :value="props.authDomainInput"
          @input="emit('update:authDomainInput', ($event.target as HTMLInputElement).value)"
          :placeholder="props.authProvider === 'azure' ? 'dev.azure.com/myorg' : 'gitlab.company.com'"
          class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
        />
      </div>

      <div class="mb-3">
        <label for="auth-token" class="text-xs text-[var(--muted-foreground)] block mb-2">Personal access token</label>
        <input
          id="auth-token"
          :value="props.authTokenInput"
          @input="emit('update:authTokenInput', ($event.target as HTMLInputElement).value)"
          type="password"
          placeholder="Paste token"
          class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
        />
      </div>

      <div v-if="props.authProvider === 'gitlab-self'" class="space-y-3 mb-4 border border-[var(--border)] rounded p-3 bg-[var(--card)]/30">
        <div class="text-[10px] text-[var(--muted-foreground)] uppercase tracking-wide">Generate and push SSH key (optional)</div>
        <input
          :value="props.authEmailInput"
          @input="emit('update:authEmailInput', ($event.target as HTMLInputElement).value)"
          placeholder="Email for SSH key"
          class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
        />
        <input
          :value="props.authKeyNameInput"
          @input="emit('update:authKeyNameInput', ($event.target as HTMLInputElement).value)"
          placeholder="Key name (default: gitswamp)"
          class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
        />
        <button
          class="w-full px-3 py-2 text-xs text-white bg-[#f59e0b] hover:opacity-90 rounded disabled:opacity-50 transition-colors"
          :disabled="props.authSubmitting || !props.authDomainInput.trim() || !props.authTokenInput.trim() || !props.authEmailInput.trim()"
          @click="emit('generateAndPushGitlabKey')"
        >
          Generate & Push SSH Key
        </button>
      </div>

      <div class="flex justify-end gap-2">
        <button @click="emit('update:showAuthRequiredDialog', false)" class="px-3 py-1.5 text-xs text-[var(--muted-foreground)] hover:text-[var(--foreground)] rounded hover:bg-[var(--secondary)] transition-colors">Cancel</button>
        <button
          @click="emit('saveAuthToken')"
          :disabled="props.authSubmitting || !props.authTokenInput.trim() || ((props.authProvider === 'gitlab-self' || props.authProvider === 'azure') && !props.authDomainInput.trim())"
          class="px-3 py-1.5 text-xs text-white bg-[var(--primary)] hover:opacity-90 rounded disabled:opacity-50 transition-colors"
        >
          Save Token
        </button>
      </div>
    </div>
  </div>
</template>
