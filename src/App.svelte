<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import AppToolbar from "./components/AppToolbar.svelte";
  import HexPane from "./components/HexPane.svelte";
  import TableSidebar from "./components/TableSidebar.svelte";
  import VirtualList from "./components/VirtualList.svelte";
  import WelcomePage from "./components/WelcomePage.svelte";
  import {
    TABLE_SPEC_LINKS,
    type FontInfo,
    type FontTableInfo,
  } from "./lib/fontTables";
  import {
    formatParsedValue,
    getParsedRawValue,
    isParsedField,
  } from "./lib/parsedFormatting";

  const UI_STATE_STORAGE_KEY = "fontabex.uiState.v1";

  interface StoredUiState {
    theme?: string;
    sidebarWidth?: number;
    hexPaneFixedWidth?: number;
    showSidebar?: boolean;
    showHexPane?: boolean;
  }

  function isBrowser() {
    return typeof window !== "undefined" && typeof localStorage !== "undefined";
  }

  function readStoredUiState(): StoredUiState {
    if (!isBrowser()) return {};

    try {
      const rawState = localStorage.getItem(UI_STATE_STORAGE_KEY);
      if (!rawState) return {};
      const parsedState = JSON.parse(rawState);
      return parsedState && typeof parsedState === "object" ? parsedState : {};
    } catch (error) {
      console.warn("Failed to read saved UI state", error);
      return {};
    }
  }

  function getStoredTheme(value: unknown) {
    return value === "dark" || value === "light" ? value : "light";
  }

  function getStoredNumber(value: unknown, fallback: number) {
    return typeof value === "number" && Number.isFinite(value)
      ? value
      : fallback;
  }

  function getStoredBoolean(value: unknown, fallback: boolean) {
    return typeof value === "boolean" ? value : fallback;
  }

  const storedUiState = readStoredUiState();

  let theme = $state(getStoredTheme(storedUiState.theme));
  let fontInfo = $state<FontInfo | null>(null);
  let fontPath = $state<string | null>(null);
  let selectedTable = $state<FontTableInfo | null>(null);
  let parsedData = $state<any>(null);
  let parsedError = $state<string | null>(null);
  let rawBytes = $state<Uint8Array | null>(null);
  let errorMsg = $state<string | null>(null);
  let loading = $state(false);
  let dataLoading = $state(false);
  let parsedPageStack = $state<
    { keyName: string; data: any; arrayItemType?: string }[]
  >([]);
  let isDraggingOver = $state(false);
  let sidebarSearch = $state("");
  let parsedSearch = $state("");
  let parsedSearchTarget = $state("index");
  let parsedSearchTargetOpen = $state(false);
  let parsedSearchTargetMenu = $state<HTMLElement>();
  let parsedColumnWidths = $state<Record<string, number>>({});
  let parsedResizePreviewWidths = $state<Record<string, number> | null>(null);
  let resizingParsedColumn = $state<string | null>(null);

  async function handleDroppedFile(path: string) {
    try {
      errorMsg = null;
      loading = true;
      const parsed = await invoke<FontInfo>("parse_font_tables", { path });
      fontPath = path;
      fontInfo = parsed;
      selectedTable = null;
      parsedData = null;
      parsedPageStack = [];
      parsedError = null;
      rawBytes = null;
    } catch (e) {
      console.error(e);
      errorMsg = String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    let unlistenPromise: Promise<() => void>;
    try {
      unlistenPromise = getCurrentWindow().onDragDropEvent((event) => {
        if (event.payload.type === "enter" || event.payload.type === "over") {
          isDraggingOver = true;
        } else if (event.payload.type === "leave") {
          isDraggingOver = false;
        } else if (event.payload.type === "drop") {
          isDraggingOver = false;
          const paths = event.payload.paths;
          if (paths && paths.length > 0) {
            handleDroppedFile(paths[0]);
          }
        }
      });
    } catch (e) {
      console.error("Failed to setup drag and drop listener", e);
    }
    return () => {
      if (unlistenPromise) {
        unlistenPromise.then((unlisten) => unlisten());
      }
    };
  });

  let isResizingHex = $state(false);
  let hexPaneFixedWidth = $state(
    getStoredNumber(storedUiState.hexPaneFixedWidth, 320),
  );

  let isResizingSidebar = $state(false);
  let sidebarWidth = $state(getStoredNumber(storedUiState.sidebarWidth, 320));

  let showSidebar = $state(getStoredBoolean(storedUiState.showSidebar, true));
  let showHexPane = $state(getStoredBoolean(storedUiState.showHexPane, false));
  let selectedByteRange = $state<{ offset: number; length: number } | null>(
    null,
  );

  const MIN_SIDEBAR = 320;
  const MIN_HEX = 320;
  const MIN_PARSED = 220;
  const HARD_MIN_SIDEBAR = 140;
  const HARD_MIN_HEX = 280;
  const HARD_MIN_PARSED = 150;

  function hasHexPane() {
    return Boolean(selectedTable && showHexPane);
  }

  function getResizersWidth() {
    return 0;
  }

  function getAvailablePaneWidth() {
    return Math.max(0, window.innerWidth - getResizersWidth());
  }

  function getPaneMins() {
    const available = getAvailablePaneWidth();
    if (!showSidebar && hasHexPane()) {
      return {
        sidebar: 0,
        hex: Math.min(
          MIN_HEX,
          Math.max(HARD_MIN_HEX, available - HARD_MIN_PARSED),
        ),
        parsed: Math.min(
          MIN_PARSED,
          Math.max(HARD_MIN_PARSED, available - HARD_MIN_HEX),
        ),
      };
    }

    if (!showSidebar) {
      return { sidebar: 0, hex: MIN_HEX, parsed: HARD_MIN_PARSED };
    }

    if (!hasHexPane()) {
      return {
        sidebar: Math.min(
          MIN_SIDEBAR,
          Math.max(HARD_MIN_SIDEBAR, available - HARD_MIN_PARSED),
        ),
        hex: MIN_HEX,
        parsed: Math.min(
          MIN_PARSED,
          Math.max(HARD_MIN_PARSED, available - HARD_MIN_SIDEBAR),
        ),
      };
    }

    const minTotal = MIN_SIDEBAR + MIN_HEX + MIN_PARSED;
    if (available >= minTotal) {
      return { sidebar: MIN_SIDEBAR, hex: MIN_HEX, parsed: MIN_PARSED };
    }

    const scale = available / minTotal;
    const sidebar = Math.max(HARD_MIN_SIDEBAR, Math.floor(MIN_SIDEBAR * scale));
    const hex = Math.max(HARD_MIN_HEX, Math.floor(MIN_HEX * scale));
    const parsed = Math.max(HARD_MIN_PARSED, available - sidebar - hex);
    return { sidebar, hex, parsed };
  }

  function clamp(value: number, min: number, max: number) {
    return Math.min(Math.max(value, min), Math.max(min, max));
  }

  function normalizePaneWidths() {
    const available = getAvailablePaneWidth();
    const mins = getPaneMins();

    if (!hasHexPane()) {
      if (!showSidebar) return;
      sidebarWidth = clamp(sidebarWidth, mins.sidebar, available - mins.parsed);
      return;
    }

    if (!showSidebar) {
      hexPaneFixedWidth = clamp(
        hexPaneFixedWidth,
        mins.hex,
        available - mins.parsed,
      );
      return;
    }

    sidebarWidth = clamp(
      sidebarWidth,
      mins.sidebar,
      available - mins.hex - mins.parsed,
    );
    hexPaneFixedWidth = clamp(
      hexPaneFixedWidth,
      mins.hex,
      available - sidebarWidth - mins.parsed,
    );

    const parsedWidth = available - sidebarWidth - hexPaneFixedWidth;
    if (parsedWidth < mins.parsed) {
      const shortfall = mins.parsed - parsedWidth;
      const reducibleHex = Math.max(0, hexPaneFixedWidth - mins.hex);
      const reduceHexBy = Math.min(shortfall, reducibleHex);
      hexPaneFixedWidth -= reduceHexBy;
      sidebarWidth = Math.max(
        mins.sidebar,
        sidebarWidth - (shortfall - reduceHexBy),
      );
    }
  }

  function startResizeMain(e: MouseEvent) {
    isResizingHex = true;
    e.preventDefault();
  }

  function startResizeSidebar(e: MouseEvent) {
    isResizingSidebar = true;
    e.preventDefault();
  }

  function doGlobalResize(e: MouseEvent) {
    if (isResizingHex) {
      const mins = getPaneMins();
      const maxHexWidth =
        getAvailablePaneWidth() -
        (showSidebar ? sidebarWidth : 0) -
        mins.parsed;
      hexPaneFixedWidth -= e.movementX;
      hexPaneFixedWidth = clamp(hexPaneFixedWidth, mins.hex, maxHexWidth);
    } else if (isResizingSidebar) {
      sidebarWidth += e.movementX;
      normalizePaneWidths();
    }
  }

  $effect(() => {
    selectedTable;
    showSidebar;
    showHexPane;
    normalizePaneWidths();
  });

  function stopGlobalResize() {
    isResizingHex = false;
    isResizingSidebar = false;
  }

  function handleWindowResize() {
    normalizePaneWidths();
  }

  let mainGridColumns = $derived.by(() => {
    const columns = [];
    if (showSidebar) {
      columns.push(`${sidebarWidth}px`);
    }
    columns.push("minmax(0, 1fr)");
    if (selectedTable && showHexPane) {
      columns.push(`${hexPaneFixedWidth}px`);
    }
    return columns.join(" ");
  });

  let hexPaneRef = $state<any>(null);

  let selectedTableSpecUrl = $derived.by(() => {
    if (!selectedTable) return null;
    return TABLE_SPEC_LINKS[selectedTable.tag.trim()] ?? null;
  });

  function toggleTheme() {
    theme = theme === "dark" ? "light" : "dark";
    document.documentElement.setAttribute("data-theme", theme);
  }

  $effect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  });

  $effect(() => {
    if (!isBrowser()) return;

    const state: StoredUiState = {
      theme,
      sidebarWidth,
      hexPaneFixedWidth,
      showSidebar,
      showHexPane,
    };

    try {
      localStorage.setItem(UI_STATE_STORAGE_KEY, JSON.stringify(state));
    } catch (error) {
      console.warn("Failed to save UI state", error);
    }
  });

  async function openFont() {
    try {
      errorMsg = null;
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: "Fonts",
            extensions: ["ttf", "otf"],
          },
        ],
      });

      if (selected) {
        loading = true;
        const path = Array.isArray(selected) ? selected[0] : selected;
        const parsed = await invoke<FontInfo>("parse_font_tables", { path });

        fontPath = path;
        fontInfo = parsed;
        selectedTable = null;
        parsedData = null;
        parsedPageStack = [];
        parsedError = null;
        rawBytes = null;
      }
    } catch (e) {
      console.error(e);
      errorMsg = String(e);
    } finally {
      loading = false;
    }
  }

  async function selectTable(table: FontTableInfo) {
    selectedTable = table;
    parsedData = null;
    parsedPageStack = [];
    parsedError = null;
    rawBytes = null;
    selectedByteRange = null;
    if (!fontPath) return;

    try {
      dataLoading = true;
      const [data, bytesArray] = await Promise.all([
        invoke<any>("parse_specific_table", {
          path: fontPath,
          tag: table.tag,
        }).catch((e) => {
          parsedError = String(e);
          return null;
        }),
        invoke<number[]>("get_table_bytes", {
          path: fontPath,
          offset: table.offset,
          length: table.length,
        }).catch((e) => {
          console.error("Failed to fetch raw bytes:", e);
          return [];
        }),
      ]);
      parsedData = data;
      rawBytes = new Uint8Array(bytesArray);
    } catch (e) {
      if (!parsedError) parsedError = String(e);
    } finally {
      dataLoading = false;
    }
  }

  function getTableEntries(data: any): [string, any][] {
    if (data && typeof data === "object" && !Array.isArray(data)) {
      return Object.entries(data);
    }
    return [];
  }

  function getParsedValueClass(value: any) {
    const rawValue = getParsedRawValue(value);
    if (rawValue === null) return "value-null";
    if (Array.isArray(rawValue)) return "value-array";
    switch (typeof rawValue) {
      case "number":
        return "value-number";
      case "string":
        return "value-string";
      case "boolean":
        return "value-boolean";
      case "object":
        return "value-record";
      default:
        return "value-unknown";
    }
  }

  function getParsedEntryKey(key: string, value: any) {
    if (isParsedField(value) && value.name) return String(value.name);
    return key;
  }

  function getParsedType(value: any) {
    if (isParsedField(value)) return value.type || "";
    const rawValue = getParsedRawValue(value);
    if (Array.isArray(rawValue)) return "Array";
    if (rawValue !== null && typeof rawValue === "object") return "Record";
    if (currentParsedPageIsArray && currentParsedArrayItemType) {
      return currentParsedArrayItemType;
    }
    return "";
  }

  function getParsedArrayItemType(value: any) {
    if (!isParsedField(value) || typeof value.type !== "string") return;
    if (!value.type.endsWith("[]")) return;
    return value.type.slice(0, -2);
  }

  let currentParsedPage = $derived.by(() => {
    if (parsedPageStack.length > 0) {
      return parsedPageStack[parsedPageStack.length - 1].data;
    }
    return parsedData;
  });

  let currentParsedArrayItemType = $derived.by(() => {
    if (parsedPageStack.length === 0) return;
    return parsedPageStack[parsedPageStack.length - 1].arrayItemType;
  });

  let currentParsedEntries = $derived.by(() => {
    if (currentParsedPage && typeof currentParsedPage === "object") {
      return Object.entries(currentParsedPage);
    }
    return [];
  });

  let currentParsedPageIsArray = $derived(Array.isArray(currentParsedPage));

  let currentParsedObjectRows = $derived.by(() => {
    if (!Array.isArray(currentParsedPage)) return [];
    if (
      !currentParsedPage.every((item) => {
        const rawItem = getParsedRawValue(item);
        return (
          rawItem !== null &&
          typeof rawItem === "object" &&
          !Array.isArray(rawItem)
        );
      })
    ) {
      return [];
    }
    return currentParsedPage.map(getParsedRawValue) as Record<string, any>[];
  });

  let currentParsedObjectColumns = $derived.by(() => {
    const columns: string[] = [];
    const seen = new Set<string>();
    for (const row of currentParsedObjectRows) {
      for (const key of Object.keys(row)) {
        if (key === "name") continue;
        if (!seen.has(key)) {
          seen.add(key);
          columns.push(key);
        }
      }
    }
    return columns;
  });

  function getParsedObjectColumnType(column: string) {
    for (const row of currentParsedObjectRows) {
      const value = row[column];
      if (isParsedField(value) && value.type) return String(value.type);
    }
    return "";
  }

  let currentParsedObjectHasKeys = $derived.by(() => {
    return currentParsedObjectRows.some(
      (row) => typeof row.name === "string" && row.name.length > 0,
    );
  });

  let currentParsedEntriesHaveNames = $derived.by(() => {
    return currentParsedEntries.some(([, value]) => {
      const parsedValue = value as any;
      return (
        isParsedField(parsedValue) &&
        typeof parsedValue.name === "string" &&
        parsedValue.name.length > 0
      );
    });
  });

  let currentParsedShowsNameColumn = $derived(
    !currentParsedPageIsArray || currentParsedEntriesHaveNames,
  );

  let currentParsedShowsTypeColumn = $derived(
    !currentParsedPageIsArray || Boolean(currentParsedArrayItemType),
  );

  const COL_MIN = 64;
  const COL_FLEX = "1fr";
  const INDEX_COL_MIN = 44;
  const INDEX_COL_FLEX = "0.5fr";

  function colMin(columnId: string) {
    return columnId === "index" ? INDEX_COL_MIN : COL_MIN;
  }

  function colFlex(columnId: string) {
    return columnId === "index" ? INDEX_COL_FLEX : COL_FLEX;
  }

  function colWidthKey(columnId: string) {
    const path = [
      selectedTable?.tag ?? "",
      ...parsedPageStack.map((p) => p.keyName),
    ].join(">");
    return `${path}:${columnId}`;
  }

  function colTemplate(columnId: string): string {
    const key = colWidthKey(columnId);
    const preview = parsedResizePreviewWidths?.[key];
    if (preview) return `${preview}px`;

    const stored = parsedColumnWidths[key];
    return stored
      ? `${stored}px`
      : `minmax(${colMin(columnId)}px, ${colFlex(columnId)})`;
  }

  let currentParsedGridTemplate = $derived.by(() => {
    const ids: string[] = [];
    if (currentParsedPageIsArray) ids.push("index");
    if (currentParsedShowsTypeColumn) ids.push("type");
    if (currentParsedShowsNameColumn) ids.push("name");
    ids.push("value");
    return ids.map(colTemplate).join(" ");
  });

  let currentParsedObjectGridTemplate = $derived.by(() => {
    const ids = [
      "index",
      ...(currentParsedObjectHasKeys ? ["name"] : []),
      ...currentParsedObjectColumns.map((c) => `column:${c}`),
    ];
    return ids.map(colTemplate).join(" ");
  });

  function getVisibleParsedColumnIds() {
    if (currentParsedObjectRows.length > 0) {
      return [
        "index",
        ...(currentParsedObjectHasKeys ? ["name"] : []),
        ...currentParsedObjectColumns.map((c) => `column:${c}`),
      ];
    }
    return [
      ...(currentParsedPageIsArray ? ["index"] : []),
      ...(currentParsedShowsTypeColumn ? ["type"] : []),
      ...(currentParsedShowsNameColumn ? ["name"] : []),
      "value",
    ];
  }

  function resetParsedColumnWidth(columnId: string) {
    const key = colWidthKey(columnId);
    const { [key]: _, ...rest } = parsedColumnWidths;
    parsedColumnWidths = rest;
  }

  function startParsedColumnResize(event: PointerEvent, columnId: string) {
    event.preventDefault();
    event.stopPropagation();

    const handle = event.currentTarget as HTMLElement;
    const headerCell = handle.closest(
      ".virtual-data-cell",
    ) as HTMLElement | null;
    const grid = handle.closest(".virtual-data-grid") as HTMLElement | null;
    if (!headerCell || !grid) return;

    const widthKey = colWidthKey(columnId);
    const startX = event.clientX;
    const visibleColumns = getVisibleParsedColumnIds();
    const headerCells = Array.from(
      grid.querySelectorAll<HTMLElement>(
        ".virtual-data-header > .virtual-data-cell",
      ),
    );
    const lastColumnId = visibleColumns[visibleColumns.length - 1];
    const frozenWidths = visibleColumns.reduce<Record<string, number>>(
      (widths, id, index) => {
        if (id === lastColumnId && id !== columnId) return widths;
        const key = colWidthKey(id);
        const measured =
          headerCells[index]?.getBoundingClientRect().width ?? colMin(id);
        widths[key] = Math.max(colMin(id), parsedColumnWidths[key] ?? measured);
        return widths;
      },
      {},
    );
    const startWidth =
      frozenWidths[widthKey] ?? headerCell.getBoundingClientRect().width;
    parsedResizePreviewWidths = frozenWidths;
    resizingParsedColumn = columnId;
    handle.setPointerCapture?.(event.pointerId);

    const handlePointerMove = (moveEvent: PointerEvent) => {
      const nextWidth = Math.max(
        colMin(columnId),
        Math.round(startWidth + moveEvent.clientX - startX),
      );
      parsedColumnWidths = {
        ...parsedColumnWidths,
        [widthKey]: nextWidth,
      };
      parsedResizePreviewWidths = {
        ...frozenWidths,
        [widthKey]: nextWidth,
      };
    };

    const handlePointerUp = () => {
      handle.releasePointerCapture?.(event.pointerId);
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
      document.body.classList.remove("resizing-column");
      parsedResizePreviewWidths = null;
      resizingParsedColumn = null;
    };

    document.body.classList.add("resizing-column");
    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", handlePointerUp, { once: true });
  }

  function formatParsedSearchColumnLabel(column: string) {
    return column
      .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
      .replace(/[_-]+/g, " ")
      .toUpperCase();
  }

  let parsedSearchTargetOptions = $derived.by(() => {
    if (currentParsedObjectRows.length > 0) {
      return [
        { value: "index", label: "Index" },
        { value: "all", label: "All Columns" },
        ...(currentParsedObjectHasKeys
          ? [{ value: "name", label: "NAME" }]
          : []),
        ...currentParsedObjectColumns.map((column) => ({
          value: `column:${column}`,
          label: formatParsedSearchColumnLabel(column),
        })),
      ];
    }

    const options = [
      ...(currentParsedPageIsArray ? [{ value: "index", label: "Index" }] : []),
      { value: "all", label: "All Columns" },
      ...(currentParsedShowsTypeColumn
        ? [{ value: "type", label: "TYPE" }]
        : []),
      ...(currentParsedShowsNameColumn
        ? [{ value: "name", label: "NAME" }]
        : []),
      { value: "value", label: "VALUE" },
    ];
    return options;
  });

  let activeParsedSearchTarget = $derived.by(() => {
    return parsedSearchTargetOptions.some(
      (option) => option.value === parsedSearchTarget,
    )
      ? parsedSearchTarget
      : (parsedSearchTargetOptions[0]?.value ?? "all");
  });

  let activeParsedSearchTargetLabel = $derived.by(() => {
    return (
      parsedSearchTargetOptions.find(
        (option) => option.value === activeParsedSearchTarget,
      )?.label ?? "All Columns"
    );
  });

  $effect(() => {
    if (parsedSearchTarget !== activeParsedSearchTarget) {
      parsedSearchTarget = activeParsedSearchTarget;
    }
  });

  $effect(() => {
    if (!parsedSearchTargetOpen) return;
    const handlePointerDown = (event: PointerEvent) => {
      if (
        parsedSearchTargetMenu &&
        !parsedSearchTargetMenu.contains(event.target as Node)
      ) {
        parsedSearchTargetOpen = false;
      }
    };
    document.addEventListener("pointerdown", handlePointerDown);
    return () => document.removeEventListener("pointerdown", handlePointerDown);
  });

  let normalizedParsedSearch = $derived(parsedSearch.trim().toLowerCase());

  function matchesParsedSearchText(value: any) {
    if (!normalizedParsedSearch) return true;
    return String(value ?? "")
      .toLowerCase()
      .includes(normalizedParsedSearch);
  }

  function matchesParsedObjectRow(row: Record<string, any>, rowIndex: number) {
    if (!normalizedParsedSearch) return true;
    if (activeParsedSearchTarget === "index") {
      return matchesParsedSearchText(rowIndex);
    }
    if (activeParsedSearchTarget === "name") {
      return matchesParsedSearchText(row.name);
    }
    if (activeParsedSearchTarget.startsWith("column:")) {
      const column = activeParsedSearchTarget.slice("column:".length);
      return matchesParsedSearchText(formatParsedValue(row[column]));
    }
    return [
      rowIndex,
      row.name,
      ...currentParsedObjectColumns.map((column) =>
        formatParsedValue(row[column]),
      ),
    ].some(matchesParsedSearchText);
  }

  function matchesParsedEntry(entry: [string, unknown], rowIndex: number) {
    if (!normalizedParsedSearch) return true;
    const [key, value] = entry;
    if (activeParsedSearchTarget === "index") {
      return matchesParsedSearchText(rowIndex);
    }
    if (activeParsedSearchTarget === "type") {
      return matchesParsedSearchText(getParsedType(value));
    }
    if (activeParsedSearchTarget === "name") {
      return matchesParsedSearchText(getParsedEntryKey(key, value));
    }
    if (activeParsedSearchTarget === "value") {
      return matchesParsedSearchText(formatParsedValue(value));
    }
    return [
      currentParsedPageIsArray ? rowIndex : "",
      currentParsedShowsTypeColumn ? getParsedType(value) : "",
      currentParsedShowsNameColumn ? getParsedEntryKey(key, value) : "",
      formatParsedValue(value),
    ].some(matchesParsedSearchText);
  }

  let filteredParsedObjectRows = $derived.by(() => {
    return currentParsedObjectRows
      .map((row, rowIndex) => ({ row, rowIndex }))
      .filter(({ row, rowIndex }) => matchesParsedObjectRow(row, rowIndex));
  });

  let filteredParsedEntries = $derived.by(() => {
    return currentParsedEntries
      .map((entry, rowIndex) => ({ entry, rowIndex }))
      .filter(({ entry, rowIndex }) => matchesParsedEntry(entry, rowIndex));
  });

  let currentParsedTotalCount = $derived.by(() => {
    if (currentParsedObjectRows.length > 0)
      return currentParsedObjectRows.length;
    return currentParsedEntries.length;
  });

  let currentParsedVisibleCount = $derived.by(() => {
    if (currentParsedObjectRows.length > 0)
      return filteredParsedObjectRows.length;
    return filteredParsedEntries.length;
  });

  function isLinkedParsedValue(value: any) {
    const rawValue = getParsedRawValue(value);
    return rawValue !== null && typeof rawValue === "object";
  }

  function isSelectedParsedField(value: any) {
    return (
      isParsedField(value) &&
      selectedByteRange !== null &&
      value.offset === selectedByteRange.offset &&
      value.length === selectedByteRange.length
    );
  }

  function navigateParsedValue(
    keyName: string,
    data: any,
    arrayItemType?: string,
  ) {
    parsedPageStack = [...parsedPageStack, { keyName, data, arrayItemType }];
  }

  function selectParsedFieldBytes(value: any) {
    if (isParsedField(value)) {
      handleSelectBytes(value.offset, value.length);
    }
  }

  function handleParsedCellLink(keyName: string, value: any) {
    selectParsedFieldBytes(value);
    navigateParsedValue(
      keyName,
      getParsedRawValue(value),
      getParsedArrayItemType(value),
    );
  }

  function handleParsedCellClick(value: any) {
    selectParsedFieldBytes(value);
  }

  function popParsedPage() {
    parsedPageStack = parsedPageStack.slice(0, -1);
  }

  function goToParsedPage(index: number) {
    parsedPageStack = parsedPageStack.slice(0, index);
  }

  function handleSelectBytes(offset: number, length: number) {
    selectedByteRange = { offset, length };
    hexPaneRef?.scrollToOffset(offset);
  }
