# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

AIrena is a Tauri v2 desktop app (Windows) that orchestrates AI discussions using local Ollama models or the DeepSeek cloud API (v1.16). Participants (GladIAteurs) discuss a topic under an AI moderator (IArbitre), with real-time streaming, emotions, reactions, cognitive personalities, Wikipedia/web knowledge, RAG document enrichment, an argument mindmap, 8 discussion modes, native reasoning / think mode, token & cost metering, and a license-key gate — 100% local with Ollama (except optional Tavily web search).

**Stack**: Tauri v2 + React 19 + TypeScript 5.8 + Vite 7 + Tailwind CSS 4 + shadcn/ui + Zustand 5 + i18next (FR/EN/ZH)

**Reference docs** (maintained alongside the code — update them when shipping a feature):
- `Docs/Technique/TECHNICAL.md` — full technical documentation + per-version changelog
- `Docs/Fonctionnel/FUNCTIONAL.md` — functional/user documentation
- `Docs/Technique/LICENSE-KEYGEN.md` — license key generation and crypto flow
- `Docs/Technique/AUDIT-2026-09-16-consolidation-deepseek.md` — the v1.16 audit + lot plan (decisions, test plan, deviations)
- `TODO.txt`, `TOKENS.txt` (repo root) — backlog and token-budget design notes

## Build & Development Commands

```bash
# Development (Vite hot-reload + Tauri window)
npm run tauri dev

# Production build → src-tauri/target/release/bundle/ (MSI + NSIS installers + standalone .exe)
npm run tauri build

# TypeScript type-check only
npm run typecheck    # tsc --noEmit

# Frontend unit tests (vitest, node env — stores + pure helpers, *.test.ts under src/)
npm test             # 40 tests
npm run test:watch

# i18n parity gate (fr.json is the reference; fails on missing/extra keys)
npm run i18n:check   # also runs first in `npm run build`
node tools/i18n-add.mjs additions.json   # merge {fr,en,zh} nested keys into the locale files

# Rust tests (376 inline #[cfg(test)] modules incl. engine/engine_tests.rs — there is no tests/ directory)
cd src-tauri && cargo test --lib

# Run a single Rust test by name
cd src-tauri && cargo test test_name

# Real DeepSeek spike (needs DEEPSEEK_API_KEY in the environment; never commit keys)
cd src-tauri && cargo test -- --ignored deepseek_live_spike

# Rust lint (must stay at zero warnings, all targets)
cd src-tauri && cargo clippy --all-targets

# Frontend only (no Tauri window)
npm run dev          # Vite dev server on http://localhost:1420
npm run build        # i18n check + tsc + vite build

# License keys (see Docs/Technique/LICENSE-KEYGEN.md)
node tools/keygen.mjs generate --email user@example.com --duration 720
node tools/keygen.mjs inspect "AIRENA-..."
```

**Prerequisites**: Node.js LTS, Rust stable toolchain (`export PATH="$HOME/.cargo/bin:$PATH"` in Git Bash), Ollama running locally with at least one model (or a DeepSeek API key). There is no ESLint/Prettier config — `tsc --noEmit` (strict, `noUnusedLocals`/`noUnusedParameters`) + vitest are the frontend checks.

**Quality ratchets** (never go below): Rust ≥ 376 tests, front ≥ 40 tests, clippy 0 on `--all-targets`, exact i18n parity, no `pages/*` file > 400 lines (SetupPage 1561 → 195, SettingsPage 933 → 100; `PersonaEditor` at 596 lines is the pre-existing exception).

## Architecture

### Backend (src-tauri/src/)

The Rust backend is a single library crate (`airena_lib`). `main.rs` just calls `lib.rs::run()`.

**Core flow**: `lib.rs` initializes logging, SQLite DB, seeds profiles, creates `AppState`, and registers ~30 Tauri commands.

