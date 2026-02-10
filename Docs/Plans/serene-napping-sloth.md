# AIrena — Modes de distribution "Démocratique" et "Autoritaire"

## Contexte

Actuellement, `TurnDistribution` a 2 modes purement algorithmiques (Sequential, Random). L'utilisateur veut 2 nouveaux modes basés sur des appels LLM :
- **Démocratique** : les GladIAteurs votent (masqué) pour l'ordre de parole. En cas d'égalité, IArbitre départage. Chaque IA parle quand même chaque tour.
- **Autoritaire** : IArbitre décide seul de l'ordre de parole selon ses critères.
- Chaque mode affiche un petit descriptif explicatif dans l'UI.

## Fichiers à modifier

| Fichier | Changement |
|---------|-----------|
| `src-tauri/src/models/discussion.rs` | Ajouter `Democratic` et `Authoritarian` à `TurnDistribution` |
| `src-tauri/src/engine/turn_manager.rs` | Nouvelle fn async `determine_speaker_order_llm` + structs vote/order |
| `src-tauri/src/engine/prompt_builder.rs` | 4 nouvelles fonctions prompt (vote, order, tiebreak, context helper) |
| `src-tauri/src/engine/orchestrator.rs` | Dispatcher sync/async selon le mode |
| `src/lib/types.ts` | Élargir le type union `turnDistribution` |
| `src/pages/SetupPage.tsx` | Grille 2×2 avec descriptions pour les 4 modes |
| `src/i18n/locales/{fr,en,zh}.json` | 6 nouvelles clés (noms + descriptions des modes) |

## Implémentation

### 1. Rust — `TurnDistribution` enum (`src-tauri/src/models/discussion.rs:6-11`)

```rust
pub enum TurnDistribution {
    Sequential,
    Random,
    Democratic,
    Authoritarian,
}
```

### 2. Rust — `turn_manager.rs` : nouvelle fn async

**Approche** : garder `determine_speaker_order` sync (Sequential/Random inchangé). Ajouter `determine_speaker_order_llm` async pour Democratic/Authoritarian. Fallback → Sequential en cas d'échec LLM.

**Structs de désérialisation** (dans le même fichier) :
```rust
#[derive(Debug, serde::Deserialize)]
struct DemocraticVote { ranking: Vec<String> }

#[derive(Debug, serde::Deserialize)]
struct AuthoritarianOrder { order: Vec<String> }
```

**Algorithme Démocratique (Borda count)** :
- Pour chaque gladiateur actif : appel LLM (JSON, non-streaming) → classement des AUTRES participants
- Score : 1er = N-1 pts, 2e = N-2 pts, …, dernier = 1 pt
- Auto-vote filtré (un gladiateur ne peut pas se voter)
- En cas d'égalité : appel LLM à IArbitre pour départager les ex-aequo
- Résolution de noms : `eq_ignore_ascii_case` (même pattern que `validate_reactions`)

**Algorithme Autoritaire** :
- 1 appel LLM à IArbitre → liste ordonnée de tous les participants
- Noms non reconnus ignorés, participants manquants ajoutés à la fin

**Fallback** : si un appel LLM échoue, tracing::warn + fallback séquentiel.

### 3. Rust — `prompt_builder.rs` : 4 nouvelles fonctions

Toutes trilingues (FR/EN/ZH), avec `format: "json"` et instruction "Réponds UNIQUEMENT avec le JSON".

- `build_democratic_vote_prompt(voter_name, other_names, topic, lang, messages_history, current_turn)` → demande au votant de classer les autres du plus au moins pertinent
- `build_authoritarian_order_prompt(participant_names, topic, lang, messages_history, current_turn)` → demande à IArbitre de décider l'ordre complet
- `build_tiebreaker_prompt(tied_names, topic, lang)` → demande à IArbitre de départager les ex-aequo
- `build_recent_context_for_vote(messages_history, current_turn)` → helper privé, résumé des interventions du tour précédent (réutilise `truncate()` existant L725)

### 4. Rust — `orchestrator.rs` (L207-211) : dispatcher

```rust
let order = match &self.config.arbitre.turn_distribution {
    TurnDistribution::Sequential | TurnDistribution::Random => {
        turn_manager::determine_speaker_order(&self.gladiateurs, &self.config.arbitre.turn_distribution)
    }
    TurnDistribution::Democratic | TurnDistribution::Authoritarian => {
        turn_manager::determine_speaker_order_llm(
            &self.gladiateurs, &self.config.arbitre.turn_distribution,
            &self.ollama_client, &self.arbitre, &self.config.topic,
            &self.config.discussion_language, &self.messages_history,
            self.current_turn, self.cancel_token.clone(),
        ).await
    }
};
```

Ajouter `use crate::models::discussion::TurnDistribution;` si pas déjà importé.

### 5. TypeScript — `types.ts` (L34)

```typescript
turnDistribution: "sequential" | "random" | "democratic" | "authoritarian";
```

### 6. Frontend — `SetupPage.tsx` (L393-413)

Remplacer le toggle 2 boutons par une grille 2×2 avec nom + description :

```tsx
<div className="grid grid-cols-2 gap-2">
  {(["sequential", "random", "democratic", "authoritarian"] as const).map((dist) => (
    <button key={dist} onClick={() => updateArbitre({ turnDistribution: dist })}
      className={cn("flex flex-col items-start rounded-md border px-3 py-2 text-left transition-colors", ...)}>
      <span className="text-sm font-medium">{t(`setup.${dist}`)}</span>
      <span className="mt-0.5 text-xs opacity-70">{t(`setup.${dist}Desc`)}</span>
    </button>
  ))}
</div>
```

### 7. i18n — 6 nouvelles clés par locale

| Clé | FR | EN | ZH |
|-----|----|----|-----|
| `democratic` | Démocratique | Democratic | 民主 |
| `authoritarian` | Autoritaire | Authoritarian | 独裁 |
| `sequentialDesc` | Chaque participant parle dans un ordre fixe | Each participant speaks in a fixed order | 每位参与者按固定顺序发言 |
| `randomDesc` | L'ordre de parole est aléatoire à chaque tour | Speaking order is randomized each turn | 每轮发言顺序随机 |
| `democraticDesc` | Les IA votent pour décider qui parle en premier | AIs vote to decide who speaks first | AI投票决定谁先发言 |
| `authoritarianDesc` | L'IArbitre décide seul de l'ordre de parole | The IArbitre alone decides the speaking order | 仲裁者独自决定发言顺序 |

## Coût LLM par tour

| Mode | Appels LLM supplémentaires | Notes |
|------|---------------------------|-------|
| Sequential | 0 | Algorithmique |
| Random | 0 | Algorithmique |
| Democratic | N + 0..1 | N votes + tiebreak optionnel (JSON, court) |
| Authoritarian | 1 | IArbitre seul (JSON, court) |

## Vérification

1. `cargo check` — compilation Rust OK
2. `npx vite build` — build frontend OK
3. Test : sélectionner chaque mode dans SetupPage, vérifier le descriptif affiché
4. Test discussion mode Autoritaire : vérifier que l'ordre change par rapport au séquentiel
5. Test discussion mode Démocratique : vérifier que les votes déterminent un ordre, fallback si échec

---

# AIrena - Plan d'implémentation v1 (révisé après double audit)

## Contexte

AIrena est une application Windows de type "arène de discussion IA" où des modèles LLM locaux (via Ollama) débattent entre eux sur un sujet défini par l'utilisateur. Un **IArbitre** (IA superviseur) orchestre la discussion, des **GladIAteurs** (IA participants) échangent des arguments, et l'utilisateur peut intervenir. L'objectif est de produire des échanges réalistes, vivants et pertinents aboutissant à une synthèse finale.

---

## Stack technologique

| Couche | Technologie | Justification |
|--------|------------|---------------|
| Desktop | **Tauri v2** | App légère (~10-15MB), backend Rust performant, packaging .msi/.exe standalone (aucun prérequis sauf Ollama), WebView2 intégré à Windows 10/11 |
| Frontend | **React 19 + TypeScript 5** | Écosystème le plus vaste, gestion d'état complexe, excellente maintenabilité |
| Build | **Vite 6** | Build ultra-rapide, HMR instantané |
| UI | **Tailwind CSS 4 + shadcn/ui** | Composants modernes, thème sombre/clair configurable, sobre et élégant. Utiliser `tw-animate-css` (pas `tailwindcss-animate` qui est déprécié avec Tailwind v4) |
| State | **Zustand** | Léger, API intuitive. **Persistence via SQLite** (commandes Tauri get/save), PAS localStorage |
| i18n | **react-i18next** | Support robuste FR/EN/ZH, lazy loading traductions |
| HTTP (Rust) | **reqwest** (features: `json`, `stream`) | Client HTTP async pour appels Ollama (streaming NDJSON) |
| Async (Rust) | **tokio** (features: `full`) | Runtime async pour orchestration concurrente |
| Sérialisation | **serde + serde_json** | Sérialisation JSON performante |
| BDD locale | **tokio-rusqlite** | SQLite async pour persistence. **Configurer `PRAGMA journal_mode=WAL`** pour lecture/écriture concurrentes. |
| Routing | **React Router v7** | `createBrowserRouter` avec routes déclaratives |
| IDs | **uuid** (Rust, feature `v4`) | Génération d'identifiants uniques |
| Dates | **chrono** (Rust, feature `serde`) | Gestion des timestamps |
| Streaming (Rust) | **futures-util** | Pour `StreamExt` sur les réponses NDJSON reqwest |
| Erreurs (Rust) | **thiserror** | Erreurs typées et ergonomiques (pas `anyhow` dans les modules library) |
| Cancellation (Rust) | **tokio-util** (feature `sync`) | `CancellationToken` pour le hard stop |
| Aléatoire (Rust) | **rand** | Shuffle pour TurnDistribution::Random |
| Logging (Rust) | **tracing + tracing-subscriber** | Logging structuré async-compatible |

