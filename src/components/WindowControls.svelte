<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  const appWindow = getCurrentWindow();
  let isMaximized = $state(false);
  let isFocused = $state(true);
  let isTogglingMaximize = $state(false);

  async function refreshMaximizedState() {
    try {
      isMaximized = await appWindow.isMaximized();
    } catch (error) {
      console.warn("Failed to read window maximized state", error);
    }
  }

  function minimizeWindow() {
    void appWindow.minimize();
  }

  async function toggleMaximizeWindow() {
    if (isTogglingMaximize) return;

    try {
      isTogglingMaximize = true;
      await appWindow.toggleMaximize();
      await refreshMaximizedState();

      window.setTimeout(() => {
        void refreshMaximizedState();
      }, 120);
    } catch (error) {
      console.warn("Failed to toggle window maximized state", error);
    } finally {
      isTogglingMaximize = false;
    }
  }

  function closeWindow() {
    void appWindow.close();
  }

  onMount(() => {
    void refreshMaximizedState();

    const unlistenResize = appWindow.onResized(() => {
      void refreshMaximizedState();
    });
    const unlistenFocus = appWindow.onFocusChanged(({ payload }) => {
      isFocused = payload;
    });

    return () => {
      void unlistenResize.then((off) => off());
      void unlistenFocus.then((off) => off());
    };
  });
</script>

<div
  class="window-controls"
  class:window-inactive={!isFocused}
  role="group"
  aria-label="Window controls"
>
  <button
    class="window-control"
    type="button"
    onclick={minimizeWindow}
    title="Minimize"
    aria-label="Minimize window"
  >
    <svg
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="currentColor"
      aria-hidden="true"
    >
      <rect x="7" y="12" width="10" height="1" />
    </svg>
  </button>
  <button
    class="window-control"
    type="button"
    onclick={toggleMaximizeWindow}
    disabled={isTogglingMaximize}
    title={isMaximized ? "Restore" : "Maximize"}
    aria-label={isMaximized ? "Restore window" : "Maximize window"}
    aria-pressed={isMaximized}
  >
    {#if isMaximized}
      <svg
        width="24"
        height="24"
        viewBox="0 0 24 24"
        fill="currentColor"
        aria-hidden="true"
      >
        <path d="M9 7h8v8h-2v-1h1v-6h-6v1h-1v-2Z" />
        <path fill-rule="evenodd" d="M7 9h8v8H7V9Zm1 1v6h6v-6H8Z" />
      </svg>
    {:else}
      <svg
        width="24"
        height="24"
        viewBox="0 0 24 24"
        fill="currentColor"
        aria-hidden="true"
      >
        <path fill-rule="evenodd" d="M7 7h10v10H7V7Zm1 1v8h8V8H8Z" />
      </svg>
    {/if}
  </button>
  <button
    class="window-control window-control-close"
    type="button"
    onclick={closeWindow}
    title="Close"
    aria-label="Close window"
  >
    <svg
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="currentColor"
      aria-hidden="true"
    >
      <path d="m7.707 7 9.193 9.193-.707.707L7 7.707 7.707 7Z" />
      <path d="M16.193 7 7 16.193l.707.707L16.9 7.707 16.193 7Z" />
    </svg>
  </button>
</div>

<style>
  .window-controls {
    align-self: stretch;
    display: flex;
    flex-shrink: 0;
    min-height: var(--toolbar-height);
  }

  .window-control {
    appearance: none;
    border: 0;
    border-radius: 0;
    background: transparent;
    color: var(--text-color);
    width: 46px;
    height: 100%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    outline: none;
    cursor: default;
    opacity: 1;
    transition:
      background-color 0.15s ease,
      color 0.15s ease,
      opacity 0.15s ease;
  }

  .window-inactive .window-control {
    opacity: 0.62;
  }

  .window-control:hover {
    background: color-mix(in srgb, var(--text-color) 10%, transparent);
  }

  .window-control:active {
    background: color-mix(in srgb, var(--text-color) 16%, transparent);
  }

  .window-control:focus-visible {
    box-shadow: inset 0 0 0 2px
      color-mix(in srgb, var(--primary) 64%, transparent);
  }

  .window-control:disabled {
    pointer-events: none;
    opacity: 0.4;
  }

  .window-control-close:hover {
    background: #dc2626;
    color: white;
  }

  .window-control-close:active {
    background: #b91c1c;
    color: white;
  }
</style>
