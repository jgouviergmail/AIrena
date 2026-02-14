<div align="center">

# 🏛️ AIrena

**Transformez vos modèles IA locaux en gladiateurs du débat**

[![Version](https://img.shields.io/badge/version-1.10-blue.svg)](https://github.com/jgouv/AIrena/releases)
[![Tauri](https://img.shields.io/badge/Tauri-2.x-24C8DB.svg)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-19-61DAFB.svg)](https://reactjs.org/)
[![Rust](https://img.shields.io/badge/Rust-1.93+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-All_Rights_Reserved-red.svg)]()

[Fonctionnalités](#-fonctionnalités-principales) •
[Installation](#-installation) •
[Documentation](#-documentation) •
[Usage](#-quick-start)

</div>

---

## 📖 Description

**AIrena** est une application desktop qui orchestre des discussions entre plusieurs intelligences artificielles locales. Définissez un sujet, sélectionnez vos personnalités IA (« GladIAteurs ») et un modérateur IA (« IArbitre »), puis observez et participez à un échange structuré en temps réel.

### 🔒 100% Local & Privé

L'application fonctionne **entièrement en local** via [Ollama](https://ollama.com), sans aucun envoi de données vers le cloud (sauf la recherche web Tavily, optionnelle).

---

## 📋 Table des matières

- [Fonctionnalités principales](#-fonctionnalités-principales)
- [Installation](#-installation)
- [Quick Start](#-quick-start)
- [Configuration](#-configuration)
- [Documentation](#-documentation)
- [Stack technique](#-stack-technique)
- [Architecture](#-architecture)
- [Développement](#-développement)
- [Build](#-build-de-production)
- [Roadmap](#-roadmap)
- [Contribution](#-contribution)
- [Support](#-support)
- [License](#-licence)

---

## ✨ Fonctionnalités principales

### 🎭 Système de personnalités

- **107 profils GladIAteur prédéfinis** répartis en 8 catégories :
  - Experts & Sciences (Scientifique, Philosophe, Biologiste, Climatologue...)
  - IT & Tech (Hackeur, Expert IA, DevOps, RSSI...)
  - Personnalités historiques (Socrate, Nietzsche, Einstein, Churchill...)
  - Écrivains (Victor Hugo, Shakespeare, Dostoïevski...)
  - Mode (Coco Chanel, Yves Saint Laurent, Karl Lagerfeld...)
  - Archétypes (Avocat du Diable, Optimiste, Féministe...)
  - Figures (Dieu, Satan, Bouddha, La Singularité, Extra-terrestre...)
  - Métiers & Sociaux (Médecin, Psychologue, Pilier de Bar, Startuper...)

- **12 profils IArbitre** : Modérateur Impartial, Provocateur, Maïeuticien, Juge Strict, Animateur TV, Thérapeute, Roi Philosophe, Agent du Chaos, Directeur Scientifique, Grand-mère...

- **Profils personnalisables** : créez vos propres personnalités avec system prompts, émotions initiales et paramètres LLM

### 🧠 Système cognitif avancé

- **Personnalités cognitives** à 5 couches :
  - Valeurs & déclencheurs émotionnels
  - Relations dynamiques (Allié, Rival, Tendu)
  - 10 actes de parole (Challenge, SteelMan, Anecdote, Provocation...)
  - Anti-répétition via historique personnel
  - Conscience situationnelle (ambiance groupe, position, bans)

- **Système émotionnel** (6 axes 0-100) :
  - Engagement, Accord, Confiance
  - Frustration, Curiosité, Enthousiasme
  - Mise à jour rule-based + influence optionnelle sur le comportement
  - Sidebar émotionnelle avec sparklines et sliders interactifs

- **Mode Think** : réflexion interne séparée du contenu visible (heuristique probabiliste selon le contexte)

### 🎪 8 modes de discussion

| Mode | Description |
|------|-------------|
| **💬 Débat** | Discussion contradictoire classique |
| **💡 Brainstorming** | Génération d'idées créatives sans critique |
| **🔨 Co-Construction** | Élaboration collaborative d'un livrable (TXT/MD/CSV) |
| **🎯 Guidé par l'utilisateur** | L'utilisateur oriente chaque tour |
| **🧘 Socratique** | Questionnement philosophique approfondi |
| **📚 Tutoriel** | Panel d'experts enseignants |
| **🔍 Revue critique** | Critique équilibrée (forces + améliorations) |
| **📖 Fiction collaborative** | Co-création narrative en relais |

### 🗺️ Carte des arguments (v1.10)

- **Extraction automatique** des thèses et arguments de la discussion via LLM
- **Mindmap interactive** (markmap) avec structure hiérarchique : Sujet → Orateur → Thèse → Arguments
- **3 types d'arguments** : ✅ Support, ❌ Contre-argument, 📊 Preuves
- **Fusion incrémentale** : la carte s'enrichit à chaque tour
- **Sidebar dédiée** dans l'Arena avec compteurs et légende
- **Export** en Markdown (.md) et SVG (.svg)
- **Persistance** dans l'historique

### 📊 Indicateur d'activité (v1.10)

- Affichage en temps réel de l'état du moteur (réflexion, écriture, recherche, émotions, synthèse…)
- Point lumineux animé dans la barre de titre de l'Arena

### 📚 RAG — Enrichissement documentaire (v1.9)

- **Import de documents** (PDF, TXT, MD, CSV, DOCX) jusqu'à 10 MB
- **Recherche hybride** : BM25 (lexical) + similarité cosine (sémantique)
- **Chunking intelligent** avec overlap configurable (800 chars, 200 overlap)
- **Embeddings** via modèle Ollama (configurable ou LLM principal)
- **Injection contextuelle** automatique dans les prompts
- **Interface de gestion** : import, suppression unitaire/globale, statut en temps réel

### 🔍 Recherche de connaissances

- **Wikipedia** : recherche encyclopédique (FR/EN/ZH) avec filtrage intelligent
- **Tavily** (optionnel) : recherche web en temps réel (API key requise, 1000 crédits/mois gratuit)
- **Pool de quotas** configurables par discussion
- **Décision LLM** : l'IA décide quand rechercher

### 📝 Document collaboratif (Co-Construction)

- **Surlignage visuel des modifications** (v1.8.1) :
  - TXT : diff au niveau des mots
  - MD : diff au niveau des lignes
  - CSV : diff au niveau des cellules
- **Badge de contribution** : "Dernière modification par : [nom]"
- **Sidebar dédiée** avec rendu markdown/tableau en temps réel

### 🎮 Contrôles avancés

- **4 modes de distribution des tours** :
  - Séquentiel (numéro d'intervention)
  - Aléatoire (mélange Fisher-Yates)
  - Démocratique (vote Borda masqué, parallèle)
  - Autoritaire (IArbitre décide seul)

- **Intervention utilisateur** : demandez la parole, timeout configurable
- **Modération IA** : commentaires, bans temporaires (1-3 tours)
- **Streaming token par token** avec buffer anti-crash (60ms)
- **Contrôles** : pause, reprise, arrêt doux/dur, force-stop

### 💾 Historique & Persistance

- **Sauvegarde automatique** (SQLite local)
- **Liste chronologique** avec emojis, sujet, nombre de tours
- **Vue détaillée** : relecture complète + synthèse + statistiques
- **Export** : téléchargement en TXT/MD/CSV

### 🌍 Internationalisation

- **3 langues** : Français (défaut), Anglais, Chinois
- **UI trilingue** + **prompts trilingues** (instructions internes adaptées)
- **Thème** : sombre / clair

---

## 🚀 Installation

### Prérequis

| Logiciel | Version | Lien |
|----------|---------|------|
| **Node.js** | LTS (20+) | [nodejs.org](https://nodejs.org/) |
| **Rust** | Stable (1.82+) | [rustup.rs](https://rustup.rs/) |
| **Ollama** | Dernière version | [ollama.com](https://ollama.com) |

### Étapes

```bash
# 1. Cloner le dépôt
git clone https://github.com/jgouv/AIrena.git
cd AIrena

# 2. Installer les dépendances
npm install

# 3. Télécharger un modèle Ollama (exemple)
ollama pull llama3.2

# 4. Lancer en mode développement
npm run tauri dev
```

---

## ⚡ Quick Start

1. **Lancer l'application**
   ```bash
   npm run tauri dev
   ```

2. **Configurer Ollama** (Paramètres)
   - Vérifier la connexion (`http://localhost:11434`)
   - Sélectionner un modèle

3. **Créer une discussion** (Nouvelle discussion)
   - **Étape 1** : Sujet + langue
   - **Étape 2** : Choisir un IArbitre + mode de distribution
   - **Étape 3** : Ajouter 2-N GladIAteurs
   - **Étape 4** : Mode de discussion + options (tours, pools)

4. **Observer** : streaming en temps réel, émotions, réactions, modération

5. **Consulter** : Synthèse + Historique

---

## ⚙️ Configuration

### Ollama

```bash
# Installer Ollama
# Windows : télécharger depuis ollama.com
# Mac/Linux : curl -fsSL https://ollama.com/install.sh | sh

# Télécharger un modèle
ollama pull llama3.2
ollama pull mistral
ollama pull qwen2.5

# Vérifier
ollama list
```

### Paramètres de l'application

| Paramètre | Description |
|-----------|-------------|
| **Nom d'utilisateur** | Nom affiché lors des interventions |
| **Langue** | Interface FR/EN/ZH |
| **Thème** | Sombre / Clair |
| **URL Ollama** | Par défaut : `http://localhost:11434` |
| **Modèle LLM** | Modèle principal pour les discussions |
| **Modèle d'embeddings** | Pour le RAG (optionnel, défaut : modèle LLM) |
| **Émotions influencent comportement** | Toggle ON/OFF |
| **Clé API Tavily** | Pour la recherche web (optionnel) |

---

## 📚 Documentation

- **[Documentation Technique](Docs/Technique/TECHNICAL.md)** — Architecture, API, patterns, stack
- **[Documentation Fonctionnelle](Docs/Fonctionnel/FUNCTIONAL.md)** — Guide utilisateur, modes, fonctionnalités
- **[CLAUDE.md](CLAUDE.md)** — Instructions pour Claude Code

---

## 🏗️ Stack technique

### Frontend

| Tech | Version | Rôle |
|------|---------|------|
| React | 19.1 | UI framework |
| TypeScript | 5.8 | Typage statique |
| Vite | 7.x | Build + HMR |
| Tailwind CSS | 4.x | Styles utilitaires |
| Zustand | 5.x | State management |
| React Router | 7.x | Routage SPA |
| i18next | 24.x | i18n |
| KaTeX | 0.16 | Rendu LaTeX |
| markmap | 0.18 | Mindmap interactive (carte des arguments) |

### Backend

| Tech | Version | Rôle |
|------|---------|------|
| Tauri | 2.x | Framework desktop |
| Rust | 1.93+ | Logique métier |
| tokio | 1.x | Runtime async |
| reqwest | 0.12 | HTTP + streaming |
| tokio-rusqlite | 0.6 | SQLite async |
| lopdf | 0.x | Parsing PDF (RAG) |

### Infrastructure

- **Ollama** : serveur LLM local (100% offline)
- **SQLite** : persistance locale
- **Tavily** (optionnel) : API de recherche web

---

## 🏛️ Architecture

```
┌─────────────────────────────────────────────┐
│           FRONTEND (WebView)                │
│   React 19 + Zustand 5 + Tailwind 4         │
│                                             │
│  Pages: Home, Setup, Arena, Summary,        │
│         History, HistoryDetail, Settings    │
│                                             │
│  Stores: Arena, Setup, Settings             │
│                                             │
│  ├─ DiscussionFeed (streaming + buffer)    │
│  ├─ EmotionSidebar (6 axes + sparklines)   │
│  ├─ DocumentSidebar (diff highlighting)    │
│  └─ MindmapSidebar (argument map)          │
│                                             │
└──────────────┬──────────────────────────────┘
               │ IPC (invoke + Channel)
┌──────────────┴──────────────────────────────┐
│           BACKEND (Rust)                    │
│   Tauri 2 + tokio + reqwest                 │
│                                             │
│  ┌─────────────────────────────────┐        │
│  │   DiscussionEngine              │        │
│  │   (orchestrator.rs ~2800 lines) │        │
│  │                                 │        │
│  │  ┌──────────┬──────────┬───────┴─┐      │
│  │  │Prompt    │Emotion   │Directive│      │
│  │  │Builder   │Engine    │Builder  │      │
│  │  └──────────┴──────────┴─────────┘      │
│  │                                 │        │
│  │  ┌──────────┬──────────┬───────┴─┐      │
│  │  │Memory    │Turn      │JSON     │      │
│  │  │Manager   │Manager   │Parser   │      │
│  │  └──────────┴──────────┴─────────┘      │
│  └─────────────────────────────────┘        │
│                                             │
│  ┌──────┬─────┬─────────┬──────────┐        │
│  │Ollama│ RAG │Wikipedia│  Tavily  │        │
│  │Client│Store│ Client  │  Client  │        │
│  └──────┴─────┴─────────┴──────────┘        │
│                                             │
│  DB: SQLite (settings, profiles,            │
│              discussions, messages)         │
└─────────────────────────────────────────────┘
```

### Flux de données

1. **Setup** → `buildConfig()` → `startDiscussion` (Tauri command)
2. **Engine** lance dans une tâche tokio
3. **Boucle de tours** :
   - Détermination ordre (Sequential/Random/Democratic/Authoritarian)
   - Construction prompt (contexte + émotions + directive)
   - Recherche RAG/Web/Wiki (optionnel)
   - Streaming LLM (+ think si heuristique active)
   - Collecte réactions (like/dislike)
   - Mise à jour émotions (rule-based)
   - Modération (ban éventuel)
   - Mise à jour document (Co-Construction)
   - Extraction carte des arguments (si activée)
4. **Événements** → `Channel<ArenaEvent>` → Zustand → React
5. **Synthèse** → Sauvegarde → Historique

---

## 🛠️ Développement

### Lancer en dev

```bash
npm run tauri dev
```

- **Frontend** : http://localhost:1420 (hot-reload)
- **Backend** : Tauri window avec DevTools

### Tests

```bash
# TypeScript
npx tsc --noEmit

# Rust (tous les tests)
cd src-tauri && cargo test

# Rust (un test spécifique)
cd src-tauri && cargo test test_name

# Lint Rust
cd src-tauri && cargo clippy
```

### Frontend seul (sans Tauri)

```bash
npm run dev          # Vite dev server
npm run build        # tsc + vite build
```

---

## 📦 Build de production

```bash
npm run tauri build
```

**Génère dans `src-tauri/target/release/bundle/` :**

- ✅ Installateur MSI : `AIrena_1.10.0_x64_en-US.msi`
- ✅ Setup NSIS : `AIrena_1.10.0_x64-setup.exe`
- ✅ Binaire standalone : `AIrena.exe`

---

## 🗺️ Roadmap

### ✅ Complété (v1.0 - v1.10)

- [x] Moteur de discussion multi-participants
- [x] Streaming temps réel
- [x] Système émotionnel (6 axes)
- [x] Personnalités cognitives (5 couches)
- [x] 8 modes de discussion
- [x] Recherche Wikipedia
- [x] Recherche web Tavily
- [x] Système RAG (documents PDF/TXT/MD/CSV/DOCX)
- [x] Document collaboratif avec diff highlighting
- [x] Carte des arguments (mindmap interactive)
- [x] Indicateur d'activité en temps réel
- [x] Historique & persistance
- [x] Internationalisation (FR/EN/ZH)

### 🚧 En cours / Prochaines versions

- [ ] **v1.11** : Visualisation graphique des relations entre participants
- [ ] **v1.12** : Profils avec avatars générés IA
- [ ] **v2.0** : Support multi-modèles (anthropic, openai, gemini)

### 💭 Idées futures

- Templates de discussions prédéfinis
- Mode spectateur (partage de discussions en cours)
- Analyse sémantique post-discussion
- Export audio (TTS par participant)

---

## 🤝 Contribution

Ce projet est actuellement un projet personnel. Les contributions ne sont pas acceptées pour le moment, mais les suggestions et les rapports de bugs sont les bienvenus via les [Issues](https://github.com/jgouv/AIrena/issues).

---

## 💬 Support

- **Bugs** : [Créer une issue](https://github.com/jgouv/AIrena/issues)
- **Documentation** : Voir [Docs/](Docs/)
- **Discussions** : [GitHub Discussions](https://github.com/jgouv/AIrena/discussions)

---

## 🙏 Remerciements

- **[Ollama](https://ollama.com)** — pour le serveur LLM local incroyable
- **[Tauri](https://tauri.app)** — pour le framework desktop moderne
- **[shadcn/ui](https://ui.shadcn.com)** — pour les composants UI élégants
- **[Anthropic](https://anthropic.com)** — pour Claude Code qui a assisté le développement

---

## 📄 Licence

© 2026 jgouv. Tous droits réservés.

Ce projet est un projet personnel. Toute reproduction, distribution ou utilisation commerciale est interdite sans autorisation expresse.

---

<div align="center">

**Fait avec ❤️ et 🦀 Rust**

[⬆ Retour en haut](#-airena)

</div>
