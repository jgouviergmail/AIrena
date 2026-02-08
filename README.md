# AIrena

**AIrena** est une application desktop qui transforme vos modeles IA locaux en gladiateurs du debat. Definissez un sujet, choisissez vos personnages, et regardez-les argumenter, reagir et s'affronter en temps reel — le tout orchestr par un arbitre IA.

L'application fonctionne **100% en local** via [Ollama](https://ollama.com), sans aucun envoi de donnees vers le cloud.

---

## Fonctionnalites

### Configuration de discussion
- **Choix du sujet** libre avec langue de discussion configurable (francais, anglais, chinois)
- **Arbitre IA (IArbitre)** : 10 profils predefinies (Moderateur Impartial, Provocateur, Maieuticien, Juge Strict, Animateur TV, Therapeute, Roi Philosophe, Agent du Chaos, Directeur Scientifique, Grand-mere) — ou creation de profils personnalises
- **GladIAteurs** : 30+ profils predefinies repartis en 5 categories :
  - **Experts** : Scientifique, Philosophe, Critique, Historien, Biologiste, Geographe, Mathematicien, Physicien, Chimiste, Climatologue
  - **Imaginaires** : Extra-terrestre, Chien, Chat, Dieu, Satan, La Singularite
  - **Personnalites** : Socrate, Nietzsche, Voltaire, Machiavel, Sun Tzu, Napoleon, Darwin, Einstein, Marx, Churchill
  - **Metiers** : Informaticien, Product Owner, Chef de Projet, Marketing, Hackeur, DevOps, RSSI, Comptable, Financier, Tradeur, Politicien
  - **Autres** : Avocat du Diable, Creatif, Optimiste, Pessimiste, Pragmatique, Feministe, Masculiniste, Complotiste, Humoriste, Pilier de Bar
- **Profils entierement personnalisables** : nom, system prompt et parametres LLM (temperature, top_p, top_k, numPredict) par participant
- **Distribution des tours** : sequentielle ou aleatoire
- **Nombre de tours** et **timeout d'intervention utilisateur** configurables

### Discussion en temps reel
- **Streaming token par token** des reponses IA avec affichage en temps reel
- **Systeme de reactions** : chaque IA peut liker/disliker les interventions des autres
- **Intervention utilisateur** : l'utilisateur peut demander la parole ; l'arbitre la lui accorde au moment opportun (avec timeout configurable)
- **Moderation par l'IArbitre** : possibilite de bannir temporairement un participant (notifications trilingues)
- **Synthese automatique** : l'IArbitre produit une synthese streamee en fin de discussion
- **Controles en direct** : force-stop, demande d'intervention

### Systeme emotionnel (6 axes)
Chaque GladIAteur possede un profil emotionnel dynamique mis a jour a chaque tour par un appel LLM :

| Axe | Bas (0) | Haut (100) |
|-----|---------|------------|
| **Engagement** | Desinteresse | Passionne |
| **Accord** | En opposition | En alignement |
| **Confiance** | Hesitant | Assertif |
| **Frustration** | Serein | Agace |
| **Curiosite** | Indifferent | Tres intrigue |
| **Enthousiasme** | Reserve | Exalte |

- **Indicateur visuel** colore par emotion dominante (vert, orange, rouge, bleu, jaune, gris) avec tooltip detaille
- **Mode emotion-driven** (optionnel) : les emotions influencent le style et le ton des reponses

### Mode reflexion (Think)
- **Heuristique probabiliste** : les IA activent le mode `think` d'Ollama selon le contexte (frustration, engagement, fin de discussion, contradiction)
- Les reflexions internes sont capturees et affichees separement du contenu visible

### Conscience de fin de discussion
- Les IA adaptent leur argumentation lorsque la fin approche (dernier tour, avant-dernier, etc.)
- Instructions trilingues injectees dans le prompt pour guider la conclusion

### Historique et persistance
- **Sauvegarde automatique** des discussions (messages, synthese, participants, emotions) en base SQLite locale
- **Page historique** : liste chronologique avec emojis des participants, sujet, nombre de tours, modele utilise
- **Consultation detaillee** : relecture complete avec bascule synthese/discussion
- **Suppression unitaire ou globale**

### Personnalisation
- **Theme** : sombre / clair
- **Langue de l'interface** : francais, anglais, chinois
- **Connexion Ollama** configurable (URL, modele, preload)
- **Avatars emoji** : attribution automatique selon le profil ou selection manuelle

---

## Stack technique

### Frontend
| Technologie | Version | Role |
|-------------|---------|------|
| **React** | 19 | UI et composants |
| **TypeScript** | 5.8 | Typage statique |
| **Vite** | 7 | Build et dev server |
| **Tailwind CSS** | 4 | Styles utilitaires |
| **shadcn/ui** | - | Composants UI (cn, clsx, tailwind-merge) |
| **Zustand** | 5 | State management |
| **React Router** | 7 | Routage SPA |
| **i18next** | 24 | Internationalisation |
| **Lucide React** | - | Icones |

