# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

AIrena is a Tauri v2 desktop app (Windows) that orchestrates AI discussions using local Ollama models. Participants (GladIAteurs) discuss a topic under an AI moderator (IArbitre), with real-time streaming, emotions, reactions, cognitive personalities, Wikipedia knowledge, multiple discussion modes, and think mode — all running 100% locally.

**Stack**: Tauri v2 + React 19 + TypeScript 5.8 + Vite 7 + Tailwind CSS 4 + shadcn/ui + Zustand 5 + i18next (FR/EN/ZH)

## Build & Development Commands

```bash
# Development (Vite hot-reload + Tauri window)
npm run tauri dev

# Production build (MSI/NSIS installers + standalone .exe)
npm run tauri build

# TypeScript type-check only
npx tsc --noEmit

# Rust tests (across engine modules)
cd src-tauri && cargo test

# Run a single Rust test by name
cd src-tauri && cargo test test_name

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

- **state.rs** — `AppState` holds `engine_cmd_tx` (mpsc channel to running engine), `cancel_token`, and `db` connection. Uses `std::sync::Mutex` (not tokio) because locks are never held across `.await`. Helper methods: `get_settings()`, `clear_engine_slots()`.
- **constants.rs** — Centralized tunable parameters (memory limits, truncation sizes, prompt thresholds). All magic numbers extracted here.
- **commands/** — Tauri IPC handlers grouped by domain: `discussion.rs`, `ollama.rs`, `settings.rs`, `history.rs`
- **engine/mod.rs** — Shared utilities: `truncate_str()`, `truncate_tail()` (UTF-8–safe), `apply_i8_clamped()` (emotion delta)
- **engine/orchestrator.rs** — `DiscussionEngine`: the main discussion loop. Handles introduction → turn loop (speaker order → web/wiki search → directive → prompt → stream → reactions → emotions → memory → moderation) → synthesis → end.
- **engine/turn_manager.rs** — Turn distribution: Sequential, Random, Democratic (masked Borda voting via parallel LLM calls), Authoritarian (IArbitre decides)
- **engine/prompt_builder.rs** — Builds context-aware prompts for each speaker turn, reactions, emotions, synthesis, and end-of-discussion awareness
- **engine/directive_builder.rs** — Cognitive personality system: 5-layer behavioral directives based on emotions, relationships, speech acts (10 types: Challenge, SteelMan, Anecdote, etc.), and situational awareness
- **engine/dynamics_parser.rs** — Parses `<dynamics>` XML sections from system prompts for cognitive personality fields (values, triggers, under_pressure, etc.) with trilingual labels
- **engine/mode_prompts.rs** — Mode-specific instructions: introduction, intervention, thought focus, synthesis, and moderation criteria per `DiscussionMode`
- **engine/emotion_engine.rs** — Rule-based emotion updates (6 axes: engagement, accord, confiance, frustration, curiosité, enthousiasme) + LLM emotion assessment
- **engine/memory_manager.rs** — Maintains discussion summaries and participant position tracking
- **engine/json_parser.rs** — Parses LLM JSON responses with fuzzy speaker name matching (exact → article-stripped → prefix → contains)
- **ollama/client.rs** — `OllamaClient`: HTTP streaming via reqwest, supports think mode, 3-attempt retry. `stream_ndjson()` unified streaming function.
- **tavily/client.rs** — `TavilyClient`: Tavily web search API (requires API key). Per-agent quota enforcement (per-discussion + per-turn). Credit tracking in settings.
- **wikipedia/client.rs** — `WikiClient`: Wikipedia search with language mapping (fr/en/zh→en fallback), smart disambiguation filtering via keyword scoring
- **db/** — SQLite via tokio-rusqlite: `schema.rs` (migrations), `repository.rs` (all queries with `row_to_profile()` + `PROFILE_COLUMNS` DRY helpers), `seed.rs` (predefined profiles)
- **models/** — All data structures with `Serialize`/`Deserialize`. Key types: `ArenaEvent` (30+ variants), `SpeakerRole`, `EmotionalProfile`, `TurnDistribution`, `DiscussionMode` (8 variants), `DocumentFormat`

### Frontend (src/)

- **pages/** — 7 pages: Home, Setup (multi-step wizard), Arena (live discussion), Summary, History, HistoryDetail, Settings
- **stores/** — Zustand 5 stores: `useArenaStore` (discussion state + event dispatch), `useSetupStore` (config including mode/format/wiki), `useSettingsStore` (app prefs)
- **lib/tauri-api.ts** — All Tauri command wrappers. `startDiscussion()` creates a `Channel<ArenaEvent>` for event streaming.
- **lib/persona-\*** — Persona system: `persona-types.ts` (OCEAN, Posture, Identity, Psychology, Voice, Dynamics types), `persona-parser.ts` (parses `<system_kernel>` XML → PersonaData), `persona-serializer.ts` (PersonaData → XML), `persona-labels.ts` (trilingual field labels)
- **lib/profile-emoji.ts** — Maps predefined profile names → emojis, keyword regex → emojis, hash fallback
- **components/discussion/** — DiscussionFeed, MessageBubble, SpeakerBadge, UserInputArea, DiscussionControls, ReadOnlyFeed
- **components/emotion/** — EmotionSidebar, ParticipantEmotionCard (6-axis display), EmotionSparkline, EmotionAxisSlider
- **components/document/** — DocumentSidebar: real-time document co-editing panel with format-specific rendering (txt, md via SimpleMd, csv as table)
- **components/shared/** — MathText (KaTeX LaTeX rendering), SimpleMd (lightweight markdown renderer)
- **components/setup/** — LlmParamsForm, EmojiPicker, PersonaEditor (visual `<system_kernel>` editor), OceanSliders (Big Five sliders)
- **components/layout/** — AppShell, TopBar, Sidebar, ResizeDivider (draggable panel divider)
- **hooks/useTokenBuffer.ts** — 60ms token batching to prevent WebView crash from per-token React re-renders
- **i18n/** — i18next with FR (default), EN, ZH locales
- **providers/ThemeProvider** — Dark/light theme via CSS variables

### IPC Pattern

The engine runs in a Tauri-spawned tokio task. It emits `ArenaEvent` variants through a Tauri `Channel`. The frontend's `useArenaStore.handleEvent()` dispatches these to update Zustand state. Commands flow back via `mpsc::Sender<EngineCommand>` (pause/resume/stop/intervene).

### Discussion Modes

8 modes in `DiscussionMode` enum: `Debate` (default), `Ideation`, `CoConstruction`, `UserDriven`, `Socratic`, `Tutorial`, `CritiqueReview`, `CollaborativeFiction`. Each mode has tailored instructions in `mode_prompts.rs` covering introduction, speaker posture, thought focus, synthesis, and moderation criteria. `CoConstruction` mode supports output in `DocumentFormat` (None, Txt, Md, Csv).

### Persona System (`<system_kernel>` XML)

Each AI personality is defined as a `<system_kernel>` XML prompt embedding a "Neuro-Cognitive Persona" architecture:
- **OCEAN Big Five** (`<big_five_matrix>`) — 5 personality axes scored 1-10 (Openness, Conscientiousness, Extraversion, Agreeableness, Neuroticism)
- **Transactional Analysis** (`<ego_state>`) — Posture: ADULTE, PARENT_CRITIQUE, PARENT_NOURRICIER, ENFANT_LIBRE, ENFANT_ADAPTÉ
- **Cognitive Biases** — Primary bias + blind spot per persona
- **Identity** — Name/role, core philosophy, background, communication style (register, sentence structure, tic)
- **Dynamic Rendering Engine** (`<dynamics>`) — Emotional state → syntax/content/relational behavior rules

Frontend: `persona-parser.ts` parses XML → `PersonaData`, `persona-serializer.ts` does the reverse. `PersonaEditor` provides a visual form editor with `OceanSliders`.

### Cognitive Personality System (Runtime)

`directive_builder.rs` generates behavioral directives per turn based on:
- Emotional state of the speaker
- Relationship graph (Ally, Rival, Tense) between participants
- Weighted random speech act selection (10 discourse strategies)
- Self-memory anti-repetition
- Situational awareness (group mood, turn position, ban returns)

`dynamics_parser.rs` extracts personality fields from `<dynamics>` XML in system prompts.

### Web Search (Tavily)

`TavilyClient` in `tavily/client.rs` provides internet search via Tavily API:
- Requires API key (configured in Settings, free tier: 1000 credits/month)
- Per-agent quota enforcement: per-discussion limit + per-turn limit (max 1/turn)
- Credit counter tracked in DB settings, auto-resets on monthly rolling period
- Forced first-search for IArbitre introduction (mirrors wiki architecture)
- Results emitted as `WebSearchPerformed` events

### Wikipedia Integration

`WikiClient` in `wikipedia/client.rs` performs knowledge lookups:
- Language-aware (fr/en/zh with zh→en fallback)
- `pick_best_result()` scores by keyword overlap, penalizes disambiguation pages
- Pool-based quota system (`wiki_search_pool` in `DiscussionConfig`)
- Forced first-search pattern per gladiateur (mirrors web search architecture)
- Results emitted as `WikiSearchPerformed` events with article URLs
- Uses `tokio::select!` for cancellation support

Both web search and Wikipedia can be enabled independently or together per discussion.

## Critical Implementation Patterns

**Tauri v2 state**: Extract values from `State<'_>` BEFORE any `.await` — the lifetime doesn't cross await points.