### Cargo.toml (dépendances)

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json", "stream"] }
tokio-rusqlite = "0.6"
futures-util = "0.3"
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2"
tokio-util = { version = "0.7", features = ["sync"] }
rand = "0.8"
tracing = "0.1"
tracing-subscriber = "0.3"
```

---

## Features incluses dans la v1

- Configuration complète (utilisateur, Ollama, langue)
- Setup discussion (thème, langue de discussion, IArbitre, GladIAteurs, LLM params)
- **Pensée interne (Inner Monologue)** : phase de réflexion structurée avant chaque intervention, visible par l'utilisateur via toggle
- **Mémoire dynamique 3 niveaux** : immédiate / contextuelle / positionnelle — pour IArbitre ET GladIAteurs
- **Profils prédéfinis** de GladIAteurs (scientifique, philosophe, avocat du diable, créatif, pragmatique, etc.)
- **Système d'engagement émotionnel** : 6 axes émotionnels par GladIAteur, évolution **rule-based** (pas d'appel LLM), indicateur coloré + tooltip
- Orchestration complète avec likes/dislikes, bans, intervention utilisateur (avec timeout)
- Synthèse finale par IArbitre
- Thème sombre/clair configurable (sobre, élégant, moderne)
- Guide Ollama intégré pour les nouveaux utilisateurs
- i18n FR/EN/ZH (interface) + langue de discussion séparée
- Parsing JSON robuste avec fallback multi-couches
- Gestion d'erreurs Ollama (retry, timeout, dégradation gracieuse)

**Reporté aux versions suivantes :** sauvegarde/historique discussions, export PDF/Markdown, métriques post-discussion, modèles différents par participant.

---

## Architecture du projet

```
AIrena/
├── src-tauri/                    # Backend Rust (Tauri v2)
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/
│   │   └── default.json
│   ├── icons/
│   └── src/
│       ├── main.rs               # Entry point Tauri
│       ├── lib.rs                # Registration commandes + état global AppState
│       ├── state.rs              # AppState (std::sync::Mutex)
│       ├── error.rs              # CommandError (thiserror) pour les commandes Tauri
│       ├── commands/             # Commandes Tauri IPC (frontend → backend)
│       │   ├── mod.rs
│       │   ├── discussion.rs     # start, pause, resume, stop, force_stop, user_intervene, submit_message, skip_user_turn
│       │   ├── ollama.rs         # check_connection, list_models
│       │   └── settings.rs       # get/save settings, CRUD profils
│       ├── models/               # Structures de données
│       │   ├── mod.rs
│       │   ├── gladiateur.rs     # GladIAteurConfig, GladIAteurState
│       │   ├── iarbitre.rs       # IArbitreConfig, IArbitreState (avec mémoire propre)
│       │   ├── discussion.rs     # DiscussionConfig, DiscussionStatus, TurnDistribution
│       │   ├── message.rs        # Message, Reaction, ReactionType, SpeakerRole
│       │   ├── events.rs         # ArenaEvent (tagged enum pour Channel<T>)
│       │   ├── engine_command.rs # EngineCommand enum
│       │   ├── memory.rs         # ParticipantMemory, TurnSnapshot, ParticipantPosition
│       │   ├── emotion.rs        # EmotionalProfile
│       │   ├── moderation.rs     # ModerationResult, ModerationAction
│       │   ├── profile.rs        # PredefinedProfile (DB + seeds)
│       │   └── settings.rs       # AppSettings, LlmParams
│       ├── engine/               # Moteur d'orchestration (coeur de l'app)
│       │   ├── mod.rs
│       │   ├── orchestrator.rs   # DiscussionEngine struct + run()
│       │   ├── memory_manager.rs # Système mémoire 3 niveaux
│       │   ├── emotion_engine.rs # Système émotionnel RULE-BASED
│       │   ├── turn_manager.rs   # Gestion des tours et ordre de parole
│       │   ├── prompt_builder.rs # Construction des prompts (multilingue)
│       │   └── json_parser.rs    # Parsing JSON robuste multi-couches
│       ├── ollama/               # Client API Ollama
│       │   ├── mod.rs
│       │   ├── client.rs         # OllamaClient (reqwest, streaming NDJSON)
│       │   ├── error.rs          # OllamaError (thiserror)
│       │   └── types.rs          # ChatRequest, ChatResponse, ModelInfo
│       └── db/                   # Persistence SQLite
│           ├── mod.rs
│           ├── schema.rs         # Migrations et création tables (WAL mode)
│           ├── repository.rs     # CRUD profils, settings
│           └── seed.rs           # Profils prédéfinis initiaux
│
├── src/                          # Frontend React
│   ├── main.tsx
│   ├── App.tsx
│   ├── components/
│   │   ├── ui/                   # Composants shadcn/ui
│   │   ├── layout/
│   │   │   ├── AppShell.tsx
│   │   │   ├── Sidebar.tsx
│   │   │   └── TopBar.tsx        # + toggle thème
│   │   ├── discussion/
│   │   │   ├── DiscussionFeed.tsx    # Flux messages + auto-scroll
│   │   │   ├── MessageBubble.tsx     # Bulle message + streaming
│   │   │   ├── ReactionBar.tsx       # Likes/dislikes
│   │   │   ├── TurnIndicator.tsx     # Tour courant + progression
│   │   │   ├── SpeakerBadge.tsx      # Nom + rôle + avatar
│   │   │   ├── BanNotification.tsx   # Notification de ban
│   │   │   ├── EmotionIndicator.tsx  # Cercle coloré + tooltip 6 axes
│   │   │   ├── ThinkingPanel.tsx     # Pensée interne (toggle)
│   │   │   ├── DiscussionControls.tsx # Intervenir, arrêter (soft/hard)
│   │   │   └── UserInputArea.tsx     # Zone de saisie utilisateur
│   │   ├── setup/
│   │   │   ├── TopicInput.tsx
│   │   │   ├── ArbitreConfig.tsx
│   │   │   ├── GladiatorConfig.tsx
│   │   │   ├── GladiatorList.tsx
│   │   │   ├── LlmParamsForm.tsx
│   │   │   └── ProfileSelector.tsx
│   │   ├── settings/
│   │   │   ├── OllamaSettings.tsx    # + guide intégré
│   │   │   └── GeneralSettings.tsx
│   │   └── common/
│   │       └── ErrorBoundary.tsx     # Error boundary React global
│   ├── stores/
│   │   ├── useArenaStore.ts      # État discussion (messages, tours, streaming)
│   │   ├── useSettingsStore.ts   # Paramètres app (hydraté depuis SQLite via Tauri commands)
│   │   └── useSetupStore.ts      # Config nouvelle discussion
│   ├── hooks/
│   │   ├── useOllama.ts
│   │   ├── useDiscussion.ts
│   │   ├── useArenaChannel.ts    # Hook Channel Tauri
│   │   ├── useTokenBuffer.ts     # Buffer tokens à 50ms pour throttle React renders
│   │   └── useAutoScroll.ts
│   ├── i18n/
│   │   ├── config.ts
│   │   └── locales/
│   │       ├── fr.json
│   │       ├── en.json
│   │       └── zh.json
│   ├── lib/
│   │   ├── tauri-api.ts          # Wrappers typés invoke() + Channel
│   │   ├── types.ts              # Types TS (miroir exact des structs Rust)
│   │   └── utils.ts
│   ├── pages/
│   │   ├── HomePage.tsx
│   │   ├── SetupPage.tsx
│   │   ├── ArenaPage.tsx
│   │   ├── SummaryPage.tsx
│   │   └── SettingsPage.tsx
│   ├── providers/
│   │   └── ThemeProvider.tsx      # Thème sombre/clair via classe CSS sur <html>
│   └── styles/
│       └── globals.css
│
├── index.html
├── package.json
├── tsconfig.json
├── vite.config.ts
├── components.json               # shadcn/ui
└── .gitignore
```

---

## Modèles de données Rust (structures clés)

### Conventions appliquées

- **Tous les structs** : `#[derive(Debug, Clone, Serialize, Deserialize)]` sauf indication contraire
- **Tous les structs sérialisés** : `#[serde(rename_all = "camelCase")]`
- **Enums sérialisés** : `#[serde(rename_all = "camelCase")]` + `#[serde(rename = "...")]` si casing spécial (ex: `GladIAteur`)
- **Pas d'`anyhow`** dans les modules : `thiserror` pour les erreurs typées
- **Nommage Rust** : snake_case fonctions/variables, PascalCase types, SCREAMING_SNAKE_CASE constantes

