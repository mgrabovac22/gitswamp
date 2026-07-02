<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  APP_THEME_OPTIONS,
  applyAppPalettePreference,
  applyThemeModePreference,
  getStoredAppPalettePreference,
  getStoredThemeModePreference,
  storeAppPalettePreference,
  storeThemeModePreference,
  type AppPalettePreference,
  type AppThemeOption,
  type ThemeModePreference,
} from "@/shared/themePreferences";
import {
  getStoredCommitAnalyzerSettings,
  updateCommitAnalyzerSettings,
} from "@/shared/config/commitAnalyzerPreferences";
import {
  getStoredSmartGitignoreWizardEnabled,
  storeSmartGitignoreWizardEnabled,
} from "@/shared/config/gitignoreWizardPreferences";
import { useGit } from "@/domain/git/UseGit";
import { useToast } from "@/shared/notifications/useToast";
import AppButton from "@/shared/ui/AppButton.vue";
import AzureDevOpsIcon from "@/shared/ui/AzureDevOpsIcon.vue";
import BitbucketIcon from "@/shared/ui/BitbucketIcon.vue";
import {
  Check,
  ExternalLink,
  Building2,
  GitBranch,
  KeyRound,
  Layout,
  Loader2,
  Monitor,
  Moon,
  Sun,
  FlaskConical,
  PlugZap,
  RefreshCw,
  Settings2,
  Shield,
  SlidersHorizontal,
  Trash2,
  Users,
  X,
} from "lucide-vue-next";
type OptionsSection = "integrations" | "git" | "preferences" | "advanced" | "organisations";
type IntegrationPlatform = "github" | "gitlab" | "bitbucket" | "azure";
type FontSizePreference = "tiny" | "small" | "medium" | "largest" | "huge" | "xlarge" | "xxlarge" | "xxxlarge" | "xxxxlarge";
type OrganisationProvider = "github" | "gitlab" | "bitbucket" | "azure";
type OrganisationSearchProvider = OrganisationProvider | "all";
type OrganisationVisibilityFilter = "all" | "private" | "public";
type OrganisationSortMode = "stars-desc" | "name-asc";

interface GithubRepo {
  full_name: string;
  clone_url: string;
  description: string;
  is_private: boolean;
  stars: number;
}

interface GitlabRepo {
  full_name: string;
  path_with_namespace: string;
  clone_url_ssh: string;
  clone_url_https: string;
  description: string;
  is_private: boolean;
  stars: number;
}

interface BitbucketRepo {
  full_name: string;
  clone_url_ssh: string;
  clone_url_https: string;
  description: string;
  is_private: boolean;
  stars: number;
}

interface AzureRepo {
  full_name: string;
  clone_url_ssh: string;
  clone_url_https: string;
  description: string;
  is_private: boolean;
  stars: number;
}

interface GithubSshKey {
  id: number;
  title: string;
  key: string;
  fingerprint?: string;
  created_at?: string;
}

interface OrganisationRepoCandidate {
  id: string;
  provider: OrganisationProvider;
  fullName: string;
  cloneUrlHttps: string;
  cloneUrlSsh: string;
  webUrl: string;
  description: string;
  isPrivate: boolean;
  stars: number;
}

interface OrganisationProfile {
  id: string;
  provider: OrganisationProvider;
  organisation: string;
  team: string;
  repositoryFilter: string;
}

const props = defineProps<{
  gitPath?: string;
  initialSection?: OptionsSection;
}>();

const emit = defineEmits<{
  close: [];
}>();

const git = useGit();
const toast = useToast();

const activeSection = ref<OptionsSection>(props.initialSection || "integrations");
const activePlatform = ref<IntegrationPlatform>("github");
const optionsScrollRef = ref<HTMLDivElement | null>(null);
let isHydrating = true;

const appThemeOptions = APP_THEME_OPTIONS;
const darkThemeOptions = appThemeOptions.filter((theme) => theme.group === "dark");
const lightThemeOptions = appThemeOptions.filter((theme) => theme.group === "light");
const DEFAULT_DARK_PALETTE: AppPalettePreference = "default";
const DEFAULT_LIGHT_PALETTE: AppPalettePreference = "default-light";

const themeMode = ref<ThemeModePreference>("dark");
const appPalette = ref<AppPalettePreference>("default");
const fontSize = ref<FontSizePreference>("medium");
const generalFontSizePx = ref(18);
const compactMode = ref(false);
const dummyMode = ref(false);
const showAvatars = ref(true);
const restoreSession = ref(true);
const reducedMotion = ref(false);
const wrapDiffLines = ref(false);
const showDiffLineNumbers = ref(true);
const notifyGitkeep = ref(true);
const smartGitignoreWizardEnabled = ref(false);
const commitAnalyzerEnabled = ref(true);
const disableGraphAnimations = ref(false);
const smoothGraphScroll = ref(false);
const autoFetchEnabled = ref(true);
const autoFetchIntervalMinutes = ref(3);

const gitPathBusy = ref(false);
const installGitBusy = ref(false);

const organisationProvider = ref<OrganisationSearchProvider>("github");
const organisationQuery = ref("");
const organisationDestination = ref(String.raw`C:\Repozitoriji`);
const organisationRepos = ref<OrganisationRepoCandidate[]>([]);
const organisationSelectedRepoIds = ref<string[]>([]);
const organisationCloneProtocol = ref<"https" | "ssh">("https");
const organisationVisibilityFilter = ref<OrganisationVisibilityFilter>("all");
const organisationSortMode = ref<OrganisationSortMode>("stars-desc");
const organisationLocalFilter = ref("");
const organisationLoading = ref(false);
const organisationError = ref<string | null>(null);
const organisationCloneBusy = ref(false);
const organisationCloneDone = ref(0);
const organisationCloneFailed = ref(0);

const organisationProfiles = ref<OrganisationProfile[]>([]);
const organisationFormProvider = ref<OrganisationProvider>("github");
const organisationFormName = ref("");
const organisationFormTeam = ref("");
const organisationFormRepoFilter = ref("");

const OPTIONS_ORGANISATION_PROFILES_KEY = "gitswamp-organisation-profiles";
const OPTIONS_SMOOTH_GRAPH_SCROLL_KEY = "gitswamp-graph-smooth-scroll";
const OPTIONS_AUTO_FETCH_ENABLED_KEY = "gitswamp-auto-fetch-enabled";
const OPTIONS_AUTO_FETCH_INTERVAL_KEY = "gitswamp-auto-fetch-interval-minutes";
const OPTIONS_GENERAL_FONT_SIZE_KEY = "gitswamp-general-font-size";
const OPTIONS_COMMIT_FONT_SIZE_KEY = "gitswamp-font-size";

const fontSizes = {
  tiny: "14px",
  small: "16px",
  medium: "18px",
  largest: "22px",
  huge: "24px",
  xlarge: "26px",
  xxlarge: "28px",
  xxxlarge: "30px",
  xxxxlarge: "32px",
};

function normalizeFontSizePreference(value: string | null): FontSizePreference {
  if (value === "tiny") return "tiny";
  if (value === "small") return "small";
  if (value === "medium") return "medium";
  if (value === "largest") return "largest";
  if (value === "huge") return "huge";
  if (value === "xlarge") return "xlarge";
  if (value === "xxlarge") return "xxlarge";
  if (value === "xxxlarge") return "xxxlarge";
  if (value === "xxxxlarge") return "xxxxlarge";
  if (value === "large") return "largest";
  return "medium";
}

function applyFontScaleClasses(value: FontSizePreference): void {
  document.documentElement.classList.remove(
    "font-scale-tiny",
    "font-scale-small",
    "font-scale-medium",
    "font-scale-largest",
    "font-scale-huge",
    "font-scale-xlarge",
    "font-scale-xxlarge",
    "font-scale-xxxlarge",
    "font-scale-xxxxlarge",
  );
  document.documentElement.classList.add(`font-scale-${value}`);
}

const commitFontScaleOrder: FontSizePreference[] = ["tiny", "small", "medium", "largest", "huge", "xlarge", "xxlarge", "xxxlarge", "xxxxlarge"];

const commitFontSliderValue = computed(() => {
  const index = commitFontScaleOrder.indexOf(fontSize.value);
  return index >= 0 ? index : 2;
});

const commitFontLabel = computed(() => {
  if (fontSize.value === "tiny") return "Tiny";
  if (fontSize.value === "small") return "Small";
  if (fontSize.value === "medium") return "Medium";
  if (fontSize.value === "largest") return "Large";
  if (fontSize.value === "xlarge") return "X-Large";
  if (fontSize.value === "xxlarge") return "XX-Large";
  if (fontSize.value === "xxxlarge") return "XXX-Large";
  if (fontSize.value === "xxxxlarge") return "XXXX-Large";
  return "Huge";
});

const generalFontLabel = computed(() => `${generalFontSizePx.value}px`);
const selectedOrganisationCount = computed(() => organisationSelectedRepoIds.value.length);

const filteredOrganisationRepos = computed(() => {
  const query = organisationLocalFilter.value.trim().toLowerCase();

  let repos = organisationRepos.value.filter((repo) => {
    if (organisationVisibilityFilter.value === "private") {
      return repo.isPrivate;
    }

    if (organisationVisibilityFilter.value === "public") {
      return !repo.isPrivate;
    }

    return true;
  });

  if (query) {
    repos = repos.filter((repo) => {
      const haystack = `${repo.fullName} ${repo.description} ${repo.provider}`.toLowerCase();
      return haystack.includes(query);
    });
  }

  const sorted = [...repos];
  if (organisationSortMode.value === "name-asc") {
    sorted.sort((a, b) => a.fullName.localeCompare(b.fullName));
  } else {
    sorted.sort((a, b) => {
      if (b.stars !== a.stars) {
        return b.stars - a.stars;
      }
      return a.fullName.localeCompare(b.fullName);
    });
  }

  return sorted;
});

const allVisibleOrganisationReposSelected = computed(() => {
  if (filteredOrganisationRepos.value.length === 0) {
    return false;
  }

  const selected = new Set(organisationSelectedRepoIds.value);
  return filteredOrganisationRepos.value.every((repo) => selected.has(repo.id));
});

const currentGitPath = computed(() => props.gitPath || git.gitPath.value || "");
const hasDetectedGit = computed(() => !!currentGitPath.value);

function clampGeneralFontSize(value: number): number {
  if (!Number.isFinite(value)) return 18;
  return Math.max(10, Math.min(40, Math.round(value)));
}

function setCommitFontFromSlider(event: Event): void {
  const raw = Number((event.target as HTMLInputElement).value);
  const index = Math.max(0, Math.min(commitFontScaleOrder.length - 1, Number.isFinite(raw) ? Math.round(raw) : 2));
  fontSize.value = commitFontScaleOrder[index];
}

function setGeneralFontFromSlider(event: Event): void {
  generalFontSizePx.value = clampGeneralFontSize(Number((event.target as HTMLInputElement).value));
}

function normalizeAutoFetchIntervalMinutes(value: string | null): number {
  const parsed = Number.parseInt(value || "", 10);
  if (!Number.isFinite(parsed)) {
    return 3;
  }
  return Math.max(1, Math.min(60, parsed));
}

function emitAutoFetchSettingsChanged() {
  globalThis.dispatchEvent(
    new CustomEvent("gitswamp:auto-fetch-settings-changed", {
      detail: {
        enabled: autoFetchEnabled.value,
        intervalMinutes: autoFetchIntervalMinutes.value,
      },
    }),
  );
}

function readOrganisationProfilesFromStorage(): OrganisationProfile[] {
  const raw = localStorage.getItem(OPTIONS_ORGANISATION_PROFILES_KEY);
  if (!raw) {
    return [];
  }

  try {
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) {
      return [];
    }

    return parsed
      .map((item) => {
        const provider = String(item?.provider || "") as OrganisationProvider;
        if (!["github", "gitlab", "bitbucket", "azure"].includes(provider)) {
          return null;
        }

        return {
          id: String(item?.id || ""),
          provider,
          organisation: String(item?.organisation || ""),
          team: String(item?.team || ""),
          repositoryFilter: String(item?.repositoryFilter || ""),
        } satisfies OrganisationProfile;
      })
      .filter((item): item is OrganisationProfile => !!item && item.id.length > 0);
  } catch {
    return [];
  }
}

