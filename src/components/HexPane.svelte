<script lang="ts">
  import { tick, untrack } from "svelte";
  import { formatBytes } from "../lib/fontTables";
  import VirtualList from "./VirtualList.svelte";

  interface ByteRange {
    offset: number;
    length: number;
  }

  interface HexRow {
    offset: number;
    bytes: Uint8Array;
  }

  interface Props {
    rawBytes: Uint8Array | null;
    dataLoading: boolean;
    selectedByteRange: ByteRange | null;
  }

  let { rawBytes, dataLoading, selectedByteRange }: Props = $props();

  let hoveredByte = $state<number | null>(null);
  let hexPaneWidth = $state<number>(0);
  let virtualListRef = $state<any>(null);

  let bytesPerRow = $derived.by(() => {
    let bpr = 4;
    if (hexPaneWidth >= 1840) bpr = 32;
    else if (hexPaneWidth >= 1000) bpr = 16;
    else if (hexPaneWidth >= 560) bpr = 8;
    return bpr;
  });

  const PAGE_SIZE = 1024 * 1024;
  let currentPage = $state(0);
  let dropdownOpen = $state(false);

  $effect(() => {
    if (rawBytes) {
      const maxPage = Math.max(0, Math.ceil(rawBytes.length / PAGE_SIZE) - 1);
      if (currentPage > maxPage) {
        currentPage = maxPage;
      }
    } else {
      currentPage = 0;
    }
  });

  let pageStart = $derived(currentPage * PAGE_SIZE);
  let pageBytes = $derived(
    rawBytes ? rawBytes.slice(pageStart, pageStart + PAGE_SIZE) : null,
  );

  let runWidthStyle = $derived(
    `--hex-byte-run-width: calc(${bytesPerRow * 22}px + ${(bytesPerRow - 1) * 0.25}rem + ${Math.max(0, bytesPerRow / 4 - 1) * 0.2}rem + 0.5rem);`,
  );

  let hexRows = $derived.by(() => {
    if (!pageBytes) return { length: 0, slice: () => [] };
    const rowSize = bytesPerRow;
    const length = Math.ceil(pageBytes.length / rowSize);
    return {
      length,
      slice(start: number, end: number) {
        const rows: HexRow[] = [];
        for (let i = start; i < end && i < length; i++) {
          const offset = i * rowSize;
          rows.push({
            offset: pageStart + offset,
            bytes: pageBytes.slice(offset, offset + rowSize),
          });
        }
        return rows;
      },
    };
  });

  export function scrollToOffset(offset: number) {
    scrollToByteOffset(offset);
  }

  $effect(() => {
    const range = selectedByteRange;
    const bpr = bytesPerRow;
    const vlist = virtualListRef;

    if (!range || bpr <= 0 || !vlist) return;

    untrack(() => {
      if (hexRows.length > 0) {
        scrollToByteOffset(range.offset);
      }
    });
  });

  async function scrollToByteOffset(offset: number) {
    if (virtualListRef && bytesPerRow > 0) {
      const targetPage = Math.floor(offset / PAGE_SIZE);
      if (currentPage !== targetPage) {
        currentPage = targetPage;
      }
      await tick();
      const offsetInPage = offset - currentPage * PAGE_SIZE;
      const rowIndex = Math.floor(offsetInPage / bytesPerRow);
      virtualListRef.scrollToIndex(rowIndex, "auto", "nearest");
    }
  }

  function formatHexOffset(offset: number) {
    return offset.toString(16).padStart(8, "0").toUpperCase();
  }

  function formatHexByte(byte: number) {
    return byte.toString(16).padStart(2, "0").toUpperCase();
  }

  function formatAsciiChar(byte: number) {
    return byte >= 32 && byte <= 126 ? String.fromCharCode(byte) : ".";
  }
</script>

