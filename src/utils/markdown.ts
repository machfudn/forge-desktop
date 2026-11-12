function escapeHtml(raw: string): string {
  return raw
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function safeHttpUrl(raw: string): string | null {
  const trimmed = raw.trim();
  if (/^https?:\/\/[^\s<>"']+$/i.test(trimmed)) {
    return trimmed;
  }
  return null;
}

function renderInlineMarkdown(raw: string): string {
  const tokenRegex = /(`[^`]+`)|(\[([^\]]+)\]\(([^)]+)\))|(\*\*[^*]+\*\*|__[^_]+__)|(\*[^*\n]+\*)|(~~[^~]+~~)/g;
  let out = "";
  let last = 0;
  let match: RegExpExecArray | null;

  while ((match = tokenRegex.exec(raw)) !== null) {
    if (match.index > last) {
      out += escapeHtml(raw.slice(last, match.index));
    }
    if (match[1]) {
      out += `<code>${escapeHtml(match[1].slice(1, -1))}</code>`;
    } else if (match[2]) {
      const label = match[3] ?? "";
      const url = safeHttpUrl(match[4] ?? "");
      if (url) {
        out += `<a href="${escapeHtml(url)}" target="_blank" rel="noopener noreferrer">${escapeHtml(label)}</a>`;
      } else {
        out += escapeHtml(match[2]);
      }
    } else if (match[5]) {
      out += `<strong>${escapeHtml(match[5].slice(2, -2))}</strong>`;
    } else if (match[6]) {
      out += `<em>${escapeHtml(match[6].slice(1, -1))}</em>`;
    } else if (match[7]) {
      out += `<del>${escapeHtml(match[7].slice(2, -2))}</del>`;
    }
    last = match.index + match[0].length;
  }

  if (last < raw.length) {
    out += escapeHtml(raw.slice(last));
  }
  return out;
}

export function renderMarkdownHtml(markdown: string): string {
  const lines = (markdown || "").replace(/\r\n?/g, "\n").split("\n");
  const blocks: string[] = [];
  let inCode = false;
  let codeLines: string[] = [];
  let listTag: "ul" | "ol" | null = null;
  let listItems: string[] = [];
  let paraLines: string[] = [];

  const flushPara = () => {
    if (paraLines.length === 0) return;
    blocks.push(`<p>${paraLines.map((l) => renderInlineMarkdown(l)).join("<br />")}</p>`);
    paraLines = [];
  };

  const flushList = () => {
    if (!listTag || listItems.length === 0) return;
    blocks.push(`<${listTag}>${listItems.join("")}</${listTag}>`);
    listTag = null;
    listItems = [];
  };

  const flushCode = () => {
    if (!inCode) return;
    blocks.push(`<pre><code>${escapeHtml(codeLines.join("\n"))}</code></pre>`);
    inCode = false;
    codeLines = [];
  };

  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.startsWith("```")) {
      flushPara();
      flushList();
      if (inCode) {
        flushCode();
      } else {
        inCode = true;
        codeLines = [];
      }
      continue;
    }

    if (inCode) {
      codeLines.push(line);
      continue;
    }

    if (!trimmed) {
      flushPara();
      flushList();
      continue;
    }

    if (/^(-{3,}|\*{3,}|_{3,})$/.test(trimmed)) {
      flushPara();
      flushList();
      blocks.push("<hr />");
      continue;
    }

    const headingMatch = /^(#{1,4})\s+(.+)$/.exec(trimmed);
    if (headingMatch) {
      flushPara();
      flushList();
      const level = headingMatch[1].length;
      blocks.push(`<h${level}>${renderInlineMarkdown(headingMatch[2])}</h${level}>`);
      continue;
    }

    if (trimmed.startsWith("> ")) {
      flushPara();
      flushList();
      blocks.push(`<blockquote>${renderInlineMarkdown(trimmed.slice(2))}</blockquote>`);
      continue;
    }

    const ulMatch = /^(?:[-*+])\s+(.+)$/.exec(trimmed);
    if (ulMatch) {
      flushPara();
      if (listTag !== "ul") {
        flushList();
        listTag = "ul";
      }
      const itemText = ulMatch[1];
      if (itemText.startsWith("[x] ") || itemText.startsWith("[X] ")) {
        listItems.push(
          `<li class="md-task"><input type="checkbox" checked disabled /> <span>${renderInlineMarkdown(itemText.slice(4))}</span></li>`
        );
      } else if (itemText.startsWith("[ ] ")) {
        listItems.push(
          `<li class="md-task"><input type="checkbox" disabled /> <span>${renderInlineMarkdown(itemText.slice(4))}</span></li>`
        );
      } else {
        listItems.push(`<li>${renderInlineMarkdown(itemText)}</li>`);
      }
      continue;
    }

    const olMatch = /^\d+\.\s+(.+)$/.exec(trimmed);
    if (olMatch) {
      flushPara();
      if (listTag !== "ol") {
        flushList();
        listTag = "ol";
      }
      listItems.push(`<li>${renderInlineMarkdown(olMatch[1])}</li>`);
      continue;
    }

    flushList();
    paraLines.push(line);
  }

  flushCode();
  flushPara();
  flushList();

  return blocks.join("");
}
