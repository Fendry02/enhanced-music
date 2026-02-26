<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy, untrack } from "svelte";

  interface Track {
    title: string;
    artist: string;
    album: string;
    is_playing: boolean;
  }

  interface AlbumInfo {
    release_year: string;
    genre: string;
    context: string;
    notable_fact: string;
  }

  interface LyricsAnalysis {
    interpretation: string;
  }

  let track          = $state<Track | null>(null);
  let artwork        = $state<string | null>(null);
  let albumInfo      = $state<AlbumInfo | null>(null);
  let albumLoading   = $state(false);
  let lyricsAnalysis = $state<LyricsAnalysis | null>(null);
  let lyricsLoading  = $state(false);

  let intervalId: ReturnType<typeof setInterval> | undefined;
  let fetching = false;

  // Generation counters: incremented on each track change so that
  // responses from a previous track are silently dropped.
  let artworkGen = 0;
  let albumGen   = 0;
  let lyricsGen  = 0;

  // Stable key — changes only when the track itself changes, not on play/pause
  const trackId = $derived(track ? `${track.title}|||${track.artist}` : null);

  $effect(() => {
    if (!trackId) { artwork = null; return; }
    const gen    = ++artworkGen;
    const title  = untrack(() => track?.title  ?? "");
    const artist = untrack(() => track?.artist ?? "");
    invoke<string | null>("get_artwork", { title, artist })
      .then(d  => { if (artworkGen === gen) artwork = d ?? null; })
      .catch(() => { if (artworkGen === gen) artwork = null; });
  });

  $effect(() => {
    if (!trackId) { albumInfo = null; albumLoading = false; return; }
    const gen    = ++albumGen;
    const album  = untrack(() => track?.album  ?? "");
    const artist = untrack(() => track?.artist ?? "");
    albumInfo    = null;
    albumLoading = true;
    invoke<AlbumInfo | null>("get_album_info", { album, artist })
      .then(d  => { if (albumGen === gen) { albumInfo = d; albumLoading = false; } })
      .catch(e => { if (albumGen === gen) { console.error("get_album_info:", e); albumInfo = null; albumLoading = false; } });
  });

  $effect(() => {
    if (!trackId) { lyricsAnalysis = null; lyricsLoading = false; return; }
    const gen    = ++lyricsGen;
    const title  = untrack(() => track?.title  ?? "");
    const artist = untrack(() => track?.artist ?? "");
    lyricsAnalysis = null;
    lyricsLoading  = true;
    invoke<LyricsAnalysis | null>("get_lyrics_analysis", { title, artist })
      .then(d  => { if (lyricsGen === gen) { lyricsAnalysis = d; lyricsLoading = false; } })
      .catch(e => { if (lyricsGen === gen) { console.error("get_lyrics_analysis:", e); lyricsAnalysis = null; lyricsLoading = false; } });
  });

  const fetchTrack = async () => {
    if (fetching) return;
    fetching = true;
    try {
      track = await invoke<Track | null>("get_current_track");
    } catch (e) {
      console.error("get_current_track:", e);
    } finally {
      fetching = false;
    }
  };

  onMount(() => {
    fetchTrack();
    intervalId = setInterval(fetchTrack, 3000);
  });

  onDestroy(() => clearInterval(intervalId));
</script>

