import { onScopeDispose, watch } from "vue";
import type { useToast } from "./useToast";

type OperationNoticeSource = {
  path: () => string;
  kind: () => string | undefined;
  hasConflicts: () => boolean;
  loading: () => boolean;
};

export function useRepositoryOperationNotice(
  source: OperationNoticeSource,
  toast: ReturnType<typeof useToast>,
  review: () => void,
  abortMerge: () => void,
  abortCherryPick: () => void,
) {
  let noticeId: number | undefined;
  let noticeKey: string | undefined;

  const clear = () => {
    if (noticeId !== undefined) toast.remove(noticeId);
    noticeId = undefined;
    noticeKey = undefined;
  };

  watch(() => [source.path(), source.kind(), source.hasConflicts(), source.loading()] as const,
    ([path, kind, conflicts, loading]) => {
      if (!path || (kind !== "merge" && kind !== "cherry-pick") || conflicts) {
        clear();
        return;
      }
      const key = JSON.stringify([path, kind]);
      if (noticeKey !== key) clear();
      if (loading || noticeKey === key) return;
      noticeKey = key;
      const cherryPick = kind === "cherry-pick";
      noticeId = toast.action("info", `Conflicts are resolved, but the ${kind} is still active.`, [
        { label: cherryPick ? "Review changes" : "Review merge", style: "primary", onClick: review },
        { label: `Abort ${kind}`, style: "danger", onClick: cherryPick ? abortCherryPick : abortMerge },
      ], 20000, cherryPick
        ? "Commit the resolved changes to complete the cherry-pick, or abort it using the header button."
        : "Review and commit the staged changes to finish the merge, or abort it using the header button.");
    });

  onScopeDispose(clear);
}