```rust
// -- error.rs --
/// Erreurs des commandes Tauri — implémente Into<InvokeError> via Serialize
#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "message")]
pub enum CommandError {
    #[error("Ollama error: {0}")]
    Ollama(String),
    #[error("Discussion error: {0}")]
    Discussion(String),
    #[error("Settings error: {0}")]
    Settings(String),
    #[error("Discussion already running")]
    AlreadyRunning,
    #[error("No active discussion")]
    NoActiveDiscussion,
}

// Tauri v2 : les commandes retournent Result<T, CommandError>
// CommandError implémente Serialize → Tauri le sérialise automatiquement

// -- models/engine_command.rs --
/// Commandes envoyées depuis le frontend vers le moteur en cours d'exécution
#[derive(Debug)]
pub enum EngineCommand {
    Pause,
    Resume,
    Stop,                              // Soft stop : finir le tour courant
    ForceStop,                         // Hard stop : interrompre immédiatement
    UserWantsToIntervene,
    SubmitUserMessage { content: String },
    SkipUserTurn,                      // L'utilisateur annule son intervention
}

// -- models/events.rs --
/// Événements envoyés du backend vers le frontend via Channel<ArenaEvent>
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "data")]
pub enum ArenaEvent {
    /// Discussion démarrée avec succès
    DiscussionStarted { discussion_id: String },
    /// Token de message en streaming
    MessageChunk { speaker_id: String, chunk: String },
    /// Message complet (après fin du streaming)
    MessageComplete { message: Message },
    /// Réaction émise par un participant
    ReactionEmitted { message_id: String, reaction: Reaction },
    /// Token de pensée interne en streaming
    ThoughtChunk { speaker_id: String, chunk: String },
    /// Pensée interne complète
    ThoughtComplete { speaker_id: String, thought: String },
    /// Début d'un nouveau tour
    TurnStarted { turn_number: u32, speaker_order: Vec<String> },
    /// Tour sauté (tous bannis)
    TurnSkipped { reason: String, next_available_turn: u32 },
    /// Le speaker actif change
    SpeakerActive { speaker_id: String },
    /// Émotions mises à jour (rule-based, instantané)
    EmotionUpdated { speaker_id: String, emotions: EmotionalProfile },
    /// Ban émis par l'IArbitre
    BanIssued { banned_id: String, banned_name: String, reason: String, duration: u32 },
    /// Ban levé (retour d'un participant)
    BanLifted { speaker_id: String, speaker_name: String },
    /// C'est au tour de l'utilisateur
    UserTurnReady,
    /// Timeout de l'intervention utilisateur
    UserTurnTimeout,
    /// Pause confirmée
    PauseConfirmed,
    /// Reprise confirmée
    ResumeConfirmed,
    /// Token de synthèse en streaming
    SynthesisChunk { chunk: String },
    /// Synthèse finale complète
    SynthesisComplete { summary: String },
    /// Discussion terminée
    DiscussionEnded,
    /// Erreur non fatale (affichée dans le feed)
    Error { message: String },
}

// -- models/message.rs --
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpeakerRole {
    #[serde(rename = "IArbitre")]
    Arbitre,
    #[serde(rename = "GladIAteur")]
    Gladiateur,
    #[serde(rename = "user")]
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ReactionType { Like, Dislike }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub discussion_id: String,
    pub turn_number: u32,
    pub speaker_id: String,
    pub speaker_name: String,
    pub role: SpeakerRole,
    pub content: String,
    pub inner_thought: Option<String>,
    pub reactions: Vec<Reaction>,
    pub is_ban_notification: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reaction {
    pub from_speaker_id: String,
    pub from_speaker_name: String,
    pub reaction_type: ReactionType,
    pub target_message_id: String,
}

// -- models/gladiateur.rs --
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GladIAteurConfig {
    pub id: String,
    pub name: String,
    pub intervention_number: u32,      // Ordre séquentiel (config statique)
    pub system_prompt: String,
    pub llm_params: LlmParams,
}

#[derive(Debug, Clone)]
pub struct GladIAteurState {
    pub config: GladIAteurConfig,
    pub ban_remaining_turns: u32,      // 0 = actif, > 0 = banni
    pub ban_issued_this_turn: bool,    // Pour éviter off-by-one sur le décrément
    pub memory: ParticipantMemory,
    pub emotions: EmotionalProfile,
}

impl GladIAteurState {
    /// Un gladiateur est banni si son compteur de ban est > 0
    pub fn is_banned(&self) -> bool {
        self.ban_remaining_turns > 0
    }
}

// -- models/iarbitre.rs --
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IArbitreConfig {
    pub id: String,
    pub name: String,
    pub system_prompt: String,
    pub turn_distribution: TurnDistribution,
    pub llm_params: LlmParams,
}

#[derive(Debug, Clone)]
pub struct IArbitreState {
    pub config: IArbitreConfig,
    pub memory: ParticipantMemory,     // L'IArbitre a SA PROPRE mémoire
}

// -- models/discussion.rs --
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TurnDistribution { Sequential, Random }

#[derive(Debug, Clone, PartialEq)]
pub enum DiscussionStatus { Active, Paused, StopRequested, ForceStopRequested, Completed }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionConfig {
    pub topic: String,
    pub discussion_language: String,   // Langue de la discussion (indépendante de l'UI)
    pub arbitre: IArbitreConfig,
    pub gladiateurs: Vec<GladIAteurConfig>,
    pub max_turns: Option<u32>,
    pub user_name: String,             // Nom de l'utilisateur pour l'affichage
    pub user_intervention_timeout_secs: u64, // Défaut: 120 secondes
}

// -- models/settings.rs --
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmParams {
    pub temperature: f32,        // défaut: 0.7
    pub top_p: f32,              // défaut: 0.9
    pub top_k: u32,              // défaut: 40
    pub num_predict: i32,        // défaut: 512
    pub num_ctx: u32,            // défaut: 8192 — taille context window
    pub repeat_penalty: f32,     // défaut: 1.1
}

impl Default for LlmParams {
    fn default() -> Self {
        Self {
            temperature: 0.7, top_p: 0.9, top_k: 40,
            num_predict: 512, num_ctx: 8192, repeat_penalty: 1.1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub username: String,
    pub language: String,        // "fr", "en", "zh" — langue UI
    pub theme: String,           // "dark", "light"
    pub ollama_url: String,      // défaut: "http://localhost:11434"
    pub ollama_model: String,    // modèle unique pour toutes les IA (v1)
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            username: String::new(),
            language: "fr".to_string(),
            theme: "dark".to_string(),
            ollama_url: "http://localhost:11434".to_string(),
            ollama_model: String::new(),
        }
    }
}

// -- models/emotion.rs --
/// Profil émotionnel à 6 axes (0-100 chacun)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmotionalProfile {
    pub engagement: u8,
    pub accord: u8,
    pub confiance: u8,
    pub frustration: u8,
    pub curiosite: u8,
    pub enthousiasme: u8,
}

impl Default for EmotionalProfile {
    fn default() -> Self {
        Self { engagement: 50, accord: 50, confiance: 50, frustration: 10, curiosite: 50, enthousiasme: 50 }
    }
}

// -- models/memory.rs --
#[derive(Debug, Clone)]
pub struct ParticipantMemory {
    /// Tours complets récents (2-3 derniers)
    pub immediate: Vec<TurnSnapshot>,
    /// Résumé cumulatif des tours anciens
    pub contextual_summary: String,
    /// Positions de chaque participant
    pub positional_map: HashMap<String, ParticipantPosition>,
}

impl Default for ParticipantMemory {
    fn default() -> Self {
        Self {
            immediate: Vec::new(),
            contextual_summary: String::new(),
            positional_map: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TurnSnapshot {
    pub turn_number: u32,
    pub messages: Vec<MessageSummary>,
}

#[derive(Debug, Clone)]
pub struct MessageSummary {
    pub speaker_name: String,
    pub content: String,  // Tronqué à ~200 tokens
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticipantPosition {
    pub participant_name: String,
    pub stance: String,
}

// -- models/moderation.rs --
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModerationResult {
    pub action: ModerationAction,
    pub comment: String,
    pub ban_reason: String,
    pub ban_duration: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ModerationAction { None, Comment, Ban }

impl Default for ModerationResult {
    fn default() -> Self {
        Self {
            action: ModerationAction::None,
            comment: String::new(),
            ban_reason: String::new(),
            ban_duration: 0,
        }
    }
}

/// Structure brute reçue du LLM pour les réactions (avant validation)
#[derive(Debug, Deserialize)]
pub struct RawReaction {
    pub speaker: String,
    pub reaction: String,
}

// -- models/profile.rs --
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PredefinedProfile {
    pub id: String,
    pub name: String,             // Ex: "Le Scientifique"
    pub personality: String,      // Ex: "Rigoureux, factuel"
    pub system_prompt: String,    // Prompt complet
    pub is_builtin: bool,        // true = non modifiable, false = custom
}
```

---

## DiscussionEngine (struct + champs)

```rust
// -- engine/orchestrator.rs --

pub struct DiscussionEngine {
    // Config
    discussion_id: String,
    config: DiscussionConfig,
    ollama_client: OllamaClient,

    // État
    status: DiscussionStatus,
    current_turn: u32,
    arbitre: IArbitreState,
    gladiateurs: Vec<GladIAteurState>,
    messages_history: Vec<Message>,        // Historique complet du tour en cours
    user_intervention_pending: bool,
    user_intervention_handled: bool,

    // Communication
    cancel_token: CancellationToken,       // Pour le hard stop — passé par valeur
}

impl DiscussionEngine {
    pub fn new(
        config: DiscussionConfig,
        discussion_id: String,
        ollama_url: &str,
        ollama_model: &str,
    ) -> Self {
        let ollama_client = OllamaClient::new(ollama_url, ollama_model);
        let arbitre = IArbitreState {
            config: config.arbitre.clone(),
            memory: ParticipantMemory::default(),
        };
        let gladiateurs = config.gladiateurs.iter().map(|g| GladIAteurState {
            config: g.clone(),
            ban_remaining_turns: 0,
            ban_issued_this_turn: false,
            memory: ParticipantMemory::default(),
            emotions: EmotionalProfile::default(),
        }).collect();

        Self {
            discussion_id,
            config,
            ollama_client,
            status: DiscussionStatus::Active,
            current_turn: 0,
            arbitre,
            gladiateurs,
            messages_history: Vec::new(),
            user_intervention_pending: false,
            user_intervention_handled: false,
            cancel_token: CancellationToken::new(),
        }
    }

    /// Boucle principale — consomme self, communique via Channel + mpsc
    pub async fn run(
        mut self,
        mut cmd_rx: mpsc::Receiver<EngineCommand>,
        channel: Channel<ArenaEvent>,
    ) {
        // Voir algorithme d'orchestration ci-dessous
    }
}
```

