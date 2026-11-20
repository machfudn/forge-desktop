#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

export function parseReleaseMeta({
  refName = "",
  branchName = "",
  inputVersion = "",
  inputChannel = "auto",
  inputDraft = "false",
  fallbackVersion = "1.1.0",
  existingTags = [],
} = {}) {
  const raw = (inputVersion || refName || fallbackVersion).trim();
  if (!raw) {
    throw new Error("Version or release tag must not be empty.");
  }

  let version = raw.replace(/^refs\/tags\//, "").replace(/^v/i, "");
  const cleanBranch = String(branchName || "")
    .replace(/^refs\/heads\//, "")
    .trim()
    .toLowerCase();
  let channelNorm = String(inputChannel || "auto").trim().toLowerCase();
  if (channelNorm === "auto" && cleanBranch === "beta") {
    channelNorm = "beta";
  }

  let prerelease;
  if (channelNorm === "beta") {
    prerelease = true;
    if (!version.includes("-")) {
      const base = version.split("-")[0];
      const usedNums = (Array.isArray(existingTags) ? existingTags : [])
        .map((t) => String(t).trim().replace(/^refs\/tags\//, "").replace(/^v/i, ""))
        .map((v) => {
          const m = v.match(
            new RegExp(`^${base.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}-beta\\.(\\d+)$`)
          );
          return m ? Number(m[1]) : 0;
        })
        .filter((n) => n > 0);
      const nextNum = usedNums.length > 0 ? Math.max(...usedNums) + 1 : 1;
      version = `${base}-beta.${nextNum}`;
    }
  } else if (channelNorm === "stable") {
    prerelease = false;
    version = version.split("-")[0];
  } else {
    prerelease = version.includes("-");
  }
  const semverMatch = version.match(
    /^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?(?:\+([0-9A-Za-z.-]+))?$/
  );
  if (!semverMatch) {
    throw new Error(`Invalid semver version format: "${version}"`);
  }

  const major = Number(semverMatch[1]);
  const minor = Number(semverMatch[2]);
  const patch = Number(semverMatch[3]);
  const prePart = semverMatch[4] || "";

  let wixVersion = null;
  if (prePart) {
    const firstNumMatch = prePart.match(/(\d+)/);
    const buildNum = firstNumMatch
      ? Math.min(Number(firstNumMatch[1]), 65535)
      : 0;
    wixVersion = `${Math.min(major, 255)}.${Math.min(minor, 255)}.${Math.min(patch, 65535)}.${buildNum}`;
  }

  const tagName = `v${version}`;
  const channel = prerelease ? "beta" : "stable";
  const draft = String(inputDraft).toLowerCase() === "true";
  const releaseName = prerelease
    ? `Forge Desktop ${tagName} (Beta)`
    : `Forge Desktop ${tagName}`;

  return {
    tagName,
    version,
    prerelease,
    channel,
    draft,
    wixVersion,
    releaseName,
  };
}

export function syncManifests(rootDir, { version, wixVersion }) {
  const pkgPath = path.join(rootDir, "package.json");
  const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf8"));
  pkg.version = version;
  fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + "\n");

  const tauriConfPath = path.join(rootDir, "src-tauri", "tauri.conf.json");
  const tauriConf = JSON.parse(fs.readFileSync(tauriConfPath, "utf8"));
  tauriConf.version = version;
  if (wixVersion) {
    tauriConf.bundle = tauriConf.bundle || {};
    tauriConf.bundle.windows = tauriConf.bundle.windows || {};
    tauriConf.bundle.windows.wix = tauriConf.bundle.windows.wix || {};
    tauriConf.bundle.windows.wix.version = wixVersion;
  }
  fs.writeFileSync(tauriConfPath, JSON.stringify(tauriConf, null, 2) + "\n");

  const cargoPath = path.join(rootDir, "src-tauri", "Cargo.toml");
  const cargoRaw = fs.readFileSync(cargoPath, "utf8");
  const eol = cargoRaw.includes("\r\n") ? "\r\n" : "\n";
  const updatedCargo = cargoRaw.replace(
    /(\[package\][\s\S]*?^version\s*=\s*)"[^"]*"/m,
    `$1"${version}"`
  );
  if (updatedCargo === cargoRaw && !cargoRaw.includes(`version = "${version}"`)) {
    throw new Error("Failed to sync version in src-tauri/Cargo.toml");
  }
  fs.writeFileSync(cargoPath, updatedCargo.replace(/\r?\n/g, eol));
}

export const CATEGORY_ORDER = [
  "Features",
  "Bug Fixes",
  "Performance & Refactoring",
  "UI & Styling",
  "Documentation",
  "Maintenance & Dependencies",
  "Other Changes",
];

export function parseGitCommits(raw = "") {
  if (!raw || typeof raw !== "string") return [];
  const lines = raw.split(/\r?\n/).map((l) => l.trim()).filter(Boolean);
  const commits = [];

  for (const line of lines) {
    let hash = "";
    let author = "";
    let subject = "";

    if (line.includes("\t")) {
      const parts = line.split("\t");
      hash = (parts[0] || "").trim();
      author = (parts[1] || "").trim();
      subject = parts.slice(2).join("\t").trim();
    } else {
      const m = line.match(/^([0-9a-f]{7,40})\s+(.+)$/i);
      if (m) {
        hash = m[1];
        subject = m[2];
      } else {
        const dashMatch = line.match(/^[-*]\s+(.*?)(?:\s+\(([0-9a-f]{7,40})\))?$/i);
        if (dashMatch) {
          subject = dashMatch[1];
          hash = dashMatch[2] || "";
        } else {
          subject = line.replace(/^[-*]\s*/, "").trim();
        }
      }
    }

    if (!subject) continue;

    // Filter out merge commits
    if (
      /^merge (pull request|branch|remote-tracking branch)/i.test(subject) ||
      /^merged in /i.test(subject)
    ) {
      continue;
    }

    // Filter out version bump commits
    if (
      /^chore\(release\):\s*bump/i.test(subject) ||
      /^chore:\s*bump.*version/i.test(subject) ||
      /^bump version to /i.test(subject)
    ) {
      continue;
    }

    commits.push({ hash, author, subject });
  }

  return commits;
}

export function categorizeCommit(subject = "", author = "") {
  const s = String(subject || "").trim();
  const lowerAuthor = String(author || "").toLowerCase();

  if (lowerAuthor.includes("dependabot") || lowerAuthor.includes("[bot]")) {
    return "Maintenance & Dependencies";
  }

  const m = s.match(/^([a-z]+)(?:\([a-z0-9_.,-]+\))?!?:/i);
  if (m) {
    const type = m[1].toLowerCase();
    if (type === "feat") return "Features";
    if (type === "fix") return "Bug Fixes";
    if (type === "perf" || type === "refactor") return "Performance & Refactoring";
    if (type === "style") return "UI & Styling";
    if (type === "docs") return "Documentation";
    if (type === "chore" || type === "ci" || type === "build" || type === "test") {
      return "Maintenance & Dependencies";
    }
  }

  return "Other Changes";
}

export function extractPrDetails(generatedNotes = "") {
  const prMap = new Map();
  const lines = String(generatedNotes || "").split(/\r?\n/);
  for (const line of lines) {
    const m = line.match(/^[-*]\s+(.+?)\s+by\s+(@[^\s]+)\s+in\s+(#\d+|https?:\/\/[^\s]+\/pull\/(\d+))/i);
    if (m) {
      const subject = m[1].trim();
      const author = m[2].trim();
      const prRef = m[3].startsWith("http") ? `#${m[4]}` : m[3];
      prMap.set(subject.toLowerCase(), { prRef, author });
    }
  }
  return prMap;
}

export function extractExtraSections(generatedNotes = "") {
  let newContributors = "";
  let fullChangelog = "";

  const contribMatch = generatedNotes.match(/##+\s*New Contributors[\s\S]*?(?=(?:##+|$|\*\*Full Changelog\*\*|Full Changelog))/i);
  if (contribMatch) {
    newContributors = contribMatch[0].trim();
  }

  const changelogMatch = generatedNotes.match(/(?:\*\*Full Changelog\*\*|Full Changelog):?\s*(https?:\/\/[^\s]+)/i);
  if (changelogMatch) {
    fullChangelog = `**Full Changelog**: ${changelogMatch[1]}`;
  }

  return { newContributors, fullChangelog };
}

export function formatChangelog({ gitCommits = "", generatedNotes = "" } = {}) {
  const categoryMap = new Map();
  for (const cat of CATEGORY_ORDER) {
    categoryMap.set(cat, []);
  }

  const prMap = extractPrDetails(generatedNotes);
  const commits = Array.isArray(gitCommits) ? gitCommits : parseGitCommits(gitCommits);

  if (commits.length > 0) {
    for (const c of commits) {
      const cat = categorizeCommit(c.subject, c.author);
      let entry = `- ${c.subject}`;
      const prInfo = prMap.get(c.subject.toLowerCase());
      if (c.hash) {
        entry += ` (${c.hash})`;
      }
      if (prInfo) {
        entry += ` in ${prInfo.prRef}`;
      } else if (c.author && (c.author.toLowerCase().includes("dependabot") || c.author.toLowerCase().includes("[bot]"))) {
        entry += ` by @${c.author}`;
      }
      categoryMap.get(cat).push(entry);
    }
  } else if (generatedNotes) {
    const lines = String(generatedNotes).split(/\r?\n/);
    for (const line of lines) {
      const m = line.match(/^[-*]\s+(.*)$/);
      if (m) {
        const text = m[1].trim();
        const cat = categorizeCommit(text);
        categoryMap.get(cat).push(`- ${text}`);
      }
    }
  }

  const blocks = [];
  for (const cat of CATEGORY_ORDER) {
    const items = categoryMap.get(cat);
    if (items && items.length > 0) {
      blocks.push(`#### ${cat}\n${items.join("\n")}`);
    }
  }

  const { newContributors, fullChangelog } = extractExtraSections(generatedNotes);
  if (newContributors) {
    blocks.push(newContributors);
  }
  if (fullChangelog) {
    blocks.push(fullChangelog);
  }

  return blocks.join("\n\n");
}

export function buildReleaseNotes({
  meta,
  customNotes = "",
  generatedNotes = "",
  gitCommits = "",
} = {}) {
  const cleanCustom = String(customNotes || "").trim();
  const changelog = formatChangelog({ gitCommits, generatedNotes });
  const sections = [];

  if (meta.prerelease) {
    sections.push(`## ${meta.releaseName}`);
    sections.push(
      [
        `> **Pre-release (Beta Channel)** — Version \`${meta.tagName}\` is intended for testing the latest features and is not marked as *Latest*.`,
        "> To update from within the app, open **Settings -> Application & Updates -> Release Channel: Beta**.",
      ].join("\n")
    );
    if (cleanCustom) {
      sections.push(`### Beta Release Notes\n\n${cleanCustom}`);
    }
    if (changelog) {
      sections.push(`### What's Changed\n\n${changelog}`);
    }
    sections.push(
      [
        "### Packages & Portable Mode (Beta)",
        `- **Windows**: \`Forge.Desktop_${meta.version}_x64-setup.exe\` (NSIS), \`Forge.Desktop_${meta.version}_x64_en-US.msi\` (MSI), \`Forge-Desktop_${meta.version}_windows_x64-portable.exe\` / \`.zip\``,
        `- **Linux**: \`Forge.Desktop_${meta.version}_amd64.AppImage\`, \`.deb\`, \`.rpm\`, \`Forge-Desktop_${meta.version}_linux_x64-portable.tar.gz\``,
        `- **macOS**: \`Forge.Desktop_${meta.version}_universal.dmg\`, \`Forge-Desktop_${meta.version}_macos_universal.app.tar.gz\``,
        "- **Integrity**: Verify `SHA256SUMS` (and `SHA256SUMS.asc` if available).",
      ].join("\n")
    );
  } else {
    sections.push(`## ${meta.releaseName}`);
    sections.push(
      "Official stable release of **Forge Desktop** for Windows, Linux, and macOS (OS installers and portable packages)."
    );
    if (cleanCustom) {
      sections.push(`### Release Summary\n\n${cleanCustom}`);
    }
    if (changelog) {
      sections.push(`### What's Changed\n\n${changelog}`);
    }
    sections.push(
      [
        "### Installation & Portable Packages",
        `- **Windows**: Installer \`*-setup.exe\` / \`*.msi\`, or portable \`Forge-Desktop_${meta.version}_windows_x64-portable.exe\` / \`.zip\``,
        `- **Linux**: Packages \`*.deb\` / \`*.rpm\`, portable \`*.AppImage\` or \`Forge-Desktop_${meta.version}_linux_x64-portable.tar.gz\``,
        `- **macOS**: \`Forge.Desktop_${meta.version}_universal.dmg\` or archive \`Forge-Desktop_${meta.version}_macos_universal.app.tar.gz\``,
        "",
        "### Integrity Verification",
        "Verify download checksums using `SHA256SUMS` (and GPG signature `SHA256SUMS.asc` if available).",
      ].join("\n")
    );
  }

  return sections.join("\n\n") + "\n";
}

function parseCliFlag(args, flag) {
  const idx = args.indexOf(flag);
  if (idx !== -1 && idx + 1 < args.length) {
    return args[idx + 1];
  }
  return "";
}

function readExistingTagsFromEnv() {
  const filePath = process.env.EXISTING_TAGS_FILE || "";
  if (filePath && fs.existsSync(filePath)) {
    return fs
      .readFileSync(filePath, "utf8")
      .split(/\r?\n/)
      .map((s) => s.trim())
      .filter(Boolean);
  }
  return (process.env.EXISTING_TAGS || "")
    .split(/[\r\n,]+/)
    .map((s) => s.trim())
    .filter(Boolean);
}

function main() {
  const args = process.argv.slice(2);
  const cmd = args[0] || "meta";
  const rootDir = process.cwd();
  const tauriConf = JSON.parse(
    fs.readFileSync(path.join(rootDir, "src-tauri", "tauri.conf.json"), "utf8")
  );

  const meta = parseReleaseMeta({
    refName: process.env.RELEASE_REF_NAME || "",
    branchName: process.env.RELEASE_BRANCH_NAME || "",
    inputVersion: process.env.INPUT_VERSION || "",
    inputChannel: process.env.INPUT_CHANNEL || "auto",
    inputDraft: process.env.INPUT_DRAFT || "false",
    fallbackVersion: tauriConf.version || "1.1.0",
    existingTags: readExistingTagsFromEnv(),
  });

  if (cmd === "sync") {
    syncManifests(rootDir, meta);
  } else if (cmd === "notes") {
    const generatedPath = process.env.GENERATED_NOTES_FILE || "";
    const generatedNotes =
      generatedPath && fs.existsSync(generatedPath)
        ? fs.readFileSync(generatedPath, "utf8")
        : "";
    const commitsPath = process.env.GIT_COMMITS_FILE || "";
    const gitCommits =
      commitsPath && fs.existsSync(commitsPath)
        ? fs.readFileSync(commitsPath, "utf8")
        : "";
    const notesText = buildReleaseNotes({
      meta,
      customNotes: process.env.CUSTOM_NOTES || "",
      generatedNotes,
      gitCommits,
    });
    const outPath = parseCliFlag(args, "--out") || "release-notes.md";
    fs.writeFileSync(path.resolve(rootDir, outPath), notesText, "utf8");
    console.log(`Generated ${meta.channel} release notes -> ${outPath}`);
    return;
  }

  if (process.env.GITHUB_OUTPUT) {
    const lines = [
      `tag_name=${meta.tagName}`,
      `version=${meta.version}`,
      `prerelease=${meta.prerelease}`,
      `channel=${meta.channel}`,
      `draft=${meta.draft}`,
      `wix_version=${meta.wixVersion || ""}`,
      `release_name=${meta.releaseName}`,
    ];
    fs.appendFileSync(process.env.GITHUB_OUTPUT, lines.join("\n") + "\n");
  } else {
    console.log(JSON.stringify(meta, null, 2));
  }
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href
) {
  main();
}
