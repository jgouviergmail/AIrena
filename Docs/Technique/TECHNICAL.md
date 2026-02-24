# AIrena — Documentation Technique

> **Version** : 1.12.1
> **Dernière mise à jour** : 2026-02-24
> **Auteur** : jgouv
> **Identifiant** : `com.jgouv.airena`

---

## Table des matières

1. [Vue d'ensemble](#1-vue-densemble)
2. [Stack technique](#2-stack-technique)
3. [Prérequis](#3-prérequis)
4. [Installation & Développement](#4-installation--développement)
5. [Architecture générale](#5-architecture-générale)
6. [Backend Rust (src-tauri/)](#6-backend-rust-src-tauri)
   - 6.1 [Point d'entrée & initialisation](#61-point-dentrée--initialisation)
   - 6.2 [État applicatif (AppState)](#62-état-applicatif-appstate)
   - 6.3 [Commandes Tauri (IPC)](#63-commandes-tauri-ipc)
   - 6.4 [Moteur de discussion (Engine)](#64-moteur-de-discussion-engine)
   - 6.5 [Modèles de données](#65-modèles-de-données)
   - 6.6 [Événements Arena (ArenaEvent)](#66-événements-arena-arenaevent)
   - 6.7 [Base de données SQLite](#67-base-de-données-sqlite)
   - 6.8 [Client Ollama](#68-client-ollama)
   - 6.9 [Client Wikipedia](#69-client-wikipedia)
   - 6.10 [Système RAG](#610-système-rag-retrieval-augmented-generation)
   - 6.11 [Client Tavily (recherche web)](#611-client-tavily-recherche-web)
   - 6.12 [Carte des arguments](#612-carte-des-arguments)
   - 6.13 [Gestion des erreurs](#613-gestion-des-erreurs)
7. [Frontend React (src/)](#7-frontend-react-src)
   - 7.1 [Routage & Layout](#71-routage--layout)
   - 7.2 [Pages](#72-pages)
   - 7.3 [Stores Zustand](#73-stores-zustand)
   - 7.4 [Composants](#74-composants)
   - 7.5 [Hooks](#75-hooks)
   - 7.6 [Utilitaires](#76-utilitaires)
   - 7.7 [Internationalisation](#77-internationalisation)
   - 7.8 [Thème & styles](#78-thème--styles)
8. [Communication IPC](#8-communication-ipc)
9. [Patterns critiques](#9-patterns-critiques)
10. [Sécurité & permissions](#10-sécurité--permissions)
11. [Logging & observabilité](#11-logging--observabilité)
12. [Build & déploiement](#12-build--déploiement)
13. [Tests](#13-tests)
14. [Arborescence du projet](#14-arborescence-du-projet)
15. [Changelog](#15-changelog)

---

## 1. Vue d'ensemble

**AIrena** est une application de bureau Windows construite avec Tauri v2 qui orchestre des discussions entre modèles d'IA locaux (via Ollama). Des participants nommés « GladIAteurs » débattent d'un sujet sous la supervision d'un modérateur IA « IArbitre », avec streaming temps réel, système émotionnel, personnalités cognitives, intégration Wikipedia et recherche web — le tout fonctionnant 100% en local.

### Caractéristiques techniques

| Propriété | Valeur |
|---|---|
| Plateforme cible | Windows (MSI/NSIS) |
| Fenêtre par défaut | 1280×800 px (min. 900×600) |
| Base de données | SQLite (WAL mode) |
| API LLM | Ollama REST (local) |
| Recherche web | Tavily API (optionnel) |
| Recherche encyclopédique | Wikipedia API (gratuit) |
| Langues UI | Français (défaut), Anglais, Chinois |

---

## 2. Stack technique

### Backend

| Technologie | Version | Rôle |
|---|---|---|
| Rust | 1.93+ (stable) | Langage backend |
| Tauri | 2.x | Framework desktop |
| tokio | 1.x (full) | Runtime async |
| reqwest | 0.12 (json, stream) | Client HTTP + streaming NDJSON |
| tokio-rusqlite | 0.6 | Accès SQLite asynchrone |
| rusqlite | 0.32 (bundled) | SQLite embarqué |
| serde / serde_json | 1.x | Sérialisation JSON |
| futures-util | 0.3 | Utilitaires async (join_all, StreamExt) |
| thiserror | 2.x | Dérivation d'erreurs |
| tokio-util | 0.7 (rt) | CancellationToken |
| uuid | 1.x (v4) | Identifiants uniques |
| chrono | 0.4 (serde) | Horodatage |
| rand | 0.8 | Aléatoire (distribution tours, speech acts) |
| tracing | 0.1 | Logging structuré |
| tracing-subscriber | 0.3 | Filtrage et sortie logs |
| tracing-appender | 0.2.4 | Rotation quotidienne des logs |
| urlencoding | 2.x | Encodage URL (Wikipedia) |
| tauri-plugin-dialog | 2.x | Dialogues fichiers (backend bridge) |
| tauri-plugin-fs | 2.x | Accès fichiers (backend bridge) |
| lopdf | 0.x | Parsing de fichiers PDF (RAG) |
| regex | 1.x | Expressions régulières (parsing RAG) |

### Frontend

| Technologie | Version | Rôle |
|---|---|---|
| React | 19.1 | UI framework |
| TypeScript | 5.8 | Typage statique |
| Vite | 7.x | Build tool + HMR |
| Tailwind CSS | 4.x | Styles utilitaires |
| tw-animate-css | 1.x | Animations CSS |
| shadcn/ui | — | Composants UI (via clsx + tailwind-merge) |
| Zustand | 5.x | State management |
| React Router | 7.x | Routage SPA |
| i18next | 24.x | Internationalisation |
| react-i18next | 15.x | Hooks i18n pour React |
| KaTeX | 0.16 | Rendu LaTeX |
| Lucide React | 0.563 | Icônes SVG |
| @tauri-apps/api | 2.x | Pont IPC Tauri |
| @tauri-apps/plugin-dialog | 2.6 | Dialogues fichiers |
| @tauri-apps/plugin-fs | 2.4 | Accès fichiers |
| @tauri-apps/plugin-opener | 2.x | Ouverture URL externe |
| diff | 8.x | Calcul de diff texte (surlignage document collaboratif) |
| markmap-lib | 0.18 | Transformation markdown → arbre (carte des arguments) |
| markmap-view | 0.18 | Rendu SVG interactif de mindmaps |
| markmap-common | 0.18 | Types partagés markmap |

---

## 3. Prérequis

| Logiciel | Version minimale | Notes |
|---|---|---|
| Node.js | LTS (20+) | npm inclus |
| Rust | stable (1.82+) | `floor_char_boundary()` requis |
| Ollama | dernière version | Au moins un modèle installé |
| Visual Studio Build Tools | 2022+ | Pour la compilation Tauri sur Windows |

---

## 4. Installation & Développement

```bash
# Cloner le dépôt
git clone <repo-url> && cd AIrena

# Installer les dépendances Node
npm install

# Développement (Vite hot-reload + fenêtre Tauri)
npm run tauri dev

# Build de production (installateurs MSI/NSIS)
npm run tauri build

# Vérification des types TypeScript
npx tsc --noEmit

# Tests Rust
cd src-tauri && cargo test

# Lint Rust
cd src-tauri && cargo clippy

# Frontend seul (sans fenêtre Tauri)
npm run dev          # Serveur Vite sur http://localhost:1420
npm run build        # tsc + vite build
```

---

## 5. Architecture générale

```
┌─────────────────────────────────────────────────────────┐
│                    FRONTEND (WebView)                     │
│  React 19 + Zustand 5 + React Router 7 + Tailwind 4     │
│                                                           │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────────┐   │
│  │  Pages   │  │  Stores  │  │   Components          │   │
│  │ (7 pages)│  │ (3 stores│  │ (discussion, emotion, │   │
│  │          │  │  Zustand) │  │  document, layout)    │   │
│  └────┬─────┘  └────┬─────┘  └──────────┬───────────┘   │
│       │              │                    │               │
│       └──────────────┼────────────────────┘               │
│                      │                                    │
│               ┌──────┴──────┐                             │
│               │ tauri-api.ts │  IPC Commands + Channel     │
│               └──────┬──────┘                             │
└──────────────────────┼────────────────────────────────────┘
                       │  Tauri IPC (invoke + Channel<ArenaEvent>)
┌──────────────────────┼────────────────────────────────────┐
│                      │        BACKEND (Rust)               │
│               ┌──────┴──────┐                             │
│               │  Commands   │  20+ handlers IPC            │
│               └──────┬──────┘                             │
│                      │                                    │
│    ┌─────────────────┼──────────────────────┐             │
│    │           ┌─────┴─────┐                │             │
│    │           │  AppState  │                │             │
│    │           │ (Mutex)    │                │             │
│    │           └─────┬─────┘                │             │
│    │                 │                      │             │
│    │    ┌────────────┼───────────┐          │             │
│    │    │            │           │          │             │
│    │  ┌─┴──┐   ┌────┴────┐  ┌──┴───┐      │             │
│    │  │ DB │   │ Engine   │  │Ollama│      │             │
│    │  │SQLite│  │Orchestr.│  │Client│      │             │
│    │  └────┘   └────┬────┘  └──┬───┘      │             │
│    │                │          │           │             │
│    │    ┌───────────┼──────────┼─────┐     │             │
│    │    │           │          │     │     │             │
│    │  ┌─┴────┐ ┌───┴───┐ ┌───┴──┐ ┌┴───┐ │             │
│    │  │Prompt│ │Emotion│ │Memory│ │Wiki │ │             │
│    │  │Build.│ │Engine │ │Mgr   │ │Clnt │ │             │
│    │  └──────┘ └───────┘ └──────┘ └─────┘ │             │
│    │                                       │             │
│    │  ┌────────┐ ┌──────────┐ ┌─────────┐ │             │
│    │  │Direct. │ │Turn Mgr  │ │JSON     │ │             │
│    │  │Builder │ │(Seq/Rnd/ │ │Parser   │ │             │
│    │  │(Cogn.) │ │Demo/Auth)│ │(Fuzzy)  │ │             │
│    │  └────────┘ └──────────┘ └─────────┘ │             │
│    └────────────────────────────────────────┘             │
└──────────────────────────────────────────────────────────┘
```

### Flux de données principal

1. Le frontend envoie un `DiscussionConfig` via `invoke("start_discussion")`
2. Le backend crée un `DiscussionEngine` et le lance dans une tâche tokio
3. Le moteur émet des `ArenaEvent` via un `Channel<ArenaEvent>` Tauri
4. Le frontend reçoit les événements et met à jour les stores Zustand
5. Les commandes utilisateur (pause, stop, intervenir) transitent par `mpsc::Sender<EngineCommand>`

---

## 6. Backend Rust (src-tauri/)

### 6.1 Point d'entrée & initialisation

**`main.rs`** — Supprime la console Windows en release et délègue à `lib.rs::run()`.

**`lib.rs`** — Séquence d'initialisation :

1. **Logging** : tracing-subscriber (console colorée + fichier rotatif quotidien, rétention 7 jours)
2. **Base de données** : ouvre `{app_data_dir}/airena.db`, exécute les migrations de schéma
3. **Seeding** : injecte les profils prédéfinis (ON CONFLICT DO UPDATE)
4. **AppState** : crée l'état partagé (canal moteur, token annulation, connexion DB)
5. **Commandes** : enregistre 24 handlers Tauri via `generate_handler!`
6. **Plugins** : active opener, dialog, fs
7. **Boucle événementielle** : lance Tauri

**Modules** :
```
commands/ — Handlers IPC (discussion, ollama, settings, history, rag)
constants.rs — Constantes centralisées (limites mémoire, tailles de troncature, seuils, quotas RAG)
db/       — SQLite (schema, repository, seed)
engine/   — Cœur métier (orchestrator, turn_manager, prompt_builder, directive_builder,
             emotion_engine, memory_manager, json_parser, dynamics_parser, mode_prompts)
error.rs  — CommandError enum
models/   — Structures de données (12 fichiers, incluant argument_map)
ollama/   — Client HTTP streaming NDJSON
rag/      — Système RAG (parser, chunker, embedder, bm25, store)
state.rs  — AppState (Mutex + rag_store)
tavily/   — Client Tavily API
wikipedia/ — Client Wikipedia API
```

### 6.2 État applicatif (AppState)

**Fichier** : `state.rs`

```rust
pub struct AppState {
    pub engine_cmd_tx: Arc<Mutex<Option<mpsc::Sender<EngineCommand>>>>,
    pub cancel_token: Arc<Mutex<Option<CancellationToken>>>,
    pub db: tokio_rusqlite::Connection,
    pub rag_store: Arc<Mutex<Option<RagStore>>>,
}
```

| Champ | Type | Rôle |
|---|---|---|
| `engine_cmd_tx` | `Arc<Mutex<Option<mpsc::Sender>>>` | Canal de commandes vers le moteur actif |
| `cancel_token` | `Arc<Mutex<Option<CancellationToken>>>` | Arrêt forcé immédiat |
| `db` | `tokio_rusqlite::Connection` | Connexion SQLite asynchrone |
| `rag_store` | `Arc<Mutex<Option<RagStore>>>` | Store RAG avec documents et embeddings |

**Helpers** :
- `lock_or_recover()` — Récupère un Mutex empoisonné
- `send_engine_command()` — Envoie une commande au moteur
- `get_settings()` — Charge les paramètres depuis la DB
- `clear_engine_slots()` — Nettoie après fin de discussion

> **Pattern** : `std::sync::Mutex` (pas tokio) car les verrous ne sont jamais maintenus au-delà d'un `.await`.

### 6.3 Commandes Tauri (IPC)

#### Discussion (`commands/discussion.rs`)

| Commande | Signature | Description |
|---|---|---|
| `start_discussion` | `(config, Channel) → Result<String>` | Lance le moteur dans une tâche tokio ; retourne le discussion_id |
| `pause_discussion` | `() → Result<()>` | Met en pause le tour courant |
| `resume_discussion` | `() → Result<()>` | Reprend depuis la pause |
| `stop_discussion` | `() → Result<()>` | Arrêt doux : termine le tour courant |
| `force_stop_discussion` | `() → Result<()>` | Arrêt dur : annulation immédiate via token |
| `user_wants_to_intervene` | `() → Result<()>` | Signale que l'utilisateur veut parler |
| `submit_user_message` | `(content) → Result<()>` | L'utilisateur prend la parole |
| `skip_user_turn` | `() → Result<()>` | L'utilisateur renonce à intervenir |
| `adjust_emotion` | `(speaker_id, axis, value) → Result<()>` | Ajuste manuellement une émotion |

#### Ollama (`commands/ollama.rs`)

| Commande | Signature | Description |
|---|---|---|
| `check_ollama_connection` | `() → Result<bool>` | Vérifie la connectivité Ollama |
| `list_ollama_models` | `() → Result<Vec<ModelInfo>>` | Liste les modèles disponibles |
| `preload_ollama_model` | `(model) → Result<()>` | Pré-charge un modèle en mémoire |

#### Paramètres (`commands/settings.rs`)

| Commande | Signature | Description |
|---|---|---|
| `get_settings` | `() → Result<AppSettings>` | Récupère tous les paramètres |
| `save_settings` | `(settings) → Result<()>` | Persiste les paramètres |
| `list_profiles` | `() → Result<Vec<PredefinedProfile>>` | Profils GladIAteur |
| `list_arbitre_profiles` | `() → Result<Vec<PredefinedProfile>>` | Profils IArbitre |
| `get_profile` | `(id) → Result<Option<PredefinedProfile>>` | Un profil par ID |
| `save_profile` | `(profile) → Result<()>` | Crée/met à jour un profil |
| `delete_profile` | `(id) → Result<()>` | Supprime un profil custom |

#### Historique (`commands/history.rs`)

| Commande | Signature | Description |
|---|---|---|
| `save_discussion_history` | `(request) → Result<()>` | Persiste une discussion terminée |
| `list_discussion_history` | `() → Result<Vec<DiscussionSummary>>` | Liste légère (sans messages) |
| `get_discussion_history` | `(id) → Result<Option<DiscussionDetail>>` | Détail complet avec messages |
| `delete_discussion_history` | `(id) → Result<()>` | Supprime une discussion |
| `delete_all_discussion_history` | `() → Result<()>` | Purge complète de l'historique |

#### RAG (`commands/rag.rs`)

| Commande | Signature | Description |
|---|---|---|
| `import_rag_document` | `(file_path) → Result<RagDocumentInfo>` | Importe un document (PDF, TXT, MD, CSV, DOCX), le chunk et l'embed |
| `remove_rag_document` | `(doc_id) → Result<bool>` | Supprime un document du store RAG |
| `get_rag_status` | `() → Result<Vec<RagDocumentInfo>>` | Liste tous les documents importés |
| `clear_rag_store` | `() → Result<()>` | Vide complètement le store RAG |

### 6.4 Moteur de discussion (Engine)

#### Constantes LLM (constants.rs)

| Constante | Valeur | Description |
|---|---|---|
| `LLM_DEFAULT_NUM_PREDICT` | 2048 | Budget tokens par défaut par réponse |
| `SYNTHESIS_NUM_PREDICT` | 4096 | Budget tokens pour la synthèse (2× le défaut, retry avec 2× en cas de troncature) |
| `THINK_NUM_PREDICT_MULTIPLIER` | 3 | Multiplicateur num_predict pour les modèles en mode think |

#### Orchestrateur (`engine/orchestrator.rs` — ~2800 lignes)

Le cœur de l'application. Exécute la boucle de discussion complète dans une tâche tokio.

**Cycle de vie d'une discussion** :

```
1. DiscussionStarted
2. Introduction (IArbitre + recherche web/wiki optionnelle)
3. BOUCLE DE TOURS :
   ├── Décrément des bans → BanLifted
   ├── Détermination de l'ordre (Séq/Aléa/Démocratique/Autoritaire)
   ├── POUR chaque orateur non-banni :
   │   ├── SpeakerActive
   │   ├── Construction du prompt (contexte + émotions + directive)
   │   ├── Recherche web/wiki (si pool > 0)
   │   ├── Streaming LLM (+ mode think optionnel)
   │   │   ├── MessageChunk / ThoughtChunk
   │   │   └── MessageComplete / ThoughtComplete
   │   ├── Collecte des réactions (like/dislike)
   │   ├── Mise à jour émotionnelle (rule-based)
   │   │   ├── EmotionUpdated
   │   │   └── EmotionalThresholdCrossed (si seuil franchi)
   │   └── Modération (IArbitre évalue) → BanIssued ou commentaire
   ├── Intervention utilisateur (si activée)
   │   ├── UserTurnReady → attente submit/skip
   │   └── Traitement du message utilisateur
   ├── Mise à jour document (mode Co-Construction)
   │   └── Extraction <document> → LLM update → DocumentUpdated
   ├── Extraction carte des arguments (si activée, tour ≥ 2)
   │   └── LLM extraction → fusion → ArgumentMapUpdated
   └── Mise à jour mémoire (résumé + positions)
4. Synthèse (SynthesisChunk + SynthesisComplete)
5. DiscussionEnded
```

**Variantes par mode** :

| Mode | Comportement spécifique de l'orchestrateur |
|---|---|
| **UserDriven** | L'utilisateur est forcé de donner son input au début de chaque tour. Chaque GladIAteur décide via LLM s'il répond ou passe. Seuls les répondants sont inclus dans le TurnStarted. Si tous passent, le tour est sauté automatiquement. |
| **CollaborativeFiction** | L'utilisateur écrit l'ouverture au tour 1. Distribution toujours séquentielle (pas de randomisation). Sémantique de relais narratif. |
| **Socratic** | L'IArbitre pose une nouvelle question d'approfondissement à chaque tour (à partir du tour 2). |
| **Autres modes** | Boucle standard avec instructions mode-spécifiques dans les prompts. |

#### Gestionnaire de tours (`engine/turn_manager.rs`)

| Mode | Algorithme |
|---|---|
| **Sequential** | Tri par `intervention_number` |
| **Random** | Mélange Fisher-Yates |
| **Democratic** | Vote Borda masqué — chaque GladIAteur classe les autres via LLM (parallèle `join_all`), IArbitre départage les ex-æquo. Shortcut N=2 → Autoritaire |
| **Authoritarian** | IArbitre seul décide l'ordre via un appel LLM |

#### Constructeur de prompts (`engine/prompt_builder.rs`)

Construit les prompts contextuels pour chaque rôle et situation. **Toutes les instructions sont trilingues** (FR/EN/ZH).

| Fonction | Rôle |
|---|---|
| `build_introduction_prompt` | Introduction par l'IArbitre |
| `build_turn_prompt` | Tour d'un orateur (contexte, émotions, directive) |
| `build_reaction_prompt` | Collecte des réactions (like/dislike + justification) — sémantique adaptée au mode |
| `build_thought_prompt` | Réflexion interne guidée par le mode (mode think) |
| `build_emotion_assessment_prompt` | Évaluation émotionnelle par le LLM |
| `build_memory_update_prompt` | Résumé + suivi des positions |
| `build_synthesis_prompt` | Synthèse finale |
| `build_democratic_vote_prompt` | Vote de classement (mode démocratique) |
| `build_authoritarian_order_prompt` | Ordre décidé par l'IArbitre |
| `build_web_search_decision_prompt` | Décision de recherche web |
| `build_wiki_search_decision_prompt` | Décision de recherche Wikipedia |
| `build_document_update_prompt` | Mise à jour du document collaboratif |
| `build_argument_extraction_prompt` | Extraction de thèses et arguments (carte des arguments) |

#### Constructeur de directives (`engine/directive_builder.rs`)

Système de **personnalité cognitive** à 5 couches :

1. **Émotion → Comportement** : Extrait les valeurs/déclencheurs depuis `<dynamics>` XML
2. **Relations** : Score d'affinité (Allié, Rival, Tendu)
3. **Acte de parole** : Sélection aléatoire pondérée parmi 10 stratégies discursives
4. **Anti-répétition** : Injecte l'historique personnel pour éviter les redites
5. **Conscience situationnelle** : Ambiance du groupe, position dans le tour, retour de ban

**10 actes de parole** : Challenge, SteelMan, Anecdote, Question, Provocation, Concession, Redirect, Humor, Appeal, Synthesis

**Awareness du mode** : Le `SpeakerTurnContext` inclut `discussion_mode: DiscussionMode`. En mode Fiction Collaborative, les émotions influencent le style narratif (pas le comportement débat) via `build_fiction_emotion_behavior()`, et les actes de parole sont remappés en actions narratives via `SpeechAct::describe_fiction()`. En mode UserDriven, un rappel contextuel « répondre au message de l'utilisateur » est injecté ; en modes non-UserDriven, un rappel « observateur uniquement » est ajouté.

#### Moteur émotionnel (`engine/emotion_engine.rs`)

Mise à jour **rule-based** (sans LLM) sur 6 axes :

| Axe | Plage | Description |
|---|---|---|
| engagement | 0-100 | Intérêt pour la discussion |
| accord | 0-100 | Alignement avec les autres |
| confiance | 0-100 | Confiance en ses propres vues |
| frustration | 0-100 | Irritation (baseline 10) |
| curiosité | 0-100 | Ouverture intellectuelle |
| enthousiasme | 0-100 | Niveau d'énergie |

**Règles** : likes → confiance↑ ; dislikes → frustration↑ ; ban → frustration↑↑ ; stagnation → engagement↓. Seuils d'alerte à 85 (haut) et 15 (bas).

#### Parseur de dynamiques (`engine/dynamics_parser.rs`)

Extrait les champs de personnalité depuis les blocs XML `<dynamics>` des system prompts. Supporte les labels trilingues (FR/EN/ZH).

#### Prompts par mode (`engine/mode_prompts.rs` — ~940 lignes)

Module de **938 lignes** fournissant des instructions trilingues (FR/EN/ZH) par mode via 7 fonctions :

| Fonction | Rôle |
|---|---|
| `mode_descriptor(mode, lang)` | Nom lisible du mode (« débat », « brainstorming »…) |
| `mode_introduction_instructions(mode, lang)` | Instructions d'introduction pour l'IArbitre |
| `mode_intervention_preamble(mode, lang)` | Posture/ton attendu de chaque orateur |
| `mode_thought_focus(mode, lang, has_context)` | Guide de réflexion interne (mode think) |
| `mode_reaction_meanings(mode, lang)` | Sémantique des likes/dislikes selon le mode |
| `mode_key_constraint(mode, lang)` | Contrainte clé insérée en dernière ligne (biais de récence LLM) |
| `user_observer_clause(lang, user_name)` | Rappel « observateur » (modes non-UserDriven) |

**Également** : `SpeechAct::describe_fiction(lang)` — remap des 10 actes de parole en actions narratives pour la fiction collaborative (Challenge → conflit, SteelMan → profondeur personnage, etc.).

| Mode | Posture |
|---|---|
| Debate | Arguments, contre-arguments, positions |
| Ideation | Idées créatives, pas de critique |
| CoConstruction | Convergence vers un livrable commun |
| UserDriven | L'utilisateur guide chaque tour |
| Socratic | Questions, introspection |
| Tutorial | Enseignement, pédagogie |
| CritiqueReview | Critique équilibrée (forces + améliorations) |
| CollaborativeFiction | Co-création narrative en relais |

#### Gestionnaire de mémoire (`engine/memory_manager.rs`)

- **Mémoire immédiate** : 3 derniers tours (snapshots complets)
- **Résumé contextuel** : Tours plus anciens condensés (max 1500 caractères)
- **Carte positionnelle** : Suivi des positions/stances de chaque participant

#### Parseur JSON (`engine/json_parser.rs`)

Extraction robuste de JSON depuis les réponses LLM :

1. Parse direct
2. Extraction depuis bloc markdown ` ```json ... ``` `
3. Détection du premier `{ ... }` ou `[ ... ]` par comptage de braces
4. Nettoyage des erreurs JSON courantes + retry

**Matching flou de noms** (4 niveaux) : exact → sans article → préfixe ≥3 → contient ≥4.

**Validation des labels** : `is_valid_thesis_label()` (`pub(crate)`) rejette les labels purement numériques et ceux de moins de `ARGMAP_MIN_THESIS_LABEL_CHARS` (8) caractères. Utilisée dans le parsing ET dans l'orchestrateur (garde anti-création).

### 6.5 Modèles de données

#### Configuration de discussion

```rust
pub struct DiscussionConfig {
    pub topic: String,
    pub discussion_language: String,      // "fr", "en", "zh"
    pub arbitre: IArbitreConfig,
    pub gladiateurs: Vec<GladIAteurConfig>,
    pub max_turns: Option<u32>,
    pub user_name: String,
    pub user_intervention_timeout_secs: u64,
    pub web_search_pool: u32,
    pub wiki_search_pool: u32,
    pub discussion_mode: DiscussionMode,
    pub document_format: DocumentFormat,
    pub argument_map_enabled: bool,
}
```

#### Modes de discussion

```rust
pub enum DiscussionMode {
    Debate, Ideation, CoConstruction, UserDriven,
    Socratic, Tutorial, CritiqueReview, CollaborativeFiction,
}
```

#### Distribution des tours

```rust
pub enum TurnDistribution {
    Sequential, Random, Democratic, Authoritarian,
}
```

#### Format de document

```rust
pub enum DocumentFormat {
    None, Txt, Md, Csv,
}
```

#### Profil émotionnel

```rust
pub struct EmotionalProfile {
    pub engagement: u8,      // 0-100
    pub accord: u8,
    pub confiance: u8,
    pub frustration: u8,
    pub curiosite: u8,
    pub enthousiasme: u8,
}
```

#### Message

```rust
pub struct Message {
    pub id: String,
    pub discussion_id: String,
    pub turn_number: u32,
    pub speaker_id: String,
    pub speaker_name: String,
    pub role: SpeakerRole,       // "IArbitre" | "GladIAteur" | "user"
    pub content: String,
    pub inner_thought: Option<String>,
    pub reactions: Vec<Reaction>,
    pub is_ban_notification: bool,
    pub timestamp: DateTime<Utc>,
}

pub struct Reaction {
    pub from_speaker_id: String,
    pub from_speaker_name: String,
    pub reaction_type: ReactionType,      // Like | Dislike
    pub target_message_id: String,
    pub justification: Option<String>,    // Courte justification (1 phrase max)
}
```

### 6.6 Événements Arena (ArenaEvent)

L'enum `ArenaEvent` (30+ variants) est sérialisé avec `tag = "type"` et `content = "data"` pour le transit IPC.

| Catégorie | Événements |
|---|---|
| **Cycle de vie** | `DiscussionStarted`, `DiscussionEnded`, `Error` |
| **Streaming** | `MessageChunk`, `MessageComplete`, `ThoughtChunk`, `ThoughtComplete`, `SynthesisChunk`, `SynthesisComplete` |
| **Tours** | `TurnStarted`, `TurnSkipped`, `DeterminingOrder` |
| **Orateurs** | `SpeakerActive`, `UserTurnReady`, `UserTurnTimeout` |
| **Émotions** | `EmotionUpdated`, `EmotionHistoryUpdate`, `EmotionalThresholdCrossed` |
| **Réactions** | `ReactionEmitted` |
| **Recherche** | `WebSearchPerformed`, `WikiSearchPerformed` |
| **Cognition** | `DirectiveGenerated`, `DocumentUpdated`, `ArgumentMapUpdated` |
| **Modération** | `BanIssued`, `BanLifted` |
| **Contrôle** | `PauseConfirmed`, `ResumeConfirmed` |

### 6.7 Base de données SQLite

**Emplacement** : `{app_data_dir}/airena.db`

#### Schéma

```sql
-- Paramètres (clé-valeur)
settings (key TEXT PK, value TEXT NOT NULL)

-- Profils prédéfinis
predefined_profiles (
    id TEXT PK,
    name TEXT, personality TEXT, system_prompt TEXT,
    is_builtin INTEGER DEFAULT 1,
    profile_type TEXT DEFAULT 'gladiateur',
    category TEXT DEFAULT 'autres',
    initial_emotions TEXT
)

-- Discussions
discussions (
    id TEXT PK,
    topic TEXT, discussion_language TEXT DEFAULT 'fr',
    model_name TEXT, participants_json TEXT DEFAULT '[]',
    total_turns INTEGER, synthesis TEXT, created_at TEXT,
    discussion_mode TEXT DEFAULT 'debate',
    document_content TEXT DEFAULT '',
    document_format TEXT DEFAULT 'none',
    argument_map_md TEXT DEFAULT ''
)

-- Messages
discussion_messages (
    id TEXT PK,
    discussion_id TEXT FK → discussions(id) ON DELETE CASCADE,
    turn_number INTEGER, speaker_id TEXT, speaker_name TEXT,
    role TEXT, content TEXT, inner_thought TEXT,
    reactions_json TEXT DEFAULT '[]',
    is_ban_notification INTEGER DEFAULT 0,
    timestamp TEXT, sort_order INTEGER
)
-- INDEX idx_dm_discussion_id ON discussion_messages(discussion_id, sort_order)
```

#### Conventions

- **Booléens** : stockés comme chaînes `"true"` / `"false"`, parsés avec `value == "true"`
- **JSON** : stocké comme TEXT pour `participants_json`, `reactions_json`, `initial_emotions`
- **Migrations** : `ALTER TABLE ADD COLUMN` idempotent avec vérification PRAGMA
- **Seeding** : `ON CONFLICT DO UPDATE SET` pour les profils builtin (permet les mises à jour)
- **97 GladIAteurs** : experts, IT, personnalités historiques, écrivains, mode, archétypes, figures, métiers
- **10 IArbitres** : Impartial, Provocateur, Maïeuticien, Juge Strict, Animateur TV, Thérapeute, Roi Philosophe, Agent du Chaos, Directeur Scientifique, Grand-mère

### 6.8 Client Ollama

**Fichier** : `ollama/client.rs`

```rust
#[derive(Clone)]
pub struct OllamaClient {
    client: reqwest::Client,   // Arc-based, clone léger
    base_url: String,
    model: String,
}
```

| Méthode | Description |
|---|---|
| `validate_model()` | Vérifie l'existence du modèle avant la discussion |
| `chat_streaming()` | Streaming contenu uniquement (NDJSON) |
| `chat_streaming_with_think()` | Streaming contenu + pensée (mode think) |
| `chat()` | Appel non-streaming (réponses JSON structurées) |
| `list_models()` | Liste tous les modèles Ollama |
| `check_connection()` | Ping du serveur |
| `preload_model()` | Pré-chargement en mémoire |

**Caractéristiques** :
- Timeout : 120 secondes
- Retry : 3 tentatives avec backoff exponentiel (2^n sec) pour erreurs de connexion
- Annulation : `tokio::select!` avec `CancellationToken`
- Mode think : le champ `thinking` est silencieusement `None` si le modèle ne le supporte pas

### 6.9 Client Wikipedia

**Fichier** : `wikipedia/client.rs`

- **Langues** : fr→fr, en→en, zh→zh (fallback en si vide)
- **Requêtes** : API Wikipedia, 3 résultats max, extraits de 500 caractères
- **Filtrage** : `pick_best_result()` score par chevauchement de mots-clés, pénalise les pages de désambiguïsation
- **Timeout** : 15 secondes
- **User-Agent** : `AIrena/{VERSION}`

### 6.10 Système RAG (Retrieval-Augmented Generation)

**Module** : `rag/` (5 fichiers)

Le système RAG permet d'enrichir les interventions des participants avec des connaissances extraites de documents importés par l'utilisateur.

#### Architecture RAG

| Fichier | Rôle |
|---|---|
| **parser.rs** | Parse 5 formats de documents (PDF, TXT, MD, CSV, DOCX) → texte brut |
| **chunker.rs** | Découpe le texte en chunks avec overlap configurable |
| **embedder.rs** | Génère des embeddings via modèle Ollama |
| **bm25.rs** | Recherche lexicale BM25 (tf-idf avec normalisation) |
| **store.rs** | Stockage en mémoire (documents + chunks + embeddings + index BM25) |

#### Flux d'import

1. **Parsing** : extraction du texte depuis le fichier (PDF via lopdf, DOCX via regex, autres via UTF-8)
2. **Chunking** : découpage en chunks de ~800 caractères avec 200 caractères d'overlap
3. **Embedding** : génération d'embeddings pour chaque chunk via le modèle configuré
4. **Stockage** : ajout au `RagStore` avec indexation BM25

#### Recherche hybride (3 étapes)

Lors d'une intervention, le moteur peut décider de rechercher des informations pertinentes dans le store RAG :

1. **Stage 1a** : Recherche par similarité cosine → top 30 (`RAG_RETRIEVAL_TOP_K`)
2. **Stage 1b** : Recherche lexicale BM25 → top 30
3. **Stage 1c** : Fusion RRF (Reciprocal Rank Fusion, k=60) → top 10 (`RAG_RRF_TOP_K`)
4. **Stage 2** : Le LLM sélectionne les chunks les plus pertinents → max 5 (`RAG_LLM_SELECT_MAX`), fallback top 3 (`RAG_FALLBACK_TOP_K`) si échec

#### Configuration

| Paramètre | Valeur | Description |
|---|---|---|
| `RAG_MAX_FILE_SIZE_BYTES` | 10 MB | Taille maximale d'un fichier importé |
| `RAG_CHUNK_TARGET_CHARS` | 2000 | Taille cible d'un chunk (~512 tokens) |
| `RAG_CHUNK_OVERLAP_CHARS` | 200 | Overlap entre chunks consécutifs |
| `RAG_RETRIEVAL_TOP_K` | 30 | Candidats par recherche vectorielle (Stage 1a) |
| `RAG_RRF_TOP_K` | 10 | Candidats après fusion RRF (Stage 1c) |
| `RAG_LLM_SELECT_MAX` | 5 | Max chunks sélectionnés par le LLM (Stage 2) |
| `RAG_FALLBACK_TOP_K` | 3 | Fallback si le LLM échoue |
| `RAG_EMBED_BATCH_SIZE` | 32 | Textes par appel d'embedding |
| `embedding_model` | (settings) | Modèle Ollama pour les embeddings (défaut : modèle LLM principal) |

#### Formats supportés

| Format | Extension | Parsing |
|---|---|---|
| PDF | `.pdf` | lopdf (extraction texte brut) |
| Texte | `.txt` | UTF-8 direct |
| Markdown | `.md` | UTF-8 direct |
| CSV | `.csv` | Conversion en tableau formaté |
| Word | `.docx` | Regex sur XML interne (fallback basique) |

#### Événement RAG

`ragContextInjected` — émis quand des chunks RAG sont injectés dans le contexte d'un orateur :
```typescript
{
  type: "ragContextInjected",
  data: {
    speakerId: string,
    speakerName: string,
    chunks: RagChunkInfo[]  // [{ fileName, chunkIndex, preview, relevanceScore }]
  }
}
```

#### Limites et considérations

- **Mémoire** : tous les documents et embeddings sont stockés en RAM
- **Cohérence** : mélanger des modèles d'embedding différents provoque une erreur de dimension
- **Lifetime** : le store RAG persiste pour toute la durée de vie de l'application (jusqu'à clearRagStore)
- **Pas de persistance** : les documents doivent être réimportés à chaque lancement

### 6.11 Client Tavily (recherche web)

**Fichier** : `tavily/client.rs`

- **API** : `https://api.tavily.com/search` (POST)
- **Mode** : `search_depth: "basic"`, 5 résultats max
- **Quota** : 1000 crédits/mois (tier gratuit), suivi dans AppSettings
- **Erreurs** : 401 (clé invalide), 429 (rate limit), 432 (quota dépassé)

### 6.12 Carte des arguments

**Module** : `models/argument_map.rs` + intégration dans `engine/orchestrator.rs`, `engine/prompt_builder.rs`, `engine/json_parser.rs`

Le système de carte des arguments extrait automatiquement les thèses et arguments de la discussion pour produire une visualisation en mindmap.

#### Modèle de données

```rust
pub enum ArgumentType { Support, Counter, Evidence }

pub struct ThesisNode {
    pub id: String,
    pub label: String,
    pub speaker_id: String,
    pub speaker_name: String,
    pub arguments: Vec<ArgumentNode>,
}

pub struct ArgumentNode {
    pub id: String,
    pub label: String,
    pub arg_type: ArgumentType,
    pub speaker_id: String,
    pub speaker_name: String,
    pub targets_thesis_id: Option<String>,
}

pub struct ArgumentMap {
    pub theses: Vec<ThesisNode>,
}
```

#### Flux d'extraction

1. **Après chaque tour** (à partir du tour 2), l'orchestrateur appelle `build_argument_extraction_prompt()` avec le contexte des interventions récentes et la carte existante (sous forme de bullet points, pas de liste numérotée)
2. Le LLM retourne un JSON avec les nouvelles thèses/arguments
3. `parse_argument_extraction()` dans `json_parser.rs` parse la réponse avec fuzzy matching des noms + filtrage `is_valid_thesis_label()` (rejette les labels numériques et < 8 chars)
4. **Résolution numérique** : si le LLM retourne un index numérique (ex. « 3 ») au lieu d'un label, l'orchestrateur tente de le résoudre vers la thèse existante à cet index
5. **Garde anti-création** : les nouvelles thèses ne sont créées que si le label passe `is_valid_thesis_label()`
6. Les résultats sont fusionnés avec la carte existante (ajout uniquement, pas de suppression)
7. `ArgumentMap::to_markdown()` convertit en markdown hiérarchique (structure `# Topic / ## Speaker / ### Thesis / - Arg`)
8. L'événement `ArgumentMapUpdated` est émis avec le markdown, le nombre de thèses et d'arguments

#### Rendu frontend

- **MarkmapViewer** (`components/mindmap/MarkmapViewer.tsx`) : composant `forwardRef` wrappant `markmap-lib` + `markmap-view`
  - `useImperativeHandle` expose `getSvgHtml()` pour l'export SVG
  - Options markmap : `autoFit`, `maxWidth: 250`, `spacingHorizontal: 80`, `spacingVertical: 8`
  - **Export SVG complet** : `buildStandaloneSvg()` utilise `getBBox()` sur le `<g>` principal pour capturer l'intégralité du contenu (pas seulement le viewport visible), supprime le transform D3 zoom/pan, conserve les classes markmap et la CSS variable `--markmap-max-width`
  - **Off-screen rendering** : sur la page Résumé, un `MarkmapViewer` est monté hors-écran (position absolute, -9999px) avec des dimensions réelles (1200×800px) quand l'onglet actif n'est pas la carte — permet l'export SVG depuis n'importe quel onglet
- **MindmapSidebar** (`components/mindmap/MindmapSidebar.tsx`) : sidebar droite avec header, badge compteurs, légende, collapse

#### Configuration (constants.rs)

| Constante | Valeur | Description |
|---|---|---|
| `ARGMAP_CONTEXT_CHARS` | 600 | Caractères par message dans le contexte d'extraction |
| `ARGMAP_MIN_THESIS_LABEL_CHARS` | 8 | Longueur minimale pour un label de thèse valide (filtre les indices numériques) |
| `ARGMAP_MAX_THESIS_LABEL` | 200 | Caractères max pour un label de thèse |
| `ARGMAP_MAX_ARGUMENT_LABEL` | 400 | Caractères max pour un label d'argument |
| `ARGMAP_MAX_THESES` | 20 | Nombre max de thèses |
| `ARGMAP_MAX_ARGUMENTS` | 100 | Nombre total max d'arguments |
| `ARGMAP_NUM_PREDICT` | 4096 | num_predict pour l'extraction |
| `ARGMAP_NUM_CTX` | 16384 | Taille du contexte pour l'extraction |
| `ARGMAP_TEMPERATURE` | 0.3 | Température basse pour un JSON structuré |

#### Dépendances frontend

| Package | Rôle |
|---|---|
| `markmap-common` | Types partagés markmap |
| `markmap-lib` | Transformation markdown → arbre |
| `markmap-view` | Rendu SVG interactif |

### 6.13 Gestion des erreurs

```rust
#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "message")]
pub enum CommandError {
    Ollama(String),
    Settings(String),
    AlreadyRunning,
    NoActiveDiscussion,
    History(String),
    Rag(String),
}
```

**Principes** :
- Aucun `unwrap()` en production (seulement dans les tests)
- `expect()` uniquement au démarrage
- Tous les chemins de sortie du moteur émettent `DiscussionEnded`
- Les erreurs LLM non-fatales émettent `Error` sans arrêter la discussion

---

## 7. Frontend React (src/)

### 7.1 Routage & Layout

| Route | Page | Description |
|---|---|---|
| `/` | HomePage | Accueil, boutons vers nouvelle discussion ou historique |
| `/setup` | SetupPage | Assistant 4 étapes de configuration |
| `/arena` | ArenaPage | Vue discussion en direct |
| `/summary` | SummaryPage | Synthèse et statistiques post-discussion |
| `/history` | HistoryPage | Liste des discussions passées |
| `/history/:id` | HistoryDetailPage | Vue détaillée d'une discussion |
| `/settings` | SettingsPage | Configuration de l'application |

**AppShell** : Sidebar fixe (64 px) + zone principale avec `<Outlet />`. Lazy loading des pages avec `<Suspense>`.

### 7.2 Pages

| Page | Responsabilité |
|---|---|
| **HomePage** | Point d'entrée ; actions « Nouvelle discussion » et « Historique » |
| **SetupPage** | 4 étapes : sujet → IArbitre → GladIAteurs → modes/options → lancement |
| **ArenaPage** | Feed temps réel + sidebar émotions + sidebar document + sidebar mindmap + contrôles + indicateur d'activité |
| **SummaryPage** | Onglets synthèse/discussion/carte des arguments, stats, téléchargement (texte, MD, SVG) |
| **HistoryPage** | Liste avec emojis participants, timestamps, suppression |
| **HistoryDetailPage** | Détail complet avec onglets (synthèse/discussion/carte) et téléchargement |
| **SettingsPage** | Username, langue, thème, URL Ollama, modèle, Tavily API |

### 7.3 Stores Zustand

#### useArenaStore

État de la discussion en cours : messages, émotions, streaming, synthèse, bans, directives.

| Champ clé | Type | Description |
|---|---|---|
| `status` | `"idle" \| "running" \| "paused" \| "synthesizing" \| "ended"` | État de la discussion |
| `messages` | `Message[]` | Tous les messages du fil |
| `emotions` | `Map<id, EmotionalProfile>` | Profils émotionnels courants |
| `synthesis` | `string` | Texte de synthèse complété |
| `documentContent` | `string` | Document collaboratif (co-construction) |
| `argumentMapMarkdown` | `string` | Carte des arguments en markdown (pour markmap) |
| `activityStatus` | `ActivityStatus \| null` | Activité en cours du moteur (type + speakerName) |

**Méthode critique** : `handleEvent(event: ArenaEvent)` — dispatch des 30+ types d'événements.

**Buffer module-level** : Les tokens de synthèse sont accumulés dans un buffer module-level (`synthBuffer[]`) avec flush à 60ms, identique au pattern des messages.

#### useSetupStore

Configuration de la discussion avant lancement : sujet, participants, modes.

| Champ clé | Type | Description |
|---|---|---|
| `step` | `number` | Étape courante (0-3) |
| `topic` | `string` | Sujet de discussion |
| `discussionMode` | `DiscussionMode` | Mode parmi 8 |
| `gladiateurs` | `GladIAteurConfig[]` | Liste des participants |
| `arbitre` | `IArbitreConfig` | Configuration du modérateur |

**Méthode critique** : `buildConfig(userName) → DiscussionConfig` — sérialise pour le backend.

#### useSettingsStore

Paramètres globaux de l'application + profils + modèles Ollama.

| Champ clé | Type | Description |
|---|---|---|
| `settings` | `AppSettings` | Paramètres persistés |
| `profiles` | `PredefinedProfile[]` | Profils GladIAteur |
| `models` | `ModelInfo[]` | Modèles Ollama disponibles |
| `ollamaConnected` | `boolean` | État de connexion Ollama |

### 7.4 Composants

#### Discussion

| Composant | Fichier | Rôle |
|---|---|---|
| DiscussionFeed | `discussion/DiscussionFeed.tsx` | Feed de messages + bulles streaming ; gère le token buffering |
| MessageBubble | `discussion/MessageBubble.tsx` | Message unitaire : réactions, pensée, surlignage des noms, badges de recherche |
| StreamingBubble | `discussion/MessageBubble.tsx` | Message en cours avec indicateur de streaming |
| SpeakerBadge | `discussion/SpeakerBadge.tsx` | Avatar emoji + nom + chip de rôle |
| DiscussionControls | `discussion/DiscussionControls.tsx` | Pause/reprise, arrêt doux/dur, intervention |
| UserInputArea | `discussion/UserInputArea.tsx` | Textarea + countdown + boutons submit/skip |
| TurnIndicator | `discussion/TurnIndicator.tsx` | Numéro de tour, spinner d'ordre, compteur recherches |
| ReadOnlyFeed | `discussion/ReadOnlyFeed.tsx` | Rendu lecture seule pour l'historique |

#### Émotion

| Composant | Rôle |
|---|---|
| EmotionSidebar | Sidebar droite repliable ; une carte par participant |
| ParticipantEmotionCard | 6 sliders + emoji mood + ban + directive |
| EmotionAxisSlider | Slider horizontal coloré (0-100) |
| EmotionSparkline | Mini-graphique de tendance par axe |

#### Document

| Composant | Rôle |
|---|---|
| DocumentSidebar | Sidebar document collaboratif (md/txt/csv) |

#### Mindmap

| Composant | Rôle |
|---|---|
| MarkmapViewer | Rendu SVG interactif (forwardRef + markmap-lib/view) |
| MindmapSidebar | Sidebar carte des arguments (header, légende, collapse) |

#### Layout

| Composant | Rôle |
|---|---|
| AppShell | Sidebar + Outlet |
| Sidebar | Navigation 64px avec icônes |
| TopBar | En-tête + toggle thème |
| ResizeDivider | Séparateur draggable entre feed et sidebars |

#### Partagés

| Composant | Rôle |
|---|---|
| SimpleMd | Rendu markdown riche (~250 lignes) : titres (h1-h3), **gras**, *italique*, `code inline`, blocs de code avec coloration, listes (bullet + numbered), tableaux pipe-delimited, règles horizontales, intégration MathText pour LaTeX |
| MathText | Rendu LaTeX via KaTeX |
| ErrorBoundary | Attrape les erreurs React ; UI de fallback |

### 7.5 Hooks

#### useTokenBuffer(intervalMs = 60)

**Fichier** : `hooks/useTokenBuffer.ts`

Réduit les re-renders de ~200+/orateur à ~1/60ms. Accumule les tokens dans un buffer puis flush périodiquement.

```typescript
const { flushed, pushToken, clearSpeaker, clearAll } = useTokenBuffer(60);
```

### 7.6 Utilitaires

| Fichier | Exports principaux |
|---|---|
| `lib/types.ts` | Toutes les interfaces TypeScript (+ types RAG) |
| `lib/tauri-api.ts` | Wrappers IPC Tauri (28 commandes + `downloadTextFile()` via plugin dialog/fs) |
| `lib/utils.ts` | `cn()` — clsx + tailwind-merge |
| `lib/logger.ts` | Logger singleton (buffer circulaire 500 entrées) |
| `lib/profile-emoji.ts` | `getProfileEmoji(name, prompt)` — matching 3 niveaux (exact → regex → hash) |
| `lib/document-diff.ts` | `computeDocumentDiff()` — surlignage différentiel (word/line/cell) |

### 7.7 Internationalisation

**Framework** : i18next + react-i18next

| Langue | Code | Statut |
|---|---|---|
| Français | `fr` | Défaut / Fallback |
| Anglais | `en` | Complet |
| Chinois | `zh` | Complet |

**Structure des clés** : `app.*`, `nav.*`, `home.*`, `setup.*`, `arena.*`, `summary.*`, `settings.*`, `history.*`, `emotions.*`, `document.*`, `profile.*`, `languages.*`

### 7.8 Thème & styles

- **Système** : Context React (`useTheme()`) avec thème dark/light
- **Variables CSS** : format oklch (30+ variables)
- **Tailwind v4** : `@import "tailwindcss"` + `@tailwindcss/vite` (pas de fichier de config)
- **Animations** : `tw-animate-css` (remplace `tailwindcss-animate` déprécié)

---

## 8. Communication IPC

```
Frontend                          Backend
────────                          ───────
invoke("start_discussion")   ──►  start_discussion()
                                    │
Channel<ArenaEvent>          ◄──  on_event.send(ArenaEvent)
  handleEvent()                     │
                                    │  (boucle de discussion)
                                    │
invoke("pause_discussion")   ──►  mpsc::Sender<EngineCommand>
invoke("submit_user_message") ──►    └── EngineCommand::SubmitUserMessage
```

**Canal descendant** (Backend → Frontend) : `Channel<ArenaEvent>` (30+ types d'événements)
**Canal montant** (Frontend → Backend) : `invoke()` → commandes Tauri → `mpsc::Sender<EngineCommand>`

---

## 9. Patterns critiques

### Token buffering (prévention crash WebView)

Les tokens LLM arrivent un par un (~10K+ par tour). Appeler `Zustand.set()` pour chaque token provoque un crash du WebView. Solution : buffer module-level avec flush toutes les 60ms.

```typescript
// useTokenBuffer.ts — hook React
const { flushed, pushToken } = useTokenBuffer(60);

// synthBuffer[] — variable module-level pour la synthèse
```

### Sécurité UTF-8

Ne jamais indexer une chaîne par position de caractère comme index d'octets. Utiliser `str::floor_char_boundary()` (stable depuis Rust 1.82). Les noms français comme « Singularité » provoquent des panics sinon.

### Extraction depuis State<'_> avant await

Contrainte de lifetime Tauri v2 : extraire toutes les valeurs du `State<'_>` AVANT tout `.await`.

```rust
#[tauri::command]
async fn my_command(state: State<'_, AppState>) -> Result<()> {
    let db = state.db.clone();  // ← extraire AVANT
    db.call(|conn| { ... }).await?;  // ← .await APRÈS
    Ok(())
}
```

### Serde defaults pour les réponses LLM

Toujours `#[serde(default)]` sur les structs de réponse LLM — les modèles retournent souvent du JSON partiel ou invalide.

### Sauvegarde depuis le frontend

Sauvegarder depuis le handler `discussionEnded` du frontend (pas le moteur), car le `messages_history` du moteur n'inclut pas les réactions appliquées dans le store Zustand.

### Document Diff (Co-Construction)

Le système de diff permet de surligner les changements apportés au document collaboratif par chaque participant. Trois stratégies selon le format :

**Format TXT** : diff au niveau des mots (`diffWords` de la lib `diff`)
- Retourne des segments `{ text, highlighted }`
- Seuls les mots ajoutés sont surlignés

**Format MD** : diff au niveau des lignes (`diffLines`)
- Retourne un Set d'indices de lignes modifiées (0-indexed)
- Les lignes ajoutées sont surlignées dans le rendu markdown

**Format CSV** : diff au niveau des cellules
- Parse le CSV (séparateur `;`)
- Retourne un Set de clés `"row,col"` (0-indexed)
- Les cellules modifiées sont surlignées dans le tableau

**Usage** : `computeDocumentDiff(previousContent, currentContent, format) → DiffResult | null`

Le diff est recalculé à chaque `DocumentUpdated` event et appliqué visuellement dans la `DocumentSidebar`.

---

## 10. Sécurité & permissions

### Capabilities Tauri

```json
{
  "permissions": [
    "core:default",
    "opener:default",
    "dialog:default",
    "fs:default",
    "fs:allow-write-text-file"
  ]
}
```

### Principes

- **100% local** : aucun serveur distant requis (Ollama tourne en local)
- **Tavily optionnel** : seul accès externe, clé API stockée localement dans SQLite
- **Pas d'authentification** : application mono-utilisateur de bureau
- **SQLite** : base de données locale dans `%APPDATA%`

---

## 11. Logging & observabilité

| Sortie | Format | Détails |
|---|---|---|
| Console | Coloré + target/thread | Développement |
| Fichier | Sans ANSI, rotation quotidienne | `logs/` à côté de l'exécutable |
| Rétention | 7 jours | Suppression automatique |
| Niveau | `RUST_LOG=airena=info,airena_lib=info` | Configurable |

**Frontend** : `logger.ts` — buffer circulaire de 500 entrées, exportable en JSON.

---

## 12. Build & déploiement

### Développement

```bash
npm run tauri dev
```

Vite hot-reload (port 1420) + fenêtre Tauri avec DevTools.

### Production

```bash
npm run tauri build
```

Produit dans `src-tauri/target/release/bundle/` :
- Installateur MSI
- Installateur NSIS
- Exécutable standalone

### Configuration Vite

- Plugin React + Tailwind CSS
- Alias `@` → `./src`
- HMR configuré pour Tauri
- Ignore `src-tauri/` dans le watch

---

## 13. Tests

### Rust

```bash
cd src-tauri
cargo test            # Tous les tests
cargo test test_name  # Un test spécifique
cargo clippy          # Lint
```

**Couverture** : tests unitaires pour le parseur JSON, le moteur émotionnel, le gestionnaire de mémoire, le matching flou de noms.

### TypeScript

```bash
npx tsc --noEmit      # Vérification des types
```

---

## 14. Arborescence du projet

```
AIrena/
├── CLAUDE.md                  # Instructions pour Claude Code
├── docs/
│   ├── Technique/
│   │   └── TECHNICAL.md       # ← Ce document
│   ├── Fonctionnel/
│   │   └── FUNCTIONAL.md      # Documentation fonctionnelle
│   └── Plans/                 # Plans d'implémentation
├── package.json               # Dépendances Node + scripts
├── vite.config.ts             # Configuration Vite 7
├── tsconfig.json              # Configuration TypeScript 5.8
├── src/                       # Frontend React
│   ├── App.tsx                # Router + providers
│   ├── main.tsx               # Point d'entrée React
│   ├── pages/                 # 7 pages (Home, Setup, Arena, Summary, History, HistoryDetail, Settings)
│   ├── stores/                # 3 stores Zustand (Arena, Setup, Settings)
│   ├── components/
│   │   ├── discussion/        # Feed, MessageBubble, Controls, UserInput
│   │   ├── emotion/           # Sidebar, Cards, Sliders, Sparklines
│   │   ├── document/          # DocumentSidebar
│   │   ├── mindmap/           # MarkmapViewer, MindmapSidebar
│   │   ├── layout/            # AppShell, Sidebar, TopBar, ResizeDivider
│   │   ├── setup/             # LlmParamsForm, PersonaEditor, EmojiPicker
│   │   ├── shared/            # SimpleMd, MathText
│   │   └── common/            # ErrorBoundary
│   ├── hooks/                 # useTokenBuffer
│   ├── lib/                   # types, tauri-api, utils, logger, profile-emoji, document-diff
│   ├── i18n/                  # Locales FR/EN/ZH
│   ├── providers/             # ThemeProvider
│   └── styles/                # globals.css (Tailwind v4)
└── src-tauri/                 # Backend Rust
    ├── Cargo.toml             # Dépendances Rust
    ├── tauri.conf.json        # Configuration Tauri v2
    ├── capabilities/          # Permissions Tauri
    └── src/
        ├── main.rs            # Point d'entrée (Windows)
        ├── lib.rs             # Initialisation Tauri + commandes
        ├── state.rs           # AppState (Mutex + rag_store)
        ├── error.rs           # CommandError enum
        ├── constants.rs       # Constantes centralisées (limites, seuils, quotas RAG)
        ├── commands/          # Handlers IPC (discussion, ollama, settings, history, rag)
        ├── engine/            # Cœur métier
        │   ├── orchestrator.rs    # Boucle de discussion (~2800 lignes)
        │   ├── turn_manager.rs    # Distribution des tours
        │   ├── prompt_builder.rs  # Construction de prompts
        │   ├── directive_builder.rs # Personnalités cognitives
        │   ├── emotion_engine.rs  # Moteur émotionnel
        │   ├── memory_manager.rs  # Gestion mémoire
        │   ├── json_parser.rs     # Parsing robuste + matching flou
        │   ├── dynamics_parser.rs # Extraction XML <dynamics>
        │   └── mode_prompts.rs    # Instructions par mode
        ├── models/            # Structures de données (12 fichiers, incluant argument_map)
        ├── db/                # SQLite (schema, repository, seed)
        ├── ollama/            # Client HTTP + streaming NDJSON
        ├── rag/               # Système RAG (parser, chunker, embedder, bm25, store)
        ├── wikipedia/         # Client Wikipedia API
        └── tavily/            # Client Tavily API
```

---

## 15. Changelog

### v1.12.1 (2026-02-24) — Synthèse élargie, Validation labels, Export SVG complet

**Backend** :
- `SYNTHESIS_NUM_PREDICT = 4096` — budget tokens dédié pour la synthèse (2× le défaut) ; retry automatique avec budget doublé si la synthèse est tronquée (absence de conclusion)
- `ARGMAP_MIN_THESIS_LABEL_CHARS = 8` — nouvelle constante pour la validation minimale des labels de thèse
- `is_valid_thesis_label()` (`pub(crate)`) dans `json_parser.rs` — rejette les labels purement numériques et les labels < 8 chars
- **Anti-numérique dans le prompt** : les thèses existantes sont présentées en bullet points (pas de liste numérotée) + instruction explicite trilingue interdisant les indices
- **Résolution numérique** dans l'orchestrateur : si le LLM retourne un index numérique, tente de le résoudre vers la thèse existante
- **Garde anti-création** : les nouvelles thèses ne sont créées que si le label passe la validation
- `ARGMAP_MAX_THESIS_LABEL` ajusté de 60 → 200 et `ARGMAP_MAX_ARGUMENT_LABEL` de 100 → 400
- Retry de synthèse clone depuis `synth_params` (boosté) au lieu de l'original
- 6 nouveaux tests unitaires pour la validation des labels

**Frontend** :
- `buildStandaloneSvg()` dans `MarkmapViewer.tsx` — utilise `getBBox()` sur le `<g>` principal pour capturer l'intégralité du contenu SVG (pas seulement le viewport visible), supprime le transform D3, préserve les classes markmap et `--markmap-max-width`
- Off-screen `MarkmapViewer` sur la page Résumé — monté avec dimensions réelles (1200×800px) hors-écran pour permettre l'export SVG depuis n'importe quel onglet
- Whitelist des classes SVG à l'export (conserve markmap/mm-*, supprime Tailwind)
- 16 nouvelles constantes `BUDGET_*` (Token Budget floors + ceilings) dans `constants.rs`

### v1.12 (2026-02-22) — Optimisation contexte & Waterfall

**Backend** :
- Système de waterfall adaptatif pour l'allocation du budget de tokens entre les sections du prompt
- 16 constantes `BUDGET_FLOOR_*` et `BUDGET_CEIL_*` pour contrôler l'allocation par section

### v1.11.2 (2026-02-19) — Fix synthèse

**Backend** :
- Fix de la synthèse tronquée via augmentation du num_predict

### v1.11.1 (2026-02-18) — Fix mineur

- Corrections mineures de stabilité

### v1.11 (2026-02-17) — Illustration

**Frontend** :
- Illustration de la page d'accueil

### v1.10 (2026-02-14) — Carte des arguments & Améliorations UX

**Nouveaux fichiers** :
- `models/argument_map.rs` — Modèle de données (ArgumentType, ThesisNode, ArgumentNode, ArgumentMap)
- `components/mindmap/MarkmapViewer.tsx` — Rendu SVG interactif (forwardRef + markmap)
- `components/mindmap/MindmapSidebar.tsx` — Sidebar carte des arguments

**Backend** :
- Système de carte des arguments : extraction LLM, fusion incrémentale, conversion markdown
- `build_argument_extraction_prompt()` dans prompt_builder.rs — trilingual (FR/EN/ZH)
- `parse_argument_extraction()` dans json_parser.rs — parsing avec fuzzy matching
- Nouvel événement `ArgumentMapUpdated` (ArenaEvent) avec markdown, theses_count, arguments_count
- `DiscussionConfig.argument_map_enabled` — nouvelle option de configuration
- Migration DB : colonne `argument_map_md` sur la table `discussions`
- 8 nouvelles constantes ARGMAP_* dans constants.rs
- Centralisation de ~10 magic values restants dans constants.rs (Tavily, Ollama, Orchestrator, RAG)

**Frontend** :
- `MarkmapViewer` — composant forwardRef avec `useImperativeHandle` pour export SVG
- `MindmapSidebar` — sidebar droite avec header, compteurs, légende, collapse
- `ActivityStatus` dans useArenaStore — 10 types d'activité (thinking, writing, reacting, webSearch, wikiSearch, ragInjection, emotions, argumentMap, synthesis, determining)
- Indicateur d'activité dans la barre de titre (ArenaPage)
- Titres des étapes Setup (stepTitle_0..3)
- Onglet carte des arguments dans SummaryPage et HistoryDetailPage
- Export SVG via bouton conditionnel (onglet carte)
- `argumentMapMarkdown`, `argumentMapThesesCount`, `argumentMapArgumentsCount` dans useArenaStore
- `argumentMapEnabled` dans useSetupStore

**Dépendances ajoutées** :
- Frontend : `markmap-common`, `markmap-lib`, `markmap-view`

**i18n** :
- Clés mindmap.* (title, expand, collapse, empty, legendSupport, legendCounter, legendEvidence)
- Clés setup.* (argumentMap, argumentMapDesc, summaryArgumentMap, stepTitle_0..3)
- Clés summary.* (downloadArgumentMapMd, downloadArgumentMapSvg)
- Clés arena.activity.* (10 types d'activité)

### v1.9.1 (2026-02-13) — RAG v2

**Améliorations** :
- Optimisation du parsing PDF (gestion robuste des erreurs)
- Amélioration de la recherche hybride BM25 + cosine
- Fix : validation de la cohérence des dimensions d'embeddings
- Fix : gestion mémoire améliorée pour les gros documents

### v1.9 (2026-02-13) — RAG (Retrieval-Augmented Generation)

**Nouveaux fichiers** :
- `rag/parser.rs` — parsing multi-format (PDF, TXT, MD, CSV, DOCX)
- `rag/chunker.rs` — découpage avec overlap
- `rag/embedder.rs` — client embeddings Ollama
- `rag/bm25.rs` — recherche lexicale BM25
- `rag/store.rs` — stockage en mémoire (documents + chunks + embeddings)
- `commands/rag.rs` — commandes IPC RAG

**Backend** :
- Système RAG complet avec recherche hybride (BM25 + similarité cosine)
- Support de 5 formats de documents (PDF, TXT, MD, CSV, DOCX)
- Chunking configurable avec overlap (800 chars, 200 overlap par défaut)
- Embeddings via modèle Ollama configuré (ou modèle principal par défaut)
- Nouvel événement `ragContextInjected` avec détails des chunks utilisés
- 4 nouvelles commandes Tauri : `import_rag_document`, `remove_rag_document`, `get_rag_status`, `clear_rag_store`
- `AppState.rag_store` — nouveau champ pour le store RAG
- `CommandError::Rag` — nouvelle variante d'erreur
- Constantes RAG dans `constants.rs` (max file size, chunk size, overlap, top-k)

**Frontend** :
- `AppSettings.embeddingModel` — nouveau champ pour le modèle d'embeddings
- Types RAG : `RagDocumentInfo`, `RagChunkInfo`
- 4 nouvelles fonctions dans `tauri-api.ts`

**Dépendances ajoutées** :
- Backend : `lopdf` (parsing PDF), `regex` (parsing DOCX)

### v1.8.1 (2026-02-13) — Document Diff

**Nouveaux fichiers** :
- `lib/document-diff.ts` — calcul de diff pour surlignage des modifications

**Frontend** :
- Système de diff pour les documents collaboratifs (Co-Construction)
- Surlignage visuel des changements :
  - Format TXT : diff au niveau des mots (word-level)
  - Format MD : diff au niveau des lignes (line-level)
  - Format CSV : diff au niveau des cellules (cell-level)
- `DocumentSidebar` — surlignage appliqué automatiquement à chaque mise à jour
- Type `DiffResult` avec variantes par format

**Dépendances ajoutées** :
- Frontend : `diff` v8.x (bibliothèque de calcul de diff)

**Fixes mineurs** :
- Amélioration de la performance du rendu markdown pour les gros documents
- Fix : scroll automatique dans la sidebar document

### v1.8 (2026-02-13) — Profils améliorés

**Backend** :
- Expansion des profils prédéfinis (97 → 107 GladIAteurs, 10 → 12 IArbitres)
- Nouvelles catégories de profils : Sciences avancées, Arts, Sports

**Frontend** :
- Amélioration de l'interface de sélection de profils (filtrage par catégorie)
- Support de profils avec émotions initiales personnalisées

### v1.7 (2026-02-13) — Documentation

**Documentation** :
- Création de `Docs/Technique/TECHNICAL.md` — documentation technique complète
- Création de `Docs/Fonctionnel/FUNCTIONAL.md` — documentation fonctionnelle complète
- Mise à jour de `CLAUDE.md` avec patterns et conventions du projet

### v1.6 (2026-02-13) — Types de discussions

**Nouveaux fichiers** :
- `constants.rs` — constantes centralisées (limites mémoire, tailles de troncature, seuils, quotas Tavily/Wikipedia)
- `engine/mode_prompts.rs` (~940 lignes) — instructions trilingues par mode (7 fonctions)
- `components/document/DocumentSidebar.tsx` — sidebar de document collaboratif
- `components/shared/MathText.tsx` — rendu LaTeX via KaTeX

**Backend** :
- 8 modes de discussion (`DiscussionMode` enum) avec comportement orchestrateur distinct
- 4 formats de document (`DocumentFormat` enum) pour le mode Co-Construction
- Nouvel événement `DocumentUpdated` (ArenaEvent)
- `Reaction.justification: Option<String>` — les réactions incluent désormais une justification courte
- `mode_prompts.rs` : 7 fonctions mode-aware (descriptor, introduction, preamble, thought_focus, reaction_meanings, key_constraint, observer_clause)
- `directive_builder.rs` mode-aware : `discussion_mode` dans `SpeakerTurnContext`, `build_fiction_emotion_behavior()`, `SpeechAct::describe_fiction()`, rappels utilisateur contextuels
- `prompt_builder.rs` : introduction, réactions et pensée adaptés au mode
- Flux orchestrateur spécifiques : UserDriven (input forcé + opt-in gladiateurs), CollaborativeFiction (ouverture utilisateur + séquentiel), Socratic (question IArbitre chaque tour)
- 3 migrations DB : `discussion_mode`, `document_content`, `document_format`

**Frontend** :
- Sélecteur de mode (grille 2×4) + sélecteur de format document dans SetupPage
- Distribution des tours conditionnelle (grisée pour UserDriven/Fiction)
- Badge de mode dans l'ArenaPage + sidebar document
- `SimpleMd` réécrit (~250 lignes) : titres h1-h3, italique, code blocks, tableaux, règles horizontales, intégration MathText
- `downloadTextFile()` dans tauri-api.ts (via plugin dialog + fs)
- Sidebar de navigation : bouton Setup désactivé pendant la discussion, bouton Arena pulse si discussion active
- Résumé : badge de mode + bouton téléchargement document
- 50+ clés i18n ajoutées (modes, formats, document, navigation)

**Dépendances ajoutées** :
- Backend : `tauri-plugin-dialog`, `tauri-plugin-fs`
- Frontend : `@tauri-apps/plugin-dialog`, `@tauri-apps/plugin-fs`, `katex`, `@types/katex`

**Permissions Tauri** :
- `dialog:default`, `fs:default`, `fs:allow-write-text-file`

### v1.5 — Wikipedia

- Intégration Wikipedia API (recherche encyclopédique locale)
- Client Wikipedia avec fallback linguistique (zh→en)
- Filtrage de désambiguïsation par score de mots-clés
- Pool de quotas par discussion (`wiki_search_pool`)
- Événement `WikiSearchPerformed` avec URLs d'articles

### v1.4 — Personnalités cognitives

- Système de directives à 5 couches (`directive_builder.rs`)
- 10 actes de parole avec pondération émotionnelle
- Parseur de dynamiques XML (`dynamics_parser.rs`)
- Anti-répétition via historique personnel
- Conscience situationnelle (ambiance, bans, position)

### v1.3 — Émotions

- Moteur émotionnel rule-based (6 axes, 0-100)
- Profils émotionnels initiaux par persona
- Mise à jour par likes/dislikes/bans/stagnation
- Seuils d'alerte (85/15) avec événements frontend
- Sidebar émotionnelle avec sparklines
- Option `emotionDriven` (influence optionnelle sur le comportement)

### v1.2 — Internet

- Recherche web Tavily (API optionnelle)
- Suivi des quotas mensuels
- Décision de recherche par le LLM
- Événement `WebSearchPerformed`

### v1.1 — Modes de distribution & Build

- Modes démocratique (vote Borda masqué) et autoritaire (IArbitre décide)
- Build exécutable Windows (MSI/NSIS)
- Sauvegarde des profils personnalisés
- Fix affichage émotions

### v1.0 — Release initiale

- Architecture Tauri v2 + React 19 + Zustand 5
- Moteur de discussion avec streaming temps réel
- Mode think (Ollama)
- Système de réactions (like/dislike)
- Modération et bans par l'IArbitre
- Mémoire contextuelle + carte positionnelle
- Sauvegarde et historique des discussions
- Internationalisation FR/EN/ZH
- Thème dark/light
- 97 profils GladIAteur + 10 profils IArbitre prédéfinis

### v0.2 — Prototype

- Première version fonctionnelle du moteur

### v0.1 — Init

- Structure initiale du projet