<div class="hex-pane" bind:clientWidth={hexPaneWidth}>
  <div class="pane-header">
    <span class="pane-header-title">
      <svg
        width="14"
        height="14"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="M14 3h7v7h-7zM3 14h7v7H3zM14 14h7v7h-7zM3 3h7v7H3z"></path>
      </svg>
      Hex Dump
    </span>
    <span class="hex-pane-badge">
      {rawBytes ? formatBytes(rawBytes.length) : "0 B"}
    </span>
  </div>

  {#if rawBytes && rawBytes.length > PAGE_SIZE}
    <div class="pagination-container">
      <div class="pagination-controls">
        <button
          class="icon-btn"
          aria-label="Previous Page"
          title="Previous Page"
          disabled={currentPage === 0}
          onclick={() => currentPage--}
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <polyline points="15 18 9 12 15 6"></polyline>
          </svg>
        </button>
        <div class="custom-select-container">
          <button
            class="custom-select-button"
            onclick={() => (dropdownOpen = !dropdownOpen)}
          >
            {formatHexOffset(currentPage * PAGE_SIZE)}
            <svg
              width="12"
              height="12"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <polyline points="6 9 12 15 18 9"></polyline>
            </svg>
          </button>

          {#if dropdownOpen}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="dropdown-overlay"
              onclick={() => (dropdownOpen = false)}
            ></div>
            <div class="custom-options">
              {#each { length: Math.ceil(rawBytes.length / PAGE_SIZE) } as _, i}
                <button
                  class="custom-option {currentPage === i ? 'selected' : ''}"
                  onclick={() => {
                    currentPage = i;
                    dropdownOpen = false;
                  }}
                >
                  {formatHexOffset(i * PAGE_SIZE)}
                </button>
              {/each}
            </div>
          {/if}
        </div>
        <button
          class="icon-btn"
          aria-label="Next Page"
          title="Next Page"
          disabled={currentPage >= Math.ceil(rawBytes.length / PAGE_SIZE) - 1}
          onclick={() => currentPage++}
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <polyline points="9 18 15 12 9 6"></polyline>
          </svg>
        </button>
      </div>
    </div>
  {/if}

  {#if dataLoading}
    <div class="loading-progress-stripe"></div>
  {/if}

  <div class="hex-scroll-container">
    <div class="hex-block-wrapper">
      <div class="hex-virtual-wrapper">
        {#if hexRows.length > 0}
          <VirtualList
            bind:this={virtualListRef}
            items={hexRows as any}
            itemHeight={24}
          >
            {#snippet children(row: HexRow)}
              <div class="hex-row" style={runWidthStyle}>
                <span class="hex-col-divider offset-col row-offset">
                  {formatHexOffset(row.offset)}
                </span>
                <span class="hex-col-divider hex-bytes-col">
                  {#each Array.from({ length: bytesPerRow }) as _, i}
                    {#if i < row.bytes.length}
                      {@const byteIndex = row.offset + i}
                      {@const isSelected =
                        selectedByteRange &&
                        byteIndex >= selectedByteRange.offset &&
                        byteIndex <
                          selectedByteRange.offset + selectedByteRange.length}
                      <!-- svelte-ignore a11y_no_static_element_interactions -->
                      <!-- svelte-ignore a11y_mouse_events_have_key_events -->
                      <span
                        class="hex-byte {hoveredByte === byteIndex
                          ? 'hovered'
                          : ''} {isSelected ? 'selected' : ''} {i > 0 &&
                        (i + 1) % 4 === 0 &&
                        i + 1 !== bytesPerRow
                          ? 'byte-gap'
                          : ''}"
                        onmouseover={() => (hoveredByte = byteIndex)}
                        onmouseout={() => (hoveredByte = null)}
                      >
                        {formatHexByte(row.bytes[i])}
                      </span>
                    {:else}
                      <span
                        class="hex-byte empty {i > 0 &&
                        (i + 1) % 4 === 0 &&
                        i + 1 !== bytesPerRow
                          ? 'byte-gap'
                          : ''}">00</span
                      >
                    {/if}
                  {/each}
                </span>
                <span class="hex-ascii-col row-ascii">
                  {#each Array.from({ length: bytesPerRow }) as _, i}
                    {#if i < row.bytes.length}
                      {@const byteIndex = row.offset + i}
                      {@const isSelected =
                        selectedByteRange &&
                        byteIndex >= selectedByteRange.offset &&
                        byteIndex <
                          selectedByteRange.offset + selectedByteRange.length}
                      <!-- svelte-ignore a11y_no_static_element_interactions -->
                      <!-- svelte-ignore a11y_mouse_events_have_key_events -->
                      <span
                        class="hex-byte {hoveredByte === byteIndex
                          ? 'hovered'
                          : ''} {isSelected ? 'selected' : ''} {i > 0 &&
                        (i + 1) % 4 === 0 &&
                        i + 1 !== bytesPerRow
                          ? 'byte-gap'
                          : ''}"
                        onmouseover={() => (hoveredByte = byteIndex)}
                        onmouseout={() => (hoveredByte = null)}
                      >
                        {formatAsciiChar(row.bytes[i])}
                      </span>
                    {:else}
                      <span
                        class="hex-byte empty {i > 0 &&
                        (i + 1) % 4 === 0 &&
                        i + 1 !== bytesPerRow
                          ? 'byte-gap'
                          : ''}">&nbsp;</span
                      >
                    {/if}
                  {/each}
                </span>
              </div>
            {/snippet}
          </VirtualList>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .hex-pane {
    background: var(--sidebar-bg);
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    flex-shrink: 0;
    container-type: inline-size;
  }

  .pane-header {
    padding: 0.4rem 0.6rem;
    border-bottom: 1px solid var(--border-color);
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-color);
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--sidebar-bg);
    height: 48px;
    flex-shrink: 0;
  }

  .pane-header-title {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
  }

  .hex-scroll-container {
    flex: 1;
    overflow-y: hidden;
    overflow-x: auto;
    font-family: "JetBrains Mono", "Fira Code", ui-monospace, monospace;
    font-size: 0.75rem;
    line-height: 24px;
    background: var(--sidebar-bg);
  }

  .hex-virtual-wrapper {
    min-width: 0;
    width: 100%;
    height: 100%;
    flex: 1;
  }

  :global(.hex-virtual-wrapper svelte-virtual-list-viewport) {
    overflow-y: scroll;
  }

  .hex-block-wrapper {
    min-width: max-content;
    width: 100%;
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .hex-row {
    display: grid;
    grid-template-columns: 4.5rem max-content max-content;
    column-gap: 0.5rem;
    width: max-content;
    margin-inline: auto;
    padding-inline: 0.4rem;
    height: 24px;
    align-items: center;
    white-space: pre;
    color: color-mix(in srgb, var(--text-color) 85%, transparent);
    font-variant-numeric: tabular-nums;
  }

  .hex-col-divider {
    border-right: 1px solid
      color-mix(in srgb, var(--border-color) 60%, transparent);
    padding-right: 0.5rem;
  }

  .offset-col {
    width: 4.5rem;
    text-align: right;
    user-select: none;
    color: color-mix(in srgb, var(--text-color) 40%, transparent);
    font-weight: 500;
  }

  .hex-bytes-col {
    display: flex;
    gap: 0.25rem;
    width: var(--hex-byte-run-width);
    min-width: 0;
  }

  .hex-ascii-col {
    display: flex;
    gap: 0.25rem;
    width: calc(var(--hex-byte-run-width) - 0.5rem);
    min-width: 0;
  }

  .hex-ascii-col.row-ascii {
    color: color-mix(in srgb, var(--text-color) 70%, transparent);
  }

  .hex-row:hover {
    background: color-mix(in srgb, var(--text-color) 2%, transparent);
  }

  .hex-byte {
    display: inline-flex;
    flex: 0 0 22px;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    height: 20px;
    width: 22px;
    transition:
      background-color 0.1s ease,
      color 0.1s ease;
    user-select: none;
    cursor: pointer;
  }

  .hex-byte.hovered {
    background: var(--primary);
    color: #ffffff !important;
  }

  .hex-byte.selected {
    background: color-mix(in srgb, var(--primary) 20%, transparent);
    color: var(--text-color);
    font-weight: 600;
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--primary) 30%, transparent);
  }

  .hex-byte.selected.hovered {
    background: var(--primary);
    color: #ffffff !important;
    box-shadow: none;
  }

  .hex-byte.empty {
    opacity: 0;
    pointer-events: none;
  }

  .byte-gap {
    margin-right: 0.2rem;
  }

  @container (max-width: 340px) {
    .hex-row {
      grid-template-columns: 4rem max-content max-content;
      column-gap: 0.5rem;
      padding-inline: 0.6rem;
    }

    .offset-col {
      width: 4rem;
    }

    .hex-col-divider {
      padding-right: 0.5rem;
    }
  }

  .hex-pane-badge {
    font-size: 0.7rem;
    background: color-mix(in srgb, var(--text-color) 8%, transparent);
    padding: 0.2rem 0.5rem;
    border-radius: 10px;
    font-weight: 400;
    color: color-mix(in srgb, var(--text-color) 70%, transparent);
  }

  .pagination-container {
    display: flex;
    justify-content: center;
    padding: 0.4rem 0.6rem;
    border-bottom: 1px solid var(--border-color);
    background: var(--sidebar-bg);
    flex-shrink: 0;
  }

  .pagination-controls {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .icon-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    color: color-mix(in srgb, var(--text-color) 70%, transparent);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0.2rem;
    border-radius: 4px;
  }

  .icon-btn:hover:not(:disabled) {
    background: color-mix(in srgb, var(--text-color) 10%, transparent);
    color: var(--text-color);
  }

  .icon-btn:disabled {
    opacity: 0.3;
    cursor: default;
  }

  .custom-select-container {
    position: relative;
  }

  .custom-select-button {
    background: var(--sidebar-bg);
    border: 1px solid var(--border-color);
    color: var(--text-color);
    border-radius: 4px;
    padding: 0.2rem 0.4rem;
    font-size: 0.75rem;
    font-family: "JetBrains Mono", "Fira Code", ui-monospace, monospace;
    cursor: pointer;
    outline: none;
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .custom-select-button:hover {
    border-color: color-mix(
      in srgb,
      var(--border-color) 80%,
      var(--text-color)
    );
  }

  .dropdown-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 99;
  }

  .custom-options {
    position: absolute;
    top: calc(100% + 4px);
    left: 50%;
    transform: translateX(-50%);
    background: var(--sidebar-bg);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    max-height: 200px;
    overflow-y: auto;
    z-index: 100;
    display: flex;
    flex-direction: column;
    min-width: 100%;
  }

  .custom-option {
    background: transparent;
    border: none;
    color: var(--text-color);
    padding: 0.4rem 0.6rem;
    font-size: 0.75rem;
    font-family: "JetBrains Mono", "Fira Code", ui-monospace, monospace;
    cursor: pointer;
    text-align: center;
    width: 100%;
  }

  .custom-option:hover {
    background: color-mix(in srgb, var(--text-color) 5%, transparent);
  }

  .custom-option.selected {
    background: var(--primary);
    color: #ffffff;
  }
</style>
