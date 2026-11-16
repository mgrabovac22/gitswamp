import assert from "node:assert/strict";
import { test } from "node:test";
import { nextReleaseVersion } from "../scripts/next-release-version.mjs";

test("a new major or minor uses the configured patch, independent of old runs", () => {
  assert.equal(nextReleaseVersion("0.6.0", ["v0.5.20", "v0.5.1000042", "v0.6.0-beta.1"]), "0.6.0");
  assert.equal(nextReleaseVersion("1.0.0", ["v0.9.88"]), "1.0.0");
  assert.equal(nextReleaseVersion("0.6.0", []), "0.6.0");
});
test("subsequent releases increment only the active line", () => {
  assert.equal(nextReleaseVersion("0.6.0", ["v0.6.9", "v0.6.10", "some-tag"]), "0.6.11");
  assert.equal(nextReleaseVersion("0.6.20", ["v0.6.10"]), "0.6.20");
});
test("retries reuse the tagged version and do not release old history", () => {
  assert.equal(nextReleaseVersion("0.6.0", ["v0.5.20", "v0.6.0"], ["v0.6.0"]), "0.6.0");
  assert.throws(() => nextReleaseVersion("0.6.0", ["v0.6.9", "v0.6.10"], ["v0.6.9"]), /older release tag/);
  assert.throws(() => nextReleaseVersion("0.5.0", ["v0.6.0"]), /behind/);
  assert.throws(() => nextReleaseVersion("0.5.0", ["v0.6.0"], ["v0.6.0"]), /behind/);
  assert.throws(() => nextReleaseVersion("1.0.0-beta.1", []), /stable semver/);
});
