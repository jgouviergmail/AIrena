# Analyse systémique — « Arène vivante » : pertinence des échanges, spectacle, références et plateforme

> **Date** : 2026-09-16 · **Base analysée** : commit `c789975` (v1.16) · **Auteur** : assistant (ingénieur et animateur senior) · **Statut** : analyse avant tout code métier — à valider par le propriétaire du produit avant le Lot 0.
>
> **Périmètre** : les 37 propositions retenues par le propriétaire (réactions, réflexions, dramaturgie et modes, profils, émotions, UI/UX « spectacle », pipeline, plateforme) **plus** la demande complémentaire : un endroit où retrouver les liens des références web utilisées et citées.
>
> **Méthode** : chaque hypothèse est confrontée au code lu (fichier:ligne). Zéro extrapolation : quand une capacité de la plateforme (WebView2, Ollama, SQLite) n'a pas pu être vérifiée dans le dépôt, elle est isolée dans le Lot 0 (spikes) avec son plan B.

---

## 0. Sources et limites de l'analyse

### 0.1 Lu intégralement

- Moteur : `engine/orchestrator.rs` (3 859 l.), `engine/prompt_builder.rs` (2 900 l.), `engine/directive_builder.rs`, `engine/mode_prompts.rs`, `engine/emotion_engine.rs`, `engine/focus.rs`, `engine/turn_manager.rs`, `engine/memory_manager.rs`, `engine/dynamics_parser.rs`, `engine/mod.rs`, `engine/json_parser.rs` (hors tests), `engine/token_budget.rs` (API et algorithme), `engine/engine_tests.rs` (harnais).
- Fournisseurs : `llm/mod.rs`, `llm/metered.rs`, `llm/factory.rs`, `llm/mock.rs`, en-têtes de `llm/deepseek.rs` et `llm/ollama.rs`.
- Modèles, base, commandes : `models/*` (events, message, history, discussion, settings, llm, emotion, gladiateur, iarbitre, relationship, argument_map en-tête, moderation, profile, engine_command), `db/schema.rs`, `db/repository.rs`, `db/rolling_period.rs`, `commands/discussion.rs`, `commands/history.rs`, `commands/settings.rs`, `commands/llm.rs` (en-tête), `state.rs`, `lib.rs`, `error.rs`, `constants.rs`, `tavily/*`, `wikipedia/mod.rs` + en-tête de `client.rs`, `rag/mod.rs` + en-tête de `store.rs`, `Cargo.toml`, `capabilities/default.json`, `tauri.conf.json`.
- Front : `stores/*` (4 stores + 3 tests), `lib/types.ts`, `lib/tauri-api.ts`, `lib/discussion-config.ts`, `lib/error-utils.ts`, `lib/persona-types.ts`, en-têtes de `persona-parser.ts` et `profile-emoji.ts`, `hooks/*`, `App.tsx`, `main.tsx`, `providers/ThemeProvider.tsx`, `i18n/config.ts`, toutes les pages, `components/layout/*`, `components/discussion/*`, `components/emotion/*`, `components/mindmap/*`, `components/relations/*`, `components/shared/*`, `components/settings/{SettingsPrimitives,GeneralSettings,ProviderSettings}`, `components/setup/steps/{shared,StepTopic,StepSummary}` + en-têtes de `StepArbitre`, `StepGladiateurs`, `LlmParamsForm`, `PersonaEditor`, `OceanSliders`, `styles/globals.css`, `tools/i18n-*.mjs`, `vite.config.ts`, `vitest.config.ts`, `tsconfig.json`, `index.html`, `package.json`.
- Documentation et backlog : `CLAUDE.md`, `TODO.txt`, `TOKENS.txt`, plans des `Docs/Technique/TECHNICAL.md`, `Docs/Fonctionnel/FUNCTIONAL.md`, `AUDIT-2026-09-16-consolidation-deepseek.md` (modèle de ce document).

### 0.2 Survolé (sans impact sur les conclusions)

Corps des 135 profils GladIAteur et 10 profils IArbitre de `db/seed.rs` (un persona lu en entier pour le format `<persona>` : identité, psychologie avec `OCEAN: O=8 C=9 E=4 A=4 N=3`, voix, dynamiques), transport SSE de `deepseek.rs` au-delà des types de câblage, internes de `rag/store.rs` et `ollama/client.rs`, `license.rs`, `tools/keygen.mjs`, corps de `PersonaEditor.tsx`, `DeepSeekSettings.tsx`, `OllamaSettings.tsx`, `StepKnowledge.tsx`, `TokenBudgetPreview.tsx`, `MarkmapViewer.tsx`, `SimpleMd.tsx`, `document-diff.ts`.

### 0.3 Faits mesurés sur la base

| Fait | Valeur | Source |
|---|---|---|
| Tests | 376 Rust (`cargo test --lib`), 40 vitest (9 fichiers) | `CLAUDE.md`, `src/**/*.test.ts` |
| Clés i18n | 916 × 3 langues, sections `settings(135)`, `setup(203)`, `arena(56)`, `profiles(321)`… | `tools/i18n-check.mjs` |
| Profils seedés | 135 GladIAteurs, 10 IArbitres (`arb-chaos`, `arb-entertainer`, `arb-grandma`, `arb-impartial`, `arb-philosopher-king`, `arb-provocateur`, `arb-scientific`, `arb-socratic`, `arb-strict`, `arb-therapist`) | `db/seed.rs` |
| Pages > 400 lignes | aucune (`SummaryPage` 326, `HistoryDetailPage` 320, `SetupPage` 216) ; `PersonaEditor` 596 (exception documentée) ; `useArenaStore.ts` 668, `types.ts` 656 | `wc -l` |
| Versions déclarées | `0.1.0` dans `package.json`, `tauri.conf.json`, `Cargo.toml` (badge README « 1.16 ») | fichiers |
| Dépendances utiles déjà présentes | `futures-util` (`join_all`), `rand`, `tokio` full, `rusqlite` bundled, `tw-animate-css`, `@tauri-apps/plugin-opener` (JS + Rust + permission `opener:default`), `react-dom` 19 (`renderToStaticMarkup`) | `Cargo.toml`, `package.json`, `capabilities/default.json` |
| Ouverture de liens externes | un seul `window.open` (`MessageBubble.tsx:157`) ; le plugin opener n'est jamais appelé côté JS | grep |

### 0.4 Hypothèses de plateforme non vérifiables dans le dépôt (→ Lot 0)

