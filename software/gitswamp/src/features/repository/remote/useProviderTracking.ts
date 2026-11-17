import { shallowRef, watch, type ComputedRef, type ShallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { IssueInfo, PullRequestInfo } from "@/types";

export type TrackingKind = "issue" | "pull-request";
export type TrackingProvider = "gitlab" | "bitbucket" | "azure";
export interface TrackingContext {
  path: string;
  remote: string;
  url: string;
  provider: TrackingProvider;
  credentials: Record<string, string | null | undefined>;
}

type Page<T> = { items: T[]; next: string | null };
type List<T> = Page<T> & { loaded: boolean; loading: boolean; error: string };
const empty = <T>(): List<T> => ({ items: [], next: null, loaded: false, loading: false, error: "" });

export function useProviderTracking(context: ComputedRef<TrackingContext | null>) {
  const issues = shallowRef<List<IssueInfo>>(empty());
  const pulls = shallowRef<List<PullRequestInfo>>(empty());

  watch(context, () => {
    issues.value = empty();
    pulls.value = empty();
  }, { flush: "sync" });

  function request<T>(active: TrackingContext, request: { kind: TrackingKind; number?: number; cursor?: string }) {
    return invoke<Page<T>>("remote_tracking", { path: active.path, remote: active.remote, credentials: active.credentials, request });
  }

  async function ensureList<T extends { number: number }>(kind: TrackingKind, state: ShallowRef<List<T>>, more: boolean): Promise<void> {
    const active = context.value;
    if (!active || (active.provider === "bitbucket" && kind === "issue")) return;
    const current = state.value;
    if (current.loading || (!more && current.loaded && !current.error) || (more && !current.next)) return;
    const cursor = more ? current.next ?? undefined : undefined;
    state.value = { ...current, loading: true, error: "" };
    try {
      const page = await request<T>(active, { kind, cursor });
      if (context.value !== active) return;
      const items = Array.from(new Map([...(more ? current.items : []), ...page.items].map(item => [item.number, item])).values());
      state.value = { items, next: items.length > 250 ? null : page.next, loaded: true, loading: false, error: "" };
    } catch (error) {
      if (context.value === active) state.value = { ...current, loading: false, error: String(error) };
    }
  }

  function load(kind: TrackingKind, more = false) {
    return kind === "issue" ? ensureList<IssueInfo>(kind, issues, more) : ensureList<PullRequestInfo>(kind, pulls, more);
  }

  async function detail<T>(kind: TrackingKind, number: number) {
    const active = context.value;
    if (!active) return null;
    const page = await request<T>(active, { kind, number });
    return context.value === active ? page.items[0] ?? null : null;
  }

  return { issues, pulls, load, detail };
}