function persistOrganisationProfiles(): void {
  localStorage.setItem(OPTIONS_ORGANISATION_PROFILES_KEY, JSON.stringify(organisationProfiles.value));
}

function addOrganisationProfile(): void {
  const organisation = organisationFormName.value.trim();
  const team = organisationFormTeam.value.trim();
  const repositoryFilter = organisationFormRepoFilter.value.trim();

  if (!organisation) {
    toast.error("Organisation name is required.");
    return;
  }

  const profile: OrganisationProfile = {
    id: `${Date.now()}-${Math.random().toString(16).slice(2, 10)}`,
    provider: organisationFormProvider.value,
    organisation,
    team,
    repositoryFilter,
  };

  organisationProfiles.value = [profile, ...organisationProfiles.value].slice(0, 40);
  persistOrganisationProfiles();

  organisationFormName.value = "";
  organisationFormTeam.value = "";
  organisationFormRepoFilter.value = "";
  toast.success("Organisation profile saved.");
}

function removeOrganisationProfile(profileId: string): void {
  organisationProfiles.value = organisationProfiles.value.filter((item) => item.id !== profileId);
  persistOrganisationProfiles();
}

function useOrganisationProfile(profile: OrganisationProfile): void {
  organisationProvider.value = profile.provider;
  organisationQuery.value = [profile.organisation, profile.team, profile.repositoryFilter].filter(Boolean).join(" ").trim();
  organisationLocalFilter.value = profile.repositoryFilter.trim();
  organisationVisibilityFilter.value = "all";
}

function getOrganisationProviderToken(provider: OrganisationProvider): string | null {
  if (provider === "github") {
    return currentGithubToken() || null;
  }
  if (provider === "gitlab") {
    return git.providerTokens.value.gitlab || null;
  }
  if (provider === "bitbucket") {
    return git.providerTokens.value.bitbucket || null;
  }
  if (provider === "azure") {
    return git.providerTokens.value.azure || null;
  }
  return null;
}

function stripGitSuffix(url: string): string {
  return url.replace(/\.git$/i, "");
}

function dedupeOrganisationRepos(items: OrganisationRepoCandidate[]): OrganisationRepoCandidate[] {
  const merged = new Map<string, OrganisationRepoCandidate>();

  for (const repo of items) {
    const existing = merged.get(repo.id);
    if (!existing) {
      merged.set(repo.id, repo);
      continue;
    }

    if (repo.stars > existing.stars) {
      merged.set(repo.id, repo);
    }
  }

  return Array.from(merged.values());
}

function getOrganisationRepoSortRank(repo: OrganisationRepoCandidate, query: string): number {
  const normalizedQuery = query.trim().toLowerCase();
  const normalizedFullName = repo.fullName.toLowerCase();
  const [ownerPart = "", repoPart = ""] = repo.fullName.split("/");
  const normalizedOwnerPart = ownerPart.toLowerCase();
  const normalizedRepoPart = repoPart.toLowerCase();

  if (!normalizedQuery) {
    return 2;
  }

  if (normalizedOwnerPart.includes(normalizedQuery)) {
    return 0;
  }

  if (normalizedFullName.includes(normalizedQuery) || normalizedRepoPart.includes(normalizedQuery)) {
    return 1;
  }

  return 2;
}

function sortOrganisationRepos(items: OrganisationRepoCandidate[], query: string): OrganisationRepoCandidate[] {
  return [...items].sort((left, right) => {
    const leftRank = getOrganisationRepoSortRank(left, query);
    const rightRank = getOrganisationRepoSortRank(right, query);
    if (leftRank !== rightRank) {
      return leftRank - rightRank;
    }

    if (left.provider !== right.provider) {
      return left.provider.localeCompare(right.provider);
    }

    if (right.stars !== left.stars) {
      return right.stars - left.stars;
    }

    return left.fullName.localeCompare(right.fullName);
  });
}

function mappedOrganisationReposFromGithub(items: GithubRepo[]): OrganisationRepoCandidate[] {
  return items.map((repo) => ({
    id: `github:${repo.full_name}`,
    provider: "github",
    fullName: repo.full_name,
    cloneUrlHttps: repo.clone_url,
    cloneUrlSsh: `git@github.com:${repo.full_name}.git`,
    webUrl: `https://github.com/${repo.full_name}`,
    description: repo.description,
    isPrivate: repo.is_private,
    stars: repo.stars,
  }));
}

function mappedOrganisationReposFromGitlab(items: GitlabRepo[]): OrganisationRepoCandidate[] {
  return items.map((repo) => ({
    id: `gitlab:${repo.full_name}`,
    provider: "gitlab",
    fullName: repo.full_name,
    cloneUrlHttps: repo.clone_url_https,
    cloneUrlSsh: repo.clone_url_ssh,
    webUrl: `https://gitlab.com/${repo.path_with_namespace}`,
    description: repo.description,
    isPrivate: repo.is_private,
    stars: repo.stars,
  }));
}

function mappedOrganisationReposFromBitbucket(items: BitbucketRepo[]): OrganisationRepoCandidate[] {
  return items.map((repo) => ({
    id: `bitbucket:${repo.full_name}`,
    provider: "bitbucket",
    fullName: repo.full_name,
    cloneUrlHttps: repo.clone_url_https,
    cloneUrlSsh: repo.clone_url_ssh,
    webUrl: `https://bitbucket.org/${repo.full_name}`,
    description: repo.description,
    isPrivate: repo.is_private,
    stars: repo.stars,
  }));
}

function mappedOrganisationReposFromAzure(items: AzureRepo[]): OrganisationRepoCandidate[] {
  return items.map((repo) => ({
    id: `azure:${repo.full_name}`,
    provider: "azure",
    fullName: repo.full_name,
    cloneUrlHttps: repo.clone_url_https,
    cloneUrlSsh: repo.clone_url_ssh,
    webUrl: stripGitSuffix(repo.clone_url_https),
    description: repo.description,
    isPrivate: repo.is_private,
    stars: repo.stars,
  }));
}

async function searchSingleOrganisationProvider(provider: OrganisationProvider, query: string): Promise<OrganisationRepoCandidate[]> {
  const token = getOrganisationProviderToken(provider);
  if (!token) {
    throw new Error(`${provider.toUpperCase()} token is not configured. Add it in Integrations first.`);
  }

  if (provider === "github") {
    const repos = await invoke<GithubRepo[]>("search_github_repos", { token, query, includePublic: true });
    return mappedOrganisationReposFromGithub(repos);
  }

  if (provider === "gitlab") {
    const repos = await invoke<GitlabRepo[]>("search_gitlab_repos", {
      domain: "gitlab.com",
      token,
      query,
    });
    return mappedOrganisationReposFromGitlab(repos);
  }

  if (provider === "bitbucket") {
    const repos = await invoke<BitbucketRepo[]>("search_bitbucket_repos", { token, query });
    return mappedOrganisationReposFromBitbucket(repos);
  }

  const domain = normalizeAzureDomainInput(azureDomainInput.value || git.providerTokens.value["azure-domain"] || "");
  if (!domain) {
    throw new Error("Azure domain is not configured. Save it in Integrations first.");
  }

  const repos = await invoke<AzureRepo[]>("search_azure_repos", {
    domain,
    token,
    query,
  });
  return mappedOrganisationReposFromAzure(repos);
}

async function searchAllOrganisationProviders(query: string): Promise<OrganisationRepoCandidate[]> {
  const providers: OrganisationProvider[] = ["github", "gitlab", "bitbucket", "azure"];
  const activeProviders = providers.filter((provider) => {
    if (provider === "azure") {
      const hasToken = !!getOrganisationProviderToken("azure");
      const domain = normalizeAzureDomainInput(azureDomainInput.value || git.providerTokens.value["azure-domain"] || "");
      return hasToken && !!domain;
    }
    return !!getOrganisationProviderToken(provider);
  });

  if (!activeProviders.length) {
    throw new Error("No provider credentials are configured. Add at least one provider token in Integrations first.");
  }

  const settled = await Promise.allSettled(activeProviders.map((provider) => searchSingleOrganisationProvider(provider, query)));
  const combined: OrganisationRepoCandidate[] = [];
  const failedProviders: string[] = [];

  for (let idx = 0; idx < settled.length; idx += 1) {
    const result = settled[idx];
    const provider = activeProviders[idx];

    if (result.status === "fulfilled") {
      combined.push(...result.value);
      continue;
    }

    failedProviders.push(provider);
  }

  if (!combined.length && failedProviders.length) {
    throw new Error(`Search failed for configured providers: ${failedProviders.join(", ")}`);
  }

  if (failedProviders.length) {
    toast.warning(`Some providers failed during search: ${failedProviders.join(", ")}`);
  }

  return dedupeOrganisationRepos(combined);
}

async function searchOrganisationRepos(): Promise<void> {
  const provider = organisationProvider.value;
  const query = organisationQuery.value.trim();
  organisationLoading.value = true;
  organisationError.value = null;
  organisationSelectedRepoIds.value = [];

  try {
    const repos = provider === "all"
      ? await searchAllOrganisationProviders(query)
      : await searchSingleOrganisationProvider(provider, query);

    organisationRepos.value = sortOrganisationRepos(repos, query).slice(0, 240);
  } catch (e) {
    organisationError.value = String(e);
    organisationRepos.value = [];
  } finally {
    organisationLoading.value = false;
  }
}

function toggleSelectAllOrganisationRepos(): void {
  const visibleRepoIds = filteredOrganisationRepos.value.map((repo) => repo.id);
  if (!visibleRepoIds.length) {
    return;
  }

  const selected = new Set(organisationSelectedRepoIds.value);
  if (allVisibleOrganisationReposSelected.value) {
    for (const repoId of visibleRepoIds) {
      selected.delete(repoId);
    }
  } else {
    for (const repoId of visibleRepoIds) {
      selected.add(repoId);
    }
  }

  organisationSelectedRepoIds.value = Array.from(selected);
}

function resolveOrganisationCloneUrl(repo: OrganisationRepoCandidate): string {
  if (organisationCloneProtocol.value === "ssh" && repo.cloneUrlSsh) {
    return repo.cloneUrlSsh;
  }

  return repo.cloneUrlHttps || repo.cloneUrlSsh;
}

function isSshCloneUrl(url: string): boolean {
  return url.startsWith("git@") || url.startsWith("ssh://");
}

async function copyOrganisationCloneUrl(repo: OrganisationRepoCandidate): Promise<void> {
  const cloneUrl = resolveOrganisationCloneUrl(repo);
  if (!cloneUrl) {
    toast.error("Clone URL is not available for this repository.");
    return;
  }

  try {
    await navigator.clipboard.writeText(cloneUrl);
    toast.success("Clone URL copied.");
  } catch {
    toast.error("Failed to copy clone URL.");
  }
}

function openOrganisationRepository(repo: OrganisationRepoCandidate): void {
  if (!repo.webUrl) {
    toast.error("Web URL is not available for this repository.");
    return;
  }

  openUrl(repo.webUrl).catch(() => {
    toast.error("Failed to open repository URL.");
  });
}

async function browseOrganisationDestination(): Promise<void> {
  const selected = await openDialog({
    directory: true,
    multiple: false,
    title: "Select base folder for batch clone",
  });

  if (selected && !Array.isArray(selected)) {
    organisationDestination.value = selected;
  }
}

