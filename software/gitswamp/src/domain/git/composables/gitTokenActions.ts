import { callTauri } from "./gitCall";
import type { GitState } from "./gitState";

export function createTokenActions(state: GitState) {
  async function loadSavedToken() {
    try {
      const token = await callTauri<string | null>("load_token");
      state.githubToken.value = token || null;
      if (token) state.providerTokens.value.github = token;
    } catch {
      state.githubToken.value = null;
    }
  }

  async function loadProviderTokens() {
    const providers = [
      "github",
      "gitlab",
      "bitbucket",
      "azure",
      "github-enterprise",
      "gitlab-self",
      "bitbucket-dc",
    ];

    for (const p of providers) {
      try {
        const token = await callTauri<string | null>("load_provider_token", { provider: p });
        state.providerTokens.value[p] = token || null;
      } catch {
        state.providerTokens.value[p] = null;
      }
    }
  }

  async function loadGitPath() {
    try {
      state.gitPath.value = await callTauri<string>("get_git_path");
    } catch {
      state.gitPath.value = "not found";
    }
  }

  async function saveToken(token: string) {
    try {
      await callTauri("save_token", { token });
      state.githubToken.value = token;
      state.providerTokens.value.github = token;
    } catch (e) {
      state.error.value = String(e);
    }
  }

  async function deleteToken() {
    try {
      await callTauri("delete_token");
      state.githubToken.value = null;
    } catch (e) {
      state.error.value = String(e);
    }
  }

  async function saveProviderToken(provider: string, token: string) {
    try {
      await callTauri("save_provider_token", { provider, token });
      state.providerTokens.value[provider] = token;
    } catch (e) {
      state.error.value = String(e);
    }
  }

  async function deleteProviderToken(provider: string) {
    try {
      await callTauri("delete_provider_token", { provider });
      state.providerTokens.value[provider] = null;
    } catch (e) {
      state.error.value = String(e);
    }
  }

  return {
    loadSavedToken,
    loadProviderTokens,
    loadGitPath,
    saveToken,
    deleteToken,
    saveProviderToken,
    deleteProviderToken,
  };
}
