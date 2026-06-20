<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  const appWindow = getCurrentWindow();
  let isMaximized = $state(false);

  async function refreshMaximizedState() {
    isMaximized = await appWindow.isMaximized();
  }

  function minimizeWindow() {
    void appWindow.minimize();
  }

  async function toggleMaximizeWindow() {
    await appWindow.toggleMaximize();
    await refreshMaximizedState();
  }

  function closeWindow() {
    void appWindow.close();
  }

  onMount(() => {
    void refreshMaximizedState();

    const unlisten = appWindow.onResized(() => {
      void refreshMaximizedState();
    });

    return () => {
      void unlisten.then((off) => off());
    };
  });
</script>

<div class="window-controls" role="group" aria-label="Window controls">
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
    title={isMaximized ? "Restore" : "Maximize"}
    aria-label={isMaximized ? "Restore window" : "Maximize window"}
  >
    {#if isMaximized}
      <svg
        width="24"
        height="24"
        viewBox="0 0 24 24"
        fill="currentColor"
        aria-hidden="true"
      >
        <path d="M9 7h8v8h-2V9H9V7Z" />
        <path d="M7 9h8v8H7V9Zm1 1v6h6v-6H8Z" fill-rule="evenodd" />
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
      <path
        fill-rule="evenodd"
        d="m7.707 7 9.193 9.193-.707.707L7 7.707 7.707 7Z"
      />
      <path
        fill-rule="evenodd"
        d="M16.193 7 7 16.193l.707.707L16.9 7.707 16.193 7Z"
      />
    </svg>
  </button>
</div>

<style>
  .window-controls {
    align-self: stretch;
    display: flex;
    flex-shrink: 0;
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
    transition: background 0.15s ease;
  }

  .window-control:hover {
    background: color-mix(in srgb, var(--text-color) 10%, transparent);
  }

  .window-control-close:hover {
    background: #dc2626;
    color: white;
  }
</style>
