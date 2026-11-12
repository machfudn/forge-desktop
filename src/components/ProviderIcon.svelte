<script lang="ts">
  import type { HostingProvider } from "../types/git";

  interface Props {
    provider?: HostingProvider | "local" | "missing" | null;
    size?: number;
  }

  let { provider = "local", size = 15 }: Props = $props();
</script>

{#if provider === "github"}
  <img
    src="/icon.svg"
    alt=""
    width={size}
    height={size}
    style="width: {size}px; height: {size}px; border-radius: 3px; flex-shrink: 0; display: block;"
  />
{:else if provider === "gitlab"}
  <svg width={size} height={size} viewBox="0 0 16 16" fill="currentColor" aria-hidden="true" style="color: #fc6d26;">
    <path d="m15.565 9.156-.872-2.685-1.727-5.318a.446.446 0 0 0-.848 0l-1.728 5.318H5.61L3.882 1.153a.446.446 0 0 0-.848 0L1.307 6.471.435 9.156a.89.89 0 0 0 .323.996L8 15.414l7.242-5.262a.89.89 0 0 0 .323-.996Z" />
  </svg>
{:else if provider === "bitbucket"}
  <svg width={size} height={size} viewBox="0 0 16 16" fill="currentColor" aria-hidden="true" style="color: #2684ff;">
    <path d="M1.12 1.5A.5.5 0 0 0 .627 2.08l1.97 11.98a.68.68 0 0 0 .67.57h9.467a.5.5 0 0 0 .493-.418l1.972-12.132a.5.5 0 0 0-.493-.58H1.12Zm8.46 8.65H6.42L5.64 6.1h4.72l-.78 4.05Z" />
  </svg>
{:else if provider === "azure_devops"}
  <svg width={size} height={size} viewBox="0 0 16 16" fill="currentColor" aria-hidden="true" style="color: #0078d4;">
    <path d="M14.5 3.1v9.5l-3.4 2.8-5.3-1.9v1.9L2.5 11.3l9.2.7V3.6L14.5 3.1Zm-3.3.6L6.1 1.4v2.1L2.2 4.6 1 6.2v4.3l1.5.7V6.6l8.7-2.9Z" />
  </svg>
{:else if provider === "missing"}
  <svg width={size} height={size} viewBox="0 0 16 16" fill="currentColor" aria-hidden="true" style="color: var(--diff-del-text);">
    <path d="M6.457 1.047c.659-1.234 2.427-1.234 3.086 0l6.082 11.378A1.75 1.75 0 0 1 14.082 15H1.918a1.75 1.75 0 0 1-1.543-2.575Zm1.763.707a.25.25 0 0 0-.44 0L1.698 13.132a.25.25 0 0 0 .22.368h12.164a.25.25 0 0 0 .22-.368Zm.53 3.996v2.5a.75.75 0 0 1-1.5 0v-2.5a.75.75 0 0 1 1.5 0ZM9 11a1 1 0 1 1-2 0 1 1 0 0 1 2 0Z" />
  </svg>
{:else}
  <svg width={size} height={size} viewBox="0 0 16 16" fill="currentColor" aria-hidden="true" style="color: var(--text-secondary);">
    <path d="M2 2.5A2.5 2.5 0 0 1 4.5 0h8.75a.75.75 0 0 1 .75.75v12.5a.75.75 0 0 1-.75.75h-2.5a.75.75 0 0 1 0-1.5h1.75v-2h-8a1 1 0 0 0-.714 1.7.75.75 0 1 1-1.072 1.05A2.495 2.495 0 0 1 2 11.5Zm10.5-1h-8a1 1 0 0 0-1 1v6.708A2.486 2.486 0 0 1 4.5 9h8ZM5 12.25a.25.25 0 0 1 .25-.25h3.5a.25.25 0 0 1 .25.25v3.25a.25.25 0 0 1-.4.2l-1.45-1.087a.249.249 0 0 0-.3 0L5.4 15.7a.25.25 0 0 1-.4-.2Z" />
  </svg>
{/if}