---

## Architecture IPC et concurrence (Tauri v2)

### Pourquoi Channel et non app.emit()

`app.emit()` est conçu pour des notifications d'état ponctuelles, **pas pour du streaming haute fréquence**. Les Tauri Channels sont spécifiquement conçus pour le streaming ordonné et performant.

### Pattern : Engine spawné + mpsc pour le contrôle

```rust
// -- state.rs --
/// std::sync::Mutex (PAS tokio::sync::Mutex) car jamais tenu across .await
pub struct AppState {
    pub engine_cmd_tx: std::sync::Mutex<Option<mpsc::Sender<EngineCommand>>>,
    pub cancel_token: std::sync::Mutex<Option<CancellationToken>>,
    pub db: tokio_rusqlite::Connection,
}

// -- commands/discussion.rs --
#[tauri::command]
async fn start_discussion(
    config: DiscussionConfig,
    on_event: Channel<ArenaEvent>,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    // IMPORTANT: extraire du State AVANT tout .await (lifetime Tauri)
    let already_running;
    let db_clone;
    {
        let guard = state.engine_cmd_tx.lock().unwrap();
        already_running = guard.is_some();
        db_clone = state.db.clone();  // si besoin
    }

    if already_running {
        return Err(CommandError::AlreadyRunning);
    }

    // 1. Lire les settings pour ollama_url et ollama_model
    let settings = db::repository::get_settings(&db_clone).await
        .map_err(|e| CommandError::Settings(e.to_string()))?;

    // 2. Valider que le modèle Ollama existe
    let client = OllamaClient::new(&settings.ollama_url, &settings.ollama_model);
    client.validate_model().await
        .map_err(|e| CommandError::Ollama(e.to_string()))?;

    // 3. Créer le canal de commandes et le CancellationToken
    let (cmd_tx, cmd_rx) = mpsc::channel::<EngineCommand>(32);
    let cancel_token = CancellationToken::new();
    let engine_cancel = cancel_token.clone();

    {
        let mut guard = state.engine_cmd_tx.lock().unwrap();
        *guard = Some(cmd_tx);
        let mut cancel_guard = state.cancel_token.lock().unwrap();
        *cancel_guard = Some(cancel_token);
    }

    // 4. Spawner le moteur sur le runtime Tauri (NON-BLOQUANT)
    let discussion_id = uuid::Uuid::new_v4().to_string();
    let id_clone = discussion_id.clone();
    tauri::async_runtime::spawn(async move {
        let mut engine = DiscussionEngine::new(
            config, id_clone, &settings.ollama_url, &settings.ollama_model,
        );
        engine.cancel_token = engine_cancel;
        engine.run(cmd_rx, on_event).await;
    });

    Ok(discussion_id)
}

#[tauri::command]
async fn pause_discussion(state: State<'_, AppState>) -> Result<(), CommandError> {
    let tx = {
        let guard = state.engine_cmd_tx.lock().unwrap();
        guard.clone()
    };
    match tx {
        Some(tx) => tx.send(EngineCommand::Pause).await
            .map_err(|_| CommandError::NoActiveDiscussion),
        None => Err(CommandError::NoActiveDiscussion),
    }
}

#[tauri::command]
async fn resume_discussion(state: State<'_, AppState>) -> Result<(), CommandError> {
    // Même pattern que pause
    let tx = { state.engine_cmd_tx.lock().unwrap().clone() };
    match tx {
        Some(tx) => tx.send(EngineCommand::Resume).await
            .map_err(|_| CommandError::NoActiveDiscussion),
        None => Err(CommandError::NoActiveDiscussion),
    }
}

#[tauri::command]
async fn stop_discussion(state: State<'_, AppState>) -> Result<(), CommandError> {
    let tx = { state.engine_cmd_tx.lock().unwrap().clone() };
    match tx {
        Some(tx) => tx.send(EngineCommand::Stop).await
            .map_err(|_| CommandError::NoActiveDiscussion),
        None => Err(CommandError::NoActiveDiscussion),
    }
}

#[tauri::command]
async fn force_stop_discussion(state: State<'_, AppState>) -> Result<(), CommandError> {
    // 1. Envoyer ForceStop via mpsc
    let tx = { state.engine_cmd_tx.lock().unwrap().clone() };
    if let Some(tx) = tx {
        let _ = tx.send(EngineCommand::ForceStop).await;
    }
    // 2. Annuler via CancellationToken (coupe le streaming en cours)
    let cancel = { state.cancel_token.lock().unwrap().take() };
    if let Some(token) = cancel {
        token.cancel();
    }
    Ok(())
}

#[tauri::command]
async fn user_wants_to_intervene(state: State<'_, AppState>) -> Result<(), CommandError> {
    let tx = { state.engine_cmd_tx.lock().unwrap().clone() };
    match tx {
        Some(tx) => tx.send(EngineCommand::UserWantsToIntervene).await
            .map_err(|_| CommandError::NoActiveDiscussion),
        None => Err(CommandError::NoActiveDiscussion),
    }
}

#[tauri::command]
async fn submit_user_message(content: String, state: State<'_, AppState>) -> Result<(), CommandError> {
    let tx = { state.engine_cmd_tx.lock().unwrap().clone() };
    match tx {
        Some(tx) => tx.send(EngineCommand::SubmitUserMessage { content }).await
            .map_err(|_| CommandError::NoActiveDiscussion),
        None => Err(CommandError::NoActiveDiscussion),
    }
}

#[tauri::command]
async fn skip_user_turn(state: State<'_, AppState>) -> Result<(), CommandError> {
    let tx = { state.engine_cmd_tx.lock().unwrap().clone() };
    match tx {
        Some(tx) => tx.send(EngineCommand::SkipUserTurn).await
            .map_err(|_| CommandError::NoActiveDiscussion),
        None => Err(CommandError::NoActiveDiscussion),
    }
}
```

### Frontend : Channel + buffer tokens

```typescript
// -- lib/tauri-api.ts --
import { Channel, invoke } from "@tauri-apps/api/core";
import type { ArenaEvent, DiscussionConfig } from "./types";

export async function startDiscussion(
  config: DiscussionConfig,
  onEvent: (event: ArenaEvent) => void
): Promise<string> {
  const channel = new Channel<ArenaEvent>();
  channel.onmessage = onEvent;
  return await invoke<string>("start_discussion", { config, onEvent: channel });
}

export async function pauseDiscussion(): Promise<void> {
  return await invoke("pause_discussion");
}

export async function resumeDiscussion(): Promise<void> {
  return await invoke("resume_discussion");
}

export async function stopDiscussion(): Promise<void> {
  return await invoke("stop_discussion");
}

export async function forceStopDiscussion(): Promise<void> {
  return await invoke("force_stop_discussion");
}

export async function userWantsToIntervene(): Promise<void> {
  return await invoke("user_wants_to_intervene");
}

export async function submitUserMessage(content: string): Promise<void> {
  return await invoke("submit_user_message", { content });
}

export async function skipUserTurn(): Promise<void> {
  return await invoke("skip_user_turn");
}

// -- hooks/useTokenBuffer.ts --
// Buffer tokens à 50ms pour éviter de re-render React à chaque token
function useTokenBuffer(intervalMs = 50) {
  const bufferRef = useRef<Map<string, string[]>>(new Map());
  const [flushedContent, setFlushedContent] = useState<Map<string, string>>(new Map());

  const pushToken = useCallback((speakerId: string, token: string) => {
    const buffer = bufferRef.current;
    if (!buffer.has(speakerId)) buffer.set(speakerId, []);
    buffer.get(speakerId)!.push(token);
  }, []);

  useEffect(() => {
    const interval = setInterval(() => {
      const buffer = bufferRef.current;
      if (buffer.size === 0) return;

      setFlushedContent(prev => {
        const next = new Map(prev);
        buffer.forEach((tokens, speakerId) => {
          next.set(speakerId, (next.get(speakerId) ?? "") + tokens.join(""));
        });
        buffer.clear();
        return next;
      });
    }, intervalMs);
    return () => clearInterval(interval);
  }, [intervalMs]);

  const clearSpeaker = useCallback((speakerId: string) => {
    bufferRef.current.delete(speakerId);
    setFlushedContent(prev => {
      const next = new Map(prev);
      next.delete(speakerId);
      return next;
    });
  }, []);

  return { flushedContent, pushToken, clearSpeaker };
}
```

---

## Algorithme d'orchestration RÉVISÉ

