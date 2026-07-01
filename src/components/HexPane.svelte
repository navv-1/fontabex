<script lang="ts">
  import { tick } from "svelte";
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
    if (hexPaneWidth >= 1840) return 32;
    if (hexPaneWidth >= 1000) return 16;
    if (hexPaneWidth >= 560) return 8;
    return 4;
  });

  let hexRows = $derived.by(() => {
    if (!rawBytes) return { length: 0, slice: () => [] };
    const rowSize = bytesPerRow;
    const length = Math.ceil(rawBytes.length / rowSize);
    return {
      length,
      slice(start: number, end: number) {
        const rows: HexRow[] = [];
        for (let i = start; i < end && i < length; i++) {
          const offset = i * rowSize;
          rows.push({
            offset,
            bytes: rawBytes.slice(offset, offset + rowSize),
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
    if (!selectedByteRange || bytesPerRow <= 0 || hexRows.length === 0) return;
    scrollToByteOffset(selectedByteRange.offset);
  });

  async function scrollToByteOffset(offset: number) {
    if (virtualListRef && bytesPerRow > 0) {
      await tick();
      const rowIndex = Math.floor(offset / bytesPerRow);
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
              <div class="hex-row hex-cols-{bytesPerRow}">
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
    padding: 0.4rem 0.75rem;
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

  .hex-cols-4 {
    --hex-byte-run-width: calc(88px + 0.75rem + 0.5rem);
  }

  .hex-cols-8 {
    --hex-byte-run-width: calc(176px + 1.75rem + 0.2rem + 0.5rem);
  }

  .hex-cols-16 {
    --hex-byte-run-width: calc(352px + 3.75rem + 0.6rem + 0.5rem);
  }

  .hex-cols-32 {
    --hex-byte-run-width: calc(704px + 7.75rem + 1.4rem + 0.5rem);
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

  .hex-loading {
    padding: 1rem;
    opacity: 0.5;
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
    .pane-header {
      padding-inline: 0.6rem;
    }

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
</style>