async function cloneSelectedOrganisationRepos(): Promise<void> {
  const selectedRepos = organisationRepos.value.filter((repo) => organisationSelectedRepoIds.value.includes(repo.id));
  if (!selectedRepos.length) {
    toast.warning("Select at least one repository for cloning.");
    return;
  }

  const destination = organisationDestination.value.trim();
  if (!destination) {
    toast.error("Choose a destination directory first.");
    return;
  }

  organisationCloneBusy.value = true;
  organisationCloneDone.value = 0;
  organisationCloneFailed.value = 0;

  const maxWorkers = Math.min(2, selectedRepos.length);
  let pointer = 0;

  const runWorker = async () => {
    while (pointer < selectedRepos.length) {
      const repoIndex = pointer;
      pointer += 1;
      const repo = selectedRepos[repoIndex];

      try {
        const cloneUrl = resolveOrganisationCloneUrl(repo);
        if (!cloneUrl) {
          throw new Error("Missing clone URL");
        }

        const token = isSshCloneUrl(cloneUrl) ? null : getOrganisationProviderToken(repo.provider);

        await invoke<string>("clone_repo", {
          url: cloneUrl,
          path: destination,
          shallow: false,
          token: token || null,
        });
      } catch {
        organisationCloneFailed.value += 1;
      } finally {
        organisationCloneDone.value += 1;
      }
    }
  };

  try {
    await Promise.all(Array.from({ length: maxWorkers }, () => runWorker()));
    if (organisationCloneFailed.value === 0) {
      toast.success(`Batch clone finished. ${organisationCloneDone.value} repositories cloned.`);
    } else {
      toast.warning(`Batch clone finished with failures (${organisationCloneFailed.value}/${organisationCloneDone.value}).`);
    }
  } finally {
    organisationCloneBusy.value = false;
  }
}

async function refreshGitPath(silent = false): Promise<void> {
  gitPathBusy.value = true;
  try {
    const path = (await invoke<string>("get_git_path")).trim();
    git.gitPath.value = path;
    if (!silent && path) {
      toast.success("Git installation detected.");
    }
    if (!silent && !path) {
      toast.warning("Git was not detected. Download or install Git from this section.");
    }
  } catch (e) {
    if (!silent) {
      toast.error("Unable to detect Git path: " + String(e));
    }
  } finally {
    gitPathBusy.value = false;
  }
}

function openGitDownload(): void {
  openUrl("https://git-scm.com/downloads").catch(() => {
    toast.error("Failed to open Git download page.");
  });
}

async function installGit(): Promise<void> {
  installGitBusy.value = true;
  try {
    const message = await invoke<string>("install_git");
    toast.success(message || "Git installation flow started.");
    await refreshGitPath(true);
  } catch (e) {
    toast.error("Git installation failed: " + String(e));
  } finally {
    installGitBusy.value = false;
  }
}

const githubTokenInput = ref("");
const gitlabTokenInput = ref("");
const bitbucketTokenInput = ref("");
const azureTokenInput = ref("");
const azureDomainInput = ref("");
const gitlabSelfTokenInput = ref("");
const gitlabSelfDomainInput = ref("");

const githubOauthBusy = ref(false);
const githubTokenBusy = ref(false);
const githubSshBusy = ref(false);
const githubKeysBusy = ref(false);
const githubUser = ref<string | null>(null);
const githubSshKeys = ref<GithubSshKey[]>([]);

const githubSshEmailInput = ref("");
const githubSshKeyNameInput = ref("gitswamp_github");
const githubGeneratedKeyTitleInput = ref("GitSwamp GitHub Key");
const githubExistingKeyTitleInput = ref("GitSwamp Existing Key");

const gitlabTokenBusy = ref(false);
const bitbucketTokenBusy = ref(false);
const azureTokenBusy = ref(false);
const gitlabSelfTokenBusy = ref(false);
const gitlabSelfSshBusy = ref(false);
const gitlabSelfEmailInput = ref("");
const gitlabSelfKeyNameInput = ref("gitswamp");

const hasGithubToken = computed(() => !!(git.providerTokens.value.github || git.githubToken.value));
const hasGitlabToken = computed(() => !!git.providerTokens.value.gitlab);
const hasBitbucketToken = computed(() => !!git.providerTokens.value.bitbucket);
const hasAzureToken = computed(() => !!git.providerTokens.value.azure);

const gitlabSelfState = computed(() => {
  const raw = git.providerTokens.value["gitlab-self"] || "";
  const separator = raw.indexOf("|");
  if (separator <= 0) {
    return { domain: "", token: "" };
  }

  return {
    domain: raw.slice(0, separator).trim(),
    token: raw.slice(separator + 1).trim(),
  };
});

const hasGitlabSelfToken = computed(() => !!gitlabSelfState.value.token);

function applySettings() {
  document.documentElement.style.setProperty("--font-size", `${generalFontSizePx.value}px`);
  applyFontScaleClasses(fontSize.value);
  document.documentElement.classList.toggle("compact", compactMode.value);
  document.documentElement.classList.toggle("dummy-mode", dummyMode.value);
  document.documentElement.classList.toggle("hide-avatars", !showAvatars.value);
  document.documentElement.classList.toggle("reduced-motion", reducedMotion.value);
  document.documentElement.classList.toggle("diff-wrap-lines", wrapDiffLines.value);
  document.documentElement.classList.toggle("hide-diff-line-numbers", !showDiffLineNumbers.value);
  document.documentElement.classList.toggle("graph-animations-off", disableGraphAnimations.value);
}

function normalizeHostInput(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) return "";

  const candidate = /^[a-z][a-z0-9+.-]*:\/\//i.test(trimmed) ? trimmed : `https://${trimmed}`;
  try {
    const parsed = new URL(candidate);
    return parsed.host.toLowerCase();
  } catch {
    const withoutProtocol = trimmed.replace(/^[a-z][a-z0-9+.-]*:\/\//i, "");
    return (withoutProtocol.split("/")[0] || "").trim().toLowerCase();
  }
}

function normalizeAzureDomainInput(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) return "";

  const candidate = /^[a-z][a-z0-9+.-]*:\/\//i.test(trimmed) ? trimmed : `https://${trimmed}`;
  try {
    const parsed = new URL(candidate);
    const host = parsed.host.toLowerCase();
    const pathSegments = parsed.pathname.split("/");
    const firstPathSegment = pathSegments.find((segment) => segment.length > 0) || "";
    if (host === "dev.azure.com") {
      const organization = firstPathSegment;
      return organization ? `${host}/${organization.toLowerCase()}` : host;
    }
    return host;
  } catch {
    const withoutProtocol = trimmed.replace(/^[a-z][a-z0-9+.-]*:\/\//i, "");
    const parts = withoutProtocol.split("/");
    const hostPart = parts.find((segment) => segment.trim().length > 0) || "";
    if (!hostPart) return "";

    const hostPartLower = hostPart.toLowerCase();
    if (hostPartLower === "dev.azure.com") {
      const orgPart = parts.find((segment, index) => index > 0 && segment.trim().length > 0) || "";
      return orgPart ? `${hostPartLower}/${orgPart.toLowerCase()}` : hostPartLower;
    }

    return hostPartLower;
  }
}

function currentGithubToken(): string {
  return git.providerTokens.value.github || git.githubToken.value || "";
}

async function assertGitActionOk(action: () => Promise<void>) {
  git.error.value = null;
  await action();
  if (git.error.value) {
    throw new Error(git.error.value);
  }
}

async function refreshGithubUser() {
  const token = currentGithubToken();
  if (!token) {
    githubUser.value = null;
    return;
  }

  try {
    const login = await invoke<string>("verify_github_token", { token });
    githubUser.value = login || null;
  } catch {
    githubUser.value = null;
  }
}

async function refreshGithubKeys() {
  const token = currentGithubToken();
  if (!token) {
    githubSshKeys.value = [];
    return;
  }

  githubKeysBusy.value = true;
  try {
    githubSshKeys.value = await invoke<GithubSshKey[]>("list_github_ssh_keys", { token });
  } catch (e) {
    toast.error("Failed to load GitHub SSH keys: " + String(e));
  } finally {
    githubKeysBusy.value = false;
  }
}

async function saveGithubToken() {
  const token = githubTokenInput.value.trim();
  if (!token) return;

  githubTokenBusy.value = true;
  try {
    await assertGitActionOk(async () => {
      await git.saveProviderToken("github", token);
      await git.reloadAuthTokens();
    });
    githubTokenInput.value = "";
    await refreshGithubUser();
    toast.success("GitHub token saved.");
  } catch (e) {
    toast.error("Failed to save GitHub token: " + String(e));
  } finally {
    githubTokenBusy.value = false;
  }
}

async function deleteGithubToken() {
  githubTokenBusy.value = true;
  try {
    await assertGitActionOk(async () => {
      await git.deleteProviderToken("github");
      await git.reloadAuthTokens();
    });
    githubTokenInput.value = "";
    githubSshKeys.value = [];
    githubUser.value = null;
    toast.success("GitHub token removed.");
  } catch (e) {
    toast.error("Failed to remove GitHub token: " + String(e));
  } finally {
    githubTokenBusy.value = false;
  }
}

async function connectGithubOAuth() {
  githubOauthBusy.value = true;
  try {
    const token = (await invoke<string>("connect_github_oauth_via_gh_cli")).trim();
    if (!token) {
      throw new Error("OAuth flow completed but token was empty.");
    }

    await assertGitActionOk(async () => {
      await git.saveProviderToken("github", token);
      await git.reloadAuthTokens();
    });

    githubTokenInput.value = "";
    await refreshGithubUser();
    toast.success("Connected to GitHub via OAuth.");
  } catch (e) {
    const message = String(e);
    if (message.toLowerCase().includes("github cli (gh) was not found")) {
      toast.warning("GitHub CLI is required for OAuth flow. Opening download page...");
      openUrl("https://cli.github.com/").catch(() => {});
    }
    toast.error("GitHub OAuth failed: " + message);
  } finally {
    githubOauthBusy.value = false;
  }
}

async function generateAndAddGithubSshKey() {
  const token = currentGithubToken();
  const email = githubSshEmailInput.value.trim();
  const keyName = githubSshKeyNameInput.value.trim() || "gitswamp_github";
  const title = githubGeneratedKeyTitleInput.value.trim() || `gitswamp-${Date.now()}`;

  if (!token) {
    toast.error("Save a GitHub token or connect via OAuth first.");
    return;
  }
  if (!email) {
    toast.error("Email is required to generate an SSH key.");
    return;
  }

  githubSshBusy.value = true;
  try {
    const generated = await invoke<[string, string]>("generate_ssh_key", {
      email,
      keyName,
    });

    await invoke("add_github_ssh_key", {
      token,
      title,
      key: generated[1],
    });

    await refreshGithubKeys();
    toast.success(`SSH key generated and added to GitHub (${generated[0]}).`);
  } catch (e) {
    toast.error("Failed to generate/add GitHub SSH key: " + String(e));
  } finally {
    githubSshBusy.value = false;
  }
}

async function addExistingGithubSshKey() {
  const token = currentGithubToken();
  if (!token) {
    toast.error("Save a GitHub token or connect via OAuth first.");
    return;
  }

  try {
    const selected = await openDialog({
      multiple: false,
      title: "Select existing SSH key",
      filters: [
        { name: "SSH Keys", extensions: ["pub", "key", "pem", "txt"] },
      ],
    });

    if (!selected || Array.isArray(selected)) {
      return;
    }

    githubSshBusy.value = true;
    const key = await invoke<string>("load_ssh_public_key_from_file", {
      filePath: selected,
    });

    await invoke("add_github_ssh_key", {
      token,
      title: githubExistingKeyTitleInput.value.trim() || `gitswamp-${Date.now()}`,
      key,
    });

    await refreshGithubKeys();
    toast.success("Existing SSH key added to GitHub.");
  } catch (e) {
    toast.error("Failed to add existing GitHub SSH key: " + String(e));
  } finally {
    githubSshBusy.value = false;
  }
}

async function deleteGithubSshKey(keyId: number) {
  const token = currentGithubToken();
  if (!token) return;

  githubKeysBusy.value = true;
  try {
    await invoke("delete_github_ssh_key", {
      token,
      keyId,
    });
    await refreshGithubKeys();
    toast.success("GitHub SSH key removed.");
  } catch (e) {
    toast.error("Failed to remove GitHub SSH key: " + String(e));
  } finally {
    githubKeysBusy.value = false;
  }
}