### Backend (Tauri)
| Technologie | Version | Role |
|-------------|---------|------|
| **Tauri** | 2 | Framework desktop natif |
| **Rust** | 2021 edition | Logique metier et moteur de discussion |
| **tokio** | 1 | Runtime async |
| **reqwest** | 0.12 | Client HTTP (streaming) |
| **tokio-rusqlite** | 0.6 | Base de donnees SQLite async |
| **serde** / **serde_json** | 1 | Serialisation JSON |
| **thiserror** | 2 | Gestion d'erreurs |
| **chrono** | 0.4 | Dates et timestamps |
| **uuid** | 1 | Identifiants uniques |
| **tracing** | 0.1 | Logging structure |

### Infrastructure
- **Ollama** : serveur d'inference LLM local (requis a l'execution)
- **SQLite** : persistance locale (via rusqlite bundled)
- **IPC Tauri v2** : communication frontend ↔ backend via commandes et channels d'evenements

---

## Architecture

```
src/                          # Frontend React
├── components/
│   ├── common/               # ErrorBoundary
│   ├── discussion/           # DiscussionFeed, MessageBubble, SpeakerBadge,
│   │                         # EmotionIndicator, TurnIndicator, UserInputArea,
│   │                         # DiscussionControls, ReadOnlyFeed
│   ├── layout/               # AppShell, TopBar, Sidebar
│   └── setup/                # LlmParamsForm, EmojiPicker
├── i18n/locales/             # fr.json, en.json, zh.json
├── lib/                      # tauri-api, types, utils, profile-emoji
├── pages/                    # HomePage, SetupPage, ArenaPage, SummaryPage,
│                             # HistoryPage, HistoryDetailPage, SettingsPage
├── providers/                # ThemeProvider
└── stores/                   # useArenaStore, useSetupStore, useSettingsStore

src-tauri/src/                # Backend Rust
├── commands/                 # discussion, history, ollama, settings
├── db/                       # repository, schema, seed
├── engine/                   # orchestrator, prompt_builder, turn_manager,
│                             # emotion_engine, memory_manager, json_parser
├── models/                   # discussion, emotion, engine_command, events,
│                             # gladiateur, history, iarbitre, memory,
│                             # message, moderation, profile, settings
├── ollama/                   # client, types, error
├── error.rs
├── state.rs
├── lib.rs
└── main.rs
```

### Flux de discussion

```
SetupPage → buildConfig() → startDiscussion (Tauri command)
    ↓
DiscussionEngine (Rust)
    ├── IArbitre introduit le sujet
    ├── Boucle de tours :
    │   ├── turn_manager : determine l'ordre de parole
    │   ├── prompt_builder : construit le prompt contextuel
    │   ├── OllamaClient : streaming de la reponse
    │   ├── emotion_engine : mise a jour des emotions (appel LLM)
    │   ├── memory_manager : maj du resume et des positions
    │   ├── Reactions (likes/dislikes) generees par LLM
    │   └── Moderation IArbitre (commentaire, ban eventuel)
    ├── Intervention utilisateur (si demandee)
    └── Synthese finale par l'IArbitre
    ↓
ArenaEvents (Channel IPC) → useArenaStore (Zustand) → DiscussionFeed (React)
    ↓
SummaryPage → auto-save en SQLite → HistoryPage
```

---

## Prerequisites

- [Node.js](https://nodejs.org/) (LTS)
- [Rust](https://rustup.rs/) (stable toolchain)
- [Ollama](https://ollama.com) lance localement avec au moins un modele telecharge

## Installation

```bash
# Cloner le depot
git clone https://github.com/jgouv/AIrena.git
cd AIrena

# Installer les dependances frontend
npm install
```

## Developpement

```bash
npm run tauri dev
```

Lance simultanement le serveur Vite (http://localhost:1420) et la fenetre Tauri avec hot-reload.

## Build de production

```bash
npm run tauri build
```

Genere :
- Installeur MSI : `src-tauri/target/release/bundle/msi/AIrena_<version>_x64_en-US.msi`
- Setup NSIS : `src-tauri/target/release/bundle/nsis/AIrena_<version>_x64-setup.exe`
- Binaire standalone : `src-tauri/target/release/AIrena.exe`

## Configuration Ollama

1. Installer Ollama depuis [ollama.com](https://ollama.com)
2. Telecharger un modele : `ollama pull llama3.2`
3. Verifier que le serveur tourne : `ollama list`
4. Dans AIrena > Parametres, verifier la connexion et selectionner le modele

---

## IDE recommande

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

---

## Licence

Ce projet est un projet personnel. Tous droits reserves.
