<script lang="ts">
  import type { FontTableInfo } from "../lib/fontTables";
  import { showTitlebarContextMenu } from "../lib/titlebarContextMenu";
  import WindowControls from "./WindowControls.svelte";

  interface Props {
    loading: boolean;
    fontPath: string | null;
    errorMsg: string | null;
    selectedTable: FontTableInfo | null;
    showSidebar: boolean;
    showHexPane: boolean;
    theme: string;
    onOpenFont: () => void;
    onToggleTheme: () => void;
  }

  let {
    loading,
    fontPath,
    errorMsg,
    selectedTable,
    showSidebar = $bindable(),
    showHexPane = $bindable(),
    theme,
    onOpenFont,
    onToggleTheme,
  }: Props = $props();
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<header class="toolbar" oncontextmenu={showTitlebarContextMenu}>
  <div class="toolbar-left">
    <img src="/logo.png" alt="Fontabex" width="24" height="24" />
    <button
      class="button button-outline open-font-btn"
      type="button"
      onclick={onOpenFont}
      disabled={loading}
    >
      {loading ? "Opening..." : "Open Font"}
    </button>
    {#if fontPath}
      <span class="toolbar-font-path" title={fontPath} data-tauri-drag-region>
        {fontPath.split(/[\\/]/).pop()}
      </span>
    {/if}
  </div>

  <div class="toolbar-drag-region" data-tauri-drag-region></div>

  <div class="toolbar-actions">
    {#if errorMsg}
      <span class="error-message">{errorMsg}</span>
    {/if}
    <button
      class="button button-outline button-icon {showSidebar ? 'active' : ''}"
      type="button"
      onclick={() => (showSidebar = !showSidebar)}
      title="Toggle Sidebar"
    >
      <svg
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <rect width="18" height="18" x="3" y="3" rx="2"></rect>
        <path d="M9 4v16"></path>
      </svg>
    </button>
    <button
      class="button button-outline button-icon {showHexPane ? 'active' : ''}"
      type="button"
      onclick={() => (showHexPane = !showHexPane)}
      disabled={!selectedTable}
      title="Toggle Hex Pane"
    >
      <svg
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <rect width="18" height="18" x="3" y="3" rx="2"></rect>
        <path d="M15 3v18"></path>
      </svg>
    </button>
    <button
      class="button button-outline button-icon"
      type="button"
      onclick={onToggleTheme}
      title="Toggle Theme"
    >
      {#if theme === "dark"}
        <svg
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <circle cx="12" cy="12" r="4" />
          <path d="M12 2v2" />
          <path d="M12 20v2" />
          <path d="m4.93 4.93 1.41 1.41" />
          <path d="m17.66 17.66 1.41 1.41" />
          <path d="M2 12h2" />
          <path d="M20 12h2" />
          <path d="m6.34 17.66-1.41 1.41" />
          <path d="m19.07 4.93-1.41 1.41" />
        </svg>
      {:else}
        <svg
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z" />
        </svg>
      {/if}
    </button>
  </div>

  <WindowControls />
</header>

<style>
  .toolbar {
    height: var(--toolbar-height);
    background: var(--sidebar-bg);
    backdrop-filter: blur(12px);
    border-bottom: 1px solid var(--border-color);
    display: flex;
    gap: 16px;
    align-items: center;
    justify-content: space-between;
    padding: 0 0 0 0.75rem;
    z-index: 10;
    user-select: none;
    cursor: default;
  }

  .toolbar-left {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
  }

  .toolbar-font-path {
    font-size: 0.825rem;
    font-weight: 600;
    color: var(--code-value);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 500px;
    cursor: default;
  }

  .toolbar-drag-region {
    align-self: stretch;
    flex: 1 1 auto;
    min-width: 1rem;
    cursor: default;
  }

  .toolbar-actions {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    flex-shrink: 0;
  }

  .error-message {
    color: #ef4444;
    font-size: 0.875rem;
    max-width: min(32vw, 420px);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .open-font-btn {
    min-width: max-content;
  }
</style>