async function saveGitlabToken() {
  const token = gitlabTokenInput.value.trim();
  if (!token) return;

  gitlabTokenBusy.value = true;
  try {
    await assertGitActionOk(async () => {
      await git.saveProviderToken("gitlab", token);
      await git.reloadAuthTokens();
    });
    gitlabTokenInput.value = "";
    toast.success("GitLab token saved.");
  } catch (e) {
    toast.error("Failed to save GitLab token: " + String(e));
  } finally {
    gitlabTokenBusy.value = false;
  }
}

async function deleteGitlabToken() {
  gitlabTokenBusy.value = true;
  try {
    await assertGitActionOk(async () => {
      await git.deleteProviderToken("gitlab");
      await git.reloadAuthTokens();
    });
    gitlabTokenInput.value = "";
    toast.success("GitLab token removed.");
  } catch (e) {
    toast.error("Failed to remove GitLab token: " + String(e));
  } finally {
    gitlabTokenBusy.value = false;
  }
}

async function saveGitlabSelfToken() {
  const token = gitlabSelfTokenInput.value.trim();
  const domain = normalizeHostInput(gitlabSelfDomainInput.value);
  if (!token || !domain) {
    toast.error("Domain and token are required for GitLab self-managed.");
    return;
  }

  gitlabSelfTokenBusy.value = true;
  try {
    await assertGitActionOk(async () => {
      await git.saveProviderToken("gitlab-self", `${domain}|${token}`);
      await git.reloadAuthTokens();
    });
    gitlabSelfDomainInput.value = domain;
    gitlabSelfTokenInput.value = "";
    toast.success("GitLab self-managed token saved.");
  } catch (e) {
    toast.error("Failed to save GitLab self-managed token: " + String(e));
  } finally {
    gitlabSelfTokenBusy.value = false;
  }
}

async function deleteGitlabSelfToken() {
  gitlabSelfTokenBusy.value = true;
  try {
    await assertGitActionOk(async () => {
      await git.deleteProviderToken("gitlab-self");
      await git.reloadAuthTokens();
    });
    gitlabSelfTokenInput.value = "";
    toast.success("GitLab self-managed token removed.");
  } catch (e) {
    toast.error("Failed to remove GitLab self-managed token: " + String(e));
  } finally {
    gitlabSelfTokenBusy.value = false;
  }
}

async function generateAndPushGitlabSelfKey() {
  const token = gitlabSelfTokenInput.value.trim();
  const domain = normalizeHostInput(gitlabSelfDomainInput.value);
  const email = gitlabSelfEmailInput.value.trim();
  const keyName = gitlabSelfKeyNameInput.value.trim() || "gitswamp";

  if (!token || !domain || !email) {
    toast.error("Domain, token and email are required.");
    return;
  }

  gitlabSelfSshBusy.value = true;
  try {
    const generated = await invoke<[string, string]>("generate_ssh_key", {
      email,
      keyName,
    });

    await invoke("add_gitlab_ssh_key", {
      domain,
      token,
      title: `gitswamp-${Date.now()}`,
      key: generated[1],
    });

    await assertGitActionOk(async () => {
      await git.saveProviderToken("gitlab-self", `${domain}|${token}`);
      await git.reloadAuthTokens();
    });

    gitlabSelfTokenInput.value = "";
    toast.success(`SSH key generated and pushed to GitLab self-managed (${generated[0]}).`);
  } catch (e) {
    toast.error("Failed to generate/push GitLab SSH key: " + String(e));
  } finally {
    gitlabSelfSshBusy.value = false;
  }
}

async function saveBitbucketToken() {
  const token = bitbucketTokenInput.value.trim();
  if (!token) return;

  bitbucketTokenBusy.value = true;
  try {
    await assertGitActionOk(async () => {
      await git.saveProviderToken("bitbucket", token);
      await git.reloadAuthTokens();
    });
    bitbucketTokenInput.value = "";
    toast.success("Bitbucket token saved.");
  } catch (e) {
    toast.error("Failed to save Bitbucket token: " + String(e));
  } finally {
    bitbucketTokenBusy.value = false;
  }
}

async function deleteBitbucketToken() {
  bitbucketTokenBusy.value = true;
  try {
    await assertGitActionOk(async () => {
      await git.deleteProviderToken("bitbucket");
      await git.reloadAuthTokens();
    });
    bitbucketTokenInput.value = "";
    toast.success("Bitbucket token removed.");
  } catch (e) {
    toast.error("Failed to remove Bitbucket token: " + String(e));
  } finally {
    bitbucketTokenBusy.value = false;
  }
}

async function saveAzureToken() {
  const token = azureTokenInput.value.trim();
  const domain = normalizeAzureDomainInput(azureDomainInput.value);
  if (!token || !domain) {
    toast.error("Azure token and domain are required (for example dev.azure.com/myorg).");
    return;
  }

  azureTokenBusy.value = true;
  try {
    await assertGitActionOk(async () => {
      await git.saveProviderToken("azure", token);
      await git.saveProviderToken("azure-domain", domain);
      await git.reloadAuthTokens();
    });
    azureTokenInput.value = "";
    azureDomainInput.value = domain;
    toast.success("Azure DevOps token saved.");
  } catch (e) {
    toast.error("Failed to save Azure DevOps token: " + String(e));
  } finally {
    azureTokenBusy.value = false;
  }
}

async function deleteAzureToken() {
  azureTokenBusy.value = true;
  try {
    await assertGitActionOk(async () => {
      await git.deleteProviderToken("azure");
      await git.deleteProviderToken("azure-domain");
      await git.reloadAuthTokens();
    });
    azureTokenInput.value = "";
    toast.success("Azure DevOps token removed.");
  } catch (e) {
    toast.error("Failed to remove Azure DevOps token: " + String(e));
  } finally {
    azureTokenBusy.value = false;
  }
}

function onThemePresetChange(event: Event) {
  const selected = (event.target as HTMLSelectElement).value as AppPalettePreference;
  const selectedTheme = appThemeOptions.find((theme) => theme.id === selected);
  if (!selectedTheme) return;

  appPalette.value = selectedTheme.id;
  if (themeMode.value !== selectedTheme.mode) {
    themeMode.value = selectedTheme.mode;
  }
}

function themeLabel(theme: AppThemeOption): string {
  return theme.mode === "dark" ? `${theme.label} (Dark)` : `${theme.label} (Light)`;
}

function toggleThemeMode() {
  const nextMode: ThemeModePreference = themeMode.value === "dark" ? "light" : "dark";
  themeMode.value = nextMode;
  appPalette.value = nextMode === "dark" ? DEFAULT_DARK_PALETTE : DEFAULT_LIGHT_PALETTE;
}

function formatGithubKeyMeta(item: GithubSshKey): string {
  const timestamp = item.created_at ? new Date(item.created_at).toLocaleDateString() : "Unknown date";
  const fp = item.fingerprint || "No fingerprint";
  return `${fp} • ${timestamp}`;
}

function resetOptionsScroll(): void {
  if (!optionsScrollRef.value) return;
  optionsScrollRef.value.scrollTo({ top: 0, behavior: "auto" });
}

onMounted(() => {
  themeMode.value = getStoredThemeModePreference();
  appPalette.value = getStoredAppPalettePreference();

  fontSize.value = normalizeFontSizePreference(localStorage.getItem(OPTIONS_COMMIT_FONT_SIZE_KEY));

  const savedGeneralFont = localStorage.getItem(OPTIONS_GENERAL_FONT_SIZE_KEY);
  if (savedGeneralFont === null) {
    generalFontSizePx.value = clampGeneralFontSize(Number.parseInt(fontSizes[fontSize.value], 10));
  } else {
    generalFontSizePx.value = clampGeneralFontSize(Number.parseInt(savedGeneralFont, 10));
  }

  const savedCompact = localStorage.getItem("gitswamp-compact-mode");
  if (savedCompact) compactMode.value = savedCompact === "true";

  const savedDummyMode = localStorage.getItem("gitswamp-dummy-mode");
  if (savedDummyMode) dummyMode.value = savedDummyMode === "true";

  const savedAvatars = localStorage.getItem("gitswamp-show-avatars");
  if (savedAvatars !== null) showAvatars.value = savedAvatars !== "false";

  const savedRestoreSession = localStorage.getItem("gitswamp-restore-session");
  if (savedRestoreSession !== null) restoreSession.value = savedRestoreSession !== "false";

  const savedReducedMotion = localStorage.getItem("gitswamp-reduced-motion");
  if (savedReducedMotion !== null) reducedMotion.value = savedReducedMotion === "true";

  const savedWrapDiffLines = localStorage.getItem("gitswamp-wrap-diff-lines");
  if (savedWrapDiffLines !== null) wrapDiffLines.value = savedWrapDiffLines === "true";

  const savedShowDiffLineNumbers = localStorage.getItem("gitswamp-show-diff-line-numbers");
  if (savedShowDiffLineNumbers !== null) showDiffLineNumbers.value = savedShowDiffLineNumbers !== "false";

  const savedNotifyGitkeep = localStorage.getItem("gitswamp-notify-gitkeep");
  if (savedNotifyGitkeep === null) {
    localStorage.setItem("gitswamp-notify-gitkeep", "true");
  } else {
    notifyGitkeep.value = savedNotifyGitkeep !== "false";
  }

  const savedDisableGraphAnimations = localStorage.getItem("gitswamp-disable-graph-animations");
  if (savedDisableGraphAnimations !== null) {
    disableGraphAnimations.value = savedDisableGraphAnimations === "true";
  }

  const savedSmoothGraphScroll = localStorage.getItem(OPTIONS_SMOOTH_GRAPH_SCROLL_KEY);
  if (savedSmoothGraphScroll !== null) {
    smoothGraphScroll.value = savedSmoothGraphScroll === "true";
  }

  const savedAutoFetchEnabled = localStorage.getItem(OPTIONS_AUTO_FETCH_ENABLED_KEY);
  if (savedAutoFetchEnabled !== null) {
    autoFetchEnabled.value = savedAutoFetchEnabled !== "false";
  }

  autoFetchIntervalMinutes.value = normalizeAutoFetchIntervalMinutes(localStorage.getItem(OPTIONS_AUTO_FETCH_INTERVAL_KEY));

  organisationProfiles.value = readOrganisationProfilesFromStorage();

  smartGitignoreWizardEnabled.value = getStoredSmartGitignoreWizardEnabled();
  commitAnalyzerEnabled.value = getStoredCommitAnalyzerSettings().enabled;
  applySettings();

  azureDomainInput.value = git.providerTokens.value["azure-domain"] || "";

  if (gitlabSelfState.value.domain) {
    gitlabSelfDomainInput.value = gitlabSelfState.value.domain;
  }

  void refreshGitPath(true);
  void refreshGithubUser();

  queueMicrotask(() => {
    isHydrating = false;
  });
});

watch([
  generalFontSizePx,
  fontSize,
  compactMode,
  dummyMode,
  showAvatars,
  restoreSession,
  reducedMotion,
  wrapDiffLines,
  showDiffLineNumbers,
  notifyGitkeep,
  disableGraphAnimations,
  smoothGraphScroll,
  autoFetchEnabled,
  autoFetchIntervalMinutes,
], () => {
  if (isHydrating) {
    return;
  }
  generalFontSizePx.value = clampGeneralFontSize(generalFontSizePx.value);
  autoFetchIntervalMinutes.value = normalizeAutoFetchIntervalMinutes(String(autoFetchIntervalMinutes.value));
  applySettings();
  localStorage.setItem(OPTIONS_COMMIT_FONT_SIZE_KEY, fontSize.value);
  localStorage.setItem(OPTIONS_GENERAL_FONT_SIZE_KEY, String(generalFontSizePx.value));
  localStorage.setItem("gitswamp-compact-mode", String(compactMode.value));
  localStorage.setItem("gitswamp-dummy-mode", String(dummyMode.value));
  localStorage.setItem("gitswamp-show-avatars", String(showAvatars.value));
  localStorage.setItem("gitswamp-restore-session", String(restoreSession.value));
  localStorage.setItem("gitswamp-reduced-motion", String(reducedMotion.value));
  localStorage.setItem("gitswamp-wrap-diff-lines", String(wrapDiffLines.value));
  localStorage.setItem("gitswamp-show-diff-line-numbers", String(showDiffLineNumbers.value));
  localStorage.setItem("gitswamp-notify-gitkeep", String(notifyGitkeep.value));
  localStorage.setItem("gitswamp-disable-graph-animations", String(disableGraphAnimations.value));
  localStorage.setItem(OPTIONS_SMOOTH_GRAPH_SCROLL_KEY, String(smoothGraphScroll.value));
  localStorage.setItem(OPTIONS_AUTO_FETCH_ENABLED_KEY, String(autoFetchEnabled.value));
  localStorage.setItem(OPTIONS_AUTO_FETCH_INTERVAL_KEY, String(autoFetchIntervalMinutes.value));
  emitAutoFetchSettingsChanged();
});

