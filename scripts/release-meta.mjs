#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

export function parseReleaseMeta({
  refName = "",
  inputVersion = "",
  inputChannel = "auto",
  inputDraft = "false",
  fallbackVersion = "1.1.0",
} = {}) {
  const raw = (inputVersion || refName || fallbackVersion).trim();
  if (!raw) {
    throw new Error("Version atau tag rilis tidak boleh kosong.");
  }

  let version = raw.replace(/^refs\/tags\//, "").replace(/^v/i, "");
  const channelNorm = String(inputChannel || "auto").trim().toLowerCase();

  let prerelease;
  if (channelNorm === "beta") {
    prerelease = true;
    if (!version.includes("-")) {
      version = `${version}-beta.1`;
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
    throw new Error(`Format versi semver tidak valid: "${version}"`);
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
    throw new Error("Gagal menyinkronkan version di src-tauri/Cargo.toml");
  }
  fs.writeFileSync(cargoPath, updatedCargo.replace(/\r?\n/g, eol));
}

export function buildReleaseNotes({
  meta,
  customNotes = "",
  generatedNotes = "",
} = {}) {
  const cleanCustom = String(customNotes || "").trim();
  const cleanGenerated = String(generatedNotes || "").trim();
  const sections = [];

  if (meta.prerelease) {
    sections.push(`## ${meta.releaseName}`);
    sections.push(
      [
        "> **Pra-rilis (Beta Channel)** — Versi \`${meta.tagName}\` ditujukan untuk pengujian fitur terbaru dan tidak ditandai sebagai rilis *Latest*.",
        "> Untuk memperbarui dari aplikasi, buka **Pengaturan -> Aplikasi & Pembaruan -> Saluran Rilis: Beta**.",
      ].join("\n")
    );
    if (cleanCustom) {
      sections.push(`### Catatan Rilis Beta\n\n${cleanCustom}`);
    }
    if (cleanGenerated) {
      sections.push(`### Perubahan Sejak Rilis Sebelumnya\n\n${cleanGenerated}`);
    }
    sections.push(
      [
        "### Paket & Mode Portable (Beta)",
        `- **Windows**: \`Forge.Desktop_${meta.version}_x64-setup.exe\` (NSIS), \`Forge.Desktop_${meta.version}_x64_en-US.msi\` (MSI), \`Forge-Desktop_${meta.version}_windows_x64-portable.exe\` / \`.zip\``,
        `- **Linux**: \`Forge.Desktop_${meta.version}_amd64.AppImage\`, \`.deb\`, \`.rpm\`, \`Forge-Desktop_${meta.version}_linux_x64-portable.tar.gz\``,
        `- **macOS**: \`Forge.Desktop_${meta.version}_universal.dmg\`, \`Forge.Desktop_${meta.version}_universal.app.tar.gz\``,
        "- **Integritas**: Periksa `SHA256SUMS` (dan `SHA256SUMS.asc` jika tersedia).",
      ].join("\n")
    );
  } else {
    sections.push(`## ${meta.releaseName}`);
    sections.push(
      "Rilis stabil resmi **Forge Desktop** untuk Windows, Linux, dan macOS (installer OS maupun versi portable)."
    );
    if (cleanCustom) {
      sections.push(`### Ringkasan Rilis\n\n${cleanCustom}`);
    }
    if (cleanGenerated) {
      sections.push(`### Daftar Perubahan\n\n${cleanGenerated}`);
    }
    sections.push(
      [
        "### Instalasi & Mode Portable",
        `- **Windows**: Installer \`*-setup.exe\` / \`*.msi\`, atau portable \`Forge-Desktop_${meta.version}_windows_x64-portable.exe\` / \`.zip\``,
        `- **Linux**: Paket \`*.deb\` / \`*.rpm\`, portable \`*.AppImage\` atau \`Forge-Desktop_${meta.version}_linux_x64-portable.tar.gz\``,
        `- **macOS**: \`Forge.Desktop_${meta.version}_universal.dmg\` atau arsip \`*.app.tar.gz\``,
        "",
        "### Verifikasi Integritas",
        "Cocokkan checksum unduhan menggunakan `SHA256SUMS` (dan tanda tangan GPG `SHA256SUMS.asc` bila tersedia).",
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

function main() {
  const args = process.argv.slice(2);
  const cmd = args[0] || "meta";
  const rootDir = process.cwd();
  const tauriConf = JSON.parse(
    fs.readFileSync(path.join(rootDir, "src-tauri", "tauri.conf.json"), "utf8")
  );

  const meta = parseReleaseMeta({
    refName: process.env.RELEASE_REF_NAME || "",
    inputVersion: process.env.INPUT_VERSION || "",
    inputChannel: process.env.INPUT_CHANNEL || "auto",
    inputDraft: process.env.INPUT_DRAFT || "false",
    fallbackVersion: tauriConf.version || "1.1.0",
  });

  if (cmd === "sync") {
    syncManifests(rootDir, meta);
  } else if (cmd === "notes") {
    const generatedPath = process.env.GENERATED_NOTES_FILE || "";
    const generatedNotes =
      generatedPath && fs.existsSync(generatedPath)
        ? fs.readFileSync(generatedPath, "utf8")
        : "";
    const notesText = buildReleaseNotes({
      meta,
      customNotes: process.env.CUSTOM_NOTES || "",
      generatedNotes,
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