```
INITIALISATION:
  1. Valider la config (≥ 1 gladiateur, modèle accessible, etc.)
  2. Initialiser mémoire pour IArbitre ET tous les GladIAteurs
  3. turn = 0, status = Active
  4. Émettre DiscussionStarted { discussion_id }

DÉMARRAGE:
  5. IArbitre génère message d'introduction (thème + participants)
  6. Émettre MessageChunk/MessageComplete via Channel

BOUCLE PRINCIPALE:
  while not should_stop(turn, max_turns, status):
    turn += 1

    // Vérifier commandes entre chaque phase (pause/stop/intervene)
    // via cmd_rx.try_recv() — non-bloquant
    // SI Pause : émettre PauseConfirmed, boucler sur cmd_rx.recv() jusqu'à Resume
    // SI Resume : émettre ResumeConfirmed, reprendre

    A. DÉBUT DU TOUR
       - Émettre TurnStarted { turn_number, speaker_order }
       - SI user_intervention_pending ET non traité au tour précédent :
         → IArbitre annonce : "{username}, vous avez la parole"
         → Émettre UserTurnReady
         → Attendre submit_user_message AVEC TIMEOUT (configurable)
         → SI timeout : émettre UserTurnTimeout, annuler l'intervention
         → SI SkipUserTurn reçu : annuler l'intervention
         → SI message reçu : intégrer dans l'historique
         → IArbitre peut brièvement commenter (pas de ban sur l'utilisateur)
         → Marquer intervention comme traitée

    B. DÉTERMINER L'ORDRE DE PAROLE
       - active_gladiateurs = exclure ceux où is_banned() == true
       - SI active_gladiateurs est VIDE :
         → Émettre TurnSkipped { reason, next_available_turn }
         → NE PAS compter ce tour vers max_turns (décrémenter turn)
         → Aller directement à E (décrémenter bans)
       - Séquentiel : trier par intervention_number
       - Aléatoire : shuffle via rand::thread_rng()

    C. POUR CHAQUE GLADIATEUR ACTIF (dans l'ordre) :
       // Vérifier commandes (pause/stop) entre chaque gladiateur

       C.1 Émettre SpeakerActive { speaker_id }

       C.2 PHASE RÉACTIONS (tour > 1 uniquement) :
           - Construire prompt avec interventions du tour PRÉCÉDENT
             (EXCLURE les propres interventions du gladiateur courant)
           - Appel LLM (format: "json" activé dans Ollama)
           - Parser via json_parser::parse_reactions() avec fallback → Vec vide
           - Normaliser les valeurs (like/agree→Like, dislike/disagree→Dislike)
           - Valider noms des speakers (case-insensitive contains match)
           - Émettre ReactionEmitted pour chaque réaction valide

       C.3 PHASE PENSÉE INTERNE (inner monologue) :
           - Prompt structuré : 3 questions + état émotionnel + personnalité
           - Appel LLM → streaming ThoughtChunk/ThoughtComplete
           - SI échec : pas de pensée interne, continuer (log warning)

       C.4 PHASE INTERVENTION PUBLIQUE :
           - Construire prompt via PromptBuilder :
             * System prompt du GladIAteur
             * Instruction langue : "Réponds en {discussion_language}"
             * Mémoire contextuelle (résumé tours anciens)
             * Mémoire positionnelle (positions de chaque participant)
             * Mémoire immédiate (2-3 derniers tours complets)
             * Tour en cours (messages déjà émis)
             * Pensée interne comme contexte
             * État émotionnel (description textuelle)
             * Nom de l'utilisateur pour l'adresser si pertinent
           - Appel LLM → streaming MessageChunk/MessageComplete
           - SI échec après retry : émettre message IArbitre
             "[{name} semble avoir des difficultés]", passer au suivant
           - SI réponse vide : retry avec température +0.2, sinon placeholder

       C.5 MISE À JOUR ÉMOTIONS (RULE-BASED, pas d'appel LLM) :
           - Compter : likes/dislikes reçus ce tour
           - Contradiction : le gladiateur a reçu ≥ 2 dislikes (heuristique simple)
           - Soutien : le gladiateur a reçu ≥ 2 likes
           - Stagnation : mêmes positions depuis 3+ tours (comparer positional_map)
           - Mettre à jour les 6 axes via règles déterministes
           - Clamper les variations à ±15 par tour
           - Émettre EmotionUpdated { speaker_id, emotions }

       C.6 MODÉRATION IArbitre :
           - Prompt : intervention du gladiateur + contexte discussion
           - Appel LLM (format: "json")
           - Parser via json_parser::parse_moderation() avec fallback → "none"
           - GARDE-FOU : si ban ET un seul gladiateur actif restant → downgrade en "comment"
           - Si ban :
             * Mettre ban_remaining_turns, ban_issued_this_turn = true
             * Émettre BanIssued
             * Ajouter message IArbitre dans l'historique du tour
           - Si comment : ajouter message IArbitre dans l'historique du tour
           - Si none : passer au suivant

       C.7 INTERVENTION UTILISATEUR OPPORTUNISTE (optionnel) :
           - SI user_intervention_pending ET non traitée :
             → Heuristique simple : si ≥ moitié des gladiateurs ont parlé
             → OU si on est au dernier gladiateur
             → Alors : traiter l'intervention (même flux qu'en A)

    D. INTERVENTION UTILISATEUR OBLIGATOIRE
       - SI user_intervention_pending ET toujours non traitée :
         → OBLIGATOIRE avant fin du tour (même flux qu'en A avec timeout)

    E. FIN DU TOUR
       - Mise à jour mémoire (1 SEUL appel LLM combiné) :
         * Résumé contextuel + extraction positions en une seule réponse JSON
         * Si échec : conserver la mémoire précédente inchangée, log warning
         * Faire cet appel pour les GladIAteurs ET l'IArbitre
       - Décrémenter compteurs ban (SAUF si ban_issued_this_turn)
       - Émettre BanLifted pour chaque gladiateur dont le ban tombe à 0
       - Reset ban_issued_this_turn flags
       - Vérifier condition d'arrêt :
         * max_turns atteint → fin
         * StopRequested → fin
         * ForceStopRequested → fin immédiate (skip synthèse)
         * sinon → tour suivant

    F. SYNTHÈSE FINALE (quand la boucle se termine, sauf ForceStop) :
       - Émettre SynthesisChunk en streaming
       - IArbitre génère synthèse (prompt dédié + mémoire complète)
       - Émettre SynthesisComplete { summary }
       - Émettre DiscussionEnded

NETTOYAGE :
    - L'engine est droppé (self consommé par run)
    - Le mpsc Sender côté AppState est nettoyé automatiquement
      (le Receiver est droppé → le Sender retourne erreur)
```

---

## Nombre d'appels LLM optimisé (par tour complet, 5 gladiateurs)

| Phase | Appels | Parallélisable | Notes |
|-------|--------|---------------|-------|
| Réactions (4 gladiateurs, excl. self) | 4 | Oui (OLLAMA_NUM_PARALLEL) | ~2s en parallèle |
| Pensée interne (speaker courant) | 1 | Non | ~4s |
| Intervention publique (speaker courant) | 1 | Non (streaming) | ~5s |
| Modération IArbitre | 1 | Non | ~3s |
| Émotions | **0** | — | Rule-based, instantané |
| **Sous-total par sub-turn** | **7** | | ~14s |
| × 5 gladiateurs | **35** | | ~70s |
| Mémoire fin de tour (1 appel combiné) | **1** | Non | ~4s |
| **TOTAL PAR TOUR** | **36** | | **~74s** (~1min15) |

---

## Système de mémoire (MemoryManager) — pour IArbitre ET GladIAteurs

**Mise à jour fin de tour — 1 SEUL appel LLM combiné :**
```
{SI contextual_summary est vide:}
C'est le début de la discussion. Crée le premier résumé.
{SINON:}
Résumé existant : {contextual_summary}

Positions actuelles : {positional_map_json}

Échanges du tour {N} :
{messages_du_tour}

Produis un JSON avec 2 champs :
{
  "summary": "résumé cumulatif mis à jour (3-8 phrases, arguments clés, consensus, désaccords, moments pivots)",
  "positions": {"Nom1": "sa position actuelle", "Nom2": "sa position actuelle"}
}

Réponds UNIQUEMENT avec le JSON.
```

**Budget tokens par composant de prompt :**

| Composant | Max tokens | Notes |
|-----------|-----------|-------|
| System prompt | 400 | Personnalité + règles |
| Mémoire contextuelle | 300 | Résumé cumulatif |
| Mémoire positionnelle | 150 | Positions terse |
| État émotionnel | 80 | Description textuelle |
| Pensée interne | 150 | Contexte privé |
| Mémoire immédiate | 600-1500 | 2-3 tours complets, cap à 200 tokens/message |
| Tour en cours | 500 | Messages déjà émis |
| Instructions | 100 | Mission + langue |
| **TOTAL INPUT** | **~2280-3180** | |
| Réservé pour output | 500 | |
| **TOTAL** | **~2780-3680** | Rentre dans 8K, large marge pour 32K |

---

## Système émotionnel RULE-BASED (pas d'appel LLM)

