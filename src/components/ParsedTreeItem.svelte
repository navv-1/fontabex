<script lang="ts">
  import {
    formatParsedValue,
    getParsedRawValue,
    isParsedField,
  } from "../lib/parsedFormatting";

  let {
    keyName,
    data,
    index,
    showIndex = false,
    showName = true,
    onSelectBytes,
    onNavigate,
    selectedByteRange,
  } = $props<{
    keyName: string;
    data: any;
    index?: number;
    showIndex?: boolean;
    showName?: boolean;
    onSelectBytes: (offset: number, length: number) => void;
    onNavigate: (keyName: string, data: any) => void;
    selectedByteRange: { offset: number; length: number } | null;
  }>();

  let isField = $derived(isParsedField(data));

  let rawValue = $derived(getParsedRawValue(data));
  let dataType = $derived(isField && data.type ? String(data.type) : "Record");
  let isLinkedValue = $derived(
    rawValue !== null && typeof rawValue === "object",
  );
  let isSelected = $derived(
    isField &&
      selectedByteRange !== null &&
      data.offset === selectedByteRange.offset &&
      data.length === selectedByteRange.length,
  );

  function handleClick(e: MouseEvent) {
    e.stopPropagation();
    if (isField) {
      onSelectBytes(data.offset, data.length);
    }
  }

  function handleNavigate(e: MouseEvent) {
    e.stopPropagation();
    onNavigate(keyName, rawValue);
  }
</script>

<tr
  onclick={handleClick}
  class="tree-row {isField ? 'clickable' : ''} {isSelected
    ? 'selected-data-row'
    : ''}"
>
  {#if showIndex}
    <td class="col-index">{index}</td>
  {/if}
  <td class="col-type">{dataType}</td>
  {#if showName}
    <td class="col-key">
      <div class="key-container">
        {keyName}
      </div>
    </td>
  {/if}
  <td class="col-val">
    {#if isLinkedValue}
      <button class="value-link" type="button" onclick={handleNavigate}>
        {formatParsedValue(data)}
      </button>
    {:else}
      {formatParsedValue(data)}
    {/if}
  </td>
</tr>

<style>
  .tree-row.clickable {
    cursor: pointer;
  }

  .key-container {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
</style>