watch(themeMode, (value) => {
  if (isHydrating) {
    return;
  }
  applyThemeModePreference(value);
  storeThemeModePreference(value);
});

watch(appPalette, (value) => {
  if (isHydrating) {
    return;
  }
  applyAppPalettePreference(value);
  storeAppPalettePreference(value);
});

watch(commitAnalyzerEnabled, (value) => {
  if (isHydrating) {
    return;
  }
  updateCommitAnalyzerSettings({ enabled: value });
});

watch(smartGitignoreWizardEnabled, (value) => {
  if (isHydrating) {
    return;
  }
  storeSmartGitignoreWizardEnabled(value);
});

watch(
  () => [git.providerTokens.value["gitlab-self"], git.providerTokens.value["azure-domain"]],
  () => {
    if (gitlabSelfState.value.domain) {
      gitlabSelfDomainInput.value = gitlabSelfState.value.domain;
    }

    if (git.providerTokens.value["azure-domain"]) {
      azureDomainInput.value = git.providerTokens.value["azure-domain"] || "";
    }
  },
);

watch(
  () => currentGithubToken(),
  () => {
    void refreshGithubUser();
  },
);

watch(
  () => props.initialSection,
  (value) => {
    if (!value) {
      return;
    }
    activeSection.value = value;
  },
);

watch(activeSection, () => {
  resetOptionsScroll();
});

watch(activePlatform, () => {
  if (activeSection.value === "integrations") {
    resetOptionsScroll();
  }
});
</script>

