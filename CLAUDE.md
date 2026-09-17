# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

AIrena is a Tauri v2 desktop app (Windows) that orchestrates AI discussions using local Ollama models, the DeepSeek cloud API or any OpenAI-compatible server (v1.20). Participants (GladIAteurs) discuss a topic under an AI moderator (IArbitre), with real-time streaming, emotions, typed reactions (immediate rounds, audience), pre-speech intentions and open loops, hidden agendas unveiled at the end, assisted casting, position trajectories, stage directions, cognitive personalities, Wikipedia/web knowledge with a sources panel, RAG document enrichment, an argument mindmap, 13 discussion modes (incl. trial, Oxford debate, negotiation, six hats, crisis cell with roles, verdicts, audience votes and dispatches), native reasoning / think mode, one model per speaker, discussion templates, full-text history with tags and favourites, standalone HTML export, long-term persona memory, bounded advanced tuning, token & cost metering, per-turn timings and diagnostics, and a license-key gate — 100% local with Ollama (except optional Tavily web search).

**Stack**: Tauri v2 + React 19 + TypeScript 5.8 + Vite 7 + Tailwind CSS 4 + shadcn/ui + Zustand 5 + i18next (FR/EN/ZH)

**Reference docs** (maintained alongside the code — update them when shipping a feature):
- `Docs/Technique/TECHNICAL.md` — full technical documentation + per-version changelog
- `Docs/Fonctionnel/FUNCTIONAL.md` — functional/user documentation
- `Docs/Technique/LICENSE-KEYGEN.md` — license key generation and crypto flow
- `Docs/Technique/AUDIT-2026-09-16-consolidation-deepseek.md` — the v1.16 audit + lot plan (decisions, test plan, deviations)
- `Docs/Technique/AUDIT-2026-09-16-arene-vivante.md` — the v1.17→1.20 "arène vivante" audit: 19 lots, 48 simulations, §8 execution journal with assumed deviations (read §8 before touching the engine)
- `TODO.txt`, `TOKENS.txt` (repo root) — backlog and token-budget design notes

## Build & Development Commands

