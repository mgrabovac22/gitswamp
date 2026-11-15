export type OptionsSection = "integrations" | "git" | "preferences" | "advanced" | "organisations" | "updates";
export type IntegrationPlatform = "github" | "gitlab" | "bitbucket" | "azure";
export type SettingResult = { id: string; title: string; section: OptionsSection; group: string; platform?: IntegrationPlatform; terms: string[] };

const sectionName: Record<OptionsSection, string> = {
  integrations: "Integrations", git: "Git Integration", preferences: "Preferences", advanced: "Advanced",
  organisations: "Organisations", updates: "Updates",
};
type Entry = [id: string, title: string, aliases?: string];
function normalize(value: string) {
  return value.normalize("NFKD").replace(/\p{M}/gu, "").toLowerCase().replace(/[^\p{L}\p{N}]+/gu, " ").trim();
}
function entries(section: OptionsSection, rows: Entry[], platform?: IntegrationPlatform, providerName?: string): SettingResult[] {
  const group = providerName ? `${sectionName[section]} / ${providerName}` : sectionName[section];
  return rows.map(([id, title, aliases]) => ({ id, title, section, group, platform, terms: normalize(`${title} ${aliases ?? ""} ${group}`).split(/\s+/) }));
}

const settings = [
  ...entries("integrations", [
    ["github-oauth", "GitHub account connection", "oauth sign in login device code povezivanje prijava"],
    ["github-token", "GitHub token", "pat personal access credentials password"],
    ["github-ssh", "GitHub SSH keys", "generate import register key email"],
  ], "github", "GitHub"),
  ...entries("integrations", [
    ["gitlab-token", "GitLab.com token", "api personal access pat sign in credentials"],
    ["gitlab-domain", "GitLab Self-Managed", "self hosted url domain instance token"],
    ["gitlab-ssh", "GitLab SSH key", "generate push self managed"],
  ], "gitlab", "GitLab"),
  ...entries("integrations", [
    ["bitbucket-token", "Bitbucket API token", "atlassian credentials app password login"],
    ["bitbucket-ssh", "Bitbucket SSH keys", "generate import register key email"],
  ], "bitbucket", "Bitbucket"),
  ...entries("integrations", [["azure-token", "Azure DevOps token", "pat personal access credentials domain organisation"]], "azure", "Azure DevOps"),
  ...entries("git", [
    ["git-runtime", "Git Runtime", "install download detection path executable git version"],
    ["fetch-enabled", "Background auto-fetch", "remote sync automatic fetch"],
    ["fetch-interval", "Auto-fetch interval", "time frequency minutes automatic fetch"],
  ]),
  ...entries("preferences", [
    ["theme", "Theme", "light dark mode system svijetlo tamno"],
    ["theme-preset", "Theme Preset", "palette accent color kitty appearance tema boje"],
    ["general-font", "General Font Size", "ui scale zoom typography velicina slova"],
    ["commit-font", "Commit Text Size", "graph commit font zoom"],
    ["compact", "Compact Mode", "density spacing layout"],
    ["dummy", "Dummy Mode", "simplified beginner help"],
    ["avatars", "Show Avatars", "profile pictures authors icons"],
    ["restore", "Restore Session on Start", "remember reopen startup tabs repository"],
    ["motion", "Reduced Motion", "accessibility movement animation effects"],
    ["wrap", "Wrap Long Diff Lines", "wordwrap horizontal scroll prelom redaka"],
    ["diff-numbers", "Show Diff Line Numbers", "numbering editor brojevi linija"],
    ["gitkeep", "Notify for .gitkeep Need", "empty folders directories notification"],
    ["gitignore", "Smart .gitignore Wizard", "ignore untracked files patterns generator"],
    ["secrets", "Accidental Secret Detector", "password api key credentials scan tajne lozinke"],
    ["undo", "Undo Destructive Actions", "recovery revert confirmation rollback ponisti"],
    ["autopsy", "Bug Autopsy", "blame debugging analysis"],
    ["identity", "Git Identity Guard", "author email username identity check"],
    ["analyzer", "Commit Analyzer", "conventional commits lint rules message poruka"],
    ["empty-commit", "Allow Empty Commit", "no changes blank commit"],
    ["org-auto-select", "Auto-select Connected Organisation Repos", "batch clone select private projects"],
    ["org-clear", "Clear Organisation Selection After Clone", "batch reset checked repos"],
    ["org-remember", "Remember Organisation Clone Folder", "default clone path location destination"],
    ["org-links", "Confirm External Repository Links", "browser open warn confirmation"],
    ["git-executable", "Git Executable", "installation binary path location"],
  ]),
  ...entries("advanced", [
    ["graph-animation", "Disable Commit Graph Animations", "effects reduce motion speed"],
    ["graph-scroll", "Smooth Scroll in Graph", "scrolling animation"],
    ["maintenance-interval", "Background maintenance interval", "schedule frequency every minutes"],
    ["maintenance-presets", "Background maintenance presets", "quiet guard speed off disable automatic"],
    ["health", "Silent Repository Health Refresh", "refresh status branches tags stashes"],
    ["hygiene", "Remote Hygiene Pulse", "remote refs background sync"],
    ["focus-sync", "Focus Sync Pulse", "refresh on app focus"],
    ["idle", "Idle-only Background Work", "quiet pause typing idle maintenance"],
    ["stale", "Stale Work Reminder", "old unsaved unstaged changes warning"],
    ["behind", "Behind Branch Reminder", "remote ahead branch update warning"],
    ["worktree-reminder", "Large Worktree Reminder", "changed files threshold limit warning"],
    ["conflict", "Conflict Reminder Pulse", "merge conflicts unresolved warning"],
    ["visualise", "Visualise operations", "visualize merge push conflict animation popup"],
    ["preloader", "Commit Detail Preloader", "cache files faster preload background"],
  ]),
  ...entries("organisations", [
    ["org-provider", "Organisation search provider", "github gitlab bitbucket azure switch source"],
    ["org-search", "Organisation Repositories", "search organization team project repository"],
    ["org-protocol", "Clone protocol", "https ssh batch clone"],
    ["org-visibility", "Repository visibility", "private public all filter"],
    ["org-filter", "Filter loaded repositories", "organisation local filter results"],
    ["org-folder", "Batch clone destination", "organisation folder path browse"],
    ["org-profiles", "Organisation Profiles", "saved search organisation team repository"],
  ]),
  ...entries("updates", [["updates", "Application updates", "check update download release version upgrade deb appimage azuriranje"]]),
];