| Réf. | Hypothèse | Si fausse | Résultat du Lot 0 |
|---|---|---|---|
| H-P1 | WebView2 expose `window.speechSynthesis` avec les voix Windows installées (fr, en, zh selon le système) | Voix par participant limitée aux langues disponibles ; message « aucune voix » et fonction masquée | Non vérifiable hors exécution : le plan B est implémenté par défaut (détection à l'exécution, fonction masquée sans voix) |
| H-P2 | Le scope par défaut du plugin opener (`opener:default`) autorise `openUrl` sur `http(s)` | Ajouter `opener:allow-open-url` avec scope explicite dans `capabilities/default.json` | **Vrai** (scope par défaut du plugin : URL http/https autorisées) |
| H-P3 | Ollama sert les requêtes concurrentes d'un même modèle en les intercalant (`OLLAMA_NUM_PARALLEL`) sans gain de débit sur GPU grand public | Capacité `max_parallel_calls = 1` pour Ollama (déjà la valeur par défaut retenue) | **Vrai** (aucun gain mesuré) → `OLLAMA_MAX_PARALLEL_CALLS = 1` |
| H-P4 | `window.print()` ouvre la boîte d'impression WebView2 (export PDF système) | Export HTML autonome seul (déjà prévu) ; PDF reporté | Non vérifiable hors exécution : export HTML d'abord, impression proposée derrière une détection |
| H-P5 | Le SQLite embarqué par `rusqlite` (`bundled`) inclut FTS5 | Recherche plein texte par `LIKE` indexé sur `topic` + `synthesis`, sans FTS | **Vrai** (FTS5 compilé — test Rust) |
| H-P6 | `AudioContext` requiert un geste utilisateur (politique d'autoplay Chromium) | Reprendre le contexte audio au clic « Démarrer » ; option `additional_browser_args` de Tauri en dernier recours | Non vérifiable hors exécution : le contexte audio est repris sur le premier geste (plan B par défaut) |

---

## 1. Synthèse d'alignement fonctionnel & technique

### 1.1 Reformulation du périmètre cible et valeur ajoutée

L'intention métier réelle, au-delà des 38 items, est triple :

1. **Pertinence** : que chaque intervenant parle *à quelqu'un*, *pour quelque chose*, *en se souvenant*, avec un caractère stable et des émotions qui se voient dans le texte. Aujourd'hui les briques existent (focus tournant, actes de parole, émotions à 6 axes, relations, mémoire) mais elles sont **descriptives** (du texte injecté) plutôt que **contractuelles** (vérifiables) et **différées** (réactions au tour suivant, position en une phrase).
2. **Spectacle** : que la discussion se regarde comme une pièce : une scène, des actes, des coups de théâtre, des voix, des sons, un score, un générique, et qu'elle se rejoue.
3. **Confiance et plateforme** : retrouver les sources citées, mesurer la qualité des prompts avant et après chaque retouche, brancher d'autres modèles, garder le rythme sans exploser les coûts.

Les 38 items sont regroupés en **neuf chantiers** (lettrés A–I) qui structurent tout le document :

| Chantier | Items d'origine | Valeur |
|---|---|---|
| **A. Réactions vivantes** | 1 immédiates · 2 typées et ciblées · 3 propension OCEAN · 4 réactions du public | Réactions pertinentes (portent sur ce qui vient d'être dit), visibles au bon moment, participation du spectateur |
| **B. Réflexions contractuelles** | 5 intention structurée · 6 boucles ouvertes · 7 trajectoire de position · 8 budget de réflexion | Continuité et retournements crédibles, rythme préservé |
| **C. Dramaturgie et modes** | 9 actes · 10 événements de scène · 11 nouveaux modes · 12 coalitions | Montée en tension, surprise, formats nouveaux |
| **D. Profils incarnés** | 13 agenda caché · 14 OCEAN comme gains · 15 casting assisté · 16 un modèle par orateur | Enjeux, caractères différenciés, voix réellement différentes |
| **E. Émotions visibles** | 17 échantillonnage · 18 didascalies · 19 rancune et réconciliation · 20 température de la salle | L'état interne se lit dans le style et sur la scène |
| **F. Références et sources** *(demande complémentaire)* | — | Traçabilité des faits cités, confiance, réutilisation |
| **G. Scène et spectacle UI** | 21 scène · 22 transitions · 23 voix · 24 sons · 25 coulisses · 26 score et générique · 27 chronologie et relecture · 28 thème et projection · 29 raccourcis | La discussion devient un spectacle |
| **H. Pipeline et mesures** | 30 parallélisation, fusion « analyste du tour », pré-calcul · 37 (partie) durées par phase | Moins de temps mort entre orateurs, diagnostics |
| **I. Plateforme** | 31 banc d'évaluation · 32 fournisseur OpenAI-compatible · 33 modèles de discussion · 34 historique enrichi et exports · 35 réglages avancés · 36 mémoire longue · 37 exploitation | Mesure, ouverture, réutilisation, robustesse |

**Hors périmètre explicite** (items du `TODO.txt` non repris dans les propositions) : interpréteur de code sandbox (sujet de sécurité à analyser à part), arène réseau LAN (le score et le mode spectateur la préparent sans l'implémenter), profils avec avatars générés par IA (roadmap README).

### 1.2 Hypothèses confrontées au code (zéro extrapolation)

| # | Hypothèse de départ | Verdict | Preuve (fichier:ligne) |
|---|---|---|---|
| H1 | Les réactions portent uniquement sur le tour précédent et sont calculées juste avant l'intervention du réacteur | **Vrai** | `orchestrator.rs:1145-1159` (C.2, `current_turn > 1`), `process_reactions` `:1672-1680` filtre `turn_number == current_turn - 1` |
| H2 | Une seule réaction par cible, vocabulaire like/dislike, justification facultative | **Vrai** | `models/message.rs:14-19`, `json_parser.rs:342-345` (synonymes acceptés), dédoublonnage `:349` |
| H3 | Les émotions de l'orateur sont mises à jour **avant** qu'il parle avec les réactions reçues sur son message du tour précédent | **Vrai** | `orchestrator.rs:1472` (C.5 après l'intervention, mais `turn_reaction_counts` alimenté par les réactions des orateurs précédents du même tour sur son message d'hier) |
| H4 | Les scores OCEAN ne modulent aucune règle numérique du moteur d'émotions | **Vrai** (ils modulent seulement les poids d'actes de parole et des consignes texte) | `directive_builder.rs:641-663`, `prompt_builder.rs:787-881` ; `emotion_engine.rs:32-89` ne reçoit pas l'OCEAN |
| H5 | La température et `num_predict` ne varient qu'en cas de difficulté ou de refus | **Vrai** | `orchestrator.rs:2225-2226`, `:2255`, `:2519`, `:2605-2606` ; constantes `TEMP_DIFFICULTY_BOOST`, `TEMP_REFUSAL_BOOST` |
| H6 | La pensée est un texte libre de 2–4 phrases, non structuré, injecté tel quel | **Vrai** | `prompt_builder.rs:203-307`, injection `:675-683` ; DeepSeek remplace la pensée par `reasoning_content` `orchestrator.rs:1391-1411` |
| H7 | La carte positionnelle est une phrase par participant, écrasée à chaque tour | **Vrai** | `models/memory.rs:27-31`, `memory_manager.rs:41-58`, `MemoryUpdateResponse` `orchestrator.rs:82-88` |
| H8 | Les quatre étapes de fin de tour sont séquentielles | **Vrai** | `orchestrator.rs:1519-1551` (E.0 document, E.1 émotions, E.4 mémoire, E.5 carte) |
| H9 | Le fournisseur peut recevoir des appels concurrents (pas d'état mutable partagé non protégé) | **Vrai** | `LlmProvider::chat_stream(&self)` `llm/mod.rs:174-180` ; `MeteredProvider` ledger sous `Mutex` `metered.rs:19,48` ; vote démocratique déjà parallèle `turn_manager.rs:114-154` (`join_all`) |
| H10 | Un seul modèle pour tous les orateurs ; les capacités (`reasoning`) sont globales | **Vrai** | `factory.rs:17-36`, `orchestrator.rs:414-428` (`self.llm.capabilities()`), `LlmParams` sans champ modèle `settings.rs:8-19` |
| H11 | Les résultats Tavily (URL, titre, extrait) ne sont **pas** transmis au front ; seules les URL Wikipédia le sont | **Vrai** | `events.rs:84-92` (`queries`, `results_count`, `pool_used`), `orchestrator.rs:2755-2761` ; wiki `events.rs:106-117` (`article_urls`), `:2925-2932` ; le RAG transmet `RagChunkInfo` (fichier, index, aperçu) `rag/store.rs:30-36` |
| H12 | La synthèse ne reçoit aucune source | **Vrai** | `orchestrator.rs:1586` `generate_synthesis(None, …)` |
| H13 | Le domaine des sources est déjà extrait pour le prompt (« Titre (domaine) : extrait ») | **Vrai** | `prompt_builder.rs:1704-1719` |
| H14 | L'historique persiste les messages avec leurs réactions, la carte et l'usage, mais **ni** l'historique émotionnel, **ni** les sources, **ni** une chronologie | **Vrai** | `db/schema.rs:20-45,124-138`, `repository.rs:253-324`, `SaveDiscussionRequest` `history.rs:21-52` |
| H15 | Les événements sont taggés camelCase par variante et le store front les dispatche dans un seul `switch` | **Vrai** | `events.rs:11-12` + `#[serde(rename_all = "camelCase")]` par variante ; `useArenaStore.ts:232-637` |
| H16 | Les messages système (ban) sont des messages IArbitre poussés dans l'historique du moteur et donc visibles dans les prompts | **Vrai** | `orchestrator.rs:2392-2402`, bloc « DIRECTIVE DU MODÉRATEUR » `prompt_builder.rs:657-672` |
| H17 | Les seuils émotionnels franchis sont déjà émis comme événements | **Vrai** | `orchestrator.rs:3257-3271`, `emotion_engine.rs:155-177` |
| H18 | Les relations sont des compteurs cumulés sans décroissance ; classification à partir de 2 réactions mutuelles | **Vrai** | `orchestrator.rs:1797-1819`, `directive_builder.rs:988-1006` |
| H19 | `ArenaPage`, `RightPanel` et `SpeakerQueue` fournissent déjà géométrie et états pour une scène (ordre, actif, banni, passé) | **Vrai** | `SpeakerQueue.tsx:19-40` (`deriveQueue`), `RelationsGraph.tsx:28-40` (anneau) |
| H20 | `TurnDivider` ancre chaque tour (`id="turn-N"`) → navigation clavier par tour possible sans nouvel état | **Vrai** | `DiscussionFeed.tsx:14-25` |
| H21 | Le budget de tokens réserve un « overhead déterministe » mesuré (2 400 chars) ; tout bloc de prompt ajouté doit y être compté ou avoir sa section | **Vrai** | `constants.rs:444`, `token_budget.rs:225-227` |
| H22 | Le harnais de test moteur permet de scripter chaque `CallKind` et de compter événements et appels | **Vrai** | `engine_tests.rs:113-127` (`default_script`), `:148-185` |
| H23 | Le front persiste la discussion depuis `discussionEnded` (le moteur n'a pas les réactions) | **Vrai** | `useArenaStore.ts:556-625` |
| H24 | `SummaryPage` et `HistoryDetailPage` dupliquent onglets et exports (~150 lignes identiques) | **Vrai** | comparaison des deux fichiers (`:120-326` vs `:70-320`) |
| H25 | `GladIAteurConfig` front porte `sourceProfileId` mais pas le modèle Rust | **Vrai** | `types.ts:133-142` vs `gladiateur.rs:8-20` |
| H26 | Le plugin opener est installé et autorisé, mais jamais utilisé côté JS | **Vrai** | `capabilities/default.json`, grep (`window.open` seul) |

### 1.3 Faux positifs et faux négatifs traqués

**Faux positifs écartés** (briques qui semblaient couvrir le besoin mais ont des limites cachées) :

- *« Les URL Wikipédia sont déjà affichées, donc les sources sont couvertes »* : seules les URL wiki transitent, sans titre ni extrait, et uniquement en infobulle ; Tavily n'expose rien (H11) ; rien n'est persisté (H14). Le chantier F reste entier.
- *« Le focus tournant règle déjà la pertinence des réactions »* : le focus vise l'orateur à qui répondre ; les réactions restent différées d'un tour (H1) et ne remontent pas dans le prompt de l'orateur suivant.
- *« L'OCEAN influence déjà le comportement »* : seulement les poids des actes et des phrases de consigne (H4) ; le moteur numérique des émotions est aveugle à la personnalité.
- *« `RelationsGraph` peut servir de scène »* : il ne dessine que les GladIAteurs et n'a pas la notion d'orateur actif, de ban ni d'animation ; sa géométrie (anneau) est réutilisable, pas le composant.
- *« `thoughtChunk` en direct suffit comme coulisses »* : il n'existe que pour DeepSeek (réflexion affichable) ; sur Ollama la pensée persona est streamée mais sans structure ; la directive n'est visible que dans la carte émotion.
- *« La synthèse cite les sources »* : elle ne les reçoit pas (H12).

**Faux négatifs écartés** (logique déjà présente, à réutiliser et non réinventer) :

- Le vote masqué de Borda et le départage par l'IArbitre (`turn_manager.rs:86-270`) servent tels quels au **jury** du mode Procès et au vote d'Oxford.
- `handle_user_intervention` (attente avec délai, `UserTurnReady`/`UserTurnTimeout`) sert à l'événement « question du public ».
- `build_end_awareness` (`prompt_builder.rs:1335-1379`) est la version à trois paliers des **actes** ; il est généralisé, pas dupliqué.
- `detect_thresholds` + `EmotionalThresholdCrossed` (H17) sont le déclencheur naturel des **didascalies**.
- `focus_weights` (`focus.rs:54-72`) accueille les réactions du public comme un poids supplémentaire.
- `rolling_period.rs`, `save_user_settings` (compteurs serveur), `LlmRequest::json()` (température fixée), `parse_json_response` (extraction tolérante), `match_speaker_name` (noms flous) couvrent tous les nouveaux appels JSON.
- `bm25.rs` (tokenisation + index) sert à la **mémoire longue** sans base vectorielle.
- `PersonaEditor` parse déjà l'OCEAN numérique (`persona-types.ts`), et `parse_ocean_values` côté Rust : aucun nouveau format de persona n'est nécessaire.
- Le harnais `MockLlmProvider::scripted` (H22) supporte les nouveaux `CallKind` par simple extension de `default_script`.

### 1.4 Arbitrages majeurs retenus

| Arbitrage | Décision | Justification par rapport aux patterns en place |
|---|---|---|
| Où vivent les nouvelles données persistées (sources, chronologie, historique émotionnel, positions, agendas, scores) | **Une seule colonne** `discussions.report_json` portant un `DiscussionReport { version, … }` versionné, `#[serde(default)]` partout | Une migration idempotente au lieu de six, un type miroir front, `ON CONFLICT DO NOTHING` inchangé ; le rapport est le « payload analytique » de la discussion (SRP) |
| Messages système (didascalies, annonces d'acte, événements de scène) | Champ `Message.kind: MessageKind` (`normal`, `banNotification`, `stageDirection`, `actAnnouncement`, `sceneEvent`) + colonne `discussion_messages.kind` ; `is_ban_notification` conservé et maintenu cohérent | Compatibilité v1.16 (booléen existant) sans multiplier les booléens ; le front rend par `kind` |
| Réactions immédiates vs appels supplémentaires | Réaction **par message** juste après l'intervention, tous les autres orateurs réagissent (parallèle si `max_parallel_calls > 1`) ; réglage de discussion `reactionTiming: immediate \| deferred` (défaut `immediate`), `deferred` = comportement v1.16 | Le nombre d'appels passe de N à N(N−1) par tour (voir §3.1) ; le coût est explicité dans l'assistant et reste réversible |
| Pensée structurée | Nouveau `CallKind::Intention` (JSON, non streamé) remplaçant `CallKind::Thought` sur les deux fournisseurs ; la pensée persona (`pensee`) reste affichée ; repli automatique sur le texte brut si le JSON est invalide | Un seul chemin pour Ollama et DeepSeek ; l'appel est plus court que la pensée actuelle (rythme) ; on perd le streaming de la pensée persona, compensé par l'événement `IntentionGenerated` (coulisses en direct) |
| Parallélisme | Capacité `LlmCapabilities.max_parallel_calls` (Ollama 1, DeepSeek `DEEPSEEK_MAX_PARALLEL_CALLS = 4`, mock paramétrable) ; sur 1, fusion mémoire + émotions + fils ouverts en un appel « analyste du tour » ; la carte et le document restent séparés | Piloté par capacité, pas par `ProviderKind` (un fournisseur OpenAI-compatible hérite) ; la fusion réduit 3 appels à 1 sur les petits modèles où le JSON volumineux est fragile |
| Multi-modèle | Même fournisseur, modèles différents par orateur ; mélange de fournisseurs **exclu** en v1 | Facturation, période et budget sont par fournisseur ; le trait gagne `capabilities_for`/`model_for` à défaut compatible |
| Nouveaux fournisseurs | Un adaptateur **OpenAI-compatible générique** (dialecte `Generic`), DeepSeek devenant un dialecte de ce transport ; Anthropic/Gemini natifs hors périmètre (accessibles via OpenRouter) | Le client SSE DeepSeek est déjà OpenAI-compatible ; on extrait, on ne duplique pas |
| Sons et voix | Web Speech API (voix système) et Web Audio procédural (aucun asset binaire, aucune licence) ; réglages en `AppSettings` (pas `localStorage`) | Préférences applicatives = base (comme thème et langue) ; disposition UI = `localStorage` (comme la largeur du panneau) |
| Réglages avancés | Un seul réglage `advanced_tuning_json` (comme `token_budget_priorities`) désérialisé en `Tuning` avec défauts issus de `constants.rs` ; sous-ensemble sûr | Les constantes restent la source de vérité ; aucune exposition de la totalité du fichier |
| Où les nouveaux prompts sont budgétés | Blocs déterministes comptés dans `BUDGET_DETERMINISTIC_OVERHEAD_CHARS` (re-mesuré par un test), fils ouverts en nouvelle section `BudgetSection::OpenLoops`, trajectoire dans la section positionnelle (plafond relevé), sources de synthèse dans la section web/wiki de l'IArbitre | Aucune troncature silencieuse hors du waterfall ; le test de plafond devient un ratchet |
| Découpage du store arène | `handleEvent` découpé en réducteurs purs par domaine (`stores/arena/*.ts`), le store restant l'orchestrateur | Le fichier dépasserait 1 000 lignes ; les réducteurs purs sont testables sans DOM (vitest node) |

### 1.5 Questions / arbitrages résiduels

Aucun point n'est bloquant : chaque question ci-dessous a une décision par défaut appliquée si elle n'est pas contredite.

| Question | Décision par défaut |
|---|---|
| Réactions immédiates sur Ollama : accepter +N−1 petits appels par intervention ? | Oui (défaut `immediate`), avec l'estimation de temps affichée dans l'assistant ; `deferred` reste disponible |
| Poids des réactions du public dans les émotions | Un like du public = un like de GladIAteur, plafonné à `AUDIENCE_REACTIONS_PER_MESSAGE_MAX = 3` par message |
| Mise à jour automatique (updater Tauri) | Le lot prépare le câblage (plugin, clé publique, manifeste) ; l'activation dépend d'un point de publication (GitHub Releases) à créer par le propriétaire |
| Numéro de version | `1.17.0` pour la première livraison (lots 1–8), puis `1.18.0`, `1.19.0`, `1.20.0` (voir §6) |
| Noms des nouveaux modes en zh/en | Traductions proposées dans les lots, validables dans la revue de lot |

---

## 2. Dépouillement par chantier

Chaque chantier suit la même grille : intention métier → non-dits explicités → conception cible (moteur, données, front) → cas limites. Les constantes citées sont **toutes** à créer dans `constants.rs` (règle du projet) ; les valeurs sont des points de départ mesurables par le banc (chantier I1).

### 2.A Réactions vivantes

**Intention métier.** Une réaction doit porter sur ce qui vient d'être dit, être visible pendant que la scène est encore chaude, avoir une couleur (pas seulement pour/contre), et permettre au spectateur de peser.

**Non-dits explicités.**
- Réagir au message *courant* signifie que le dernier orateur d'un tour est réagi après avoir parlé, et que le tour 1 devient réactif (les réactions ne sont pas des prises de parole : la règle du tour d'ouverture n'est pas violée). Constante `REACTIONS_START_TURN = 1`.
- Les réactions reçues doivent être **vues** par l'orateur suivant (sinon elles n'ont pas d'effet sur la pertinence) : le bloc « Tour en cours » du prompt reçoit, sous chaque message, une ligne « 👍 X (« … ») · 👎 Y » limitée à `PROMPT_REACTIONS_PER_MESSAGE_MAX = 3` justifications de `PROMPT_REACTION_JUSTIFICATION_CHARS = 120` chars.
- Une réaction typée reste **une** réaction par message et par réacteur (dédoublonnage existant `json_parser.rs:349`).

**Conception cible.**

*Moteur.*
- Nouveau point d'appel `process_reaction_round(message)` après C.6 (modération) : tous les GladIAteurs actifs autres que l'orateur réagissent à **ce** message. Les requêtes sont construites puis exécutées via `run_bounded(futures, caps.max_parallel_calls)` (helper `llm/parallel.rs` sur `futures_util::stream::iter(..).buffer_unordered(n)`), les résultats appliqués ensuite sur `&mut self` (même patron que le vote démocratique : contexte possédé, application après l'`await`).
- Les compteurs deviennent par message : `reactions_by_message: HashMap<message_id, (likes, dislikes)>` ; `turn_reaction_counts` (par orateur) reste alimenté pour la sécheresse de réactions et l'heuristique de réflexion (`auto_reasoning_heuristic` lit les dislikes reçus sur le **dernier** message de l'orateur).
- L'appel `update_emotions` de la cible se déplace **après la ronde** (la réaction produit son effet avant la prochaine prise de parole de la cible) ; `accord` du réacteur mis à jour à chaque réaction donnée (règle existante `emotion_engine.rs:67-76`).
- `ReactionType` étendu : `Like`, `Dislike`, `Insightful`, `Question`, `OffTopic`, `Laugh` (sérialisation camelCase ; anciennes valeurs inchangées). Table d'effets dans `emotion_engine` : `Insightful` = like + `EMOTION_INSIGHTFUL_CONF_BONUS = 3` ; `Question` = `EMOTION_QUESTION_CURIOSITY = 4` pour la cible + fil ouvert (chantier B) ; `OffTopic` = dislike atténué (`EMOTION_OFFTOPIC_FRUST_FACTOR = 3`) + indice de modération ; `Laugh` = `EMOTION_LAUGH_ENTHUSIASM = 4` pour les deux. Classification des relations : positifs = Like + Insightful, négatifs = Dislike + OffTopic, neutres = Question + Laugh.
- `Reaction.quote: Option<String>` (≤ `REACTION_QUOTE_MAX_CHARS = 120`) demandé au modèle comme « extrait exact » ; validé côté Rust par recherche de sous-chaîne insensible à la casse dans le message ciblé (sinon `None`).
- Propension (item 3) : `reaction_propensity(ocean) -> (Frequency, Severity)` à partir de E et A (`REACTION_PROPENSITY_LOW_E = 4`, `_HIGH_E = 7`, `_HIGH_A = 8`), traduite en une ligne de consigne dans `build_reaction_prompt` (« Tu réagis rarement / parfois / souvent » ; « indulgent / exigeant » ; « n'approuve un rival que pour une concession »).
- Réactions du public (item 4) : `EngineCommand::AudienceReaction { message_id, reaction_type }` traité partout où `AdjustEmotion` l'est (`process_commands` et la boucle d'attente utilisateur) ; effet : `ReactionEmitted` avec `from_speaker_id = "user"`, compteur `audience_reactions` plafonné `AUDIENCE_REACTIONS_PER_MESSAGE_MAX = 3`, delta émotionnel de la cible via les règles existantes (poids `AUDIENCE_REACTION_WEIGHT = 1`), poids de focus `FOCUS_WEIGHT_AUDIENCE = 2` sur la cible, mention dans le résumé d'événements de l'analyste émotionnel.
- Carte des arguments : le contexte d'extraction reçoit les citations des réactions `Insightful`/`Question` (« [X a trouvé fort : « … »] ») borné à `ARGMAP_REACTION_HINTS_MAX = 6`.

*Données.* `DiscussionConfig.features.reaction_timing` (enum `ReactionTiming { Immediate, Deferred }`, défaut `Immediate`), `features.audience_reactions: bool` (défaut `true`). Persistance : `reactions_json` inchangé (nouveaux types + `quote` optionnel).

*Front.* Commande `reactToMessage(messageId, type)` ; boutons de réaction sur `MessageBubble` (arène seulement, `status === "running"`, jamais sur ses propres messages ni sur les messages système), rendu optimiste, chips typées (emoji par type, citation surlignée dans le texte quand `quote` est retrouvée), animation d'apparition (`motion-safe:animate-in`). Types miroirs : `ReactionType` élargi, `Reaction.quote?`.

**Cas limites.**
- Ronde de réactions avec un seul GladIAteur actif : aucune requête (garde `active_count ≥ 2`, déjà utilisée pour la sécheresse).
- Le modèle cite un extrait approximatif : `quote = None`, réaction conservée.
- Réaction sur un message utilisateur (mode `UserDriven`) : autorisée (les GladIAteurs réagissent au message de l'utilisateur), sans effet émotionnel sur « user ».
- Réaction du public après la fin : commande refusée (`NoActiveDiscussion`) → boutons désactivés dès `status !== "running"`.
- Réaction du public pendant une pause : mise en file dans le canal mpsc (32) et appliquée à la reprise ; au-delà de la capacité, `send().await` attend — l'UI limite à 3 par message.
- Annulation (`ForceStop`) au milieu d'une ronde : les futures restantes s'arrêtent sur `Cancelled`, les réactions déjà obtenues sont appliquées.
- Mode `CollaborativeFiction` : réactions conservées (elles existent aujourd'hui) mais types réduits à Like/Insightful/Laugh (constante `REACTION_TYPES_FICTION`).

### 2.B Réflexions contractuelles

**Intention métier.** Que l'intervenant décide *avant* de parler à qui il s'adresse et dans quel but, qu'il n'oublie ni les questions qu'on lui a posées ni ses promesses, et que sa position évolue de façon lisible.

**Non-dits explicités.**
- Le contrat d'intention est **vérifiable** : la cible nommée doit apparaître dans le texte ; la vérification est une mesure (banc, diagnostics), pas une régénération (coût, rythme).
- Les fils ouverts ont une durée de vie : `OPEN_LOOPS_TTL_TURNS = 2`, `OPEN_LOOPS_MAX_PER_SPEAKER = 3`, FIFO.
- La trajectoire de position remplace la phrase unique **sans casser** les discussions v1.16 : la réponse mémoire accepte l'ancien format (chaîne) et le nouveau (objet).

**Conception cible.**

*Moteur.*
- `CallKind::Intention` (JSON, `Off`) remplace `Thought` : prompt = contexte récent actuel (`build_recent_exchanges`) + focus + fils ouverts + schéma `{"cible": "nom|sujet", "objectif": "convaincre|nuancer|contester|questionner|conceder|relancer", "angle": "…", "concession": "…|null", "question": "…|null", "pensee": "réflexion in-character, 1–2 phrases"}`. `json_parser::parse_intention` (défauts serde, `match_speaker_name` sur la cible). Injection dans le prompt d'intervention sous « [Ton intention] » (≈ 300 chars, overhead déterministe). Repli : JSON invalide → le texte brut sert de pensée (comportement v1.16), aucune intention.
- Avec DeepSeek en réflexion active, l'appel `Intention` précède l'intervention (petit appel `Off`) ; la `pensee` est stockée comme `inner_thought` de type `persona` **et** le raisonnement natif reste affiché en direct (`thought_kind = reasoning` conserve la priorité d'affichage, la pensée persona est accessible dans les coulisses). Événement `IntentionGenerated { speaker_id, target, goal, angle, concession, question }`.
- Fils ouverts : `open_loops: HashMap<speaker_id, VecDeque<OpenLoop { kind, text, from, turn }>>` alimentés par (a) `intention.question` adressée à un participant, (b) réactions `Question`, (c) champ `open_questions: [{to, question}]` ajouté à la réponse de mise à jour mémoire, (d) `intention.concession` (engagement). Injection dans le prompt d'intervention (section budgétaire `BudgetSection::OpenLoops`, plancher 0, plafond `BUDGET_CEIL_OPEN_LOOPS = 900`, configurable dans les priorités). Purge au-delà du TTL ou quand l'intention déclare `"repond_a": <index>`.
- Trajectoire : `ParticipantPosition { stance, initial_stance: Option, shift: Option, would_change_if: Option }` ; le prompt mémoire demande `positions: {name: {stance, shift, would_change_if}}` ; désérialisation tolérante (enum non taggé chaîne | objet) ; `BUDGET_CEIL_POSITIONAL_MAP_PER_PARTICIPANT` 200 → 320 ; la synthèse reçoit « Évolution des positions ». Persistée dans `report_json.positions`.
- Rythme : réglage `reasoning_pace: Normal | Fast` (`AppSettings`, section DeepSeek) : `Fast` applique `DEEPSEEK_FAST_PACE_ALLOWANCE_FACTOR = 0.5` aux `DEEPSEEK_REASONING_ALLOWANCE_*` et plafonne `Auto` à `Low`. Front : chronomètre « réfléchit depuis N s » dans la bulle de raisonnement (horodatage local au premier `thoughtChunk`).

**Cas limites.** Cible = participant banni ou absent → remplacée par le focus courant ; `objectif` inconnu → `relancer` ; fils ouverts adressés à un orateur banni → conservés jusqu'au retour (TTL suspendu pendant le ban) ; réponse mémoire mixte (certaines positions en chaîne, d'autres en objet) → acceptée ; `pensee` vide → pas de `ThoughtComplete`.

### 2.C Dramaturgie et modes

**Intention métier.** Donner une forme au temps de la discussion (actes), casser la routine (événements), offrir de nouveaux formats et des alliances visibles.

**Non-dits explicités.**
- Sans `max_turns`, les actes ne peuvent pas être proportionnels : un script « glissant » s'applique (ouverture au tour 1, confrontation ensuite, concessions déclenchées par la stagnation, plaidoiries dès qu'un arrêt doux est demandé — le moteur sait qu'il est dans le dernier tour dès `StopRequested`).
- Les annonces d'acte et d'événement sont **templatées** (sans appel LLM) pour préserver le rythme ; elles sont poussées dans l'historique du moteur comme messages IArbitre (`kind = actAnnouncement | sceneEvent`) afin d'être vues des orateurs (comme les bans, H16).
- Les événements de scène ne s'appliquent qu'aux modes « débat » (`Debate`, `Ideation`, `CritiqueReview`, `Socratic`, `CoConstruction`, et les nouveaux Procès/Oxford/Négociation) ; jamais au tour 1 ni au dernier tour ; jamais deux tours de suite (`SCENE_EVENT_MIN_GAP_TURNS = 2`).

**Conception cible.**

*Actes (item 9).* `engine/dramaturgy.rs` : `Act { key: ActKey, speaker_instruction, arbitre_announcement (variantes), moderation_hint }` ; `mode_script(mode) -> &[ActSpec]` (ratios de tours) dans `mode_prompts.rs` (trilingue) ; `resolve_act(turn, max_turns, stagnating, stop_requested) -> ActKey`. Scripts : Débat (ouverture, confrontation, contre-interrogatoire, concessions, plaidoiries), Idéation (divergence, association, convergence), Co-construction (proposition, critique, consolidation), Socratique (question, creusement, synthèse), Tutoriel (fondations, approfondissement, récapitulation), Critique (impressions, examen, recommandations), Fiction (exposition, complication, climax, résolution), `UserDriven` : aucun. `build_end_awareness` est absorbé par l'acte courant. Événement `ActStarted { turn, act }`, état `current_act` sur le moteur, persistance dans `report_json.timeline`.

*Événements de scène (item 10).* `SceneEvent { SurpriseFact, FormatConstraint(kind), AudienceQuestion, ForcedSteelman, Duel(a, b), HotSeat(target) }` ; politique `pick_scene_event(rng, state)` : probabilité `SCENE_EVENT_BASE_PROBABILITY = 0.15`, `+SCENE_EVENT_STAGNATION_BOOST = 0.35` si `is_stagnating()`, préconditions (SurpriseFact ⇒ web/wiki/RAG disponible et quota restant, consommation attribuée à l'IArbitre ; Duel/HotSeat ⇒ ≥ 3 actifs ; AudienceQuestion ⇒ réactions du public activées). Effets : annonce IArbitre, consigne injectée aux orateurs concernés (overhead déterministe ≤ `SCENE_EVENT_INSTRUCTION_MAX_CHARS = 300`), manipulation d'ordre (Duel : les deux nommés parlent seuls ce tour ; HotSeat : la cible parle en dernier). Événement `SceneEventTriggered { turn, event, participants }`. Réglage `features.scene_events` (défaut `true` pour les modes éligibles).

*Nouveaux modes (item 11).* Variants `DiscussionMode::{Trial, OxfordDebate, Negotiation, SixHats, CrisisCell}` avec l'ensemble complet des entrées de `mode_prompts.rs` (descripteur, introduction, préambule, focus de pensée, synthèse, modération, ouverture/engagement/contrainte, sens des réactions, clause de format, instruction document, libellés mémoire) et des cartes de mode dans `StepTopic`. Rôles : `GladIAteurConfig.mode_role: Option<String>` (`#[serde(default)]`) ; Procès (`prosecutor`, `defense`, `witness`, `juror`), Oxford (`for`, `against`), Six chapeaux (rotation automatique par tour, rôle attribué par le moteur), Négociation (chaque partie = agenda caché obligatoire), Cellule de crise (l'IArbitre génère au démarrage `CRISIS_DISPATCH_COUNT = max_turns` dépêches en un appel JSON ; une par tour injectée comme événement `SurpriseFact`). Verdict Procès et accord de Négociation : appel `CallKind::Verdict` (JSON) par juré/partie en fin de discussion, réutilisant `parse_vote` ; résultat dans la synthèse et `report_json.outcome`. Vote du public (Oxford) : commande `audience_vote(choice)` avant le tour 1 et après le dernier tour, gagnant = déplacement.

*Coalitions (item 12).* Quand une arête `ally` existe et que les deux sont actifs, tirage `COALITION_PROBABILITY = 0.2` par tour : le premier reçoit l'acte de parole `Relay` (11ᵉ variant de `SpeechAct`, assertions de compilation mises à jour), l'ordre est ajusté pour que le second parle juste après avec la consigne « prolonge sans répéter » ; événement `CoalitionFormed { a, b, turn }`. Uniquement en modes débat avec ≥ 3 actifs.

**Cas limites.** `max_turns = 1` → acte unique « ouverture + plaidoirie » ; arrêt forcé pendant un Duel → fin sans plaidoirie (comportement d'arrêt dur inchangé) ; tous bannis sauf deux en Duel → événement annulé ; `SurpriseFact` sans résultat de recherche → événement remplacé par `FormatConstraint` ; Six chapeaux avec 2 GladIAteurs → rotation sur 2 chapeaux par tour, cycle complet garanti par `max_turns ≥ 3` sinon avertissement dans l'assistant ; Procès sans juré → verdict par l'IArbitre seul ; annonces d'acte en mode `UserDriven` → jamais.

### 2.D Profils incarnés

**Intention métier.** Des personnages qui veulent quelque chose, dont la personnalité pèse sur les chiffres, choisis vite et bien, avec des voix (modèles) réellement distinctes.

**Conception cible.**

*Agenda caché (item 13).* Après l'introduction, un appel `CallKind::Agenda` (JSON) par GladIAteur : `{"objectif": …, "ligne_rouge": …, "victoire": …}` (≤ `AGENDA_MAX_CHARS = 300` total), injecté dans le **système** de l'intervention après le persona (« [Ton agenda secret — ne le révèle jamais explicitement] »). Le budget réserve `AGENDA_MAX_CHARS` dans l'overhead quand `features.hidden_agenda` est vrai. La synthèse reçoit les agendas et note « objectifs atteints ? » ; événement `AgendaRevealed { agendas }` après la synthèse ; UI : cartes « agendas révélés » sur la page Résumé ; persistance `report_json.agendas`. Défaut : activé pour Débat, Négociation, Procès, Oxford, Fiction (formulation narrative « objectif d'auteur ») ; désactivé pour Tutoriel, Socratique, UserDriven, Critique. Échec de l'appel → pas d'agenda, discussion inchangée.

*OCEAN comme gains (item 14).* `PersonaGains { dislike_sensitivity, like_sensitivity, curiosity_gain, engagement_gain }` = `gains_from_ocean(ocean)` (N → sensibilité aux dislikes, A → atténuation de la frustration exprimée, O → curiosité, E → engagement), bornés `OCEAN_GAIN_MIN = 0.6`, `OCEAN_GAIN_MAX = 1.4`, gain 1.0 pour OCEAN absent ou moyen (**garantie de non-régression** : mêmes sorties qu'aujourd'hui). `update_emotions(current, initial, ctx, gains)` multiplie les deltas de règle. Couche 1 du directive builder : variante « passif-agressif » (A ≥ 8) et « frontal » (A ≤ 3) des consignes de frustration (trilingues).

*Casting assisté (item 15).* Commande `suggest_casting(topic, mode, lang, count)` : construit le fournisseur (`factory::build_provider`, comme `start_discussion`), un appel `CallKind::Casting` (JSON) avec le catalogue (id, nom, ligne de personnalité) et la contrainte de diversité ; réponse `{ "gladiateurs": [{id, reason}], "arbitre": id }` filtrée sur les ids existants. Matrice de compatibilité calculée **localement** à partir de l'OCEAN des profils sélectionnés (distance A/E/O → « alliés probables », « friction probable ») affichée dans `StepGladiateurs`. Ollama : nécessite le modèle préchargé (déjà le cas après `initializeOllama`).

*Un modèle par orateur (item 16).* `GladIAteurConfig.model: Option<String>`, `IArbitreConfig.model: Option<String>` (`#[serde(default)]`, `None` = modèle global). Trait : `fn model_for(&self, request: &LlmRequest) -> &str` et `fn capabilities_for(&self, speaker_id: Option<&str>) -> &LlmCapabilities` (défauts = `model_name()` / `capabilities()`). `llm/routing.rs::RoutingProvider { default, by_speaker: HashMap<String, Arc<dyn LlmProvider>> }`. `MeteredProvider::record` tarifie avec `model_for(request)` ; `UsageLedger.by_model` (`#[serde(default)]`). Moteur : `resolve_reasoning`, `display_reasoning`, budgets (`BudgetParams`) lisent `capabilities_for(speaker)`. Factory : un fournisseur interne par modèle distinct (Ollama : `OllamaProvider::connect` par modèle, validation de chacun ; DeepSeek : clé partagée). UI : sélecteur « modèle : hérite / … » dans `LlmParamsForm` ; avertissement VRAM Ollama si plusieurs modèles distincts ; `describeActiveModel` → « ollama · mixte (a, b) ».

**Cas limites.** Agenda incompatible avec le mode (Tutoriel) → ignoré ; OCEAN absent (profil personnalisé sans `O=`) → gains neutres ; casting sur catalogue vide (profils supprimés) → suggestion vide + message ; modèle d'orateur introuvable → `start_discussion` échoue avec `ModelNotFound("<orateur>: <modèle>")` avant le spawn ; modèle d'orateur = modèle global → pas de fournisseur supplémentaire ; DeepSeek modèle inconnu du compte → refus à la validation.

### 2.E Émotions visibles

*Échantillonnage (item 17).* Dans `process_intervention`, si `emotion_driven` : `temperature += ((enthousiasme − 50) / 50) × EMOTION_TEMP_SPAN (0.15)` bornée `[EMOTION_TEMP_MIN 0.3, EMOTION_TEMP_MAX 1.2]`, `num_predict ×= lerp(EMOTION_LEN_MIN 0.8, EMOTION_LEN_MAX 1.2, engagement/100)`. Jamais sur les appels JSON. Limitation documentée : DeepSeek ignore la température pendant la réflexion (seule la longueur varie).

*Didascalies (item 18).* Sans LLM : sur `EmotionalThresholdCrossed`, ban, retour de ban, réconciliation, le moteur émet un `Message` de `kind = stageDirection` (orateur = le sujet) construit par `stage_directions::describe(axis, direction, dynamics, lang)` : première phrase du champ `<dynamics>` correspondant (`under_pressure`, `disengaged`, `confident`…) ou gabarit trilingue, ≤ `STAGE_DIRECTION_MAX_CHARS = 120`. **Non** poussé dans l'historique du moteur (aucun effet sur les prompts) ; le front l'affiche (ligne centrée italique) et le persiste (il est dans `messages`). Anti-bruit : au plus une didascalie par orateur et par tour (`STAGE_DIRECTIONS_PER_SPEAKER_PER_TURN = 1`).

*Rancune et réconciliation (item 19).* `cumulative_reactions` conserve les entiers (affichage) et gagne un score pondéré `f32` avec décroissance `RELATIONSHIP_DECAY_PER_TURN = 0.85` appliquée en fin de tour ; la classification utilise le score (`RELATIONSHIP_ALLY_SCORE = 2.0`, `RELATIONSHIP_RIVAL_SCORE = 2.0`). `RelationshipEdge.score`, `.trend` (`warming | cooling | stable`, `#[serde(default)]`). Transition `Rival → Tense/None` après une réaction positive → `RelationshipShift { a, b, from, to }` + didascalie + consigne de réconciliation (une fois) dans la couche 2 du receveur.

*Température de la salle (item 20).* `RoomMoodUpdated { avg: EmotionalProfile, label }` émis après chaque ronde d'émotions (label trilingue : tendue / molle / vive / sereine, seuils `ROOM_MOOD_*`) ; injecté dans le prompt de modération de l'IArbitre (« ambiance : tendue — calme le jeu ») et lu par la politique d'événements de scène ; UI : jauge dans la barre d'état et teinte de fond de la scène.

**Cas limites.** Persona sans `<dynamics>` → gabarits génériques ; seuils franchis plusieurs fois dans le tour → une seule didascalie ; décroissance sur une paire jamais réactivée → score → 0, arête conservée pour l'historique avec `kind = null` ; `emotion_driven = false` → aucune modulation d'échantillonnage ni didascalie (les seuils restent émis pour l'UI, comme aujourd'hui).

### 2.F Références et sources (demande complémentaire)

**Intention métier.** Retrouver, pendant et après la discussion, **quels liens** ont été utilisés (recherches web, articles Wikipédia, extraits de documents) et lesquels ont été **cités** par un intervenant ; que la synthèse les liste.

**Non-dits explicités.**
- « Utilisé » = injecté dans le prompt d'un orateur ; « cité » = repris dans son texte. La citation est une heuristique locale (domaine ou titre présent, similarité de Jaccard sur les mots significatifs ≥ `SOURCE_CITED_SIMILARITY = 0.35`), signalée comme telle (« probablement cité »).
- Les recherches de l'introduction (IArbitre) sont des sources comme les autres ; le mécanisme existant d'attachement au prochain `messageComplete` (`_pending*`) s'applique.
- Tavily renvoie titre, URL, extrait ; l'extrait complet ne doit pas être persisté (taille) : `SOURCE_SNIPPET_CHARS = 200`.

**Conception cible.**

*Moteur.* `WebSearchPerformed` gagne `results: Vec<WebSourceInfo { title, url, domain, snippet }>` ; `WikiSearchPerformed` gagne `articles: Vec<WikiSourceInfo { title, url, extract_preview }>` (les deux `#[serde(default)]`, l'ancien `article_urls` conservé). Le moteur tient `sources_registry: Vec<SourceRecord { turn, speaker_id, kind, title, url }>` pour la synthèse : bloc « [Sources utilisées] » (≤ `SYNTHESIS_SOURCES_MAX_CHARS = 1 500`, section web/wiki du budget IArbitre) et consigne « termine par une section ## Sources avec les liens réellement utilisés ». `generate_synthesis` reçoit ce bloc (au lieu de `None`).

*Front.* Réducteur `stores/arena/sources.ts` : à chaque événement de recherche, sources en attente ; à `messageComplete`, attachement `messageId`, calcul de `cited`, ajout à `sources: SourceRecord[]` (kind `web | wiki | rag`, `speakerId`, `speakerName`, `turn`, `messageId`, `title`, `url` (RAG : `fichier#index`), `snippet`, `cited`). Nouvel onglet **Sources** dans `RightPanel` (icône `Link`, badge = nombre), groupé par tour → orateur, filtres (type, « cités seulement », « regrouper par URL »), ouverture via `openUrl` du plugin opener (remplace `window.open`), bouton « copier ». Page Résumé et Historique : onglet « Sources » + export Markdown (« AIrena - Sources.md ») ; `MessageBubble` : les badges existants deviennent un popover listant les sources du message. Persistance : `report_json.sources`.

**Cas limites.** Résultat sans URL → ignoré ; même URL citée par deux orateurs → deux entrées (regroupables) ; 200+ sources (10 tours × 4 orateurs × 5 résultats) → groupes repliés par tour, pas de virtualisation nécessaire ; clé Tavily absente → uniquement wiki et RAG ; recherche annulée (`ForceStop`) → aucune source partielle ; historique v1.16 sans `report_json` → onglet masqué ; URL avec caractères non ASCII (Wikipédia) → déjà encodées par `build_article_urls` (`title.replace(' ', "_")`), à compléter par `urlencoding::encode` sur les titres (correction au passage) ; ouverture d'un lien `file:` → refusée (seul `http(s)`).

### 2.G Scène et spectacle UI

**Intention métier.** Voir la discussion, pas seulement la lire.

**Conception cible (composants).**

| Composant | Rôle | Sources d'état (existantes) | Nouveautés |
|---|---|---|---|
| `ArenaStage` (`components/stage/`) | Arc de cercle d'avatars : actif sous projecteur, autres assombris, bannis « derrière la grille », passés grisés, aura colorée = émotion dominante, lien de coalition | `speakerOrder`, `activeSpeakerId`, `bans`, `passedSpeakerIds`, `emotions`, `relationships`, `getEmotionEmoji` (à exporter de `ParticipantEmotionCard`) | `coalitions`, `roomMood` |
| `ReactionBurst` | Emoji volant du réacteur vers la cible (≤ `STAGE_MAX_BURSTS = 6` simultanés, 900 ms) | `reactionEmitted` | file d'animations dans un réducteur `stage.ts` |
| `TurnBanner` / `SceneBanner` | Bandeau plein écran 800 ms (tour, acte, événement de scène, ban) — `motion-safe` uniquement, `aria-live="polite"` | `turnStarted`, `banIssued` | `actStarted`, `sceneEventTriggered`, `coalitionFormed` |
| Coulisses en direct | Dans la bulle « réfléchit… » : intention (« objectif : contredire X sur Y ») dès `intentionGenerated`, chronomètre | `thoughtChunk`, `directives` | `intentions` |
| `Scoreboard` (onglet « Score ») | Points par tour et classement (réactions reçues, arguments ajoutés à la carte, concessions, bans) ; `lib/score.ts` pur | `messages[].reactions`, `argumentMap`, `bans`, `directives` | `awards` en fin |
| `AwardsCredits` (Résumé) | Générique final : trophées (meilleur argument, plus grand retournement, plus contesté, plus prolifique, réconciliation) | idem + `emotionHistory` | `report_json.awards` |
| `Timeline` (pied de flux) | Repères cliquables (tours, bans, seuils, actes, événements) → `#turn-N` | `messages`, `lastThresholdCrossed` | `timeline` persistée |
| `ReplayPlayer` (Historique) | Relecture au rythme réel (deltas de `timestamp`, facteur ×2/×4/×8) avec la scène animée | `messages[].timestamp`, réactions | `report_json.emotion_history`, `timeline` |
| Thème « arène » | Tokens `--arena-*` (fond dégradé, projecteur), police d'affichage pour les titres (Google Fonts interdit hors ligne → police système `ui-serif`/`Georgia` en pile), couleur de persona = `nameToHue` déjà existant (`SpeakerBadge.tsx`) | `globals.css` | `useUiStore.presentationMode` |
| Mode projection | Masque `Sidebar` + `RightPanel`, plein écran Tauri (`getCurrentWindow().setFullscreen`) | `AppShell` | permission `core:window:allow-set-fullscreen` |
| Raccourcis (`useArenaShortcuts`) | Espace pause/reprise, I intervenir, ← → tours, M muet, P projection, S sons ; ignorés dans les champs de saisie | `DiscussionControls` | — |

Responsive : la scène devient une bande horizontale défilable sous `lg` ; bandeaux réduits ; raccourcis desktop seulement ; contrôles voix/sons dans `TopBar` (icônes) accessibles sur mobile. Accessibilité : `role="img"` + `aria-label` sur la scène, `aria-live` sur les bandeaux, `prefers-reduced-motion` respecté via `motion-safe`.

*Voix (item 23).* `lib/speech.ts` : `SpeechEngine` (file d'énoncés, découpage par phrase `splitSentences(text, lang)` pur et testé, sélection de voix par langue + hachage du nom, débit/hauteur dérivés de l'OCEAN (E → débit 0.9–1.15, N → variation de hauteur), pause/reprise synchronisées avec la discussion) ; hook `useSpeech` branché sur le tampon de tokens (phrase complète → `speak`). Politique de retard : `ttsMode: follow` (annule le retard au changement d'orateur, défaut) | `full` (lit tout). Réglages `AppSettings`: `tts_enabled`, `tts_mode`, `tts_volume`. Sans voix disponible pour la langue → repli sur la voix par défaut, ou désactivation avec message.

*Sons (item 24).* `lib/sounds.ts` : Web Audio procédural (gong = sinus + enveloppe, murmure = bruit filtré, applaudissements = rafales de bruit, sifflet = onde carrée), aucun asset ; réglages `sound_enabled`, `sound_volume` ; reprise de l'`AudioContext` au clic « Démarrer » (H-P6).

**Cas limites.** Scène avec 8 GladIAteurs → arc réduit, noms tronqués ; deux réactions simultanées vers la même cible → file ; changement d'onglet pendant un bandeau → bandeau annulé ; TTS : texte contenant des formules (KaTeX) → nettoyage `$…$` avant lecture ; synthèse finale : lue par la voix de l'IArbitre si `tts_enabled` ; relecture d'une discussion sans `report_json` (v1.16) → relecture des messages seuls, scène sans émotions.

### 2.H Pipeline et mesures

*Capacité.* `LlmCapabilities.max_parallel_calls: usize` (Ollama 1, DeepSeek 4, OpenAI-compatible réglable `OPENAI_COMPAT_MAX_PARALLEL_CALLS = 2`, mock paramétrable).

*Fin de tour.* Les entrées (clones) sont préparées, puis `document`, `emotion`, `memory`, `argmap` s'exécutent via `run_bounded` ; les résultats sont appliqués séquentiellement sur `&mut self`. Avec `max_parallel_calls == 1` : appel fusionné `CallKind::TurnAnalyst` (JSON : `summary`, `positions`, `open_questions`, `emotions`, `stagnating`) remplaçant `Memory` + `Emotion` ; `ArgumentMap` et `DocumentUpdate` restent séparés. Parsing tolérant : chaque sous-partie absente laisse l'état précédent.

*Recouvrement.* En cloud, la ronde de réactions du message N s'exécute en parallèle de la phase de **recherche** de l'orateur N+1 (indépendantes) ; l'intention de N+1 attend la fin de la ronde. Optionnel : si la complexité de borrow dépasse un `join!` sur contextes possédés, le recouvrement est abandonné (documenté).

*Mesures.* Spans `tracing` par phase + événement `TurnTimings { turn, phases: [{name, ms}] }` en fin de tour ; `DiscussionDiagnostics { json_parse_failures: {call_kind: n}, refusals, retries, intention_compliance }` émis avant `DiscussionEnded` ; les deux persistés dans `report_json` et affichés dans un panneau « diagnostic » repliable (Résumé). C'est la matière du banc (I1) et la réponse à « pas d'erreurs silencieuses ».

### 2.I Plateforme

*Banc d'évaluation (item 31).* Test `#[ignore] bench_prompts` (variables d'environnement : `AIRENA_BENCH_PROVIDER = ollama|deepseek`, clé/URL) jouant 3 scénarios (débat 3 GladIAteurs 4 tours ; idéation ; socratique) avec émotions, carte et intentions ; métriques calculées à partir des événements : répétition (Jaccard entre interventions successives d'un même orateur), taux d'usage des noms, fuites Markdown (regex), refus, longueur moyenne, conformité d'intention, taux de parsing JSON, durées par phase ; rapport `target/bench/<date>.md` + JSON ; comparaison au dernier rapport (`tools/bench-compare.mjs`). Les métriques pures (`engine/bench_metrics.rs`) ont des tests unitaires.

*Fournisseur OpenAI-compatible (item 32).* Extraction du transport SSE de `deepseek.rs` vers `llm/openai_compat.rs` (types de câblage, parsing SSE, retry/backoff, usage) avec `Dialect { DeepSeek, Generic }` ; `DeepSeekProvider` devient une configuration du transport (tests de transport existants conservés). `ProviderKind::OpenAiCompat` ; réglages `openai_compat_base_url`, `openai_compat_api_key` (vide autorisé pour un serveur local), `openai_compat_model`, `openai_compat_models` (liste manuelle, `/models` tenté puis repli) ; capacités : pas de réflexion, usage si rapporté, `billable = false` (coût `None`, pas de plafond). `needsOllama` inchangé. Anthropic/Gemini natifs : hors périmètre (OpenRouter couvre le besoin via le même dialecte).

*Modèles de discussion (item 33).* Table `discussion_templates (id, name, config_json, builtin, created_at)` ; commandes `list_/save_/delete_discussion_template`, `apply` côté front (fusion dans `useSetupStore` avec résolution des profils par id, profils manquants signalés) ; 4 modèles seedés (procès d'une idée, brainstorming produit, revue d'un document, fiction à trois voix) ; boutons « Charger un modèle » / « Enregistrer comme modèle » dans le pas 1.

*Historique enrichi et exports (item 34).* FTS5 `discussions_fts (topic, synthesis, content)` alimentée à la sauvegarde (spike H-P5, repli `LIKE`) ; `search_discussion_history(query)` ; colonnes `discussions.tags TEXT DEFAULT '[]'`, `favorite INTEGER DEFAULT 0` + commandes `set_discussion_tags`, `set_discussion_favorite` (mises à jour, pas des inserts) ; filtres front par mode/fournisseur/persona/favori. Export HTML autonome (`lib/export-html.ts` : `renderToStaticMarkup` des mêmes composants, CSS inline, carte SVG intégrée, sources) via `downloadTextFile` ; « Imprimer / PDF » via `window.print()` + feuille de style d'impression (spike H-P4). Composant partagé `DiscussionReport` extrait de `SummaryPage`/`HistoryDetailPage` (H24) avant d'ajouter les onglets Sources/Score/Agendas.

*Réglages avancés (item 35).* `Tuning` (`engine/tuning.rs`) : sous-ensemble sûr des constantes (émotions : facteurs et plafonds ; focus : poids ; événements de scène : probabilités ; réactions : plafonds ; coalitions ; décroissance des relations) avec `Default` = constantes ; persisté dans `advanced_tuning_json` ; passé à `DiscussionEngine::new` comme les priorités ; section « Réglages avancés » (repliée) avec curseurs bornés et « réinitialiser ». Les fonctions du moteur reçoivent `&Tuning` là où elles lisaient une constante (les constantes restent les défauts).

*Mémoire longue (item 36).* En fin de discussion, appel `CallKind::Recap` par GladIAteur (JSON : positions défendues, meilleures phrases, alliés/rivaux, ce qu'il retient) → table `persona_memories (id, profile_id, discussion_id FK CASCADE, created_at, recap_json)` ; `GladIAteurConfig.source_profile_id` ajouté côté Rust (H25) ; au démarrage, pour chaque orateur issu d'un profil, les `PERSONA_MEMORY_MAX_RECAPS = 3` souvenirs les plus proches du sujet (BM25 sur `recap_json`) sont injectés dans le système (« [Souvenirs de discussions passées] », ≤ `PERSONA_MEMORY_MAX_CHARS = 900`, comptés dans l'overhead) ; réglage `persona_memory_enabled` ; suppression d'une discussion → souvenirs supprimés (cascade) ; « oublier tout » dans les réglages.

*Exploitation (item 37).* `tauri-plugin-updater` câblé (clé publique, manifeste GitHub Releases, signature au build), activation conditionnée au point de publication ; versions `1.17.0` alignées dans les trois fichiers ; bouton « Exporter le journal » (front : `logger.ts` ; back : fichier de log du jour) dans les réglages ; `TurnTimings`/`DiscussionDiagnostics` (chantier H).

---

## 3. Piliers applicatifs et contraintes transverses

### 3.1 Architecture LLM, tokens, coûts, quotas et limites de débit

**Appels par tour (N GladIAteurs, Ollama, toutes options actives).**

| Phase | v1.16 | Cible (`Immediate`, parallèle 1) | Cible (parallèle ≥ 2) |
|---|---|---|---|
| Réactions | N (une liste par réacteur) | N(N−1) petits appels | idem, par rondes parallèles |
| Décision de recherche | 0–2 par orateur | inchangé | inchangé (recouvrement avec la ronde) |
| Pensée / intention | N (Ollama) · 0 (DeepSeek) | N (`Intention`) | N |
| Intervention | N | N | N |
| Modération | N | N | N |
| Fin de tour | 2–4 (document, émotions, mémoire, carte) | 1–3 (`TurnAnalyst` fusionne mémoire + émotions) | 2–4 en parallèle |
| Ponctuels | — | agendas (N, une fois), verdicts (N, une fois), recaps (N, une fois), casting (1, hors discussion), dépêches de crise (1, une fois) | idem |
| **Total, N = 3** | 14–22 | 16–24 | 17–25 (durée ↓) |

Lecture : le surcoût structurel vient des réactions immédiates (+N(N−2) appels). En cloud il est absorbé par le parallélisme ; en local il est visible (+4–6 s par intervention pour N = 3, hypothèse 2 s par petit appel JSON) et reste désactivable (`deferred`).

**Dimensionnement des prompts.** Blocs ajoutés au prompt d'intervention et leur budget :

| Bloc | Taille max (chars) | Où |
|---|---|---|
| Intention | 300 | overhead déterministe |
| Acte + événement de scène | 250 + 300 | overhead |
| Agenda (système) | 300 | overhead (si activé) |
| Souvenirs (système) | 900 | overhead (si activé) |
| Réactions sous les messages du tour | 3 × 150 par message | déduites du budget par message (`current_turn_msg_chars`) |
| Fils ouverts | 900 | nouvelle section `OpenLoops` |
| Trajectoire de positions | +120 par participant | section positionnelle (plafond 320) |
| Sources (synthèse) | 1 500 | section web/wiki de l'IArbitre |

`BUDGET_DETERMINISTIC_OVERHEAD_CHARS` passe de 2 400 à une valeur **mesurée** par un test qui construit un prompt d'intervention toutes options actives et sections variables vides, et vérifie `len ≤ constante` (ratchet). Avec 8 192 tokens de contexte (défaut), l'overhead reste < 15 % du budget.

**Coût cloud (DeepSeek flash, tarif de pointe).** +≈ 250 tokens de prompt par intervention (≈ 0,000075 $) et +N(N−2) appels de réaction (≈ 1 200 tokens de prompt chacun avec le persona, ≈ 0,0004 $) : pour N = 3, 6 tours, le surcoût (réactions + intentions) est d'environ 0,015 $ par discussion. Le comptage passe intégralement par `MeteredProvider` (nouveaux `CallKind` : `Intention`, `Agenda`, `Casting`, `Recap`, `TurnAnalyst`, `Verdict`) ; `UsageLedger.by_call_kind` et le front (`CallKind` union, `estimateTurnCost` recalibré sur le nouveau nombre d'appels) suivent.

**Quotas et débit.** Tavily : les événements `SurpriseFact` consomment le pool de la discussion et le quota mensuel (règles existantes `can_search_web`), attribués à l'IArbitre ; Wikipédia : gratuit, `WIKI_MAX_LAG_SECS` inchangé ; DeepSeek : concurrence bornée par `max_parallel_calls`, 429 → backoff existant (`DEEPSEEK_MAX_RETRIES`), erreurs fatales latchées ; Ollama : séquentiel. Plafond mensuel : inchangé (le budget est vérifié après chaque orateur et à chaque fin de tour ; les rondes parallèles s'arrêtent au prochain point de contrôle).

**Mode sans clé fournie.** Sans clé DeepSeek : tout fonctionne sur Ollama (intentions, agendas, casting, recaps, banc), avec la fusion « analyste du tour » ; sans clé Tavily : sources = Wikipédia + RAG, `SurpriseFact` se rabat sur wiki/RAG ou `FormatConstraint` ; OpenAI-compatible local sans clé : autorisé ; voix, sons, score, relecture : aucun appel.

### 3.2 Registres et mémoire

**État du moteur ajouté** (cycle de vie) :

| Registre | Portée | Réinitialisation | Persistance |
|---|---|---|---|
| `reactions_by_message`, `audience_reactions` | discussion | jamais (les compteurs par tour sont dérivés) | réactions dans `messages` |
| `open_loops` | discussion | TTL par fil, purge au tour | non (régénérable) |
| `intentions` (dernière par orateur) | tour | par orateur à chaque intervention | `report_json.timeline` (résumé) |
| `current_act`, `scene_history` | discussion | acte à chaque tour | `report_json.timeline` |
| `agendas` | discussion | non | `report_json.agendas` |
| `sources_registry` | discussion | non | `report_json.sources` (via le front) |
| `relationship_scores` (pondérés) | discussion | décroissance par tour | `RelationshipEdge.score` (événement) |
| `coalitions` (tour) | tour | chaque tour | `report_json.timeline` |
| `timings`, `diagnostics` | discussion | non | `report_json` |

**Typage strict.** Rust : enums (`ReactionType`, `MessageKind`, `ActKey`, `SceneEvent`, `ReactionTiming`, `Goal` d'intention) avec `#[serde(rename_all = "camelCase")]` et `#[serde(default)]` sur toute structure lue depuis le LLM ou le front ; TS : unions littérales miroirs dans `types.ts`, `ArenaEvent` étendu, tests de store.

**Intégrité.** `report_json` inséré avec la discussion (même `INSERT … ON CONFLICT DO NOTHING`) ; tags/favoris/templates/persona_memories via `UPDATE`/`INSERT` dédiés ; migrations idempotentes selon le patron `db/schema.rs:124-138` ; `save_user_settings` continue de protéger les compteurs serveur (nouveaux réglages = champs utilisateur, donc écrits) ; le rapport d'une discussion v1.16 est `DiscussionReport::default()`.

**Front.** `useArenaStore` découpé : `stores/arena/{knowledge,emotions,stage,sources,score}.ts` (réducteurs purs `(state, event) → Partial<ArenaState>`), le store ne conservant que l'orchestration et la sauvegarde ; nouveau `useUiStore` (présentation, muet, panneau) ; `report` construit à `discussionEnded` à partir des réducteurs et envoyé dans `SaveDiscussionRequest.reportJson`.

### 3.3 Front-end, ergonomie, responsive, accessibilité

- **Templates existants** : `RightPanel` (onglets Sources et Score ajoutés via `tabs`), `Section`/`Field`/`Explainer`/`ChoiceRow` pour les nouveaux réglages, `SectionLabel`/`Toggle`/`OptionCard` pour les nouvelles options de l'assistant, `StatCard`/`UsageSummaryCard` pour le Résumé, `ToastContainer` pour les alertes. Aucun nouveau primitif d'UI hors `components/stage/`.
- **Mobile-first** : chaque nouvelle vue est conçue à 400 px (scène en bande, bandeaux compacts, onglets défilables, boutons de réaction en ligne sous le message), puis étendue à `lg`. Aucun `min-width` supérieur à l'écran ; SVG en `viewBox`.
- **Accessibilité** : rôles ARIA sur scène et bandeaux, `aria-pressed` sur les boutons de réaction, focus visible, raccourcis annoncés dans un panneau d'aide (`?`), `motion-safe` sur toutes les animations, contraste vérifié en sombre et clair (tokens existants).
- **Charte** : tokens oklch existants ; nouveaux tokens `--arena-*` déclarés dans `:root` et `.dark` ; aucune police externe (hors ligne) ; icônes `lucide-react` uniquement.
- **i18n** : toutes les chaînes nouvelles dans `fr/en/zh.json` (parité bloquante au build) ; estimation +260 clés (modes +60, scène +40, sources +25, score/générique +35, réglages +45, modèles +20, divers +35).

---

## 4. Cartographie des impacts et matrice des risques

### 4.1 Impacts par couche

| Couche | Fichiers touchés (existants) | Nouveaux fichiers | Nature |
|---|---|---|---|
| **Moteur** | `orchestrator.rs` (ronde de réactions, intention, actes, événements, agendas, fin de tour parallèle, sources, didascalies, salle), `prompt_builder.rs` (réactions dans le tour, intention, fils ouverts, trajectoire, agenda, sources de synthèse, prompts des nouveaux modes), `mode_prompts.rs` (scripts d'actes, événements, nouveaux modes), `directive_builder.rs` (acte `Relay`, variantes A, réconciliation), `emotion_engine.rs` (gains, types de réactions, décroissance), `json_parser.rs` (intention, positions tolérantes, `TurnAnalyst`, verdict, casting, recap), `focus.rs` (poids public), `token_budget.rs` (`OpenLoops`, overhead), `memory_manager.rs` (trajectoire) | `engine/dramaturgy.rs`, `engine/stage_directions.rs`, `engine/tuning.rs`, `engine/bench_metrics.rs`, `llm/parallel.rs`, `llm/routing.rs`, `llm/openai_compat.rs` | Extension ; le chemin `deferred` + gains neutres + événements désactivés reproduit v1.16 (garantie testée) |
| **Fournisseurs** | `llm/mod.rs` (capacité `max_parallel_calls`, `model_for`, `capabilities_for`), `metered.rs` (tarif par modèle, `by_model`), `deepseek.rs` (dialecte), `factory.rs` (routage, OpenAI-compatible), `mock.rs` (parallélisme scripté) | `openai_compat.rs`, `routing.rs` | Trait étendu par méthodes à défaut : implémentations existantes inchangées |
| **Modèles / événements** | `events.rs` (+10 variants, 3 étendus), `message.rs` (`kind`, `quote`, types), `discussion.rs` (`features`, modes, `mode_role`, `model`), `gladiateur.rs` (`source_profile_id`, `model`), `settings.rs` (+12 champs), `llm.rs` (`CallKind` +6, `ProviderKind` +1, `by_model`), `memory.rs`, `relationship.rs`, `history.rs` (`report_json`, tags, favori) | `models/report.rs`, `models/template.rs`, `models/persona_memory.rs` | Tout nouveau champ `#[serde(default)]` |
| **Base** | `schema.rs` (colonnes `report_json`, `kind`, `tags`, `favorite` ; tables `discussion_templates`, `persona_memories` ; FTS5), `repository.rs` (lecture/écriture, recherche, templates, souvenirs), `seed.rs` (templates) | — | Migrations idempotentes, base v1.16 lisible |
| **Commandes** | `discussion.rs` (réaction du public, vote, routage multi-modèle), `settings.rs`, `llm.rs` (OpenAI-compatible), `history.rs` (recherche, tags, favori), `lib.rs` (enregistrement) | `commands/templates.rs`, `commands/casting.rs`, `commands/memories.rs` | ~14 commandes nouvelles |
| **Front — état** | `useArenaStore.ts` (découpage), `useSetupStore.ts` (features, rôles, modèles, templates), `useSettingsStore.ts` (nouveaux réglages, fournisseur), `types.ts`, `tauri-api.ts` | `stores/arena/*.ts`, `useUiStore.ts`, `lib/{score,speech,sounds,export-html,timeline,replay}.ts` | Réducteurs purs testés |
| **Front — UI** | `ArenaPage.tsx` (scène, onglets, bandeaux, raccourcis, projection), `MessageBubble.tsx` (réactions, sources, citations, kinds), `RightPanel.tsx` (inchangé, nouveaux onglets), `SummaryPage.tsx`/`HistoryDetailPage.tsx` (→ `DiscussionReport` partagé), `HistoryPage.tsx` (recherche, filtres, tags), `StepTopic.tsx` (modes, mise en scène, templates), `StepGladiateurs.tsx` (casting, rôles, modèle), `LlmParamsForm.tsx` (modèle), `SettingsPage.tsx` (+3 sections), `TopBar.tsx` (voix, sons, projection), `AppShell.tsx` (projection) | `components/stage/*`, `components/sources/*`, `components/score/*`, `components/replay/*`, `components/settings/{AudioSettings,AdvancedTuning,OpenAiCompatSettings,MemorySettings}.tsx`, `components/setup/{CastingAssistant,TemplatePicker}.tsx` | Pages maintenues < 400 lignes |
| **i18n** | `fr/en/zh.json` | — | +≈ 260 clés, parité bloquante |
| **Docs** | `CLAUDE.md`, `TECHNICAL.md`, `FUNCTIONAL.md`, `README.md`, ce document (§8 bilan) | — | Par lot |

### 4.2 Matrice des risques

| # | Risque | Prob. | Impact | Mitigation concrète |
|---|---|---|---|---|
| R1 | Les réactions immédiates ralentissent trop les discussions locales | Moyenne | Moyen | Réglage `deferred` ; petit prompt de réaction (un message, ≤ 800 chars) ; mesure `TurnTimings` ; seuil d'alerte dans l'assistant (« +N s par intervention estimés ») |
| R2 | Le JSON d'intention casse sur les petits modèles | Moyenne | Faible | Repli automatique sur la pensée brute ; `parse_json_response` tolérant ; taux de parsing dans `DiscussionDiagnostics` ; le banc mesure avant/après |
| R3 | Prompt d'intervention trop long → troncatures silencieuses du contexte utile | Moyenne | Élevé | Overhead re-mesuré par test-ratchet ; sections budgétées (`OpenLoops`) ; aperçu du budget dans l'assistant recalculé avec les options actives |
| R4 | Les événements de scène rendent la discussion incohérente (surprises trop fréquentes) | Faible | Moyen | Probabilité basse, écart minimal de 2 tours, jamais au premier ni au dernier tour, désactivable, annonce explicite de l'IArbitre |
| R5 | Régression émotionnelle (gains OCEAN, nouveaux types) modifie le comportement des discussions existantes | Faible | Élevé | Gains neutres pour OCEAN moyen/absent ; tests golden sur `update_emotions` ; simulation « v1.16 équivalent » (`deferred`, sans événements) doit produire les mêmes séquences que les tests actuels |
| R6 | Appels parallèles cloud → 429 en rafale | Faible | Faible | Concurrence bornée (4), backoff existant, aucune reprise après émission de tokens (règle existante) |
| R7 | Multi-modèle Ollama : swaps de modèles à chaque orateur (VRAM) | Moyenne | Moyen | Avertissement explicite dans l'assistant ; recommandation « même famille » ; mesure des timings ; option laissée au choix |
| R8 | Extraction du transport OpenAI-compatible casse DeepSeek | Faible | Élevé | Tests de transport existants (`transport_tests`) conservés tels quels et exécutés sur le dialecte DeepSeek ; golden des corps de requête |
| R9 | Web Speech / Web Audio indisponibles ou silencieux dans WebView2 | Faible | Faible | Spike H-P1/H-P6 ; détection à l'exécution ; fonctions masquées si absentes |
| R10 | `report_json` volumineux (sources + historique émotionnel + timeline) | Faible | Faible | Extraits tronqués (`SOURCE_SNIPPET_CHARS`), historique émotionnel déjà plafonné à 30 instantanés, timeline ≤ 500 entrées (`TIMELINE_MAX_ENTRIES`) |
| R11 | Découpage du store arène introduit une régression d'événement | Moyenne | Moyen | Tests existants (`useArenaStore.test.ts`) comme filet ; un test par réducteur ; exécution du scénario complet d'événements enregistré (fixture JSON) |
| R12 | Sécurité : ouverture d'URL arbitraires (sources) | Faible | Moyen | Seuls `http(s)` ; ouverture via le plugin opener (scope) ; jamais `file:` ; les URL viennent des API (Tavily/Wikipédia), pas du texte LLM |
| R13 | Nouveaux réglages avancés mal bornés | Faible | Moyen | Bornes min/max par champ dans `Tuning::validate()`, réinitialisation, valeurs par défaut = constantes |
| R14 | Charge i18n (+260 clés) | Certaine | Faible | `tools/i18n-add.mjs` par lot ; parité bloquante ; revue des zh/en par lot |
| R15 | FTS5 absent du SQLite embarqué | Faible | Faible | Spike H-P5 ; repli `LIKE` |
| R16 | Dérive de taille de fichiers (orchestrator > 4 500 l.) | Certaine | Moyen | Extraction de modules (`dramaturgy`, `stage_directions`, `reactions.rs` pour la ronde, `sources.rs`) ; ratchet « orchestrator ≤ 4 000 lignes » |

### 4.3 Garanties de non-régression

- Configuration « v1.16 » = `reaction_timing = Deferred`, `features.scene_events = false`, `hidden_agenda = false`, `coalitions = false`, `audience_reactions = false`, OCEAN neutre, réglages audio désactivés : les tests `engine_tests.rs` existants passent **sans modification de leurs assertions** ; seuls le constructeur `config()` du harnais (champ `features` fixé au profil « v1.16 », une ligne) et `default_script` (nouveaux `CallKind`) évoluent.
- Historique v1.16 : lecture d'une base sans `report_json`/`kind`/`tags` → valeurs par défaut, pages inchangées (onglets nouveaux masqués).
- Événements existants : aucun renommage, uniquement des champs ajoutés avec défaut ; le store ignore les types inconnus (déjà le cas via `switch` sans `default` fatal).
- Réglages : `save_user_settings` continue de protéger les compteurs serveur ; les nouveaux réglages ont des défauts serde.
- Ratchets : tests Rust ≥ 376 → cible ≥ 520 ; front ≥ 40 → cible ≥ 110 ; clippy 0 ; parité i18n ; `pages/*` ≤ 400 lignes ; nouveau : composants `components/**` ≤ 400 lignes (hors `PersonaEditor`), `orchestrator.rs` ≤ 4 000 lignes, test de plafond de l'overhead déterministe.

---

## 5. Plan de test directeur (socle TDD)

### 5.1 Matrice de couverture

| Domaine | Tests unitaires (Rust) | Tests unitaires (vitest, node) | Intégration | E2E / simulation (`engine_tests.rs` sur mock) |
|---|---|---|---|---|
| Réactions (A) | table d'effets par type, propension OCEAN, validation de citation, parsing des nouveaux types et de `quote`, classification avec neutres | chips typées (`reactionEmoji`), réducteur de réactions du public (optimiste), garde `status` | commande `react_to_message` → événement | S1–S7 |
| Réflexions (B) | `parse_intention` (complet, partiel, invalide), TTL des fils ouverts, positions chaîne/objet, `reasoning_pace` sur les allowances | chronomètre pur (`elapsedLabel`), coulisses (réducteur intentions) | — | S8–S12 |
| Dramaturgie (C) | `resolve_act` (avec/sans `max_turns`, stagnation, arrêt doux), `pick_scene_event` (préconditions, écart, probabilité seedée), ordre Duel/HotSeat/Relais, prompts des nouveaux modes non vides ×3 langues, verdict/vote | timeline (réducteur), bandeaux (file) | — | S13–S20 |
| Profils (D) | `gains_from_ocean` (neutre, extrêmes, bornes), `RoutingProvider` (dispatch, `model_for`, capacités), factory multi-modèle (validation par modèle), parsing casting/agenda/recap | matrice de compatibilité pure, sélecteur de modèle (store) | `suggest_casting` avec mock | S21–S26 |
| Émotions (E) | modulation d'échantillonnage (bornes, JSON exclu), didascalies (dynamics/gabarit, limite par tour), décroissance et transitions de relation, label de salle | aura/dominante (`dominantEmotion`), jauge de salle | — | S27–S31 |
| Sources (F) | `SourceRecord` depuis Tavily/Wiki/RAG, encodage d'URL, bloc de synthèse borné | réducteur sources (attachement au message, `cited` heuristique, regroupement), export Markdown | sauvegarde/lecture `report_json` | S32–S34 |
| Scène et UI (G) | — | `score.ts` (points, classement, récompenses, égalités), `splitSentences` (fr/en/zh, abréviations, KaTeX), `sounds` (planification pure), `replay` (planification par deltas et facteur), raccourcis (mapping, champs ignorés), `export-html` (chaîne autonome) | — | Liste de contrôle manuelle §5.3 |
| Pipeline (H) | `run_bounded` (ordre d'application, limite, annulation), parsing `TurnAnalyst` partiel | — | — | S35–S38 |
| Plateforme (I) | métriques du banc (répétition, fuites, conformité), transport OpenAI-compatible (dialectes, clé vide), templates (repo), FTS (création, recherche, repli), souvenirs (BM25, cascade), `Tuning::validate` | filtre/recherche d'historique (store), application de template (fusion, profils manquants) | migrations sur base v1.16 (fixture), FTS5 réel | S39–S45 + banc réel (ignoré) |

### 5.2 Simulations et cas limites à éprouver (avant et pendant le développement)

Toutes sur `MockLlmProvider` sauf mention « réel ». Chaque simulation fixe une attente vérifiable (événements, appels, état).

1. **S1** Ronde immédiate : 3 GladIAteurs, 2 tours → chaque intervention est suivie de 2 appels `Reaction` (parallèle 1 : séquentiels ; parallèle 4 : `recorded_calls` montre les deux requêtes avant tout autre appel), les `ReactionEmitted` précèdent le `SpeakerActive` suivant.
2. **S2** `deferred` reproduit exactement la séquence v1.16 (assertion sur la liste des `CallKind`).
3. **S3** Un seul actif (les autres bannis) → aucune ronde ; sécheresse de réactions non signalée.
4. **S4** Types : `insightful` sur A → confiance de A +bonus ; `question` → fil ouvert pour A ; `offTopic` → frustration atténuée ; `laugh` → enthousiasme des deux ; classification ignore les neutres.
5. **S5** Citation exacte retrouvée / non retrouvée → `quote` présent / `None`.
6. **S6** Public : 4 réactions sur un même message → 3 comptées ; réaction après `DiscussionEnded` → erreur de commande ; réaction pendant une pause → appliquée à la reprise.
7. **S7** Réactions au tour 1 n'empêchent pas la consigne « tour d'ouverture » ; le prompt du 2ᵉ orateur contient la ligne 👍/👎 sous le 1ᵉʳ message.
8. **S8** Intention valide → prompt d'intervention contient « [Ton intention] » avec la cible ; `ThoughtComplete` porte `pensee` ; `IntentionGenerated` émis.
9. **S9** Intention invalide (texte libre) → repli : pensée = texte, pas d'intention, aucune erreur.
10. **S10** Fils ouverts : question de A à B → présente dans le prompt de B au tour suivant ; disparue après TTL ; suspendue pendant un ban.
11. **S11** Positions : réponse mémoire mixte (chaîne + objet) → carte cohérente ; prompt d'intervention affiche « a évolué depuis … ».
12. **S12** `reasoning_pace = Fast` → `max_tokens` DeepSeek réduit (golden de corps de requête), `Auto` plafonné à `Low`.
13. **S13** Actes avec `max_turns = 5` → séquence `ActStarted` ouverture/confrontation/contre-interrogatoire/concessions/plaidoiries ; annonces IArbitre présentes dans les prompts (bloc modérateur).
14. **S14** Sans `max_turns` + arrêt doux au tour 3 → le tour 3 reçoit l'acte « plaidoiries ».
15. **S15** Stagnation détectée → acte « concessions » anticipé ; probabilité d'événement de scène relevée (rng seedé).
16. **S16** Duel avec 4 actifs → `TurnStarted.speakerOrder` de longueur 2 ; les deux autres reçoivent quand même les réactions (rien à réagir) et la fin de tour s'exécute.
17. **S17** `SurpriseFact` sans recherche disponible → remplacé par `FormatConstraint` ; avec pool web = 1 → consommé et attribué à l'IArbitre.
18. **S18** Procès : rôles présents dans les prompts ; verdict = N appels `Verdict` + synthèse contenant le verdict ; sans juré → verdict IArbitre seul.
19. **S19** Oxford : votes du public avant/après → `report_json.outcome.swing`.
20. **S20** Coalition : arête `ally` établie → acte `Relay` puis l'allié parle juste après ; `CoalitionFormed` émis ; jamais avec 2 actifs.
21. **S21** Agenda : N appels `Agenda` après l'introduction ; texte présent dans le système d'intervention ; synthèse reçoit les agendas ; `AgendaRevealed` après `SynthesisComplete` ; échec d'un appel → orateur sans agenda.
22. **S22** Gains OCEAN : même événement, N = 9 vs N = 3 → deltas de frustration différents ; OCEAN absent → deltas v1.16 exacts (golden).
23. **S23** Routage : deux modèles distincts → `by_model` du ledger contient les deux ; coût par modèle (mock facturable) ; capacités par orateur (l'un raisonne, l'autre non).
24. **S24** Modèle d'orateur invalide → `start_discussion` refuse avant le spawn (test de commande avec factory mockée).
25. **S25** Casting : catalogue de 5 profils → suggestion filtrée sur ids existants ; id inconnu ignoré.
26. **S26** Souvenirs : 2 discussions passées → au démarrage suivant, le prompt système contient les 2 souvenirs classés par BM25 ; suppression de la discussion → souvenir supprimé.
27. **S27** Échantillonnage : enthousiasme 90 → température +0,12 ; engagement 20 → `num_predict` réduit ; appels JSON inchangés ; `emotion_driven = false` → paramètres d'origine.
28. **S28** Didascalies : seuil franchi → message `stageDirection` émis, **absent** des prompts suivants ; deux seuils dans le tour → une seule didascalie.
29. **S29** Décroissance : rivalité (2/2) sans nouvelle réaction pendant 4 tours → `kind = null` ; une réaction positive → `RelationshipShift`.
30. **S30** Salle : moyenne de frustration > 65 → `RoomMoodUpdated.label = tense` et prompt de modération contient l'indice.
31. **S31** Contagion et salle avec un seul actif → aucun événement de salle.
32. **S32** Sources : recherche web (2 résultats) + wiki (1 article) + RAG (2 chunks) pour un orateur → 5 `SourceRecord` attachés à son `messageId` ; `cited` vrai si le domaine apparaît dans le message.
33. **S33** Introduction avec recherche → sources attachées au message d'introduction.
34. **S34** Synthèse : bloc « Sources utilisées » présent et borné ; sans source → bloc absent.
35. **S35** Fin de tour parallèle (mock parallèle 4) → les quatre appels démarrent avant la première application ; état final identique au séquentiel (golden).
36. **S36** Parallèle 1 → un seul appel `TurnAnalyst` remplace `Memory` + `Emotion` ; réponse partielle (pas d'émotions) → mémoire mise à jour, émotions inchangées.
37. **S37** Annulation pendant la fin de tour parallèle → `DiscussionEnded` unique, usage enregistré.
38. **S38** `TurnTimings` et `DiscussionDiagnostics` émis ; compteur de parsing incrémenté sur JSON invalide.
39. **S39** Transport OpenAI-compatible : dialecte générique n'envoie ni `thinking` ni `reasoning_effort` ; clé vide acceptée ; 401 → fatal.
40. **S40** Templates : sauvegarde/liste/suppression ; application avec un profil manquant → toast et profil ignoré.
41. **S41** FTS : recherche « automatisation » retrouve la discussion par sa synthèse ; sans FTS5 (feature simulée) → repli `LIKE` équivalent.
42. **S42** Tags/favori : mise à jour puis relecture ; suppression de la discussion → cascade.
43. **S43** `Tuning` hors bornes → `validate()` corrige et journalise ; `Tuning::default() == constantes`.
44. **S44** Banc (métriques pures) : répétition = 1,0 pour deux textes identiques, fuite Markdown détectée sur `**gras**`, conformité d'intention = 0 si la cible n'apparaît pas.
45. **S45** Simulation complète « toutes options » (3 GladIAteurs, 5 tours, `immediate`, actes, événements seedés, agendas, coalitions, sources, multi-modèle) : aucun `error`, `report_json` complet, séquence d'événements stable (golden) ; et sa jumelle « v1.16 équivalent ».
46. **S46 (réel, ignoré)** Banc sur Ollama et sur DeepSeek : rapport généré, comparaison au rapport précédent, aucune fuite Markdown, refus = 0.
47. **S47 (front)** Fixture d'événements enregistrée (S45) rejouée dans les réducteurs → état final attendu (sources, score, timeline, scène).
48. **S48 (front)** Relecture : 12 messages avec timestamps → planification ×4 respecte les deltas et les bornes (`REPLAY_MAX_GAP_MS`).

### 5.3 Listes de contrôle manuelles (non automatisables)

- Voix : lecture fr/en/zh, changement d'orateur en mode `follow`, pause/reprise, formules KaTeX nettoyées, muet.
- Sons : gong au tour, applaudissements sur réaction du public, volume, désactivation, premier son après le clic « Démarrer ».
- Scène : 2, 4, 8 GladIAteurs ; ban ; passage ; coalition ; aura ; largeur 400 px ; thème sombre/clair ; `prefers-reduced-motion`.
- Raccourcis : aucun déclenchement dans un champ de saisie ; aide `?`.
- Projection : plein écran, sortie par Échap, panneau masqué.
- Sources : ouverture dans le navigateur système (H-P2), copie, filtre « cités ».
- Export HTML : ouverture hors ligne dans un navigateur, carte SVG visible ; impression PDF (H-P4).

### 5.4 Fixtures à créer

- `engine_tests.rs` : `default_script` étendu (`Intention`, `Agenda`, `Recap`, `TurnAnalyst`, `Verdict`, `Casting`), scripts de réactions typées, fixture d'événements S45 exportée en JSON (`src-tauri/fixtures/events-full.json`) consommée par les tests front.
- `llm/openai_compat.rs` : captures SSE génériques (`fixtures/openai-compat-*.sse`).
- `db` : base v1.16 minimale (`fixtures/airena-v1.16.db` générée par un test) pour les migrations.
- Front : `src/test/fixtures/events-full.json` (copie du précédent via script `tools/sync-fixtures.mjs`).

---

## 6. Plan d'actions consolidé et séquencé

Chaque lot est atomique, livrable seul, commence par ses tests (TDD) et se termine par : `cargo test --lib`, `cargo clippy --all-targets` (0), `npm test`, `npm run typecheck`, `npm run build` (parité i18n), mise à jour des docs, revue adversariale à froid, bilan dans §8 de ce document. Aucun `git add`/commit sans demande explicite. Estimations en jours-homme.

| Lot | Version | Objectif | Contenu (tests → implémentation) | Dépend de | Critère de done |
|---|---|---|---|---|---|
| **0 — Spikes (0,5 j)** | — | Lever H-P1…H-P6 | Page de test jetable : `speechSynthesis.getVoices()`, `openUrl`, `AudioContext`, `window.print()` ; test Rust FTS5 ; 2 requêtes Ollama concurrentes chronométrées. Rien de conservé hors constantes/capacités | — | Tableau §0.4 renseigné (vrai/faux + plan B choisi) |
| **1 — Socle données, événements, store (2 j)** | 1.17 | Fondations communes | `DiscussionReport` + colonne `report_json`, `Message.kind` + colonne, `ReactionType` étendu + `quote`, `DiscussionFeatures`, `CallKind` +6, `LlmCapabilities.max_parallel_calls`, `llm/parallel.rs`, `Tuning` (squelette, défauts = constantes), découpage de `useArenaStore` en réducteurs, `DiscussionReport` partagé (Résumé/Historique), fixture d'événements | 0 | S45 « v1.16 équivalent » vert ; base v1.16 migrée ; 40 tests front passent inchangés |
| **2 — Banc d'évaluation (1 j)** | 1.17 | Mesurer avant de changer les prompts | `bench_metrics.rs` + tests ; test ignoré `bench_prompts` ; `tools/bench-compare.mjs` ; **rapport de référence v1.16** archivé dans `Docs/Technique/bench/` | 1 | S44, S46 ; rapport de référence produit sur Ollama (et DeepSeek si clé) |
| **3 — Références et sources (1,5 j)** | 1.17 | Demande complémentaire | Événements enrichis, registre moteur, bloc de synthèse, réducteur sources, onglet Sources, popover sur les badges, `openUrl`, export Markdown, persistance | 1 | S32–S34 ; liens ouverts dans le navigateur ; historique v1.16 sans onglet |
| **4 — Réactions vivantes (2 j)** | 1.17 | Chantier A | Ronde immédiate, types, citation, propension, réactions du public (commande + UI), réactions dans le prompt, indices pour la carte | 1, 2 | S1–S7 ; banc : taux de réactions parsées ≥ référence |
| **5 — Intention, fils ouverts, trajectoire (2 j)** | 1.17 | Chantier B | `CallKind::Intention`, `IntentionGenerated`, fils ouverts + section budget, trajectoire, `reasoning_pace`, chronomètre | 1, 2, 4 | S8–S12 ; banc : conformité d'intention mesurée, répétition ≤ référence |
| **6 — Émotions incarnées (1,5 j)** | 1.17 | Chantiers D14, E | Gains OCEAN, échantillonnage, didascalies, décroissance/réconciliation, salle | 1 | S22, S27–S31 ; golden v1.16 sur OCEAN neutre |
| **7 — Pipeline et mesures (1,5 j)** | 1.17 | Chantier H | Fin de tour parallèle, `TurnAnalyst`, recouvrement (cloud), `TurnTimings`, `DiscussionDiagnostics`, panneau diagnostic | 1, 4 | S35–S38 ; temps mort entre orateurs mesuré ↓ (rapport) |
| **8 — Dramaturgie (2,5 j)** | 1.18 | Chantier C (actes, événements, coalitions) | `dramaturgy.rs`, scripts par mode, annonces, événements de scène, `Relay`, ordre Duel/HotSeat, options de mise en scène dans l'assistant | 5, 6 | S13–S17, S20 |
| **9 — Scène et spectacle (3 j)** | 1.18 | Chantier G (hors voix/sons/score) | `ArenaStage`, `ReactionBurst`, bandeaux, coulisses en direct, thème arène, mode projection, raccourcis, timeline | 4, 5, 8 | Liste §5.3 (scène, raccourcis, projection) ; responsive 400 px |
| **10 — Voix et sons (1,5 j)** | 1.18 | Chantier G (23, 24) | `speech.ts`, `sounds.ts`, réglages audio, contrôles `TopBar`, lecture de la synthèse | 0, 9 | Liste §5.3 (voix, sons) ; `splitSentences` testé |
| **11 — Score, récompenses, relecture (2 j)** | 1.18 | Chantier G (26, 27) | `score.ts`, onglet Score, générique, `ReplayPlayer`, historique émotionnel persisté | 9 | S47, S48 |
| **12 — Agenda caché et casting (1,5 j)** | 1.19 | Chantier D (13, 15) | `Agenda`, révélation, `suggest_casting`, matrice de compatibilité | 5 | S21, S25 |
| **13 — Nouveaux modes I (2 j)** | 1.19 | Procès, Oxford | Variants, prompts ×3 langues, rôles, verdict/vote, cartes de mode | 8, 12 | S18, S19 |
| **14 — Nouveaux modes II (2 j)** | 1.19 | Négociation, Six chapeaux, Cellule de crise | Rotation de rôles, dépêches, accord | 13 | Prompts non vides ×3 langues ; simulation par mode sans erreur |
| **15 — Multi-modèle et OpenAI-compatible (2,5 j)** | 1.20 | Chantiers D16, I32 | Trait étendu, `RoutingProvider`, factory, UI modèle, `openai_compat.rs` + dialecte DeepSeek, réglages | 1 | S23, S24, S39 ; tests de transport DeepSeek inchangés |
| **16 — Modèles, historique enrichi, exports (2 j)** | 1.20 | Chantiers I33, I34 | Templates, FTS/repli, tags/favori, filtres, export HTML, impression | 0, 3 | S40–S42 ; export ouvert hors ligne |
| **17 — Réglages avancés et mémoire longue (2 j)** | 1.20 | Chantiers I35, I36 | Section réglages avancés, `Tuning` branché, `Recap`, `persona_memories`, injection, oubli | 1, 6 | S26, S43 |
| **18 — Exploitation, docs, clôture (1 j)** | 1.20 | Chantier I37 | Updater câblé (désactivé sans point de publication), versions alignées, export du journal, `CLAUDE.md`/`TECHNICAL.md`/`FUNCTIONAL.md`/`README.md`, ratchets, auto-évaluation finale (§8) | tous | Zéro avertissement ; docs cohérentes ; banc final comparé à la référence |

**Ordre recommandé** : 0 → 1 → 2 → 3 → 4 → 5 → 6 → 7 (livraison 1.17 « pertinence et sources ») → 8 → 9 → 10 → 11 (1.18 « spectacle ») → 12 → 13 → 14 (1.19 « dramaturgie avancée ») → 15 → 16 → 17 → 18 (1.20 « plateforme »). Total ≈ 34 j-h. Les lots 15 et 16 ne dépendent que du socle et peuvent être avancés si l'ouverture à d'autres modèles est prioritaire.

**Ratchets à faire évoluer** : tests Rust 376 → ≥ 520 ; front 40 → ≥ 110 ; parité i18n (≈ 1 180 clés) ; clippy 0 ; `pages/*` ≤ 400 l. ; `components/**` ≤ 400 l. ; `orchestrator.rs` ≤ 4 000 l. ; overhead déterministe ≤ constante (test) ; banc : fuites Markdown = 0, refus = 0, répétition ≤ référence v1.16, conformité d'intention ≥ 0,8 (Ollama de référence) — valeurs à fixer au Lot 2.

---

## 7. Grille d'auto-évaluation de l'analyse

| Critère | Réponse | Justification |
|---|---|---|
| Analyse complète, robuste, viable, sans zone de flou bloquante ? | **Oui, avec six hypothèses de plateforme isolées** | Les 38 items sont couverts (§1.1 → §2), chacun avec une conception, des données, un front et des cas limites. Les seules inconnues (H-P1…H-P6) concernent l'environnement d'exécution, pas la base de code ; chacune a un plan B et ne change qu'un composant optionnel (voix, sons, impression, FTS, ouverture de lien, parallélisme Ollama). Le Lot 0 les lève en une demi-journée |
| Toutes les hypothèses confrontées au code réel ? | **Oui** | 26 hypothèses tracées avec fichier:ligne (§1.2), 6 faux positifs et 9 faux négatifs traités (§1.3). Aucune affirmation sur un fichier non lu (§0.2 liste les survols et leur absence d'impact) |
| Contraintes tokens/coûts LLM, registres et responsive intégralement cadrées ? | **Oui** | §3.1 : tableau d'appels avant/après, dimensionnement de chaque bloc de prompt avec sa section budgétaire, coût cloud chiffré, quotas et débit, mode sans clé. §3.2 : registres, cycles de vie, persistance unique `report_json`, typage. §3.3 : templates, mobile-first 400 px, accessibilité, charte, i18n |
| Plan d'actions et plan de tests suffisamment précis pour démarrer immédiatement ? | **Oui** | 19 lots atomiques (0 à 18) avec dépendances, versions cibles et critères de done ; 48 simulations numérotées avec attentes vérifiables ; fixtures nommées ; listes de contrôle manuelles ; ratchets chiffrés. Le Lot 0 puis le Lot 1 démarrent sans décision métier préalable |
| Conformité aux patterns de la base | **Oui** | Constantes centralisées ; `#[serde(default)]` sur tout nouveau champ ; événements camelCase par variante ; migrations idempotentes ; `std::sync::Mutex` inchangé ; erreurs `thiserror`/`CommandError` ; streaming tamponné ; quotas selon le patron Tavily ; réglages serveur protégés ; trait fournisseur étendu par défauts ; réducteurs purs testables en node ; pages < 400 lignes ; parité i18n bloquante |
| Réserves de faisabilité | **Aucune bloquante** | Les points les plus lourds (multi-modèle, extraction du transport OpenAI-compatible, découpage du store) sont sécurisés par les golden tests et les tests de transport existants ; le coût des réactions immédiates en local est mesuré, affiché et réversible |

---

## 8. Journal d'exécution (mis à jour lot par lot)

Chaque entrée : ce qui a été livré, les écarts par rapport au plan (§2/§6) et leur raison, les mesures de contrôle. Les ratchets sont ceux mesurés à la fin du lot (`cargo test --lib`, `cargo clippy --all-targets`, `npm test`, `npm run typecheck`, `node tools/i18n-check.mjs`).

### Lot 0 — Spikes

Livré : hypothèses H-P2, H-P3 et H-P5 vérifiées (voir §0.4, colonne « Résultat ») ; H-P1, H-P4, H-P6 dépendent de WebView2 à l'exécution et sont traitées par leur plan B (détection à l'exécution). Rien de conservé hors constantes.

### Lot 1 — Socle données, événements, store

Livré : `DiscussionReport` v1 (`report_json`), `Message.kind` (`discussion_messages.kind`, dérivé du drapeau de bannissement sur les anciennes lignes), `ReactionType` à six couleurs + `quote`, `DiscussionFeatures` (`legacy()` réservé aux tests), `CallKind` +6, `LlmCapabilities.max_parallel_calls`, `llm/parallel.rs::run_bounded`, découpage de `useArenaStore` en réducteurs purs, `DiscussionReportView` partagé (Résumé / Historique), fixture `src-tauri/fixtures/events-full.json` rejouée par `fixture.test.ts`.
Écarts : le squelette `Tuning` est reporté au premier consommateur (Lot 6) — un module sans consommateur n'aurait été que du code mort.

### Lot 2 — Banc d'évaluation

Livré : `engine/bench_metrics.rs` (répétition, usage des noms, fuites Markdown, refus, longueur, conformité d'intention, temps mort), test ignoré `bench_prompts` piloté par variables d'environnement, `tools/bench-compare.mjs`, rapport de référence v1.16 archivé dans `Docs/Technique/bench/2026-09-16-v1.16-ollama-mistral-small3.1.{md,json}`.

### Lot 3 — Références et sources

Livré : `WebSearchPerformed.results` / `WikiSearchPerformed.articles`, registre moteur `sources_registry` → bloc « [Sources utilisées] » et consigne « ## Sources » dans la synthèse, réducteur `sources`, onglet Sources (arène et rapport), popover sur les badges, `openExternalUrl` (http/https uniquement), export Markdown, persistance dans `report_json.sources`.
Correction connexe : le moteur n'abandonnait pas le RAG en repli lexical quand les embeddings différés échouaient (faux négatif corrigé, `ensure_embeddings` n'est tenté qu'une fois).

### Lot 4 — Réactions vivantes

Livré : ronde immédiate après chaque intervention (`run_bounded` borné par `max_parallel_calls`), réactions typées et citées (`validated_quote`), propension OCEAN (`engine/reactions.rs`), réactions du public (commande `react_to_message`, plafond `AUDIENCE_REACTIONS_PER_MESSAGE_MAX`, optimisme côté store), réactions visibles dans le prompt du tour, indices pour la carte des arguments. Simulations S1–S7.

### Lot 5 — Intention, fils ouverts, trajectoire, rythme

Livré :
- `CallKind::Intention` (JSON, `Off`) remplace l'appel `Thought` : `models/intention.rs` (`Intention`, `IntentionGoal` trilingue), `json_parser::parse_intention` (alias FR/ZH des clés, cible résolue par `match_speaker_name`, mot « sujet » → aucune cible, champs bornés), `prompt_builder::build_intention_prompt` (participants adressables, priorité du focus, fils ouverts numérotés, schéma JSON), bloc « [Ton intention] » ≤ `INTENTION_BLOCK_MAX_CHARS` dans le prompt d'intervention, événement `IntentionGenerated`, repli prose → pensée persona (v1.16), JSON cassé → ni pensée ni intention, jamais d'erreur. Avec le raisonnement natif actif (DeepSeek), l'appel Intention précède l'intervention et le raisonnement garde la priorité d'affichage.
- Fils ouverts : `engine/open_loops.rs` (`OpenLoopRegistry`, FIFO ≤ 3, TTL = 2 interventions **propres** — un banni ne parle pas, ses fils attendent), alimentés par la question de l'intention, la concession (engagement), les réactions `Question` justifiées et les `open_questions` de l'analyste mémoire ; section budgétaire `BudgetSection::OpenLoops` (configurable, rang 11, documents 12–13).
- Trajectoire : `ParticipantPosition { stance, initial_stance, shift, would_change_if }`, réponse mémoire tolérante (chaîne | objet, mixte accepté), noms normalisés sur les participants connus (un nom inventé n'entre plus dans la carte), événement `PositionsUpdated`, bloc « [Évolution des positions] » + consigne de section dans la synthèse, onglet Positions du rapport (`PositionsTable`), persistance `report_json.positions`.
- Rythme : `ReasoningPace { Normal, Fast }` (`AppSettings.reasoning_pace`, clé `reasoning_pace`), `LlmRequest.pace`, DeepSeek `DEEPSEEK_FAST_PACE_ALLOWANCE_FACTOR = 0.5` sur les réserves, `Auto` plafonné à `Low` (`ReasoningPace::cap_auto`), réglage dans la section Fournisseur, chronomètre « réfléchit depuis N s » dans la bulle de raisonnement.
- Simulations S8–S12 (+ S10 bis bannissement, S11 bis noms inconnus / fiction) ; tests unitaires des fils, du parseur, de la mémoire, du budget, des prompts, du fournisseur ; fixture régénérée (997 événements) ; tests front (réducteurs, rapport, migration des priorités).

Écarts assumés :
- `BUDGET_FLOOR_OPEN_LOOPS = 300` (le plan disait 0) : la cascade ne distribue le surplus qu'en ordre de rang, un plancher nul aurait masqué les fils sous ≈ 16k de contexte ; 300 caractères (≈ 80 jetons) affichent un ou deux fils.
- Plancher de la carte des positions rendu **par participant** (`BUDGET_FLOOR_POSITIONAL_MAP` 50 × N) : le plancher plat de 50 caractères donnait 16 caractères par participant à 8k, la trajectoire aurait été illisible.
- `BUDGET_DETERMINISTIC_OVERHEAD_CHARS` = 2 400 + 300 : le bloc d'intention est un coût réel, il est compté ; deux tests « budget serré à 4k » ont été réalignés (à 4k avec quatre orateurs, seules les deux premières sections sont servies).
- Les positions absentes d'une réponse mémoire sont conservées (v1.16 les effaçait) : la mémoire n'oublie pas un participant que l'analyste a omis.
- `CallKind::Thought` reste dans l'énumération (registres persistés) mais n'est plus émis.

Ratchets : Rust 421 tests (3 ignorés), clippy 0, front 53 tests, i18n 990 clés × 3, `tsc` propre.

### Lot 6 — Émotions incarnées

Livré :
- Gains OCEAN (`emotion_engine::PersonaGains`, `gains_from_ocean`) : N → sensibilité aux désapprobations et contradictions, A → sensibilité aux approbations et soutiens, O → curiosité, E → engagement ; bande neutre 4..=7 (gain 1,0 = deltas v1.16 exacts, golden S22), extrêmes linéaires vers `OCEAN_GAIN_MIN/MAX`. Couche 1 du bâtisseur de consignes : variantes « passif-agressif » (A ≥ 8) et « frontal » (A ≤ 3) de la consigne de frustration (les `<dynamics>` du persona gardent la priorité).
- Échantillonnage (`modulate_sampling`) : enthousiasme → température (±0,15, bornée), engagement → `num_predict` (×0,8 à ×1,2) ; interventions seulement, jamais les appels JSON, rien sans `emotion_driven` (S27).
- Didascalies (`engine/stage_directions.rs`) : ligne de théâtre sans LLM sur franchissement de seuil (le plus dramatique des axes franchis), bannissement, retour, réconciliation ; première phrase du champ `<dynamics>` correspondant ou gabarit trilingue ; ≤ 120 caractères ; ≤ 1 par orateur et par tour ; message `kind = stageDirection` dans le flux, jamais dans l'historique moteur ni les prompts (S28).
- Relations pondérées (`engine/relationships.rs`) : scores décroissants (× 0,85 par tour) à côté des compteurs entiers, classification sur le **net** de chaque direction (approbations − désapprobations), `RelationshipEdge.score/trend`, événement `RelationshipShift`, réconciliation (rival qui approuve → didascalie + consigne unique au receveur) (S29).
- Salle (`RoomMood`) : moyenne des GladIAteurs actifs après chaque ronde d'émotions, événement `RoomMoodUpdated`, indice dans le prompt de modération, pastille dans la barre d'état de l'arène (S30, S31).
- `engine/tuning.rs` : `Tuning` (défauts = constantes) consommé par les gains, l'échantillonnage et les relations — le Lot 17 le rendra configurable.
- Fixture régénérée avec `emotion_driven` (1 049 événements, didascalies et salle rejouées côté front) ; les `#[allow(dead_code)]` transitoires sont tous retirés.

Écarts assumés :
- Classification des relations sur le **net** par direction (v1.16 : minimum des compteurs bruts) : nécessaire pour qu'une approbation d'un rival compte comme un pas vers la réconciliation ; un échange équilibré (autant d'approbations que de désapprobations) n'est plus « allié ».
- Réserve de sortie du budget = `num_predict × EMOTION_LEN_MAX` pour tous (≈ 5 % du contexte) plutôt que conditionnée à `emotion_driven` : le calcul de budget ne connaît pas ce réglage et une réserve conservatrice évite tout débordement de contexte.
- Le bannissement émet une didascalie côté sujet en plus de la notification du modérateur (les seuils franchis par la pénalité restent silencieux ce tour-là).

Ratchets : Rust 429 tests (3 ignorés), clippy 0, front 54 tests, i18n 998 clés × 3, `tsc` propre.

### Lot 7 — Pipeline et mesures

Livré :
- Fin de tour parallèle : les quatre phases LLM (document du tour, analyse émotionnelle, mise à jour mémoire, extraction d'arguments) sont scindées en `prepare_*` (`&self`, requête possédée) / `apply_*` (`&mut self`), lancées ensemble via `run_bounded` (borné par `max_parallel_calls`) et appliquées dans l'ordre historique ; contagion, salle, décroissance et instantané d'historique gardent leur place entre émotions et mémoire (S35, golden parallèle = séquentiel).
- Fournisseur séquentiel (`max_parallel_calls == 1`, Ollama) : appel fusionné `CallKind::TurnAnalyst` (`build_turn_analyst_prompt` = les deux prompts intacts + consigne de fusion, `json_parser::parse_turn_analyst` tolérant : une partie absente laisse son état) remplaçant `Memory` + `Emotion` — un appel de moins par tour en local (S36).
- Mesures : `TurnTimer` → événement `TurnTimings { turn, phases }` (« speakers », « endOfTurn » et chaque phase mesurée dans son propre futur) ; `DiagnosticsCounters` → `DiagnosticsReady { diagnostics }` juste avant `DiscussionEnded` (JSON illisibles par type d'appel — intention, modération, mémoire, émotions, analyste, carte —, refus, relances, conformité d'intention mesurée sur le texte prononcé) (S37, S38). Persistés dans `report_json.timings/diagnostics`, panneau « Diagnostic du moteur » repliable sur la page Résumé / Historique.
- Le mock de test passe à `max_parallel_calls = 2` (les appels restent séparés) ; `sequential_caps()` exerce la voie fusionnée ; la fixture est régénérée (timings et diagnostics rejoués côté front).

Écarts assumés :
- **Recouvrement abandonné** (option prévue dans le plan) : faire tourner la ronde de réactions du message N pendant la recherche de l'orateur N+1 exigerait de sortir ces deux phases de `&mut self` (contextes possédés) pour un gain limité au cloud ; la ronde de réactions est déjà parallèle en interne et la fin de tour l'est désormais.
- La mise à jour mémoire dont le résumé est vide conserve le résumé précédent (v1.16 l'effaçait).
- L'analyse émotionnelle « {} » (aucun delta) n'est pas comptée comme un échec : seul un texte non JSON l'est.

Ratchets : Rust 437 tests (3 ignorés), clippy 0, front 55 tests, i18n 1 016 clés × 3, `tsc` propre.

### Clôture de la livraison 1.17 « pertinence et sources »

- Documentation alignée : `CLAUDE.md` (architecture, ratchets, commandes), `Docs/Technique/TECHNICAL.md` (§6.16 + changelog v1.17), `Docs/Fonctionnel/FUNCTIONAL.md` (§13 réactions, §14 résumé, §26 arène vivante, changelog), `README.md` (badge 1.17, fonctionnalités, tests, roadmap). L'alignement des numéros de version des manifestes (`package.json`, `Cargo.toml`, `tauri.conf.json`, encore à 0.1.0) reste au Lot 18 comme prévu.
- Banc de prompts v1.17 sur `mistral-small3.1` (Ollama, 947 s) archivé : `Docs/Technique/bench/2026-09-16-v1.17-ollama-mistral-small3.1.{md,json}`. `node tools/bench-compare.mjs` contre la référence v1.16 : **aucune régression** — usage des noms 0,44 → 0,67 (idéation) et 0,67 → 0,83 (socratique), fuites Markdown 1 → 0 (débat, idéation), répétition stable (0,17–0,21), refus 0, écart entre orateurs −1,4 à −1,5 s par scénario (appel fusionné), conformité d'intention 0,89 / 0,71 / 1,00 (cible ≥ 0,8 atteinte en débat et socratique ; l'idéation vise par nature moins souvent une personne).
- Fixture d'événements régénérée après chaque lot (`export_event_fixture`) et rejouée par le front.

### Lot 8 — Dramaturgie (1.18)

Livré :
- Actes (`engine/dramaturgy.rs`) : `ActKey` (24 actes typés), scripts par mode (Débat 5 actes, Idéation / Co-construction / Socratique / Tutoriel / Critique 3, Fiction 4, UserDriven aucun), `resolve_act` (proportionnel avec `max_turns` — chaque acte médian garde au moins un tour —, glissant sans limite, concessions anticipées par la stagnation, plaidoiries dès l'arrêt doux, `max_turns = 1` = plaidoirie), annonce IArbitre templatée (`kind = actAnnouncement`, dans l'historique moteur donc visible des orateurs), consigne d'acte dans le bloc « [Mise en scène de ce tour] » du prompt d'intervention (le rappel générique de fin s'efface devant l'acte de clôture), indice d'acte dans le prompt de modération, événement `ActStarted`.
- Événements de scène (`engine/scene_events.rs`) : `SceneEvent { SurpriseFact, FormatConstraint(5 contraintes), AudienceQuestion, ForcedSteelman, Duel, HotSeat }`, politique pure (`SCENE_EVENT_BASE_PROBABILITY` + `SCENE_EVENT_STAGNATION_BOOST`, jamais au tour 1 ni au dernier, `SCENE_EVENT_MIN_GAP_TURNS`, modes éligibles, préconditions : source de connaissance avec quota, public activé, ≥ 3 actifs pour duel / sellette), matérialisation par le moteur (fait surprise cherché sur Wikipedia puis le web au nom de l'IArbitre, repli sur une contrainte de forme ; duel = paire la plus tendue ; sellette = orateur le plus réactif ; question de la salle = orateur aux fils ouverts), annonce `kind = sceneEvent`, consignes ≤ 300 caractères aux orateurs concernés, manipulation de l'ordre (duel : deux orateurs seuls ; sellette : cible en dernier), événement `SceneEventTriggered`.
- Coalitions : `SpeechAct::Relay` (11ᵉ acte, jamais tiré, forcé sur le meneur), suiveur déplacé juste après avec la consigne « prolonge sans répéter », `CoalitionFormed`, `COALITION_PROBABILITY`, uniquement modes débat avec ≥ 3 actifs, jamais en plus d'un événement de scène.
- Front : réducteur `stage.ts` (acte courant, événement du tour, **timeline** des moments — actes, événements, coalitions, bans, retours, bascules de relation, tours de l'utilisateur — persistée dans `report_json.timeline`), pastille de l'acte dans la barre de l'arène ; les annonces et événements s'affichent déjà dans le fil (kinds).
- Tests : S13–S17, S20 (+ sellette), `dramaturgy` et `scene_events` unitaires (rng seedé), `Relay` jamais tiré ; crochets de test `force_scene_event`, `force_coalitions`, `disable_random_staging` (le harnais est déterministe : les dés ne roulent que sur demande).

Écarts assumés :
- **Arrêt doux** : le moteur coupait après l'orateur en cours ; il laisse désormais les orateurs restants du tour faire leur plaidoirie, exécute la fin de tour puis la synthèse — conforme au libellé du bouton (« Terminer après ce tour ») et à `FUNCTIONAL.md`. L'arrêt forcé reste immédiat.
- Un arrêt doux reçu **entre** deux tours ne déclenche pas de tour de plaidoiries supplémentaire (fin immédiate, comme avant).
- Question de la salle : sans saisie du public, la consigne demande de répondre à la question la plus gênante déjà posée (fils ouverts) ; les bandeaux et la barre de timeline arrivent au Lot 9.

Ratchets : Rust 452 tests (3 ignorés), clippy 0, front 56 tests, i18n 1 017 clés × 3, `tsc` propre.

### Lot 9 — Scène et spectacle (1.18)

Livré :
- `lib/stage.ts` (pur, testé) : positions sur un arc (`arcPositions`), émotion dominante et teintes d'aura (`dominantEmotion`, `EMOTION_HUE`), constantes des effets (`STAGE_MAX_BURSTS = 6`, `BURST_MS = 900`, `BANNER_MS = 800`), table des raccourcis (`shortcutAction` : Espace, I, ← →, M, P, S, ?, Échap ; ignorés dans les champs, les boutons et avec modificateur), `neighbourTurn`.
- `stores/useUiStore.ts` (testé) : préférences de présentation persistées (`stageVisible`, `muted`, `soundsEnabled`), `presentationMode` (jamais persisté), `viewedTurn`.
- Réducteur `stage.ts` étendu : réactions volantes (`bursts`, file plafonnée, retirées par la scène), bandeaux (tour, acte, événement, ban), coalition du tour ; actions `dismissBurst` / `dismissBanner`.
- `components/stage/` : `ArenaStage` (arc d'avatars, projecteur sur l'actif, autres assombris, bannis derrière la grille, passés grisés, aura colorée par l'émotion dominante, lien de coalition, teinte de fond selon l'ambiance, rang dans l'ordre du tour, `ReactionBurst` en CSS `arena-burst`, `role="img"` + `aria-label`, défilement horizontal sous 560 px), `SceneBanner` (`aria-live="polite"`, `motion-safe` uniquement, police d'affichage système), `TimelineBar` (un segment par tour, repères colorés, clic → `#turn-N`).
- Coulisses en direct : la bulle « réfléchit… » affiche l'objectif, la cible et l'angle de l'intention dès `intentionGenerated`, à côté du chronomètre.
- Thème « arène » : jetons `--arena-stage-top/bottom`, `--arena-spotlight` (clair / sombre), classe `.font-display` (pile serif système, aucune police web hors ligne), `@keyframes arena-burst`.
- Mode projection : `lib/presentation.ts` (`setPresentation` : barre latérale et panneau masqués, plein écran Tauri `getCurrentWindow().setFullscreen`, permission `core:window:allow-set-fullscreen`, repli silencieux hors Tauri), boutons scène / projection dans la `TopBar`, sortie par Échap ou en quittant l'arène.
- Raccourcis (`hooks/useArenaShortcuts.ts`, bureau uniquement) branchés sur les commandes existantes ; aide `?` en toast.

Écarts assumés :
- Bandeau à chaque tour en plus des actes / événements / bans (800 ms, `motion-safe`) : utile pour suivre le rythme en projection ; la préférence « scène » le masque avec la scène ? Non — le bandeau reste indépendant de la scène.
- Les libellés des repères de la timeline sont ceux du moteur (langue de la discussion), les titres de repères sont traduits (langue de l'interface), comme pour les messages.
- Le viewedTurn du clavier est remis à zéro à chaque nouvelle discussion.

Contrôles manuels (§5.3, scène / raccourcis / projection) : à passer en revue dans l'application à la prochaine session interactive — vérifiés ici par la compilation (`npm run build`), les tests des helpers purs et des réducteurs, et la revue à froid.

Ratchets : Rust 452 tests (3 ignorés), clippy 0, front 63 tests, i18n 1 040 clés × 3, `tsc` propre, `npm run build` OK, `pages/ArenaPage.tsx` 247 lignes.

### Lot 10 — Voix et sons (1.18)

Livré :
- `lib/speech.ts` (testé) : `splitSentences` (terminateurs latins et CJK, reste inachevé conservé), `cleanForSpeech` (formules KaTeX et résidus Markdown retirés), `pickVoice` (voix de la langue de discussion choisie par hachage du nom, voix par défaut sinon), `prosodyFromOcean` (E → débit 0,9–1,15, N → hauteur 0,9–1,2), `oceanFromPrompt`, `SpeechEngine` (file d'énoncés, `feed` par morceaux diffusés → phrase complète → `speak`, `flush` à la fin du message, `say` pour la synthèse, pause / reprise, politique `follow` | `full`, silence propre sans `speechSynthesis`).
- `lib/sounds.ts` (testé) : `SoundEngine` Web Audio procédural — gong (trois partiels sinus, enveloppe), murmure (bruit filtré), applaudissements (rafales de bruit passe-haut), sifflet (onde carrée) —, contexte créé paresseusement et repris au premier geste (`unlock`), silence sans Web Audio ou contexte suspendu.
- `hooks/useArenaAudio.ts` : `useAudioSettingsSync` (coquille de l'application : réglages → moteurs, couper la voix la stoppe partout), `useArenaAudio` (arène : profils vocaux du casting, puits audio du store — texte diffusé et événements —, synthèse lue par la voix de l'IArbitre, indices sonores : gong au tour, sifflet au ban, murmure sur événement de scène, applaudissements sur approbation du public, pause / reprise synchronisées ; la voix se tait en quittant l'arène sauf après la fin, pour que la synthèse soit lue sur le résumé).
- Réglages durables `AppSettings` : `tts_enabled`, `tts_mode` (`TtsMode::{Follow, Full}`), `tts_volume`, `sound_enabled`, `sound_volume` (clés DB, défauts, `Debug` masqué) ; section « Voix et sons » des Réglages (`AudioSettings`, test du gong) ; boutons voix / sons dans la barre de l'arène ; raccourcis M / S.
- `useArenaStore.registerAudioSink` : le puits audio reçoit les morceaux bruts et les événements sans provoquer de rendu.

Écarts assumés :
- Les bascules voix / sons sont des **réglages durables** (base) plutôt que des préférences de session dans `useUiStore` : une seule source de vérité, les raccourcis M / S et les boutons de l'arène passent par `updateSettings`.
- Les réactions des GladIAteurs entre eux ne déclenchent pas d'applaudissements (bruit) — seules celles du public.
- La voix suit la vue de l'arène : naviguer ailleurs pendant une discussion la coupe (sauf vers le résumé à la fin).

Contrôles manuels (§5.3, voix / sons) : à passer en revue dans l'application ; couverts ici par les tests des moteurs sur doubles (synthèse et contexte audio simulés) et la compilation.

Ratchets : Rust 452 tests (3 ignorés), clippy 0, front 71 tests, i18n 1 058 clés × 3, `tsc` propre, `npm run build` OK.

### Lot 11 — Score, récompenses, relecture (1.18)

Livré :
- `lib/score.ts` (testé) : `computeScores` (réactions reçues pondérées par couleur, thèses et arguments ajoutés à la carte, concessions déclarées par les intentions, bans ; points par tour ; classement avec égalités), `computeAwards` (meilleur argument, plus grand retournement — positions —, plus contesté, plus prolifique, réconciliation — timeline —, seules les récompenses attribuables).
- `lib/replay.ts` (testé, S48) : `scheduleReplay` (deltas réels bornés par `REPLAY_MAX_GAP_MS` puis divisés par la vitesse ×1/×2/×4/×8, plancher `REPLAY_MIN_GAP_MS`, horodatages manquants ou désordonnés tolérés), `replayDuration`.
- Store : `concessions` par orateur (réducteur des intentions), `awards` calculés dans `buildReport` (persistés dans `report_json.awards`).
- UI : onglet **Score** du panneau latéral (`Scoreboard` : podium, détail des points, tableau par tour), **générique de fin** (`AwardsCredits`, trophées révélés en cascade `motion-safe`) sur le résumé et l'historique, onglet **Relecture** (`ReplayPlayer` : lecture / pause / recommencer, vitesse, scène animée à partir de l'historique émotionnel et des réactions persistés, timeline, flux progressif) ; `ArenaStage` scindé en `StageView` (props) + `ArenaStage` (store) pour servir la relecture.
- Fixture : le rejeu vérifie les récompenses et la timeline.

Écarts assumés :
- Les concessions comptées au score viennent des **intentions** (concession déclarée ou objectif « concéder »), pas d'une détection dans le texte ; elles ne sont pas persistées : le score en direct les inclut, les récompenses persistées reposent sur ce qui est dans le rapport.
- La relecture d'une discussion antérieure à la 1.17 (sans rapport) rejoue les messages et leurs réactions avec une scène sans émotions, comme prévu.

Ratchets : Rust 452 tests (3 ignorés), clippy 0, front 75 tests, i18n 1 078 clés × 3, `tsc` propre, `npm run build` OK.

### Clôture de la livraison 1.18 « spectacle »

- Documentation alignée : `CLAUDE.md`, `TECHNICAL.md` (§6.17 + changelog v1.18), `FUNCTIONAL.md` (§26 étendu, arrêt doux, changelog v1.18), `README.md` (badge 1.18, fonctionnalités).
- Contrôles manuels §5.3 (scène, raccourcis, projection, voix, sons) : à dérouler dans l'application ; tout ce qui est automatisable l'est (helpers purs, réducteurs, moteurs sur doubles, fixture rejouée, build).

### Lot 12 — Agenda caché et casting (1.19)

Livré :
- `models/agenda.rs` : `Agenda { objective, red_line, victory }`, `AgendaReveal` (aplati pour le front, `achieved: Option<bool>`), `CastingSuggestion` / `CastingPick`.
- Moteur : après l'introduction, un appel `CallKind::Agenda` (JSON, `AGENDA_NUM_PREDICT`) par GladIAteur, lancés ensemble (`run_bounded`) ; échec, annulation ou réponse inutilisable → orateur sans agenda (compté dans les diagnostics `agenda`), discussion inchangée. Le bloc « [Ton agenda secret — ne le révèle jamais explicitement] » est injecté dans le **système** de l'intervention juste après le persona (avant la clause de mode et le préambule) ; l'objectif est rappelé dans le prompt d'intention (« ton intention doit le servir sans le trahir ») ; la synthèse reçoit « [Agendas secrets] » et l'instruction d'une section « ## Agendas » (« objectif atteint / non atteint — pourquoi ») ; `AgendaRevealed { agendas }` après `SynthesisComplete` avec le verdict lu dans cette seule section (`json_parser::agenda_outcome`, trilingue, « partiellement » → inconnu).
- Modes : `DiscussionMode::supports_hidden_agenda` (Débat, Fiction — formulation « agenda d'auteur » ; Procès, Oxford, Négociation s'y ajouteront au lot 13) ; l'interrupteur signale dans le wizard qu'il est sans effet ailleurs (`lib/casting.ts::modeSupportsHiddenAgenda` miroir).
- Budget : `BudgetFeatures.agenda_chars` (= `AGENDA_MAX_CHARS + AGENDA_BLOCK_OVERHEAD_CHARS` quand la fonction s'applique) ajouté à l'overhead déterministe ; l'aperçu du wizard passe la même valeur via `get_engine_constants.agenda_block_max_chars`.
- Casting : commande `suggest_casting(topic, mode, lang, count)` (`commands/casting.rs`) — fournisseur construit par la factory comme `start_discussion`, pré-vol du budget mensuel partagé (`commands::llm::cloud_budget_preflight`, extrait de `start_discussion`), catalogue « id — nom : personnalité » borné par le contexte (`catalogue_bound`, jamais coupé en milieu de ligne, modérateurs sur un quart), `CallKind::Casting` en JSON, ids filtrés sur le catalogue (`parse_casting`), usage cloud enregistré dans la période même en cas d'erreur.
- Front : `CastingAssistant` (nombre 2..`casting_max_gladiateurs`, raison par choix, IArbitre suggéré, « Remplacer la sélection » / « Ajouter »), `CompatibilityMatrix` (alliés probables / friction probable d'après la distance A/E/O des OCEAN, profils frontaux, contraste du casting ; profils sans OCEAN signalés) ; `AgendaCards` sur l'onglet Positions du résumé et de l'historique (`report_json.agendas`) ; réducteur `agendaRevealed`.
- Tests : S21 (appels, injection système seulement, rappel d'intention, synthèse, révélation après la synthèse, échec d'un appel, profil v1.16, mode incompatible, voix d'auteur), S25 (`parse_casting`, `catalogue_bound`, prompt borné), parseur d'agenda et verdicts, blocs de prompts dans les trois langues (le bloc système tient dans la réserve), `lib/casting.test.ts`, store et fixture (`events-full.json` régénérée : 1 108 événements, verdicts vrai / faux / inconnu).

Écarts assumés :
- `AGENDA_MAX_CHARS = 300` borne le **contenu** (3 × 100) ; le bloc système ajoute `AGENDA_BLOCK_OVERHEAD_CHARS = 260` d'étiquettes et de consigne, et c'est la somme qui est réservée au budget (le plan comptait 300 tout compris, insuffisant pour une consigne de non-révélation trilingue).
- Le verdict n'est lu que dans la section « ## Agendas » de la synthèse (un « a atteint un consensus » ailleurs ne compte pas) ; sans section, issue « incertaine ».
- Les agendas sont révélés même si la synthèse a été sautée (erreur fatale) — verdicts inconnus.
- Le casting ne passe pas la barrière de licence : le wizard n'est atteignable qu'avec une licence valide ; il passe en revanche le pré-vol du budget mensuel DeepSeek.

Ratchets : Rust 462 tests (3 ignorés), clippy 0, front 79 tests, i18n 1 110 clés × 3, `tsc` propre, `npm run build` OK, `StepGladiateurs.tsx` 362 lignes.

### Lots 13 et 14 — Nouveaux modes (1.19)

Livré (livrés ensemble : le socle « rôles / issue / dépêches » est commun) :
- `DiscussionMode::{Trial, OxfordDebate, Negotiation, SixHats, CrisisCell}` avec l'ensemble complet des entrées de `mode_prompts.rs` (descripteur, introduction, préambule, focus de pensée ×2, synthèse, modération, ouverture, engagement, contrainte, sens des réactions, clause de format) et de `prompt_builder.rs` (libellés mémoire, instruction document), les poids d'actes de parole (`directive_builder`), l'éligibilité aux événements de scène (Procès, Oxford, Négociation ; pas Six chapeaux ni Cellule de crise), et cinq scripts d'actes (`dramaturgy.rs`) : Procès et Oxford réutilisent ouverture / confrontation / contre-interrogatoire / plaidoiries ; Négociation = ouverture, **marchandage**, concessions, **accord** ; Six chapeaux = **cadrage**, **exploration**, convergence ; Cellule de crise = **alerte**, **riposte**, **débriefing** (7 nouveaux `ActKey`, trilingues, rangés dans les groupes d'indices de modération).
- Rôles (`engine/mode_roles.rs`) : `GladIAteurConfig.mode_role` (`#[serde(default)]`), rôles sélectionnables Procès (accusation, défense, témoin, juré) et Oxford (pour, contre), distribution par défaut depuis l'ordre du casting (procès : accusation, défense, puis juré / témoin en alternance ; Oxford : alternance), six chapeaux en rotation par tour (`hat_for`), bloc « [Ton rôle — tiens-le du début à la fin] » / « [Ton chapeau ce tour] » borné (`ROLE_BLOCK_MAX_CHARS`) ajouté au persona pour l'intention et l'intervention (`system_prompt_for`), réservé dans le budget ; événement `RolesAssigned { turn, roles }` (une fois au tour 1 ; à chaque tour pour les chapeaux).
- Issue de mode (`models/outcome.rs`, `ModeOutcome` étiqueté `kind`) : Procès → un appel `Verdict` (JSON) par juré lancés ensemble, majorité, l'IArbitre tranche seul sans jury ou départage une égalité (`by_arbitre`) ; Négociation → un appel `Verdict` par partie (« signes-tu l'accord ? »), accord si toutes signent, réponse inutilisable = refus ; Oxford → `AudienceSwing` (vote avant / après, vainqueur par déplacement). L'issue est résolue avant la synthèse, émise (`OutcomeReady`), injectée dans le prompt de synthèse (« [Verdict] / [Accord] / [Vote du public] » + section dédiée, « n'invente jamais une autre issue ») et persistée dans `report_json.outcome` (typé côté front).
- Vote du public (Oxford) : commande `audience_vote(choice)`, fenêtres `AudienceVoteRequested { phase, timeout_secs }` après l'introduction et après le dernier tour (bornées par le délai d'intervention utilisateur ; `SkipUserTurn` ferme la fenêtre ; les autres commandes restent servies), `AudienceVoteRecorded` ; panneau `AudienceVote` dans l'arène (Pour / Contre / Ne pas voter, compte à rebours).
- Cellule de crise : un appel `CallKind::CrisisDispatches` (JSON) après l'introduction écrit `max_turns` dépêches (défaut 5 sans limite, plafond 12, bornées en longueur), une par tour livrée comme événement de scène `SceneEvent::Dispatch { text, index, total }` (annonce de l'IArbitre + consigne aux orateurs), à la place de la politique aléatoire ; réponse inutilisable → cellule sans dépêches (comptée dans les diagnostics).
- Négociation : agendas secrets **obligatoires** (même interrupteur désactivé — signalé dans le wizard).
- Front : cartes de mode ×5, sélecteur de rôle par GladIAteur (Procès, Oxford ; « rôle automatique » sinon), pastille de rôle / chapeau sur la scène, `OutcomePanel` en tête de la synthèse (résumé et historique), `lib/modes.ts` (modes, agendas, rôles, ordre fixe, choix de vote — miroir Rust), réducteurs `rolesAssigned` / `audienceVoteRequested` / `audienceVoteRecorded` / `outcomeReady`, `report.outcome` typé.
- Tests : S18 (rôles par défaut et configurés dans les prompts, verdict du juré, issue avant la synthèse, synthèse informée, IArbitre seul sans jury, jury partagé départagé), S19 (deux fenêtres de vote, choix inconnu ignoré, issue par déplacement, refus de vote ferme la fenêtre sans attendre), négociation (décision par partie, refus sur réponse inutilisable, agendas forcés), six chapeaux (rotation par tour, chapeau dans le système), cellule de crise (dépêches en nombre = tours, une par tour, sans limite → défaut, réponse inutilisable), balayage de tous les modes sans erreur, textes non vides ×13 modes ×3 langues, `mode_roles`, `outcome`, parseurs (verdict, accord, dépêches), blocs de synthèse ×3 langues ; store front (rôles, fenêtre de vote, issue).

Écarts assumés :
- Les verdicts et décisions n'emploient pas `parse_vote` (classement Borda) : un parseur dédié tolérant (`parse_verdict` — accusation / défense dans les trois langues, coupable / non coupable ; `parse_agreement` — booléen ou oui / non) est plus sûr qu'un détournement du vote ordinal.
- Les dépêches sont un variant `SceneEvent::Dispatch` (annonce « Dépêche n/N ») plutôt qu'un `SurpriseFact` déguisé : la bannière et la chronologie les nomment correctement.
- Six chapeaux et Cellule de crise n'ont ni agendas, ni événements de scène aléatoires, ni coalitions (structure imposée) ; Procès, Oxford et Négociation gardent tout.
- Le vote du public est celui d'un seul spectateur (l'utilisateur) : le « déplacement » est le changement d'avis entre avant et après ; sans changement, pas de vainqueur.
- Les fenêtres de vote coûtent au plus deux délais d'intervention utilisateur ; « Ne pas voter » les ferme immédiatement.

Ratchets : Rust 474 tests (3 ignorés), clippy 0, front 80 tests, i18n 1 165 clés × 3, `tsc` propre, `npm run build` OK, `StepGladiateurs.tsx` 377 lignes, `ArenaPage.tsx` 277 lignes.

### Clôture de la livraison 1.19 « enjeux et formats »

- Documentation alignée : `CLAUDE.md` (vue d'ensemble v1.19, ratchets, modules `mode_roles.rs` / `casting.rs`, sections « Hidden Agendas and Casting » et « Structured modes »), `TECHNICAL.md` (§6.18, arborescence, tests, changelog v1.19), `FUNCTIONAL.md` (13 modes, sections Procès / Oxford / Négociation / Six chapeaux / Cellule de crise, agendas cachés, casting assisté, changelog v1.19), `README.md` (badge 1.19, section « Enjeux et formats », 13 modes, roadmap).
- Contrôles manuels à dérouler dans l'application : fenêtre de vote (Pour / Contre / Ne pas voter) sur un débat d'Oxford réel, sélecteur de rôle et pastilles sur la scène, cartes d'agendas et panneau d'issue sur le résumé, casting sur Ollama et DeepSeek (catalogue borné, refus au-delà du budget mensuel).

### Lot 15 — Multi-modèle et OpenAI-compatible (1.20)

Livré :
- `llm/openai_compat.rs` : transport SSE partagé (`OpenAiCompatTransport` : types de câblage, parsing SSE, usage normalisé, retry/backoff, `/models`, classification HTTP) avec `Dialect::{DeepSeek, Generic}` — le dialecte générique n'envoie ni `thinking` ni `reasoning_effort` (S39), passe température / `top_p` tels quels, borne `max_tokens` (`OPENAI_COMPAT_MAX_OUTPUT_TOKENS`), accepte une clé vide (pas d'en-tête `Authorization`) ; `OpenAiCompatProvider` (`ProviderKind::OpenAiCompat`, pas de réflexion, JSON, `billable = false`, `OPENAI_COMPAT_MAX_PARALLEL_CALLS`), validation tolérante à l'absence de catalogue mais pas à un modèle absent d'un catalogue publié, 401 → fatal non relancé. `DeepSeekProvider` devient une configuration du transport (dialecte DeepSeek + `/user/balance`) — ses tests unitaires et de transport sont conservés à l'identique.
- Réglages `openai_compat_base_url` (défaut `OPENAI_COMPAT_DEFAULT_BASE_URL`), `openai_compat_api_key` (masquée dans `Debug`), `openai_compat_model`, `openai_compat_models` (liste manuelle JSON) ; commandes `list_openai_compat_models`, `validate_openai_compat` ; `LlmConstants.openai_compat_default_base_url`.
- Multi-modèle : `GladIAteurConfig.model` / `IArbitreConfig.model` (`#[serde(default)]`), trait `LlmProvider` étendu par méthodes à défaut `model_for(request)` / `capabilities_for(speaker)`, `llm/routing.rs::RoutingProvider` (dispatch par `speaker_id`, repli sur le modèle global pour les utilitaires, validation de chaque fournisseur distinct nommant l'orateur en échec — S24), `factory::build_provider_for(settings, overrides)` (un fournisseur interne par modèle distinct, `start_discussion` refuse avant le spawn), `MeteredProvider` tarifie avec `model_for` et attribue `UsageLedger.by_model` (S23), moteur : `resolve_reasoning` / affichage du raisonnement lisent `capabilities_for(orateur)`.
- Front : troisième fournisseur dans les Réglages (`OpenAiCompatSettings` : URL, clé facultative, modèle avec catalogue ou saisie manuelle, test de connexion), sélecteur « modèle de cet orateur : hérite / … » dans `LlmParamsForm` (GladIAteurs et IArbitre, listes par fournisseur via `availableModels`), `describeActiveModel(settings, overrides)` → « ollama · mixte (a, b) », avertissement VRAM sur le résumé quand plusieurs modèles Ollama, garde du wizard `openaiCompatRequired`, Ollama en mode embeddings seuls hors fournisseur local.
- Tests : dialecte générique (câblage sans champs de réflexion, clé vide sans en-tête, 401 fatal, validation avec / sans catalogue) sur serveur TCP jetable, routage (dispatch, capacités par orateur, nommage de l'orateur en échec), fabrique (refus de configuration, repli sans routeur), ledger `by_model`, store front (`mixte`, listes par fournisseur, liste manuelle).

Écarts assumés :
- Les temporisations du transport restent les constantes `DEEPSEEK_*` (documentées comme partagées) plutôt que dupliquées sous un nouveau nom.
- `stream_options.include_usage` et `response_format: json_object` sont envoyés au dialecte générique (OpenAI, LM Studio, vLLM, llama.cpp récent, OpenRouter les acceptent) ; un serveur qui les refuse renvoie une erreur cliente explicite.
- La clé OpenAI-compatible est enregistrée par l'autosauvegarde des Réglages (contrairement à la clé DeepSeek validée avant stockage) : elle est facultative et vise surtout des serveurs locaux ; le bouton « Tester la connexion » joue le rôle de validation.
- L'événement `LlmUsageUpdated.model` reste le modèle global ; la répartition par modèle est dans le ledger (`by_model`), l'étiquette « mixte » vient du front.

Ratchets : Rust 484 tests (3 ignorés), clippy 0, front 82 tests, i18n 1 188 clés × 3, `tsc` propre, `npm run build` OK.

### Lot 16 — Modèles, historique enrichi, exports (1.20)

Livré :
- Base : colonnes `discussions.tags` (JSON) et `discussions.favorite` (migrations idempotentes), table `discussion_templates`, index plein texte FTS5 `discussions_fts (discussion_id UNINDEXED, topic, synthesis, content)` créé quand le module est compilé (H-P5 vrai), **rétro-indexé** au démarrage pour les discussions antérieures (`backfill_fts`, idempotent), alimenté dans la transaction de sauvegarde, purgé à la suppression ; sans FTS5 ou sur une requête refusée → balayage `LIKE` équivalent (`like_search`). Requête FTS assainie (`fts_query` : chaque terme cité et préfixé, termes tous requis).
- Dépôt : `search_discussions`, `set_discussion_tags` (tags nettoyés), `set_discussion_favorite`, `SUMMARY_COLUMNS` / `row_to_summary` partagés par la liste et la recherche ; `list_/save_/delete_template` (les modèles seedés ne sont ni écrasés — `ON CONFLICT … WHERE builtin = 0` — ni supprimés) ; 4 modèles seedés (`seed_templates` : procès d'une idée, brainstorming produit, revue d'un document, fiction à trois voix) dont les profils référencés sont vérifiés contre le catalogue par un test.
- Commandes : `search_discussion_history`, `set_discussion_tags`, `set_discussion_favorite`, `list_/save_/delete_discussion_template` (`commands/templates.rs`, validation `DiscussionTemplate::validate`, `builtin` forcé à faux, `created_at` rempli).
- Front : `lib/templates.ts` (forme `TemplateConfig`, `templateFromSetup`, `parseTemplateConfig`, `applyTemplate` avec profils manquants signalés — S40), `TemplatePicker` en tête du pas 1 (charger / enregistrer / supprimer, noms des modèles seedés traduits par `templates.<id>`), `useSetupStore.applyTemplatePatch` ; `lib/history-filters.ts` (mode, fournisseur, participant, tag, favoris, `collectTags`, `normaliseTag`), `HistoryFilters` + `TagEditor`, page Historique avec recherche plein texte débouncée (réponses périmées ignorées), étoile de favori et tags en ligne (mise à jour optimiste, retour arrière sur erreur) ; `lib/export-html.tsx` (`renderDiscussionHtml` : page autonome, CSS en ligne, `SimpleMd` rendu par `react-dom/server`, cartes SVG intégrées quand rendues, issue, agendas, positions, fil, sources, document, sans script, échappement systématique), boutons « Exporter la page (.html) » et « Imprimer / PDF » (`window.print()` + règles `@media print`, H-P4 plan B), en-tête `data-topbar`.
- Tests : dépôt (S41 recherche par synthèse et message, termes requis, ponctuation, `LIKE` équivalent, purge à la suppression, rétro-indexation ; S42 tags / favori aller-retour ; S40 modèles seedés, protégés, sauvegardés, remplacés, supprimés), modèle (`validate`), front (`templates.test.ts`, `history-filters.test.ts`, `export-html.test.tsx` — vitest accepte désormais les tests `.tsx`, CSS désactivé).

Écarts assumés :
- Le fil de discussion de l'export HTML est rendu en HTML simple (pas `ReadOnlyFeed` / `renderToStaticMarkup` des composants liés aux stores) : mêmes contenus, mise en page dédiée à l'impression ; les formules KaTeX y sont affichées via leur MathML (partie HTML masquée).
- L'impression passe par `window.print()` sans détection préalable de WebView2 (H-P4) : le bouton est toujours proposé.
- Les filtres (mode, fournisseur, participant, tag, favori) s'appliquent côté front sur la liste (ou le résultat de recherche) ; seule la recherche plein texte interroge la base.

Ratchets : Rust 489 tests (3 ignorés), clippy 0, front 89 tests, i18n 1 226 clés × 3, `tsc` propre, `npm run build` OK, `HistoryPage.tsx` 180 lignes.

### Lot 17 — Réglages avancés et mémoire longue (1.20)

Livré :
- `engine/tuning.rs` : `Tuning` sérialisable (camelCase, `#[serde(default)]` par champ → un JSON partiel suffit), 14 boutons (gains OCEAN, échantillonnage émotionnel, relations, **probabilités des coups de théâtre et des coalitions** ajoutées), `Tuning::BOUNDS`, `validate()` (bornage, non-fini → défaut, paires min/max croisées → défauts, corrections journalisées — S43), `from_settings(json)` ; réglage `advanced_tuning_json` (clé DB, défaut « {} »), `DiscussionEngine::set_tuning` appelé par `start_discussion`, `SceneContext { base_probability, stagnation_boost }` et `try_coalition` lisent le tuning ; commande `get_tuning_info` (défauts + bornes). Front : `lib/tuning.ts` (parse / fusion / sérialisation des seuls écarts, pas de curseur), section « Réglages avancés » repliée (`AdvancedTuning` : curseurs bornés par groupe, valeur par défaut rappelée, réinitialisation).
- Mémoire longue : `models/persona_memory.rs` (`PersonaRecap { positions, best_lines, allies, rivals, lesson }`, `PersonaRecapRecord`, `PersonaMemory`), table `persona_memories` (FK CASCADE sur la discussion, index profil), `GladIAteurConfig.source_profile_id` (H25), réglage `persona_memory_enabled` (défaut vrai). Fin de discussion : un appel `CallKind::Recap` (JSON, `RECAP_NUM_PREDICT`) par GladIAteur issu d'un profil, lancés ensemble, avec ses dernières interventions, le résumé final et ses alliés / rivaux (`build_recap_prompt`, trilingue), parsé et borné (`parse_recap` : listes ≤ 3, éléments ≤ 160 caractères, noms rapprochés des participants), émis en `PersonaRecapReady` — **persisté par le front** avec la discussion (`SaveDiscussionRequest.recaps` → `persona_memories`, dans la même transaction, pour que la cascade tienne). Démarrage : `recall_persona_memories` (BM25 sur « sujet + recap », mots courts ignorés, la récence complète — S26) → bloc « [Souvenirs de discussions passées] » (`build_memories_block`, ≤ `PERSONA_MEMORY_MAX_CHARS`, réservé dans le budget) ajouté au persona pour l'intention et l'intervention (`system_prompt_for`). Commandes `count_persona_memories`, `forget_persona_memories` ; section « Mémoire des personas » (interrupteur, compteur, « Tout oublier »).
- Tests : `tuning` (défauts = constantes, bornage, paires, JSON partiel, NaN, aller-retour), dépôt (S26 : sauvegarde filtrée, classement par sujet, complément par récence, cascade, oubli), parseur de recap, blocs de prompts ×3 langues, moteur (S26 : rappel dans le système avec le sujet le plus proche d'abord, un `Recap` par orateur issu d'un profil après la synthèse, événement avant `DiscussionEnded`, rien pour un persona personnalisé, réglage désactivé → rien), front (`tuning.test.ts`, store des recaps).

Écarts assumés :
- La réserve de budget pour les souvenirs (`PERSONA_MEMORY_MAX_CHARS`) est comptée pour tout orateur issu d'un profil, même mémoire désactivée (≈ 250 jetons de marge, plus simple qu'un budget dépendant d'un réglage).
- Le rappel n'ignore que les mots de moins de trois caractères (pas de liste de mots vides par langue) : suffisant pour écarter articles et prépositions.
- Aucun recap n'est écrit après un arrêt forcé (l'utilisateur veut la fin immédiate) ni après une erreur fatale.
- La commande `validate_tuning` prévue n'est pas exposée : le front n'écrit que les écarts et le moteur borne à la lecture.

Ratchets : Rust 494 tests (3 ignorés), clippy 0, front 92 tests, i18n 1 273 clés × 3, `tsc` propre, `npm run build` OK.

### Lot 18 — Exploitation, docs, clôture (1.20)

Livré :
- Versions alignées à **1.20.0** (`package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `Cargo.lock`).
- `commands/diagnostics.rs` : `read_backend_log` (fin du journal du jour, `LOG_EXPORT_MAX_BYTES`, coupé sur une ligne, UTF-8 sûr), `get_app_version` ; `LOG_DIR_NAME` / `LOG_FILE_PREFIX` partagés avec l'initialisation du journal ; `LlmConstants.releases_url`. Section « À propos et maintenance » (`AboutSettings`) : version installée, « Vérifier les mises à jour » (ouvre la page des versions dans le navigateur), « Exporter le journal » (tampon du logger front + journal moteur dans un fichier texte).
- Documentation alignée : `CLAUDE.md` (vue d'ensemble v1.20, ratchets, modules `openai_compat` / `routing` / `templates` / `diagnostics`, section « Platform (v1.20) »), `TECHNICAL.md` (§6.19, arborescence, tests, ratchets, changelog v1.20, tableau des API LLM), `FUNCTIONAL.md` (paramètres, sections multi-modèle / modèles-historique-exports / mémoire, changelog v1.20), `README.md` (badge 1.20, section « Plateforme », roadmap).

Écarts assumés :
- **Updater Tauri non câblé** : `tauri-plugin-updater` exige une clé publique valide et un point de publication signant les paquets ; sans eux, un câblage « à blanc » ferait échouer le démarrage ou le build (`createUpdaterArtifacts`). La marche à suivre est documentée dans `TECHNICAL.md` §6.19 ; la vérification des mises à jour est manuelle (page des versions).
- Le journal exporté ne contient que la fin du journal du jour (512 Ko) : suffisant pour un diagnostic, sans exposer plusieurs jours.

- `engine/orchestrator.rs` (5 262 lignes après les lots 12 à 17, au-delà du ratchet « ≤ 4 000 ») découpé en **module répertoire** `engine/orchestrator/` : `mod.rs` (structure, cycle de vie, boucle de tours, fin de tour, synthèse — 3 059 lignes), `staging.rs` (dramaturgie — 666), `structured.rs` (modes structurés, agendas, mémoire des personas — 431), `knowledge.rs` (web, Wikipédia, RAG, déduplication — 478), `analysis.rs` (analyse de fin de tour, carte des arguments — 654). Déplacement pur : chaque fichier enfant est un bloc `impl DiscussionEngine` dont les méthodes passent en `pub(super)` ; aucun test modifié, 495 tests et clippy 0 inchangés.
- **Banc final v1.20 non rejoué** : la campagne lancée sur `mistral-small3.1` (Ollama) a été interrompue — la carte graphique était occupée par un autre processus (22,3 Go sur 23 Go, 100 % d'utilisation), le débit de génération est tombé de 43 à 0,5 jeton/s et les appels expiraient (délai d'inactivité, HTTP 500 côté Ollama). Rejouer sur une machine libre : `AIRENA_BENCH_PROVIDER=ollama OLLAMA_MODEL=mistral-small3.1:latest cargo test --lib bench_prompts -- --ignored --nocapture` puis `node tools/bench-compare.mjs Docs/Technique/bench/2026-09-16-v1.17-ollama-mistral-small3.1.json src-tauri/target/bench/<date>-ollama-mistral-small3_1_latest.json` et archiver le résultat sous `Docs/Technique/bench/2026-09-16-v1.20-…`. La dernière mesure valable reste celle de la clôture 1.17 (aucune régression contre v1.16) ; les lots 1.18 à 1.20 ont ajouté aux prompts d'intervention des blocs actifs par défaut (actes, coups de théâtre, coalitions) ou optionnels (agendas, rôles, souvenirs) dont l'effet sur la répétition, l'usage des noms et la conformité d'intention n'est donc **pas mesuré** — la mesure fait partie de la dette restante.

Mesures : Rust 495 tests (3 ignorés), clippy 0, front 92 tests, i18n 1 284 clés × 3, `tsc` propre, `npm run build` OK.

### Auto-évaluation finale (clôture 1.20)

**Complétude.** Les 19 lots (0 à 18), lots optionnels compris, sont livrés dans l'arbre de travail (non commité, à la demande de l'utilisateur). Les 48 simulations S1–S48 sont couvertes par des tests automatisés, sauf les cinq qui exigent un fournisseur réel ou l'application lancée : S46 (banc réel — joué sur Ollama en v1.16 et v1.17, jamais sur DeepSeek faute de clé dans l'environnement), et les listes manuelles de §5.3 (voix, sons, scène, raccourcis, projection, sources, export HTML hors ligne, impression), qui restent à dérouler dans l'application par l'utilisateur.

**Ratchets** (cibles fixées en §6 avant développement, mesures au dernier lot) :

| Ratchet | Cible §6 | Mesuré | Verdict |
|---|---|---|---|
| Tests Rust | ≥ 520 | **495** (+3 ignorés), soit +119 depuis 376 | Sous la cible (−5 %). Cause : consigne « pas de test miroir de l'implémentation » appliquée aux déplacements purs (découpage de l'orchestrateur), aux commandes Tauri de simple délégation et aux prompts (couverts par le banc, non par des tests de chaîne) |
| Tests front | ≥ 110 | **92**, soit +52 depuis 40 | Sous la cible (−16 %). Même cause : les composants sans logique (cartes, sélecteurs, sections de réglages) n'ont pas de test ; la logique est dans `lib/*` et les réducteurs, tous testés |
| Parité i18n | ≈ 1 180 clés | **1 284 × 3**, contrôle bloquant dans `npm run build` | Atteint |
| Clippy | 0 sur `--all-targets` | **0** | Atteint |
| `pages/*` ≤ 400 l. | — | max 195 | Atteint |
| `components/**` ≤ 400 l. | — | tous ≤ 400 sauf `PersonaEditor` (596, exception antérieure au plan) | Atteint, exception connue |
| Orchestrateur ≤ 4 000 l. | 1 fichier | module répertoire, plus gros fichier 3 059 l. (total 5 288) | Atteint par fichier ; le périmètre (agendas, cinq modes, mémoire) a grandi de 2 400 lignes |
| Surcoût déterministe ≤ constante | test | `BUDGET_DETERMINISTIC_OVERHEAD_CHARS` mesuré par test | Atteint |
| Banc : fuites 0, refus 0, répétition ≤ réf, conformité ≥ 0,8 | v1.16 réf. | v1.17 : 0 / 0 / 0,18–0,21 / 0,89–1,00 (idéation 0,71) ; v1.20 non rejoué (voir Lot 18) | Partiel : à rejouer sur GPU libre |

**Écarts assumés** (tous consignés dans les entrées de lot) : updater Tauri non câblé (Lot 18) ; réactions immédiates coûteuses en local, réversibles par `reactionTiming = deferred` (Lot 4) ; `DiscussionMode::ALL` réservé aux tests ; réserve de budget des souvenirs comptée même mémoire désactivée et aucun recap après un arrêt forcé (Lot 17) ; commande `validate_tuning` non exposée (le front n'écrit que les écarts, le moteur borne à la lecture) ; banc DeepSeek non joué.

**Sécurité (OWASP, secrets).** Aucune clé journalisée (les transports n'inscrivent que le statut HTTP ; `DeepSeekSettings` valide la clé avant de la stocker, la clé OpenAI-compatible — facultative pour un serveur local — se teste à la demande) ; `tools/.keys.json` intact et ignoré ; export HTML autonome sans script distant, contenu échappé par `renderToStaticMarkup` ; FTS5 avec paramètres liés et repli par balayage en mémoire (aucune interpolation SQL de la requête) ; `read_backend_log` limité au journal du jour et borné (`LOG_EXPORT_MAX_BYTES`) ; commandes Tauri sans chemin libre (le journal est lu dans le répertoire de l'exécutable, l'export passe par la boîte de dialogue système).

**Dette restante, par ordre d'intérêt** : (1) rejouer le banc v1.20 et l'archiver ; (2) dérouler §5.3 dans l'application ; (3) câbler l'updater dès qu'un point de publication signe les paquets ; (4) remonter les tests front vers 110 en couvrant `CastingAssistant` et `TemplatePicker` (logique d'état non triviale) ; (5) `PersonaEditor` (596 l.) à découper à la prochaine intervention sur ce composant.

**Réponse à la question de CLAUDE.md** (« pleinement satisfait et convaincu ? ») : oui sur le fond (chaque lot livré avec ses tests, ses constantes, ses trois langues, ses docs et sa revue à froid) et sur la forme (gates verts) ; avec les deux réserves chiffrées ci-dessus — ratchets de tests sous les cibles du plan pour une raison de doctrine, non de négligence, et banc final à rejouer.

### Revue adversariale à froid de la livraison complète (post-clôture 1.20)

Relecture intégrale, code en main, de tout ce que les lots 1 à 18 ont ajouté ou modifié (≈ 30 000 lignes Rust, 166 fichiers), avec un plan de vérification empirique en plus des suites de tests : migration d'une base v1.16 réelle, serveur HTTP jetable pour le transport OpenAI-compatible, rendu à 400 px (Accueil, Réglages, Assistant, Historique) sur le serveur Vite, banc de prompts sur un modèle réel.

**Défauts trouvés et corrigés (tous couverts par un test quand le comportement est observable)** :

| # | Domaine | Défaut | Correction |
|---|---|---|---|
| 1 | Fournisseur OpenAI-compatible | Un serveur injoignable (connexion refusée) passait pour « sans catalogue » : `validate()` réussissait, « Tester la connexion » disait OK et `start_discussion` lançait un moteur condamné | Seules les réponses HTTP 4xx / charge illisible valent « pas de catalogue » ; erreur de connexion, 5xx et clé refusée font échouer la validation (test `validate_reports_an_unreachable_server` ; le test de la fabrique utilise désormais un serveur local 404) |
| 2 | Procès | Jury présent mais aucun verdict exploitable → aucun verdict du tout (`winner: None`, `byArbitre: false`) | Le modérateur tranche dès qu'aucune majorité ne se dégage, jury muet compris ; `byArbitre` = aucun vote de juré (test moteur) |
| 3 | Mémoire longue | Les recaps (un appel facturé par persona) étaient écrits après l'épuisement du budget mensuel | Aucun recap une fois l'alerte « exceeded » émise (test `budget_warning_then_soft_stop_when_exhausted` étendu) |
| 4 | Historique | Repli `LIKE` de la recherche en N+1 (deux requêtes par discussion) | Une seule requête (messages agrégés par sous-requête corrélée), filtrage Unicode en Rust ; test existant |
| 5 | Historique | Tags non bornés côté serveur (nombre, longueur), ni normalisés | `normalise_tags` : trim, minuscules, `#` retiré, ≤ `HISTORY_TAG_MAX_CHARS`, ≤ `HISTORY_TAGS_MAX`, dédoublonnage (test) |
| 6 | Modèles de discussion | Nom et configuration non bornés (`validate`) | `TEMPLATE_NAME_MAX_CHARS`, `TEMPLATE_CONFIG_MAX_BYTES` (test) ; `maxLength` sur le champ nom |
| 7 | Exploitation | `read_backend_log` lisait tout le fichier du jour en mémoire | Lecture positionnée de la queue (`seek`) ; test existant |
| 8 | Scène | `hot_seat_target` indexait `order[0]` (panique sur un ordre vide en test forcé) | Retour `Option`, garde dans `materialise_scene_event`, égalité résolue au premier de l'ordre |
| 9 | Interventions | Boost de température inversé entre refus et réponse vide (préexistant v1.16) | Aligné sur l'intention des constantes (`TEMP_REFUSAL_BOOST` / `TEMP_DIFFICULTY_BOOST`) |
| 10 | Réactions | Différence de tally par soustraction non saturante | `ReactionTally::since` |
| 11 | Diagnostics | Conformité d'intention sensible à la casse (le banc ne l'était pas) | Comparaison insensible à la casse, alignée sur `bench_metrics` |
| 12 | Schéma | Neuf blocs `PRAGMA table_info` copiés-collés | Table `COLUMN_MIGRATIONS` + `has_column` ; **test de migration réelle** depuis une base v1.16 (colonnes, index plein texte rejoué, genre des messages dérivé du drapeau d'exclusion, tags/favori sur l'ancienne ligne) |
| 13 | Front — historique | Le nom de modèle persisté ignorait les surcharges par orateur (« mixte » affiché sur le résumé, pas dans l'historique) | `describeActiveModel(settings, overrides)` à la sauvegarde (test) |
| 14 | Front — modèles | `parseTemplateConfig` acceptait n'importe quel `discussionMode` et des types erronés (une configuration d'une autre version aurait fait échouer `start_discussion`) | Mode validé contre `DISCUSSION_MODES`, formats et options typés (test) |
| 15 | Front — export HTML | Le SVG des cartes (libellés écrits par le modèle, rendu markmap) était embarqué tel quel dans une page ouverte en `file://` | `sanitizeSvg` : scripts, gestionnaires `on*`, liens `javascript:` retirés (test) |
| 16 | Front — export | Échecs d'écriture signalés seulement en console | Toast + journal (`summary.exportFailed`, trois langues) |
| 17 | Front — vote d'Oxford | « Ne pas voter » laissait la fenêtre ouverte jusqu'au tour suivant | Fermeture locale immédiate |
| 18 | Front — réglages | Toast d'erreur à chaque visite si le compteur de souvenirs échoue (doublé en mode strict) | Compteur silencieux (journal), clé i18n retirée |
| 19 | Front — assistant | Champ « Nom du modèle » écrasé à 400 px | Largeur minimale |
| 20 | Intention (banc) | Le premier orateur d'une discussion recevait des cibles nominatives alors que personne n'avait parlé (l'annonce d'acte de la dramaturgie comptait comme « contexte ») : cibles inventées, jamais honorées, mesurées comme des manquements ; l'en-tête « tu es le premier à parler » ne s'affichait plus | Le contexte préalable ne compte que les contributions (`MessageKind::Normal`) ; sans contexte, aucune cible proposée (S8 et S10 réalignés, test de diagnostics réécrit sur trois tours) |

**Vérifié sans correction nécessaire** : masquage des clés (Debug des transports et des réglages), aucune interpolation SQL, FTS à paramètres liés, commandes Tauri sans chemin libre, `openExternalUrl` limité à http(s), KaTeX sans `trust`, budgets des nouveaux blocs (agenda, rôle, souvenirs) réservés par orateur, pré-vol de budget cloud partagé par `start_discussion` et le casting, quotas Tavily/Wikipédia inchangés, fenêtre de vote fermée sur `turnStarted` / `outcomeReady`, raccourcis inertes dans les champs, aucun `unwrap` hors tests dans le code ajouté, rendu à 400 px des pages Accueil / Réglages / Assistant (étape 1) / Historique.

**Banc de prompts v1.20** (GPU libéré en fin de revue) : campagne complète sur `mistral-small3.1` (Ollama, 900 s) archivée sous `Docs/Technique/bench/2026-09-16-v1.20-ollama-mistral-small3.1.{md,json}`. `node tools/bench-compare.mjs` contre la référence v1.17 : fuites Markdown 0, refus 0, erreurs 0, répétition stable (0,17–0,22), usage des noms 0,75 / 0,67 / **1,00** (socratique : 0,83 → 1,00), conformité d'intention 0,67 / **0,86** / 1,00 (idéation : 0,71 → 0,86), écart entre orateurs −7,3 s en débat (fin de tour parallèle et intention plus courte), +1,1 s en socratique (bruit : 6 interventions), longueur moyenne +12 à +36 % (blocs de mise en scène et fils ouverts). Une seule régression signalée : **conformité d'intention en débat 0,89 → 0,67**. Le banc a été instrumenté (`AIRENA_BENCH_SCENARIOS`, événements bruts par scénario) et le scénario rejoué seul (0,75) ; l'analyse des événements montre que les manquements viennent du **tour 1** : le premier orateur déclarait une cible (« relancer Le Philosophe ») alors que personne n'avait encore parlé — depuis la 1.18, l'annonce d'acte du modérateur est poussée dans les messages du tour et comptait comme « contexte préalable », ce qui offrait des cibles au premier orateur et supprimait l'en-tête « tu es le premier à parler ». Corrigé (défaut n° 20) : hors tour 1, les 9 interventions ciblées sont conformes à 8/9 (0,89), au niveau de la référence.

Mesures après revue : Rust 497 tests (3 ignorés), clippy 0, front 93 tests, i18n 1 284 clés × 3, `tsc` propre, `npm run build` OK. Ratchets relevés dans `CLAUDE.md` et `TECHNICAL.md`.

**Auto-évaluation de la revue** : chaque défaut listé est reproduit par un test ou une vérification empirique avant correction ; aucun point ouvert bloquant. Dette résiduelle connue et assumée (inchangée) : updater non câblé, listes manuelles §5.3 dans l'application, `PersonaEditor` à découper, banc DeepSeek jamais joué faute de clé.

### Lot 19 — Retours du premier build (v1.20.1)

Retours de l'utilisateur après le premier build 1.20 (six points) ; chaque point est traité, testé et documenté ; le sixième (profondeur des argumentaires) a été instrumenté et simulé sur le banc.

| # | Retour | Livré |
|---|---|---|
| 1 | Coulisses : beaucoup de chaînes tronquées | Les champs d'intention (angle, concession, question) étaient coupés à 160 / 120 caractères *avant* l'affichage, pour le budget du prompt. Désormais conservés en entier pour les coulisses (`INTENTION_DISPLAY_MAX_CHARS` = 600) ; le bloc « [Ton intention] » applique ses propres bornes au moment de construire le prompt (budget inchangé). Côté UI : ligne d'humeur des coulisses en retour à la ligne, bannière de scène sur deux lignes |
| 2 | Synthèse vocale : meilleure qualité, mais CPU modeste et tout dans le bundle | **Arbitrage demandé** (voir bilan) : la voix actuelle est celle du système (Web Speech / WebView2). Une voix neuronale embarquée (Piper/ONNX, licence MIT) coûte 100 à 300 Mo de modèles pour fr/en/zh et une chaîne de build lourde (espeak-ng, onnxruntime) ; rien n'est engagé sans décision |
| 3 | Vocal « suivre » : passage à l'orateur suivant déclenché manuellement | **Mode pas-à-pas** : quand la voix est active en mode « suivre », l'arène envoie `set_step_mode(true)` et le moteur attend le signal du public avant **chaque** orateur (`await_cue` : événement `AwaitingCue` juste avant `SpeakerActive`, libéré par `NextSpeaker`, par la sortie du mode, un arrêt ou la fermeture du canal ; les signaux reçus d'avance sont comptés). Pendant l'attente, pause, réactions du public, réglages d'émotion et demandes d'intervention restent servis. Bouton « Orateur suivant : … » et touche N. Test moteur à quatre scénarios (attente et signal, signaux d'avance, sortie du mode, mode éteint) |
| 4 | Couper / reprendre la voix et passer de « suivre » à « tout lire » dans la discussion | Barre de l'arène : pause / reprise de la voix (`SpeechEngine.paused`, la file continue de se remplir et `resume` la vide), bascule suivre ↔ tout lire (réglage durable `ttsMode`), en plus du muet existant |
| 5 | Panneau de droite limité en largeur | Plus de maximum fixe (640 px) : le panneau peut prendre toute la fenêtre moins la largeur minimale du fil (320 px) ; la largeur mémorisée est bornée à l'ouverture |
| 6 | Argumentaires de surface (carte rarement au-delà de 2–3) | Voir ci-dessous |

**Profondeur des argumentaires — ce que le banc a révélé d'abord.** Le dump des événements du scénario « débat » (v1.20, `mistral-small3.1`) ne contenait **aucun** `argumentMapUpdated` : les 8 appels d'extraction (4 tours × 2 essais) avaient échoué au parsing. Le banc a été instrumenté (`AIRENA_BENCH_TRACE`, `AIRENA_BENCH_TURNS`, réponse brute journalisée quand les deux essais échouent — `ARGMAP_RAW_LOG_MAX_CHARS`) : le modèle renvoie `"new_theses":[null]` ou des objets `{"text": …}`, un objet unique à la place du tableau `arguments`, `"type": null`, des noms avec espaces finaux. Un schéma strict perdait tout. **Correction** : parseur tolérant (`loose_text`, `loose_list`, `loose_argument` ; type déduit d'`against_thesis` quand il manque), test unitaire sur les formes réelles. Sur les modèles locaux, c'est probablement la cause première des cartes plates observées.

**Leviers de profondeur livrés** (aucun ne force un sujet ; tous réversibles par leurs constantes) :
- `ArgumentMap::unanswered_objections()` (contre-argument sans réponse d'un autre orateur ; débiteur = auteur de l'argument visé, ou de la thèse) et `depth_stats()` (profondeur max, part d'arguments à profondeur ≥ 2, objections ouvertes), émis dans `ArgumentMapUpdated.depth` et affichés par le panneau de la carte.
- Après chaque extraction, les nouvelles objections deviennent des **fils ouverts** (`OpenLoopKind::Objection`, un par débiteur et par tour, TTL habituel) : « Objection de X (tour n) : « … » — réponds sur le fond ou concède », dans le prompt d'intention et d'intervention.
- Acte de parole **`Deepen`** (« reprends l'objection la plus forte et réponds-y sur le fond : mécanisme, preuve, exemple »), pondéré `SPEECH_ACT_DEEPEN_BONUS` dans les modes `DiscussionMode::rewards_depth` (débat, procès, Oxford, critique, socratique, négociation, guidé), `SPEECH_ACT_DEEPEN_OWED_BONUS` quand une objection est due (elle est alors nommée dans la directive), nul en idéation, fiction, chapeaux, crise.
- **IArbitre** : critère de profondeur du débat (« si un orateur empile des affirmations sans répondre aux objections, demande en une phrase d'y répondre sur le fond — sans imposer de sujet ») et indice dynamique `depth_hint_for` nommant l'objection due par l'orateur qui vient de parler.
- **Extraction** : la liste des objections encore sans réponse est jointe au prompt d'extraction pour que la réponse d'un orateur soit rattachée à l'objection (`targets_argument`) plutôt que posée à plat sur la thèse (`ARGMAP_PROMPT_MAX_OBJECTIONS`).
- Banc : métriques `argmapMaxDepth`, `argmapDeepShare`, `argmapUnanswered`.

**Simulations (débat, 4 tours, `mistral-small3.1`, un passage chacune — n = 12 interventions, à lire comme des tendances)** :

| Passage | Carte | Prof. max | Part ≥ 2 | Objections ouvertes | Intention | Noms | Actes Deepen |
|---|---|---|---|---|---|---|---|
| v1.20 (parseur strict) | aucune (8 échecs) | — | — | — | 0,67 / 0,75 | 0,75 | — |
| parseur tolérant + fils d'objection + Deepen + indice IArbitre | 8 thèses, 16 arguments | 1 | 0,00 | 3 | 0,73 | 0,67 | 2 / 12 |
| + objections dans le prompt d'extraction | 6 thèses, 10 arguments | 2 | 0,10 | 0 | 0,89 | 0,83 | 2 / 12 |

Lecture honnête : les leviers fonctionnent (fils ouverts posés, acte tiré deux fois, indice transmis, conformité d'intention et usage des noms au niveau de la référence) mais la **profondeur mesurée reste bornée par l'extracteur** sur ce modèle local : la plupart des contre-arguments arrivent sans `against_thesis` (parqués « non rattachés », donc jamais comptés comme objections) et la profondeur 2 obtenue vient d'une preuve de l'objecteur sous sa propre objection, pas d'une réponse adverse. Deux essais d'extraction ont encore échoué au second passage (JSON cassé). La mesure de l'effet des leviers sur la profondeur réelle demande un extracteur plus fiable (DeepSeek, ou un modèle local plus fort) et plusieurs passages ; c'est la première dette du lot.

Mesures : Rust 502 tests (3 ignorés), clippy 0, front 94 tests, i18n 1 296 clés × 3, `tsc` propre, `npm run build` OK ; versions 1.20.1 ; docs (`CLAUDE.md`, `TECHNICAL.md` changelog v1.20.1, `FUNCTIONAL.md` §« Pas-à-pas vocal » et §« Profondeur des argumentaires », `README.md`) à jour.

Dette du lot : (1) rejouer le banc débat sur DeepSeek et sur plusieurs passages pour mesurer la profondeur ; (2) rattacher les contre-arguments sans thèse cible (heuristique à concevoir sans fausses attributions) ; (3) synthèse vocale neuronale embarquée — sur arbitrage.

### Lot 20 — Retours du deuxième build (v1.20.2)

**Analyse du journal d'exécution** (`target/release/logs/airena.log.2026-09-16`, dernière discussion `62615fe3`, débat 6 tours × 4 GladIAteurs sur DeepSeek, 241 appels, 575 k jetons de prompt, 0,12 USD ; carte : 9 thèses, 31 arguments, profondeur 3, part ≥ 2 = 0,19, aucun échec de parsing ; conformité d'intention 0,68). Constats factuels :

| Constat | Preuve | Réglage / correction |
|---|---|---|
| L'utilisateur n'est jamais pris pour interlocuteur | 24 intentions, aucune cible « Jey » ; 12 réactions reçues sur ses 3 messages ; dernier message « MAIS PERSONNE NE M'ECOUTE ICI ? » ; en mode piloté par les émotions, la directive disait « Ne t'adresse PAS à Jey qui n'est qu'un observateur » même après ses interventions | Le public devient participant dès sa première prise de parole : focus forcé sur lui pour l'orateur suivant, rappel « réponds-lui D'ABORD en le nommant », noms adressables, indice au modérateur si l'intervention l'ignore ; ensuite « a pris part au débat » et candidat au focus (test moteur) |
| L'utilisateur absent des représentations | `known_participant_names` = modérateur + GladIAteurs : ses thèses ne pouvaient entrer dans la carte ; scène, score, graphe limités au casting | Noms connus et id `user` dans l'extracteur et la fusion ; front : siège sur la scène, score et générique, nœud du graphe (`spokenParticipants`, test) |
| Le modérateur répète la même ouverture | 10 commentaires sur 24 interventions (42 % contre « ~80 % none » demandé), les 10 commencent par « MAIS ATTENDEZ ! Coup de théâtre, chers téléspectateurs » (tic du persona *L'Animateur TV*) | Bloc de style dans le prompt de modération : ouvertures récentes citées, tic au plus une fois sur trois, taux plafonné (`MODERATION_COMMENT_RATE_MAX_PERCENT` = 25 %) ; décisions journalisées ; les orateurs reçoivent le même rappel sur leurs ouvertures (18 interventions sur 24 s'ouvraient par « Nom, votre… » — le contrat d'intention exigeait le nom, désormais « pas forcément dès les premiers mots ») |
| Préchargement Ollama inutile | Au démarrage sous DeepSeek : « Unloaded 1 model(s) », « Preloading qwen3.8:27b » (17 Go de VRAM, 12 s) puis échec du préchargement de l'embedding (HTTP 400) ; recommandation de `num_ctx` (14 918) calculée pour un modèle qui ne sert pas | `startup_preload_plan` : modèle de chat seulement si Ollama sert la discussion ; sinon embeddings seuls, sans balayage VRAM ni recommandation (test) |
| Recherche web | 24 crédits Tavily autorisés et consommés en 6 tours (une recherche par orateur presque à chaque tour) | Réglage de l'utilisateur, laissé tel quel ; à surveiller sur le quota mensuel |
| Contre-arguments non rattachés | 3 avertissements « target unresolved — parked as unattached » | Inchangé (dette du Lot 19, extracteur) |

Mesures : Rust 505 tests (3 ignorés), clippy 0, front 95 tests, i18n 1 297 clés × 3, `tsc` propre ; versions 1.20.2 ; docs à jour. Le build 1.20.1 précédent avait été empaqueté pendant qu'une instance de l'application tournait (os error 32) : à refaire.

### Lot 21 — Réalisme : mécanismes rigides, répétitions, conscience du débat (v1.20.3)

Retours de l'utilisateur : « La salle a une question pour L'Expert IA : réponds… à la question la plus gênante » répété deux fois dans une discussion et sans lien avec le sujet ; chaque intervenant doit connaître la personnalité des autres ; réduire les mécanismes rigides et les répétitions, améliorer réalisme, pertinence et conscience globale ; phrases du générique tronquées.

| Mécanisme rigide | Avant | Livré |
|---|---|---|
| Annonces d'actes et d'événements | Chaînes fixes identiques d'une discussion à l'autre, sans la voix du persona | Un appel court `Announcement` : le modérateur dit l'annonce avec sa voix, briefé par le gabarit, ses ouvertures récentes interdites ; gabarit en secours (échec, refus, vide, annulation) ; test moteur (annonces = texte du modérateur, brief et ouvertures dans l'appel, repli sur le gabarit quand l'appel échoue) |
| Question de la salle | Formule générique (« la question la plus gênante… »), pouvait revenir plusieurs fois | Question écrite par le modérateur depuis le sujet, le résumé, la position et les fils ouverts de la cible (`AudienceQuestion`, JSON, ≥ 12 caractères, ≤ 240) ; portée par l'événement, l'annonce et la consigne de la cible ; aucune question exploitable → pas d'événement ; test |
| Types d'événements | Tirage uniforme, un type pouvait se répéter | Un type déjà joué n'est retiré que lorsque tous les autres disponibles l'ont été (`used_kinds`, test) |
| Conscience des autres | Les orateurs ne connaissaient des autres que leur nom et leurs propos | `engine/cast.rs` : portrait (rôle, credo, registre) lu dans le kernel de chaque persona, bloc « [Les autres participants — qui ils sont] » dans le prompt système de chaque orateur, « [Ton plateau — qui ils sont] » dans celui du modérateur (introduction, modération, synthèse, annonces) ; réservé au budget (`CAST_BLOCK_MAX_CHARS`) ; tests unitaires et moteur |
| Conscience du débat | La carte des arguments ne servait qu'à l'extraction et aux fils d'objection | Section de budget `DebateState` (rang 12, documents 13-14, active avec la carte, ajoutée aux ordres sauvegardés) : « [État du débat] » — thèses les plus argumentées avec auteur et objections ouvertes, dernières objections sans réponse, consigne « prolonge, réponds ou déplace plutôt que redire » ; test moteur |
| Générique | Phrases primées tronquées à 90 caractères et coupées par le CSS | 280 caractères, affichage complet |

Incident de séance, consigné : une normalisation des continuations de chaînes dans `prompt_builder.rs` a retiré l'indentation de tout le fichier ; le fichier a été reformaté avec rustfmt puis ré-indenté à la main dans les zones que rustfmt ne traite pas ; les chaînes à l'exécution sont inchangées (508 tests verts avant et après), seule la mise en forme du source a changé.

Mesures : Rust 511 tests (3 ignorés), clippy 0, front 95 tests, i18n 1 301 clés × 3, `tsc` propre ; fixture d'événements régénérée ; versions 1.20.3 ; docs à jour. Coût : une annonce ≈ 150 jetons de sortie par acte ou événement (≤ 8 par discussion), une question de la salle ≈ 200 ; sur Ollama, 2 à 5 s de plus par annonce.

### Lot 22 — Émotions réalistes, réactions sincères, phrases entières (v1.20.4)

Retours de l'utilisateur, sur les journaux de la dernière exécution (débat 6 tours, DeepSeek) : émotions saturées près de 100 % très vite — améliorer quantification, variabilité et impact sur le raisonnement et l'expression ; encore des troncatures sur les messages inter-tour (bulles bleues) et les descriptions des interventions ; la distribution systématique de 💡 à chaque intervenant est inutile, les réactions doivent être utiles, sincères et exprimer une opinion.

Diagnostic (base d'historique copiée avec son `-wal`) : pour les quatre gladiateurs, confiance à 100 dès le tour 1-2, engagement / accord / enthousiasme à 99-100 au tour 5-6, toutes les relations « alliés » ; 56 réactions sur 72 étaient 💡 (78 %), trois intervenants n'ont donné **que** des 💡 (17-18 chacun), les justifications étaient des compliments même en cas de désaccord (« … même s'il esquive la question de l'alignement »). Causes : effet additif (trois 💡 polis = +29 de confiance par intervention), accord donné +12 par tour, aucune homéostasie sur quatre axes, deltas LLM presque toujours positifs, contagion vers une moyenne haute ; l'exemple du prompt de réaction ancrait le modèle sur `"insightful"` et le sens « un point fort, sur lequel bâtir » en faisait la réponse polie par défaut.

| Sujet | Avant | Livré |
|---|---|---|
| Quantification | Deltas additifs bornés à 100, saturation en deux tours | `AxisDeltas` : rendements décroissants harmoniques par type, plafond de ronde par axe (12 bruts) **suivant les gains** du persona (`apply_felt` — l'or S22 « neutre = brut » est conservé, un névrosé encaisse toujours plus), résistance élastique près des extrêmes (plein effet dans la bande de confort, 15 % au bord, retour gratuit), deltas de l'analyste élastiques ; le prompt de l'analyste attend autant de baisses que de hausses ; `EmotionalProfile::apply_delta` supprimé (code mort) |
| Variabilité | Décroissance seulement sur frustration et enthousiasme | Homéostasie sur les six axes vers le profil initial (12 % de la distance, un point au moins) ; accord donné facteur 2 / plafond 6, symétrique ; enthousiasme rappelé à 2 |
| Impact sur l'expression | Couche 1 sur niveaux absolus (70/30) : avec la saturation, tout le monde « en confiance, curieux, enthousiaste » à chaque tour | Mouvements depuis le profil initial (`dominant_shift`, ≥ 15) : ligne de mouvement dans la directive (ébranlé, rapproché, durci, happé, refroidi, apaisé ; jamais en double d'un état) ; entrée dans la zone notable = franchissement de seuil (flash, frise, didascalie) à côté des seuils absolus 85/15 devenus rares |
| Impact sur le raisonnement | Déclencheurs : frustration > 70, contradiction, fin proche | + orateur ébranlé (confiance en baisse notable) : déclencheur fort (`THINK_SHAKEN_BOOST`) |
| Réactions sincères | Exemple ancré sur `insightful`, sens « sur lequel bâtir », aucune règle | Exemple tournant avec le contenu, jamais `insightful`, « none » en premier ; règles de sincérité trilingues (opinion et non politesse, 💡 se mérite et reste rare, le désaccord se dit avec les couleurs permises par le mode, justification à la première personne) ; indulgence = « like » ; tests (rotation, fiction sans dislike) |
| Troncatures | Annonces coupées au mot à 420 caractères (160 jetons), didascalies à 120, faits à 300 caractères bruts | `truncate_at_sentence_boundary` (phrases entières, repli au mot) : annonces 900 / 220 jetons, didascalies 240, faits surprenants 500 ; tests |

Tests : les tests unitaires à nombres exacts du moteur émotionnel deviennent des tests de propriétés (rendements décroissants, résistance et bornes, deltas élastiques, homéostasie, mouvement dominant, franchissements uniques) ; tests moteur de stagnation réécrits en propriétés (pénalité nette de l'homéostasie) ; S22 et le chemin doré OCEAN neutre inchangés ; test de la ligne de mouvement (états, doublon interdit, EN/ZH) ; test du prompt de réaction ; test de troncature par phrases (latin, CJK, coupure par le quota de jetons) ; fixture d'événements régénérée (les didascalies et les franchissements y viennent désormais des mouvements).

Mesures : Rust 517 tests (3 ignorés), clippy 0, front 95 tests, i18n 1 301 clés × 3, `tsc` propre ; versions 1.20.4 ; docs à jour. Coût : règles de sincérité ≈ 120 jetons d'entrée par appel de réaction ; ligne de mouvement ≈ 40 jetons quand elle s'applique ; annonces +60 jetons de sortie autorisés. À vérifier sur la prochaine exécution réelle : trajectoires émotionnelles (attendu : 35-80 avec des allers-retours), part des 💡 (attendu : < 30 %), présence de 👎 / ❓ entre adversaires.

### Repasse d'équilibrage — analyses, tests et simulations (v1.20.5)

Demande de l'utilisateur : après les déséquilibres et bugs identifiés au fil des lots, une repasse d'analyses approfondies, de tests et de simulations pour valider que tout est fonctionnel, équilibré et paramétré de manière optimale.

**Méthode.** (1) Relecture à froid des chemins émotionnels du moteur (réception, réactions données, analyste, contagion, modérateur, public) et des seuils consommateurs (directives 70/30, résumé d'état, alertes 85/15, ambiance de salle 65/35/65, raisonnement 70, échantillonnage). (2) Un harnais de simulation déterministe (`engine/emotion_sim.rs`) qui rejoue la chaîne du moteur sans LLM sur huit tours et affirme des propriétés d'équilibre. (3) Le banc sur modèle réel (`mistral-small3.1`) enrichi des métriques manquantes : réactions par intervention, part des 💡, part critique (👎 ❓ ↩️), pic émotionnel, part de photos saturées.

**Déséquilibres trouvés et corrigés.**

| # | Constat | Origine | Correction |
|---|---|---|---|
| 1 | Battement des didascalies : un orateur oscillant autour de 15 points d'écart (l'homéostasie ramène à 13, une réaction repousse à 16) redéclenchait un franchissement — et une didascalie — presque à chaque intervention | `detect_shift_crossings` sans état (v1.20.4) | `ShiftZones` : occupation de zone par orateur et par axe, hystérésis (`EMOTION_SHIFT_REARM` = 5 : on ne quitte la zone qu'en deçà de 10) ; test unitaire (oscillation, sortie, bascule opposée, orateurs séparés) |
| 2 | Le modérateur n'avait aucune homéostasie : deltas de l'analyste (souvent positifs) et bans s'accumulaient sans rappel — la dérive monotone reprochée aux gladiateurs | `update_arbitre_emotions` (bans + stagnation seulement) | Première tentative : homéostasie à chaque modération → sur-amortissement (trois orateurs = trois rappels et trois pénalités de stagnation par tour, le modérateur finissait à 41 dans le test réaliste). Livré : `settle_arbitre_emotions` **une fois par tour** avant la contagion (homéostasie vers le profil neutre + pénalité de stagnation des gladiateurs) ; la modération ne garde que le delta de ban ; `EMOTION_ARBITRE_STAGNATION_ENG` supprimée |
| 3 | Frustration et enthousiasme n'avaient qu'un rappel fixe de 2 points : sous pression constante ils grimpaient encore vers 95-97 (l'équilibre est fixé par la forme de la résistance, pas par le rappel) | v1.16 conservé en v1.20.4 | Homéostasie unifiée : la part proportionnelle (12 %) s'applique à tous les axes, le taux explicite devient un plancher |
| 4 | Résistance linéaire trop tardive : à 85 elle laissait encore passer 51 % d'un delta ; plateau à 88 sous un chœur unanime, 76 sous une foule hostile | v1.20.4 | Freinage **quadratique** dans la distance à la bande de confort (58 % à 75, 31 % à 85, 17 % à 95) : plateau 82 / 73 |
| 5 | Le test S28 partait de 80/20 et comptait sur ±10 bruts pour franchir 85/15 en un tour | test écrit sur le modèle additif | Point de départ 83/17 ; le comportement testé (une didascalie par orateur et par tour) est inchangé |

**Simulations (huit tours, quatre orateurs neutres ; une intervention = une ronde de réactions + dérive ; fin de tour = analyste + contagion).**

| Scénario | Avant repasse (v1.20.4) | Après (v1.20.5) | Propriétés vérifiées |
|---|---|---|---|
| Chœur poli (3 💡 par intervention, accord +3 donné, analyste +3/+2/+3/+2) | confiance 50 → 63 → 74 → 81 → 85 → 87 → 88 → 88 → 88 ; 1 franchissement absolu par orateur | confiance 50 → 63 → 74 → 79 → 80 → 81 → 82 → 82 → 82 ; engagement → 79, accord → 76, enthousiasme → 80 ; 0 franchissement absolu, 4 mouvements (un par axe, une fois) | pic ≤ 88, 80 atteint après deux tours au plus tôt, éloge ressenti (≥ 75), théâtre ≤ 6 et ≤ 2 par axe |
| Foule hostile (3 👎 par intervention, accord −2, analyste −2/−3/+3/−2) | frustration 10 → 23 → 35 → 45 → 54 → 62 → 69 → 74 → 76 ; confiance → 18 | frustration 10 → 23 → 35 → 45 → 54 → 62 → 69 → 72 → 73 ; confiance → 21 ; accord → 27 ; 4 mouvements par orateur (frustration ↑, confiance ↓, accord ↓, engagement ↑ — la contradiction accroche) | ressenti en trois tours (≥ 40), pic ≤ 82, plancher ≥ 18, plateau (< 5 points sur les deux derniers tours), théâtre ≥ 1 et borné |
| Réactions sincères (surtout aucune, une opinion tous les 3-4 pas, analyste alterné, stagnation un tour sur quatre) | — | confiance 50 → 52 → 48 → 46 → 41 → 52 → 53 → 54 → 50 ; frustration 10-22 ; engagement 48-57 ; 0 mouvement, 1-2 franchissements « frustration basse » | axes 30-75, ≥ 3 changements de direction de la confiance, théâtre ≤ 3 |
| Sensibilité OCEAN (même orage : N=9 / neutre / N=3 ; même éloge : A=9 / neutre) | — | névrosé ≥ neutre ≥ stable à chaque pas, écart final ≥ 5 ; agréable > neutre en confiance | les gains survivent aux plafonds et à la résistance |
| Récupération (4 tours hostiles puis 6 calmes) | — | frustration et confiance regagnent au moins la moitié de l'écart ; la rancune ne disparaît pas | mémoire sans blocage |
| Persona extrême (confiance 85, frustration 60, enthousiasme 30 ; 6 tours de chœur puis 6 calmes) | — | confiance ≤ 95 puis retour à 85 ± 3 (jamais vers 50) ; frustration ≥ 55 ; enthousiasme revenu à mi-chemin | le tempérament propre est la ligne de base |
| Modérateur (analyste +4 engagement / +3 confiance pendant 12 tours) | brut 98 / 86 | 70 / 64 (équilibre) | borné, encore ressenti (≥ 58) |

**Paramètres revus sans changement** (justifiés par les trajectoires) : seuils de directive 70/30 — atteignables en deux rondes de soutien et abandonnés dès un tour sans soutien ; alertes 85/15 — désormais rares, réservées aux vrais extrêmes (les mouvements prennent le relais pour le théâtre) ; ambiance de salle (tendue > 65 de frustration moyenne, vive > 65 d'enthousiasme) — atteignable sous orage / soutien soutenus, sereine par défaut ; raisonnement (frustration > 70) — atteint sous foule hostile, plus par un persona nerveux ; échantillonnage (température ± 0,15 × enthousiasme, longueur 0,8-1,2 × engagement) — impact typique ± 0,06 / ± 6 %, présent sans dominer ; réactions du public — appliquées une par une (au plus trois par message, +5 de confiance chacune) : c'est un acte délibéré de l'utilisateur, laissé tel quel ; relations — alliés à partir de deux approbations mutuelles nettes avec décroissance 0,85 : l'entrée (💡 systématiques) était en cause, pas la classification.

**Bancs sur modèles réels.** Le banc a d'abord dû être outillé : métriques de sincérité et de saturation (réactions par intervention, part 💡, part critique 👎 ❓ ↩️, pic émotionnel, part de photos à ≥ 95), passes multiples moyennées (`AIRENA_BENCH_RUNS`), rejeu d'une passe sauvegardée (`bench_replay`). Un **artefact de mesure** est apparu à la première comparaison : la conformité d'intention et l'usage des noms exigeaient le nom avec son article (« Le Créatif ») alors que la variété de forme de 1.20.2 pousse à « Créatif, tu… » — d'où de fausses régressions (idéation 0,86 → 0,43). Corrigé par `json_parser::mentions_name` (article facultatif), partagé par le diagnostic du moteur et le banc ; les passes ont été rejouées avec la métrique corrigée.

| Scénario | Modèle | Passes | Répétition | Noms | Intention | Réactions / interv. | Part 💡 | Part critique | Pic émotionnel | Part saturée | Écart orateurs |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Débat | référence v1.20, `mistral-small3.1` | 1 | 0,18 | 0,75 | 0,67 | — | — | — | — | — | 37,0 s |
| Débat | `mistral-small3.1` | 1 | 0,18 | 0,83 | 0,90 | 1,58 | **0,11** | 0,58 | 84 | **0 %** | 30,7 s |
| Débat | `deepseek-flash` | 3 (moyenne) | 0,15 | 0,92 | 0,85 (0,64-1,00) | 2,00 | **0,36** | 0,36 | 86 | **0 %** | 12,8 s |
| Idéation | référence v1.20 | 1 | 0,17 | 0,67 | 0,86 | — | — | — | — | — | 21,3 s |
| Idéation | `mistral-small3.1` | 1 | 0,18 | 0,67 | 0,71 | 1,67 | 0,20 | 0,27 | 87 | 0 % | 23,7 s |
| Idéation | `deepseek-flash` | 3 | 0,16 | 0,85 | 0,83 | 2,00 | 0,15 | 0,37 | 88 | 0 % | 11,6 s |
| Socratique | référence v1.20 | 1 | 0,22 | 1,00 | 1,00 | — | — | — | — | — | — |
| Socratique | `mistral-small3.1` | 1 | 0,18 | 0,67 | 0,80 | 0,67 | 0,00 | 0,75 | 79 | 0 % | 14,6 s |
| Socratique | `deepseek-flash` | 3 | 0,17 | 0,72 (0,50-0,83) | 0,80 (0,60-1,00) | 1,00 | 0,33 | 0,50 | 83 | 0 % | 11,8 s |

Lecture. (1) **Plus de saturation** sur aucun modèle : pic 79-88, 0 % de photos à ≥ 95 (le débat réel de référence était à 100 dès le tour 2). (2) **Réactions sincères** : sur `mistral-small3.1` la part des 💡 tombe à 0-20 % avec 27-75 % de réactions critiques ; sur `deepseek-flash` les justifications sont de vraies opinions (« Ce qui me dérangeait comme une indigence est en réalité une vertu épistémique que je n'avais pas su nommer »), mais le modèle réagit à **toutes** les interventions (2,00 sur 2 réacteurs possibles, jamais « none ») et garde 36 % de 💡 en débat pour 1 seul 👎 sur 72 — 61 % de réactions positives à 100 % de taux de réaction. (3) Conséquence sur les trajectoires DeepSeek (4 tours) : bornées et différenciées (frustration 14-36, accord 20-54 selon la persona) mais la confiance de tous monte encore de façon monotone de 65-70 à 82-85 : le modèle numérique fait ce que l'entrée lui dit. Leviers essayés puis retenus : voir les quatre campagnes DeepSeek ci-dessous. (4) Répétition, fuites Markdown, refus, erreurs : inchangés (0,15-0,18 ; 0 ; 0 ; 0). (5) **Variance** : d'une passe à l'autre, l'intention en débat va de 0,64 à 1,00 et les noms en socratique de 0,50 à 0,83 sur 6 interventions — toute conclusion demande au moins trois passes ; les « régressions » d'un `bench-compare` sur une passe unique se lisent avec cette réserve. Références archivées : `Docs/Technique/bench/2026-09-17-v1.20.5-ollama-mistral-small3.1.*` et `…-deepseek-flash-x3.*`.

**Quatre campagnes DeepSeek Flash sur le débat (3 passes moyennées chacune, 12 interventions, 2 réacteurs possibles par intervention).**

| Campagne | Changement testé | Réactions / interv. | Part 💡 | Part critique | Noms | Intention | Pic | Verdict |
|---|---|---|---|---|---|---|---|---|
| x3 | règles de sincérité qualitatives (v1.20.4) | 2,00 | 0,36 | 0,36 | 0,92 | 0,85 | 86 | réagit à tout, 💡 fréquent |
| x3b | + ancres quantitatives (« none ≈ une sur deux », « 💡 ≤ une sur cinq ») | 2,00 | 0,43 | 0,35 | 0,81 | 0,82 | 86 | aucun effet : le modèle ne se calibre pas sur des nombres |
| x3c | + exemple JSON « none » une fois sur deux, « — ou "none" » dans la consigne, propension des extravertis assouplie | 2,00 | 0,40 | 0,42 | 0,89 | 0,88 | 85 | aucun effet non plus |
| x3d | + champ `"reacts"` décidé en premier, **crédit 💡** (un par cinq réactions données, dit dans le prompt, excédent enregistré 👍) | 2,00 | **0,14** | **0,75** | 0,86 | 0,88 | 86 | 💡 divisé par trois ; le modèle bascule sur ❓ (16/24) et 👎 quand le crédit est épuisé ; `reacts: false` jamais choisi |

Trajectoires x3d (4 tours, passe 2) : confiance du Scientifique 70 → 79 → 74 → 76 → 69, du Philosophe 65 → 65 → 67 → 66 → 62 ; frustration 20 → 30 et 15 → 20 ; accord de l'Avocat du diable 20-24 — les axes montent **et descendent**, la montée monotone de la confiance (x3 : tous à 82-85 au tour 4) a disparu ; pic 86, saturation 0 %, 3 à 5 didascalies par débat.

Conclusions. (a) Sur `deepseek-flash`, la formulation seule (ancres, exemple) n'agit pas sur la fréquence ni sur la couleur des réactions ; un **état de prompt piloté par le moteur** (le crédit épuisé, comme l'anti-répétition des actes de parole) agit immédiatement. Le crédit est honnête : le modèle est prévenu avant de répondre, et un 💡 excédentaire reste une approbation (👍) avec sa justification. (b) Le champ `reacts` est conservé (tolérant, inoffensif, utile aux modèles qui savent se taire — `mistral-small3.1` était déjà à 1,58 / 2 réactions par intervention sans lui) ; sur DeepSeek Flash la réaction systématique est une **limite connue** du modèle, compensée par la couleur honnête des réactions. (c) Les mécanismes livrés restent tous invisibles pour l'utilisateur et sans magie de chiffres arbitraires : une constante (`INSIGHTFUL_CREDIT_WINDOW`), un état par intervenant, une ligne de prompt.

**Effet de bord détecté à l'usage (relations figées).** Après le build 1.20.5, l'utilisateur observe des relations « beaucoup moins réactives, principalement neutres après 4-5 tours ». Analyse du dernier débat (6 tours, 72 réactions — 33 ❓, 15 👍, 14 👎, 10 💡 : la sincérité fonctionne) : **zéro** changement de relation. Rejeu des scores décroissants du moteur : Singularité → Expert +2,81 / retour +1,20 (8 approbations pour 1 désapprobation), Dieu → Boomer +3,53 / +0,20, Boomer → Singularité −1,96 / −0,44, Boomer ↔ Expert −1,23 / −1,06 — aucune paire classée, parce que la classification v1.17 exigeait **2,0 dans chaque sens** : calibrée pour le régime « 💡 partout » (deux approbations par tour et par sens), elle ne se déclenche plus quand la moitié des réactions sont des questions (neutres). Correction : classification sur la **somme** des deux sens (≥ 2,5), chaque sens penchant du bon côté (≥ 0,8), rivalité seulement sans approbation fraîche (un mot aimable la fait passer en tension — la réconciliation v1.17 est conservée, testée), tension aussi quand un seul sens est froid de 2,0 (critique persistant, même ignoré), et consigne de couche 2 qui nomme le critique (`RelationshipLean`). Rejeu des deux derniers débats réels : 14 changements (au lieu de 0) et 8 (au lieu de 3) ; état final du dernier : Singularité–Expert alliés, Boomer–Expert rivaux, Boomer–Singularité tendus. Réserve : une paire au voisinage du seuil peut basculer deux fois dans un tour (Dieu–Expert, tour 5) — accepté, une hystérésis de classification rendrait `classify_pair` stateful pour un cas rare.

**Tests.** Rust 530 (5 ignorés dont `emotion_sim_report` et `bench_replay`), clippy 0, front 95, i18n 1 301 × 3, `tsc` propre ; fixture d'événements régénérée ; versions 1.20.5 ; références de banc archivées (`2026-09-17-v1.20.5-ollama-mistral-small3.1`, `…-deepseek-flash-x3`, `…-x3b-anchors`, `…-x3c-none-example`, `…-x3d-reacts-credit`).