```rust
// -- engine/emotion_engine.rs --

/// Contexte émotionnel pour la mise à jour rule-based
pub struct EmotionContext {
    pub likes_received: u32,
    pub dislikes_received: u32,
    pub was_recently_banned: bool,
    pub is_discussion_stagnating: bool,
}

impl EmotionContext {
    /// Heuristique : contradiction = ≥ 2 dislikes reçus
    pub fn was_contradicted(&self) -> bool {
        self.dislikes_received >= 2
    }
    /// Heuristique : soutien = ≥ 2 likes reçus
    pub fn was_supported(&self) -> bool {
        self.likes_received >= 2
    }
}

pub fn update_emotions(current: &EmotionalProfile, ctx: &EmotionContext) -> EmotionalProfile {
    let mut new = current.clone();

    // Likes → confiance ↑, engagement ↑ (calcul en u16 pour éviter overflow u8)
    if ctx.likes_received > 0 {
        let delta_conf = ((5u16 * ctx.likes_received as u16) as u8).min(15);
        let delta_eng = ((3u16 * ctx.likes_received as u16) as u8).min(10);
        new.confiance = add_clamped(new.confiance, delta_conf);
        new.engagement = add_clamped(new.engagement, delta_eng);
    }
    // Dislikes → frustration ↑, confiance ↓
    if ctx.dislikes_received > 0 {
        let delta_frust = ((5u16 * ctx.dislikes_received as u16) as u8).min(15);
        let delta_conf = ((3u16 * ctx.dislikes_received as u16) as u8).min(10);
        new.frustration = add_clamped(new.frustration, delta_frust);
        new.confiance = sub_clamped(new.confiance, delta_conf);
    }
    // Contradiction → frustration ↑, engagement ↑
    if ctx.was_contradicted() {
        new.frustration = add_clamped(new.frustration, 8);
        new.engagement = add_clamped(new.engagement, 5);
    }
    // Soutien → enthousiasme ↑, confiance ↑
    if ctx.was_supported() {
        new.enthousiasme = add_clamped(new.enthousiasme, 8);
        new.confiance = add_clamped(new.confiance, 5);
    }
    // Ban → frustration ↑↑, engagement ↓
    if ctx.was_recently_banned {
        new.frustration = add_clamped(new.frustration, 15);
        new.engagement = sub_clamped(new.engagement, 10);
    }
    // Stagnation → engagement ↓, curiosité ↓
    if ctx.is_discussion_stagnating {
        new.engagement = sub_clamped(new.engagement, 5);
        new.curiosite = sub_clamped(new.curiosite, 5);
    }
    // Decay naturel : les extrêmes reviennent vers 50
    new.frustration = decay_toward(new.frustration, 50, 2);
    new.enthousiasme = decay_toward(new.enthousiasme, 50, 1);

    new
}

/// Addition saturée à 100 (pas d'overflow)
fn add_clamped(val: u8, delta: u8) -> u8 {
    val.saturating_add(delta).min(100)
}

/// Soustraction saturée à 0
fn sub_clamped(val: u8, delta: u8) -> u8 {
    val.saturating_sub(delta)
}

/// Retour progressif vers la valeur cible
fn decay_toward(val: u8, target: u8, rate: u8) -> u8 {
    if val > target { val.saturating_sub(rate) }
    else if val < target { val.saturating_add(rate).min(100) }
    else { val }
}
```

### Affichage visuel : indicateur coloré + tooltip

- Cercle coloré à côté du nom du GladIAteur
- Couleur = émotion dominante (vert=confiant, orange=engagé, rouge=frustré, bleu=curieux, jaune=enthousiaste, gris=désengagé)
- Tooltip au survol : barres de progression miniatures des 6 axes
- CSS transition sur la couleur

---

## Parsing JSON robuste

```rust
// -- engine/json_parser.rs --

use thiserror::Error;

#[derive(Debug, Error)]
pub enum JsonParseError {
    #[error("Failed to parse JSON: {0}")]
    ParseFailed(String),
}

/// Extraction JSON multi-couches
pub fn parse_json_response<T: DeserializeOwned>(raw: &str) -> Result<T, JsonParseError> {
    // 1. Parse directe
    if let Ok(val) = serde_json::from_str::<T>(raw) { return Ok(val); }
    // 2. Extraire depuis bloc markdown ```json ... ```
    if let Some(block) = extract_markdown_json(raw) {
        if let Ok(val) = serde_json::from_str::<T>(&block) { return Ok(val); }
    }
    // 3. Trouver le premier { ... } ou [ ... ] via comptage d'accolades
    if let Some(obj) = extract_first_json_object(raw) {
        if let Ok(val) = serde_json::from_str::<T>(&obj) { return Ok(val); }
    }
    // 4. Nettoyer problèmes courants (single quotes, trailing commas)
    let cleaned = fix_common_json_issues(raw);
    if let Ok(val) = serde_json::from_str::<T>(&cleaned) { return Ok(val); }

    Err(JsonParseError::ParseFailed(raw[..raw.len().min(200)].to_string()))
}

/// Fallbacks par type de prompt
pub fn parse_moderation(raw: &str) -> ModerationResult {
    parse_json_response(raw).unwrap_or_default() // défaut: action="none"
}

pub fn parse_reactions(raw: &str, known_speakers: &[String]) -> Vec<Reaction> {
    parse_json_response::<Vec<RawReaction>>(raw)
        .map(|r| validate_reactions(r, known_speakers))
        .unwrap_or_default() // défaut: pas de réactions
}

/// Valider et normaliser les réactions brutes
fn validate_reactions(raw: Vec<RawReaction>, known_speakers: &[String]) -> Vec<Reaction> {
    raw.into_iter()
        .filter_map(|r| {
            // Match case-insensitive du nom du speaker
            let speaker = known_speakers.iter()
                .find(|s| s.to_lowercase().contains(&r.speaker.to_lowercase()))?;
            // Normaliser la réaction
            let reaction_type = match r.reaction.to_lowercase().as_str() {
                "like" | "agree" | "d'accord" => Some(ReactionType::Like),
                "dislike" | "disagree" | "pas d'accord" => Some(ReactionType::Dislike),
                _ => None, // "none" ou invalide → filtré
            }?;
            Some(/* construire Reaction */)
        })
        .collect()
}
```

---

## Client Ollama robuste

```rust
// -- ollama/error.rs --
#[derive(Debug, thiserror::Error)]
pub enum OllamaError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    #[error("Request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Connection lost after retries")]
    ConnectionLost,
    #[error("Request cancelled")]
    Cancelled,
    #[error("Empty response from model")]
    EmptyResponse,
}

impl OllamaError {
    pub fn is_connection_error(&self) -> bool {
        matches!(self, Self::ConnectionFailed(_) | Self::ConnectionLost)
    }
}

// -- ollama/client.rs --
pub struct OllamaClient {
    client: reqwest::Client,
    base_url: String,
    model: String,
}

impl OllamaClient {
    pub fn new(base_url: &str, model: &str) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .expect("Failed to build HTTP client"),
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
        }
    }

    /// Vérifier que le modèle existe AVANT de démarrer la discussion
    pub async fn validate_model(&self) -> Result<(), OllamaError> {
        let models = self.list_models().await?;
        if !models.iter().any(|m| m.name == self.model || m.name.starts_with(&self.model)) {
            return Err(OllamaError::ModelNotFound(self.model.clone()));
        }
        Ok(())
    }

    /// Chat streaming avec retry et timeout
    /// NOTE: on_token doit être Send pour traverser les .await
    pub async fn chat_streaming(
        &self,
        request: &ChatRequest,
        on_token: impl Fn(&str) + Send,
        cancel: CancellationToken,      // Par VALEUR (Clone est cheap)
    ) -> Result<String, OllamaError> {
        for attempt in 0..=2u32 {
            match self.chat_streaming_inner(request, &on_token, &cancel).await {
                Ok(content) => return Ok(content),
                Err(OllamaError::Cancelled) => return Err(OllamaError::Cancelled),
                Err(e) if e.is_connection_error() && attempt < 2 => {
                    tracing::warn!("Ollama connection error (attempt {}): {}", attempt + 1, e);
                    tokio::time::sleep(Duration::from_secs(2u64.pow(attempt))).await;
                }
                Err(e) => return Err(e),
            }
        }
        Err(OllamaError::ConnectionLost)
    }

    /// Streaming NDJSON — parsing bufferisé avec Vec<u8> (pas de réallocation String)
    async fn chat_streaming_inner(
        &self,
        request: &ChatRequest,
        on_token: &(impl Fn(&str) + Send),
        cancel: &CancellationToken,
    ) -> Result<String, OllamaError> {
        let url = format!("{}/api/chat", self.base_url);
        let response = self.client.post(&url).json(request).send().await?;

        if !response.status().is_success() {
            return Err(OllamaError::ConnectionFailed(
                format!("HTTP {}", response.status())
            ));
        }

        let mut stream = response.bytes_stream();
        let mut buf = Vec::<u8>::new();
        let mut accumulated = String::new();

        loop {
            tokio::select! {
                chunk = stream.next() => {
                    match chunk {
                        Some(Ok(bytes)) => {
                            buf.extend_from_slice(&bytes);
                            // NDJSON: traiter les lignes complètes
                            while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                                let line: Vec<u8> = buf.drain(..=pos).collect();
                                let line = String::from_utf8_lossy(&line);
                                let line = line.trim();
                                if !line.is_empty() {
                                    let resp: OllamaChatResponse = serde_json::from_str(line)?;
                                    if resp.done {
                                        return Ok(accumulated);
                                    }
                                    on_token(&resp.message.content);
                                    accumulated.push_str(&resp.message.content);
                                }
                            }
                        }
                        Some(Err(e)) => return Err(OllamaError::RequestFailed(e)),
                        None => return Ok(accumulated), // Stream terminé
                    }
                }
                _ = cancel.cancelled() => {
                    return Err(OllamaError::Cancelled);
                }
            }
        }
    }

    pub async fn check_connection(&self) -> bool {
        self.client.get(&format!("{}/api/tags", self.base_url))
            .timeout(Duration::from_secs(5))
            .send().await.is_ok()
    }

    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, OllamaError> {
        let url = format!("{}/api/tags", self.base_url);
        let resp = self.client.get(&url).send().await?;
        let body: serde_json::Value = resp.json().await?;
        let models = body["models"].as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|m| serde_json::from_value(m.clone()).ok())
            .collect();
        Ok(models)
    }
}
```