```bash
# Development (Vite hot-reload + Tauri window)
npm run tauri dev

# Production build → src-tauri/target/release/bundle/ (MSI + NSIS installers + standalone .exe)
npm run tauri build

# TypeScript type-check only
npm run typecheck    # tsc --noEmit

# Frontend unit tests (vitest, node env — stores, reducers, pure helpers, replayed event fixture, *.test.ts under src/)
npm test             # 95 tests
npm run test:watch

# i18n parity gate (fr.json is the reference; fails on missing/extra keys)
npm run i18n:check   # also runs first in `npm run build`
node tools/i18n-add.mjs additions.json   # merge {fr,en,zh} nested keys into the locale files

# Rust tests (525 inline #[cfg(test)] modules incl. engine/engine_tests.rs — there is no tests/ directory)
cd src-tauri && cargo test --lib

# Deterministic emotion simulations (engine/emotion_sim.rs) — trajectories for the audit journal
cd src-tauri && cargo test --lib emotion_sim_report -- --ignored --nocapture

# Regenerate the event fixture replayed by the frontend (src-tauri/fixtures/events-full.json)
cd src-tauri && cargo test --lib export_event_fixture -- --ignored

# Prompt bench on a real model (report in target/bench/, references under Docs/Technique/bench/)
cd src-tauri && AIRENA_BENCH_PROVIDER=ollama OLLAMA_MODEL=<model> cargo test --lib bench_prompts -- --ignored --nocapture
#   AIRENA_BENCH_SCENARIOS=debat,ideation restricts the run; raw events land in target/bench/<date>-<provider>-<scenario>.events.json
#   AIRENA_BENCH_RUNS=3 plays every scenario three times and reports the mean; AIRENA_BENCH_TAG=x3 suffixes the report names
cd src-tauri && AIRENA_BENCH_REPLAY=target/bench/<date>-<provider> cargo test --lib bench_replay -- --ignored --nocapture   # recompute metrics from saved events
node tools/bench-compare.mjs <before.json> <after.json> [--strict]

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

**Quality ratchets** (never go below): Rust ≥ 530 tests, front ≥ 95 tests, clippy 0 on `--all-targets`, exact i18n parity (1 301 keys), no `pages/*` file > 400 lines (SetupPage 1561 → 195, SettingsPage 933 → 100; `PersonaEditor` at 596 lines is the pre-existing exception).

## Architecture

### Backend (src-tauri/src/)

The Rust backend is a single library crate (`airena_lib`). `main.rs` just calls `lib.rs::run()`.

**Core flow**: `lib.rs` initializes logging, SQLite DB, seeds profiles, creates `AppState`, and registers ~30 Tauri commands.

- **state.rs** — `AppState` holds `engine_cmd_tx` (mpsc channel to running engine), `cancel_token`, `db` connection, and `rag_store` (RAG document store). Uses `std::sync::Mutex` (not tokio) because locks are never held across `.await`. Helper methods: `get_settings()`, `clear_engine_slots()`, `lock_or_recover()`.
- **constants.rs** — Centralized tunable parameters, grouped by `// ── Section ──` headers (memory, prompt truncation, search, orchestrator, Tavily, Wikipedia, Ollama, emotions, moderation, search dedup, think mode, temperature, LLM defaults, RAG, token budget, argument map, license). All magic numbers/strings live here.
- **error.rs** — `CommandError` enum (`Ollama`, `Settings`, `History`, `Rag`, `License`, …) serialized to the frontend as `{ kind, message }`.
- **license.rs** — License key decode/verify (see License System below).
- **commands/** — Tauri IPC handlers grouped by domain: `discussion.rs` (incl. `audience_vote`, `get_engine_constants`), `casting.rs` (`suggest_casting`, v1.19), `templates.rs` (discussion templates, v1.20), `diagnostics.rs` (`read_backend_log`, `get_app_version`, v1.20), `ollama.rs`, `settings.rs` (also profiles + license, `get_tuning_info`, persona memories), `history.rs` (search, tags, favourite), `rag.rs`, `llm.rs` (`get_llm_constants`, `list_deepseek_models`, `validate_deepseek_key`, `get_llm_usage_period`, `reset_llm_usage_period`, `cloud_budget_preflight` shared by `start_discussion` and `suggest_casting`)
- **llm/** — Provider abstraction (v1.16, extended v1.20). `mod.rs`: `LlmProvider` trait (`chat_stream`/`chat`/`validate`/`capabilities`, plus the defaulted `model_for(request)` / `capabilities_for(speaker)`), `openai_compat.rs`: the shared SSE transport (`OpenAiCompatTransport`, `Dialect::{DeepSeek, Generic}`) and the generic `OpenAiCompatProvider` (no reasoning, never billed, empty key allowed), `routing.rs`: `RoutingProvider` (one inner provider per speaker model, default for everything else), `LlmRequest` builder (`.json()` also pins `TEMP_JSON_OUTPUT`, `.reasoning()`, `.pace()`, `.speaker()`), `LlmResponse`, `LlmError`; `LlmCapabilities.max_parallel_calls` (Ollama 1, DeepSeek 4) bounds every concurrent round through `parallel.rs::run_bounded` (order preserved). `ollama.rs`: iso-functional adapter over `ollama/client.rs`. `deepseek.rs`: the transport configured for the DeepSeek dialect (thinking + `reasoning_effort`, explicit `max_tokens`, top_p floor / temperature omitted while thinking, backoff retry only when nothing was emitted, idle timeout, `/models`, `/user/balance`). `pricing.rs`: dated price list + UTC peak windows. `metered.rs`: `MeteredProvider` decorator (usage ledger by call kind/speaker, cost, fatal-error latch). `factory.rs`: `build_provider(&AppSettings)` and `build_provider_for(&AppSettings, &[SpeakerModel])` (routing when speakers override the model; every model validated before the engine spawns). `mock.rs`: `MockLlmProvider::scripted()` for engine tests.
- **models/llm.rs** — `ProviderKind`, `ReasoningLevel` (`Off/Low/High/Max/Auto`), `ReasoningPace` (`Normal/Fast`: caps `Auto` at `Low`, halves the DeepSeek reasoning allowances), `CallKind` (`Thought` is legacy — the engine issues `Intention`; `TurnAnalyst` fuses memory + emotion on sequential providers), `LlmUsage`, `UsageLedger`, `PeriodUsage`, `PeriodHistoryEntry`.
- **db/rolling_period.rs** — Monthly rolling period shared by Tavily credits and DeepSeek spend (`rollover()`).
- **engine/mod.rs** — Shared utilities: `truncate_str()`, `truncate_tail()` (UTF-8–safe), `truncate_at_word_boundary()` (truncates at last space, appends `"…"`), `truncate_at_sentence_boundary()` (whole sentences only — model output shown as is, v1.20.4), `apply_i8_clamped()` (emotion delta)
- **engine/token_budget.rs** — Waterfall token budget system: computes per-speaker character budgets from `num_ctx`, model params, and user-configured section priorities. `TokenBudget::compute()` allocates in strict rank order (floor → proportional fill → ceiling). `BudgetSection` enum (10 variants: CurrentTurnMessages, ImmediateMemory, ContextualSummary, CognitiveDirectives, ArbitreDirectives, FullDocument, RagContext, WebWikiSearch, PositionalMap, OpenLoops), `SectionPriority` (rank + floor/ceiling), `TokenBudgetPreview` for frontend display. Sets `full_document_mode` when the entire RAG document fits in the `FullDocument` allocation. Sections added after v1.16 (`ADDED_AFTER_V116`) are appended to a saved order instead of invalidating it; the output reserve is `num_predict × EMOTION_LEN_MAX`; the deterministic overhead includes the intention block.
- **engine/orchestrator/** — `DiscussionEngine` (directory module since v1.20: `mod.rs` = struct, lifecycle and the turn loop; `staging.rs` = dramaturgy, scene events, coalitions; `structured.rs` = structured modes, agendas, persona memory; `knowledge.rs` = web/wiki/RAG retrieval; `analysis.rs` = end-of-turn analysis and argument map — each child is one `impl DiscussionEngine` block with `pub(super)` methods): the main discussion loop over an `Arc<MeteredProvider>`. Handles introduction → turn loop (speaker order → deferred reactions → web/wiki/RAG search → focus + reasoning level → directive → **intention** (JSON contract, open loops shown) → intervention (emotion-modulated sampling) → rule emotions + room mood → moderation (room-mood hint) → immediate reaction round → usage/budget guard) → end of turn (`run_end_of_turn_calls`: document, emotion analysis, memory, argument map prepared from `&self`, run through `run_bounded`, applied in the historical order; fused `TurnAnalyst` when `max_parallel_calls == 1`; contagion, room mood, relationship decay and history in between) → `TurnTimings` → synthesis → `DiagnosticsReady` → end. Also owns search dedup (`is_duplicate_query()`), the per-speaker `rag_cache`, stagnation signals (`is_stagnating()`), `pick_focus()`, the sources registry, open loops, relationship scores, stage directions (feed only, never in prompts), budget alerts and period persistence. End-of-turn steps run on the last turn too (only a hard stop/cancellation skips them).
- **engine/emotion_sim.rs** — Deterministic simulations of the emotion model (v1.20.5, test-only): scripted reaction streams, analyst deltas and contagion over eight turns; asserts the balance properties (no saturation, sensitivity, variability, recovery, persona differentiation, bounded theatre); `emotion_sim_report` (ignored) prints the trajectories.
- **engine/engine_tests.rs** — End-to-end engine tests on `MockLlmProvider` (event sequence, metering, reasoning fallback, cancellation, fatal errors, budget, focus rotation, stagnation, ban, fiction opening, UserDriven passes, per-turn document, socratic memory, reactions S1–S7, intention/open loops/positions/pace S8–S12, emotions S22/S27–S31, pipeline S35–S38). The mock defaults to `max_parallel_calls = 2` (separate end-of-turn calls); `sequential_caps()` exercises the fused analyst. `export_event_fixture` (ignored) writes `fixtures/events-full.json`, replayed by `src/stores/arena/fixture.test.ts`. Extend these when touching the loop.
- **engine/reactions.rs** — Reaction rounds: scope (previous turn / last intervention), OCEAN propensity (E → frequency, A → severity), per-mode allowed colours, hints for the argument extractor.
- **engine/open_loops.rs** — `OpenLoopRegistry`: questions, commitments and **objections** (v1.20.1: a counter-argument of the map nobody answered, opened on the speaker it hits) per speaker (FIFO ≤ 3, TTL = 2 **own** interventions — banned speakers keep theirs).
- **engine/relationships.rs** — `RelationshipScores`: decaying weighted scores next to the integer counts, classification on the net warmth per direction, edges with `score`/`trend`.
- **engine/stage_directions.rs** — Theatre lines without LLM (persona `<dynamics>` first sentence or trilingual templates), one per speaker and turn at most.
- **engine/tuning.rs** — `Tuning` (defaults = constants, serialisable, bounded by `Tuning::BOUNDS`, `validate()` clamps) consumed by gains, sampling modulation, relationships, scene-event and coalition probabilities; overridden by the advanced settings (`advanced_tuning_json`).
- **engine/diagnostics.rs** — `DiagnosticsCounters` (parse failures by call kind, refusals, retries, intention compliance) and `TurnTimer`.
- **engine/bench.rs**, **engine/bench_metrics.rs** — Ignored real-model bench and its pure metrics (repetition, name usage, Markdown leaks, refusals, length, intention compliance, speaker gap, argument depth, and since v1.20.5 reactions per intervention, 💡 share, critical share, emotional peak and saturated share); `AIRENA_BENCH_RUNS` averages several runs (`BenchMetrics::mean`), `bench_replay` recomputes a saved run; name matching tolerates a missing article (`json_parser::mentions_name`, shared with the engine's intention-compliance diagnostic); compare with `tools/bench-compare.mjs`.
- **engine/dramaturgy.rs** — `ActKey` (31 acts), scripts per mode (13 modes), `resolve_act` (proportional with a turn limit, sliding without, stagnation → concessions, soft stop → closing statements), trilingual announcements / speaker instructions / moderation hints.
- **engine/mode_roles.rs** — Roles of the structured modes (v1.19): trial (prosecutor, defence, witness, juror) and Oxford (for, against) dealt from `mode_role` or the casting order, six hats rotating per turn (`hat_for`), trilingual labels and the "[Ton rôle]" block appended to the persona (`system_prompt_for`).
- **engine/scene_events.rs** — `SceneEvent` (surprise fact, format constraint, audience question **with its text**, forced steelman, duel, hot seat, crisis dispatch), pure policy (`pick_scene_event`, seeded-RNG tested; `used_kinds`: a kind is drawn again only once every other available kind was played), templated announcements (the **brief** the moderator voices) and per-speaker instructions. The engine materialises events in `stage_turn` (may reorder or shorten the speaker order) and forms coalitions (`SpeechAct::Relay` on the leader, follower right after).
- **engine/focus.rs** — Rotating conversational focus: weighted draw (`FOCUS_WEIGHT_*`) among recent speakers not yet targeted this turn, related participants, or the topic. Not used in fiction / UserDriven.
- **engine/cast.rs** — Who is who (v1.20.3): `portrait_from_kernel` reads role, creed and register from the persona `<identity>` / `<voice>`; `build_cast_block` renders "[Les autres participants — qui ils sont]" (speakers) or "[Ton plateau — qui ils sont]" (moderator), bounded by `CAST_PORTRAIT_MAX_CHARS` / `CAST_BLOCK_MAX_CHARS` and reserved in the budgets.
- **engine/argument_merge.rs** — Merges LLM extractions into the `ArgumentMap`: normalised-token similarity (accents, stop words, CJK per character), thesis dedup (`ARGMAP_THESIS_SIMILARITY_THRESHOLD`) vs looser reference resolution (`ARGMAP_REFERENCE_SIMILARITY_THRESHOLD`), orphan counter-arguments parked in a per-speaker "unattached" bucket, `MergeReport` (new node ids, deduplicated, unattached, dropped).
- **engine/turn_manager.rs** — Turn distribution: Sequential, Random, Democratic (masked Borda voting via parallel LLM calls), Authoritarian (IArbitre decides)
- **engine/prompt_builder.rs** — Builds context-aware prompts for each speaker turn, reactions, emotions, synthesis, search decisions (with past-queries block), argument extraction, and end-of-discussion awareness
- **engine/directive_builder.rs** — Cognitive personality system: 5-layer behavioral directives based on emotions, relationships, speech acts (10 types: Challenge, SteelMan, Anecdote, etc.), self-memory anti-repetition, and situational awareness
- **engine/dynamics_parser.rs** — Parses `<dynamics>` XML sections from system prompts for cognitive personality fields (values, triggers, under_pressure, etc.) with trilingual labels
- **engine/mode_prompts.rs** — Mode-specific instructions: introduction, intervention, thought focus, synthesis, and moderation criteria per `DiscussionMode`
- **engine/emotion_engine.rs** — Rule-based emotion updates (6 axes: engagement, accord, confiance, frustration, curiosité, enthousiasme): typed reaction tallies (`ReactionTally`, `GivenReactions`), `PersonaGains` from OCEAN (neutral 4..=7 = the raw event). **Realistic model (v1.20.4)**: diminishing returns on repeated reactions of a kind (`scaled`, harmonic), one reception round capped per axis (`EMOTION_ROUND_AXIS_CAP`, the cap scales with the persona's sensitivity — `AxisDeltas::apply_felt`), elastic resistance near the extremes (`resistance` / `elastic`: full effect inside the comfort band `EMOTION_COMFORT_HIGH`, then a **quadratic** brake down to `EMOTION_EXTREME_RESISTANCE` at 0/100 — v1.20.5 —, coming back is free), homeostasis on all six axes toward the persona's **initial** profile (`EMOTION_HOMEOSTASIS_PERCENT` of the distance, one point at least; the explicit rates of frustration / enthousiasme are the floor), LLM deltas `clamp_delta()` then `apply_llm_delta` (elastic), movements from the baseline (`dominant_shift`, `ShiftZones` with hysteresis `EMOTION_SHIFT_REARM`, `EMOTION_NOTABLE_SHIFT`) feeding the directive's movement line, the stage cues and the reasoning heuristic (`THINK_SHAKEN_BOOST`); `apply_ban_penalty()`, `accord` from reactions given (factor 2, cap 6, symmetric), real stagnation flag, `text_similarity()` (Jaccard), contagion, `modulate_sampling()` (enthusiasm → temperature, engagement → `num_predict`, interventions only), `room_mood()`. The LLM analyst only adjusts for tone/content (drops expected as much as rises) and may set `"stagnating"` (`json_parser::parse_emotion_analysis`).
- **engine/memory_manager.rs** — Maintains discussion summaries and participant positions with their trajectory (`initial_stance`, `shift`, `would_change_if`; a blank update keeps the previous stance, absent participants are kept)
- **engine/json_parser.rs** — Parses LLM JSON responses with fuzzy speaker name matching (exact → article-stripped → prefix → contains); `parse_intention`, tolerant `MemoryUpdateResponse` (`PositionInput` string | object, `open_questions`), `parse_turn_analyst`, `parse_moderation` returns `Result` so failures are counted
- **ollama/client.rs** — `OllamaClient`: HTTP streaming via reqwest, supports think mode, 3-attempt retry. `stream_ndjson()` unified streaming function. `strip_think_tags()` removes leaked `<think>…</think>` blocks from non-streamed content. Also `show_model()`, `list_running_models()`, `preload_model()`, `unload_model()`.
- **ollama/model_info.rs** — Parses `/api/show` into `ModelArchInfo` (family, layers, KV heads, quantization, KV bytes/token), detects GPU VRAM via `nvidia-smi` (`detect_gpu_vram()`, NVIDIA only — AMD needs manual config), and computes `recommend_num_ctx()`. `detect_think_support()` checks the model's Go template for `.Think`. `build_model_budget_info()` feeds the frontend `VramIndicator` / `TokenBudgetPreview`.
- **rag/** — RAG system: `parser.rs` (PDF/TXT/MD/CSV/DOCX parsing), `chunker.rs` (text splitting with overlap), `embedder.rs` (Ollama embeddings), `bm25.rs` (lexical search), `store.rs` (in-memory document/chunk/embedding storage with hybrid search + full-text access)
- **tavily/client.rs** — `TavilyClient`: Tavily web search API (requires API key). Per-agent quota enforcement (per-discussion + per-turn). Credit tracking in settings.
- **wikipedia/client.rs** — `WikiClient`: Wikipedia search with language mapping (fr/en/zh→en fallback), smart disambiguation filtering via keyword scoring
- **db/** — SQLite via tokio-rusqlite: `schema.rs` (migrations), `repository.rs` (all queries with `row_to_profile()` + `PROFILE_COLUMNS` DRY helpers, license tracking), `seed.rs` (~120 predefined profiles, ~4000 lines)
- **models/** — All data structures with `Serialize`/`Deserialize`. Key types: `ArenaEvent` (40+ variants), `SpeakerRole`, `EmotionalProfile`, `RoomMood`, `TurnDistribution`, `DiscussionMode` (8 variants), `DiscussionFeatures` (reaction timing, audience, scene events, hidden agenda, coalitions), `DocumentFormat`, `DocumentInjectionMode`, `ArgumentMap`, `ReactionType` (6 colours + classes), `MessageKind` (normal, banNotification, stageDirection, actAnnouncement, sceneEvent), `Intention`/`IntentionGoal`, `SourceRecord`/`WebSourceInfo`/`WikiSourceInfo`, `TurnTimings`/`DiscussionDiagnostics`
- **models/argument_map.rs** — `ArgumentMap`, `ThesisNode`, `ArgumentNode` (recursive `children`), `ArgumentType` (Support/Counter/Evidence). `to_markdown(topic, new_ids)` (by thesis) and `to_markdown_by_speaker(topic, new_ids, lang)` produce hierarchical markdown for markmap rendering; nodes in `new_ids` get the ✨ prefix and argument-only speakers get an "Arguments" branch.
- **models/relationship.rs** — `RelationshipKind`, `RelationshipTrend`, `RelationshipEdge` (undirected reaction edge with both directions, classified kind, decaying net `score` and `trend`), emitted as `RelationshipsUpdated` by `RelationshipScores::edges()`; class changes emit `RelationshipShift`.

### Frontend (src/)

Path alias `@/` → `src/` (tsconfig + vite).

- **pages/** — 7 pages: Home, Setup (wizard shell — steps live in `components/setup/steps/`), Arena (live discussion), Summary, History, HistoryDetail, Settings (composition of `components/settings/*` + debounced autosave)
- **stores/useUiStore.ts** — Presentation preferences (`stageVisible`, persisted), `presentationMode` (never persisted), `viewedTurn`. Audio toggles are durable settings, not UI prefs.
- **stores/** — Zustand 5 stores: `useArenaStore` (orchestration, side effects, `buildReport()`, `reactAsAudience()`; the data lives in `stores/arena/types.ts` and is updated by pure reducers `stores/arena/reducers/{messages,knowledge,emotions,turn,artifacts,sources}.ts` — testable in node), `useSetupStore` (config including mode/format/wiki/document injection mode/document update granularity/features), `useSettingsStore` (app prefs + license + provider + `engineConstants`; helpers `describeActiveModel()`, `needsOllama()`, `loadLlmConstants()`), `useToastStore` (global notifications rendered by `ToastContainer`). Each store has a vitest file next to it; `stores/arena/fixture.test.ts` replays the engine fixture.
- **lib/report.ts** — `DiscussionReport` v1 (sources, timeline, emotion history, relationships, positions, agendas, awards, outcome, timings, diagnostics) persisted as `discussions.report_json`; `parseReportJson()` tolerates older discussions. `lib/sources.ts` attaches sources to messages (citation heuristic) and exports Markdown; `lib/reactions.ts` maps colours to emoji/tone.
- **lib/tauri-api.ts** — All Tauri command wrappers. `startDiscussion()` creates a `Channel<ArenaEvent>` for event streaming.
- **lib/cost-estimate.ts** — `estimateTurnCost()` (order-of-magnitude cost per turn for cloud providers), `formatUsd()`.
- **lib/discussion-config.ts** — `applyGlobalContext()`: the global `numCtx` (Ollama window / DeepSeek budget) is applied to every speaker right before `startDiscussion`.
- **lib/error-utils.ts** — `extractErrorMessage()`: normalizes caught values (`Error`, Tauri `CommandError` `{kind, message}`, strings). Always use it instead of `String(e)`.
- **lib/logger.ts** — Structured frontend logger with circular buffer and export.
- **lib/persona-\*** — Persona system: `persona-types.ts` (OCEAN, Posture, Identity, Psychology, Voice, Dynamics types), `persona-parser.ts` (parses `<system_kernel>` XML → PersonaData), `persona-serializer.ts` (PersonaData → XML), `persona-labels.ts` (trilingual field labels)
- **lib/profile-emoji.ts** — Maps predefined profile names → emojis, keyword regex → emojis, hash fallback
- **lib/document-diff.ts** — Diff strategies for co-constructed documents (see below)
- **components/discussion/** — DiscussionFeed (turn dividers + "jump to latest", no auto-scroll, audience wiring), MessageBubble (message kinds, typed reaction chips with quotes, sources toggle, thought vs model reasoning, `StreamingBubble` with the reasoning chronometer), ReactionBar (audience), SpeakerBadge, TurnIndicator (room-mood pill), UsagePill (tokens/cost/peak), SpeakerQueue (`deriveQueue()`), UserInputArea, DiscussionControls, ReadOnlyFeed
- **components/report/** — `DiscussionReportView` shared by Summary and HistoryDetail (tabs Synthesis / Discussion / Replay / maps / Positions / Sources, downloads, `OutcomePanel` (verdict / agreement / audience swing) above the synthesis, `AwardsCredits`, `DiagnosticsPanel` collapsed by default), `PositionsTable`, `AgendaCards` (unveiled agendas, Positions tab), `ReplayPlayer` (schedule from `lib/replay.ts`, stage animated from the report). **components/sources/** — `SourcesList`, `SourcesPanel` (arena tab).
- **components/setup/CastingAssistant.tsx** — `CastingAssistant` (one `suggest_casting` call, replace / add the cast, suggested moderator) and `CompatibilityMatrix` (probable allies / friction from OCEAN, `lib/casting.ts`). `lib/modes.ts` mirrors the Rust mode facts (hidden-agenda modes, selectable roles, fixed-order modes, vote choices).
- **components/discussion/AudienceVote.tsx** — Oxford vote window (for / against / skip) driven by `audienceVoteRequested`.
- **components/stage/** — `StageView` (props) / `ArenaStage` (store): arc of avatars with role / hat pills, spotlight, auras (`lib/stage.ts`), coalition link, flying reactions (`arena-burst` keyframes); `SceneBanner` (`aria-live`), `TimelineBar` (jump to `#turn-N`), `Scoreboard` (`lib/score.ts`).
- **hooks/useArenaShortcuts.ts**, **hooks/useArenaAudio.ts** — Desktop shortcuts (`lib/stage.ts::shortcutAction`, never inside inputs/buttons; `N` = next speaker in step mode) and the audio layer (`lib/speech.ts` sentence-by-sentence TTS with OCEAN prosody, `lib/sounds.ts` procedural Web Audio; `useAudioSettingsSync` lives in `AppShell`, the arena registers an `AudioSink` on the arena store). `lib/presentation.ts` toggles the projection mode (fullscreen needs `core:window:allow-set-fullscreen`).
- **components/emotion/** — EmotionSidebar exports `EmotionPanel` (bars/radar toggle, threshold flashes from `lastThresholdCrossed`), ParticipantEmotionCard (backstage: intention + directive incl. focus + reasoning level), EmotionRadar (SVG), EmotionSparkline, EmotionAxisSlider
- **components/document/** — DocumentSidebar exports `DocumentPanel`: real-time document co-editing panel with format-specific rendering (txt, md via SimpleMd, csv as table)
- **components/mindmap/** — MarkmapViewer (forwardRef, `getSvgHtml()`, `fit()`; keeps the user's zoom once they interacted), MindmapSidebar exports `ArgumentMapPanel` (counters, ✨ new, dropped warning, per-speaker chips, recentre)
- **components/relations/** — RelationsGraph (ring layout, edges coloured by ally/rival/tense)
- **components/layout/** — AppShell, TopBar, Sidebar, ResizeDivider, RightPanel (tabbed side panel: Emotions / Document / Map / Relations; column on `lg`, drawer below; tab/width/collapse remembered in localStorage)
- **components/settings/** — SettingsPrimitives (Section/Field/StatusPill/ChoiceRow), GeneralSettings, LicenseSettings, ProviderSettings (reasoning level, live reasoning, reasoning pace), DeepSeekSettings (key validated before it is stored, models, context budget, monthly cap, period gauge/history/reset), OllamaSettings (`chatEnabled=false` → embeddings only), TokenBudgetPriorities (`parsePriorities` appends sections newer than the saved order), TavilySettings
- **components/setup/** — LlmParamsForm (Ollama vs DeepSeek variant: reasoning level, temperature disabled while thinking, top_p floor, bounds from `get_llm_constants`), EmojiPicker, PersonaEditor, OceanSliders, TokenBudgetPreview (pages from backend + cost line), VramIndicator, `steps/` (StepTopic with `LivelinessOptions` — reaction timing + features —, StepArbitre, StepGladiateurs, StepKnowledge, StepSummary + shared primitives)
- **components/shared/** — MathText, SimpleMd, StatCard, UsageSummaryCard, ToastContainer
- **components/common/ErrorBoundary** — top-level React error boundary
- **hooks/useTokenBuffer.ts** — 60ms token batching to prevent WebView crash from per-token React re-renders
- **hooks/useMediaQuery.ts** — reactive `matchMedia` (`LG_BREAKPOINT_QUERY` drives the panel layout)
- **i18n/** — i18next with FR (default), EN, ZH locales. Every user-facing string goes in all three `locales/*.json` files.
- **providers/ThemeProvider** — Dark/light theme via CSS variables

### IPC Pattern

The engine runs in a Tauri-spawned tokio task. It emits `ArenaEvent` variants through a Tauri `Channel`. The frontend's `useArenaStore.handleEvent()` dispatches these to update Zustand state. Commands flow back via `mpsc::Sender<EngineCommand>` (pause/resume/stop/intervene).

Tauri permissions are declared in `src-tauri/capabilities/default.json` (`core`, `opener` — `openExternalUrl()` only opens http(s) links —, `dialog`, `fs` + `fs:allow-write-text-file` for exports).

Commands flow back via `mpsc::Sender<EngineCommand>` (pause/resume/stop/intervene/**audience reaction**); `get_engine_constants` exposes the engine limits the UI mirrors (never duplicate them in TS).

### Discussion Modes

13 modes in `DiscussionMode` enum: `Debate` (default), `Ideation`, `CoConstruction`, `UserDriven`, `Socratic`, `Tutorial`, `CritiqueReview`, `CollaborativeFiction`, and since v1.19 `Trial`, `OxfordDebate`, `Negotiation`, `SixHats`, `CrisisCell`. Each mode has tailored instructions in `mode_prompts.rs` covering introduction, speaker posture, thought focus, synthesis, and moderation criteria (the sweep test `every_mode_has_its_full_set_of_texts_in_three_languages` enforces completeness; `DiscussionMode::ALL` is the test-only list). `CoConstruction` mode supports output in `DocumentFormat` (None, Txt, Md, Csv).

**Structured modes (v1.19).** `DiscussionMode::supports_hidden_agenda` (Debate, CollaborativeFiction, Trial, OxfordDebate, Negotiation — a negotiation always has agendas). Roles: `GladIAteurConfig.mode_role` → `mode_roles::resolve_role` (trial: prosecutor, defence, then juror / witness alternating; Oxford: alternating camps), `RolesAssigned` once on turn 1; six hats re-dealt every turn (`hat_for`). Outcome (`models/outcome.rs`, `ModeOutcome` tagged by `kind`): trial → one `CallKind::Verdict` JSON call per juror (`parse_verdict`, majority, the moderator rules alone without jury or breaks a tie, `by_arbitre`); negotiation → one `Verdict` call per party (`parse_agreement`, reached when all sign, unusable = refusal); Oxford → `AudienceSwing` from the two vote windows (`collect_audience_vote`: `AudienceVoteRequested` after the introduction and after the last turn, bounded by `user_intervention_timeout_secs`, `SkipUserTurn` closes it, `audience_vote` command). The outcome is resolved before the synthesis, emitted as `OutcomeReady`, injected into the synthesis prompt (`build_outcome_synthesis_block`) and persisted in `report_json.outcome`. Crisis cell: one `CallKind::CrisisDispatches` call after the introduction (`max_turns` dispatches, default `CRISIS_DISPATCH_DEFAULT_COUNT`, cap `CRISIS_DISPATCH_MAX_COUNT`), one `SceneEvent::Dispatch` per turn instead of the random scene policy.

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

### Reactions, Intentions, Open Loops (v1.17)

- **Reaction rounds**: `DiscussionFeatures.reaction_timing` — `Immediate` (a round of `CallKind::Reaction` per other active speaker right after each intervention, `run_bounded` by `max_parallel_calls`, effects applied before the next speaker) or `Deferred` (v1.16: each speaker reacts to the previous turn before speaking). Six typed colours with an exact `quote` (`validated_quote`), OCEAN propensity in the prompt, fiction restricted to `REACTION_TYPES_FICTION`. Reactions are attached to the engine's messages too (they reach the prompts of the turn and the argument extractor's hints).
- **Audience**: `react_to_message` → `EngineCommand::AudienceReaction` (also drained while paused), capped by `AUDIENCE_REACTIONS_PER_MESSAGE_MAX`, felt at once by the target (gladiateur or moderator), favoured by the next focus draws. The store shows the chip optimistically (`pending`) and withdraws it on error.
- **Intention**: every intervention is preceded by `CallKind::Intention` (JSON, `Off`, addressable names = other active gladiateurs + the user when they spoke; none in fiction). Prose answer → persona thought (v1.16 behaviour); broken JSON → nothing, counted in diagnostics. Unknown/banned target → the turn's focus or the topic. The "[Ton intention]" block is bounded by `INTENTION_BLOCK_MAX_CHARS`; compliance (target named in the text) is measured, never enforced by regeneration.
- **Open loops**: fed by intention questions (to the target), concessions (own commitment), justified `question` reactions, the memory analyst's `open_questions` and, in the argumentative modes (`DiscussionMode::rewards_depth`), the objections of the argument map (`ArgumentMap::unanswered_objections`, one per debtor and turn, opened right after the extraction); `answers: n` closes loop n; TTL counts the owner's own interventions. Shown in both the intention prompt (numbered) and the intervention prompt (`BudgetSection::OpenLoops`, floor 300 so 8k contexts still show one or two).
- **Positions**: `ParticipantPosition` keeps `initial_stance`/`shift`/`would_change_if`; names are normalised to known participants (gladiateurs + user) before merging; `PositionsUpdated` after each memory update; the synthesis gets "[Évolution des positions]" when someone moved.

### Emotions Incarnées (v1.17)

- `PersonaGains` (from the OCEAN line `O=x C=x E=x A=x N=x` of the system prompt) scale the rule deltas; neutral traits (4..=7) keep the v1.16 numbers exactly — golden-tested.
- `modulate_sampling` only touches `CallKind::Intervention` requests and only when `emotion_driven`; JSON calls always keep `TEMP_JSON_OUTPUT`.
- Stage directions are `MessageComplete` events with `kind = stageDirection` — the frontend renders and persists them; the engine never pushes them into `messages_history`/`turn_messages`, so prompts never see them. Cap: `STAGE_DIRECTIONS_PER_SPEAKER_PER_TURN`.
- Relationships: `RelationshipScores` decays at the end of each turn (`RELATIONSHIP_DECAY_PER_TURN`); classification (v1.20.5, calibrated for the sincere-reaction regime where half of the reactions are neutral questions) judges the pair on the **sum** of both directions' net warmth (`RELATIONSHIP_ALLY_SCORE` / `RIVAL_SCORE` 2.5), each direction merely leaning the right way (`RELATIONSHIP_MUTUAL_MIN` 0.8): ally, rival (no fresh approval either way — a like from a rival thaws the rivalry into a tension → `RelationshipShift`, a stage direction and a one-off "reconciliation" line in the target's next directive), tense (one warm and one cold with a spread ≥ `RELATIONSHIP_TENSE_SCORE`, **or one direction cold on its own by that much** — a persistent critic). `RelationshipScores::lean_of` names the critic; the layer-2 tense hint says who it is (`RelationshipLean`).
- Room mood needs ≥ 2 active gladiateurs; it is remembered for the moderation prompt and emitted as `RoomMoodUpdated`.

### Platform (v1.20)

- **Providers**: `ProviderKind::{Ollama, DeepSeek, OpenAiCompat}`; the generic dialect never sends `thinking`/`reasoning_effort`, sends `stream_options.include_usage` and `response_format: json_object`, and treats a missing `/models` catalogue (HTTP 4xx / unreadable payload) as acceptable — an unreachable server or a refused key fails `validate()`. The transport timings are the `DEEPSEEK_*` constants (shared). Speaker model overrides (`GladIAteurConfig.model`, `IArbitreConfig.model`) go through `RoutingProvider`; `MeteredProvider` prices with `model_for` and fills `UsageLedger.by_model`; the engine reads `capabilities_for(speaker)` for reasoning decisions. Frontend: `availableModels` / `describeActiveModel(settings, overrides)` in `useSettingsStore`.
- **Templates / history**: `discussion_templates` (builtin rows are never overwritten nor deleted: `ON CONFLICT … WHERE builtin = 0`), `TemplateConfig` lives only in `src/lib/templates.ts` (the backend stores an opaque JSON object). FTS5 index `discussions_fts` created when available, backfilled by `schema::backfill_fts`, fed inside `save_discussion`'s transaction, purged on delete; `search_discussions` falls back to a `LIKE` scan when FTS is missing or rejects the query (`fts_query` quotes every term). Tags/favourite are `UPDATE`s on the row; tags are normalised and bounded server-side (`repository::normalise_tags`, `HISTORY_TAGS_MAX` / `HISTORY_TAG_MAX_CHARS`), templates by `DiscussionTemplate::validate` (`TEMPLATE_NAME_MAX_CHARS` / `TEMPLATE_CONFIG_MAX_BYTES`). Column migrations live in `schema::COLUMN_MIGRATIONS` (one `has_column` check each); `v116_database_migrates_in_place` replays a v1.16 database.
- **Long memory**: the engine never writes `persona_memories` itself — it emits `PersonaRecapReady` and the frontend saves the recaps with the discussion (same transaction, FK cascade). Recall: `repository::recall_persona_memories` (BM25 over topic + recap, tokens < `PERSONA_MEMORY_MIN_TOKEN_CHARS` ignored, recency completes) → `build_memories_block` appended to the persona in `system_prompt_for` (reserved as `PERSONA_MEMORY_MAX_CHARS` for any speaker with a `source_profile_id`). No recap after a hard stop or a fatal error.
- **Tuning**: `Tuning::from_settings(json)` (partial JSON, `validate()` clamps into `Tuning::BOUNDS`, crossed pairs reset), applied by `set_tuning` in `start_discussion`; `SceneContext` carries the probabilities, `try_coalition` reads `coalition_probability`. The frontend persists only the knobs that differ (`serialiseTuningOverrides`).
- **Operations**: versions aligned to 2.0.0 (the first release since 1.16 — the 1.17→1.20.5 milestones stay named in code comments and in the audit journal); the Tauri updater is **not** wired (needs a signing publication point) — `AboutSettings` opens `RELEASES_URL`; `read_backend_log` returns the tail of today's log (API keys are masked before logging).

### Hidden Agendas and Casting (v1.19)

- After the introduction, `generate_agendas` issues one `CallKind::Agenda` JSON call per gladiateur (`build_agenda_prompt`, `parse_agenda`, bounded by `AGENDA_FIELD_MAX_CHARS`), run together with `run_bounded`; a failed / unusable answer leaves that speaker without agenda. The block "[Ton agenda secret — ne le révèle jamais explicitement]" (`build_agenda_block`, ≤ `AGENDA_MAX_CHARS + AGENDA_BLOCK_OVERHEAD_CHARS`, reserved through `BudgetFeatures.agenda_chars`) goes into the intervention **system** prompt right after the persona; the intention prompt only gets the objective as a reminder. The synthesis receives `[Agendas secrets]` and must write a "## Agendas" section; `agenda_outcome` reads that section only (partial → unknown) and `AgendaRevealed` follows `SynthesisComplete` (also after a skipped synthesis, with unknown verdicts).
- Casting: `commands/casting.rs::suggest_casting` builds the provider through the factory (never a running engine), runs the monthly cloud budget pre-flight, bounds the catalogue by the context (`catalogue_bound`, never cutting a line, moderators on a quarter), one `CallKind::Casting` call, ids filtered by `parse_casting`, cloud usage recorded even on error. The frontend matrix (`lib/casting.ts::analyseCast`) is local: A/E/O distance → allies / friction, two frontal profiles clash, cast contrast low / medium / high.

### Dramaturgy (v1.18)

- `stage_turn` runs once the speaker order is known and before `TurnStarted`: it announces a new act (`ActStarted` + an IArbitre line of kind `actAnnouncement` pushed into `turn_messages`/`messages_history`, so speakers see it as a moderator directive), may draw a scene event (`features.scene_events`), then a coalition (`features.coalitions`, never on top of a scene event). Duel → the two duellists only; hot seat → the target last; coalition → the follower right after the leader.
- Speaker prompts get a `StageBlock` ("[Mise en scène de ce tour]": act instruction + scene instruction); the closing act silences the generic end-of-discussion reminder. The moderation prompt gets the act's hint.
- **Soft stop** (`EngineCommand::Stop`) no longer breaks the speaker loop: `refresh_act_on_stop` switches the act to the closing one and the remaining speakers deliver closing statements, then the end of turn runs and the synthesis follows. Only `ForceStop` (and cancellation) ends a turn at once.
- Engine tests are deterministic: `run_engine_with` calls `disable_random_staging()`; tests force events with `force_scene_event(kind)` / `force_coalitions()`.

### End of Turn Pipeline (v1.17)

`run_end_of_turn_calls`: `prepare_document_turn` / `prepare_emotion_analysis` / `prepare_memory_update` (also snapshots the turn into every memory) / `prepare_argument_extraction` build owned `LlmRequest`s; `run_phase` runs each (retry once on an empty answer for memory/analyst/argmap) inside `run_bounded`; results are applied in the historical order with `apply_*`. With `max_parallel_calls == 1` the memory and emotion prompts are wrapped verbatim in one `TurnAnalyst` prompt (`build_turn_analyst_prompt`) whose answer is split by `parse_turn_analyst` (a missing part leaves its state). Every phase records its duration in `TurnTimer` → `TurnTimings`; parse failures/refusals/retries/intention compliance land in `DiagnosticsReady`, emitted right before `DiscussionEnded` (normal and fatal exits). The reaction-round/search overlap of the plan was **not** implemented (see the audit §8, Lot 7).

### Step Mode and Voice Controls (v1.20.1)

- **Step mode**: while the voice follows the live discussion (`ttsEnabled && ttsMode === "follow"`), the arena sends `set_step_mode(true)` and the engine waits for the audience's cue before **each** speaker (`await_cue`: `AwaitingCue { speakerId, speakerName }` right before `SpeakerActive`, released by `EngineCommand::NextSpeaker`, by leaving the mode, a stop or the channel closing; cues received ahead are counted in `cues_pending`). The other commands (pause, audience reactions, emotion adjustments, intervention requests) keep being served during the wait. UI: "Orateur suivant" button in `DiscussionControls` (shortcut `N`), `awaitingCue` in the arena store (cleared on `speakerActive`).
- **Voice controls** in the arena top bar: mute, pause / resume the voice (`SpeechEngine.paused`: the queue keeps growing, `resume` drains it), switch "follow" ↔ "read everything" (`ttsMode`, a durable setting).
- **Argument depth**: `ArgumentMap::unanswered_objections()` / `depth_stats()` (emitted as `depth` in `ArgumentMapUpdated`, shown by the map panel); the debtor of an unanswered counter gets an `Objection` open loop, the `Deepen` speech act (`SPEECH_ACT_DEEPEN_BONUS`, `SPEECH_ACT_DEEPEN_OWED_BONUS` when an objection is owed, 0 outside `rewards_depth` modes) names it, and the moderation prompt receives a one-line depth hint (`depth_hint_for`) plus a depth criterion in debate. The extraction parser reads loosely shaped answers (`loose_text` / `loose_list`: null or object theses, a single argument object, a missing type inferred from `against_thesis`) — real models produce them. Bench: `argmapMaxDepth`, `argmapDeepShare`, `argmapUnanswered`.
- **Backstage**: the intention fields are kept in full for the display (`INTENTION_DISPLAY_MAX_CHARS`); the prompt block applies its own bounds.

### Emotional Realism, Sincere Reactions, Whole Sentences (v1.20.4)

- **No saturation**: every rule effect goes through `AxisDeltas` — per-kind diminishing returns, a per-axis round cap scaled by the persona's gains (`apply_felt`, so the neutral OCEAN golden path still equals the raw event and a nervous persona still feels more), then `elastic` resistance near the extremes; the analyst's deltas take the same path (`apply_llm_delta`). `apply_turn_effects` is a homeostasis on all six axes toward `initial_emotions` (frustration / enthousiasme keep their explicit rates). `EmotionalProfile::apply_delta` is gone.
- **Movements, not levels**: `dominant_shift(current, baseline)` (≥ `EMOTION_NOTABLE_SHIFT`) gives the directive one movement line (`movement_line`: shaken, converging, hardening, hooked, cooled, calmed — never doubling a triggered state; `SpeakerTurnContext.baseline`), boosts the auto reasoning of a shaken speaker (`THINK_SHAKEN_BOOST`), and entering the notable zone (`ShiftZones::crossings`, kept per speaker and axis in `DiscussionEngine.shift_zones`, hysteresis: a zone is left only below `EMOTION_NOTABLE_SHIFT − EMOTION_SHIFT_REARM`) counts as a threshold crossing in `emit_threshold_events` (UI flash, timeline mark, stage direction) next to the absolute 85 / 15 thresholds.
- **Balance pass (v1.20.5)**: the moderator is settled once per turn by `settle_arbitre_emotions` (homeostasis toward the neutral profile + the stagnation penalty, before the contagion) instead of at every moderation; `update_arbitre_emotions` keeps the ban delta only. The bench measures sincerity and saturation (`reactions_per_intervention`, `reaction_insightful_share`, `reaction_critical_share`, `emotion_peak`, `emotion_saturated_share`; `bench-compare.mjs` guards them).
- **Sincere reactions**: the reaction prompt's example never shows `insightful` and rotates with the content (`reactions::example_reaction_kind`), `reactions::sincerity_rules` states that a reaction is an opinion (`none` is normal — about one intervention out of two —, `insightful` is earned — at most one out of five —, disagreement is said with the colours the mode allows, the justification is the reader's own view; the quantitative anchors are v1.20.5: models calibrate on numbers). Measured on a real debate before the change: 78 % of reactions were 💡; after: 11 % on mistral-small3.1, 36-43 % on deepseek-flash whatever the wording (anchors, "none" example). Hence two model-independent levers (v1.20.5): the answer carries `"reacts": true|false` decided first (`RawReaction::declined` — false, "non", "no" → a silence like `none`; absent → as before), and **the insightful credit**: a reactor may single out one strong point per `INSIGHTFUL_CREDIT_WINDOW` (5) reactions given — `DiscussionEngine.recent_given_kinds`, `insightful_credit()`, the prompt says the credit is spent, a 💡 given anyway is recorded as a like (`apply_parsed_reactions`).
- **Whole sentences**: `truncate_at_sentence_boundary` on voiced announcements (`ANNOUNCEMENT_MAX_CHARS` 900, `ANNOUNCEMENT_NUM_PREDICT` 220), stage directions (`STAGE_DIRECTION_MAX_CHARS` 240) and surprise facts (`SURPRISE_FACT_MAX_CHARS` 500).

### Realism: Voiced Announcements, Grounded Room Questions, Cast Awareness, Debate State (v1.20.3)

- **Announcements in the moderator's voice**: every act announcement and scene event goes through `voice_announcement` (one `CallKind::Announcement` call, `ANNOUNCEMENT_NUM_PREDICT`, briefed by the template via `build_announcement_prompt` with the moderator's recent openings; quotation marks stripped, bounded by `ANNOUNCEMENT_MAX_CHARS`; the template is the fallback on failure / refusal / cancellation). The moderator's in-character calls (introduction, moderation, synthesis, announcements) use `arbitre_system` = persona + cast block.
- **The room's question is written from the debate**: `generate_audience_question` (`CallKind::AudienceQuestion`, JSON, `build_audience_question_prompt` with the topic, the summary, the target's stance and open loops; `parse_audience_question`, ≥ `AUDIENCE_QUESTION_MIN_CHARS`) — no usable question, no event; the question travels in `SceneEvent::AudienceQuestion { target, question }`, the announcement and the target's staging block. Scene kinds cycle: `scene_kinds_used` feeds `SceneContext.used_kinds`.
- **Cast awareness**: `system_prompt_for` appends the speaker's cast block after the memories, before the role.
- **State of the debate**: `BudgetSection::DebateState` (rank 12, `BUDGET_FLOOR/CEIL_DEBATE_STATE`, active only with the argument map, in `ADDED_AFTER_V116`; document sections moved to ranks 13-14), rendered by `debate_state_block` (the `DEBATE_STATE_MAX_THESES` most argued theses with owner and open objections, then the `DEBATE_STATE_MAX_OBJECTIONS` newest unanswered objections) as "[État du débat]" in the intervention prompt.
- **Awards**: the winning sentences are shown in full (`AWARD_DETAIL_MAX_CHARS`).

### The Audience as a Participant, Form Variety (v1.20.2)

- **The user who speaks is a participant**: `handle_user_intervention` sets `user_reply_pending` (excerpt, `AUDIENCE_MESSAGE_EXCERPT_CHARS`) and `user_has_spoken`. The next speaker gets the user as **forced focus** (`pick_focus`, any turn, not in fiction / UserDriven), the directive's user reminder switches from "observer" to "answer them FIRST, by name" (`build_user_reminder` reads `SpeakerTurnContext.audience_message` / `user_has_spoken`; later speakers get "has taken part: answer them like any participant"), `addressable_names` lists the user, and the moderation prompt gets `audience_hint_for` when the intervention never names them. The debt is settled after that speaker's moderation. Once the user has spoken they join the focus draw of the turn, `known_participant_names` (argument extractor, analysts) and the merge's speaker map under `USER_SPEAKER_ID`. Frontend: `lib/stage.ts::spokenParticipants` / `userHasSpoken` — the user takes a seat on the stage (spotlight during their turn), is ranked by the score and awards, and joins the relations ring once an edge names them.
- **Form variety**: the engine remembers the openings of the moderator's own lines (`arbitre_recent_openings`, `ARBITRE_RECENT_OPENINGS`, `directive_builder::opening_of`) and counts `moderation_checks` / `moderation_comments`; `build_moderation_prompt` takes a `ModerationSituation { room_mood, hint, style }` whose `ModerationStyle` block quotes the openings ("never open two comments the same way, a tic at most once in three") and, past `MODERATION_COMMENT_RATE_MAX_PERCENT`, tells the moderator to answer `none`. Speakers: layer 4 quotes the openings of their last `SELF_MEMORY_MESSAGES` interventions ("not the same first words, not systematically the interlocutor's name"); the intention contract no longer demands the name in the first words. Every moderation decision is logged (`Moderation decided`).
- **Startup**: `commands/ollama.rs::startup_preload_plan` — the chat model is preloaded only when Ollama serves the discussion; with a cloud provider only the embedding model is loaded (no VRAM sweep, no `recommended_num_ctx`, so the DeepSeek context budget is never overwritten).

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

Two layers prevent repeated queries (`orchestrator/knowledge.rs`):
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

**Discussion saving**: Save from the frontend `discussionEnded` handler (not the engine), because the engine's `messages_history` holds neither the audience reactions' UI state nor the stage directions; the frontend also builds `report_json` (`buildReport()`) from its reducers.

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

SQLite at `{app_data_dir}/airena.db`. Tables: `settings` (key-value, includes `embedding_model` for RAG, `license_key`, license tracking, Tavily credits, `llm_provider`, `reasoning_level`, `show_model_reasoning`, `reasoning_pace`, `tts_*` / `sound_*` audio, `deepseek_*` key/model/budget/period), `predefined_profiles`, `discussions` (with `discussion_mode`, `document_format`, `document_content`, `argument_map_md`, `argument_map_md_by_speaker`, `argument_map_json`, `llm_provider`, `usage_json`, `estimated_cost_usd`, `report_json` columns), `discussion_messages` (FK CASCADE, `thought_kind`, `kind` — derived from the ban flag on pre-1.17 rows). Schema uses idempotent `CREATE TABLE IF NOT EXISTS` + `ALTER TABLE` migrations with column existence checks. Profile seeding runs on every startup with `ON CONFLICT DO NOTHING`.

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
