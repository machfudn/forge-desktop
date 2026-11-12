import test from "node:test";
import assert from "node:assert/strict";
import { buildReleaseNotes, parseReleaseMeta } from "./release-meta.mjs";

test("main branch follows manifest version for stable and explicit beta", () => {
  const stable = parseReleaseMeta({
    branchName: "main",
    fallbackVersion: "1.2.0",
  });
  assert.equal(stable.tagName, "v1.2.0");
  assert.equal(stable.version, "1.2.0");
  assert.equal(stable.prerelease, false);
  assert.equal(stable.channel, "stable");
  assert.equal(stable.wixVersion, null);

  const explicitBeta = parseReleaseMeta({
    branchName: "refs/heads/main",
    fallbackVersion: "1.2.0-beta.1",
  });
  assert.equal(explicitBeta.tagName, "v1.2.0-beta.1");
  assert.equal(explicitBeta.prerelease, true);
  assert.equal(explicitBeta.channel, "beta");
  assert.equal(explicitBeta.wixVersion, "1.2.0.1");
});

test("beta branch forces beta channel and increments -beta.N from existing tags", () => {
  const first = parseReleaseMeta({
    branchName: "beta",
    fallbackVersion: "1.1.0",
    existingTags: ["v1.1.0"],
  });
  assert.equal(first.tagName, "v1.1.0-beta.1");
  assert.equal(first.prerelease, true);
  assert.equal(first.channel, "beta");
  assert.equal(first.wixVersion, "1.1.0.1");

  const next = parseReleaseMeta({
    branchName: "refs/heads/beta",
    fallbackVersion: "1.1.0",
    existingTags: ["v1.1.0", "v1.1.0-beta.1", "refs/tags/v1.1.0-beta.2", "v1.0.0-beta.9"],
  });
  assert.equal(next.tagName, "v1.1.0-beta.3");
  assert.equal(next.version, "1.1.0-beta.3");
  assert.equal(next.wixVersion, "1.1.0.3");
});

test("beta branch keeps explicit prerelease suffix and honors manual stable override", () => {
  const explicit = parseReleaseMeta({
    branchName: "beta",
    fallbackVersion: "1.2.0-rc.2",
    existingTags: ["v1.2.0-beta.5"],
  });
  assert.equal(explicit.tagName, "v1.2.0-rc.2");
  assert.equal(explicit.prerelease, true);
  assert.equal(explicit.wixVersion, "1.2.0.2");

  const overrideStable = parseReleaseMeta({
    branchName: "beta",
    inputVersion: "1.2.0-beta.3",
    inputChannel: "stable",
  });
  assert.equal(overrideStable.tagName, "v1.2.0");
  assert.equal(overrideStable.prerelease, false);
  assert.equal(overrideStable.channel, "stable");
});

test("buildReleaseNotes renders channel-specific sections for stable and beta", () => {
  const betaMeta = parseReleaseMeta({ refName: "v1.2.0-beta.1" });
  const betaNotes = buildReleaseNotes({
    meta: betaMeta,
    customNotes: "Uji coba fitur baru",
    generatedNotes: "- feat: tambah diff viewer",
  });
  assert.match(betaNotes, /Pra-rilis \(Beta Channel\)/);
  assert.match(betaNotes, /### Catatan Rilis Beta[\s\S]*Uji coba fitur baru/);
  assert.match(betaNotes, /### Perubahan Sejak Rilis Sebelumnya[\s\S]*tambah diff viewer/);

  const stableMeta = parseReleaseMeta({ refName: "v1.2.0" });
  const stableNotes = buildReleaseNotes({
    meta: stableMeta,
    generatedNotes: "- fix: perbaiki crash",
  });
  assert.match(stableNotes, /Rilis stabil resmi/);
  assert.match(stableNotes, /### Daftar Perubahan[\s\S]*perbaiki crash/);
  assert.doesNotMatch(stableNotes, /Pra-rilis/);
});
