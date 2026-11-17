import type { BranchInfo } from "@/types";
import type { WorkflowConfig } from "@/features/repository/workflow/branchWorkflow";

const common = ["dev", "main", "master", "develop"];
const compareName = new Intl.Collator(undefined, { numeric: true, sensitivity: "base" }).compare;

export function branchOnRemote(ref: string, remote: string): string {
  const name = ref.replace(/^refs\/remotes\/|^remotes\//, "");
  const branch = name.startsWith(`${remote}/`) ? name.slice(remote.length + 1) : "";
  return branch === "HEAD" ? "" : branch;
}

export function fetchedBranches(branches: BranchInfo[], remote: string): string[] {
  return [...new Set(branches.map(branch => branchOnRemote(branch.name, remote)).filter(Boolean))];
}

export function recommendedBase(source: string, branches: readonly string[], defaultBranch: string, workflow?: WorkflowConfig | null) {
  const available = (name: string) => !!name && branches.includes(name);
  const hasPrefix = (prefix: string) => !!prefix && source.startsWith(prefix) && source.length > prefix.length;
  let branch = "";
  let reason = "";
  if (workflow && workflow.strategy !== "manual") {
    if (workflow.strategy === "gitflow") {
      if (hasPrefix(workflow.release) || hasPrefix(workflow.hotfix)) branch = workflow.primary;
      else if (hasPrefix(workflow.feature) || hasPrefix(workflow.fix)) branch = workflow.development;
    } else branch = workflow.primary;
    if (available(branch)) reason = "Repository workflow";
    else branch = "";
  }
  if (!branch && available(defaultBranch)) {
    branch = defaultBranch;
    reason = "GitHub default";
  }
  if (!branch) {
    branch = ["main", "master", "develop", "dev"].find(available) ?? "";
    if (branch) reason = "Common base";
  }
  // An integration branch selected as the source does not imply a reverse merge.
  return branch && source !== branch ? { branch, reason } : null;
}

export function orderBranches(branches: readonly string[], preferred: readonly string[], excluded: string) {
  const rank = [...new Set([...preferred.filter(Boolean), ...common])];
  const names = new Set(branches);
  names.delete(excluded);
  const pinned = rank.filter(branch => names.delete(branch));
  return { pinned, other: [...names].sort(compareName) };
}