<div class="panel">
  <div class="panel-glow" aria-hidden="true"></div>

  <header class="header">
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
      <path d="M5 10.5V3.5L12 2V9" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
      <circle cx="3.5" cy="10.5" r="1.5" fill="currentColor"/>
      <circle cx="10.5" cy="9"    r="1.5" fill="currentColor"/>
    </svg>
    <span>Enhanced Music</span>
  </header>

  <div class="content">
    {#if track}

      <!-- ── Track ─────────────────────────────────────────── -->
      <div class="card">
        <div class="card-glow" aria-hidden="true"></div>
        <div class="track-row">
          <div class="artwork-wrap">
            {#if artwork}
              <img class="artwork" src={artwork} alt="Album artwork" />
            {:else}
              <div class="artwork-empty" aria-hidden="true">
                <svg width="22" height="22" viewBox="0 0 22 22" fill="none">
                  <path d="M8 16.5V7L18 5V14" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>
                  <circle cx="5.5"  cy="16.5" r="2.5" fill="currentColor"/>
                  <circle cx="15.5" cy="14"   r="2.5" fill="currentColor"/>
                </svg>
              </div>
            {/if}
          </div>

          <div class="track-info">
            <span class="pill" class:playing={track.is_playing} class:paused={!track.is_playing}>
              <span class="pill-dot" aria-hidden="true"></span>
              {track.is_playing ? "Now Playing" : "Paused"}
            </span>
            <p class="track-title">{track.title}</p>
            <p class="track-sub">
              <span class="track-artist">{track.artist}</span>
              <span class="sep" aria-hidden="true">·</span>
              <span class="track-album">{track.album}</span>
            </p>
            {#if albumInfo?.release_year || albumInfo?.genre}
              <div class="badges track-badges">
                {#if albumInfo.release_year}<span class="badge">{albumInfo.release_year}</span>{/if}
                {#if albumInfo.genre}<span class="badge">{albumInfo.genre}</span>{/if}
              </div>
            {:else if albumLoading}
              <div class="badges track-badges">
                <div class="skel pill-s"></div>
                <div class="skel pill-s wide"></div>
              </div>
            {/if}
          </div>
        </div>
      </div>

      <!-- ── Album ─────────────────────────────────────────── -->
      <div class="card">
        <div class="card-glow" aria-hidden="true"></div>
        <div class="section-head">
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true">
            <circle cx="6" cy="6" r="5" stroke="currentColor" stroke-width="1.2"/>
            <circle cx="6" cy="6" r="1.5" fill="currentColor"/>
          </svg>
          <span class="section-label">Album</span>
        </div>

        {#if albumLoading}
          <div class="skels">
            <div class="skel line"></div>
            <div class="skel line short"></div>
            <div class="skel line mid"></div>
          </div>
        {:else if albumInfo?.context}
          <p class="body-text">{albumInfo.context}</p>
        {:else}
          <p class="empty">Informations indisponibles</p>
        {/if}
      </div>

      <!-- ── Anecdote ───────────────────────────────────────── -->
      {#if albumLoading || albumInfo?.notable_fact}
        <div class="card card-fact">
          <div class="card-glow" aria-hidden="true"></div>
          <div class="section-head">
            <span class="fact-icon" aria-hidden="true">✦</span>
            <span class="section-label">Anecdote</span>
          </div>
          {#if albumLoading}
            <div class="skels">
              <div class="skel line"></div>
              <div class="skel line mid"></div>
              <div class="skel line short"></div>
            </div>
          {:else}
            <p class="body-text">{albumInfo!.notable_fact}</p>
          {/if}
        </div>
      {/if}

      <!-- ── Paroles ────────────────────────────────────────── -->
      <div class="card">
        <div class="card-glow" aria-hidden="true"></div>
        <div class="section-head">
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true">
            <path d="M4 9.5V4L10 3V8" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
            <circle cx="2.5" cy="9.5" r="1.5" fill="currentColor"/>
            <circle cx="8.5" cy="8"   r="1.5" fill="currentColor"/>
          </svg>
          <span class="section-label">Paroles</span>
        </div>

        {#if lyricsLoading}
          <div class="skels">
            <div class="skel line"></div>
            <div class="skel line mid"></div>
            <div class="skel line short"></div>
            <div class="skel line"></div>
            <div class="skel line mid"></div>
          </div>
        {:else if lyricsAnalysis?.interpretation}
          <p class="body-text italic">{lyricsAnalysis.interpretation}</p>
        {:else}
          <p class="empty">Analyse indisponible</p>
        {/if}
      </div>

    {:else}

      <!-- ── Idle ───────────────────────────────────────────── -->
      <div class="card idle">
        <div class="card-glow" aria-hidden="true"></div>
        <svg width="32" height="32" viewBox="0 0 32 32" fill="none" aria-hidden="true" class="idle-icon">
          <path d="M11 24V9L26 6V21" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          <circle cx="8"  cy="24" r="3" fill="currentColor"/>
          <circle cx="23" cy="21" r="3" fill="currentColor"/>
        </svg>
        <p class="idle-title">Nothing playing</p>
        <p class="idle-sub">Open Apple Music to get started</p>
      </div>

    {/if}
  </div>
</div>

<style>
  /* ── Design tokens ── light ──────────────────────────────── */
  :root {
    --glass-bg:      rgba(255, 255, 255, 0.52);
    --glass-border:  rgba(0, 0, 0, 0.11);
    --glass-shine:   rgba(255, 255, 255, 0.82);
    --glass-shadow:  rgba(0, 0, 0, 0.16);
    --card-bg:       rgba(255, 255, 255, 0.76);
    --card-border:   rgba(0, 0, 0, 0.09);
    --card-shine:    rgba(255, 255, 255, 0.92);
    --fact-card-bg:  rgba(255, 193, 7, 0.08);
    --fact-card-border: rgba(255, 193, 7, 0.22);
    --text-1:        rgba(0, 0, 0, 0.92);
    --text-2:        rgba(0, 0, 0, 0.72);
    --text-3:        rgba(0, 0, 0, 0.44);
    --badge-bg:      rgba(0, 0, 0, 0.07);
    --badge-fg:      rgba(0, 0, 0, 0.68);
    --skel-base:     rgba(0, 0, 0, 0.07);
    --skel-shine:    rgba(255, 255, 255, 0.55);
    --green:         rgb(28, 185, 68);
    --amber:         rgb(210, 120, 0);
    --gold:          rgb(180, 130, 0);
    --r-panel:       22px;
    --r-card:        16px;
  }

  /* ── Design tokens ── dark ───────────────────────────────── */
  @media (prefers-color-scheme: dark) {
    :root {
      --glass-bg:      rgba(26, 26, 32, 0.68);
      --glass-border:  rgba(255, 255, 255, 0.16);
      --glass-shine:   rgba(255, 255, 255, 0.22);
      --glass-shadow:  rgba(0, 0, 0, 0.50);
      --card-bg:       rgba(255, 255, 255, 0.09);
      --card-border:   rgba(255, 255, 255, 0.14);
      --card-shine:    rgba(255, 255, 255, 0.18);
      --fact-card-bg:  rgba(255, 200, 50, 0.08);
      --fact-card-border: rgba(255, 200, 50, 0.20);
      --text-1:        rgba(255, 255, 255, 1.00);
      --text-2:        rgba(255, 255, 255, 0.88);
      --text-3:        rgba(255, 255, 255, 0.52);
      --badge-bg:      rgba(255, 255, 255, 0.12);
      --badge-fg:      rgba(255, 255, 255, 0.82);
      --skel-base:     rgba(255, 255, 255, 0.09);
      --skel-shine:    rgba(255, 255, 255, 0.20);
      --green:         rgb(48, 219, 91);
      --amber:         rgb(255, 159, 10);
      --gold:          rgb(255, 200, 50);
    }
  }

  /* ── Panel ───────────────────────────────────────────────── */
  .panel {
    position: fixed;
    inset: 0;
    border-radius: var(--r-panel);
    background: var(--glass-bg);
    backdrop-filter: blur(40px) saturate(1.8);
    -webkit-backdrop-filter: blur(40px) saturate(1.8);
    border: 1px solid var(--glass-border);
    box-shadow: 0 8px 32px var(--glass-shadow), inset 0 1px 0 var(--glass-shine);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    font-family: -apple-system, BlinkMacSystemFont, 'Helvetica Neue', sans-serif;
    -webkit-font-smoothing: antialiased;
    font-synthesis: none;
  }

  .panel-glow {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: linear-gradient(155deg, rgba(255,255,255,0.52) 0%, rgba(255,255,255,0.12) 28%, transparent 52%);
    pointer-events: none;
    z-index: 0;
  }

  /* ── Header ──────────────────────────────────────────────── */
  .header {
    position: relative;
    z-index: 1;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 13px 18px 12px;
    border-bottom: 1px solid var(--glass-border);
    color: var(--text-3);
    font-size: 14px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .header span { color: var(--text-2); }

  /* ── Scroll area ─────────────────────────────────────────── */
  .content {
    position: relative;
    z-index: 1;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: 12px;
    padding-bottom: 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .content::-webkit-scrollbar { display: none; }

  /* ── Cards ───────────────────────────────────────────────── */
  .card {
    position: relative;
    flex-shrink: 0;
    border-radius: var(--r-card);
    background: var(--card-bg);
    border: 1px solid var(--card-border);
    box-shadow: inset 0 1px 0 var(--card-shine);
    padding: 14px;
    overflow: hidden;
  }

  .card-glow {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: linear-gradient(165deg, rgba(255,255,255,0.15) 0%, transparent 44%);
    pointer-events: none;
    z-index: 0;
  }

  /* All direct card children sit above the glow overlay */
  .card > :not(.card-glow) {
    position: relative;
    z-index: 1;
  }

  /* ── Track row ───────────────────────────────────────────── */
  .track-row {
    display: flex;
    gap: 13px;
    align-items: flex-start;
  }

  .artwork-wrap {
    flex-shrink: 0;
    width: 70px;
    height: 70px;
    border-radius: 10px;
    overflow: hidden;
    background: var(--badge-bg);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .artwork       { width: 100%; height: 100%; object-fit: cover; display: block; }
  .artwork-empty { color: var(--text-3); }

  .track-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  /* ── Status pill ─────────────────────────────────────────── */
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    align-self: flex-start;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.02em;
    padding: 3px 9px 3px 7px;
    border-radius: 20px;
  }

  .pill.playing {
    color: var(--green);
    background: color-mix(in srgb, var(--green) 15%, transparent);
    border: 1px solid color-mix(in srgb, var(--green) 30%, transparent);
  }

  .pill.paused {
    color: var(--amber);
    background: color-mix(in srgb, var(--amber) 15%, transparent);
    border: 1px solid color-mix(in srgb, var(--amber) 30%, transparent);
  }

  .pill-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .pill.playing .pill-dot { background: var(--green); animation: pulse 2s ease-in-out infinite; }
  .pill.paused  .pill-dot { background: var(--amber); }

  @keyframes pulse {
    0%, 100% { opacity: 1;   transform: scale(1);   }
    50%       { opacity: 0.4; transform: scale(0.7); }
  }

  /* ── Track text ──────────────────────────────────────────── */
  .track-title {
    font-size: 17px;
    font-weight: 700;
    color: var(--text-1);
    letter-spacing: -0.025em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .track-sub {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 13px;
    color: var(--text-2);
    min-width: 0;
  }

  .track-artist { flex-shrink: 0; }
  .sep          { color: var(--text-3); flex-shrink: 0; }

  .track-album {
    color: var(--text-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    min-width: 0;
  }

  /* ── Section header ──────────────────────────────────────── */
  .section-head {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 11px;
    color: var(--text-3);
  }

  .section-label {
    font-size: 11.5px;
    font-weight: 600;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--text-3);
  }

  /* ── Badges ──────────────────────────────────────────────── */
  .badges {
    display: flex;
    gap: 5px;
    flex-wrap: wrap;
    margin-bottom: 10px;
  }

  .track-badges { margin-bottom: 0; margin-top: 1px; }

  .badge {
    font-size: 12px;
    font-weight: 500;
    padding: 2px 9px;
    border-radius: 20px;
    background: var(--badge-bg);
    color: var(--badge-fg);
    border: 1px solid var(--card-border);
  }

  /* ── Body text ───────────────────────────────────────────── */
  .body-text {
    font-size: 14px;
    line-height: 1.65;
    color: var(--text-2);
    margin-bottom: 10px;
  }

  .body-text:last-child { margin-bottom: 0; }
  .body-text.italic     { margin-bottom: 0; }

  /* ── Anecdote card ───────────────────────────────────────── */
  .card-fact {
    background: var(--fact-card-bg);
    border-color: var(--fact-card-border);
  }

  .fact-icon {
    font-size: 11px;
    color: var(--gold);
    flex-shrink: 0;
    line-height: 1;
  }

  /* ── Empty state ─────────────────────────────────────────── */
  .empty {
    font-size: 13px;
    color: var(--text-3);
    text-align: center;
    padding: 6px 0 2px;
  }

  /* ── Skeleton loader ─────────────────────────────────────── */
  @keyframes shimmer {
    from { background-position: -200% center; }
    to   { background-position:  200% center; }
  }

  .skel {
    border-radius: 5px;
    background: linear-gradient(90deg, var(--skel-base) 0%, var(--skel-shine) 50%, var(--skel-base) 100%);
    background-size: 200% 100%;
    animation: shimmer 1.6s ease-in-out infinite;
  }

  .skels { display: flex; flex-direction: column; gap: 7px; }

  .skel.pill-s       { height: 21px; width: 40px; border-radius: 20px; }
  .skel.pill-s.wide  { width: 64px; }
  .skel.line         { height: 11px; width: 100%; }
  .skel.line.short   { width: 62%; }
  .skel.line.mid     { width: 80%; }

  /* ── Idle ────────────────────────────────────────────────── */
  .card.idle {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 40px 14px;
  }

  .idle-icon  { color: var(--text-3); margin-bottom: 2px; }
  .idle-title { font-size: 15px; font-weight: 600; color: var(--text-1); }
  .idle-sub   { font-size: 13px; color: var(--text-3); }
</style>
