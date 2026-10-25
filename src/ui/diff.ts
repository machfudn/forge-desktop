export function renderDiff(container: HTMLElement, filePath: string, diffText: string): void {
  if (!diffText.trim()) {
    container.innerHTML = `
      <div class="diff-container">
        <div class="diff-header">${escapeHtml(filePath)}</div>
        <div class="diff-content" style="padding: 24px; color: var(--text-muted); text-align: center;">
          Tidak ada perbedaan teks untuk ditampilkan.
        </div>
      </div>
    `;
    return;
  }

  const lines = diffText.split("\n");
  const renderedLines = lines
    .map((line) => {
      let className = "diff-line";
      if (line.startsWith("+") && !line.startsWith("+++")) {
        className += " add";
      } else if (line.startsWith("-") && !line.startsWith("---")) {
        className += " del";
      } else if (line.startsWith("@")) {
        className += " meta";
      }
      return `<div class="${className}">${escapeHtml(line)}</div>`;
    })
    .join("");

  container.innerHTML = `
    <div class="diff-container">
      <div class="diff-header">${escapeHtml(filePath)}</div>
      <div class="diff-content">${renderedLines}</div>
    </div>
  `;
}

function escapeHtml(text: string): string {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
}
