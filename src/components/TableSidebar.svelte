<script lang="ts">
  import {
    formatBytes,
    TABLE_DESCRIPTIONS,
    type FontInfo,
    type FontTableInfo,
  } from "../lib/fontTables";

  interface Props {
    fontInfo: FontInfo;
    selectedTable: FontTableInfo | null;
    sidebarSearch: string;
    onSelectTable: (table: FontTableInfo) => void;
  }

  let {
    fontInfo,
    selectedTable,
    sidebarSearch = $bindable(),
    onSelectTable,
  }: Props = $props();

  let filteredTables = $derived.by(() => {
    const search = sidebarSearch.trim().toLowerCase();
    if (!search) return fontInfo.tables;
    return fontInfo.tables.filter((table) => {
      const tag = table.tag.toLowerCase();
      const desc = (
        TABLE_DESCRIPTIONS[table.tag.trim()] || "Unknown Table"
      ).toLowerCase();
      return tag.includes(search) || desc.includes(search);
    });
  });

  let tableDirectory = $derived<FontTableInfo>({
    tag: "Table Directory",
    offset: 0,
    length: 12 + fontInfo.num_tables * 16,
  });
</script>

<aside class="sidebar">
  <div
    class="table-list-item sidebar-dir-row {selectedTable?.tag ===
    'Table Directory'
      ? 'active'
      : ''}"
    onclick={() => onSelectTable(tableDirectory)}
    role="button"
    tabindex="0"
    onkeydown={(event) =>
      event.key === "Enter" && onSelectTable(tableDirectory)}
  >
    <div class="item-info">
      <span class="item-title">Table Directory</span>
      <span class="item-size">{formatBytes(tableDirectory.length)}</span>
    </div>
  </div>

  <div class="sidebar-search-container">
    <div class="sidebar-search-input-wrap">
      <input
        type="text"
        placeholder="Filter tables..."
        bind:value={sidebarSearch}
        class="sidebar-search-input"
      />
      {#if sidebarSearch}
        <button
          class="clear-search-btn"
          onclick={() => (sidebarSearch = "")}
          type="button">&times;</button
        >
      {/if}
    </div>
    <span class="item-size table-count-badge">
      {#if sidebarSearch.trim()}
        {filteredTables.length}/{fontInfo.num_tables}
      {:else}
        {fontInfo.num_tables}
      {/if}
      tables
    </span>
  </div>

  <div class="sidebar-scrollable">
    {#each filteredTables as table}
      <div
        class="table-list-item {selectedTable?.tag === table.tag
          ? 'active'
          : ''}"
        onclick={() => onSelectTable(table)}
        role="button"
        tabindex="0"
        onkeydown={(event) => event.key === "Enter" && onSelectTable(table)}
      >
        <div
          class="tag-badge {selectedTable?.tag === table.tag ? 'active' : ''}"
        >
          {table.tag.trim()}
        </div>
        <div class="item-info">
          <span class="item-title">
            {TABLE_DESCRIPTIONS[table.tag.trim()] || "Unknown Table"}
          </span>
          <span class="item-size">{formatBytes(table.length)}</span>
        </div>
      </div>
    {/each}
  </div>
</aside>

<style>
  .sidebar {
    background: var(--sidebar-bg);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .sidebar-search-container {
    padding: 0 0.5rem 0 0.75rem;
    border-bottom: 1px solid var(--border-color);
    background: var(--sidebar-bg);
    display: flex;
    align-items: center;
    gap: 0.5rem;
    height: var(--toolbar-height);
  }

  .sidebar-search-input-wrap {
    position: relative;
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    background: var(--bg-color);
  }

  .sidebar-search-input {
    width: 100%;
    height: 30px;
    padding: 0 1.75rem 0 0.75rem;
    font-size: 0.825rem;
    border: 1px solid var(--border-color);
    border-radius: 6px;
    background: transparent;
    color: var(--text-color);
    outline: none;
    transition: all 0.15s ease;
  }

  .sidebar-search-input:focus {
    border-color: var(--primary);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--primary) 15%, transparent);
  }

  :global(.clear-search-btn) {
    position: absolute;
    right: 0.45rem;
    background: transparent;
    border: none;
    color: color-mix(in srgb, var(--text-color) 40%, transparent);
    cursor: pointer;
    font-size: 1.1rem;
    line-height: 1;
    padding: 0.2rem;
  }

  :global(.clear-search-btn:hover) {
    color: var(--text-color);
  }

  .sidebar-scrollable {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .sidebar-dir-row {
    border-bottom: 1px solid var(--border-color);
  }

  .table-list-item {
    padding: 0.5rem 0.75rem;
    min-height: var(--toolbar-height);
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 0.75rem;
    transition: all 0.15s ease;
    border-left: 3px solid transparent;
  }

  .table-list-item:hover {
    background: color-mix(in srgb, var(--primary) 5%, transparent);
  }

  .table-list-item.active {
    background: color-mix(in srgb, var(--primary) 8%, transparent);
    border-left-color: var(--primary);
  }

  .tag-badge {
    background: color-mix(in srgb, var(--text-color) 6%, transparent);
    border: 1px solid color-mix(in srgb, var(--border-color) 60%, transparent);
    color: var(--text-color);
    padding: 0 0.35rem;
    height: 2rem;
    line-height: 1;
    border-radius: 6px;
    font-family: var(--font-mono);
    font-weight: 600;
    font-size: 0.75rem;
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 3.5rem;
    flex-shrink: 0;
    transition: all 0.15s ease;
  }

  .tag-badge.active {
    background: var(--primary);
    border-color: var(--primary);
    color: #ffffff;
  }

  .item-info {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    min-width: 0;
  }

  .item-title {
    font-size: 0.8rem;
    line-height: 1.6;
    font-weight: 500;
    color: var(--text-color);
    text-overflow: ellipsis;
    overflow: hidden;
    white-space: nowrap;
  }

  .item-size {
    font-size: 0.7rem;
    color: color-mix(in srgb, var(--text-color) 50%, transparent);
  }

  .table-count-badge {
    margin-left: auto;
    font-size: 0.7rem;
    background: color-mix(in srgb, var(--text-color) 8%, transparent);
    padding: 0.2rem 0.5rem;
    border-radius: 10px;
    color: color-mix(in srgb, var(--text-color) 70%, transparent);
    white-space: nowrap;
    flex-shrink: 0;
  }
</style>
