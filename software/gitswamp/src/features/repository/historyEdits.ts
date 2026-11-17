import { GitCommitHorizontal, GitMerge, PencilLine, Plus, Trash2 } from "lucide-vue-next";

export const amendActions = {
  edit: { label: "Edit or add files…", description: "Pause on this commit to stage changes and amend it.", icon: Plus },
  reword: { label: "Reword…", description: "Change the message without changing this commit's files.", icon: PencilLine },
  squash: { label: "Squash into parent…", description: "Combine both changes and edit the resulting message.", icon: GitMerge },
  fixup: { label: "Fixup into parent…", description: "Combine both changes and keep only the parent's message.", icon: GitCommitHorizontal },
  drop: { label: "Drop commit…", description: "Remove the commit and replay the later changes.", icon: Trash2 },
} as const;

export type AmendAction = keyof typeof amendActions;
export type AmendConfirmation = { head: string; branch: string; message?: string; acknowledgePublished: boolean };
export type PatchRequest = { kind: "import" | "export"; sha?: string };
export type PatchScope = "all-tracked" | "staged" | "unstaged";