</script>

<svelte:window
  onmousemove={doGlobalResize}
  onmouseup={stopGlobalResize}
  onresize={handleWindowResize}
/>

{#if !fontInfo}
  <WelcomePage
    {theme}
    {loading}
    {isDraggingOver}
    {errorMsg}
    onOpenFont={openFont}
    onToggleTheme={toggleTheme}
  />
{:else}
  <div class="app-layout">
    <AppToolbar
      {loading}
      {fontPath}
      {errorMsg}
      {selectedTable}
      bind:showSidebar
      bind:showHexPane
      {theme}
      onOpenFont={openFont}
      onToggleTheme={toggleTheme}
    />

    <!-- MAIN CONTENT -->
    <div class="main-area" style="grid-template-columns: {mainGridColumns};">
      {#if showSidebar}
        <TableSidebar
          {fontInfo}
          {selectedTable}
          bind:sidebarSearch
          onSelectTable={selectTable}
        />

        <!-- RESIZER FOR SIDEBAR -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <div
          class="split-resizer sidebar-resizer {isResizingSidebar
            ? 'active'
            : ''}"
          style="left: {sidebarWidth}px;"
          onmousedown={startResizeSidebar}
          role="separator"
          aria-orientation="vertical"
          aria-label="Resize sidebar"
        ></div>
      {/if}

      <!-- Content Area -->
      <main class="content-area">
        {#if !selectedTable}
          <div class="empty-state">
            <p>Select a table to view details.</p>
          </div>
        {:else}
          <div class="content-header">
            <button
              class="button button-outline button-icon"
              type="button"
              onclick={popParsedPage}
              disabled={parsedPageStack.length === 0}
              title="Back"
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
                <path d="m12 19-7-7 7-7"></path>
                <path d="M19 12H5"></path>
              </svg>
            </button>
            <div class="parsed-nav">
              <h2>
                <svg
                  width="16"
                  height="16"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  ><rect x="3" y="3" width="18" height="18" rx="2" ry="2"
                  ></rect><line x1="3" y1="9" x2="21" y2="9"></line><line
                    x1="9"
                    y1="21"
                    x2="9"
                    y2="9"
                  ></line></svg
                >
                <button
                  class="parsed-crumb"
                  type="button"
                  onclick={() => goToParsedPage(0)}
                >
                  {selectedTable.tag}
                </button>
                {#each parsedPageStack as page, index}
                  <span class="parsed-crumb-separator" aria-hidden="true">
                    <svg
                      width="13"
                      height="13"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="2.4"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                    >
                      <path d="m9 18 6-6-6-6"></path>
                    </svg>
                  </span>
                  <button
                    class="parsed-crumb"
                    type="button"
                    onclick={() => goToParsedPage(index + 1)}
                  >
                    {page.keyName}
                  </button>
                {/each}
              </h2>
            </div>

            <div class="parsed-search" role="search">
              <span class="table-badge parsed-count-badge">
                {#if normalizedParsedSearch}
                  {currentParsedVisibleCount}/{currentParsedTotalCount}
                {:else}
                  {currentParsedTotalCount}
                {/if}
              </span>
              <div class="parsed-search-group">
                <div
                  class="parsed-search-select-wrap"
                  bind:this={parsedSearchTargetMenu}
                >
                  <button
                    class="parsed-search-target"
                    class:open={parsedSearchTargetOpen}
                    type="button"
                    aria-label="Search target"
                    aria-haspopup="listbox"
                    aria-expanded={parsedSearchTargetOpen}
                    onclick={() =>
                      (parsedSearchTargetOpen = !parsedSearchTargetOpen)}
                  >
                    {activeParsedSearchTargetLabel}
                  </button>
                  <svg
                    width="10"
                    height="10"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2.5"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    aria-hidden="true"
                  >
                    <path d="m6 9 6 6 6-6"></path>
                  </svg>
                  {#if parsedSearchTargetOpen}
                    <div class="parsed-search-menu" role="listbox">
                      {#each parsedSearchTargetOptions as option}
                        <button
                          class="parsed-search-option {option.value ===
                          activeParsedSearchTarget
                            ? 'selected'
                            : ''}"
                          type="button"
                          role="option"
                          aria-selected={option.value ===
                            activeParsedSearchTarget}
                          onclick={() => {
                            parsedSearchTarget = option.value;
                            parsedSearchTargetOpen = false;
                          }}
                        >
                          {option.label}
                        </button>
                      {/each}
                    </div>
                  {/if}
                </div>
                <div class="parsed-search-divider"></div>
                <div class="parsed-search-input-wrap">
                  <svg
                    class="parsed-search-icon"
                    width="12"
                    height="12"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2.5"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    aria-hidden="true"
                  >
                    <circle cx="11" cy="11" r="8"></circle>
                    <path d="m21 21-4.3-4.3"></path>
                  </svg>
                  <input
                    class="parsed-search-input"
                    type="text"
                    bind:value={parsedSearch}
                    placeholder={activeParsedSearchTarget === "index"
                      ? "0-18"
                      : "Search..."}
                    aria-label="Search parsed data"
                  />
                  {#if parsedSearch}
                    <button
                      class="clear-search-btn"
                      onclick={() => (parsedSearch = "")}
                      type="button"
                      aria-label="Clear search"
                    >
                      &times;
                    </button>
                  {/if}
                </div>
              </div>
            </div>
          </div>
          {#if selectedTableSpecUrl}
            <div class="spec-link-row">
              <a
                class="spec-link"
                href={selectedTableSpecUrl}
                target="_blank"
                rel="noreferrer"
              >
                View Specification
              </a>
            </div>
          {/if}

          <!-- LEFT PANE: Parsed Data Grid -->
          <div class="parsed-pane">
            {#if dataLoading}
              <div class="loading-state">Loading data...</div>
            {:else if parsedError}
              <div class="error-container">
                <div class="error-title">{parsedError}</div>
                <p class="error-subtitle">
                  Currently displaying fallback metadata layout.
                </p>
                <table class="data-grid">
                  <thead>
                    <tr>
                      <th class="col-type">Type</th>
                      <th>NAME</th>
                      <th>VALUE</th>
                    </tr>
                  </thead>
                  <tbody>
                    {#if selectedTable}
                      <tr
                        ><td class="col-type">Tag</td><td class="col-key"
                          >tag</td
                        ><td class="col-val">{selectedTable.tag}</td></tr
                      >
                      <tr
                        ><td class="col-type">uint32</td><td class="col-key"
                          >offset</td
                        ><td class="col-val"
                          >0x{selectedTable.offset
                            .toString(16)
                            .toUpperCase()}</td
                        ></tr
                      >
                      <tr
                        ><td class="col-type">uint32</td><td class="col-key"
                          >length</td
                        ><td class="col-val">{selectedTable.length}</td></tr
                      >
                    {/if}
                  </tbody>
                </table>
              </div>
            {:else if currentParsedPage && typeof currentParsedPage === "object"}
              {#if currentParsedObjectRows.length > 0}
                <div
                  class="virtual-data-grid object-array-grid"
                  style={`--parsed-grid-template: ${currentParsedObjectGridTemplate};`}
                >
                  <div class="virtual-data-header" role="row">
                    <div
                      class="virtual-data-cell col-index resizable-column-header"
                      role="columnheader"
                    >
                      <span class="column-header-label">#</span>
                      <button
                        class="column-resize-handle {resizingParsedColumn ===
                        'index'
                          ? 'active'
                          : ''}"
                        type="button"
                        aria-label="Resize index column"
                        onpointerdown={(event) =>
                          startParsedColumnResize(event, "index")}
                        ondblclick={() => resetParsedColumnWidth("index")}
                      ></button>
                    </div>
                    {#if currentParsedObjectHasKeys}
                      <div
                        class="virtual-data-cell col-key resizable-column-header"
                        role="columnheader"
                      >
                        <span class="column-header-label">Name</span>
                        <button
                          class="column-resize-handle {resizingParsedColumn ===
                          'name'
                            ? 'active'
                            : ''}"
                          type="button"
                          aria-label="Resize name column"
                          onpointerdown={(event) =>
                            startParsedColumnResize(event, "name")}
                          ondblclick={() => resetParsedColumnWidth("name")}
                        ></button>
                      </div>
                    {/if}
                    {#each currentParsedObjectColumns as column}
                      {@const columnLabel =
                        formatParsedSearchColumnLabel(column)}
                      {@const columnType = getParsedObjectColumnType(column)}
                      <div
                        class="virtual-data-cell resizable-column-header"
                        role="columnheader"
                      >
                        <span class="column-header-stack">
                          <span class="column-header-label">{columnLabel}</span>
                          {#if columnType}
                            <span class="column-header-type">
                              {columnType}
                            </span>
                          {/if}
                        </span>
                        <button
                          class="column-resize-handle {resizingParsedColumn ===
                          `column:${column}`
                            ? 'active'
                            : ''}"
                          type="button"
                          aria-label={`Resize ${columnLabel} column`}
                          onpointerdown={(event) =>
                            startParsedColumnResize(event, `column:${column}`)}
                          ondblclick={() =>
                            resetParsedColumnWidth(`column:${column}`)}
                        ></button>
                      </div>
                    {/each}
                  </div>
                  {#if filteredParsedObjectRows.length === 0}
                    <div class="table-empty-row virtual-empty-row">
                      No matching entries found.
                    </div>
                  {:else}
                    <VirtualList
                      class="virtual-data-body"
                      items={filteredParsedObjectRows}
                      height="auto"
                      itemHeight={44}
                    >
                      {#snippet children(item)}
                        {@const row = item.row}
                        {@const rowIndex = item.rowIndex}
                        <div class="virtual-data-row" role="row">
                          <div class="virtual-data-cell col-index" role="cell">
                            {rowIndex}
                          </div>
                          {#if currentParsedObjectHasKeys}
                            <div class="virtual-data-cell col-key" role="cell">
                              {row.name ?? ""}
                            </div>
                          {/if}
                          {#each currentParsedObjectColumns as column}
                            {@const cellValue = row[column]}
                            {@const rawCellValue = getParsedRawValue(cellValue)}
                            <!-- svelte-ignore a11y_click_events_have_key_events -->
                            <!-- svelte-ignore a11y_interactive_supports_focus -->
                            <div
                              class="virtual-data-cell col-val {isParsedField(
                                cellValue,
                              )
                                ? 'clickable-cell'
                                : ''} {getParsedValueClass(
                                cellValue,
                              )} {isSelectedParsedField(cellValue)
                                ? 'selected-data-cell'
                                : ''}"
                              role="cell"
                              onclick={() => handleParsedCellClick(cellValue)}
                            >
                              {#if cellValue === undefined}
                                <span class="empty-cell">-</span>
                              {:else if isLinkedParsedValue(rawCellValue)}
                                <button
                                  class="value-link"
                                  type="button"
                                  onclick={(event) => {
                                    event.stopPropagation();
                                    handleParsedCellLink(
                                      `${rowIndex}.${column}`,
                                      cellValue,
                                    );
                                  }}
                                >
                                  {formatParsedValue(cellValue)}
                                </button>
                              {:else}
                                {formatParsedValue(cellValue)}
                              {/if}
                            </div>
                          {/each}
                        </div>
                      {/snippet}
                    </VirtualList>
                  {/if}
                </div>
              {:else}
                <div
                  class="virtual-data-grid"
                  style={`--parsed-grid-template: ${currentParsedGridTemplate};`}
                >
                  <div class="virtual-data-header" role="row">
                    {#if currentParsedPageIsArray}
                      <div
                        class="virtual-data-cell col-index resizable-column-header"
                        role="columnheader"
                      >
                        <span class="column-header-label">#</span>
                        <button
                          class="column-resize-handle {resizingParsedColumn ===
                          'index'
                            ? 'active'
                            : ''}"
                          type="button"
                          aria-label="Resize index column"
                          onpointerdown={(event) =>
                            startParsedColumnResize(event, "index")}
                          ondblclick={() => resetParsedColumnWidth("index")}
                        ></button>
                      </div>
                    {/if}
                    {#if currentParsedShowsTypeColumn}
                      <div
                        class="virtual-data-cell col-type resizable-column-header"
                        role="columnheader"
                      >
                        <span class="column-header-label">Type</span>
                        <button
                          class="column-resize-handle {resizingParsedColumn ===
                          'type'
                            ? 'active'
                            : ''}"
                          type="button"
                          aria-label="Resize type column"
                          onpointerdown={(event) =>
                            startParsedColumnResize(event, "type")}
                          ondblclick={() => resetParsedColumnWidth("type")}
                        ></button>
                      </div>
                    {/if}
                    {#if currentParsedShowsNameColumn}
                      <div
                        class="virtual-data-cell col-key resizable-column-header"
                        role="columnheader"
                      >
                        <span class="column-header-label">Name</span>
                        <button
                          class="column-resize-handle {resizingParsedColumn ===
                          'name'
                            ? 'active'
                            : ''}"
                          type="button"
                          aria-label="Resize name column"
                          onpointerdown={(event) =>
                            startParsedColumnResize(event, "name")}
                          ondblclick={() => resetParsedColumnWidth("name")}
                        ></button>
                      </div>
                    {/if}
                    <div
                      class="virtual-data-cell col-val resizable-column-header"
                      role="columnheader"
                    >
                      <span class="column-header-label">Value</span>
                      <button
                        class="column-resize-handle {resizingParsedColumn ===
                        'value'
                          ? 'active'
                          : ''}"
                        type="button"
                        aria-label="Resize value column"
                        onpointerdown={(event) =>
                          startParsedColumnResize(event, "value")}
                        ondblclick={() => resetParsedColumnWidth("value")}
                      ></button>
                    </div>
                  </div>
                  {#if filteredParsedEntries.length === 0}
                    <div class="table-empty-row virtual-empty-row">
                      {normalizedParsedSearch
                        ? "No matching entries found."
                        : "No entries found."}
                    </div>
                  {:else}
                    <VirtualList
                      class="virtual-data-body"
                      items={filteredParsedEntries}
                      height="auto"
                      itemHeight={44}
                    >
                      {#snippet children(item)}
                        {@const entry = item.entry}
                        {@const rowIndex = item.rowIndex}
                        {@const key = entry[0]}
                        {@const value = entry[1]}
                        {@const rawValue = getParsedRawValue(value)}
                        {@const keyName = getParsedEntryKey(key, value)}
                        <!-- svelte-ignore a11y_click_events_have_key_events -->
                        <!-- svelte-ignore a11y_interactive_supports_focus -->
                        <div
                          class="virtual-data-row {isSelectedParsedField(value)
                            ? 'selected-data-row'
                            : ''}"
                          role="row"
                          onclick={() => handleParsedCellClick(value)}
                        >
                          {#if currentParsedPageIsArray}
                            <div
                              class="virtual-data-cell col-index"
                              role="cell"
                            >
                              {rowIndex}
                            </div>
                          {/if}
                          {#if currentParsedShowsTypeColumn}
                            <div class="virtual-data-cell col-type" role="cell">
                              {getParsedType(value)}
                            </div>
                          {/if}
                          {#if currentParsedShowsNameColumn}
                            <div class="virtual-data-cell col-key" role="cell">
                              {keyName}
                            </div>
                          {/if}
                          <div
                            class="virtual-data-cell col-val {isParsedField(
                              value,
                            )
                              ? 'clickable-cell'
                              : ''} {getParsedValueClass(
                              value,
                            )} {isSelectedParsedField(value)
                              ? 'selected-data-cell'
                              : ''}"
                            role="cell"
                          >
                            {#if isLinkedParsedValue(rawValue)}
                              <button
                                class="value-link"
                                type="button"
                                onclick={(event) => {
                                  event.stopPropagation();
                                  handleParsedCellLink(keyName, value);
                                }}
                              >
                                {formatParsedValue(value)}
                              </button>
                            {:else}
                              {formatParsedValue(value)}
                            {/if}
                          </div>
                        </div>
                      {/snippet}
                    </VirtualList>
                  {/if}
                </div>
              {/if}
            {/if}
          </div>
        {/if}
      </main>

      {#if selectedTable && showHexPane}
        <!-- RESIZER FOR MAIN SPLIT -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <div
          class="split-resizer hex-resizer {isResizingHex ? 'active' : ''}"
          style="right: {hexPaneFixedWidth}px;"
          onmousedown={startResizeMain}
          role="separator"
          aria-orientation="vertical"
          aria-label="Resize hex pane"
        ></div>

        <HexPane
          bind:this={hexPaneRef}
          {rawBytes}
          {dataLoading}
          {selectedByteRange}
        />
      {/if}
    </div>
  </div>
{/if}