---

## Stratégie de prompting RÉVISÉE (avec few-shot + multilingue)

### Prompt modération IArbitre
```
Tu viens d'entendre l'intervention suivante de {speaker_name} :
"{intervention_text}"

Évalue cette intervention et réponds avec un JSON.

Exemples de réponses valides :
{"action":"none","comment":"","ban_reason":"","ban_duration":0}
{"action":"comment","comment":"Bon point, restons sur le sujet.","ban_reason":"","ban_duration":0}
{"action":"ban","comment":"","ban_reason":"Hors sujet répété","ban_duration":2}

Critères : pertinence au sujet "{topic}", originalité, ton constructif.
- "none" : intervention acceptable (cas le plus fréquent, ~80% du temps)
- "comment" : bref commentaire utile (1-2 phrases)
- "ban" : clairement hors sujet ou non constructif de manière répétée
- "ban_duration" : 1, 2 ou 3 (nombre de tours)

Réponds UNIQUEMENT avec le JSON, sans texte avant ou après.
```

### Prompt réactions
```
Voici les interventions des AUTRES participants au tour précédent :
{filtered_list excluant le gladiateur courant}

Pour chaque intervention, choisis ta réaction :
- "like" : d'accord ou argument pertinent
- "dislike" : en désaccord ou argument faible
- "none" : neutre

Exemple : [{"speaker":"Alice","reaction":"like"},{"speaker":"Bob","reaction":"none"}]

Réponds UNIQUEMENT avec le JSON.
```

### Prompt pensée interne (structuré)
```
[Ce texte est ta réflexion PRIVÉE, invisible des autres participants.]

Réfléchis en 2-4 phrases :
1. Quel argument du dernier tour t'a le plus marqué et pourquoi ?
2. Quel angle vas-tu prendre dans ton intervention ?
3. Y a-t-il un point faible dans ta position que tu dois anticiper ?

Ton état émotionnel : {emotion_description}
```

### Prompt mémoire combiné (résumé + positions)
```
{SI contextual_summary est vide:}
C'est le début de la discussion. Crée le premier résumé.
{SINON:}
Résumé existant : {contextual_summary}

Positions actuelles : {positional_map_json}

Échanges du tour {N} :
{messages_du_tour}

Produis un JSON avec 2 champs :
{
  "summary": "résumé cumulatif (3-8 phrases : arguments clés, consensus, désaccords, moments pivots)",
  "positions": {"Nom1": "sa position en 1 phrase", "Nom2": "sa position en 1 phrase"}
}

Réponds UNIQUEMENT avec le JSON.
```

### Multilingue
Tous les prompts système ont une version FR, EN, ZH sélectionnée selon `discussion_language`. Les prompts utilisateur (system_prompt des GladIAteurs/IArbitre) sont libres. L'instruction `"Réponds en {discussion_language}"` est injectée dans chaque prompt d'intervention.

---

## Profils prédéfinis de GladIAteurs

| Nom | Personnalité | System prompt (résumé) |
|-----|-------------|----------------------|
| Le Scientifique | Rigoureux, factuel | "Tu es un scientifique rigoureux. Tu exiges des preuves, tu cites des études, tu penses en hypothèses vérifiables. Tu ne te laisses pas convaincre par les arguments d'autorité. Tu privilégies les données et la méthode." |
| Le Philosophe | Conceptuel, nuancé | "Tu es un philosophe. Tu questionnes les présupposés, tu explores les implications éthiques et existentielles. Tu cherches les contradictions logiques et tu élèves le débat vers l'abstraction." |
| L'Avocat du Diable | Challenger | "Tu es l'avocat du diable. Tu adoptes systématiquement le contre-pied de la position dominante. Ton rôle est de tester la solidité des arguments en les attaquant de manière constructive." |
| Le Créatif | Disruptif, original | "Tu es un créatif. Tu proposes des idées inattendues, tu fais des analogies surprenantes, tu penses latéralement. Tu n'as pas peur de sortir du cadre pour apporter une perspective nouvelle." |
| Le Pragmatique | Concret, orienté action | "Tu es un pragmatique. Tu ramènes au concret, tu évalues la faisabilité, tu proposes des solutions applicables. Tu détestes les discussions théoriques qui ne mènent nulle part." |
| L'Optimiste | Positif, constructif | "Tu es un optimiste constructif. Tu vois les opportunités, tu encourages les bonnes idées, tu synthétises le positif. Tu cherches à faire avancer le débat vers des solutions." |
| Le Critique | Exigeant, analytique | "Tu es un critique exigeant. Tu identifies les failles logiques, tu pousses à la rigueur, tu ne laisses rien passer. Tu es respectueux mais intransigeant sur la qualité des arguments." |

Ces profils sont stockés en SQLite (table `predefined_profiles`) et seedés au premier lancement via `db/seed.rs`.

---

## Persistence SQLite

### Schéma initial (db/schema.rs)

```sql
-- Activé à la connexion
PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS predefined_profiles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    personality TEXT NOT NULL,
    system_prompt TEXT NOT NULL,
    is_builtin INTEGER NOT NULL DEFAULT 1
);
```

### Seed initial (db/seed.rs)

Au premier lancement, vérifier si la table `predefined_profiles` est vide et insérer les 7 profils prédéfinis avec leurs prompts complets.

### Repository (db/repository.rs)

```rust
pub async fn get_settings(db: &Connection) -> Result<AppSettings>;
pub async fn save_settings(db: &Connection, settings: &AppSettings) -> Result<()>;
pub async fn list_profiles(db: &Connection) -> Result<Vec<PredefinedProfile>>;
pub async fn get_profile(db: &Connection, id: &str) -> Result<Option<PredefinedProfile>>;
pub async fn save_profile(db: &Connection, profile: &PredefinedProfile) -> Result<()>;
pub async fn delete_profile(db: &Connection, id: &str) -> Result<()>;
```

---

## Gestion des cas limites

| Cas | Solution |
|-----|---------|
| 1 seul GladIAteur | Mode monologue : skip réactions, IArbitre devient interlocuteur actif |
| max_turns = 1 | Warning UI "minimum 3 tours recommandé", synthèse adaptée |
| Réponse Ollama vide | Retry température +0.2, sinon placeholder "[pas de réponse]" |
| JSON invalide du LLM | Parsing multi-couches + fallback silencieux (pas de crash) |
| Tous les gladiateurs bannis | Skip tour (pas compté), décrémenter bans, notification UI + BanLifted |
| Ban du dernier actif | Garde-fou : downgrade en "comment" au lieu de ban |
| User intervention sans réponse | Timeout configurable → annulation automatique |
| User annule intervention | SkipUserTurn via commande |
| Ollama déconnecté mid-stream | Retry ×2 avec backoff, puis pause discussion avec Error event |
| User clique "stop" mid-streaming | Soft stop = fin du tour, Hard stop = abort via CancellationToken |
| Modèle supprimé entre config et start | Validation au démarrage → CommandError::Ollama |
| Discussion déjà en cours | CommandError::AlreadyRunning |
| Commande sans discussion active | CommandError::NoActiveDiscussion |
| Mémoire LLM échoue | Conserver mémoire précédente, log warning via tracing |

---

## Plan d'implémentation par phases

### Phase 1 : Scaffolding et infrastructure
1. Initialiser projet Tauri v2 + React 19 + Vite 6 + TypeScript 5
2. Configurer Tailwind CSS 4 + shadcn/ui (thème sombre/clair, `tw-animate-css`)
3. Configurer react-i18next avec fichiers FR/EN/ZH
4. Structurer les modules Rust (models, commands, engine, ollama, db)
5. Définir tous les modèles de données Rust (structs, enums, derives, serde attrs)
6. Définir error.rs (CommandError avec thiserror)
7. Définir AppState (std::sync::Mutex) + EngineCommand + ArenaEvent
8. Configurer SQLite (WAL mode) avec schéma initial + seed profils prédéfinis
9. Implémenter OllamaClient robuste (connexion, models, chat streaming NDJSON, retry, timeout, CancellationToken par valeur, on_token Send)
10. Implémenter json_parser.rs (parsing multi-couches + fallbacks + thiserror)
11. `.gitignore` + Cargo.toml complet (toutes les dépendances)

### Phase 2 : Settings et navigation
12. ThemeProvider (classe CSS sur html, persist via SQLite)
13. ErrorBoundary React global
14. Layout AppShell (sidebar + contenu + topbar + toggle thème)
15. Page Settings : nom utilisateur, langue, thème, config Ollama
16. Guide Ollama intégré (détection auto + instructions installation)
17. Store Zustand settings (hydraté depuis SQLite via commandes Tauri, PAS localStorage)
18. Commandes Tauri settings (get/save) + CRUD profils prédéfinis

### Phase 3 : Configuration de discussion
19. Page Setup : formulaire en étapes (stepper)
20. Étape 1 : Sujet/thème + langue de discussion
21. Étape 2 : Config IArbitre (nom, system prompt, distribution, LLM params)
22. Étape 3 : Config GladIAteurs (ajout/suppression, profils prédéfinis, LLM params)
23. Étape 4 : Récapitulatif + max tours optionnel + timeout intervention + GO
24. Store Zustand setup + validation config
25. Formulaire LlmParams avec défauts (LlmParams::default()) + validation modèle