- **state.rs** — `AppState` holds `engine_cmd_tx` (mpsc channel to running engine), `cancel_token`, `db` connection, and `rag_store` (RAG document store). Uses `std::sync::Mutex` (not tokio) because locks are never held across `.await`. Helper methods: `get_settings()`, `clear_engine_slots()`, `lock_or_recover()`.
- **constants.rs** — Centralized tunable parameters, grouped by `// ── Section ──` headers (memory, prompt truncation, search, orchestrator, Tavily, Wikipedia, Ollama, emotions, moderation, search dedup, think mode, temperature, LLM defaults, RAG, token budget, argument map, license). All magic numbers/strings live here.
- **error.rs** — `CommandError` enum (`Ollama`, `Settings`, `History`, `Rag`, `License`, …) serialized to the frontend as `{ kind, message }`.
- **license.rs** — License key decode/verify (see License System below).
- **commands/** — Tauri IPC handlers grouped by domain: `discussion.rs`, `ollama.rs`, `settings.rs` (also profiles + license), `history.rs`, `rag.rs`, `llm.rs` (`get_llm_constants`, `list_deepseek_models`, `validate_deepseek_key`, `get_llm_usage_period`, `reset_llm_usage_period`)
- **llm/** — Provider abstraction (v1.16). `mod.rs`: `LlmProvider` trait (`chat_stream`/`chat`/`validate`/`capabilities`), `LlmRequest` builder (`.json()` also pins `TEMP_JSON_OUTPUT`, `.reasoning()`, `.speaker()`), `LlmResponse`, `LlmError`. `ollama.rs`: iso-functional adapter over `ollama/client.rs`. `deepseek.rs`: OpenAI-compatible SSE client (thinking + `reasoning_effort`, explicit `max_tokens`, top_p floor / temperature omitted while thinking, backoff retry only when nothing was emitted, idle timeout, `/models`, `/user/balance`). `pricing.rs`: dated price list + UTC peak windows. `metered.rs`: `MeteredProvider` decorator (usage ledger by call kind/speaker, cost, fatal-error latch). `factory.rs`: `build_provider(&AppSettings)`. `mock.rs`: `MockLlmProvider::scripted()` for engine tests.
- **models/llm.rs** — `ProviderKind`, `ReasoningLevel` (`Off/Low/High/Max/Auto`), `CallKind`, `LlmUsage`, `UsageLedger`, `PeriodUsage`, `PeriodHistoryEntry`.
- **db/rolling_period.rs** — Monthly rolling period shared by Tavily credits and DeepSeek spend (`rollover()`).
- **engine/mod.rs** — Shared utilities: `truncate_str()`, `truncate_tail()` (UTF-8–safe), `truncate_at_word_boundary()` (truncates at last space, appends `"…"`), `apply_i8_clamped()` (emotion delta)
- **engine/token_budget.rs** — Waterfall token budget system: computes per-speaker character budgets from `num_ctx`, model params, and user-configured section priorities. `TokenBudget::compute()` allocates in strict rank order (floor → proportional fill → ceiling). `BudgetSection` enum (9 variants: CurrentTurnMessages, ImmediateMemory, ContextualSummary, CognitiveDirectives, ArbitreDirectives, FullDocument, RagContext, WebWikiSearch, PositionalMap), `SectionPriority` (rank + floor/ceiling), `TokenBudgetPreview` for frontend display. Sets `full_document_mode` when the entire RAG document fits in the `FullDocument` allocation.
- **engine/orchestrator.rs** — `DiscussionEngine`: the main discussion loop over an `Arc<MeteredProvider>`. Handles introduction → turn loop (speaker order → reactions → web/wiki/RAG search → focus + reasoning level → directive → prompt → stream → emotions → moderation → usage/budget guard) → end of turn (per-turn document update → LLM emotion analysis → contagion → history → memory → argument map) → synthesis → end. Also owns search dedup (`is_duplicate_query()`), the per-speaker `rag_cache`, stagnation signals (`is_stagnating()`), `pick_focus()`, budget alerts and period persistence. End-of-turn steps run on the last turn too (only a hard stop/cancellation skips them).
- **engine/engine_tests.rs** — End-to-end engine tests on `MockLlmProvider` (event sequence, metering, reasoning fallback, cancellation, fatal errors, budget, focus rotation, stagnation, ban, fiction opening, UserDriven passes, per-turn document, socratic memory). Extend these when touching the loop.
- **engine/focus.rs** — Rotating conversational focus: weighted draw (`FOCUS_WEIGHT_*`) among recent speakers not yet targeted this turn, related participants, or the topic. Not used in fiction / UserDriven.
- **engine/argument_merge.rs** — Merges LLM extractions into the `ArgumentMap`: normalised-token similarity (accents, stop words, CJK per character), thesis dedup (`ARGMAP_THESIS_SIMILARITY_THRESHOLD`) vs looser reference resolution (`ARGMAP_REFERENCE_SIMILARITY_THRESHOLD`), orphan counter-arguments parked in a per-speaker "unattached" bucket, `MergeReport` (new node ids, deduplicated, unattached, dropped).
- **engine/turn_manager.rs** — Turn distribution: Sequential, Random, Democratic (masked Borda voting via parallel LLM calls), Authoritarian (IArbitre decides)
- **engine/prompt_builder.rs** — Builds context-aware prompts for each speaker turn, reactions, emotions, synthesis, search decisions (with past-queries block), argument extraction, and end-of-discussion awareness
- **engine/directive_builder.rs** — Cognitive personality system: 5-layer behavioral directives based on emotions, relationships, speech acts (10 types: Challenge, SteelMan, Anecdote, etc.), self-memory anti-repetition, and situational awareness
- **engine/dynamics_parser.rs** — Parses `<dynamics>` XML sections from system prompts for cognitive personality fields (values, triggers, under_pressure, etc.) with trilingual labels
- **engine/mode_prompts.rs** — Mode-specific instructions: introduction, intervention, thought focus, synthesis, and moderation criteria per `DiscussionMode`
- **engine/emotion_engine.rs** — Rule-based emotion updates (6 axes: engagement, accord, confiance, frustration, curiosité, enthousiasme): decay toward the persona's **initial** profile, `apply_ban_penalty()`, `accord` from reactions given, real stagnation flag, `clamp_delta()` for LLM deltas, `text_similarity()` (Jaccard), contagion. The LLM analyst only adjusts for tone/content and may set `"stagnating"` (`json_parser::parse_emotion_analysis`).
- **engine/memory_manager.rs** — Maintains discussion summaries and participant position tracking
- **engine/json_parser.rs** — Parses LLM JSON responses with fuzzy speaker name matching (exact → article-stripped → prefix → contains)
- **ollama/client.rs** — `OllamaClient`: HTTP streaming via reqwest, supports think mode, 3-attempt retry. `stream_ndjson()` unified streaming function. `strip_think_tags()` removes leaked `<think>…</think>` blocks from non-streamed content. Also `show_model()`, `list_running_models()`, `preload_model()`, `unload_model()`.
- **ollama/model_info.rs** — Parses `/api/show` into `ModelArchInfo` (family, layers, KV heads, quantization, KV bytes/token), detects GPU VRAM via `nvidia-smi` (`detect_gpu_vram()`, NVIDIA only — AMD needs manual config), and computes `recommend_num_ctx()`. `detect_think_support()` checks the model's Go template for `.Think`. `build_model_budget_info()` feeds the frontend `VramIndicator` / `TokenBudgetPreview`.
- **rag/** — RAG system: `parser.rs` (PDF/TXT/MD/CSV/DOCX parsing), `chunker.rs` (text splitting with overlap), `embedder.rs` (Ollama embeddings), `bm25.rs` (lexical search), `store.rs` (in-memory document/chunk/embedding storage with hybrid search + full-text access)
- **tavily/client.rs** — `TavilyClient`: Tavily web search API (requires API key). Per-agent quota enforcement (per-discussion + per-turn). Credit tracking in settings.
- **wikipedia/client.rs** — `WikiClient`: Wikipedia search with language mapping (fr/en/zh→en fallback), smart disambiguation filtering via keyword scoring
- **db/** — SQLite via tokio-rusqlite: `schema.rs` (migrations), `repository.rs` (all queries with `row_to_profile()` + `PROFILE_COLUMNS` DRY helpers, license tracking), `seed.rs` (~120 predefined profiles, ~4000 lines)
- **models/** — All data structures with `Serialize`/`Deserialize`. Key types: `ArenaEvent` (30+ variants), `SpeakerRole`, `EmotionalProfile`, `TurnDistribution`, `DiscussionMode` (8 variants), `DocumentFormat`, `DocumentInjectionMode`, `ArgumentMap`
- **models/argument_map.rs** — `ArgumentMap`, `ThesisNode`, `ArgumentNode` (recursive `children`), `ArgumentType` (Support/Counter/Evidence). `to_markdown(topic, new_ids)` (by thesis) and `to_markdown_by_speaker(topic, new_ids, lang)` produce hierarchical markdown for markmap rendering; nodes in `new_ids` get the ✨ prefix and argument-only speakers get an "Arguments" branch.
- **models/relationship.rs** — `RelationshipEdge` (undirected reaction edge with both directions + classified kind), emitted as `RelationshipsUpdated` by `directive_builder::relationship_edges()`.

### Frontend (src/)

Path alias `@/` → `src/` (tsconfig + vite).

- **pages/** — 7 pages: Home, Setup (wizard shell — steps live in `components/setup/steps/`), Arena (live discussion), Summary, History, HistoryDetail, Settings (composition of `components/settings/*` + debounced autosave)
- **stores/** — Zustand 5 stores: `useArenaStore` (discussion state + event dispatch; also `llmUsage`, `budgetAlert`, `argumentMap` JSON, `relationships`, `passedSpeakerIds`, `lastThresholdCrossed`), `useSetupStore` (config including mode/format/wiki/document injection mode/document update granularity), `useSettingsStore` (app prefs + license + provider; helpers `describeActiveModel()`, `needsOllama()`, `loadLlmConstants()`), `useToastStore` (global notifications rendered by `ToastContainer`). Each store has a vitest file next to it.
- **lib/tauri-api.ts** — All Tauri command wrappers. `startDiscussion()` creates a `Channel<ArenaEvent>` for event streaming.
- **lib/cost-estimate.ts** — `estimateTurnCost()` (order-of-magnitude cost per turn for cloud providers), `formatUsd()`.
- **lib/discussion-config.ts** — `applyGlobalContext()`: the global `numCtx` (Ollama window / DeepSeek budget) is applied to every speaker right before `startDiscussion`.
- **lib/error-utils.ts** — `extractErrorMessage()`: normalizes caught values (`Error`, Tauri `CommandError` `{kind, message}`, strings). Always use it instead of `String(e)`.
- **lib/logger.ts** — Structured frontend logger with circular buffer and export.
- **lib/persona-\*** — Persona system: `persona-types.ts` (OCEAN, Posture, Identity, Psychology, Voice, Dynamics types), `persona-parser.ts` (parses `<system_kernel>` XML → PersonaData), `persona-serializer.ts` (PersonaData → XML), `persona-labels.ts` (trilingual field labels)
- **lib/profile-emoji.ts** — Maps predefined profile names → emojis, keyword regex → emojis, hash fallback
- **lib/document-diff.ts** — Diff strategies for co-constructed documents (see below)
- **components/discussion/** — DiscussionFeed (turn dividers + "jump to latest", no auto-scroll), MessageBubble (thought vs model reasoning), SpeakerBadge, TurnIndicator, UsagePill (tokens/cost/peak), SpeakerQueue (`deriveQueue()`), UserInputArea, DiscussionControls, ReadOnlyFeed
- **components/emotion/** — EmotionSidebar exports `EmotionPanel` (bars/radar toggle, threshold flashes from `lastThresholdCrossed`), ParticipantEmotionCard (directive incl. focus + reasoning level), EmotionRadar (SVG), EmotionSparkline, EmotionAxisSlider
- **components/document/** — DocumentSidebar exports `DocumentPanel`: real-time document co-editing panel with format-specific rendering (txt, md via SimpleMd, csv as table)
- **components/mindmap/** — MarkmapViewer (forwardRef, `getSvgHtml()`, `fit()`; keeps the user's zoom once they interacted), MindmapSidebar exports `ArgumentMapPanel` (counters, ✨ new, dropped warning, per-speaker chips, recentre)
- **components/relations/** — RelationsGraph (ring layout, edges coloured by ally/rival/tense)
- **components/layout/** — AppShell, TopBar, Sidebar, ResizeDivider, RightPanel (tabbed side panel: Emotions / Document / Map / Relations; column on `lg`, drawer below; tab/width/collapse remembered in localStorage)
- **components/settings/** — SettingsPrimitives (Section/Field/StatusPill/ChoiceRow), GeneralSettings, LicenseSettings, ProviderSettings, DeepSeekSettings (key validated before it is stored, models, context budget, monthly cap, period gauge/history/reset), OllamaSettings (`chatEnabled=false` → embeddings only), TokenBudgetPriorities, TavilySettings
- **components/setup/** — LlmParamsForm (Ollama vs DeepSeek variant: reasoning level, temperature disabled while thinking, top_p floor, bounds from `get_llm_constants`), EmojiPicker, PersonaEditor, OceanSliders, TokenBudgetPreview (pages from backend + cost line), VramIndicator, `steps/` (StepTopic, StepArbitre, StepGladiateurs, StepKnowledge, StepSummary + shared primitives)
- **components/shared/** — MathText, SimpleMd, StatCard, UsageSummaryCard, ToastContainer
- **components/common/ErrorBoundary** — top-level React error boundary
- **hooks/useTokenBuffer.ts** — 60ms token batching to prevent WebView crash from per-token React re-renders
- **hooks/useMediaQuery.ts** — reactive `matchMedia` (`LG_BREAKPOINT_QUERY` drives the panel layout)
- **i18n/** — i18next with FR (default), EN, ZH locales. Every user-facing string goes in all three `locales/*.json` files.
- **providers/ThemeProvider** — Dark/light theme via CSS variables

### IPC Pattern

The engine runs in a Tauri-spawned tokio task. It emits `ArenaEvent` variants through a Tauri `Channel`. The frontend's `useArenaStore.handleEvent()` dispatches these to update Zustand state. Commands flow back via `mpsc::Sender<EngineCommand>` (pause/resume/stop/intervene).

Tauri permissions are declared in `src-tauri/capabilities/default.json` (`core`, `opener`, `dialog`, `fs` + `fs:allow-write-text-file` for exports).

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
- Self-memory anti-repetition (last 2 own messages + last speech act)
- Situational awareness (group mood, turn position, ban returns)

`dynamics_parser.rs` extracts personality fields from `<dynamics>` XML in system prompts.

### Reasoning / Think Mode

- **Policy per call kind**: utilities (reactions, moderation, memory, emotions, votes, search decisions, RAG selection, respond/pass, argmap) are always `ReasoningLevel::Off` + JSON mode; introduction `Low`; interventions resolved from `Auto` (`resolve_gladiateur_reasoning()`: strong triggers — frustration, contradiction, near end — → `High`, else `Low`, never on turn 1; providers without levels get `High`); synthesis `High`. Per-speaker `LlmParams.reasoning_level` overrides the global setting.
- **DeepSeek**: `reasoning_content` replaces the separate in-character thought call; streamed as `ThoughtChunk` (if `show_model_reasoning`) and stored as `Message.inner_thought` with `thought_kind = reasoning`. After `REASONING_MAX_FAILURES` consecutive empty/refused answers the engine falls back to the classic thought + intervention path.
- **Ollama**: `detect_think_support()` inspects the model template for `.Think`; thinking models get the ×3 `num_predict` boost on discussion-content calls (`CallKind::is_discussion_content`) because `num_predict` caps thinking + content combined. `think` is never sent explicitly. `strip_think_tags()` is a safety net for non-streamed calls.

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

### Search Deduplication

Two layers prevent repeated queries (`orchestrator.rs`):
- **Soft**: the search-decision prompt lists the speaker's past queries and asks for a different angle (`build_past_queries_block()` in `prompt_builder.rs`)
- **Hard**: `is_duplicate_query()` filters near-duplicates (case-insensitive exact match, or substring match when the shorter query ≥ `SEARCH_DEDUP_MIN_SUBSTRING_LEN` = 8 bytes) against the speaker's `search_queries_history` and against queries already run by other speakers this turn (reset each turn)

## RAG System

`rag/` module provides Retrieval-Augmented Generation capabilities:

### Architecture
- **parser.rs** — Multi-format parsing (PDF via lopdf, TXT/MD via UTF-8, CSV as formatted table, DOCX via regex)
- **chunker.rs** — Semantic text splitting with configurable target size (2000 chars) and overlap (200 chars)
- **embedder.rs** — `EmbeddingClient` generates embeddings via Ollama model (uses `embedding_model` setting or falls back to main LLM model)
- **bm25.rs** — Lexical search using BM25 algorithm (tf-idf with length normalization)
- **store.rs** — `RagStore` holds documents, chunks, embeddings, BM25 index, and full texts in memory

### Two injection modes (`DocumentInjectionMode` in `DiscussionConfig`)
- **`Rag`** (default) — per-turn hybrid retrieval of the most relevant chunks
- **`FullInjection`** — the whole document is injected into every prompt. Only honored when `TokenBudget.full_document_mode` is true (entire document fits the `FullDocument` allocation); otherwise the engine silently falls back to chunk retrieval. Import with `skipEmbeddings=true` stores text-only (`add_document_text_only()`); `ensure_embeddings()` computes them lazily on the first fallback query.

### Workflow
1. **Import** (`import_rag_document`) — parse file → chunk text → generate embeddings (unless skipped) → store
2. **Search** (during discussion) — LLM decides to search → hybrid BM25 + cosine similarity → top-K chunks
3. **Injection** — chunks injected into prompt context → `RagContextInjected` event emitted (`cached: true` when served from cache)

### Retrieval pipeline (3-stage)
1. **Stage 1a** — Vector cosine similarity → top 30 (`RAG_RETRIEVAL_TOP_K`)
2. **Stage 1b** — BM25 lexical search → top 30
3. **Stage 1c** — RRF (Reciprocal Rank Fusion) merges both → top 10 (`RAG_RRF_TOP_K`)
4. **Stage 2** — LLM selects most relevant → max 5 chunks (`RAG_LLM_SELECT_MAX`), fallback top 3 RRF if selection fails
5. Anti "Lost in the Middle" reordering → formatted context (≤ `RAG_MAX_CONTEXT_LEN`)

### Per-speaker cache
`DiscussionEngine.rag_cache: HashMap<speaker_id, RagCacheEntry>` — a speaker's retrieved context is reused for `RAG_CACHE_TTL_TURNS` (3) turns before running the full pipeline again.

### Key constraints
- **No persistence** — RAG store lives in memory only (cleared on app restart or explicit clear)
- **Embedding dimension consistency** — mixing models with different embedding dimensions causes errors
- **Memory usage** — all documents + embeddings stored in RAM; `RAG_MAX_FILE_SIZE_BYTES` = 10 MB per file

## Document Diff (Co-Construction)

`lib/document-diff.ts` provides visual diff highlighting for collaborative documents:

### Strategies by format
- **TXT** — word-level diff via `diffWords()` → segments with `highlighted` flag
- **MD** — line-level diff via `diffLines()` → Set of changed line indices
- **CSV** — cell-level diff → Set of `"row,col"` keys for changed cells

### Usage
- Called on every `DocumentUpdated` event
- Returns `DiffResult | null` (null if no changes)
- Applied visually in `DocumentSidebar` component

## Argument Map (Carte des arguments)

`models/argument_map.rs` + orchestrator integration provides automatic thesis/argument extraction:

### Architecture
- **Extraction** — After each turn (≥2), `build_argument_extraction_prompt()` sends recent context + existing map to LLM
- **Parsing** — `parse_argument_extraction()` in `json_parser.rs` with fuzzy speaker name matching
- **Merge** — New theses/arguments merged with existing map (additive only). Arguments are **recursive** (`ArgumentNode.children`) up to `ARGMAP_MAX_ARGUMENT_DEPTH` = 4 (Support → Counter → Refutation → Evidence); a target already at max depth attaches the new argument flat to the thesis.
- **Rendering** — two views, both emitted in `ArgumentMapUpdated { markdown, markdown_by_speaker, … }`:
  - `to_markdown()` → `# Topic / ## Thesis (Speaker) / - Arg` (by thesis)
  - `to_markdown_by_speaker()` → `# Topic / ## Speaker / ### Thesis / - Arg` (by speaker)
- **Frontend** — `MarkmapViewer` (markmap-lib + markmap-view) renders interactive SVG mindmap; `MindmapSidebar` toggles between views
- **Persistence** — `discussions.argument_map_md` + `discussions.argument_map_md_by_speaker` columns

### Configuration (constants.rs)
- `ARGMAP_MAX_THESIS_LABEL` — 100 chars max per thesis label
- `ARGMAP_MAX_ARGUMENT_LABEL` — 200 chars max per argument label
- `ARGMAP_MAX_THESES` — 20 max theses
- `ARGMAP_MAX_ARGUMENTS` — 100 max total arguments (recursive count)
- `ARGMAP_MAX_ARGUMENT_DEPTH` — 4
- `ARGMAP_NUM_PREDICT` — 4096 (generous for quality JSON)
- Temperature: `TEMP_JSON_OUTPUT` (0.3) — pinned by `LlmRequest::json()` like every structured call

### Key constraints
- Labels must be natural language in discussion language (not technical IDs)
- Extraction prompt includes explicit language instructions (FR/EN/ZH)
- SVG export via `forwardRef` + `useImperativeHandle` on `MarkmapViewer`

## License System

A license key is required to start a discussion (`Docs/Technique/LICENSE-KEYGEN.md` has the full flow).

- **Format**: `AIRENA-xxxxx-xxxxx-…` = Base64(nonce ‖ AES-256-GCM(payload ‖ Ed25519 signature)). Payload `{v, e (email), t (created), d (hours), n (nonce)}`.
- **Backend** (`license.rs`): `decode_license_key()` (strip prefix → Base64 → AES-GCM decrypt → Ed25519 verify → JSON), `hash_license_key()` (SHA-256, identifies the key for the counter), `check_license_status()` (version, expiry, clock-rollback tolerance `LICENSE_CLOCK_TOLERANCE_SECS` = 2h, quota `LICENSE_DISCUSSIONS_PER_DAY` = 50 per 24h of validity).
- **Gate**: `start_discussion` validates the key (returns `CommandError::License`) before any work; the discussion counter is incremented via `repository::increment_license_discussions()` only after Ollama validation succeeds and before the engine spawns. A new key hash resets the counter.
- **Commands**: `validate_license_key`, `check_license_status` (in `commands/settings.rs`). Frontend gates the "New discussion" buttons on Home/Setup and shows a license section in Settings.
- **Keys**: `LICENSE_ED25519_PUBLIC_KEY_HEX` + `LICENSE_AES_KEY_HEX` in `constants.rs` are compiled into the binary. The private signing key lives in `tools/.keys.json` (gitignored — never commit, print, or regenerate it: regenerating invalidates every issued key).
- **Generator**: `tools/keygen.mjs` (Node native crypto, zero deps) — `init` / `generate` / `inspect`.

## Logging

File-based logging via `tracing` + `tracing-appender`:
- Daily rotation to `{exe_dir}/logs/airena.log` (7-day retention)
- Dual output: console + file (non-blocking writer)
- Default filter: `airena=info,airena_lib=info` (override via `RUST_LOG` env var)
- Guard leaked on startup to keep file writer alive until app shutdown
- Frontend: `lib/logger.ts` (structured, circular buffer)

## Critical Implementation Patterns

**Tauri v2 state**: Extract values from `State<'_>` BEFORE any `.await` — the lifetime doesn't cross await points.

**Token buffering**: Streaming tokens MUST be buffered before React state updates. Direct Zustand `set()` per token crashes the WebView (~10K+ re-renders). Both message tokens and synthesis tokens use module-level buffers with 60ms flush intervals.

**serde defaults**: Always use `#[serde(default)]` on LLM response structs — models frequently return partial/invalid JSON. Also used on config structs for backward compatibility when adding new fields.

**SpeakerRole serialization**: Uses per-variant `#[serde(rename)]` — serializes as `"IArbitre"`, `"GladIAteur"`, `"user"`. Not camelCase.

**ArenaEvent serialization**: Uses `#[serde(rename_all = "camelCase", tag = "type", content = "data")]` on the enum, plus per-variant `#[serde(rename_all = "camelCase")]` on struct variants. When adding new events, always add the per-variant rename attribute for field names to match frontend expectations, and mirror the variant in `src/lib/types.ts` + `useArenaStore.handleEvent()`.

**UTF-8 safety**: Never index a string by char position as a byte index. Use `str::floor_char_boundary()` (stable since Rust 1.82) or `char_indices()`. French names like "Singularité" trigger panics otherwise.

**Error handling**: `CommandError` enum with `#[derive(Serialize)]` is sufficient for Tauri command return types. All `unwrap()` is in test code only; `expect()` at startup only. All engine exit paths emit `DiscussionEnded`. Frontend: catch as `unknown` and pass through `extractErrorMessage()`.

**DB booleans**: The settings table is key-value. Booleans are stored as `"true"`/`"false"` strings, parsed with `value == "true"`.

**Discussion saving**: Save from the frontend `discussionEnded` handler (not the engine), because engine's `messages_history` doesn't include reactions applied in the frontend Zustand store.

**tokio-util**: `CancellationToken` is available by default — no `sync` feature exists. Only use the `rt` feature.

**tokio-rusqlite 0.6**: Uses rusqlite 0.32 — must add `rusqlite = "0.32"` as a direct dependency for `params!` macro and error types.

**Mutex poison recovery**: `AppState::lock_or_recover()` recovers from poisoned `std::sync::Mutex` — safe because AppState fields are always left in a consistent state. Use this instead of `.lock().unwrap()`.

**Backward-compatible config evolution**: New config fields always use `#[serde(default)]` so existing serialized data deserializes without errors. DB migrations use idempotent `ALTER TABLE ADD COLUMN` with `PRAGMA table_info` existence checks.

**Structured LLM calls**: Every call is an `LlmRequest` with a `CallKind`; utilities use `.json()` (JSON mode + `TEMP_JSON_OUTPUT`) and never request reasoning. Only `CallKind::is_discussion_content()` calls get the Ollama thinking `num_predict` boost.

**Provider errors**: `LlmError::Cancelled` ends quietly; `Auth` / `InsufficientBalance` / `ModelNotFound` are fatal (`MeteredProvider::has_fatal_error()`) — the engine stops after the current speaker, skips synthesis and emits `Error` + `DiscussionEnded`; everything else is a non-fatal "X seems to have difficulties" message. Never log API keys (`tools/.keys.json` is git-ignored).

**Cost metering**: `LlmUsageUpdated` after each speaker / end of turn / synthesis; `BudgetAlert` warn at `LLM_BUDGET_WARN_RATIO`, soft stop at 100 %; period spend persisted on every engine exit via `record_deepseek_usage`. The frontend saves `llmProvider`, `usage`, `estimatedCostUsd`, `argumentMapJson` with the discussion (`modelName` = `provider · model`).

**Server-owned settings**: the `save_settings` command goes through `repository::save_user_settings`, which re-reads the DeepSeek period counters (`deepseek_period_start`, `deepseek_period_usage_json`, `deepseek_usage_history`) from the DB before writing — a stale frontend payload must never overwrite spend recorded by the engine. Internal code that legitimately updates those counters calls `repository::save_settings`. The arena store re-hydrates `useSettingsStore` on `discussionEnded` for the same reason (Tavily credits included).

**Live reasoning**: `thoughtChunk` events are buffered under `reasoningStreamKey(speakerId)` (`useArenaStore`) and rendered by `StreamingBubble variant="reasoning"` (tail preview); `show_model_reasoning=false` only stops the live stream — the reasoning is still stored on the message (`thought_kind = reasoning`).

**Async turn context**: `AsyncTurnContext` (turn_manager) has all fields OWNED (no lifetimes) so parallel LLM calls can run across `.await` without borrowing `&mut self`. `OllamaClient` is `Clone` (reqwest client is Arc-based).

## i18n

Three languages: French (default), English, Chinese. Frontend uses `useTranslation()` hook from react-i18next. Backend system messages in `prompt_builder.rs`, `mode_prompts.rs`, and orchestrator helpers (`speaker_difficulty_msg()`, `ban_notification_msg()`) have trilingual branches (FR/EN/ZH) based on `discussion_language`. `dynamics_parser.rs` supports trilingual XML labels.

## Database

SQLite at `{app_data_dir}/airena.db`. Tables: `settings` (key-value, includes `embedding_model` for RAG, `license_key`, license tracking, Tavily credits, `llm_provider`, `reasoning_level`, `show_model_reasoning`, `deepseek_*` key/model/budget/period), `predefined_profiles`, `discussions` (with `discussion_mode`, `document_format`, `document_content`, `argument_map_md`, `argument_map_md_by_speaker`, `argument_map_json`, `llm_provider`, `usage_json`, `estimated_cost_usd` columns), `discussion_messages` (FK CASCADE, `thought_kind`). Schema uses idempotent `CREATE TABLE IF NOT EXISTS` + `ALTER TABLE` migrations with column existence checks. Profile seeding runs on every startup with `ON CONFLICT DO NOTHING`.

## Good practices

- Privilégie toujours les fonctionnalité et la qualité à la consommation de tokens.

- Toutes les constants, magic string et magic number doivent être dans le fichier constants

- Es tu intellectuellement (logique fonctionnelle), fonctionnellement (bonne exécution fonctionnelle) et techniquement (implémentaion correcte, conforme aux bonnes pratiques, optimale) pleinement satisfait et convaincu de ton plan ou implémentation du plan ?

- Vérifie sur le fond et la forme, notamment, mais sans être exhaustif :
1. assure toi bien de bien valider la complétude du plan d'implémentation
2. assure toi d'être conforme aux patterns techniques et fonctionnels de la code base
3. vérifie bien les nommages de imports, classes, méthodes, fonctions, variables, constantes que tous les aruguments soient bien définis et transmis
4. respect du Rust Style Guide,
5. respect du Node Style Guide,
6. respect du Tauri Style Guide,
7. respect DRY, YAGNI, KISS, SRP, SoC, Boy Scout Rule, Composition over Inheritance
8. mise en oeuvre des bonnes pratiques générales de développement moderne,
9. réutilisation au maximum des classe et méthodes existantes (helpers, classes, méthodes, variables et constantes),
10. pas de code dupliqué dans la base code,
11. code générique et évolutif,
12. respect des patterns techniques des Framework dans les versions utilisées par le projet, documente toi sur internet si besoin de te mettre à jour sur ces versions
13. respect des patterns techniques et fonctionnels de la code base,
14. gestion professionnelle des erreurs et exceptions
