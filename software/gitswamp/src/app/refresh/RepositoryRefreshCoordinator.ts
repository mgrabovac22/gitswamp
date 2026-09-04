export type RepositoryRefreshKind = "status" | "commits" | "branches" | "stashes" | "tags";

type RefreshRunner = (kinds: ReadonlySet<RepositoryRefreshKind>) => Promise<void> | void;

export class RepositoryRefreshCoordinator {
  private timer: ReturnType<typeof setTimeout> | null = null;
  private pending = new Set<RepositoryRefreshKind>();
  private running: Promise<void> | null = null;

  constructor(private readonly delayMs = 350) {}

  request(kinds: RepositoryRefreshKind | RepositoryRefreshKind[], runner: RefreshRunner): void {
    const nextKinds = Array.isArray(kinds) ? kinds : [kinds];
    for (const kind of nextKinds) {
      this.pending.add(kind);
    }

    if (this.timer) {
      clearTimeout(this.timer);
    }

    this.timer = setTimeout(() => {
      this.timer = null;
      void this.flush(runner);
    }, this.delayMs);
  }

  async flush(runner: RefreshRunner): Promise<void> {
    if (this.running) {
      await this.running;
    }

    if (this.pending.size === 0) {
      return;
    }

    const kinds = new Set(this.pending);
    this.pending.clear();

    this.running = Promise.resolve(runner(kinds)).finally(() => {
      this.running = null;
    });

    await this.running;

    if (this.pending.size > 0) {
      await this.flush(runner);
    }
  }

  cancel(): void {
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
    }
    this.pending.clear();
  }
}
