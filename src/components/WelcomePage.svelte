<script lang="ts">
  import { showTitlebarContextMenu } from "../lib/titlebarContextMenu";
  import WindowControls from "./WindowControls.svelte";

  interface Props {
    theme: string;
    loading: boolean;
    isDraggingOver: boolean;
    errorMsg: string | null;
    onOpenFont: () => void;
    onToggleTheme: () => void;
  }

  let {
    theme,
    loading,
    isDraggingOver,
    errorMsg,
    onOpenFont,
    onToggleTheme,
  }: Props = $props();
</script>

<div class="welcome-shell">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <header class="welcome-header" oncontextmenu={showTitlebarContextMenu}>
    <div class="welcome-drag-region" data-tauri-drag-region></div>
    <button
      class="button button-outline button-icon theme-toggle"
      onclick={onToggleTheme}
      title="Toggle Theme"
      aria-label="Toggle theme"
      type="button"
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
    <WindowControls />
  </header>

  <main class="welcome-main">
    <div class="welcome-content">
      <!-- App branding -->
      <div class="branding">
        <img src="/logo.png" alt="Fontabex" width="32" height="32" />
        <h1 class="app-title">fontabex</h1>
        <p class="app-tagline">Explore OpenType font tables</p>
      </div>

      <!-- Drop zone -->
      <section
        class="drop-zone {isDraggingOver ? 'dragging' : ''}"
        aria-label="Open font file"
      >
        {#if loading}
          <div class="loading-state">
            <div class="spinner-ring">
              <svg viewBox="0 0 50 50">
                <circle
                  cx="25"
                  cy="25"
                  r="20"
                  fill="none"
                  stroke-width="3"
                  class="spinner-track"
                />
                <circle
                  cx="25"
                  cy="25"
                  r="20"
                  fill="none"
                  stroke-width="3"
                  stroke-linecap="round"
                  class="spinner-arc"
                />
              </svg>
            </div>
            <div class="loading-text">
              <h2>Parsing font&hellip;</h2>
              <p>Reading table data, this won't take long.</p>
            </div>
          </div>
        {:else}
          <div class="drop-icon" aria-hidden="true">
            <svg
              width="44"
              height="44"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              {#if isDraggingOver}
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                <polyline points="7 10 12 15 17 10" />
                <line x1="12" y1="15" x2="12" y2="3" />
              {:else}
                <path
                  d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8Z"
                />
                <path d="M14 2v6h6" />
                <path d="M9 15h6" />
                <path d="M12 12v6" />
              {/if}
            </svg>
          </div>

          <div class="drop-copy">
            <h2>
              {isDraggingOver ? "Drop to open" : "Drag & drop a font file"}
            </h2>
            <p>
              Supports <span class="format-tag">.ttf</span> and
              <span class="format-tag">.otf</span> files
            </p>
          </div>

          <div class="drop-divider">
            <span class="divider-line"></span>
            <span class="divider-text">or</span>
            <span class="divider-line"></span>
          </div>

          <button class="open-button" onclick={onOpenFont} type="button">
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
              <path
                d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"
              />
            </svg>
            Browse Files
          </button>
        {/if}

        {#if errorMsg}
          <div class="error-bubble" role="alert">
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
              <circle cx="12" cy="12" r="10" />
              <line x1="12" y1="8" x2="12" y2="12" />
              <line x1="12" y1="16" x2="12.01" y2="16" />
            </svg>
            {errorMsg}
          </div>
        {/if}
      </section>
    </div>
  </main>
</div>

<style>
  /* ── Shell ────────────────────────────────────── */
  .welcome-shell {
    position: relative;
    min-height: 100vh;
    width: 100vw;
    display: flex;
    flex-direction: column;
    background: var(--bg-color);
    color: var(--text-color);
    overflow: hidden;
  }

  /* ── Header ──────────────────────────────────── */
  .welcome-header {
    position: relative;
    z-index: 2;
    height: 42px;
    display: flex;
    gap: 16px;
    align-items: center;
    justify-content: flex-end;
    padding: 0 0 0 1.25rem;
    user-select: none;
  }

  .welcome-drag-region {
    align-self: stretch;
    flex: 1 1 auto;
    min-width: 1rem;
  }

  .theme-toggle {
    flex-shrink: 0;
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    background: color-mix(in srgb, var(--card-bg) 60%, transparent);
    transition: all 0.2s ease;
  }

  .theme-toggle:hover {
    background: color-mix(in srgb, var(--card-bg) 85%, transparent);
  }

  /* ── Main ─────────────────────────────────────── */
  .welcome-main {
    position: relative;
    z-index: 1;
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0 1.5rem 3rem;
  }

  .welcome-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1.75rem;
    width: 100%;
    max-width: 460px;
  }

  /* ── Branding ─────────────────────────────────── */
  .branding {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    text-align: center;
  }

  .app-title {
    margin: 0;
    font-size: 1.65rem;
    font-weight: 750;
    letter-spacing: -0.03em;
    color: var(--primary);
    animation: titleEntrance 0.6s cubic-bezier(0.34, 1.56, 0.64, 1) 0.1s both;
  }

  @keyframes titleEntrance {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .app-tagline {
    margin: 0;
    font-size: 0.88rem;
    color: color-mix(in srgb, var(--text-color) 55%, transparent);
    font-weight: 450;
    animation: titleEntrance 0.6s cubic-bezier(0.34, 1.56, 0.64, 1) 0.2s both;
  }

  /* ── Drop Zone ────────────────────────────────── */
  .drop-zone {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1.25rem;
    padding: 2.5rem 2rem;
    border: 1.5px dashed color-mix(in srgb, var(--text-color) 16%, transparent);
    border-radius: 16px;
    background: color-mix(in srgb, var(--card-bg) 55%, transparent);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    text-align: center;
    transition:
      border-color 0.25s ease,
      background-color 0.25s ease,
      box-shadow 0.25s ease,
      transform 0.25s ease;
    animation: zoneEntrance 0.6s cubic-bezier(0.34, 1.56, 0.64, 1) 0.25s both;
  }

  @keyframes zoneEntrance {
    from {
      opacity: 0;
      transform: translateY(16px) scale(0.97);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .drop-zone:hover {
    border-color: color-mix(in srgb, var(--primary) 40%, transparent);
    background: color-mix(in srgb, var(--card-bg) 70%, transparent);
    box-shadow:
      0 4px 24px color-mix(in srgb, var(--primary) 6%, transparent),
      0 0 0 1px color-mix(in srgb, var(--primary) 8%, transparent);
  }

  .drop-zone.dragging {
    border-color: var(--primary);
    border-style: solid;
    background: color-mix(in srgb, var(--primary) 8%, var(--card-bg));
    box-shadow:
      0 0 0 4px color-mix(in srgb, var(--primary) 12%, transparent),
      0 8px 32px color-mix(in srgb, var(--primary) 10%, transparent);
    transform: scale(1.01);
  }

  /* ── Drop Icon ────────────────────────────────── */
  .drop-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 72px;
    height: 72px;
    border-radius: 18px;
    background: color-mix(in srgb, var(--primary) 8%, transparent);
    color: var(--primary);
    transition: all 0.3s ease;
  }

  .drop-zone:hover .drop-icon {
    background: color-mix(in srgb, var(--primary) 12%, transparent);
    transform: translateY(-2px);
  }

  .drop-zone.dragging .drop-icon {
    background: color-mix(in srgb, var(--primary) 15%, transparent);
    transform: translateY(-4px) scale(1.05);
    animation: iconBounce 0.6s ease-in-out infinite alternate;
  }

  @keyframes iconBounce {
    from {
      transform: translateY(-4px) scale(1.05);
    }
    to {
      transform: translateY(-8px) scale(1.08);
    }
  }

  /* ── Copy ──────────────────────────────────────── */
  .drop-copy h2,
  .loading-state h2 {
    margin: 0;
    font-size: 1.05rem;
    font-weight: 650;
    line-height: 1.3;
  }

  .drop-copy p,
  .loading-state p {
    margin: 0.35rem 0 0;
    color: color-mix(in srgb, var(--text-color) 50%, transparent);
    font-size: 0.84rem;
    line-height: 1.5;
  }

  .format-tag {
    display: inline-block;
    padding: 0.1em 0.4em;
    border-radius: 4px;
    background: color-mix(in srgb, var(--text-color) 6%, transparent);
    font-family: var(--font-mono);
    font-size: 0.78rem;
    font-weight: 550;
    letter-spacing: 0.01em;
  }

  /* ── Divider ───────────────────────────────────── */
  .drop-divider {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    width: 60%;
  }

  .divider-line {
    flex: 1;
    height: 1px;
    background: color-mix(in srgb, var(--text-color) 12%, transparent);
  }

  .divider-text {
    font-size: 0.75rem;
    font-weight: 500;
    color: color-mix(in srgb, var(--text-color) 35%, transparent);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  /* ── Open Button ───────────────────────────────── */
  .open-button {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    min-height: 40px;
    padding: 0 1.35rem;
    border: 0;
    border-radius: 10px;
    background-color: var(--primary);
    color: #ffffff;
    font-family: var(--font-sans);
    font-size: 0.88rem;
    font-weight: 600;
    cursor: pointer;
    transition:
      transform 0.2s ease,
      box-shadow 0.2s ease,
      filter 0.2s ease;
    box-shadow:
      0 2px 8px color-mix(in srgb, var(--primary) 25%, transparent),
      0 0 0 0 color-mix(in srgb, var(--primary) 0%, transparent);
  }

  .open-button:hover {
    transform: translateY(-1px);
    box-shadow:
      0 4px 16px color-mix(in srgb, var(--primary) 30%, transparent),
      0 0 0 2px color-mix(in srgb, var(--primary) 12%, transparent);
    filter: brightness(1.06);
  }

  .open-button:active {
    transform: translateY(0);
    box-shadow: 0 1px 4px color-mix(in srgb, var(--primary) 20%, transparent);
  }

  /* ── Loading State ─────────────────────────────── */
  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1.25rem;
  }

  .spinner-ring {
    width: 48px;
    height: 48px;
  }

  .spinner-ring svg {
    width: 100%;
    height: 100%;
    animation: spinnerRotate 1.4s linear infinite;
  }

  .spinner-track {
    stroke: color-mix(in srgb, var(--primary) 14%, transparent);
  }

  .spinner-arc {
    stroke: var(--primary);
    stroke-dasharray: 80, 126;
    stroke-dashoffset: 0;
    animation: spinnerDash 1.4s ease-in-out infinite;
  }

  @keyframes spinnerRotate {
    to {
      transform: rotate(360deg);
    }
  }

  @keyframes spinnerDash {
    0% {
      stroke-dasharray: 1, 126;
      stroke-dashoffset: 0;
    }
    50% {
      stroke-dasharray: 80, 126;
      stroke-dashoffset: -35;
    }
    100% {
      stroke-dasharray: 80, 126;
      stroke-dashoffset: -124;
    }
  }

  .loading-text {
    text-align: center;
  }

  /* ── Error Bubble ──────────────────────────────── */
  .error-bubble {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    width: fit-content;
    max-width: 100%;
    margin-top: 0.25rem;
    padding: 0.7rem 2rem;
    border: 1px solid color-mix(in srgb, var(--error-color) 20%, transparent);
    border-radius: 10px;
    background: color-mix(in srgb, var(--error-color) 6%, transparent);
    color: var(--error-color);
    font-size: 0.82rem;
    overflow-wrap: anywhere;
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
  }

  .error-bubble svg {
    flex-shrink: 0;
    margin-top: 1px;
  }

  /* ── Responsive ────────────────────────────────── */
  @media (max-width: 520px) {
    .welcome-header {
      padding: 0 0 0 0.85rem;
    }

    .welcome-main {
      padding: 0 1rem 2rem;
    }

    .drop-zone {
      padding: 2rem 1.25rem;
      border-radius: 12px;
    }

    .branding {
      gap: 0.35rem;
    }

    .app-title {
      font-size: 1.4rem;
    }
  }
</style>
