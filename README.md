# Enhanced Music

Une app macOS dans la barre de menus qui enrichit l'écoute Apple Music en temps réel : pochette HD, contexte de l'album, anecdote, et analyse des paroles — le tout généré par IA.

![macOS](https://img.shields.io/badge/macOS-13%2B-black?logo=apple)
![Tauri](https://img.shields.io/badge/Tauri-2-blue?logo=tauri)
![Svelte](https://img.shields.io/badge/Svelte-5-orange?logo=svelte)
![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)

---

## Fonctionnalités

- **Piste en cours** — titre, artiste, album, pochette HD (iTunes)
- **Année & genre** — récupérés automatiquement via l'API iTunes
- **Contexte album** — genèse et histoire de l'album (Claude + Genius)
- **Anecdote** — un fait marquant sur l'album
- **Analyse des paroles** — interprétation du morceau en français (Claude + Genius)
- **Interface native** — vitre macOS (NSVisualEffectView), mode sombre/clair automatique, police système SF Pro
- **Polling non-bloquant** — mise à jour toutes les 3 secondes, sans doublon ni données périmées

---

## Stack

| Couche | Techno |
|---|---|
| UI | Svelte 5 + TypeScript |
| Shell | Tauri 2 (Rust) |
| IA | Anthropic Claude Haiku |
| Métadonnées | iTunes Search API (gratuit) |
| Paroles | Genius API + scraping |
| Style | CSS custom properties, glassmorphism natif macOS |

---

## Prérequis

- macOS 13 Ventura ou supérieur
- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 18+
- [Tauri CLI v2](https://tauri.app/start/prerequisites/)

---

## Installation

```bash
git clone https://github.com/yourname/enhanced-music
cd enhanced-music
npm install
```

### Clés API

Crée le fichier de configuration :

```bash
mkdir -p ~/.config/enhanced-music
```

```toml
# ~/.config/enhanced-music/config.toml

[api]
genius_token  = "VOTRE_TOKEN_GENIUS"
anthropic_key = "VOTRE_CLE_ANTHROPIC"
```

- **Genius** — token gratuit sur [genius.com/api-clients](https://genius.com/api-clients)
- **Anthropic** — clé API sur [console.anthropic.com](https://console.anthropic.com)

> Sans ces clés, l'app affiche quand même la piste et la pochette. Les sections Album, Anecdote et Paroles restent vides.

---

## Développement

```bash
npm run tauri dev
```

L'app se compile, se lance et se recharge automatiquement à chaque modification.

## Build

```bash
npm run tauri build
```

Génère un `.dmg` et un `.app` dans `src-tauri/target/release/bundle/`.

---

## Architecture

```
enhanced-music/
├── src/
│   └── routes/
│       └── +page.svelte       # UI complète (Svelte 5 runes)
└── src-tauri/
    └── src/
        ├── lib.rs             # Commandes Tauri, appels API, scraping Genius
        └── config.rs          # Chargement de ~/.config/enhanced-music/config.toml
```

### Flux de données

```
Apple Music (AppleScript)
    └── get_current_track      → titre / artiste / album / état lecture
    └── get_artwork            → pochette base64 via iTunes Search
    └── get_album_info         → iTunes (année/genre) + Genius + Claude
    └── get_lyrics_analysis    → Genius search + scraping paroles + Claude
```

Chaque commande Tauri est synchrone côté Rust (thread pool) et non-bloquante côté UI grâce aux generation counters Svelte : si la piste change pendant un fetch, la réponse obsolète est ignorée.

---

## Configuration avancée

La fenêtre est définie dans `src-tauri/tauri.conf.json` :

```json
{
  "width": 380,
  "height": 720,
  "decorations": false,
  "transparent": true,
  "alwaysOnTop": true
}
```

Le modèle Claude utilisé est `claude-haiku-4-5-20251001` (rapide et économique). Il peut être changé dans `src-tauri/src/lib.rs` dans la fonction `call_claude`.
