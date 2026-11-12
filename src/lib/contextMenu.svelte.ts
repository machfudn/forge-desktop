import { t } from "../i18n/index.svelte";

export type ContextMenuItem =
  | { type: "divider" }
  | {
      type?: "item";
      label: string;
      hint?: string;
      danger?: boolean;
      disabled?: boolean;
      onSelect: () => void;
    };

export interface ContextMenuState {
  open: boolean;
  x: number;
  y: number;
  items: ContextMenuItem[];
}

const menuState = $state<ContextMenuState>({
  open: false,
  x: 0,
  y: 0,
  items: [],
});

export function getContextMenuState(): ContextMenuState {
  return menuState;
}

export function closeContextMenu(): void {
  menuState.open = false;
  menuState.items = [];
}

export function openContextMenu(event: MouseEvent, items: ContextMenuItem[]): void {
  event.preventDefault();
  event.stopPropagation();

  const cleaned = trimMenuDividers(items);
  if (cleaned.length === 0) {
    closeContextMenu();
    return;
  }

  menuState.x = event.clientX;
  menuState.y = event.clientY;
  menuState.items = cleaned;
  menuState.open = true;
}

function trimMenuDividers(items: ContextMenuItem[]): ContextMenuItem[] {
  const out: ContextMenuItem[] = [];
  for (const item of items) {
    if (item.type === "divider") {
      if (out.length === 0 || out[out.length - 1].type === "divider") continue;
      out.push(item);
    } else {
      out.push(item);
    }
  }
  while (out.length > 0 && out[out.length - 1].type === "divider") {
    out.pop();
  }
  return out;
}

export async function copyTextToClipboard(
  text: string,
  onStatus?: (msg: string, isError?: boolean) => void,
  statusMessage?: string
): Promise<boolean> {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
    } else {
      const ta = document.createElement("textarea");
      ta.value = text;
      ta.setAttribute("readonly", "");
      ta.style.position = "fixed";
      ta.style.opacity = "0";
      document.body.appendChild(ta);
      ta.select();
      document.execCommand("copy");
      document.body.removeChild(ta);
    }
    if (onStatus && statusMessage) {
      onStatus(statusMessage);
    }
    return true;
  } catch (err) {
    if (onStatus) {
      onStatus(String(err), true);
    }
    return false;
  }
}

const EDITABLE_INPUT_TYPES: Record<string, true> = {
  text: true,
  search: true,
  email: true,
  url: true,
  password: true,
  tel: true,
  number: true,
};

function getEditableTarget(target: EventTarget | null): HTMLInputElement | HTMLTextAreaElement | null {
  if (!(target instanceof HTMLElement)) return null;
  if (target instanceof HTMLTextAreaElement) {
    return target.disabled || target.readOnly ? null : target;
  }
  if (target instanceof HTMLInputElement) {
    const type = (target.type || "text").toLowerCase();
    if (!EDITABLE_INPUT_TYPES[type]) return null;
    return target.disabled || target.readOnly ? null : target;
  }
  return null;
}

export function handleGlobalContextMenu(
  event: MouseEvent,
  onStatus?: (msg: string, isError?: boolean) => void
): void {
  if (event.defaultPrevented) return;
  event.preventDefault();

  const editable = getEditableTarget(event.target);
  if (editable) {
    const start = editable.selectionStart ?? 0;
    const end = editable.selectionEnd ?? 0;
    const hasSelection = end > start;
    const hasValue = editable.value.length > 0;
    const selected = hasSelection ? editable.value.slice(start, end) : "";

    openContextMenu(event, [
      {
        label: t("ctx.edit.cut"),
        hint: "Ctrl+X",
        disabled: !hasSelection,
        onSelect: () => {
          void copyTextToClipboard(selected);
          editable.focus();
          editable.setRangeText("", start, end, "end");
          editable.dispatchEvent(new Event("input", { bubbles: true }));
        },
      },
      {
        label: t("ctx.edit.copy"),
        hint: "Ctrl+C",
        disabled: !hasSelection,
        onSelect: () => {
          void copyTextToClipboard(selected);
          editable.focus();
          editable.setSelectionRange(start, end);
        },
      },
      {
        label: t("ctx.edit.paste"),
        hint: "Ctrl+V",
        onSelect: async () => {
          try {
            const clip = await navigator.clipboard.readText();
            editable.focus();
            editable.setRangeText(clip, start, end, "end");
            editable.dispatchEvent(new Event("input", { bubbles: true }));
          } catch (err) {
            onStatus?.(String(err), true);
          }
        },
      },
      { type: "divider" },
      {
        label: t("ctx.edit.selectAll"),
        hint: "Ctrl+A",
        disabled: !hasValue,
        onSelect: () => {
          editable.focus();
          editable.select();
        },
      },
    ]);
    return;
  }

  const selectionText = window.getSelection()?.toString() || "";
  if (selectionText.trim().length > 0) {
    openContextMenu(event, [
      {
        label: t("ctx.edit.copy"),
        hint: "Ctrl+C",
        onSelect: () => {
          void copyTextToClipboard(selectionText, onStatus, t("ctx.status.copiedSelection"));
        },
      },
    ]);
    return;
  }

  closeContextMenu();
}
