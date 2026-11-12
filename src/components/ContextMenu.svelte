<script lang="ts">
  import { tick } from "svelte";
  import {
    closeContextMenu,
    getContextMenuState,
    type ContextMenuItem,
  } from "../lib/contextMenu.svelte";
  import { t } from "../i18n/index.svelte";

  const menuState = getContextMenuState();
  let menuEl = $state<HTMLDivElement | null>(null);
  let posX = $state(0);
  let posY = $state(0);

  $effect(() => {
    if (!menuState.open) return;
    posX = menuState.x;
    posY = menuState.y;

    void tick().then(() => {
      if (!menuEl || !menuState.open) return;
      const rect = menuEl.getBoundingClientRect();
      const pad = 8;
      const maxX = Math.max(pad, window.innerWidth - rect.width - pad);
      const maxY = Math.max(pad, window.innerHeight - rect.height - pad);
      posX = Math.min(Math.max(pad, menuState.x), maxX);
      posY = Math.min(Math.max(pad, menuState.y), maxY);
      const firstBtn = menuEl.querySelector<HTMLButtonElement>("button:not(:disabled)");
      firstBtn?.focus({ preventScroll: true });
    });

    const handlePointerDown = (e: PointerEvent) => {
      if (menuEl && e.target instanceof Node && menuEl.contains(e.target)) return;
      closeContextMenu();
    };

    const handleKeydown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        closeContextMenu();
        return;
      }
      if (!menuEl) return;
      const buttons = Array.from(
        menuEl.querySelectorAll<HTMLButtonElement>('button[role="menuitem"]:not(:disabled)')
      );
      if (buttons.length === 0) return;

      const idx = buttons.indexOf(document.activeElement as HTMLButtonElement);
      if (e.key === "ArrowDown") {
        e.preventDefault();
        buttons[(idx + 1 + buttons.length) % buttons.length].focus();
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        buttons[(idx - 1 + buttons.length) % buttons.length].focus();
      } else if (e.key === "Home") {
        e.preventDefault();
        buttons[0].focus();
      } else if (e.key === "End") {
        e.preventDefault();
        buttons[buttons.length - 1].focus();
      } else if (e.key === "Tab") {
        closeContextMenu();
      }
    };

    const handleScroll = (e: Event) => {
      if (menuEl && e.target instanceof Node && menuEl.contains(e.target)) return;
      closeContextMenu();
    };

    const handleWindowChange = () => {
      closeContextMenu();
    };

    window.addEventListener("pointerdown", handlePointerDown, true);
    window.addEventListener("keydown", handleKeydown, true);
    window.addEventListener("scroll", handleScroll, true);
    window.addEventListener("resize", handleWindowChange);
    window.addEventListener("blur", handleWindowChange);

    return () => {
      window.removeEventListener("pointerdown", handlePointerDown, true);
      window.removeEventListener("keydown", handleKeydown, true);
      window.removeEventListener("scroll", handleScroll, true);
      window.removeEventListener("resize", handleWindowChange);
      window.removeEventListener("blur", handleWindowChange);
    };
  });

  function triggerItem(item: Extract<ContextMenuItem, { onSelect: () => void }>) {
    if (item.disabled) return;
    closeContextMenu();
    item.onSelect();
  }
</script>

{#if menuState.open}
  <div
    bind:this={menuEl}
    class="context-menu"
    role="menu"
    tabindex="-1"
    aria-label={t("ctx.menuAria")}
    style="left: {posX}px; top: {posY}px;"
    onmousedown={(e) => e.preventDefault()}
    oncontextmenu={(e) => e.preventDefault()}
  >
    {#each menuState.items as item, i (i)}
      {#if item.type === "divider"}
        <div class="popover-divider" role="separator"></div>
      {:else}
        <button
          type="button"
          role="menuitem"
          class="popover-item {item.danger ? 'danger' : ''}"
          disabled={item.disabled}
          onclick={() => triggerItem(item)}
        >
          <span>{item.label}</span>
          {#if item.hint}
            <span class="popover-item-hint">{item.hint}</span>
          {/if}
        </button>
      {/if}
    {/each}
  </div>
{/if}
