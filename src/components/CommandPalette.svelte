<script lang="ts">
  import { t, type TranslationKey } from "../i18n/index.svelte";

  export type PaletteCategory =
    | "Git"
    | "File"
    | "Branch"
    | "Commit"
    | "Repo"
    | "View"
    | "Tools"
    | "AI"
    | "Theme"
    | "Settings";

  export interface PaletteCommand {
    id: string;
    label: string;
    category: PaletteCategory;
    description?: string;
    keywords?: string[];
    shortcut?: string;
    badge?: string;
    danger?: boolean;
    action: () => void | Promise<void>;
  }

  interface Props {
    commands: PaletteCommand[];
    onClose: () => void;
  }

  type FilterTab = "all" | "commands" | "files" | "branches" | "commits" | "repos" | "themes" | "ai";

  interface TextSegment {
    text: string;
    match: boolean;
  }

  interface ScoredCommand {
    cmd: PaletteCommand;
    score: number;
    labelSegments: TextSegment[];
    descSegments: TextSegment[];
  }

  let { commands, onClose }: Props = $props();

  let query = $state("");
  let selectedFilter = $state<FilterTab>("all");
  let activeIndex = $state(0);
  let inputEl = $state<HTMLInputElement | null>(null);
  let lastPointer = { x: -1, y: -1 };

  function handlePointerMove(e: PointerEvent, idx: number) {
    if (e.clientX === lastPointer.x && e.clientY === lastPointer.y) return;
    lastPointer = { x: e.clientX, y: e.clientY };
    activeIndex = idx;
  }

  function scrollIfActive(node: HTMLElement, isActive: boolean) {
    if (isActive) {
      node.scrollIntoView({ block: "nearest" });
    }
    return {
      update(active: boolean) {
        if (active) {
          node.scrollIntoView({ block: "nearest" });
        }
      },
    };
  }

  const filterTabs: Array<{ id: FilterTab; labelKey: TranslationKey; prefix?: string }> = [
    { id: "all", labelKey: "palette.filterAll" },
    { id: "commands", labelKey: "palette.filterCommands", prefix: ">" },
    { id: "files", labelKey: "palette.filterFiles", prefix: "#" },
    { id: "branches", labelKey: "palette.filterBranches", prefix: "@" },
    { id: "commits", labelKey: "palette.filterCommits", prefix: ":" },
    { id: "repos", labelKey: "palette.filterRepos", prefix: "~" },
    { id: "themes", labelKey: "palette.filterThemes", prefix: "$" },
    { id: "ai", labelKey: "palette.filterAi" },
  ];

  function matchesCategory(category: PaletteCategory, filter: FilterTab): boolean {
    switch (filter) {
      case "all":
        return true;
      case "commands":
        return category === "Git" || category === "Tools" || category === "View" || category === "Settings";
      case "files":
        return category === "File";
      case "branches":
        return category === "Branch";
      case "commits":
        return category === "Commit";
      case "repos":
        return category === "Repo";
      case "themes":
        return category === "Theme";
      case "ai":
        return category === "AI";
      default:
        return true;
    }
  }

  // Count items per category tab
  const tabCounts = $derived.by<Record<FilterTab, number>>(() => {
    const counts: Record<FilterTab, number> = {
      all: commands.length,
      commands: 0,
      files: 0,
      branches: 0,
      commits: 0,
      repos: 0,
      themes: 0,
      ai: 0,
    };
    for (const cmd of commands) {
      if (cmd.category === "Git" || cmd.category === "Tools" || cmd.category === "View" || cmd.category === "Settings") {
        counts.commands++;
      } else if (cmd.category === "File") {
        counts.files++;
      } else if (cmd.category === "Branch") {
        counts.branches++;
      } else if (cmd.category === "Commit") {
        counts.commits++;
      } else if (cmd.category === "Repo") {
        counts.repos++;
      } else if (cmd.category === "Theme") {
        counts.themes++;
      } else if (cmd.category === "AI") {
        counts.ai++;
      }
    }
    return counts;
  });

  // Parse prefix if user typed leading prefix character
  const parsedSearch = $derived.by<{ effectiveFilter: FilterTab; cleanQuery: string }>(() => {
    const raw = query.trim();
    if (raw.startsWith(">")) {
      return { effectiveFilter: "commands", cleanQuery: raw.slice(1).trim().toLowerCase() };
    }
    if (raw.startsWith("#")) {
      return { effectiveFilter: "files", cleanQuery: raw.slice(1).trim().toLowerCase() };
    }
    if (raw.startsWith("@")) {
      return { effectiveFilter: "branches", cleanQuery: raw.slice(1).trim().toLowerCase() };
    }
    if (raw.startsWith(":")) {
      return { effectiveFilter: "commits", cleanQuery: raw.slice(1).trim().toLowerCase() };
    }
    if (raw.startsWith("~")) {
      return { effectiveFilter: "repos", cleanQuery: raw.slice(1).trim().toLowerCase() };
    }
    if (raw.startsWith("$")) {
      return { effectiveFilter: "themes", cleanQuery: raw.slice(1).trim().toLowerCase() };
    }
    return { effectiveFilter: selectedFilter, cleanQuery: raw.toLowerCase() };
  });

  function splitHighlight(text: string, tokens: string[]): TextSegment[] {
    if (!tokens.length || !text) return [{ text, match: false }];
    const lower = text.toLowerCase();
    const intervals: Array<{ start: number; end: number }> = [];

    for (const tok of tokens) {
      if (!tok) continue;
      let startIdx = 0;
      while (startIdx < lower.length) {
        const idx = lower.indexOf(tok, startIdx);
        if (idx === -1) break;
        intervals.push({ start: idx, end: idx + tok.length });
        startIdx = idx + tok.length;
      }
    }

    if (intervals.length === 0) return [{ text, match: false }];

    // Merge overlapping intervals
    intervals.sort((a, b) => a.start - b.start);
    const merged: Array<{ start: number; end: number }> = [intervals[0]];
    for (let i = 1; i < intervals.length; i++) {
      const last = merged[merged.length - 1];
      const cur = intervals[i];
      if (cur.start <= last.end) {
        last.end = Math.max(last.end, cur.end);
      } else {
        merged.push(cur);
      }
    }

    const segments: TextSegment[] = [];
    let cursor = 0;
    for (const m of merged) {
      if (m.start > cursor) {
        segments.push({ text: text.slice(cursor, m.start), match: false });
      }
      segments.push({ text: text.slice(m.start, m.end), match: true });
      cursor = m.end;
    }
    if (cursor < text.length) {
      segments.push({ text: text.slice(cursor), match: false });
    }
    return segments;
  }

  const filtered = $derived.by<ScoredCommand[]>(() => {
    const { effectiveFilter, cleanQuery } = parsedSearch;
    const tokens = cleanQuery.split(/\s+/).filter(Boolean);

    const candidates = commands.filter((cmd) => matchesCategory(cmd.category, effectiveFilter));

    if (tokens.length === 0) {
      return candidates.slice(0, 50).map((cmd) => ({
        cmd,
        score: 0,
        labelSegments: [{ text: cmd.label, match: false }],
        descSegments: cmd.description ? [{ text: cmd.description, match: false }] : [],
      }));
    }

    const scored: ScoredCommand[] = [];

    for (const cmd of candidates) {
      const labelLower = cmd.label.toLowerCase();
      const descLower = (cmd.description || "").toLowerCase();
      const catLower = cmd.category.toLowerCase();
      const kwLower = (cmd.keywords || []).map((k) => k.toLowerCase()).join(" ");

      // Check whether every token matches at least somewhere in the command
      const allTokensMatch = tokens.every(
        (tok) =>
          labelLower.includes(tok) ||
          descLower.includes(tok) ||
          catLower.includes(tok) ||
          kwLower.includes(tok)
      );

      if (!allTokensMatch) continue;

      let score = 0;
      if (labelLower === cleanQuery) {
        score += 150;
      } else if (labelLower.startsWith(cleanQuery)) {
        score += 80;
      } else if (labelLower.includes(cleanQuery)) {
        score += 40;
      }

      for (const tok of tokens) {
        if (labelLower.startsWith(tok)) score += 25;
        else if (labelLower.includes(tok)) score += 15;
        if (descLower.includes(tok)) score += 10;
        if (kwLower.includes(tok)) score += 15;
        if (catLower.includes(tok)) score += 5;
      }

      // Bonus for current active entities
      if (cmd.badge === "Aktif" || cmd.badge === "Active") score += 10;

      scored.push({
        cmd,
        score,
        labelSegments: splitHighlight(cmd.label, tokens),
        descSegments: cmd.description ? splitHighlight(cmd.description, tokens) : [],
      });
    }

    scored.sort((a, b) => b.score - a.score);
    return scored.slice(0, 50);
  });

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

  function cycleFilter(direction: 1 | -1) {
    const curIdx = filterTabs.findIndex((t) => t.id === selectedFilter);
    const nextIdx = (curIdx + direction + filterTabs.length) % filterTabs.length;
    selectedFilter = filterTabs[nextIdx].id;
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
    } else if (e.key === "PageDown") {
      e.preventDefault();
      if (filtered.length > 0) activeIndex = Math.min(activeIndex + 6, filtered.length - 1);
    } else if (e.key === "PageUp") {
      e.preventDefault();
      if (filtered.length > 0) activeIndex = Math.max(activeIndex - 6, 0);
    } else if (e.key === "Home") {
      e.preventDefault();
      activeIndex = 0;
    } else if (e.key === "End") {
      e.preventDefault();
      if (filtered.length > 0) activeIndex = filtered.length - 1;
    } else if (e.key === "Tab") {
      e.preventDefault();
      cycleFilter(e.shiftKey ? -1 : 1);
    } else if (e.key === "Enter" && filtered[activeIndex]) {
      e.preventDefault();
      runItem(filtered[activeIndex].cmd);
    }
  }

  function getCategoryClass(category: PaletteCategory): string {
    switch (category) {
      case "File":
        return "palette-cat-file";
      case "Branch":
        return "palette-cat-branch";
      case "Commit":
        return "palette-cat-commit";
      case "Repo":
        return "palette-cat-repo";
      case "Theme":
        return "palette-cat-theme";
      case "AI":
        return "palette-cat-ai";
      case "Settings":
        return "palette-cat-settings";
      default:
        return "palette-cat-git";
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
      <svg class="palette-search-icon" width="16" height="16" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
        <path d="M11.742 10.344a6.5 6.5 0 1 0-1.397 1.398h-.001c.03.04.062.078.098.115l3.85 3.85a1 1 0 0 0 1.415-1.414l-3.85-3.85a1.007 1.007 0 0 0-.115-.1zM12 6.5a5.5 5.5 0 1 1-11 0 5.5 5.5 0 0 1 11 0z"/>
      </svg>
      <input
        bind:this={inputEl}
        type="text"
        class="palette-input"
        bind:value={query}
        placeholder={t("palette.placeholder")}
        onkeydown={handleKeydown}
        autocomplete="off"
        spellcheck="false"
      />
      {#if query}
        <button
          type="button"
          class="palette-clear-btn"
          onclick={() => { query = ""; inputEl?.focus(); }}
          aria-label={t("common.clear")}
        >
          ✕
        </button>
      {/if}
      <span class="kbd-hint">Esc</span>
    </div>

    <!-- Category Filter Chips -->
    <div class="palette-filters" role="tablist" aria-label="Palette Categories">
      {#each filterTabs as tab}
        {@const count = tabCounts[tab.id]}
        {#if count > 0 || tab.id === "all"}
          <button
            type="button"
            role="tab"
            aria-selected={parsedSearch.effectiveFilter === tab.id}
            class="palette-filter-chip {parsedSearch.effectiveFilter === tab.id ? 'active' : ''}"
            onclick={() => {
              selectedFilter = tab.id;
              inputEl?.focus();
            }}
          >
            {#if tab.prefix}
              <span class="palette-filter-prefix">{tab.prefix}</span>
            {/if}
            <span class="palette-filter-text">{t(tab.labelKey)}</span>
            <span class="palette-filter-count">{count}</span>
          </button>
        {/if}
      {/each}
    </div>

    <!-- Result List -->
    <div class="palette-list" role="listbox">
      {#if filtered.length === 0}
        <div class="palette-empty-container">
          <div class="palette-empty-text">{t("palette.noMatch")}</div>
          <div class="palette-cheatsheet">
            <span class="cheatsheet-title">{t("palette.prefixHelp")}</span>
          </div>
        </div>
      {:else}
        {#each filtered as item, idx (item.cmd.id)}
          <button
            type="button"
            role="option"
            aria-selected={idx === activeIndex}
            class="palette-item {idx === activeIndex ? 'active' : ''} {item.cmd.danger ? 'danger' : ''}"
            use:scrollIfActive={idx === activeIndex}
            onpointermove={(e) => handlePointerMove(e, idx)}
            onclick={() => runItem(item.cmd)}
          >
            <div class="palette-item-left">
              {#if idx === activeIndex}
                <span class="palette-active-pointer" aria-hidden="true">›</span>
              {/if}
              <span class="palette-cat {getCategoryClass(item.cmd.category)}">{item.cmd.category}</span>
            </div>

            <div class="palette-item-center">
              <div class="palette-item-title-row">
                <span class="palette-label">
                  {#each item.labelSegments as seg}
                    {#if seg.match}
                      <mark class="palette-match">{seg.text}</mark>
                    {:else}
                      <span>{seg.text}</span>
                    {/if}
                  {/each}
                </span>

                {#if item.cmd.badge}
                  <span class="palette-badge {item.cmd.danger ? 'badge-danger' : ''}">{item.cmd.badge}</span>
                {/if}
              </div>

              {#if item.cmd.description}
                <div class="palette-desc">
                  {#each item.descSegments as seg}
                    {#if seg.match}
                      <mark class="palette-match">{seg.text}</mark>
                    {:else}
                      <span>{seg.text}</span>
                    {/if}
                  {/each}
                </div>
              {/if}
            </div>

            <div class="palette-item-right">
              {#if idx === activeIndex}
                <span class="palette-action-hint is-active">{t("palette.actionRun")}</span>
                {#if item.cmd.shortcut}
                  <span class="kbd-hint">{item.cmd.shortcut}</span>
                {/if}
              {:else if item.cmd.shortcut}
                <span class="kbd-hint">{item.cmd.shortcut}</span>
              {:else}
                <span class="palette-action-hint">↵</span>
              {/if}
            </div>
          </button>
        {/each}
      {/if}
    </div>

    <!-- Palette Footer -->
    <div class="palette-footer">
      <span class="palette-results-count">
        {t("palette.resultsCount", { count: filtered.length })}
      </span>
      <span class="palette-footer-hints">
        {t("palette.footerHints")}
      </span>
    </div>
  </div>
</div>
