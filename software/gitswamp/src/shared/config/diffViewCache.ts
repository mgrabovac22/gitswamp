import type { FileDiff } from "@/types";

interface CacheEntry<T> {
  value: T;
  size: number;
  expiresAt: number;
  lastAccess: number;
}

interface CacheStats {
  totalSize: number;
}

interface CachePolicy {
  ttlMs: number;
  maxEntries: number;
  maxTotalSize: number;
}

const TEXT_CACHE_TTL_MS = 25_000;
const TEXT_CACHE_MAX_ENTRIES = 6;
const TEXT_CACHE_MAX_TOTAL_CHARS = 520_000;
const TEXT_CACHE_MAX_ITEM_CHARS = 260_000;

const DIFF_CACHE_TTL_MS = 22_000;
const DIFF_CACHE_MAX_ENTRIES = 4;
const DIFF_CACHE_MAX_TOTAL_CHARS = 340_000;
const DIFF_CACHE_MAX_ITEM_CHARS = 170_000;

const textCache = new Map<string, CacheEntry<string>>();
const diffCache = new Map<string, CacheEntry<FileDiff>>();
const textStats: CacheStats = { totalSize: 0 };
const diffStats: CacheStats = { totalSize: 0 };

function removeCacheEntry<T>(
  cache: Map<string, CacheEntry<T>>,
  stats: CacheStats,
  key: string,
): void {
  const entry = cache.get(key);
  if (!entry) {
    return;
  }

  cache.delete(key);
  stats.totalSize = Math.max(0, stats.totalSize - entry.size);
}

function removeExpiredEntries<T>(
  cache: Map<string, CacheEntry<T>>,
  stats: CacheStats,
  now: number,
): void {
  for (const [key, entry] of cache.entries()) {
    if (entry.expiresAt <= now) {
      removeCacheEntry(cache, stats, key);
    }
  }
}

function removeLeastRecentlyUsedEntry<T>(
  cache: Map<string, CacheEntry<T>>,
  stats: CacheStats,
): void {
  let oldestKey: string | null = null;
  let oldestAccess = Number.POSITIVE_INFINITY;

  for (const [key, entry] of cache.entries()) {
    if (entry.lastAccess < oldestAccess) {
      oldestAccess = entry.lastAccess;
      oldestKey = key;
    }
  }

  if (oldestKey) {
    removeCacheEntry(cache, stats, oldestKey);
  }
}

function pruneCache<T>(
  cache: Map<string, CacheEntry<T>>,
  stats: CacheStats,
  now: number,
  maxEntries: number,
  maxTotalSize: number,
): void {
  removeExpiredEntries(cache, stats, now);

  while (cache.size > maxEntries || stats.totalSize > maxTotalSize) {
    removeLeastRecentlyUsedEntry(cache, stats);

    if (cache.size === 0) {
      break;
    }
  }
}

function getCachedEntry<T>(
  cache: Map<string, CacheEntry<T>>,
  stats: CacheStats,
  key: string,
  now: number,
): T | null {
  const entry = cache.get(key);
  if (!entry) {
    return null;
  }

  if (entry.expiresAt <= now) {
    removeCacheEntry(cache, stats, key);
    return null;
  }

  entry.lastAccess = now;
  return entry.value;
}

function setCachedEntry<T>(
  cache: Map<string, CacheEntry<T>>,
  stats: CacheStats,
  key: string,
  value: T,
  size: number,
  policy: CachePolicy,
): void {
  const normalizedSize = Math.max(0, size);
  const now = Date.now();

  const existing = cache.get(key);
  if (existing) {
    stats.totalSize = Math.max(0, stats.totalSize - existing.size);
  }

  cache.set(key, {
    value,
    size: normalizedSize,
    expiresAt: now + policy.ttlMs,
    lastAccess: now,
  });
  stats.totalSize += normalizedSize;

  pruneCache(cache, stats, now, policy.maxEntries, policy.maxTotalSize);
}

export function getCachedDiffText(key: string): string | null {
  const now = Date.now();
  pruneCache(textCache, textStats, now, TEXT_CACHE_MAX_ENTRIES, TEXT_CACHE_MAX_TOTAL_CHARS);
  return getCachedEntry(textCache, textStats, key, now);
}

export function setCachedDiffText(key: string, content: string): void {
  if (content.length > TEXT_CACHE_MAX_ITEM_CHARS) {
    return;
  }

  setCachedEntry(
    textCache,
    textStats,
    key,
    content,
    content.length,
    {
      ttlMs: TEXT_CACHE_TTL_MS,
      maxEntries: TEXT_CACHE_MAX_ENTRIES,
      maxTotalSize: TEXT_CACHE_MAX_TOTAL_CHARS,
    },
  );
}

export function getCachedStructuredDiff(key: string): FileDiff | null {
  const now = Date.now();
  pruneCache(diffCache, diffStats, now, DIFF_CACHE_MAX_ENTRIES, DIFF_CACHE_MAX_TOTAL_CHARS);
  return getCachedEntry(diffCache, diffStats, key, now);
}

export function setCachedStructuredDiff(key: string, diff: FileDiff, estimatedChars: number): void {
  if (estimatedChars > DIFF_CACHE_MAX_ITEM_CHARS) {
    return;
  }

  setCachedEntry(
    diffCache,
    diffStats,
    key,
    diff,
    estimatedChars,
    {
      ttlMs: DIFF_CACHE_TTL_MS,
      maxEntries: DIFF_CACHE_MAX_ENTRIES,
      maxTotalSize: DIFF_CACHE_MAX_TOTAL_CHARS,
    },
  );
}

export function pruneDiffViewerCaches(): void {
  const now = Date.now();
  pruneCache(textCache, textStats, now, TEXT_CACHE_MAX_ENTRIES, TEXT_CACHE_MAX_TOTAL_CHARS);
  pruneCache(diffCache, diffStats, now, DIFF_CACHE_MAX_ENTRIES, DIFF_CACHE_MAX_TOTAL_CHARS);
}

export function clearDiffViewerCaches(): void {
  textCache.clear();
  diffCache.clear();
  textStats.totalSize = 0;
  diffStats.totalSize = 0;
}
