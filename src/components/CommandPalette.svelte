<script lang="ts">
  import { t } from "../i18n/index.svelte";

  export interface PaletteCommand {
    id: string;
    label: string;
    category: string;
    shortcut?: string;
    action: () => void | Promise<void>;
  }

  interface Props {
    commands: PaletteCommand[];
    onClose: () => void;
  }

  let { commands, onClose }: Props = $props();

  let query = $state("");
  let activeIndex = $state(0);
  let inputEl = $state<HTMLInputElement | null>(null);

  let filtered = $derived(
    (() => {
      const q = query.trim().toLowerCase();
      if (!q) return commands.slice(0, 30);
      return commands
        .filter(
          (c) =>
            c.label.toLowerCase().includes(q) ||
            c.category.toLowerCase().includes(q)
        )
        .slice(0, 30);
    })()
  );

  $effect(() => {
    void filtered;
    activeIndex = 0;
  });

  $effect(() => {
    queueMicrotask(() => inputEl?.focus());
  });

  function runItem(cmd: PaletteCommand) {
    onClose();
    void cmd.action();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      if (filtered.length > 0) activeIndex = (activeIndex + 1) % filtered.length;
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (filtered.length > 0) activeIndex = (activeIndex - 1 + filtered.length) % filtered.length;
    } else if (e.key === "Enter" && filtered[activeIndex]) {
      e.preventDefault();
      runItem(filtered[activeIndex]);
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="modal-overlay palette-overlay"
  onclick={(e) => {
    if (e.target === e.currentTarget) onClose();
  }}
  onkeydown={handleKeydown}
>
  <div class="palette-dialog" role="dialog" aria-modal="true" aria-label={t("palette.title")}>
    <div class="palette-search-row">
      <input
        bind:this={inputEl}
        type="text"
        class="palette-input"
        bind:value={query}
        placeholder={t("palette.placeholder")}
      />
      <span class="kbd-hint">Esc</span>
    </div>
    <div class="palette-list" role="listbox">
      {#if filtered.length === 0}
        <div class="palette-empty">{t("palette.noMatch")}</div>
      {:else}
        {#each filtered as item, idx (item.id)}
          <button
            type="button"
            role="option"
            aria-selected={idx === activeIndex}
            class="palette-item {idx === activeIndex ? 'active' : ''}"
            onmouseenter={() => (activeIndex = idx)}
            onclick={() => runItem(item)}
          >
            <span class="palette-cat">{item.category}</span>
            <span class="palette-label">{item.label}</span>
            {#if item.shortcut}
              <span class="kbd-hint">{item.shortcut}</span>
            {/if}
          </button>
        {/each}
      {/if}
    </div>
  </div>
</div>
