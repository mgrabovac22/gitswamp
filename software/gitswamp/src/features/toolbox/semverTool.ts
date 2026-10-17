export type SemverParts = {
  major: number;
  minor: number;
  patch: number;
  prerelease: string[];
};

const SEMVER_PATTERN = /^v?(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?(?:\+[0-9A-Za-z.-]+)?$/;

export function parseSemver(value: string): SemverParts | null {
  const match = value.trim().match(SEMVER_PATTERN);
  if (!match) return null;

  return {
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3]),
    prerelease: match[4] ? match[4].split(".") : [],
  };
}

function comparePrerelease(a: string[], b: string[]): number {
  if (!a.length && !b.length) return 0;
  if (!a.length) return 1;
  if (!b.length) return -1;

  for (let index = 0; index < Math.max(a.length, b.length); index += 1) {
    const left = a[index];
    const right = b[index];
    if (left === undefined) return -1;
    if (right === undefined) return 1;
    if (left === right) continue;

    const leftNumber = /^\d+$/.test(left);
    const rightNumber = /^\d+$/.test(right);
    if (leftNumber && rightNumber) return Number(left) - Number(right);
    if (leftNumber) return -1;
    if (rightNumber) return 1;
    return left < right ? -1 : 1;
  }
  return 0;
}

export function compareSemver(left: SemverParts, right: SemverParts): number {
  if (left.major !== right.major) return left.major - right.major;
  if (left.minor !== right.minor) return left.minor - right.minor;
  if (left.patch !== right.patch) return left.patch - right.patch;
  return comparePrerelease(left.prerelease, right.prerelease);
}

export function bumpSemver(value: SemverParts, type: "major" | "minor" | "patch"): string {
  if (type === "major") return `${value.major + 1}.0.0`;
  if (type === "minor") return `${value.major}.${value.minor + 1}.0`;
  return `${value.major}.${value.minor}.${value.patch + 1}`;
}
