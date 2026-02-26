<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";

  interface Track {
    title: string;
    artist: string;
    album: string;
    is_playing: boolean;
  }

  let track = $state<Track | null>(null);
  let intervalId: ReturnType<typeof setInterval> | undefined;
  let fetching = false;

  const fetchTrack = async () => {
    if (fetching) return;
    fetching = true;
    try {
      track = await invoke<Track | null>("get_current_track");
    } catch (e) {
      console.error("Failed to fetch track:", e);
    } finally {
      fetching = false;
    }
  };

  onMount(() => {
    fetchTrack();
    intervalId = setInterval(fetchTrack, 3000);
  });

  onDestroy(() => {
    if (intervalId !== undefined) clearInterval(intervalId);
  });
</script>

<div class="panel">
  <!-- Specular highlight — top surface reflection -->
  <div class="specular" aria-hidden="true"></div>

  <header class="header">
    <svg class="header-icon" width="14" height="14" viewBox="0 0 14 14" fill="none">
      <path d="M5 10.5V3.5L12 2V9" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
      <circle cx="3.5" cy="10.5" r="1.5" fill="currentColor"/>
      <circle cx="10.5" cy="9" r="1.5" fill="currentColor"/>
    </svg>
    <span class="header-title">Enhanced Music</span>
  </header>

  <div class="content">
    {#if track}
      <div class="track-card">
        <div class="card-specular" aria-hidden="true"></div>
        <span class="label" class:playing={track.is_playing} class:paused={!track.is_playing}>
          <span class="label-dot" aria-hidden="true"></span>
          {track.is_playing ? "Now Playing" : "Paused"}
        </span>
        <p class="track-title">{track.title}</p>
        <div class="track-meta">
          <span class="track-artist">{track.artist}</span>
          <span class="dot">·</span>
          <span class="track-album">{track.album}</span>
        </div>
      </div>
    {:else}
      <div class="track-card idle">
        <div class="card-specular" aria-hidden="true"></div>
        <svg class="idle-icon" width="32" height="32" viewBox="0 0 32 32" fill="none">
          <path d="M11 24V9L26 6V21" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          <circle cx="8" cy="24" r="3" fill="currentColor"/>
          <circle cx="23" cy="21" r="3" fill="currentColor"/>
        </svg>
        <p class="idle-text">Nothing playing</p>
        <p class="idle-sub">Open Apple Music to get started</p>
      </div>
    {/if}
  </div>
</div>

<style>
  /* ─── Glass design tokens — light mode ───────────────── */
  :root {
    --glass-tint:       rgba(255, 255, 255, 0.30);
    --glass-border:     rgba(0, 0, 0, 0.10);
    --glass-inset-top:  rgba(255, 255, 255, 0.70);
    --glass-shadow:     rgba(0, 0, 0, 0.12);
    --card-bg:          rgba(255, 255, 255, 0.45);
    --card-border:      rgba(0, 0, 0, 0.08);
    --card-inset-top:   rgba(255, 255, 255, 0.80);
    --radius-panel:     22px;
    --radius-card:      16px;
    --text-primary:     rgba(0, 0, 0, 0.85);
    --text-secondary:   rgba(0, 0, 0, 0.50);
    --text-tertiary:    rgba(0, 0, 0, 0.30);
    --specular-start:   rgba(255, 255, 255, 0.55);
    --specular-mid:     rgba(255, 255, 255, 0.15);
    --accent-playing:   rgba(48, 209, 88,  0.90);
    --accent-paused:    rgba(255, 149, 0,  0.90);
  }

  /* ─── Dark mode overrides ─────────────────────────────── */
  @media (prefers-color-scheme: dark) {
    :root {
      --glass-tint:       rgba(30, 30, 36, 0.35);
      --glass-border:     rgba(255, 255, 255, 0.18);
      --glass-inset-top:  rgba(255, 255, 255, 0.28);
      --glass-shadow:     rgba(0, 0, 0, 0.30);
      --card-bg:          rgba(255, 255, 255, 0.09);
      --card-border:      rgba(255, 255, 255, 0.16);
      --card-inset-top:   rgba(255, 255, 255, 0.22);
      --text-primary:     rgba(255, 255, 255, 0.92);
      --text-secondary:   rgba(255, 255, 255, 0.52);
      --text-tertiary:    rgba(255, 255, 255, 0.28);
      --specular-start:   rgba(255, 255, 255, 0.14);
      --specular-mid:     rgba(255, 255, 255, 0.04);
      --accent-playing:   rgba(48, 209, 88,  0.95);
      --accent-paused:    rgba(255, 149, 0,  0.95);
    }
  }

  /* ─── Panel ───────────────────────────────────────────── */
  .panel {
    position: relative;
    width: 100%;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--glass-tint);
    border-radius: var(--radius-panel);
    border: 0.5px solid var(--glass-border);
    overflow: hidden;
    font-family: -apple-system, "SF Pro Display", "SF Pro Text", system-ui, sans-serif;
    -webkit-font-smoothing: antialiased;
  }

  /* ─── Top specular highlight (Liquid Glass signature) ─── */
  .specular {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: linear-gradient(
      180deg,
      var(--specular-start) 0%,
      var(--specular-mid)   28%,
      transparent           55%
    );
    pointer-events: none;
    z-index: 0;
  }

  /* ─── Header ──────────────────────────────────────────── */
  .header {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 16px 18px 10px;
    position: relative;
    z-index: 1;
    color: var(--text-secondary);
  }

  .header-icon {
    flex-shrink: 0;
    opacity: 0.7;
  }

  .header-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    letter-spacing: 0.03em;
  }

  /* ─── Content ─────────────────────────────────────────── */
  .content {
    flex: 1;
    padding: 4px 12px 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    position: relative;
    z-index: 1;
  }

  /* ─── Track card ──────────────────────────────────────── */
  .track-card {
    position: relative;
    overflow: hidden;
    border-radius: var(--radius-card);
    border: 0.5px solid var(--card-border);
    background: var(--card-bg);
    padding: 16px 16px 18px;
    box-shadow:
      inset 0 1px 0 var(--card-inset-top),
      inset 0 -1px 0 rgba(0, 0, 0, 0.06),
      0 6px 28px var(--glass-shadow);
  }

  /* Per-card specular */
  .card-specular {
    position: absolute;
    top: 0; left: 0; right: 0;
    height: 55%;
    background: linear-gradient(
      180deg,
      var(--specular-start) 0%,
      transparent           100%
    );
    border-radius: var(--radius-card) var(--radius-card) 0 0;
    pointer-events: none;
  }

  .label {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    align-self: flex-start;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.02em;
    padding: 3px 9px 3px 7px;
    border-radius: 20px;
    margin-bottom: 12px;
  }

  .label.playing {
    color: var(--accent-playing);
    background: color-mix(in srgb, var(--accent-playing) 15%, transparent);
  }
  .label.paused {
    color: var(--accent-paused);
    background: color-mix(in srgb, var(--accent-paused) 15%, transparent);
  }

  .label-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .label.playing .label-dot {
    background: var(--accent-playing);
    animation: pulse 2s ease-in-out infinite;
  }

  .label.paused .label-dot {
    background: var(--accent-paused);
    animation: none;
    opacity: 0.8;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1;   transform: scale(1); }
    50%       { opacity: 0.4; transform: scale(0.75); }
  }

  .track-title {
    font-size: 20px;
    font-weight: 700;
    color: var(--text-primary);
    line-height: 1.2;
    margin-bottom: 8px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .track-meta {
    display: flex;
    align-items: center;
    gap: 5px;
    overflow: hidden;
  }

  .track-artist,
  .track-album {
    font-size: 13px;
    font-weight: 400;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .dot {
    font-size: 13px;
    color: var(--text-tertiary);
    flex-shrink: 0;
  }

  /* ─── Idle state ──────────────────────────────────────── */
  .track-card.idle {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 36px 16px;
    gap: 8px;
  }

  .idle-icon {
    color: var(--text-tertiary);
    margin-bottom: 6px;
  }

  .idle-text {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .idle-sub {
    font-size: 12px;
    color: var(--text-tertiary);
    text-align: center;
    line-height: 1.5;
  }
</style>
