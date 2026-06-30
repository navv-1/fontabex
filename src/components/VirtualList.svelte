<script lang="ts">
  import { onMount, tick, untrack, type Snippet } from "svelte";

  interface Props {
    items: any[];
    height?: string;
    itemHeight?: number;
    start?: number;
    end?: number;
    scrollTop?: number;
    snapScroll?: boolean;
    children?: Snippet<[any, number]>;
    footer?: Snippet;
    onreachedbottom?: () => void;
    [key: string]: any;
  }

  type ScrollAlign = "nearest" | "start" | "center" | "end";

  let {
    items = [],
    height = "100%",
    itemHeight = undefined,
    start = $bindable(0),
    end = $bindable(0),
    scrollTop = $bindable(0),
    snapScroll = false,
    children,
    footer,
    onreachedbottom,
    ...rest
  }: Props = $props();

  // local state
  let height_map: number[] = [];
  let rows: HTMLCollectionOf<HTMLElement>;
  let viewport = $state<HTMLElement>();
  let contents = $state<HTMLElement>();
  let viewport_height = $state(0);
  let mounted = $state(false);

  let top = $state(0);
  let bottom = $state(0);
  let average_height = 0;

  let visible = $derived(
    items.slice(start, end).map((data, i) => {
      return { index: i + start, data };
    }),
  );

  let last_internal_scroll_top = -1;

  // Manual ResizeObserver for viewport height — only fires on meaningful changes
  // This replaces bind:offsetHeight which triggers Svelte reactivity on every
  // sub-pixel change, causing cascading layout thrashing in flex containers.
  $effect(() => {
    if (!viewport) return;

    const ro = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const newHeight = Math.round(entry.contentRect.height);
        if (Math.abs(newHeight - viewport_height) >= 1) {
          viewport_height = newHeight;
        }
      }
    });
    ro.observe(viewport);
    // Seed initial value
    const initial = Math.round(viewport.offsetHeight);
    if (initial !== viewport_height) {
      viewport_height = initial;
    }
    return () => ro.disconnect();
  });

  // whenever `items` or viewport_height changes, refresh the visible window
  $effect(() => {
    const currentItems = items;
    const currentVh = viewport_height;
    const currentIh = itemHeight;
    const isMounted = mounted;

    if (isMounted) {
      untrack(() => {
        if (start >= currentItems.length && currentItems.length > 0) {
          start = 0;
          top = 0;
          scrollTop = 0;
          last_internal_scroll_top = 0;
          if (viewport) viewport.scrollTop = scrollTop;
        }
        refresh(currentItems, currentVh, currentIh);
      });
    }
  });

  $effect(() => {
    if (!mounted || !viewport) return;
    const nextScrollTop = scrollTop;
    if (nextScrollTop === last_internal_scroll_top) return;

    tick().then(() => {
      if (!viewport) return;
      if (Math.abs(viewport.scrollTop - nextScrollTop) > 1) {
        viewport.scrollTop = nextScrollTop;
      }
    });
  });

  async function refresh(
    items: any[],
    viewport_height: number,
    itemHeight: number | undefined = undefined,
  ) {
    if (!viewport || !contents) return;
    const currentScrollTop = viewport.scrollTop;

    await tick(); // wait until the DOM is up to date

    if (itemHeight) {
      start = Math.floor(currentScrollTop / itemHeight);
      top = start * itemHeight;
      end = Math.min(
        items.length,
        start + Math.ceil(viewport_height / itemHeight) + 1,
      );
      bottom = (items.length - end) * itemHeight;
      if (scrollTop > 0 && Math.abs(viewport.scrollTop - scrollTop) > 1) {
        viewport.scrollTop = scrollTop;
      }
      return;
    }

    let content_height = top - currentScrollTop;
    let i = start;

    while (content_height < viewport_height && i < items.length) {
      let row = rows[i - start] as HTMLElement;

      if (!row) {
        end = i + 1;
        await tick(); // render the newly visible row
        row = rows[i - start] as HTMLElement;
        if (!row) break; // still no row? safety exit
      }

      const row_height = (height_map[i] = itemHeight || row.offsetHeight);
      content_height += row_height;
      i += 1;
    }

    end = i;

    const remaining = items.length - end;
    average_height = (top + content_height) / end || 0;

    bottom = remaining * average_height;
    height_map.length = items.length;

    if (scrollTop > 0 && Math.abs(viewport.scrollTop - scrollTop) > 1) {
      viewport.scrollTop = scrollTop;
    }
  }

  let scroll_raf: number | null = null;

  function handle_scroll() {
    if (!viewport) return;
    if (scroll_raf) return;
    scroll_raf = requestAnimationFrame(() => {
      scroll_raf = null;
      if (!viewport) return;
      handle_scroll_immediate();
    });
  }

  async function handle_scroll_immediate() {
    if (!viewport) return;
    const currentScrollTop = viewport.scrollTop;
    last_internal_scroll_top = currentScrollTop;
    scrollTop = currentScrollTop;
    const listLength = items.length;

    if (itemHeight) {
      start = Math.floor(currentScrollTop / itemHeight);
      top = start * itemHeight;
      end = Math.min(
        listLength,
        start + Math.ceil(viewport_height / itemHeight) + 1,
      );
      bottom = (listLength - end) * itemHeight;
      if (end >= listLength - 5 && onreachedbottom) {
        onreachedbottom();
      }
      return;
    }

    const old_start = start;

    for (let v = 0; v < rows.length; v += 1) {
      height_map[start + v] =
        itemHeight || (rows[v] as HTMLElement).offsetHeight;
    }

    let i = 0;
    let y = 0;

    while (i < items.length) {
      const row_height = height_map[i] || average_height;
      if (y + row_height > currentScrollTop) {
        start = i;
        top = y;
        break;
      }
      y += row_height;
      i += 1;
    }

    while (i < items.length) {
      y += height_map[i] || average_height;
      i += 1;
      if (y > currentScrollTop + viewport_height) break;
    }

    end = i;

    const remaining = items.length - end;
    average_height = y / end || 0;

    while (i < items.length) height_map[i++] = average_height;
    bottom = remaining * average_height;

    if (end >= items.length - 5 && onreachedbottom) {
      onreachedbottom();
    }

    // prevent jumping if we scrolled up into unknown territory
    if (start < old_start) {
      await tick();

      let expected_height = 0;
      let actual_height = 0;

      for (let i = start; i < old_start; i += 1) {
        if (rows[i - start]) {
          expected_height += height_map[i];
          actual_height +=
            itemHeight || (rows[i - start] as HTMLElement).offsetHeight;
        }
      }

      const d = actual_height - expected_height;
      viewport.scrollTo(0, currentScrollTop + d);
    }
  }

  export function scrollToIndex(
    index: number,
    behavior: ScrollBehavior = "auto",
    align: ScrollAlign = "nearest",
  ) {
    if (!viewport) return;
    const rowHeight = (rowIndex: number) =>
      height_map[rowIndex] || itemHeight || average_height || 0;
    let y = 0;
    if (itemHeight) {
      y = index * itemHeight;
    } else {
      for (let i = 0; i < index; i += 1) {
        y += rowHeight(i);
      }
    }

    const { scrollTop, offsetHeight } = viewport;
    const targetItemHeight = rowHeight(index);
    let targetScrollTop: number | null = null;

    if (align === "start") {
      targetScrollTop = y;
    } else if (align === "center") {
      targetScrollTop = y - (offsetHeight - targetItemHeight) / 2;
    } else if (align === "end") {
      targetScrollTop = y + targetItemHeight - offsetHeight;
    } else if (y < scrollTop) {
      targetScrollTop = y;
    } else if (y + targetItemHeight > scrollTop + offsetHeight) {
      targetScrollTop = y + targetItemHeight - offsetHeight;
    }

    if (targetScrollTop !== null) {
      viewport.scrollTo({
        top: Math.max(0, targetScrollTop),
        behavior,
      });
    }
  }

  function handle_wheel(e: WheelEvent) {
    if (!snapScroll || !viewport) return;
    e.preventDefault();

    // Determine scroll direction: one row per tick
    const direction = e.deltaY > 0 ? 1 : e.deltaY < 0 ? -1 : 0;
    if (direction === 0) return;

    const rowHeight = height_map[start] || average_height || 50;
    viewport.scrollTop += direction * rowHeight;
  }

  // trigger initial refresh
  onMount(() => {
    if (contents) {
      rows = contents.getElementsByTagName(
        "svelte-virtual-list-row",
      ) as HTMLCollectionOf<HTMLElement>;
    }
    mounted = true;
    return () => {
      if (scroll_raf) cancelAnimationFrame(scroll_raf);
    };
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<svelte-virtual-list-viewport
  bind:this={viewport}
  onscroll={handle_scroll}
  onwheel={handle_wheel}
  style="height: {height};"
  {...rest}
>
  <svelte-virtual-list-contents
    bind:this={contents}
    style="padding-top: {top}px; padding-bottom: {bottom}px;"
  >
    {#each visible as row (row.index)}
      <svelte-virtual-list-row>
        {@render children?.(row.data, row.index)}
      </svelte-virtual-list-row>
    {/each}
    {@render footer?.()}
  </svelte-virtual-list-contents>
</svelte-virtual-list-viewport>

<style>
  svelte-virtual-list-viewport {
    position: relative;
    overflow-y: auto;
    display: block;
    contain: layout paint;
    will-change: transform;
    scrollbar-gutter: stable;
  }

  svelte-virtual-list-contents,
  svelte-virtual-list-row {
    display: block;
  }

  svelte-virtual-list-row {
    overflow: hidden;
  }
</style>