### Phase 4 : Moteur de discussion (coeur)
26. `orchestrator.rs` : DiscussionEngine struct + new() + run() — boucle principale async avec mpsc
27. `turn_manager.rs` : ordre de parole, exclusion bannis, garde-fou all-banned, shuffle avec rand
28. `prompt_builder.rs` : tous les prompts (multilingues, few-shot, structured, prompts complets FR/EN/ZH)
29. `memory_manager.rs` : mémoire 3 niveaux (IArbitre + GladIAteurs), appel combiné
30. `emotion_engine.rs` : émotions rule-based (EmotionContext, 6 axes, decay, saturating_add/sub)
31. Commande `start_discussion` avec Channel<ArenaEvent> + spawn engine + garde AlreadyRunning
32. Commandes pause/resume/stop/force_stop/user_intervene/submit_message/skip_user_turn via mpsc
33. Système réactions (parse JSON robuste, validation noms case-insensitive, normalisation)
34. Système ban (garde-fou dernier actif, ban_issued_this_turn flag, BanLifted event)
35. Intervention utilisateur (timeout, SkipUserTurn, opportuniste + obligatoire)
36. Synthèse finale IArbitre (streaming)
37. Gestion erreurs Ollama (retry, timeout, CancellationToken, dégradation gracieuse, tracing::warn)

### Phase 5 : Interface de discussion
38. ArenaPage + DiscussionFeed
39. useArenaChannel hook (Channel Tauri) + useTokenBuffer (throttle 50ms, clearSpeaker)
40. MessageBubble (streaming token par token, placeholder si vide)
41. ReactionBar (emojis + nom de l'IA)
42. SpeakerBadge (couleur unique, rôle)
43. EmotionIndicator (cercle coloré + tooltip 6 axes)
44. TurnIndicator (numéro, progression, notification turn skipped)
45. BanNotification (alerte dans le feed + BanLifted)
46. ThinkingPanel (pensée interne, toggle)
47. DiscussionControls (intervenir, skip, soft stop, hard stop avec confirmation)
48. UserInputArea (apparaît sur UserTurnReady, timer visible, bouton skip)

### Phase 6 : Synthèse et finalisation
49. SummaryPage (synthèse streamée, structurée)
50. HomePage sobre

### Phase 7 : Polish
51. Animations et transitions (messages, tours, émotions, thème)
52. Responsive et adaptation fenêtre
53. Gestion d'erreurs UX (Ollama déconnecté, modèle indisponible, etc.)
54. Build et packaging Windows (.msi)

---

## Fichiers critiques (ordre de priorité)

1. **`src-tauri/src/ollama/client.rs`** — Client HTTP robuste avec streaming NDJSON, retry, timeout, CancellationToken par valeur
2. **`src-tauri/src/engine/orchestrator.rs`** — DiscussionEngine struct + boucle principale async, spawn, mpsc, gestion de tous les cas limites
3. **`src-tauri/src/engine/json_parser.rs`** — Parsing JSON multi-couches, fallbacks, thiserror
4. **`src-tauri/src/engine/prompt_builder.rs`** — Tous les prompts multilingues FR/EN/ZH, few-shot
5. **`src-tauri/src/engine/memory_manager.rs`** — Mémoire 3 niveaux, appel combiné
6. **`src-tauri/src/engine/emotion_engine.rs`** — Émotions rule-based, EmotionContext, saturating arithmetic
7. **`src-tauri/src/error.rs`** — CommandError typé (thiserror + Serialize)
8. **`src-tauri/src/state.rs`** + **`commands/discussion.rs`** — Architecture IPC Channel + mpsc + gardes
9. **`src/hooks/useArenaChannel.ts`** + **`useTokenBuffer.ts`** — Réception streaming + throttle React
10. **`src/stores/useArenaStore.ts`** — État frontend discussion
11. **`src/lib/tauri-api.ts`** + **`types.ts`** — Bridge typé IPC Channel + types miroir Rust

---

## Types TypeScript (miroir Rust) — src/lib/types.ts

```typescript
// Miroir exact des enums/structs Rust sérialisés

export interface LlmParams {
  temperature: number;
  topP: number;
  topK: number;
  numPredict: number;
  numCtx: number;
  repeatPenalty: number;
}

export interface GladIAteurConfig {
  id: string;
  name: string;
  interventionNumber: number;
  systemPrompt: string;
  llmParams: LlmParams;
}

export interface IArbitreConfig {
  id: string;
  name: string;
  systemPrompt: string;
  turnDistribution: "sequential" | "random";
  llmParams: LlmParams;
}

export interface DiscussionConfig {
  topic: string;
  discussionLanguage: string;
  arbitre: IArbitreConfig;
  gladiateurs: GladIAteurConfig[];
  maxTurns: number | null;
  userName: string;
  userInterventionTimeoutSecs: number;
}

export type SpeakerRole = "IArbitre" | "GladIAteur" | "user";
export type ReactionType = "like" | "dislike";

export interface Reaction {
  fromSpeakerId: string;
  fromSpeakerName: string;
  reactionType: ReactionType;
  targetMessageId: string;
}

export interface Message {
  id: string;
  discussionId: string;
  turnNumber: number;
  speakerId: string;
  speakerName: string;
  role: SpeakerRole;
  content: string;
  innerThought: string | null;
  reactions: Reaction[];
  isBanNotification: boolean;
  timestamp: string;
}

export interface EmotionalProfile {
  engagement: number;
  accord: number;
  confiance: number;
  frustration: number;
  curiosite: number;
  enthousiasme: number;
}

export interface AppSettings {
  username: string;
  language: string;
  theme: string;
  ollamaUrl: string;
  ollamaModel: string;
}

export interface PredefinedProfile {
  id: string;
  name: string;
  personality: string;
  systemPrompt: string;
  isBuiltin: boolean;
}

// ArenaEvent — tagged union (discriminated via "type" field)
export type ArenaEvent =
  | { type: "discussionStarted"; data: { discussionId: string } }
  | { type: "messageChunk"; data: { speakerId: string; chunk: string } }
  | { type: "messageComplete"; data: { message: Message } }
  | { type: "reactionEmitted"; data: { messageId: string; reaction: Reaction } }
  | { type: "thoughtChunk"; data: { speakerId: string; chunk: string } }
  | { type: "thoughtComplete"; data: { speakerId: string; thought: string } }
  | { type: "turnStarted"; data: { turnNumber: number; speakerOrder: string[] } }
  | { type: "turnSkipped"; data: { reason: string; nextAvailableTurn: number } }
  | { type: "speakerActive"; data: { speakerId: string } }
  | { type: "emotionUpdated"; data: { speakerId: string; emotions: EmotionalProfile } }
  | { type: "banIssued"; data: { bannedId: string; bannedName: string; reason: string; duration: number } }
  | { type: "banLifted"; data: { speakerId: string; speakerName: string } }
  | { type: "userTurnReady"; data: null }
  | { type: "userTurnTimeout"; data: null }
  | { type: "pauseConfirmed"; data: null }
  | { type: "resumeConfirmed"; data: null }
  | { type: "synthesisChunk"; data: { chunk: string } }
  | { type: "synthesisComplete"; data: { summary: string } }
  | { type: "discussionEnded"; data: null }
  | { type: "error"; data: { message: string } };
```

---

## Vérification et tests

| Test | Description | Critère de succès |
|------|------------|-------------------|
| Connexion Ollama | `check_ollama_connection` | Détecte Ollama, guide si absent |
| Validation modèle | Supprimer modèle, lancer discussion | CommandError::Ollama claire |
| Discussion 2 tours | 2 GladIAteurs + IArbitre, 2 tours | Flux complet : intro → tours → synthèse |
| Discussion 1 gladiateur | Mode monologue | IArbitre interlocuteur, pas de réactions |
| Streaming | Token par token dans le feed | Fluidité, buffer 50ms, pas de blocage UI |
| JSON invalide | Forcer un modèle qui produit du mauvais JSON | Parsing multi-couches, fallback silencieux |
| Mémoire | Discussion 5+ tours | Les IA référencent des points anciens |
| Réactions | Likes/dislikes affichés | Emojis + nom, pas de self-reaction |
| Ban | Provocateur → ban | Notification, skip tours, BanLifted quand terminé |
| Ban dernier actif | Tous sauf 1 bannis, tenter ban du dernier | Downgrade en commentaire |
| Tous bannis | Tous bannis simultanément | Tour skippé (non compté), bans décrémentent |
| Intervention user | Bouton → IArbitre donne la parole | Message intégré, timeout visible |
| Skip intervention | Bouton "annuler" pendant intervention | SkipUserTurn, retour au flux normal |
| Timeout intervention | Ne pas répondre | Annulation auto, UserTurnTimeout |
| Pause/Resume | Bouton pause → resume | PauseConfirmed/ResumeConfirmed events |
| Soft stop | Bouton "mettre fin" | Tour se termine, synthèse générée |
| Hard stop | Bouton dédié avec confirmation | CancellationToken, pas de synthèse |
| Double discussion | Lancer 2 fois | CommandError::AlreadyRunning |
| Ollama déconnecté | Arrêter Ollama mid-discussion | Retry ×2, puis Error event |
| Émotions rule-based | Observer les indicateurs | Couleur change selon dynamique (instantané) |
| Thème sombre/clair | Toggle | Transition fluide, persist en SQLite |
| i18n | FR/EN/ZH | Interface traduite |
| Langue discussion | UI en FR, discussion en EN | Prompts en EN, UI en FR |
| Profils prédéfinis | Sélection + custom | Config auto-remplie, CRUD custom |
| Settings persistence | Modifier settings, relancer | Valeurs restaurées depuis SQLite |
