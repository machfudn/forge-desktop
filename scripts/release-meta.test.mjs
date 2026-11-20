import test from "node:test";
import assert from "node:assert/strict";
import {
  buildReleaseNotes,
  categorizeCommit,
  formatChangelog,
  parseGitCommits,
  parseReleaseMeta,
} from "./release-meta.mjs";

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

test("buildReleaseNotes renders English channel-specific sections for stable and beta", () => {
  const betaMeta = parseReleaseMeta({ refName: "v1.2.0-beta.1" });
  const betaNotes = buildReleaseNotes({
    meta: betaMeta,
    customNotes: "Testing new feature",
    gitCommits: "36d108c\tMachfudin\tfeat(palette): deep search command palette",
  });
  assert.match(betaNotes, /Pre-release \(Beta Channel\)/);
  assert.match(betaNotes, /### Beta Release Notes[\s\S]*Testing new feature/);
  assert.match(betaNotes, /### What's Changed[\s\S]*#### Features[\s\S]*deep search command palette/);
  assert.match(betaNotes, /### Packages & Portable Mode \(Beta\)/);
  assert.doesNotMatch(betaNotes, /Pra-rilis/);

  const stableMeta = parseReleaseMeta({ refName: "v1.2.0" });
  const stableNotes = buildReleaseNotes({
    meta: stableMeta,
    gitCommits: "a2873d2\tMachfudin\tfix(ui): prevent push dropdown popover clipping",
  });
  assert.match(stableNotes, /Official stable release of \*\*Forge Desktop\*\*/);
  assert.match(stableNotes, /### What's Changed[\s\S]*#### Bug Fixes[\s\S]*prevent push dropdown popover clipping/);
  assert.match(stableNotes, /### Installation & Portable Packages/);
  assert.match(stableNotes, /### Integrity Verification/);
  assert.doesNotMatch(stableNotes, /Daftar Perubahan/);
  assert.doesNotMatch(stableNotes, /Rilis stabil resmi/);
});

test("parseGitCommits filters merge commits and release bumps", () => {
  const sampleLog = `
fa4420d\tMachfudin\tchore(release): bump version to 1.1.6
36d108c\tMachfudin\tfeat(palette): enhance command palette with deep search and UX
4792365\tMachfudin\tMerge pull request #7 from machfudn/dependabot/npm_and_yarn/main/typescript-7.0.2
fca41fc\tMachfudin\tMerge branch 'main' into dependabot/npm_and_yarn/main/typescript-7.0.2
a2873d2\tMachfudin\tfix(ui): prevent push dropdown popover from clipping on narrow screens
2d867db\tdependabot[bot]\tchore(deps-dev)(deps-dev): bump typescript from 6.0.3 to 7.0.2
`;
  const commits = parseGitCommits(sampleLog);
  assert.equal(commits.length, 3);
  assert.equal(commits[0].subject, "feat(palette): enhance command palette with deep search and UX");
  assert.equal(commits[1].subject, "fix(ui): prevent push dropdown popover from clipping on narrow screens");
  assert.equal(commits[2].subject, "chore(deps-dev)(deps-dev): bump typescript from 6.0.3 to 7.0.2");
});

test("categorizeCommit accurately maps conventional commit types and bot authors", () => {
  assert.equal(categorizeCommit("feat(search): add quick search"), "Features");
  assert.equal(categorizeCommit("fix(auth): fix token expiration"), "Bug Fixes");
  assert.equal(categorizeCommit("perf(vfs): speed up cache"), "Performance & Refactoring");
  assert.equal(categorizeCommit("refactor: clean up git backend"), "Performance & Refactoring");
  assert.equal(categorizeCommit("style(button): modern button styles"), "UI & Styling");
  assert.equal(categorizeCommit("docs(readme): update build instructions"), "Documentation");
  assert.equal(categorizeCommit("chore(deps): update tauri"), "Maintenance & Dependencies");
  assert.equal(categorizeCommit("ci: optimize github actions"), "Maintenance & Dependencies");
  assert.equal(categorizeCommit("bump something", "dependabot[bot]"), "Maintenance & Dependencies");
  assert.equal(categorizeCommit("random unknown message"), "Other Changes");
});

test("formatChangelog groups commits under English headers and merges PR info", () => {
  const sampleLog = `
36d108c\tMachfudin\tfeat(palette): enhance command palette with deep search and UX
a2873d2\tMachfudin\tfix(ui): prevent push dropdown popover from clipping
7b260aa\tMachfudin\tstyle(diff): redesign hunk action buttons
2d867db\tdependabot[bot]\tchore(deps-dev)(deps-dev): bump typescript from 6.0.3 to 7.0.2
`;
  const sampleGenerated = `
## What's Changed
* chore(deps-dev)(deps-dev): bump typescript from 6.0.3 to 7.0.2 by @dependabot[bot] in #7

## New Contributors
* @dependabot[bot] made their first contribution in #1

**Full Changelog**: https://github.com/machfudn/forge-desktop/compare/v1.1.5...v1.1.6
`;
  const changelog = formatChangelog({
    gitCommits: sampleLog,
    generatedNotes: sampleGenerated,
  });

  assert.match(changelog, /#### Features[\s\S]*- feat\(palette\): enhance command palette with deep search and UX \(36d108c\)/);
  assert.match(changelog, /#### Bug Fixes[\s\S]*- fix\(ui\): prevent push dropdown popover from clipping \(a2873d2\)/);
  assert.match(changelog, /#### UI & Styling[\s\S]*- style\(diff\): redesign hunk action buttons \(7b260aa\)/);
  assert.match(changelog, /#### Maintenance & Dependencies[\s\S]*- chore\(deps-dev\)\(deps-dev\): bump typescript from 6.0.3 to 7.0.2 \(2d867db\) in #7/);
  assert.match(changelog, /## New Contributors[\s\S]*@dependabot\[bot\]/);
  assert.match(changelog, /\*\*Full Changelog\*\*: https:\/\/github\.com\/machfudn\/forge-desktop\/compare\/v1\.1\.5\.\.\.v1\.1\.6/);
});