<template>
  <div class="fixed inset-0 z-[7600] flex items-center justify-center bg-black/60 backdrop-blur-sm" @click.self="emit('close')">
    <div class="w-[980px] max-w-[96vw] max-h-[92vh] h-[92vh] bg-[var(--card)] border border-[var(--border)] rounded-xl shadow-2xl overflow-hidden flex flex-col">
      <div class="flex items-center justify-between px-5 py-4 border-b border-[var(--border)] flex-shrink-0">
        <div class="flex items-center gap-2">
          <Settings2 class="w-4 h-4 text-[var(--primary)]" />
          <h2 class="text-sm font-semibold text-[var(--foreground)]">Options</h2>
        </div>
        <button @click="emit('close')" class="p-1 rounded hover:bg-[var(--secondary)] transition-colors">
          <X class="w-4 h-4 text-[var(--muted-foreground)]" />
        </button>
      </div>

      <div class="flex flex-1 min-h-0">
        <div class="w-[170px] border-r border-[var(--border)] bg-[var(--secondary)]/35 p-2.5">
          <button
            class="w-full text-left px-2.5 py-2 rounded text-xs font-medium transition-colors mb-1"
            :class="activeSection === 'integrations'
              ? 'bg-[var(--primary)]/15 text-[var(--foreground)]'
              : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--secondary)]'"
            @click="activeSection = 'integrations'"
          >
            <div class="flex items-center gap-2">
              <PlugZap class="w-3.5 h-3.5" />
              Integrations
            </div>
          </button>

          <button
            class="w-full text-left px-2.5 py-2 rounded text-xs font-medium transition-colors"
            :class="activeSection === 'git'
              ? 'bg-[var(--primary)]/15 text-[var(--foreground)]'
              : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--secondary)]'"
            @click="activeSection = 'git'"
          >
            <div class="flex items-center gap-2">
              <GitBranch class="w-3.5 h-3.5" />
              Git Integration
            </div>
          </button>

          <button
            class="w-full text-left px-2.5 py-2 rounded text-xs font-medium transition-colors mt-1"
            :class="activeSection === 'preferences'
              ? 'bg-[var(--primary)]/15 text-[var(--foreground)]'
              : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--secondary)]'"
            @click="activeSection = 'preferences'"
          >
            <div class="flex items-center gap-2">
              <SlidersHorizontal class="w-3.5 h-3.5" />
              Preferences
            </div>
          </button>

          <button
            class="w-full text-left px-2.5 py-2 rounded text-xs font-medium transition-colors mt-1"
            :class="activeSection === 'advanced'
              ? 'bg-[var(--primary)]/15 text-[var(--foreground)]'
              : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--secondary)]'"
            @click="activeSection = 'advanced'"
          >
            <div class="flex items-center gap-2">
              <FlaskConical class="w-3.5 h-3.5" />
              Advanced
            </div>
          </button>

          <button
            class="w-full text-left px-2.5 py-2 rounded text-xs font-medium transition-colors mt-1"
            :class="activeSection === 'organisations'
              ? 'bg-[var(--primary)]/15 text-[var(--foreground)]'
              : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--secondary)]'"
            @click="activeSection = 'organisations'"
          >
            <div class="flex items-center gap-2">
              <Building2 class="w-3.5 h-3.5" />
              Organisations
            </div>
          </button>
        </div>

        <div class="flex-1 overflow-y-auto p-5" ref="optionsScrollRef">
          <template v-if="activeSection === 'integrations'">
            <div class="flex items-center gap-2 mb-4">
              <button
                class="px-3 py-1.5 rounded text-xs font-medium border transition-colors"
                :class="activePlatform === 'github'
                  ? 'bg-[var(--primary)]/15 border-[var(--primary)]/30 text-[var(--foreground)]'
                  : 'border-[var(--border)] text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--secondary)]'"
                @click="activePlatform = 'github'"
              >
                GitHub
              </button>
              <button
                class="px-3 py-1.5 rounded text-xs font-medium border transition-colors"
                :class="activePlatform === 'gitlab'
                  ? 'bg-[var(--primary)]/15 border-[var(--primary)]/30 text-[var(--foreground)]'
                  : 'border-[var(--border)] text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--secondary)]'"
                @click="activePlatform = 'gitlab'"
              >
                GitLab
              </button>
              <button
                class="px-3 py-1.5 rounded text-xs font-medium border transition-colors inline-flex items-center gap-1.5"
                :class="activePlatform === 'bitbucket'
                  ? 'bg-[var(--primary)]/15 border-[var(--primary)]/30 text-[var(--foreground)]'
                  : 'border-[var(--border)] text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--secondary)]'"
                @click="activePlatform = 'bitbucket'"
              >
                <BitbucketIcon class="w-3.5 h-3.5" />
                Bitbucket
              </button>
              <button
                class="px-3 py-1.5 rounded text-xs font-medium border transition-colors inline-flex items-center gap-1.5"
                :class="activePlatform === 'azure'
                  ? 'bg-[var(--primary)]/15 border-[var(--primary)]/30 text-[var(--foreground)]'
                  : 'border-[var(--border)] text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--secondary)]'"
                @click="activePlatform = 'azure'"
              >
                <AzureDevOpsIcon class="w-3.5 h-3.5" />
                Azure DevOps
              </button>
            </div>

            <div v-if="activePlatform === 'github'" class="space-y-4">
              <div class="border border-[var(--border)] rounded-lg p-3.5 bg-[var(--popover)]/50 space-y-3">
                <div class="flex items-center justify-between gap-3">
                  <div>
                    <div class="text-xs font-semibold text-[var(--foreground)]">Account Connection</div>
                    <div class="text-[10px] text-[var(--muted-foreground)] mt-0.5">
                      OAuth with GitHub CLI browser flow, or token fallback.
                    </div>
                  </div>
                  <div class="flex items-center gap-2">
                    <AppButton
                      class="text-xs h-8 bg-[var(--primary)] text-white hover:opacity-90"
                      :disabled="githubOauthBusy"
                      @click="connectGithubOAuth"
                    >
                      <Loader2 v-if="githubOauthBusy" class="w-3.5 h-3.5 mr-1 animate-spin" />
                      <ExternalLink v-else class="w-3.5 h-3.5 mr-1" />
                      Connect via OAuth
                    </AppButton>
                    <button
                      class="h-8 px-2.5 rounded border border-[var(--border)] text-[10px] text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--secondary)] transition-colors"
                      @click="openUrl('https://cli.github.com/').catch(() => {})"
                    >
                      Install GitHub CLI
                    </button>
                  </div>
                </div>

                <div class="text-[11px]" :class="hasGithubToken ? 'text-[#10b981]' : 'text-[#f59e0b]'">
                  {{ hasGithubToken ? `Connected${githubUser ? ` as ${githubUser}` : ''}` : 'Not connected' }}
                </div>

                <div class="flex items-center gap-2">
                  <input
                    v-model="githubTokenInput"
                    type="password"
                    placeholder="Paste GitHub token (optional fallback)"
                    class="flex-1 px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                  />
                  <AppButton
                    class="h-8 text-xs bg-[var(--primary)] text-white hover:opacity-90"
                    :disabled="githubTokenBusy || !githubTokenInput.trim()"
                    @click="saveGithubToken"
                  >
                    <Loader2 v-if="githubTokenBusy" class="w-3.5 h-3.5 animate-spin" />
                    <template v-else>{{ hasGithubToken ? 'Replace Token' : 'Save Token' }}</template>
                  </AppButton>
                  <AppButton
                    class="h-8 text-xs bg-[#ef4444]/20 text-[#ef4444] hover:bg-[#ef4444]/30"
                    :disabled="githubTokenBusy || !hasGithubToken"
                    @click="deleteGithubToken"
                  >
                    <Trash2 class="w-3.5 h-3.5" />
                  </AppButton>
                </div>
                <div class="text-[10px] text-[var(--muted-foreground)]">
                  Token is stored encrypted and never shown again in the UI.
                </div>
              </div>

              <div class="border border-[var(--border)] rounded-lg p-3.5 bg-[var(--popover)]/50 space-y-3">
                <div>
                  <div class="text-xs font-semibold text-[var(--foreground)]">SSH Keys</div>
                  <div class="text-[10px] text-[var(--muted-foreground)] mt-0.5">
                    Generate a new SSH key and add it to GitHub, or import an existing key.
                  </div>
                </div>

                <div class="space-y-2 max-w-[760px]">
                  <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">Key Generation</div>
                  <input
                    v-model="githubSshEmailInput"
                    placeholder="Email for generated SSH key"
                    class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                  />
                  <input
                    v-model="githubSshKeyNameInput"
                    placeholder="Key file name (default gitswamp_github)"
                    class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                  />

                  <div class="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)] pt-1">Titles on GitHub</div>
                  <input
                    v-model="githubGeneratedKeyTitleInput"
                    placeholder="Title for generated key"
                    class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                  />
                  <input
                    v-model="githubExistingKeyTitleInput"
                    placeholder="Title for imported existing key"
                    class="w-full px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                  />
                </div>

                <div class="flex items-center gap-2 flex-wrap">
                  <AppButton
                    class="h-8 text-xs bg-[var(--primary)] text-white hover:opacity-90"
                    :disabled="githubSshBusy"
                    @click="generateAndAddGithubSshKey"
                  >
                    <Loader2 v-if="githubSshBusy" class="w-3.5 h-3.5 mr-1 animate-spin" />
                    <KeyRound v-else class="w-3.5 h-3.5 mr-1" />
                    Generate and Add
                  </AppButton>
                  <AppButton
                    class="h-8 text-xs bg-[var(--secondary)] text-[var(--foreground)] hover:opacity-90"
                    :disabled="githubSshBusy"
                    @click="addExistingGithubSshKey"
                  >
                    <KeyRound class="w-3.5 h-3.5 mr-1" />
                    Add Existing SSH Key
                  </AppButton>
                </div>

                <div class="text-[10px] text-[var(--muted-foreground)]">
                  Tip: if key creation returns 404, reconnect OAuth with scope <span class="font-mono">admin:public_key</span> or replace token with the same scope.
                </div>

                <div class="border border-[var(--border)] rounded p-2.5 bg-[var(--card)]/50">
                  <div class="flex items-center justify-between mb-2">
                    <div class="text-[11px] font-medium text-[var(--foreground)]">Registered GitHub SSH Keys</div>
                    <button
                      class="text-[10px] text-[var(--muted-foreground)] hover:text-[var(--foreground)] inline-flex items-center gap-1"
                      @click="refreshGithubKeys"
                    >
                      <RefreshCw class="w-3 h-3" />
                      Refresh
                    </button>
                  </div>

                  <div v-if="githubKeysBusy" class="text-[11px] text-[var(--muted-foreground)] py-2">
                    Loading keys...
                  </div>
                  <div v-else-if="!githubSshKeys.length" class="text-[11px] text-[var(--muted-foreground)] py-2">
                    No SSH keys found for this account.
                  </div>
                  <div v-else class="space-y-2 max-h-[200px] overflow-y-auto pr-1">
                    <div v-for="item in githubSshKeys" :key="item.id" class="border border-[var(--border)] rounded p-2">
                      <div class="flex items-start justify-between gap-2">
                        <div class="min-w-0">
                          <div class="text-[11px] font-medium text-[var(--foreground)] truncate">{{ item.title }}</div>
                          <div class="text-[10px] text-[var(--muted-foreground)] mt-0.5 truncate">{{ formatGithubKeyMeta(item) }}</div>
                        </div>
                        <button
                          class="text-[#ef4444] hover:text-[#f87171] p-1 rounded hover:bg-[#ef4444]/10"
                          @click="deleteGithubSshKey(item.id)"
                          title="Delete key"
                        >
                          <Trash2 class="w-3.5 h-3.5" />
                        </button>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <div v-else-if="activePlatform === 'gitlab'" class="space-y-4">
              <div class="border border-[var(--border)] rounded-lg p-3.5 bg-[var(--popover)]/50 space-y-3">
                <div class="text-xs font-semibold text-[var(--foreground)]">GitLab.com Token</div>
                <div class="text-[11px]" :class="hasGitlabToken ? 'text-[#10b981]' : 'text-[#f59e0b]'">
                  {{ hasGitlabToken ? 'Configured' : 'Not configured' }}
                </div>
                <div class="flex items-center gap-2">
                  <input
                    v-model="gitlabTokenInput"
                    type="password"
                    placeholder="GitLab personal access token"
                    class="flex-1 px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                  />
                  <AppButton class="h-8 text-xs bg-[var(--primary)] text-white" :disabled="gitlabTokenBusy || !gitlabTokenInput.trim()" @click="saveGitlabToken">
                    <Loader2 v-if="gitlabTokenBusy" class="w-3.5 h-3.5 animate-spin" />
                    <template v-else>{{ hasGitlabToken ? 'Replace' : 'Save' }}</template>
                  </AppButton>
                  <AppButton class="h-8 text-xs bg-[#ef4444]/20 text-[#ef4444]" :disabled="gitlabTokenBusy || !hasGitlabToken" @click="deleteGitlabToken">
                    <Trash2 class="w-3.5 h-3.5" />
                  </AppButton>
                </div>
              </div>

              <div class="border border-[var(--border)] rounded-lg p-3.5 bg-[var(--popover)]/50 space-y-3">
                <div class="text-xs font-semibold text-[var(--foreground)]">GitLab Self-Managed</div>
                <div class="text-[11px]" :class="hasGitlabSelfToken ? 'text-[#10b981]' : 'text-[#f59e0b]'">
                  {{ hasGitlabSelfToken ? `Configured (${gitlabSelfState.domain})` : 'Not configured' }}
                </div>

                <div class="grid grid-cols-2 gap-2">
                  <input
                    v-model="gitlabSelfDomainInput"
                    placeholder="gitlab.company.com"
                    class="px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                  />
                  <input
                    v-model="gitlabSelfTokenInput"
                    type="password"
                    placeholder="Self-managed token"
                    class="px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                  />
                </div>

                <div class="flex items-center gap-2">
                  <AppButton class="h-8 text-xs bg-[var(--primary)] text-white" :disabled="gitlabSelfTokenBusy || !gitlabSelfTokenInput.trim() || !gitlabSelfDomainInput.trim()" @click="saveGitlabSelfToken">
                    <Loader2 v-if="gitlabSelfTokenBusy" class="w-3.5 h-3.5 animate-spin" />
                    <template v-else>{{ hasGitlabSelfToken ? 'Replace' : 'Save' }}</template>
                  </AppButton>
                  <AppButton class="h-8 text-xs bg-[#ef4444]/20 text-[#ef4444]" :disabled="gitlabSelfTokenBusy || !hasGitlabSelfToken" @click="deleteGitlabSelfToken">
                    <Trash2 class="w-3.5 h-3.5" />
                  </AppButton>
                </div>

                <div class="border border-[var(--border)] rounded p-2.5 bg-[var(--card)]/50 space-y-2">
                  <div class="text-[11px] font-medium text-[var(--foreground)]">Generate and Push SSH Key (self-managed)</div>
                  <div class="grid grid-cols-2 gap-2">
                    <input
                      v-model="gitlabSelfEmailInput"
                      placeholder="Email for SSH key"
                      class="px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                    />
                    <input
                      v-model="gitlabSelfKeyNameInput"
                      placeholder="Key file name"
                      class="px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                    />
                  </div>
                  <AppButton
                    class="h-8 text-xs bg-[#f59e0b] text-white hover:opacity-90"
                    :disabled="gitlabSelfSshBusy || !gitlabSelfEmailInput.trim() || !gitlabSelfTokenInput.trim() || !gitlabSelfDomainInput.trim()"
                    @click="generateAndPushGitlabSelfKey"
                  >
                    <Loader2 v-if="gitlabSelfSshBusy" class="w-3.5 h-3.5 mr-1 animate-spin" />
                    <KeyRound v-else class="w-3.5 h-3.5 mr-1" />
                    Generate and Push SSH Key
                  </AppButton>
                </div>
              </div>
            </div>

            <div v-else-if="activePlatform === 'bitbucket'" class="space-y-4">
              <div class="border border-[var(--border)] rounded-lg p-3.5 bg-[var(--popover)]/50 space-y-3">
                <div class="text-xs font-semibold text-[var(--foreground)]">Bitbucket Token</div>
                <div class="text-[11px]" :class="hasBitbucketToken ? 'text-[#10b981]' : 'text-[#f59e0b]'">
                  {{ hasBitbucketToken ? 'Configured' : 'Not configured' }}
                </div>
                <div class="flex items-center gap-2">
                  <input
                    v-model="bitbucketTokenInput"
                    type="password"
                    placeholder="Bitbucket app password/token"
                    class="flex-1 px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                  />
                  <AppButton class="h-8 text-xs bg-[var(--primary)] text-white" :disabled="bitbucketTokenBusy || !bitbucketTokenInput.trim()" @click="saveBitbucketToken">
                    <Loader2 v-if="bitbucketTokenBusy" class="w-3.5 h-3.5 animate-spin" />
                    <template v-else>{{ hasBitbucketToken ? 'Replace' : 'Save' }}</template>
                  </AppButton>
                  <AppButton class="h-8 text-xs bg-[#ef4444]/20 text-[#ef4444]" :disabled="bitbucketTokenBusy || !hasBitbucketToken" @click="deleteBitbucketToken">
                    <Trash2 class="w-3.5 h-3.5" />
                  </AppButton>
                </div>
              </div>
            </div>

            <div v-else class="space-y-4">
              <div class="border border-[var(--border)] rounded-lg p-3.5 bg-[var(--popover)]/50 space-y-3">
                <div class="text-xs font-semibold text-[var(--foreground)]">Azure DevOps Token</div>
                <div class="text-[11px]" :class="hasAzureToken ? 'text-[#10b981]' : 'text-[#f59e0b]'">
                  {{ hasAzureToken ? 'Configured' : 'Not configured' }}
                </div>

                <div class="grid grid-cols-2 gap-2">
                  <input
                    v-model="azureDomainInput"
                    placeholder="dev.azure.com/myorg"
                    class="px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                  />
                  <input
                    v-model="azureTokenInput"
                    type="password"
                    placeholder="Azure PAT"
                    class="px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                  />
                </div>

                <div class="flex items-center gap-2">
                  <AppButton class="h-8 text-xs bg-[var(--primary)] text-white" :disabled="azureTokenBusy || !azureTokenInput.trim() || !azureDomainInput.trim()" @click="saveAzureToken">
                    <Loader2 v-if="azureTokenBusy" class="w-3.5 h-3.5 animate-spin" />
                    <template v-else>{{ hasAzureToken ? 'Replace' : 'Save' }}</template>
                  </AppButton>
                  <AppButton class="h-8 text-xs bg-[#ef4444]/20 text-[#ef4444]" :disabled="azureTokenBusy || !hasAzureToken" @click="deleteAzureToken">
                    <Trash2 class="w-3.5 h-3.5" />
                  </AppButton>
                </div>
              </div>
            </div>
          </template>

          <template v-else-if="activeSection === 'git'">
            <div class="space-y-4">
              <div class="border border-[var(--border)] rounded-lg p-3.5 bg-[var(--popover)]/50 space-y-3">
                <div class="flex items-start justify-between gap-3">
                  <div>
                    <div class="text-xs font-semibold text-[var(--foreground)]">Git Runtime</div>
                    <div class="text-[10px] text-[var(--muted-foreground)] mt-0.5">
                      Detect Git on your machine. If missing, download or start installation.
                    </div>
                  </div>
                  <div class="inline-flex items-center gap-1.5 text-[11px]" :class="hasDetectedGit ? 'text-[#10b981]' : 'text-[#f59e0b]'">
                    <Check v-if="hasDetectedGit" class="w-3.5 h-3.5" />
                    <X v-else class="w-3.5 h-3.5" />
                    {{ hasDetectedGit ? 'Detected' : 'Not detected' }}
                  </div>
                </div>

                <div class="px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs font-mono text-[var(--foreground)] break-all">
                  {{ currentGitPath || 'Git executable was not detected yet.' }}
                </div>

                <div class="flex flex-wrap items-center gap-2">
                  <AppButton
                    class="h-8 text-xs bg-[var(--secondary)] text-[var(--foreground)]"
                    :disabled="gitPathBusy"
                    @click="refreshGitPath()"
                  >
                    <Loader2 v-if="gitPathBusy" class="w-3.5 h-3.5 mr-1 animate-spin" />
                    <RefreshCw v-else class="w-3.5 h-3.5 mr-1" />
                    Refresh detection
                  </AppButton>
                  <AppButton
                    class="h-8 text-xs bg-[var(--primary)] text-white"
                    @click="openGitDownload"
                  >
                    <ExternalLink class="w-3.5 h-3.5 mr-1" />
                    Download Git
                  </AppButton>
                  <AppButton
                    class="h-8 text-xs bg-[#f59e0b] text-white"
                    :disabled="installGitBusy"
                    @click="installGit"
                  >
                    <Loader2 v-if="installGitBusy" class="w-3.5 h-3.5 mr-1 animate-spin" />
                    <GitBranch v-else class="w-3.5 h-3.5 mr-1" />
                    Install Git
                  </AppButton>
                </div>
              </div>

              <div class="border border-[var(--border)] rounded-lg p-3.5 bg-[var(--popover)]/50 space-y-3">
                <div class="text-xs font-semibold text-[var(--foreground)]">Active Repository Auto-Fetch</div>
                <div class="text-[10px] text-[var(--muted-foreground)]">
                  Lightweight background service that fetches only the active repository. Default: every 3 minutes.
                </div>

                <div class="flex items-center justify-between py-1">
                  <div>
                    <div class="text-xs font-medium text-[var(--foreground)]">Enable background auto-fetch</div>
                    <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Runs one timer, skips when repository is missing or when a fetch is already running.</p>
                  </div>
                  <button @click="autoFetchEnabled = !autoFetchEnabled" class="relative w-10 h-5 rounded-full transition-colors" :class="autoFetchEnabled ? 'bg-[var(--primary)]' : 'bg-[var(--muted)]'">
                    <div class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all" :class="autoFetchEnabled ? 'left-[calc(100%-1.125rem)]' : 'left-0.5'" />
                  </button>
                </div>

                <div class="space-y-1.5">
                  <div class="flex items-center justify-between text-[11px] text-[var(--foreground)]">
                    <span>Interval (minutes)</span>
                    <span class="font-mono">{{ autoFetchIntervalMinutes }}</span>
                  </div>
                  <input
                    type="range"
                    min="1"
                    max="60"
                    step="1"
                    :value="autoFetchIntervalMinutes"
                    @input="autoFetchIntervalMinutes = normalizeAutoFetchIntervalMinutes(($event.target as HTMLInputElement).value)"
                    class="w-full"
                  />
                  <div class="text-[10px] text-[var(--muted-foreground)]">1-60 minutes. Lower values increase network activity.</div>
                </div>
              </div>
            </div>
          </template>

          <template v-else-if="activeSection === 'preferences'">
            <div class="space-y-4">
              <div class="space-y-3">
                <div class="flex items-center gap-2 text-[10px] font-semibold text-[var(--muted-foreground)] uppercase tracking-wider">
                  <Monitor class="w-3 h-3" />
                  Appearance
                </div>

                <div class="flex items-center justify-between py-2">
                  <div>
                    <div class="text-xs font-medium text-[var(--foreground)]">Theme</div>
                    <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Dark or light mode</p>
                  </div>
                  <button
                    @click="toggleThemeMode"
                    class="relative w-14 h-7 rounded-full transition-colors duration-300"
                    :class="themeMode === 'dark' ? 'bg-[var(--accent)]' : 'bg-[var(--primary)]'"
                  >
                    <div
                      class="absolute top-0.5 w-6 h-6 rounded-full bg-white shadow-md flex items-center justify-center transition-all duration-300"
                      :class="themeMode === 'dark' ? 'left-0.5' : 'left-[calc(100%-1.625rem)]'"
                    >
                      <Moon v-if="themeMode === 'dark'" class="w-3.5 h-3.5 text-[var(--accent)]" />
                      <Sun v-else class="w-3.5 h-3.5 text-[var(--primary)]" />
                    </div>
                  </button>
                </div>

                <div class="flex items-center justify-between py-2">
                  <div>
                    <div class="text-xs font-medium text-[var(--foreground)]">Theme Preset</div>
                    <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Grouped dark/light themes</p>
                  </div>
                  <select
                    :value="appPalette"
                    @change="onThemePresetChange"
                    class="w-[240px] px-2.5 py-1.5 text-[11px] rounded bg-[var(--input-background)] border border-[var(--border)] text-[var(--foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]"
                  >
                    <optgroup label="Dark Themes">
                      <option v-for="theme in darkThemeOptions" :key="theme.id" :value="theme.id">
                        {{ themeLabel(theme) }}
                      </option>
                    </optgroup>
                    <optgroup label="Light Themes">
                      <option v-for="theme in lightThemeOptions" :key="theme.id" :value="theme.id">
                        {{ themeLabel(theme) }}
                      </option>
                    </optgroup>
                  </select>
                </div>

                <div class="flex items-center justify-between py-2">
                  <div>
                    <div class="text-xs font-medium text-[var(--foreground)]">General Font Size</div>
                    <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Global UI text scale for panels, labels and dialogs.</p>
                  </div>
                  <div class="w-[290px] max-w-full">
                    <div class="flex items-center justify-between text-[10px] text-[var(--muted-foreground)] mb-1">
                      <span>10px</span>
                      <span class="font-mono text-[var(--foreground)]">{{ generalFontLabel }}</span>
                      <span>40px</span>
                    </div>
                    <input
                      type="range"
                      min="10"
                      max="40"
                      step="1"
                      :value="generalFontSizePx"
                      @input="setGeneralFontFromSlider"
                      class="w-full"
                    />
                  </div>
                </div>

                <div class="flex items-center justify-between py-2">
                  <div>
                    <div class="text-xs font-medium text-[var(--foreground)]">Commit Text Size</div>
                    <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Changes text density in commit list and history views only.</p>
                  </div>
                  <div class="w-[290px] max-w-full">
                    <div class="flex items-center justify-between text-[10px] text-[var(--muted-foreground)] mb-1">
                      <span>Tiny</span>
                      <span class="font-mono text-[var(--foreground)]">{{ commitFontLabel }}</span>
                      <span>XXXX-Large</span>
                    </div>
                    <input
                      type="range"
                      min="0"
                      max="8"
                      step="1"
                      :value="commitFontSliderValue"
                      @input="setCommitFontFromSlider"
                      class="w-full"
                    />
                  </div>
                </div>

                <div class="flex items-center justify-between py-2">
                  <div>
                    <div class="text-xs font-medium text-[var(--foreground)]">Compact Mode</div>
                    <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Reduce spacing in commit list</p>
                  </div>
                  <button @click="compactMode = !compactMode" class="relative w-10 h-5 rounded-full transition-colors" :class="compactMode ? 'bg-[var(--primary)]' : 'bg-[var(--muted)]'">
                    <div class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all" :class="compactMode ? 'left-[calc(100%-1.125rem)]' : 'left-0.5'" />
                  </button>
                </div>

                <div class="flex items-center justify-between py-2">
                  <div>
                    <div class="text-xs font-medium text-[var(--foreground)]">Dummy Mode</div>
                    <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Show beginner descriptions in menus</p>
                  </div>
                  <button @click="dummyMode = !dummyMode" class="relative w-10 h-5 rounded-full transition-colors" :class="dummyMode ? 'bg-[var(--primary)]' : 'bg-[var(--muted)]'">
                    <div class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all" :class="dummyMode ? 'left-[calc(100%-1.125rem)]' : 'left-0.5'" />
                  </button>
                </div>

                <div class="flex items-center justify-between py-2">
                  <div>
                    <div class="text-xs font-medium text-[var(--foreground)]">Show Avatars</div>
                    <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Show user avatars in commit list</p>
                  </div>
                  <button @click="showAvatars = !showAvatars" class="relative w-10 h-5 rounded-full transition-colors" :class="showAvatars ? 'bg-[var(--primary)]' : 'bg-[var(--muted)]'">
                    <div class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all" :class="showAvatars ? 'left-[calc(100%-1.125rem)]' : 'left-0.5'" />
                  </button>
                </div>

                <div class="flex items-center justify-between py-2">
                  <div>
                    <div class="text-xs font-medium text-[var(--foreground)]">Restore Session on Start</div>
                    <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Reopen last active tabs and repository</p>
                  </div>
                  <button @click="restoreSession = !restoreSession" class="relative w-10 h-5 rounded-full transition-colors" :class="restoreSession ? 'bg-[var(--primary)]' : 'bg-[var(--muted)]'">
                    <div class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all" :class="restoreSession ? 'left-[calc(100%-1.125rem)]' : 'left-0.5'" />
                  </button>
                </div>

                <div class="flex items-center justify-between py-2">
                  <div>
                    <div class="text-xs font-medium text-[var(--foreground)]">Reduced Motion</div>
                    <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Disable heavy animations</p>
                  </div>
                  <button @click="reducedMotion = !reducedMotion" class="relative w-10 h-5 rounded-full transition-colors" :class="reducedMotion ? 'bg-[var(--primary)]' : 'bg-[var(--muted)]'">
                    <div class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all" :class="reducedMotion ? 'left-[calc(100%-1.125rem)]' : 'left-0.5'" />
                  </button>
                </div>

                <div class="flex items-center justify-between py-2">
                  <div>
                    <div class="text-xs font-medium text-[var(--foreground)]">Wrap Long Diff Lines</div>
                    <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Wrap long lines instead of horizontal scroll</p>
                  </div>
                  <button @click="wrapDiffLines = !wrapDiffLines" class="relative w-10 h-5 rounded-full transition-colors" :class="wrapDiffLines ? 'bg-[var(--primary)]' : 'bg-[var(--muted)]'">
                    <div class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all" :class="wrapDiffLines ? 'left-[calc(100%-1.125rem)]' : 'left-0.5'" />
                  </button>
                </div>

                <div class="flex items-center justify-between py-2">
                  <div>
                    <div class="text-xs font-medium text-[var(--foreground)]">Show Diff Line Numbers</div>
                    <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Display old/new line numbers in diff view</p>
                  </div>
                  <button @click="showDiffLineNumbers = !showDiffLineNumbers" class="relative w-10 h-5 rounded-full transition-colors" :class="showDiffLineNumbers ? 'bg-[var(--primary)]' : 'bg-[var(--muted)]'">
                    <div class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all" :class="showDiffLineNumbers ? 'left-[calc(100%-1.125rem)]' : 'left-0.5'" />
                  </button>
                </div>

                <div class="flex items-center justify-between py-2">
                  <div>
                    <div class="text-xs font-medium text-[var(--foreground)]">Notify for .gitkeep Need</div>
                    <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Warn when empty folders are detected</p>
                  </div>
                  <button @click="notifyGitkeep = !notifyGitkeep" class="relative w-10 h-5 rounded-full transition-colors" :class="notifyGitkeep ? 'bg-[var(--primary)]' : 'bg-[var(--muted)]'">
                    <div class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all" :class="notifyGitkeep ? 'left-[calc(100%-1.125rem)]' : 'left-0.5'" />
                  </button>
                </div>

                <div class="flex items-center justify-between py-2">
                  <div>
                    <div class="text-xs font-medium text-[var(--foreground)]">Smart .gitignore Wizard</div>
                    <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Suggest ignore rules from untracked files only when enabled</p>
                  </div>
                  <button @click="smartGitignoreWizardEnabled = !smartGitignoreWizardEnabled" class="relative w-10 h-5 rounded-full transition-colors" :class="smartGitignoreWizardEnabled ? 'bg-[var(--primary)]' : 'bg-[var(--muted)]'">
                    <div class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all" :class="smartGitignoreWizardEnabled ? 'left-[calc(100%-1.125rem)]' : 'left-0.5'" />
                  </button>
                </div>

                <div class="flex items-center justify-between py-2">
                  <div>
                    <div class="text-xs font-medium text-[var(--foreground)]">Commit Analyzer</div>
                    <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Realtime commit quality analysis</p>
                  </div>
                  <button @click="commitAnalyzerEnabled = !commitAnalyzerEnabled" class="relative w-10 h-5 rounded-full transition-colors" :class="commitAnalyzerEnabled ? 'bg-[var(--primary)]' : 'bg-[var(--muted)]'">
                    <div class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all" :class="commitAnalyzerEnabled ? 'left-[calc(100%-1.125rem)]' : 'left-0.5'" />
                  </button>
                </div>
              </div>

              <div class="border-t border-[var(--border)] pt-4">
                <div class="flex items-center gap-2 text-[10px] font-semibold text-[var(--muted-foreground)] uppercase tracking-wider mb-3">
                  <Layout class="w-3 h-3" />
                  System
                </div>

                <div>
                  <div class="text-xs font-medium text-[var(--foreground)] mb-1.5">Git Executable</div>
                  <div class="flex items-center gap-2 px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs font-mono text-[var(--primary)]">
                    {{ props.gitPath || git.gitPath.value || 'Detecting...' }}
                  </div>
                </div>
              </div>
            </div>
          </template>

          <template v-else-if="activeSection === 'advanced'">
            <div class="space-y-4">
              <div class="space-y-3">
                <div class="flex items-center gap-2 text-[10px] font-semibold text-[var(--muted-foreground)] uppercase tracking-wider">
                  <FlaskConical class="w-3 h-3" />
                  Graph Behavior
                </div>

                <div class="flex items-center justify-between py-2">
                  <div>
                    <div class="text-xs font-medium text-[var(--foreground)]">Disable Commit Graph Animations</div>
                    <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Disables animation only in the graph rows and graph lines.</p>
                  </div>
                  <button @click="disableGraphAnimations = !disableGraphAnimations" class="relative w-10 h-5 rounded-full transition-colors" :class="disableGraphAnimations ? 'bg-[var(--primary)]' : 'bg-[var(--muted)]'">
                    <div class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all" :class="disableGraphAnimations ? 'left-[calc(100%-1.125rem)]' : 'left-0.5'" />
                  </button>
                </div>

                <div class="flex items-center justify-between py-2">
                  <div>
                    <div class="text-xs font-medium text-[var(--foreground)]">Smooth Scroll in Graph</div>
                    <p class="text-[10px] text-[var(--muted-foreground)] mt-0.5">Used when jumping to commit/search/home/end. Default is off.</p>
                  </div>
                  <button @click="smoothGraphScroll = !smoothGraphScroll" class="relative w-10 h-5 rounded-full transition-colors" :class="smoothGraphScroll ? 'bg-[var(--primary)]' : 'bg-[var(--muted)]'">
                    <div class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all" :class="smoothGraphScroll ? 'left-[calc(100%-1.125rem)]' : 'left-0.5'" />
                  </button>
                </div>
              </div>

              <div class="text-[10px] leading-relaxed text-[var(--muted-foreground)] border border-[var(--border)] rounded-lg p-3 bg-[var(--popover)]/40">
                Advanced controls are limited to graph animation and scroll behavior to keep runtime overhead low.
              </div>
            </div>
          </template>

          <template v-else-if="activeSection === 'organisations'">
            <div class="space-y-4">
              <div class="border border-[var(--border)] rounded-lg p-3.5 bg-[var(--popover)]/50 space-y-3">
                <div class="flex items-center gap-2 text-[10px] font-semibold text-[var(--muted-foreground)] uppercase tracking-wider">
                  <Users class="w-3.5 h-3.5" />
                  Organisation Repositories
                </div>

                <div class="flex items-center gap-2 flex-wrap">
                  <button
                    v-for="provider in ['github', 'gitlab', 'bitbucket', 'azure', 'all'] as const"
                    :key="provider"
                    class="px-3 py-1.5 rounded text-xs font-medium border transition-colors capitalize"
                    :class="organisationProvider === provider
                      ? 'bg-[var(--primary)]/15 border-[var(--primary)]/30 text-[var(--foreground)]'
                      : 'border-[var(--border)] text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--secondary)]'"
                    @click="organisationProvider = provider"
                  >
                    {{ provider === 'all' ? 'all providers' : provider }}
                  </button>
                </div>

                <div class="flex items-center gap-2">
                  <input
                    v-model="organisationQuery"
                    type="text"
                    placeholder="Search organisations, teams, projects or repositories"
                    class="flex-1 px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                  />
                  <AppButton
                    class="h-8 text-xs bg-[var(--primary)] text-white"
                    :disabled="organisationLoading"
                    @click="searchOrganisationRepos"
                  >
                    <Loader2 v-if="organisationLoading" class="w-3.5 h-3.5 animate-spin" />
                    <template v-else>Search</template>
                  </AppButton>
                </div>

                <div class="grid grid-cols-4 gap-2">
                  <select
                    v-model="organisationCloneProtocol"
                    class="px-2.5 py-2 text-[11px] rounded bg-[var(--input-background)] border border-[var(--border)] text-[var(--foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]"
                  >
                    <option value="https">Clone via HTTPS</option>
                    <option value="ssh">Clone via SSH</option>
                  </select>
                  <select
                    v-model="organisationVisibilityFilter"
                    class="px-2.5 py-2 text-[11px] rounded bg-[var(--input-background)] border border-[var(--border)] text-[var(--foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]"
                  >
                    <option value="all">Visibility: all</option>
                    <option value="private">Visibility: private</option>
                    <option value="public">Visibility: public</option>
                  </select>
                  <select
                    v-model="organisationSortMode"
                    class="px-2.5 py-2 text-[11px] rounded bg-[var(--input-background)] border border-[var(--border)] text-[var(--foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]"
                  >
                    <option value="stars-desc">Sort: stars</option>
                    <option value="name-asc">Sort: name</option>
                  </select>
                  <input
                    v-model="organisationLocalFilter"
                    type="text"
                    placeholder="Filter loaded repos"
                    class="px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                  />
                </div>

                <div class="flex items-center gap-2">
                  <input
                    v-model="organisationDestination"
                    type="text"
                    placeholder="Destination folder for batch clone"
                    class="flex-1 px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                  />
                  <AppButton class="h-8 text-xs bg-[var(--secondary)] text-[var(--foreground)]" @click="browseOrganisationDestination">
                    Browse
                  </AppButton>
                </div>

                <div v-if="organisationError" class="text-[11px] text-[#ef4444]">{{ organisationError }}</div>

                <div class="flex items-center justify-between">
                  <button
                    class="text-[11px] text-[var(--muted-foreground)] hover:text-[var(--foreground)]"
                    :disabled="!filteredOrganisationRepos.length"
                    @click="toggleSelectAllOrganisationRepos"
                  >
                    {{ allVisibleOrganisationReposSelected ? 'Clear visible selection' : 'Select visible' }}
                  </button>
                  <div class="text-[11px] text-[var(--muted-foreground)]">
                    {{ selectedOrganisationCount }} selected / {{ filteredOrganisationRepos.length }} visible ({{ organisationRepos.length }} loaded)
                  </div>
                </div>

                <div class="border border-[var(--border)] rounded p-2.5 max-h-[280px] overflow-y-auto bg-[var(--card)]/50">
                  <div v-if="!filteredOrganisationRepos.length && !organisationLoading" class="text-[11px] text-[var(--muted-foreground)] py-2">
                    No repositories match the current query/filters.
                  </div>
                  <div v-else class="space-y-1.5">
                    <label
                      v-for="repo in filteredOrganisationRepos"
                      :key="repo.id"
                      class="flex items-start gap-2 p-2 rounded border border-[var(--border)] hover:bg-[var(--secondary)]/40"
                    >
                      <input
                        v-model="organisationSelectedRepoIds"
                        type="checkbox"
                        :value="repo.id"
                        class="mt-0.5"
                      />
                      <div class="min-w-0 flex-1">
                        <div class="text-[11px] font-medium text-[var(--foreground)] truncate">{{ repo.fullName }}</div>
                        <div class="text-[10px] text-[var(--muted-foreground)] mt-0.5 inline-flex items-center gap-1.5">
                          <span class="px-1.5 py-0.5 rounded bg-[var(--secondary)] text-[9px] uppercase tracking-wide">{{ repo.provider }}</span>
                          <span class="px-1.5 py-0.5 rounded" :class="repo.isPrivate ? 'bg-[#ef4444]/15 text-[#ef4444]' : 'bg-[#10b981]/15 text-[#10b981]'">
                            {{ repo.isPrivate ? 'private' : 'public' }}
                          </span>
                        </div>
                        <div class="text-[10px] text-[var(--muted-foreground)] truncate">{{ repo.description || 'No description' }}</div>
                        <div class="flex items-center gap-1.5 mt-1">
                          <button class="text-[10px] px-2 py-0.5 rounded bg-[var(--secondary)] text-[var(--foreground)]" type="button" @click.prevent="copyOrganisationCloneUrl(repo)">Copy clone URL</button>
                          <button class="text-[10px] px-2 py-0.5 rounded bg-[var(--secondary)] text-[var(--foreground)]" type="button" @click.prevent="openOrganisationRepository(repo)">Open</button>
                        </div>
                      </div>
                      <div class="text-[10px] text-[var(--muted-foreground)] whitespace-nowrap">★ {{ repo.stars }}</div>
                    </label>
                  </div>
                </div>

                <div class="flex items-center gap-2">
                  <AppButton
                    class="h-8 text-xs bg-[#10b981] text-white"
                    :disabled="organisationCloneBusy || selectedOrganisationCount === 0"
                    @click="cloneSelectedOrganisationRepos"
                  >
                    <Loader2 v-if="organisationCloneBusy" class="w-3.5 h-3.5 mr-1 animate-spin" />
                    Clone selected ({{ organisationCloneProtocol.toUpperCase() }})
                  </AppButton>
                  <div class="text-[10px] text-[var(--muted-foreground)]" v-if="organisationCloneBusy">
                    Progress: {{ organisationCloneDone }}/{{ selectedOrganisationCount }} (failed: {{ organisationCloneFailed }})
                  </div>
                </div>
              </div>

              <div class="border border-[var(--border)] rounded-lg p-3.5 bg-[var(--popover)]/50 space-y-3">
                <div class="text-[10px] font-semibold text-[var(--muted-foreground)] uppercase tracking-wider">Organisation / Team Data</div>

                <div class="grid grid-cols-4 gap-2">
                  <select
                    v-model="organisationFormProvider"
                    class="px-2.5 py-2 text-[11px] rounded bg-[var(--input-background)] border border-[var(--border)] text-[var(--foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]"
                  >
                    <option value="github">GitHub</option>
                    <option value="gitlab">GitLab</option>
                    <option value="bitbucket">Bitbucket</option>
                    <option value="azure">Azure</option>
                  </select>
                  <input
                    v-model="organisationFormName"
                    type="text"
                    placeholder="Organisation"
                    class="px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                  />
                  <input
                    v-model="organisationFormTeam"
                    type="text"
                    placeholder="Team"
                    class="px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                  />
                  <input
                    v-model="organisationFormRepoFilter"
                    type="text"
                    placeholder="Repo filter"
                    class="px-3 py-2 bg-[var(--input-background)] border border-[var(--border)] rounded text-xs text-[var(--foreground)] placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-1 focus:ring-[var(--ring)]/40"
                  />
                </div>

                <div class="flex items-center gap-2">
                  <AppButton class="h-8 text-xs bg-[var(--primary)] text-white" @click="addOrganisationProfile">
                    Save profile
                  </AppButton>
                </div>

                <div class="border border-[var(--border)] rounded p-2.5 max-h-[180px] overflow-y-auto bg-[var(--card)]/50">
                  <div v-if="!organisationProfiles.length" class="text-[11px] text-[var(--muted-foreground)] py-2">
                    No saved organisation profiles.
                  </div>
                  <div v-else class="space-y-2">
                    <div v-for="profile in organisationProfiles" :key="profile.id" class="flex items-center justify-between gap-2 border border-[var(--border)] rounded p-2">
                      <div class="min-w-0">
                        <div class="text-[11px] font-medium text-[var(--foreground)] truncate">{{ profile.organisation }} <span class="text-[var(--muted-foreground)]">({{ profile.provider }})</span></div>
                        <div class="text-[10px] text-[var(--muted-foreground)] truncate">Team: {{ profile.team || 'n/a' }} • Repo filter: {{ profile.repositoryFilter || 'n/a' }}</div>
                      </div>
                      <div class="flex items-center gap-1">
                        <button class="text-[10px] px-2 py-1 rounded bg-[var(--secondary)] text-[var(--foreground)]" @click="useOrganisationProfile(profile)">Use</button>
                        <button class="text-[10px] px-2 py-1 rounded bg-[#ef4444]/20 text-[#ef4444]" @click="removeOrganisationProfile(profile.id)">Delete</button>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </template>
        </div>
      </div>

      <div class="px-5 py-3 border-t border-[var(--border)] flex items-center justify-between text-[10px] text-[var(--muted-foreground)]">
        <div class="inline-flex items-center gap-1.5">
          <Shield class="w-3 h-3" />
          Integration tokens are encrypted at rest and never shown in plain text after saving.
        </div>
        <div class="inline-flex items-center gap-1.5 text-[#10b981]" v-if="hasGithubToken">
          <Check class="w-3 h-3" />
          GitHub ready
        </div>
      </div>
    </div>
  </div>
</template>
