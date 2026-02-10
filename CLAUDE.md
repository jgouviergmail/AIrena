# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

AIrena is a Tauri v2 desktop app (Windows) that orchestrates AI debates using local Ollama models. Participants (GladIAteurs) debate a topic under an AI moderator (IArbitre), with real-time streaming, emotions, reactions, moderation, and think mode — all running 100% locally.

**Stack**: Tauri v2 + React 19 + TypeScript 5.8 + Vite 7 + Tailwind CSS 4 + shadcn/ui + Zustand 5 + i18next (FR/EN/ZH)

## Build & Development Commands

```bash
# Development (Vite hot-reload + Tauri window)
npm run tauri dev

# Production build (MSI/NSIS installers + standalone .exe)
npm run tauri build

# TypeScript type-check only
npx tsc --noEmit

# Rust tests (38 tests across engine modules)
cd src-tauri && cargo test

# Rust lint
cd src-tauri && cargo clippy

# Frontend only (no Tauri window)
npm run dev          # Vite dev server on http://localhost:1420
npm run build        # tsc + vite build
```

**Prerequisites**: Node.js LTS, Rust stable toolchain, Ollama running locally with at least one model.

## Architecture

### Backend (src-tauri/src/)

The Rust backend is a single library crate (`airena_lib`). `main.rs` just calls `lib.rs::run()`.

**Core flow**: `lib.rs` initializes logging, SQLite DB, seeds profiles, creates `AppState`, and registers 20+ Tauri commands.

