// ANSI Escape Sequence Tokenizer
// Parses SGR color and style codes into styled spans with design token CSS classes.
// ponytail: Standard 16-color ANSI + basic styling. Add 256-color palette when needed.

export interface AnsiSpan {
  text: string;
  classes?: string;
}

interface AnsiStyleState {
  bold: boolean;
  dim: boolean;

  italic: boolean;
  underline: boolean;
  color?: string;
}

export function stripAnsi(input: string): string {
  if (!input) return "";
  return input.replace(/\x1b\[[0-9;?]*[a-zA-Z]|\x1b\([a-zA-Z]/g, "");
}

export function parseAnsiText(input: string): AnsiSpan[] {
  if (!input) return [];
  const spans: AnsiSpan[] = [];

  // Match ANSI escape sequences: SGR (\x1b[...m) or non-SGR sequences to strip (\x1b[...[a-zA-Z])
  const ansiRegex = /\x1b\[([0-9;]*)m|\x1b\[[0-9;?]*[a-zA-Z]|\x1b\([a-zA-Z]/g;
  const state: AnsiStyleState = {
    bold: false,
    dim: false,
    italic: false,
    underline: false,
    color: undefined,
  };

  let lastIndex = 0;
  let match: RegExpExecArray | null;

  function getCurrentClasses(): string | undefined {
    const list: string[] = [];
    if (state.bold) list.push("ansi-bold");
    if (state.dim) list.push("ansi-dim");
    if (state.italic) list.push("ansi-italic");
    if (state.underline) list.push("ansi-underline");
    if (state.color) list.push(`ansi-${state.color}`);
    return list.length > 0 ? list.join(" ") : undefined;
  }

  function applySgrCodes(codeStr: string) {
    if (!codeStr || codeStr === "" || codeStr === "0") {
      state.bold = false;
      state.dim = false;
      state.italic = false;
      state.underline = false;
      state.color = undefined;
      return;
    }

    const codes = codeStr.split(";").map((c) => parseInt(c, 10));
    for (let i = 0; i < codes.length; i++) {
      const code = isNaN(codes[i]) ? 0 : codes[i];
      if (code === 0) {
        state.bold = false;
        state.dim = false;
        state.italic = false;
        state.underline = false;
        state.color = undefined;
      } else if (code === 1) {
        state.bold = true;
      } else if (code === 2) {
        state.dim = true;
      } else if (code === 3) {
        state.italic = true;
      } else if (code === 4) {
        state.underline = true;
      } else if (code === 22) {
        state.bold = false;
        state.dim = false;
      } else if (code === 23) {
        state.italic = false;
      } else if (code === 24) {
        state.underline = false;
      } else if (code === 30 || code === 90) {
        state.color = "dim";
      } else if (code === 31 || code === 91) {
        state.color = "red";
      } else if (code === 32 || code === 92) {
        state.color = "green";
      } else if (code === 33 || code === 93) {
        state.color = "yellow";
      } else if (code === 34 || code === 94) {
        state.color = "blue";
      } else if (code === 35 || code === 95) {
        state.color = "magenta";
      } else if (code === 36 || code === 96) {
        state.color = "cyan";
      } else if (code === 37 || code === 97) {
        state.color = undefined;
      } else if (code === 39) {
        state.color = undefined;
      }
    }
  }

  while ((match = ansiRegex.exec(input)) !== null) {
    if (match.index > lastIndex) {
      const text = input.slice(lastIndex, match.index);
      if (text) {
        spans.push({
          text,
          classes: getCurrentClasses(),
        });
      }
    }
    lastIndex = ansiRegex.lastIndex;

    // If it was an SGR sequence (match[1] is defined)
    if (match[1] !== undefined) {
      applySgrCodes(match[1]);
    }
  }

  if (lastIndex < input.length) {
    const text = input.slice(lastIndex);
    if (text) {
      spans.push({
        text,
        classes: getCurrentClasses(),
      });
    }
  }

  return spans;
}