**Token buffering**: Streaming tokens MUST be buffered before React state updates. Direct Zustand `set()` per token crashes the WebView (~10K+ re-renders). Both message tokens and synthesis tokens use module-level buffers with 60ms flush intervals.

**serde defaults**: Always use `#[serde(default)]` on LLM response structs — models frequently return partial/invalid JSON. Also used on config structs for backward compatibility when adding new fields.

**SpeakerRole serialization**: Uses per-variant `#[serde(rename)]` — serializes as `"IArbitre"`, `"GladIAteur"`, `"user"`. Not camelCase.

**UTF-8 safety**: Never index a string by char position as a byte index. Use `str::floor_char_boundary()` (stable since Rust 1.82) or `char_indices()`. French names like "Singularité" trigger panics otherwise.

**Error handling**: `CommandError` enum with `#[derive(Serialize)]` is sufficient for Tauri command return types. All `unwrap()` is in test code only; `expect()` at startup only. All engine exit paths emit `DiscussionEnded`.

**DB booleans**: The settings table is key-value. Booleans are stored as `"true"`/`"false"` strings, parsed with `value == "true"`.

**Discussion saving**: Save from the frontend `discussionEnded` handler (not the engine), because engine's `messages_history` doesn't include reactions applied in the frontend Zustand store.

**tokio-util**: `CancellationToken` is available by default — no `sync` feature exists. Only use the `rt` feature.

**tokio-rusqlite 0.6**: Uses rusqlite 0.32 — must add `rusqlite = "0.32"` as a direct dependency for `params!` macro and error types.

**Backward-compatible config evolution**: New config fields always use `#[serde(default)]` so existing serialized data deserializes without errors. DB migrations use idempotent `ALTER TABLE ADD COLUMN` with existence checks.

## i18n

Three languages: French (default), English, Chinese. Frontend uses `useTranslation()` hook from react-i18next. Backend system messages in `prompt_builder.rs` and `mode_prompts.rs` have trilingual branches (FR/EN/ZH) based on `discussion_language`. `dynamics_parser.rs` supports trilingual XML labels.

## Database

SQLite at `{app_data_dir}/airena.db`. Tables: `settings` (key-value), `predefined_profiles`, `discussions` (with `discussion_mode`, `document_format`, `document_content` columns), `discussion_messages` (FK CASCADE). Schema uses idempotent `CREATE TABLE IF NOT EXISTS` + `ALTER TABLE` migrations with column existence checks. Profile seeding runs on every startup with `ON CONFLICT DO NOTHING`.

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
