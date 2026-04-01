import type { CommitInfo } from "@/types";

interface AvatarSvgOptions {
  svgBgOuter: string;
  svgBgInner: string;
  colors: readonly string[];
}

interface MergeDotOptions {
  topColor?: string;
  bottomColor?: string;
  showTop?: boolean;
  showBottom?: boolean;
}

export function nameHash(name: string): number {
  let h = 0;
  for (let i = 0; i < name.length; i++) h = (name.codePointAt(i) ?? 0) + ((h << 5) - h);
  return Math.abs(h);
}

export function avatarColor(name: string, colors: readonly string[]): string {
  return colors[nameHash(name) % colors.length];
}

export function avatarSvg(
  name: string,
  cx: number,
  cy: number,
  r: number,
  branchColor: string,
  options: AvatarSvgOptions,
): string {
  const { svgBgOuter, svgBgInner, colors } = options;
  const h = nameHash(name);
  const c = avatarColor(name, colors);
  const s = r * 0.48;
  const ox = cx - s * 1.5;
  const oy = cy - s * 1.5;
  let cells = "";

  for (let row = 0; row < 3; row++) {
    for (let col = 0; col < 2; col++) {
      const bit = (h >> (row * 2 + col)) & 1;
      if (bit) {
        const x1 = ox + col * s;
        const y1 = oy + row * s;
        const x2 = ox + (2 - col) * s;
        cells += '<rect class="commit-avatar-pattern-cell" x="' + x1 + '" y="' + y1 + '" width="' + s + '" height="' + s + '" rx="1" fill="' + c + '" opacity="0.85"/>';
        cells += '<rect class="commit-avatar-pattern-cell" x="' + x2 + '" y="' + y1 + '" width="' + s + '" height="' + s + '" rx="1" fill="' + c + '" opacity="0.85"/>';
      }
    }
    const centerBit = (h >> (row + 6)) & 1;
    if (centerBit) {
      cells += '<rect class="commit-avatar-pattern-cell" x="' + (ox + s) + '" y="' + (oy + row * s) + '" width="' + s + '" height="' + s + '" rx="1" fill="' + c + '" opacity="0.9"/>';
    }
  }

  return '<g class="commit-avatar" style="--avatar-color:' + c + '"><circle class="commit-avatar-outer" cx="' + cx + '" cy="' + cy + '" r="' + (r + 2) + '" fill="' + svgBgOuter + '"/>'
    + '<circle class="commit-avatar-inner" cx="' + cx + '" cy="' + cy + '" r="' + r + '" fill="' + svgBgInner + '"/>'
    + '<g class="commit-avatar-pattern">' + cells + '</g>'
    + '<circle class="commit-avatar-ring" cx="' + cx + '" cy="' + cy + '" r="' + r + '" fill="none" stroke="' + branchColor + '" stroke-width="1.5" opacity="0.8"/></g>';
}

export function mergeDotSvg(
  cx: number,
  cy: number,
  color: string,
  svgBgOuter: string,
  options: MergeDotOptions = {},
): string {
  const top = options.topColor || color;
  const bottom = options.bottomColor || color;
  const showTop = options.showTop ?? true;
  const showBottom = options.showBottom ?? true;
  const topSegment = showTop
    ? '<line x1="' + cx + '" y1="' + (cy - 7) + '" x2="' + cx + '" y2="' + cy + '" stroke="' + top + '" stroke-width="2" stroke-linecap="round" opacity="0.95"/>'
    : "";
  const bottomSegment = showBottom
    ? '<line x1="' + cx + '" y1="' + cy + '" x2="' + cx + '" y2="' + (cy + 7) + '" stroke="' + bottom + '" stroke-width="2" stroke-linecap="round" opacity="0.95"/>'
    : "";

  return '<circle cx="' + cx + '" cy="' + cy + '" r="7" fill="' + svgBgOuter + '"/>'
    + topSegment
    + bottomSegment
    + '<circle cx="' + cx + '" cy="' + cy + '" r="4" fill="' + color + '" opacity="0.94"/>';
}

export function isMergeCommit(commit: CommitInfo): boolean {
  return commit.parent_shas.length > 1;
}

export function providerIconSvg(remoteProvider: "github" | "gitlab" | "bitbucket" | "azure" | "unknown" | undefined): string {
  switch (remoteProvider) {
    case "github":
      return '<svg class="w-2.5 h-2.5 flex-shrink-0 opacity-75" viewBox="0 0 16 16" fill="currentColor"><path d="M8 1.2a6.8 6.8 0 00-2.15 13.25c.34.06.46-.14.46-.33v-1.15c-1.88.4-2.27-.8-2.27-.8-.31-.76-.75-.96-.75-.96-.61-.42.05-.41.05-.41.67.04 1.02.69 1.02.69.6 1.03 1.57.73 1.95.56.06-.43.23-.73.41-.9-1.5-.18-3.08-.75-3.08-3.35 0-.74.26-1.34.69-1.81-.07-.17-.3-.88.06-1.83 0 0 .56-.18 1.85.69a6.4 6.4 0 013.36 0c1.28-.87 1.84-.69 1.84-.69.37.95.14 1.66.07 1.83.43.47.69 1.07.69 1.81 0 2.6-1.59 3.17-3.1 3.34.24.21.45.63.45 1.28v1.9c0 .19.12.39.47.33A6.8 6.8 0 008 1.2z"/></svg>';
    case "gitlab":
      return '<svg class="w-2.5 h-2.5 flex-shrink-0 opacity-75" viewBox="0 0 16 16" fill="currentColor"><path d="M8 14.5l-2.3-7h4.6L8 14.5zM1.4 7.5L.5 10.3c-.1.2 0 .5.2.6L8 14.5 1.4 7.5zm.8-2.2L.5 10.3h3.6l1.1-3.4-3 1.6zm11.4 2.2L15.5 10.3c.1.2 0 .5-.2.6L8 14.5l6.6-7zm-.8-2.2l1.7 5h-3.6l-1.1-3.4 3-1.6z"/></svg>';
    case "bitbucket":
      return '<svg class="w-2.5 h-2.5 flex-shrink-0 opacity-75" viewBox="0 0 16 16" fill="currentColor"><path d="M.8 1.5h14.4c.5 0 .8.4.8.8v.2l-2 12c-.1.4-.4.7-.8.7H3c-.4 0-.8-.3-.8-.7l-2-12c-.1-.4.2-.8.6-.8zm8.4 9H6.8L6 6.5h4l-.8 4z"/></svg>';
    case "azure":
      return '<svg class="w-2.5 h-2.5 flex-shrink-0 opacity-75" viewBox="0 0 16 16" fill="currentColor"><path d="M4.8 1.5L1 6.3l2.3 8.2h9.4l2.3-8.2L11.2 1.5H4.8zM8 5l2 3.5H6L8 5z"/></svg>';
    default:
      return '<svg class="w-2.5 h-2.5 flex-shrink-0 opacity-70" viewBox="0 0 16 16" fill="currentColor"><path d="M8 1C5.2 1 3 3 3 5.5c0 .8.2 1.5.5 2.1C2.1 8.2 1 9.5 1 11c0 2 1.6 3.5 3.6 3.5h7.8c2 0 3.6-1.5 3.6-3.5 0-1.5-1.1-2.8-2.5-3.4.3-.6.5-1.3.5-2.1C13 3 10.8 1 8 1z"/></svg>';
  }
}