- **state.rs** — `AppState` holds `engine_cmd_tx` (mpsc channel to running engine), `cancel_token`, and `db` connection. Uses `std::sync::Mutex` (not tokio) because locks are never held across `.await`.
- **commands/** — Tauri IPC handlers grouped by domain: `discussion.rs`, `ollama.rs`, `settings.rs`, `history.rs`
- **engine/orchestrator.rs** — `DiscussionEngine`: the main debate loop (~2100 lines). Handles introduction → turn loop (speaker order → prompt → stream → reactions → emotions → memory → moderation) → synthesis → end.
- **engine/turn_manager.rs** — Turn distribution: Sequential, Random, Democratic (masked Borda voting via parallel LLM calls), Authoritarian (IArbitre decides)
- **engine/prompt_builder.rs** — Builds context-aware prompts for each speaker turn, reactions, emotions, synthesis, and end-of-discussion awareness
- **engine/emotion_engine.rs** — Rule-based emotion updates (6 axes: engagement, accord, confiance, frustration, curiosité, enthousiasme) + LLM emotion assessment
- **engine/memory_manager.rs** — Maintains discussion summaries and participant position tracking
- **engine/json_parser.rs** — Parses LLM JSON responses with fuzzy speaker name matching (exact → article-stripped → prefix → contains)
- **ollama/client.rs** — `OllamaClient`: HTTP streaming via reqwest, supports think mode, 3-attempt retry
- **db/** — SQLite via tokio-rusqlite: `schema.rs` (migrations), `repository.rs` (all queries), `seed.rs` (predefined profiles)
- **models/** — All data structures with `Serialize`/`Deserialize`. Key types: `ArenaEvent` (30+ variants), `SpeakerRole`, `EmotionalProfile`, `TurnDistribution`

### Frontend (src/)

- **pages/** — 7 pages: Home, Setup (4-step wizard), Arena (live debate), Summary, History, HistoryDetail, Settings
- **stores/** — Zustand 5 stores: `useArenaStore` (discussion state + event dispatch), `useSetupStore` (config), `useSettingsStore` (app prefs)
- **lib/tauri-api.ts** — All Tauri command wrappers. `startDiscussion()` creates a `Channel<ArenaEvent>` for event streaming.
- **components/discussion/** — DiscussionFeed, MessageBubble, SpeakerBadge, UserInputArea, DiscussionControls
- **components/emotion/** — EmotionSidebar with ParticipantEmotionCard (6-axis display + sparklines)
- **hooks/useTokenBuffer.ts** — 60ms token batching to prevent WebView crash from per-token React re-renders
- **i18n/** — i18next with FR (default), EN, ZH locales
- **providers/ThemeProvider** — Dark/light theme via CSS variables

### IPC Pattern

The engine runs in a Tauri-spawned tokio task. It emits `ArenaEvent` variants through a Tauri `Channel`. The frontend's `useArenaStore.handleEvent()` dispatches these to update Zustand state. Commands flow back via `mpsc::Sender<EngineCommand>` (pause/resume/stop/intervene).

## Critical Implementation Patterns

**Tauri v2 state**: Extract values from `State<'_>` BEFORE any `.await` — the lifetime doesn't cross await points.

**Token buffering**: Streaming tokens MUST be buffered before React state updates. Direct Zustand `set()` per token crashes the WebView (~10K+ re-renders). Both message tokens and synthesis tokens use module-level buffers with 60ms flush intervals.

**serde defaults**: Always use `#[serde(default)]` on LLM response structs — models frequently return partial/invalid JSON.

**SpeakerRole serialization**: Uses per-variant `#[serde(rename)]` — serializes as `"IArbitre"`, `"GladIAteur"`, `"user"`. Not camelCase.

**UTF-8 safety**: Never index a string by char position as a byte index. Use `str::floor_char_boundary()` (stable since Rust 1.82) or `char_indices()`. French names like "Singularité" trigger panics otherwise.

**Error handling**: `CommandError` enum with `#[derive(Serialize)]` is sufficient for Tauri command return types. All `unwrap()` is in test code only; `expect()` at startup only. All engine exit paths emit `DiscussionEnded`.

**DB booleans**: The settings table is key-value. Booleans are stored as `"true"`/`"false"` strings, parsed with `value == "true"`.

**Discussion saving**: Save from the frontend `discussionEnded` handler (not the engine), because engine's `messages_history` doesn't include reactions applied in the frontend Zustand store.

**tokio-util**: `CancellationToken` is available by default — no `sync` feature exists. Only use the `rt` feature.

**tokio-rusqlite 0.6**: Uses rusqlite 0.32 — must add `rusqlite = "0.32"` as a direct dependency for `params!` macro and error types.

## i18n

Three languages: French (default), English, Chinese. Frontend uses `useTranslation()` hook from react-i18next. Backend system messages in `prompt_builder.rs` have trilingual branches (FR/EN/ZH) based on `discussion_language`.

## Database

SQLite at `{app_data_dir}/airena.db`. Tables: `settings` (key-value), `predefined_profiles`, `discussions`, `discussion_messages` (FK CASCADE). Schema uses idempotent `CREATE TABLE IF NOT EXISTS` + `ALTER TABLE` migrations. Profile seeding runs on every startup with `ON CONFLICT DO NOTHING`.

## Good practices

Es tu intellectuellement (logique fonctionnelle), fonctionnellement (bonne exécution fonctionnelle) et techniquement (implémentaion correcte, conforme aux bonnes pratiques, optimale) pleinement satisfait et convaincu de ton plan ou implémentation du plan ?

Vérifie sur le fond et la forme, notamment, mais sans être exhaustif :
- assure toi bien de bien valider la complétude du plan d'implémentation
- assure toi d'être conforme aux patterns techniques et fonctionnels de la code base
- vérifie bien les nommages de imports, classes, méthodes, fonctions, variables, constantes que tous les aruguments soient bien définis et transmis
- respect du Rust Style Guide, 
- respect du Node Style Guide, 
- respect du Tauri Style Guide, 
- respect DRY, YAGNI, KISS, SRP, SoC, Boy Scout Rule, Composition over Inheritance
- mise en oeuvre des bonnes pratiques générales de développement moderne,
- réutilisation au maximum des classe et méthodes existantes (helpers, classes, méthodes, variables et constantes),
- pas de code dupliqué dans la base code,
- code générique et évolutif,
- respect des patterns techniques des Framework dans les versions utilisées par le projet, documente toi sur internet si besoin de te mettre à jour sur ces versions
- respect des patterns techniques et fonctionnels de la code base,
- gestion professionnelle des erreurs et exceptions