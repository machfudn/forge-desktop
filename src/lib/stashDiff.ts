export interface StashFileItem {
  path: string;
  status: "modified" | "added" | "deleted";
  additions: number;
  deletions: number;
  diffText: string;
}

export function splitMultiFileDiff(raw: string): StashFileItem[] {
  if (!raw || !raw.trim()) return [];
  const lines = raw.split(/\r?\n/);
  const files: StashFileItem[] = [];
  let currentFile: (StashFileItem & { lines?: string[] }) | null = null;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (line.startsWith("diff --git ")) {
      if (currentFile && currentFile.lines) {
        currentFile.diffText = currentFile.lines.join("\n");
        delete currentFile.lines;
        files.push(currentFile as StashFileItem);
      }
      const match = line.match(/^diff --git a\/(.+?) b\/(.+)$/);
      const filePath = match ? match[2] : line.slice(11).trim();
      currentFile = {
        path: filePath,
        status: "modified",
        additions: 0,
        deletions: 0,
        diffText: "",
        lines: [line],
      };
    } else if (currentFile && currentFile.lines) {
      currentFile.lines.push(line);
      if (line.startsWith("new file mode")) {
        currentFile.status = "added";
      } else if (line.startsWith("deleted file mode")) {
        currentFile.status = "deleted";
      } else if (line.startsWith("+") && !line.startsWith("+++")) {
        currentFile.additions++;
      } else if (line.startsWith("-") && !line.startsWith("---")) {
        currentFile.deletions++;
      }
    }
  }

  if (currentFile && currentFile.lines) {
    currentFile.diffText = currentFile.lines.join("\n");
    delete currentFile.lines;
    files.push(currentFile as StashFileItem);
  }

  return files;
}