function editDistance(a: string, b: string) {
  let prior = Array.from({ length: b.length + 1 }, (_, index) => index);
  let beforePrior = prior;
  for (let i = 1; i <= a.length; i++) {
    const current = [i];
    for (let j = 1; j <= b.length; j++) {
      const substitution = prior[j - 1]! + Number(a[i - 1] !== b[j - 1]);
      let value = Math.min(prior[j]! + 1, current[j - 1]! + 1, substitution);
      if (i > 1 && j > 1 && a[i - 1] === b[j - 2] && a[i - 2] === b[j - 1]) value = Math.min(value, beforePrior[j - 2]! + 1);
      current.push(value);
    }
    beforePrior = prior;
    prior = current;
  }
  return prior[b.length]!;
}

function score(term: string, word: string) {
  if (word === term) return 14;
  if (word.startsWith(term)) return 11;
  if (term.length >= 3 && word.includes(term)) return 8;
  if (term.length < 3) return 0;
  const allowed = term.length >= 6 ? 2 : 1;
  if (Math.abs(word.length - term.length) <= allowed) {
    const errors = editDistance(term, word);
    if (errors <= allowed) return 7 - errors;
  }
  if (term.length >= 4 && word.length - term.length <= 4) {
    let index = 0;
    for (const letter of word) if (letter === term[index]) index++;
    if (index === term.length) return 4;
  }
  return 0;
}

export function searchSettings(input: string): SettingResult[] {
  const query = normalize(input.slice(0, 96));
  if (!query) return [];
  const terms = [...new Set(query.split(/\s+/))].slice(0, 7);
  return settings.flatMap(setting => {
    const matches = terms.map(term => Math.max(...setting.terms.map(word => score(term, word))));
    if (matches.includes(0)) return [];
    const name = normalize(setting.title);
    const boost = name === query ? 30 : name.startsWith(query) ? 17 : name.includes(query) ? 12 : 0;
    return [{ setting, rank: boost + matches.reduce((sum, value) => sum + value, 0) }];
  }).sort((a, b) => b.rank - a.rank || a.setting.title.localeCompare(b.setting.title)).slice(0, 12).map(({ setting }) => setting);
}
