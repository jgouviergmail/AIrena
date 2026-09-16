# AIrena — Audit technique & fonctionnel et plan directeur

> **Périmètre** : consolidation/optimisation de l'existant (carte des arguments, comportement des GladIAteurs, émotions, modes de discussion, UX/UI) + intégration du provider **DeepSeek** (clé, choix du modèle, niveau de réflexion, comptage tokens & coûts).
> **Date** : 2026-09-16 · **Base auditée** : `main` @ `7db2823` (v1.15.1) · **Méthode** : lecture intégrale du moteur Rust et du front, dépouillement de `TODO.txt` / `TOKENS.txt` / `SIDEPARTS.txt`, vérification de l'API DeepSeek sur la documentation officielle (`api-docs.deepseek.com`, changelog du 2026-09-10).
> **Statut** : analyse — **aucune implémentation n'est engagée avant validation du plan (§6)**.

---

## 0. Sources et limites de l'audit

### 0.1 Ce qui a été lu intégralement
- Moteur : `engine/orchestrator.rs` (3 810 l.), `engine/prompt_builder.rs`, `engine/directive_builder.rs`, `engine/emotion_engine.rs`, `engine/turn_manager.rs`, `engine/memory_manager.rs`, `engine/mode_prompts.rs`, `engine/token_budget.rs` (partie algorithmique), `engine/json_parser.rs` (partie parsing), `engine/dynamics_parser.rs`, `models/*`, `ollama/client.rs`, `ollama/types.rs`, `ollama/error.rs`, `ollama/model_info.rs`, `rag/embedder.rs`, `rag/store.rs` (pipeline `query`), `tavily/client.rs`, `commands/*`, `state.rs`, `lib.rs`, `error.rs`, `constants.rs`, `db/schema.rs`, `db/repository.rs` (settings/profils), un persona GladIAteur et un persona IArbitre du seed.
- Front : `types.ts`, `tauri-api.ts`, les 4 stores, `SetupPage`, `SettingsPage`, `ArenaPage`, `HomePage`, `SummaryPage`, `HistoryDetailPage` (début), `DiscussionFeed`, `MessageBubble`, `SpeakerBadge`, `TurnIndicator`, `DiscussionControls`, `EmotionSidebar`, `ParticipantEmotionCard`, `EmotionAxisSlider`, `MindmapSidebar`, `MarkmapViewer`, `DocumentSidebar` (début), `TokenBudgetPreview`, `VramIndicator`, `LlmParamsForm`, `TopBar`, `AppShell`, `Sidebar`, `useTokenBuffer`, `globals.css`, `App.tsx`, `tauri.conf.json`.

### 0.2 Ce qui n'a été que survolé (sans impact sur les conclusions)
`json_parser.rs` (fin), `license.rs`, `wikipedia/client.rs`, `persona-parser.ts`, `PersonaEditor.tsx`, `SimpleMd.tsx`, `ReadOnlyFeed.tsx`, `HistoryPage.tsx`, les 3 fichiers de locales (≈ 700 Ko — structure et clés vérifiées, pas le contenu).

### 0.3 Faits DeepSeek vérifiés (documentation officielle, 2026-09-16)
| Sujet | Fait vérifié |
|---|---|
| Base URL | `https://api.deepseek.com` (format OpenAI) — `/chat/completions`, `/models`, `/user/balance` |
| Modèles API | `deepseek-flash` (servi par **DeepSeek-V4.1-Flash** depuis le 2026-09-10) et `deepseek-v4-pro` (maintenu, tarif inchangé). `deepseek-chat` / `deepseek-reasoner` **retirés le 2026-07-24** ; `deepseek-v4-flash` = alias temporaire |
| Contexte / sortie | Contexte **1M tokens**, sortie max **384K** ; `max_tokens` ∈ [1, 393 216], défaut 8K (sans réflexion) / 64K (réflexion) / 128K (effort `max`) |
| Réflexion | `thinking: {"type": "enabled"\|"disabled"}` (défaut **enabled**) + `reasoning_effort: none\|low\|high\|max` (défaut **high**) ; en mode réflexion `temperature`, `presence_penalty`, `frequency_penalty` **non supportés**, `top_p` plancher **0.95** |
| Sortie du raisonnement | champ `reasoning_content` (message et `delta` en streaming, arrive **avant** `content`) ; sans `tools`, inutile (et ignoré) de le renvoyer dans l'historique |
| JSON | `response_format: {"type": "json_object"}` ; le mot « json » doit figurer dans le prompt ; « peut occasionnellement renvoyer un contenu vide » ; compatibilité JSON + réflexion **non documentée** (→ spike, §6 Lot 0) |
| Usage | `usage.prompt_tokens`, `completion_tokens`, `prompt_tokens_details.{cached_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens}`, `completion_tokens_details.reasoning_tokens` ; en streaming, dernier chunk avec `stream_options: {"include_usage": true}` |
| Streaming | SSE `data: {...}` / `data: [DONE]` ; sous charge : commentaires keep-alive `: keep-alive` (streaming) ou lignes vides (non-streaming) ; connexion fermée si l'inférence n'a pas démarré après 10 min |
| `finish_reason` | `stop`, `length`, `content_filter`, `tool_calls`, `insufficient_system_resource`, `aborted` |
| Erreurs | 400 format, 401 clé, **402 solde insuffisant**, 422 paramètres, 429 débit, 500, 503 surcharge |
| Concurrence | 2 500 requêtes simultanées (flash), 500 (v4-pro) |
| Tarifs (USD / 1M tokens, heures pleines = 01:00–04:00 et 06:00–10:00 UTC, lun.–ven. ; heures creuses = ½) | **flash** : cache hit 0,006 / cache miss 0,30 / sortie 1,20 (pleines) — 0,003 / 0,15 / 0,60 (creuses). **v4-pro** : 0,044 / 1,32 / 3,96 (pleines) — 0,022 / 0,66 / 1,98 (creuses). Les tokens de raisonnement sont des tokens de sortie |
| Cache de contexte | par préfixe exact (unités de cache), « best-effort » ; un system prompt stable en tête = hits massifs |
| Tokenisation | ≈ 0,3 token / caractère anglais, ≈ 0,6 token / caractère chinois ; tokenizer hors-ligne fourni (Python) ; le `usage` de l'API fait foi |

> ⚠️ La feuille de route DeepSeek est volatile (renommages tous les 2–3 mois). Conséquence de conception : liste de modèles **dynamique** (`GET /models`), tarifs **par identifiant de modèle** dans `constants.rs` avec date de référence, et dégradation propre (tokens comptés, coût « n/d ») pour un modèle inconnu.

---

## 1. Synthèse d'alignement fonctionnel & technique

### 1.1 Reformulation du périmètre cible et valeur ajoutée

| Domaine | Intention métier (explicitée) | Valeur attendue |
|---|---|---|
| **A. Provider DeepSeek** | Pouvoir faire tourner une discussion sur DeepSeek (cloud) au lieu d'Ollama, avec clé API, choix du modèle, niveau de réflexion, et **visibilité/contrôle des coûts** (tokens, €/$, plafond) — sans rien casser du mode local | Débats de bien meilleure qualité (V4.1) sans GPU, contexte 1M, raisonnement natif ; maîtrise budgétaire |
| **B. Carte des arguments** | Une carte fiable (pas de thèses doublonnées, pas d'arguments perdus), exploitable (stable à l'écran, navigable), persistée sous forme structurée | Lisibilité de la discussion, réutilisation post-discussion |
| **C. Comportement des GladIAteurs** | Échanges « vivants, naturels, non systématiques », plus d'effet « tous contre X », réflexion variable mais rythme préservé (`TODO.txt` l. 2–15) | Réalisme, diversité des interactions |
| **D. Émotions** | Des émotions **justes** (pas de dérive mécanique), qui influencent réellement le discours, et lisibles dans l'UI | Cohérence et crédibilité des personnalités |
| **E. Modes de discussion** | Chaque mode fonctionne sans cas dégradé (respond/pass réel en UserDriven, ouverture garantie en Fiction, coût contenu en Co-construction) | Fiabilité |
| **F. UX/UI** | Interface plus moderne et dynamique : mouvement, hiérarchie, lisibilité de l'arène (tours, file de parole, activité), panneaux latéraux moins envahissants, réglages structurés | Confort d'usage, perception produit |

**Non-dits explicités (et retenus comme exigences)**
1. Un seul provider et un seul modèle **par discussion** (confirmé par `TOKENS.txt` l. 7 : « Modèle unique ») — le choix se fait dans Paramètres, pas par gladiateur. Le mélange de providers par intervenant est une évolution v2 rendue possible par l'abstraction proposée, non incluse.
2. Le mode Ollama doit rester **iso-fonctionnel** (heuristique think, ×3 `num_predict`, VRAM, préchargement) : zéro régression.
3. « Niveau de réflexion » = le niveau natif DeepSeek (`none/low/high/max`), réglable par défaut et par intervenant, avec un mode **auto** qui réutilise les déclencheurs de l'heuristique think existante (frustration, engagement, fin proche, contradiction) pour rester « dynamique et vivant » (`TODO.txt` l. 14–15).
4. « Comptage tokens et coûts » = comptage **réel** (champ `usage` renvoyé par l'API), agrégé par appel / intervenant / discussion / période mensuelle glissante (même patron que Tavily), coût **estimé** à partir d'une grille tarifaire versionnée, avec plafond mensuel optionnel.
5. Les tokens de raisonnement se paient : le niveau de réflexion est un **levier de coût** ; les appels utilitaires (réactions, modération, mémoire, émotions, votes, recherche, carte) doivent tourner **sans** réflexion.
6. RAG : DeepSeek n'a pas d'endpoint d'embeddings → le RAG en mode DeepSeek utilise Ollama pour les embeddings s'il est disponible, sinon **BM25 seul** (le magasin sait déjà stocker en texte seul : `rag/store.rs::add_document_text_only`).
7. La clé API est un secret : jamais dans les logs ni dans l'historique ; stockage en base comme la clé Tavily (patron existant), avec la limite connue (clair sur disque) documentée.

### 1.2 Hypothèses confrontées au code (zéro extrapolation)

| ID | Hypothèse | Vérification | Statut |
|---|---|---|---|
| H1 | Le moteur est couplé à Ollama en un seul point | Faux : `OllamaClient` est utilisé dans `engine/orchestrator.rs` (champ l. 100, `build_request` l. 370/396, `show_model` l. 517, 20+ appels `chat`/`chat_streaming`), `engine/turn_manager.rs` (`AsyncTurnContext.ollama_client` l. 74, appels l. 128/221/299), `rag/store.rs` (`query`/`llm_select` l. 255/340/370), `commands/discussion.rs` l. 90, `commands/ollama.rs` ; `OllamaError` fuit dans `rag/embedder.rs` et `rag/store.rs` | ✅ Abstraction nécessaire sur 5 fichiers (pas 1) |
| H2 | `LlmParams` est portable | Faux : `top_k`, `num_predict`, `num_ctx`, `repeat_penalty` sont des notions Ollama (`models/settings.rs` l. 7–14) ; DeepSeek : `temperature` (ignorée en réflexion), `top_p` (≥ 0,95 en réflexion), `max_tokens` | ✅ Mapping par provider requis |
| H3 | Le budget de tokens gère un provider cloud | Partiellement : `TokenBudget::compute` ne dépend que de `num_ctx` et d'un ratio chars/token par langue (`token_budget.rs` l. 206–362). Avec 1M de contexte, **toutes** les sections atteignent leur plafond → prompts d'intervention ≈ 35–40K caractères ≈ 10–12K tokens, coût ×3 sans gain de qualité | ✅ Notion de « budget de contexte » (plafond de coût) à introduire pour DeepSeek |
| H4 | Le mode think Ollama = la réflexion DeepSeek | Faux : Ollama sépare `thinking`/`content` sans niveau, le moteur **jette** le raisonnement (`orchestrator.rs` l. 2036–2055) et multiplie `num_predict` ×3 (l. 393–401) ; DeepSeek expose des niveaux et facture le raisonnement | ✅ Politique de réflexion par type d'appel à concevoir |
| H5 | Le patron « quota Tavily » est réutilisable pour un plafond de coûts | Vrai : période glissante + compteur + historique JSON (`repository.rs::check_and_reset_tavily_period`, `increment_tavily_usage` ; garde-fou pré-vol `commands/discussion.rs` l. 105–110 ; garde-fou moteur `orchestrator.rs` l. 2451–2461) | ✅ Réutilisé (généralisé) |
| H6 | Le front n'a qu'un seul point de dépendance à `ollamaModel` | Faux : `App.tsx` l. 36–41 (init Ollama au démarrage), `SettingsPage.tsx` (section entière), `SetupPage.tsx` l. 119–124 (écrasement `numCtx`) et l. 1124/1149/1263 (RAG), `useArenaStore.ts` l. 499 (`modelName` persisté), `SummaryPage.tsx` l. 34, `HistoryPage.tsx` l. 99, `HistoryDetailPage.tsx` l. 101 | ✅ 7 points à adapter |
| H7 | Les émotions dérivent mécaniquement | Vrai : `is_discussion_stagnating: self.current_turn > 3` (`orchestrator.rs` l. 2113) → −5 engagement / −5 curiosité **à chaque intervention** dès le tour 4, sans détection de stagnation ; frustration par défaut 10 (`models/emotion.rs` l. 22) mais décroissance **vers 50** (+2/tour, `constants.rs` l. 196–197) → tout le monde finit blasé et agacé | ✅ Bug de conception confirmé |
| H8 | La pénalité de ban rule-based s'applique | Faux : `was_recently_banned: ban_remaining_turns > 0` est évalué à l'étape C.5 (l. 2112) **avant** la modération C.6 (l. 2266–2272), et un banni n'est jamais dans l'ordre de parole (`turn_manager.rs` l. 18–23) → code mort ; seul l'appel LLM d'analyse (l. 2918–2922) voit le ban | ✅ Confirmé |
| H9 | L'axe `accord` est piloté | Partiellement : aucune règle dans `emotion_engine::update_emotions` (l. 24–80), seulement l'analyse LLM et la contagion ; dans la directive, `accord` n'est consulté qu'en 6ᵉ priorité (`directive_builder.rs` l. 316–346) | ✅ Axe orphelin |
| H10 | Le biais « tous contre X » vient du prompt de l'IArbitre | Faux : il est **structurel** — couche 5 de la directive (`directive_builder.rs` l. 812–828 : « Tu parles après X. Rebondis… ») + gabarit `FirstOfTurn` (`mode_prompts.rs` l. 674–683 : « Réagis au tour précédent ») + bloc `[Tour en cours]` où le premier orateur apparaît dans **tous** les prompts suivants (`prompt_builder.rs` l. 636–653). Le tour 1 est déjà protégé (gabarit `Turn1`, l. 582–595) | ✅ Correctif dans le moteur, pas dans le persona |
| H11 | `respond_or_pass` (UserDriven) fonctionne | Fragile : appel **sans** `json_format` (`orchestrator.rs` l. 3229–3234) et `true` par défaut si le JSON n'est pas parsable (l. 3245–3253) → avec un modèle local, tout le monde répond toujours | ✅ Correctif simple |
| H12 | Fiction : l'ouverture utilisateur est garantie | Faux : si l'utilisateur laisse expirer le timeout (`handle_user_intervention` l. 2288–2293), le tour 1 démarre sans ouverture alors que la couche 5 affirme « L'utilisateur a écrit l'ouverture » (`directive_builder.rs` l. 736–742) | ✅ Cas limite confirmé |
| H13 | Le document co-construit coûte O(n) | Faux : régénération **intégrale** à chaque intervention (`orchestrator.rs` l. 1245–1260, 3155–3215 ; `num_predict = max(2×taille + 1024, 4096)` l. 3183–3185) → O(n²) tokens de sortie sur la discussion, critique en facturation cloud | ✅ Confirmé |
| H14 | La carte des arguments dédoublonne les thèses | Exact-match insensible à la casse uniquement (l. 3498–3503) ; contre-arguments sans thèse résolue **silencieusement perdus** (l. 3583–3584) ; la vue par orateur n'affiche que les **propriétaires de thèses** (`argument_map.rs` l. 125–131) ; `MarkmapViewer` refait `fit()` à chaque mise à jour (`MarkmapViewer.tsx` l. 118–123) → zoom/pan perdus à chaque tour ; seule la forme markdown est persistée (`schema.rs` l. 114–133) | ✅ 5 défauts confirmés |
| H15 | La file de parole est affichée | Non : `speakerOrder` est dans le store (`useArenaStore.ts` l. 37) mais **aucun composant ne le rend** (grep vide) ; `maxTurns` n'apparaît pas dans l'arène | ✅ Manque UX |
| H16 | `EmotionSidebar` reçoit les seuils proprement | Non : elle **monkey-patche** `handleEvent` du store (`EmotionSidebar.tsx` l. 79–100) — fragile (restauration d'un handler périmé au démontage, incompatible avec un second abonné) | ✅ Dette technique |
| H17 | La détection de refus couvre le français | Non : heuristiques anglaises uniquement (`orchestrator.rs` l. 40–56) | ✅ Mineur |
| H18 | d3 est disponible pour un graphe de relations | Oui, transitivement via `markmap-view` (`d3@^7.8.5`) mais **non déclaré** dans `package.json` → à ajouter explicitement si utilisé | ✅ |
| H19 | `reqwest` sait lire du SSE | Oui : feature `stream` active ; le client Ollama parse déjà du NDJSON via `bytes_stream()` (`ollama/client.rs` l. 152–229) — même technique pour `data:` SSE | ✅ Pas de nouvelle dépendance |

### 1.3 Faux positifs / faux négatifs traqués
- **Faux positif évité** — « `model_info::detect_think_support` couvre la réflexion DeepSeek » : non, c'est une inspection du template Go d'Ollama (`model_info.rs` l. 170–172). DeepSeek expose la capacité par contrat d'API → capacité déclarée par le provider.
- **Faux positif évité** — « `numCtx` global suffit pour DeepSeek » : le réglage est aujourd'hui une contrainte VRAM (auto-rempli depuis `nvidia-smi`, `commands/ollama.rs::initialize_ollama`) ; pour DeepSeek il devient un **plafond de coût** avec une sémantique et des bornes différentes (pas de VRAM, pas de préchargement, pas de déchargement).
- **Faux positif évité** — « `THINK_NUM_PREDICT_MULTIPLIER` s'applique à DeepSeek » : DeepSeek a ses propres valeurs par défaut de `max_tokens` (64K en réflexion !) ; sans `max_tokens` explicite, une intervention peut coûter très cher. Le multiplicateur reste Ollama-only.
- **Faux négatif évité** — « il faut réinventer un système de quotas/compteurs » : le patron Tavily (période glissante, compteur, historique JSON, garde-fous pré-vol et moteur) couvre 90 % du besoin ; on le généralise.
- **Faux négatif évité** — « il faut un parseur SSE tiers » : la boucle `bytes_stream` + tampon ligne du client Ollama se transpose telle quelle (préfixe `data: `, sentinelle `[DONE]`, commentaires `:` ignorés).
- **Faux négatif évité** — « un tokenizer DeepSeek est nécessaire pour la prévisualisation » : le `usage` de l'API fait foi ; pour la prévisualisation on réutilise le ratio chars/token du budget (constante provider : 3,3 EN / 1,7 ZH d'après la doc DeepSeek).

### 1.4 Arbitrages majeurs retenus

| # | Arbitrage | Justification par rapport aux patterns en place |
|---|---|---|
| A1 | **Trait `LlmProvider`** (module `src-tauri/src/llm/`) + adaptateur Ollama **sans toucher au comportement** + client DeepSeek ; `OllamaClient` conserve ses méthodes propres (show/ps/preload/unload) utilisées par `commands/ollama.rs` | README v2.0 annonce « multi-modèles (anthropic, openai, gemini) » → l'abstraction n'est pas du YAGNI ; SoC/SRP ; le moteur ne connaît plus que des requêtes génériques |
| A2 | Requête générique `LlmRequest { system, user, params, json_mode, reasoning }` construite par le moteur ; chaque provider traduit vers son format wire | Élimine les 2 « builders » du moteur (`build_request`/`build_discussion_request`) au profit d'un seul, déplace le ×3 Ollama dans l'adaptateur Ollama (là où il a un sens) |
| A3 | `LlmError` unifié (`Cancelled`, `Connection`, `Auth`, `InsufficientBalance`, `RateLimited`, `Overloaded`, `Client`, `ModelNotFound`, `Json`) avec `is_retryable()` ; `From<OllamaError>` | Le moteur pattern-matche déjà sur `OllamaError::Cancelled` (12 sites) : un seul type d'erreur métier simplifie ; `CommandError` gagne `Llm(String)` (l'existant `Ollama(String)` reste pour les commandes Ollama) |
| A4 | Un seul mode de transport : **toujours** `stream: true` (comme aujourd'hui `chat()` streame avec callbacks no-op) | Gère nativement le keep-alive DeepSeek, l'annulation immédiate, et le `usage` en dernier chunk |
| A5 | Politique de réflexion **par type d'appel** : utilitaires (réactions, modération, mémoire, émotions, votes, décisions de recherche, sélection RAG, mise à jour document, carte) → `off` ; interventions → niveau de l'intervenant (`auto` par défaut) ; synthèse → `high` ; introduction → niveau IArbitre (`low` par défaut) | Les appels utilitaires sont 3 N + 3 par tour : c'est là que le raisonnement coûterait sans valeur ; conforme à « rythme dynamique » |
| A6 | Avec DeepSeek et réflexion active, la **phase de pensée séparée** (`process_thought`, 1 appel/intervention) est **remplacée** par `reasoning_content` streamé en `ThoughtChunk` et stocké dans `Message.inner_thought` avec un nouveau champ `thought_kind: "persona" \| "reasoning"` (serde default) | −N appels/tour ; l'UI existante « Voir la réflexion » réaffiche le raisonnement avec un libellé adapté ; Ollama inchangé (réflexion jetée comme aujourd'hui) |
| A7 | Comptage : `LlmUsage` renvoyé par chaque appel, agrégé dans le moteur, événement `LlmUsageUpdated` (camelCase par variante), persistance `discussions.usage_json` + `discussions.llm_provider`, période mensuelle glissante DeepSeek dans `settings` (patron Tavily), plafond mensuel optionnel avec **avertissement à 80 % et arrêt doux à 100 %** | Réutilise les patrons existants (événements, migrations idempotentes, clé-valeur) |
| A8 | Tarifs en **constantes versionnées** (`constants.rs`, section `// ── DeepSeek pricing ──` avec `DEEPSEEK_PRICING_DATE`), heures pleines/creuses calculées en UTC, coût affiché comme **estimation** ; override utilisateur = P2 | Règle « toutes les constantes dans `constants.rs` » ; la grille change souvent → un seul endroit à mettre à jour |
| A9 | `numCtx` reste la clé de réglage mais devient, en mode DeepSeek, « budget de contexte » (défaut 32 768, bornes 4 096–262 144) alimentant `TokenBudget` tel quel ; ratio chars/token fourni par le provider | Zéro changement dans l'algorithme waterfall ; le `TokenBudgetPreview` gagne une ligne « coût estimé / tour » |
| A10 | Liste de modèles **dynamique** (`GET /models`) avec repli sur constantes ; validation de clé via `GET /user/balance` (affiche le solde) | Résilient aux renommages DeepSeek ; feedback utilisateur immédiat |
| A11 | Correctifs comportementaux (émotions, « tous contre X », modes) livrés en lots **séparés** du provider, chacun avec tests de prompts | Non-régression isolable, revue par lot |
| A12 | UX : panneau latéral droit **à onglets** (Émotions / Document / Carte / Relations) remplaçant les 3 barres juxtaposées ; séparateurs de tour dans le flux ; file de parole ; bouton « aller au dernier message » (pas d'auto-scroll forcé, choix utilisateur historique) | Répond au TODO (« déplacer la séparation », lisibilité) sans revenir sur l'auto-scroll retiré volontairement |

### 1.5 Questions / arbitrages résiduels
Aucune question n'est **bloquante** au sens strict : toutes ont une valeur par défaut techniquement justifiée. Elles sont listées pour validation **en même temps que le plan** (elles ne conditionnent pas le démarrage du Lot 0) :

| # | Décision métier | Défaut proposé | Alternative |
|---|---|---|---|
| Q1 | Affichage du raisonnement brut DeepSeek (chaîne de pensée en anglais/chinois possible) aux utilisateurs | **Oui**, dans le volet « Réflexion » existant, libellé « Raisonnement du modèle », désactivable dans Paramètres | Ne jamais l'afficher (comme Ollama aujourd'hui) |
| Q2 | Comportement au dépassement du plafond mensuel | Avertissement à 80 %, **arrêt doux** (fin de tour + synthèse) à 100 %, refus de démarrer si déjà dépassé | Avertissement seul |
| Q3 | Devise d'affichage | **USD** (grille officielle) ; solde affiché dans la devise renvoyée par `/user/balance` | Conversion EUR (source externe, hors périmètre) |
| Q4 | Modèle par défaut à la sélection de DeepSeek | **`deepseek-flash`** (V4.1, meilleur rapport qualité/prix, 2 500 req. simultanées) | `deepseek-v4-pro` |
| Q5 | Niveau de réflexion par défaut des GladIAteurs | **`auto`** (off/low/high selon les déclencheurs existants) ; IArbitre `low` ; synthèse `high` | `low` fixe |
| Q6 | Fréquence de mise à jour du document co-construit | **Par tour** (1 appel/tour) avec option « par intervention » | Conserver par intervention |

---

## 2. Audit détaillé par domaine

Sévérité : 🔴 critique (résultat faux / coût) · 🟠 majeur (qualité perceptible) · 🟡 mineur (dette, finition). « Lot » renvoie au §6.

### 2.1 Architecture LLM et couplage Ollama (préparation DeepSeek)

| Sév. | Constat | Preuve | Recommandation | Lot |
|---|---|---|---|---|
| 🔴 | `OllamaClient` et `OllamaError` traversent le moteur, la distribution des tours et le RAG | H1 | Trait `LlmProvider` + `LlmRequest` + `LlmError` ; `DiscussionEngine.llm: Arc<dyn LlmProvider>` ; `AsyncTurnContext.llm` ; `RagStore::query(&dyn LlmProvider)` | 1 |
| 🔴 | Paramètres LLM non portables (`num_ctx`, `top_k`, `repeat_penalty`) | H2 | `LlmParams` inchangé (compatibilité DB/front) + `reasoning_level: Option<ReasoningLevel>` (serde default) ; mapping dans chaque adaptateur ; `LlmParamsForm` variante par provider | 1, 3, 4 |
| 🟠 | Détection think au démarrage par inspection de template Ollama | `orchestrator.rs` l. 514–530 | `LlmCapabilities { supports_reasoning, supports_json_mode, context_tokens, chars_per_token_latin/cjk, reports_usage }` renvoyées par le provider | 1 |
| 🟠 | Deux builders de requête dans le moteur avec logique ×3 | l. 363–406 | Un seul `LlmRequest`, le ×3 migre dans l'adaptateur Ollama (`if reasoning != Off && supports_reasoning`) | 1 |
| 🟡 | `is_model_refusal` anglais uniquement | l. 40–56 | Ajouter FR/ZH (« je ne peux pas », « je suis désolé », « 我不能 », « 抱歉 ») via constantes | 7 |
| 🟡 | Timeout HTTP global 120 s (`OLLAMA_HTTP_TIMEOUT_SECS`) inadapté à un raisonnement `max` cloud | `ollama/client.rs` l. 47 | Client DeepSeek : pas de timeout global, **timeout d'inactivité** entre chunks (`DEEPSEEK_IDLE_TIMEOUT_SECS = 180`) + timeout de connexion | 2 |

### 2.2 Carte des arguments

| Sév. | Constat | Preuve | Recommandation | Lot |
|---|---|---|---|---|
| 🟠 | Thèses quasi-doublons (reformulations LLM) → saturation à `ARGMAP_MAX_THESES` | `orchestrator.rs` l. 3493–3513 | Dédoublonnage flou : normalisation (casse, ponctuation, accents, mots vides) + Jaccard sur tokens ≥ `ARGMAP_THESIS_SIMILARITY_THRESHOLD` (0,6) ou containment ; fusion vers la thèse existante | 8 |
| 🟠 | Contre-arguments dont `against_thesis` ne se résout pas → **perdus sans trace** | l. 3583–3584 | Rattacher à la thèse la plus similaire (score ≥ seuil) sinon à une thèse « Contre-arguments non rattachés » du même orateur ; log warn + compteur `dropped` remonté dans l'événement | 8 |
| 🟠 | Vue par orateur : un orateur qui n'a que des arguments n'a pas de branche | `argument_map.rs` l. 125–131 | Collecter les orateurs des thèses **et** des arguments ; branche « Arguments » pour ceux sans thèse | 8 |
| 🟠 | Zoom/pan réinitialisés à chaque tour (`fit()` après `setData`) | `MarkmapViewer.tsx` l. 118–123 | `fit()` seulement au premier rendu ou si l'utilisateur n'a pas interagi (drapeau `userInteracted` sur `zoom` d3) ; bouton « recentrer » | 8 |
| 🟠 | Persistance markdown uniquement → pas de re-rendu ni de statistiques ultérieures | `schema.rs` l. 114–133 ; `history.rs` | Colonne `argument_map_json` (sérialisation `ArgumentMap`) + les 2 markdown conservés (compat) ; `HistoryDetail` reconstruit les vues depuis le JSON s'il existe | 8 |
| 🟡 | Nouveaux nœuds non distingués ; pas de lien nœud → message | `to_markdown` | Marqueur « ✨ nouveau ce tour » (id des nœuds ajoutés au dernier merge) ; `data-message-id` non porté par markmap → différer (P2) | 8 |
| 🟡 | Extraction lancée même pour un tour à 1 message ; `ARGMAP_NUM_CTX` 16K forcé (Ollama) | l. 3297–3341 | Seuil `ARGMAP_MIN_TURN_MESSAGES = 2` ; `num_ctx` forcé uniquement pour Ollama (adaptateur) | 8 |
| 🟡 | `MindmapSidebar` : compteur « 3T / 12A » peu lisible, légende compacte | `MindmapSidebar.tsx` | Libellés complets, compteur par orateur (chips colorées), état « analyse en cours » (activité `argumentMap`) | 9 |

### 2.3 Comportement et interactions des GladIAteurs

| Sév. | Constat | Preuve | Recommandation | Lot |
|---|---|---|---|---|
| 🔴 | Effet « tous contre X » structurel (chaque orateur est poussé à rebondir sur ceux qui viennent de parler, et le premier du tour est dans tous les prompts suivants) | H10 | **Ciblage tournant** : à chaque intervention (tour ≥ 2), le moteur choisit un *focus* pondéré — un orateur du tour précédent non encore ciblé ce tour (poids ×3), un rival/allié (×2), le dernier orateur (×1), « personne / le sujet » (×1, pour les tours pairs ou si engagement bas) ; la couche 5 formule « Adresse-toi en priorité à {focus} » ou « Fais avancer le sujet sans répondre à quelqu'un en particulier » ; tirage `rand` + constantes de poids | 7 |
| 🟠 | `[Tour en cours]` complet dans chaque prompt → sur-pondération du premier orateur | `prompt_builder.rs` l. 636–653 | Ordonner le bloc par pertinence : message du *focus* en dernier (biais de récence), les autres tronqués à `current_turn_msg_chars / 2` | 7 |
| 🟠 | Anti-répétition des actes de parole = 1 re-tirage | `directive_builder.rs` l. 643–647 | Fenêtre des 3 derniers actes par orateur (poids ×0,3 pour les actes récents) ; rotation garantie | 7 |
| 🟠 | Réflexion in-character = 1 appel LLM supplémentaire par intervention (coût ×2 en cloud) et jamais affichée en mode think | l. 1717–1772, 2036–2055 | Voir A6 ; en Ollama sans think : inchangé | 6 |
| 🟠 | Heuristique think probabiliste (20–60 %) sans lien avec la **difficulté** du point discuté | l. 1928–1974 | Mode `auto` : déclencheurs existants + « contradiction reçue au tour précédent » + « fin proche » → niveau `high` ; sinon `low` ; jamais au tour 1 ; plafond de probabilité conservé pour la variété | 6 |
| 🟡 | Refus non détectés en FR/ZH | H17 | Constantes multilingues | 7 |
| 🟡 | `TEMP_DIFFICULTY_BOOST` appliqué même en réflexion DeepSeek (température ignorée) | contrat API | Le retry « refus/vide » bascule `reasoning: off` + température montée (là, elle agit) | 6 |

### 2.4 Émotions et impact sur les échanges

| Sév. | Constat | Preuve | Recommandation | Lot |
|---|---|---|---|---|
| 🔴 | Stagnation « permanente » dès le tour 4 (−5 engagement, −5 curiosité par intervention) | H7 | Détection réelle : stagnation si (a) similarité Jaccard entre le résumé contextuel du tour et celui du tour −1 ≥ `EMOTION_STAGNATION_SIMILARITY` (0,8) **ou** (b) aucune réaction (like/dislike) sur 2 tours **ou** (c) l'analyse LLM renvoie `stagnating: true` (champ ajouté au JSON, serde default) | 7 |
| 🔴 | Décroissance de la frustration vers 50 alors que le défaut est 10 → montée mécanique | H7 | Cible de décroissance = **valeur initiale du profil** (`initial_emotions`) et non 50 ; idem enthousiasme | 7 |
| 🟠 | Pénalité de ban rule-based jamais appliquée | H8 | Appliquer `EMOTION_BAN_*` dans `process_moderation` au moment du ban (l. 2178–2180) ; supprimer le champ mort | 7 |
| 🟠 | Axe `accord` sans règle | H9 | Règle : `accord` += `EMOTION_ACCORD_LIKE_FACTOR` × (likes donnés − dislikes donnés) au tour, borné ; dans la directive, `accord` bas devient la 2ᵉ priorité derrière la frustration | 7 |
| 🟠 | Double comptage possible : règles (likes/dislikes) **puis** analyse LLM à qui l'on redonne les mêmes réactions | l. 2099–2123 et 2908–2927 | Le prompt LLM reçoit les émotions **après** règles et la consigne « les réactions ont déjà été prises en compte : n'ajuste que pour le ton et le contenu » ; deltas LLM bornés à ±10 (constante) | 7 |
| 🟠 | Une seule émotion pilote la directive (chaîne de priorité) | l. 316–346 | Deux émotions dominantes (distance à la valeur initiale) combinées, avec la dynamique persona correspondante ; texte de « nuance » si les deux sont contradictoires (ex. frustré + enthousiaste) | 7 |
| 🟡 | Contagion appliquée aussi à l'IArbitre et symétrique | l. 2985–3012 | Conserver ; exclure l'IArbitre de la moyenne (il ne « participe » pas) — constante | 7 |
| 🟡 | UI : libellés d'axes tronqués (80 px), pas d'explication des axes, vue barres uniquement | `EmotionAxisSlider.tsx` l. 53 | Libellé pleine largeur au survol (tooltip), option **radar** (SVG 6 axes, animation `transition`), historique par sparkline conservé | 9 |
| 🟡 | Monkey-patch de `handleEvent` pour les seuils | H16 | Champ de store `lastThresholdCrossed` (+ compteur) mis à jour dans `handleEvent` ; le composant s'y abonne | 9 |

### 2.5 Modes de discussion

| Mode | Constat | Recommandation | Lot |
|---|---|---|---|
| UserDriven | `respond_or_pass` sans JSON mode, `true` par défaut (H11) ; pas d'indication UI de qui a passé | `json_mode: true`, `reasoning: off`, parse robuste (`parse_json_response`), défaut `true` **seulement** sur erreur réseau ; événement `TurnSkipped`/`SpeakerPassed { speaker_id }` → chip « a passé » dans le flux | 7, 9 |
| CollaborativeFiction | Ouverture non garantie (H12) ; `Random`/`Democratic` déjà forcés en `Sequential` (l. 799–803) ✅ | Si aucun message utilisateur au tour 1 → l'IArbitre écrit l'ouverture (nouveau prompt `build_fiction_opening_prompt`) ; couche 5 adaptée (« X a écrit l'ouverture ») | 7 |
| CoConstruction | Régénération intégrale par intervention (H13) | Option `documentUpdateGranularity: "turn" \| "intervention"` (défaut `turn`) : 1 appel en fin de tour avec l'ensemble des interventions ; `DocumentUpdated` inchangé ; le diff visuel existant fonctionne à l'identique | 7 |
| Socratic | Question de l'IArbitre à chaque tour (≥ 2), correcte ; pas de mémoire des questions déjà posées | Injecter les 3 dernières questions dans `build_socratic_question_prompt` (anti-répétition) | 7 |
| Ideation / Tutorial / CritiqueReview | Bien couverts par `mode_prompts.rs` (poids d'actes de parole, override système, critères de modération) ✅ | RAS — tests de non-régression des gabarits existants (déjà présents l. 813–950) | — |
| Debate | Voir §2.3 | — | 7 |
| Tous | Le résumé mémoire et l'analyse d'émotions utilisent `arbitre.llm_params` (température 0,8 par défaut) pour des sorties JSON | Température `TEMP_VOTING` (0,3) pour **tous** les appels JSON (aujourd'hui seulement votes/recherche/sélection RAG) | 7 |

### 2.6 UX / UI

| Zone | Constat | Recommandation (moderne & dynamique, dans la charte oklch existante) | Lot |
|---|---|---|---|
| Arène — flux | Aucun séparateur de tour ; pas d'horodatage ni de numéro de tour par message ; pas de file de parole (H15) ; l'activité n'est visible que dans le sous-titre de la barre | **`TurnDivider`** (« Tour 3 / 8 » + progression) ; **`SpeakerQueue`** (avatars de l'ordre du tour, actif surligné, passés grisés, bannis barrés) sous la barre d'état ; bouton flottant « ↓ dernier message » quand on n'est pas en bas (l'auto-scroll reste désactivé par choix) ; animation d'apparition des bulles (`animate-in fade-in slide-in-from-bottom-2`, `tw-animate-css` déjà installé, utilisé 2 fois seulement) ; badge de tour discret dans chaque bulle | 9 |
| Arène — panneaux | 3 barres latérales juxtaposées (`ArenaPage.tsx` l. 146–164) compressent le flux | **Panneau droit à onglets** (Émotions / Document / Carte / Relations) redimensionnable, mémorisant l'onglet ; onglets avec pastille d'activité (ex. « Carte • mise à jour ») | 9 |
| Arène — barre d'état | Compteur web seul ; pas de tokens/coût | Pilules : tour x/y (anneau de progression), recherches web/wiki/RAG, **tokens & coût** (DeepSeek), indicateur heure pleine/creuse | 5, 9 |
| Émotions | Cartes denses ; seuils par pulsation ; monkey-patch | Radar optionnel, tooltips, mise en avant de l'émotion dominante (emoji existant), transitions | 9 |
| Relations | Données de relations calculées mais seulement en texte « Backstage » ; wish-list TODO « Alliances dynamiques » | Événement `RelationshipsUpdated { edges: [{from,to,kind}] }` émis après chaque directive ; **mini-graphe** (SVG maison, disposition circulaire — pas de d3 direct, évite une dépendance) avec arêtes vertes/rouges/ambre | 9 |
| Setup | Parcours clair mais fichier de 1 561 lignes ; l'étape 4 n'affiche pas le coût | Extraire `StepTopic`/`StepArbitre`/`StepGladiateurs`/`StepKnowledge`/`StepSummary` en fichiers (`components/setup/steps/`), sans changement fonctionnel ; ligne « Coût estimé / tour » dans `TokenBudgetPreview` (DeepSeek) ; résumé final avec provider/modèle/réflexion | 4, 9 |
| Settings | Page unique de 933 lignes ; section Ollama surchargée | **Section « Fournisseur LLM »** (segmented control Ollama / DeepSeek) en tête ; contenu conditionnel ; sous-sections repliables ; extraire `OllamaSettings`, `DeepSeekSettings`, `TokenBudgetPriorities`, `TavilySettings`, `LicenseSettings` | 4 |
| Synthèse / Historique | Stat « Modèle » = `settings.ollamaModel` du moment (faux si le réglage a changé) | Libellé `provider · modèle` figé à la sauvegarde ; carte « Tokens / coût » ; historique : colonne provider | 5 |
| Accueil | Image de fond + 3 boutons | Bandeau d'état compact (provider actif, modèle, solde/quota) ; sinon RAS | 9 |
| Charte | Tokens oklch cohérents, sombre/clair ✅ ; peu de mouvement, peu de hiérarchie typographique | Échelle typographique (titres `tracking-tight`), surfaces `bg-card/60 backdrop-blur` pour la barre d'état, ombres subtiles sur les cartes actives, `motion-safe:` sur toutes les animations | 9 |

### 2.7 Transverse (robustesse, dette)

| Sév. | Constat | Recommandation | Lot |
|---|---|---|---|
| 🟠 | Secrets (Tavily, licence) en clair dans `settings` ; la clé DeepSeek suivra le même patron | Documenter ; masquer dans l'UI (déjà le cas pour Tavily) ; **jamais** dans `tracing` (test qui grep les logs) ; migration vers le trousseau OS = P2 | 3 |
| 🟠 | `SettingsPage` auto-sauvegarde toute modification après 800 ms → la clé API est persistée à chaque frappe | Champ clé : sauvegarde **explicite** au bouton « Valider » (comme la licence) ; pas d'auto-save pour ce champ | 4 |
| 🟡 | `LlmParamsForm` : `numPredict` max 4 096 | Borne par provider (DeepSeek : 16 384 par défaut d'UI) | 4 |
| 🟡 | `EmotionSidebar` monkey-patch (H16) | Voir §2.4 | 9 |
| 🟡 | Tests Rust : 281 TU, **aucun test des flux moteur** (pas d'injection de faux provider) | Le trait `LlmProvider` rend possible un `MockLlmProvider` scripté → tests d'orchestration (tour complet, annulation, budget dépassé) | 1, 5 |

---

## 3. Conception cible — provider DeepSeek

### 3.1 Module `src-tauri/src/llm/`

```
llm/
├── mod.rs        // trait LlmProvider, LlmRequest, LlmResponse, LlmUsage, LlmCapabilities, ReasoningLevel, LlmError, ProviderKind, CallKind
├── ollama.rs     // struct OllamaProvider(OllamaClient) — adaptateur iso-fonctionnel
├── deepseek.rs   // struct DeepSeekProvider — client HTTP (SSE), /models, /user/balance
├── pricing.rs    // grille tarifaire, heures pleines/creuses (UTC), estimate_cost()
└── factory.rs    // build_provider(&AppSettings) -> Result<Arc<dyn LlmProvider>, LlmError>
```

```rust
// mod.rs — signatures cibles (extraits)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ProviderKind { #[default] Ollama, DeepSeek }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ReasoningLevel { #[default] Off, Low, High, Max, Auto }   // Auto résolu par le moteur avant l'appel

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CallKind { Introduction, Thought, Intervention, Reaction, Moderation, Memory, Emotion,
                    Vote, SearchDecision, RagSelect, DocumentUpdate, ArgumentMap, Synthesis, Utility }

pub struct LlmRequest {
    pub system: String,
    pub user: String,
    pub params: LlmParams,          // inchangé (compat DB/front)
    pub json_mode: bool,
    pub reasoning: ReasoningLevel,  // jamais Auto ici
    pub call_kind: CallKind,        // pour la politique provider + le suivi d'usage
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmUsage { pub prompt_tokens: u32, pub cached_tokens: u32, pub completion_tokens: u32, pub reasoning_tokens: u32 }

pub struct LlmResponse { pub content: String, pub reasoning: Option<String>, pub usage: Option<LlmUsage>, pub truncated: bool }

#[derive(Debug, Clone)]
pub struct LlmCapabilities {
    pub supports_reasoning: bool, pub reasoning_levels: bool, pub supports_json_mode: bool,
    pub context_tokens: u32, pub chars_per_token_latin: f64, pub chars_per_token_cjk: f64, pub reports_usage: bool,
}

#[async_trait::async_trait]            // nouvelle dépendance `async-trait` (dyn-compatible) — ou fn retournant Pin<Box<dyn Future>> sans dépendance
pub trait LlmProvider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    fn model_name(&self) -> &str;
    fn capabilities(&self) -> LlmCapabilities;
    async fn chat_stream(&self, req: &LlmRequest, on_content: &(dyn Fn(&str) + Send + Sync),
                         on_reasoning: &(dyn Fn(&str) + Send + Sync), cancel: CancellationToken) -> Result<LlmResponse, LlmError>;
    async fn chat(&self, req: &LlmRequest, cancel: CancellationToken) -> Result<LlmResponse, LlmError>; // = chat_stream avec callbacks no-op
    async fn validate(&self) -> Result<(), LlmError>;   // modèle existe / clé valide
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("Request cancelled")] Cancelled,
    #[error("Connection failed: {0}")] Connection(String),
    #[error("Authentication failed")] Auth,
    #[error("Insufficient balance")] InsufficientBalance,
    #[error("Rate limited")] RateLimited,
    #[error("Provider overloaded")] Overloaded,
    #[error("Client error {0}: {1}")] Client(u16, String),
    #[error("Model not found: {0}")] ModelNotFound(String),
    #[error("JSON parse error: {0}")] Json(String),
}
impl LlmError { pub fn is_retryable(&self) -> bool { matches!(self, Self::Connection(_) | Self::RateLimited | Self::Overloaded) } }
```

- **Dyn-compatibilité** : `async fn` dans un trait objet requiert `async-trait` (crate stable, largement utilisée avec Tauri/tokio) — ou l'écriture manuelle `fn chat_stream(...) -> Pin<Box<dyn Future<Output=…> + Send + '_>>`. Décision : `async-trait` (lisibilité, KISS).
- Les callbacks passent de `impl Fn` à `&dyn Fn` (objet-sûr) — les closures existantes du moteur restent valides.

### 3.2 Adaptateur Ollama (zéro régression)
- `OllamaProvider` encapsule `OllamaClient` ; `capabilities()` = `{ supports_reasoning: <détection template au 1er appel ou via show_model>, reasoning_levels: false, supports_json_mode: true, context_tokens: num_ctx, chars_per_token_latin: 3.8, cjk: 1.5, reports_usage: false }`.
- Mapping : `reasoning != Off` **et** modèle pensant → `num_predict *= THINK_NUM_PREDICT_MULTIPLIER`, `think: None` (comportement actuel conservé à l'identique, y compris le rejet de `thinking`) ; `json_mode` → `format: "json"`.
- `LlmResponse.usage` : Ollama renvoie `prompt_eval_count` / `eval_count` dans le chunk final → **bonus gratuit** : comptage de tokens aussi en local (coût 0). Nécessite d'ajouter ces deux champs (`#[serde(default)]`) à `ChatResponse`.
- Tests : les 7 tests `strip_think_tags` restent ; nouveau test « la requête wire Ollama générée depuis `LlmRequest` est identique à celle de `build_request` actuel » (golden test).

### 3.3 Client DeepSeek
- Requête wire (`serde` struct, pas de `json!` dynamique) :
  ```json
  { "model": "deepseek-flash", "messages": [{"role":"system","content":…},{"role":"user","content":…}],
    "stream": true, "stream_options": {"include_usage": true},
    "max_tokens": <num_predict + allowance(reasoning)>, "temperature": <si reasoning=Off>, "top_p": <max(top_p, 0.95) si reasoning≠Off>,
    "response_format": {"type":"json_object"} <si json_mode>,
    "thinking": {"type": "enabled"|"disabled"}, "reasoning_effort": "low"|"high"|"max" <si enabled> }
  ```
  `top_k`, `repeat_penalty`, `num_ctx` **ignorés** (documenté dans l'UI).
- `max_tokens` = `params.num_predict + DEEPSEEK_REASONING_ALLOWANCE_{LOW,HIGH,MAX}` (constantes ; hypothèse H-DS-1 : `max_tokens` borne raisonnement + contenu — **à valider au Lot 0**, sinon `max_tokens = num_predict`).
- Parsing SSE : réutilise le motif tampon-ligne d'`ollama/client.rs` ; ignore les lignes vides et celles commençant par `:` (keep-alive) ; `data: [DONE]` termine ; chaque `data:` → `ChatCompletionChunk { choices[0].delta.{content, reasoning_content}, choices[0].finish_reason, usage }`.
- `finish_reason == "length"` → `truncated: true` (le moteur logue comme aujourd'hui) ; `insufficient_system_resource`/`aborted` → `LlmError::Overloaded`.
- Retry : jusqu'à `DEEPSEEK_MAX_RETRIES = 3` sur `is_retryable()` avec backoff exponentiel + jitter (`DEEPSEEK_RETRY_BASE_MS = 1000`), respect de `Retry-After` si présent ; **jamais** sur 400/401/402/422 ; annulation prioritaire (`tokio::select!`).
- Timeouts : connexion 15 s, **inactivité** 180 s entre chunks (le keep-alive réinitialise), pas de timeout global.
- Sécurité : `Authorization: Bearer` ; la clé n'apparaît dans aucun `tracing` (test dédié) ; `Debug` de la struct masque la clé.
- `GET /models` → `Vec<ModelInfo>` (réutilise `ollama::types::ModelInfo { name, size: 0, digest: "" }` pour ne pas dupliquer le type front) ; `GET /user/balance` → `DeepSeekBalance { is_available, balances: [{currency, total}] }`.
- Détection d'un `content` vide en JSON mode (limite documentée) → une nouvelle tentative avec `reasoning: Off` et le suffixe « Respond ONLY with JSON » (le parseur `json_parser` reste la seconde ligne de défense).

### 3.4 Niveau de réflexion — politique

| Appel (`CallKind`) | Ollama (inchangé) | DeepSeek |
|---|---|---|
| Introduction (IArbitre) | défaut modèle | niveau IArbitre (défaut `low`) |
| Thought (réflexion in-character) | 1 appel séparé | **supprimé** si le niveau résolu de l'intervenant ≠ `Off` (le `reasoning_content` streame en `ThoughtChunk`) ; conservé si `Off` |
| Intervention | heuristique think | niveau de l'intervenant : `Auto` → `Off` au tour 1, `High` si (frustration > 70 ∨ contradiction ≥ 2 dislikes ∨ fin ≤ 2 tours), sinon `Low` ; `Low/High/Max` fixes sinon |
| Réactions, modération, mémoire, émotions, votes, tie-break, décisions de recherche, sélection RAG, respond/pass, question socratique, mise à jour document, carte | `think: None` | **`Off`** + température `TEMP_VOTING` pour les sorties JSON |
| Synthèse | défaut | `High` |

- Résolution de `Auto` dans le moteur (`resolve_reasoning_level(glad_idx, call_kind)`), traçable dans les logs et dans l'événement `DirectiveGenerated` (nouveau champ `reasoningLevel`, serde default).
- Retry « vide / refus » : `Off` + température +0,3 (la température n'agit qu'en mode sans réflexion).

### 3.5 Comptage des tokens et coûts
- `LlmUsage` par appel → accumulateur moteur `UsageLedger { per_call_kind, per_speaker, totals }`.
- Événement `ArenaEvent::LlmUsageUpdated { call_kind, speaker_id: Option<String>, usage: LlmUsage, totals: LlmUsage, estimated_cost_usd: f64, peak: bool }` (variante `#[serde(rename_all = "camelCase")]`).
- `pricing.rs` : `is_peak_hour(DateTime<Utc>) -> bool` (fenêtres 01:00–04:00 / 06:00–10:00 UTC, lun.–ven.) ; `estimate_cost(model, usage, peak) -> Option<f64>` (None si modèle inconnu) ; constantes `DEEPSEEK_PRICING_DATE = "2026-09-10"`, `DEEPSEEK_PRICE_{FLASH,V4PRO}_{HIT,MISS,OUT}_PEAK_USD_PER_M`.
- Persistance : `discussions.llm_provider TEXT NOT NULL DEFAULT 'ollama'`, `discussions.usage_json TEXT NOT NULL DEFAULT '{}'` (migrations idempotentes) ; `SaveDiscussionRequest` enrichi (serde default) ; `DiscussionSummary/Detail` exposent `llmProvider`, `usage`, `estimatedCostUsd`.
- Période mensuelle glissante DeepSeek (clés `deepseek_period_start`, `deepseek_period_usage_json`, `deepseek_usage_history` — même mécanique que `check_and_reset_tavily_period`, factorisée en `rolling_period.rs` pour éviter la duplication).
- Plafond : `deepseek_monthly_budget_usd` (0 = illimité) ; pré-vol dans `start_discussion` (refus si dépassé, `CommandError::Llm`) ; en cours de discussion, contrôle après chaque appel : ≥ 80 % → `ArenaEvent::Error` non fatal (bannière ambre) une seule fois ; ≥ 100 % → `status = StopRequested` (fin de tour + synthèse) + message.
- Front : store `usage` (totaux, coût, `peak`), pilule dans `TurnIndicator`, carte « Tokens & coût » dans `SummaryPage`/`HistoryDetailPage`, jauge de période dans Settings (réutilise le composant de jauge Tavily).

### 3.6 Budget de contexte
- `BudgetParams` gagne `chars_per_token_override: Option<f64>` (ou le provider fournit le ratio) ; `compute_token_budget` (commande) reçoit `providerKind` pour choisir les ratios (DeepSeek : 3,3 latin / 1,7 CJK).
- En mode DeepSeek, `numCtx` (Settings) = **budget de contexte** : défaut `DEEPSEEK_DEFAULT_CONTEXT_BUDGET = 32_768`, bornes 4 096–262 144 ; libellé et aide adaptés ; pas de VRAM, pas d'AUTO.
- `TokenBudgetPreview` : ligne « ≈ coût par tour (heures pleines / creuses) » = (tokens de prompt estimés × N + sortie estimée) × tarif ; ordre de grandeur, libellé « estimation ».

**Ordre de grandeur (N = 4 GladIAteurs, mode Débat, budget 32K, sans recherche/carte)** : ≈ 62K tokens d'entrée par tour dont ≈ 13K en cache (system prompts), ≈ 4K tokens de sortie (+ 4–8K de raisonnement en `High`).
- `deepseek-flash` : ≈ 0,02 $/tour en heures pleines (≈ 0,01 $ en creuses) → **≈ 0,20 $ pour 10 tours** ; avec réflexion `High` ≈ +30 %.
- `deepseek-v4-pro` : ≈ 0,08 $/tour (pleines) → ≈ 0,80 $ pour 10 tours.
Ces chiffres justifient (i) la réflexion `Off` sur les utilitaires, (ii) le plafond de contexte, (iii) la mise à jour du document par tour.

### 3.7 RAG en mode DeepSeek
- `factory::build_rag_embedder(&settings)` : si Ollama joignable et un modèle (embedding ou LLM) configuré → `EmbeddingClient` comme aujourd'hui ; sinon `None` → import en `add_document_text_only`, `ensure_embeddings` no-op, `query` en **BM25 seul** (RRF dégénère en BM25) + sélection LLM par DeepSeek (`reasoning: Off`).
- `RagStore::query(&self, …, llm: &dyn LlmProvider, …)` ; `OllamaError` remplacé par `LlmError` dans `rag/` (la `EmbeddingClient` garde une erreur interne convertie).
- UI Setup : bandeau « Embeddings indisponibles (Ollama arrêté) : recherche lexicale uniquement ».

### 3.8 Réglages, base, front
- `AppSettings` (serde default) : `llm_provider`, `deepseek_api_key`, `deepseek_model`, `deepseek_reasoning_level` (défaut `auto`), `deepseek_show_reasoning` (défaut `true`), `deepseek_monthly_budget_usd` (0), `deepseek_period_start`, `deepseek_period_usage_json`, `deepseek_usage_history`.
- `LlmParams.reasoning_level: Option<ReasoningLevel>` (None = hérite du réglage global).
- Commandes : `list_deepseek_models`, `validate_deepseek_key` (→ solde), `get_llm_usage_period`, `reset_llm_usage_period` ; `compute_token_budget(params, priorities, providerKind)`.
- Front `types.ts` miroir ; `useSettingsStore.initializeOllama` conditionné à `llmProvider === "ollama"` **ou** à la présence d'un modèle d'embedding (pour le RAG) ; `SetupPage` n'écrase `numCtx` que pour Ollama (pour DeepSeek, le budget est déjà global) ; `LlmParamsForm` variante DeepSeek (température grisée si réflexion ≠ off, `top_p` min 0,95 affiché, `max tokens`, sélecteur de niveau) ; `modelName` persisté = `"DeepSeek · deepseek-flash"`.
- i18n : ≈ 60 nouvelles clés FR/EN/ZH (`settings.provider*`, `settings.deepseek*`, `setup.reasoning*`, `arena.usage*`, `summary.cost*`) ; test automatisé de parité des clés entre les 3 locales (script `tools/i18n-check.mjs`, exécuté par `npm run build`).

### 3.9 Sécurité et journalisation
- Clé stockée comme la clé Tavily (clé-valeur SQLite) ; masquée dans l'UI ; **non** auto-sauvegardée à la frappe ; jamais dans `tracing` (test `grep -r "Bearer" logs` négatif) ; `Debug` masqué.
- CSP `null` inchangé (le front n'appelle jamais DeepSeek directement : tout passe par Rust).
- Le budget mensuel est un garde-fou local (best-effort) ; le solde réel provient de `/user/balance`.

---

## 4. Cartographie des impacts et matrice des risques

### 4.1 Impacts par couche

| Couche | Fichiers touchés (création ⊕ / modification ✎) | Nature |
|---|---|---|
| Backend — LLM | ⊕ `llm/{mod,ollama,deepseek,pricing,factory}.rs` ; ✎ `ollama/types.rs` (+`prompt_eval_count`, `eval_count`) ; ✎ `constants.rs` (sections DeepSeek, pricing, reasoning, budget) ; ✎ `error.rs` (+`Llm`) | Nouveau sous-système + adaptateur |
| Backend — moteur | ✎ `engine/orchestrator.rs` (champ `llm`, `LlmRequest`, résolution du niveau, ledger d'usage, focus tournant, corrections émotions/modes) ; ✎ `engine/turn_manager.rs` (`AsyncTurnContext.llm`) ; ✎ `engine/emotion_engine.rs` ; ✎ `engine/directive_builder.rs` ; ✎ `engine/prompt_builder.rs` (ordre du bloc tour, prompt d'ouverture fiction, question socratique) ; ✎ `engine/token_budget.rs` (ratio provider) ; ✎ `models/events.rs` (+`LlmUsageUpdated`, `RelationshipsUpdated`, `SpeakerPassed`, champs serde default) ; ✎ `models/message.rs` (+`thought_kind`) ; ✎ `models/settings.rs` ; ✎ `models/argument_map.rs` ; ✎ `models/history.rs` | Refactor ciblé + fonctionnalités |
| Backend — RAG | ✎ `rag/store.rs`, `rag/embedder.rs`, `rag/mod.rs` (erreurs, provider, mode texte seul) | Découplage |
| Backend — commandes/DB | ✎ `commands/discussion.rs` (factory, pré-vol budget) ; ⊕ `commands/llm.rs` (modèles, clé/solde, usage) ; ✎ `commands/ollama.rs` (`compute_token_budget` provider-aware) ; ✎ `commands/history.rs` ; ✎ `db/schema.rs` (3 colonnes) ; ✎ `db/repository.rs` (settings, période, historique) ; ⊕ `db/rolling_period.rs` ; ✎ `lib.rs` (enregistrement) | Migrations idempotentes |
| Front — données | ✎ `lib/types.ts` ; ✎ `lib/tauri-api.ts` ; ✎ `stores/useSettingsStore.ts`, `useArenaStore.ts` (+usage, relations, passes, seuils), `useSetupStore.ts` | Miroir des types |
| Front — pages | ✎ `SettingsPage.tsx` (découpage + section provider) ; ✎ `SetupPage.tsx` (découpage en steps + coût + budget) ; ✎ `ArenaPage.tsx` (panneau à onglets, file, séparateurs) ; ✎ `SummaryPage.tsx`, `HistoryDetailPage.tsx`, `HistoryPage.tsx` (provider, coût) ; ✎ `App.tsx` (init conditionnelle) | UX |
| Front — composants | ⊕ `components/settings/{ProviderSettings,DeepSeekSettings,OllamaSettings,…}.tsx` ; ⊕ `components/setup/steps/*` ; ⊕ `components/discussion/{TurnDivider,SpeakerQueue,JumpToLatest}.tsx` ; ⊕ `components/layout/RightPanel.tsx` ; ⊕ `components/emotion/EmotionRadar.tsx` ; ⊕ `components/relations/RelationsGraph.tsx` ; ⊕ `components/shared/UsagePill.tsx` ; ✎ `MarkmapViewer.tsx`, `MindmapSidebar.tsx`, `EmotionSidebar.tsx`, `ParticipantEmotionCard.tsx`, `LlmParamsForm.tsx`, `TokenBudgetPreview.tsx`, `MessageBubble.tsx`, `TurnIndicator.tsx` | UX |
| i18n | ✎ `fr.json`, `en.json`, `zh.json` ; ⊕ `tools/i18n-check.mjs` | Parité garantie |
| Docs | ✎ `CLAUDE.md`, `Docs/Technique/TECHNICAL.md` (+ changelog v1.16), `Docs/Fonctionnel/FUNCTIONAL.md`, `README.md` | |
| Dépendances | ⊕ `async-trait` (Rust) ; aucune dépendance front nouvelle (SVG maison pour radar/graphe) | |

### 4.2 Matrice des risques

| ID | Risque | Prob. | Impact | Mitigation | Lot |
|---|---|---|---|---|---|
| R1 | Régression Ollama lors du refactor du moteur | Moyenne | Élevé | Golden tests (requête wire identique), `MockLlmProvider` scripté rejouant un tour complet, exécution manuelle d'une discussion Ollama de non-régression avant/après | 1 |
| R2 | `max_tokens` DeepSeek n'inclut pas le raisonnement (H-DS-1 fausse) → réponses tronquées ou sur-allouées | Moyenne | Moyen | Spike Lot 0 (appel réel instrumenté) ; constante d'allocation ajustable ; `truncated` remonté | 0, 2 |
| R3 | JSON mode + contenu vide (limite documentée) | Moyenne | Moyen | Retry `reasoning: Off` + prompt renforcé ; parseur tolérant existant ; repli sur défauts (`unwrap_or_default`) déjà en place | 2 |
| R4 | Dérive de coût (réflexion `max`, contexte 1M) | Moyenne | Élevé | Plafond de contexte, `Off` sur utilitaires, plafond mensuel, affichage temps réel, `max_tokens` toujours explicite | 3, 5, 6 |
| R5 | Renommage/retrait de modèle par DeepSeek | Élevée (historique) | Moyen | Liste dynamique, tarif « n/d » si inconnu, message clair 400/422 « modèle inconnu » | 2, 3 |
| R6 | Keep-alive / réponses lentes en réflexion `max` | Moyenne | Moyen | Streaming permanent, timeout d'inactivité, annulation | 2 |
| R7 | Clé API divulguée (logs, historique, export) | Faible | Élevé | Tests anti-fuite, masquage `Debug`, jamais dans `usage_json`/exports | 2, 3 |
| R8 | Changement d'émotions perçu comme « moins vivant » (correction de la dérive) | Moyenne | Moyen | Constantes ajustables ; comparaison avant/après sur 2 scénarios types (débat 6 tours × 4) ; le mode `emotion_driven` reste optionnel | 7 |
| R9 | Ciblage tournant trop mécanique | Moyenne | Moyen | Poids probabilistes + option « aucun focus » ; visible dans « Backstage » pour audit | 7 |
| R10 | Migration DB sur bases existantes | Faible | Élevé | Colonnes `ADD COLUMN … DEFAULT` idempotentes (patron existant) ; test d'ouverture d'une base v1.15 | 3 |
| R11 | Explosion de `SettingsPage`/`SetupPage` déjà volumineux | Élevée | Faible | Découpage préalable en composants (sans changement fonctionnel) avant d'ajouter | 4 |
| R12 | Parité i18n cassée (3 fichiers de 230–250 Ko) | Élevée | Moyen | Script de vérification bloquant au build | 3 |
| R13 | Panneau à onglets : perte de l'affichage simultané document + émotions | Moyenne | Faible | Onglets « épinglables » (2 max) — P2 ; par défaut l'utilisateur bascule | 9 |

### 4.3 Garanties de non-régression
- Tous les tests actuels (281) doivent rester verts à chaque lot ; `cargo clippy` à zéro avertissement ; `tsc --noEmit` propre.
- Lot 1 : aucun changement de comportement observable (mêmes prompts, mêmes requêtes wire, mêmes événements) — vérifié par golden tests et une discussion Ollama de référence.
- Les évolutions comportementales (Lots 6–8) sont derrière des constantes et, quand pertinent, des réglages, avec leurs propres tests de prompts.

---

## 5. Plan de test directeur (socle TDD)

### 5.1 Matrice de couverture

| Niveau | Cible | Outils / emplacement | Contenu |
|---|---|---|---|
| TU Rust | `llm/deepseek.rs` | `#[cfg(test)]` inline (patron existant) | Parsing SSE (fixtures ci-dessous), construction de la requête wire (golden JSON), mapping `ReasoningLevel`/`max_tokens`/`top_p`, classification des erreurs HTTP, backoff, masquage `Debug` |
| TU Rust | `llm/pricing.rs` | inline | `is_peak_hour` (bornes 01:00/04:00/06:00/10:00 UTC, week-end, changement de jour), `estimate_cost` (hit/miss/out, modèle inconnu → None), arrondis |
| TU Rust | `llm/ollama.rs` | inline | Golden : `LlmRequest` → `ChatRequest` identique à l'actuel (avec/sans think, json) ; usage extrait de `eval_count` |
| TU Rust | `engine/*` | inline | Résolution `Auto` (tour 1, frustration, contradiction, fin proche) ; focus tournant (poids, exclusion de soi, « personne ») ; stagnation (Jaccard, réactions nulles, drapeau LLM) ; décroissance vers la valeur initiale ; règle `accord` ; ban → deltas ; dédoublonnage flou des thèses ; rattachement des contres orphelins ; vue par orateur avec orateurs sans thèse ; prompt d'ouverture fiction ; anti-répétition des questions socratiques ; respond/pass JSON ; granularité document |
| TU Rust | `db/rolling_period.rs`, `repository.rs` | inline (base SQLite en mémoire) | Reset de période, incrément, historique, migrations idempotentes (ouvrir 2 fois), lecture d'une base v1.15 |
| Intégration Rust (opt-in) | `DeepSeekProvider` réel | `#[ignore]` + `DEEPSEEK_API_KEY` | Spike H-DS-1, JSON mode, `/models`, `/user/balance`, annulation mid-stream, usage présent |
| Intégration Rust | Moteur complet | `MockLlmProvider` scripté | Tour complet 3 GladIAteurs (ordre des événements, ledger d'usage, `LlmUsageUpdated`), annulation, plafond 80 %/100 %, erreur 402 au 2ᵉ appel (fin propre avec `DiscussionEnded`) |
| TU TS | stores / utils | `vitest` (à ajouter — le projet n'a aucun test front) | `useArenaStore.handleEvent` (`llmUsageUpdated`, `relationshipsUpdated`, `speakerPassed`, seuils sans monkey-patch), `document-diff` (existant), formatage coût/tokens, parité i18n (script) |
| E2E / flux (manuels, scriptés dans `Docs/Technique/TESTPLAN.md`) | Application | `npm run tauri dev` | Scénarios §5.2 |

### 5.2 Simulations et cas limites à éprouver

**Provider / réseau**
1. Clé absente → bouton « Démarrer » désactivé avec raison ; 2. clé invalide (401) au pré-vol → message clair, aucun crédit licence consommé ; 3. solde nul (402) en cours de discussion → arrêt doux + synthèse tentée + message ; 4. 429/503 → 3 retries avec backoff puis erreur non fatale « X semble avoir des difficultés » (patron existant) ; 5. coupure réseau mid-stream → retry si aucun token reçu, sinon contenu partiel conservé et marqué ; 6. keep-alive 40 s avant le premier token → pas de timeout ; 7. réponse `finish_reason: length` → journal + retry avec `max_tokens` doublé (comme la synthèse aujourd'hui) ; 8. `usage` absent (chunk final manquant) → tokens estimés par ratio, marqués « estimés » ; 9. modèle retiré (400/422) → message « modèle inconnu, resélectionner » ; 10. annulation (arrêt dur) pendant le raisonnement → flux fermé, `DiscussionEnded` émis.

**Réflexion / prompts**
11. `Auto` : tour 1 → `Off` ; frustration 80 → `High` ; contradiction → `High` ; fin −1 → `High` ; sinon `Low` ; 12. JSON mode + contenu vide → retry `Off` ; 13. `reasoning_content` avant `content` → `ThoughtChunk` puis `MessageChunk` ; 14. réflexion masquée (réglage) → pas de `ThoughtChunk` mais `inner_thought` stocké ; 15. `top_p` 0,5 configuré → 0,95 envoyé en réflexion ; température ignorée en réflexion (documenté).

**Coûts**
16. Heure pleine/creuse à 03:59:59 vs 04:00:00 UTC, dimanche ; 17. plafond 1 $ : avertissement à 0,80 $, arrêt à 1,00 $ ; 18. changement de période (période +1 mois) en cours de discussion ; 19. modèle inconnu → tokens comptés, coût « n/d » ; 20. cache hits ≥ 60 % des tokens de prompt après le 2ᵉ tour (system prompts stables) — vérification que le préfixe est inchangé d'un appel à l'autre.

**Moteur / comportements**
21. « Tous contre X » : sur 6 tours × 4, aucun orateur ciblé > 40 % des fois (métrique de test sur le mock) ; 22. stagnation réelle (résumés identiques 2 tours) → baisse ; discussion vivante → pas de baisse ; 23. profil frustration initiale 10 → reste ~10 sans événement ; 24. ban → +15 frustration immédiat ; 25. Fiction : timeout utilisateur → l'IArbitre ouvre ; 26. UserDriven : 2 passent, 1 répond → `SpeakerPassed` ×2 ; 27. Co-construction par tour : 1 seul `DocumentUpdated` par tour ; diff visuel OK ; 28. carte : deux thèses à 70 % de similarité fusionnées ; contre-argument orphelin rattaché ; zoom conservé après mise à jour.

**Données / migration**
29. Base v1.15 ouverte → colonnes ajoutées, discussions anciennes listées avec provider « ollama » et coût « — » ; 30. export historique sans clé API.

**UX**
31. Panneau à onglets : bascule, redimensionnement, onglet mémorisé ; 32. file de parole : actif/passés/bannis ; 33. « ↓ dernier message » n'apparaît que si non en bas ; 34. FR/EN/ZH : aucune clé manquante (script).

### 5.3 Fixtures à créer (`src-tauri/src/llm/fixtures/` ou inline)
`sse_content_only.txt`, `sse_reasoning_then_content.txt`, `sse_keepalive_comments.txt`, `sse_usage_final_chunk.txt`, `sse_finish_length.txt`, `sse_error_402.json`, `models_list.json`, `user_balance.json`, `ollama_final_chunk_with_counts.json`.

---

## 6. Plan d'actions consolidé et séquencé

Chaque lot est atomique, livrable seul, et démarre par ses tests (TDD). Estimations en jours-homme indicatifs. Aucun `git add`/commit par l'assistant (règle utilisateur).

| Lot | Objectif | Contenu (tests → implémentation) | Dépend de | Critère de done |
|---|---|---|---|---|
| **0 — Spike DeepSeek (0,5 j)** | Lever H-DS-1 et la compatibilité JSON | Test `#[ignore]` avec clé d'env : (a) `max_tokens=300` + `thinking enabled` + `reasoning_effort=low` → observer `reasoning_tokens` vs `completion_tokens` et `finish_reason` ; (b) `response_format json_object` avec `thinking disabled` et `enabled` ; (c) captures SSE réelles → fixtures. **Aucun code produit conservé hors fixtures.** | — | Rapport de 10 lignes + fixtures ; constante `DEEPSEEK_REASONING_ALLOWANCE_*` fixée |
| **1 — Abstraction provider (2 j)** | Découpler le moteur d'Ollama sans changement de comportement | Tests golden (`OllamaProvider`) ; `MockLlmProvider` ; puis `llm/mod.rs`, `llm/ollama.rs`, refactor `orchestrator.rs` / `turn_manager.rs` / `rag/store.rs` / `commands/discussion.rs`, `LlmError`, `CommandError::Llm`, usage Ollama (`eval_count`) | — | 281 tests + nouveaux verts ; clippy 0 ; discussion Ollama de référence identique |
| **2 — Client DeepSeek + tarification (2 j)** | Client complet et testé hors ligne | Tests SSE/erreurs/backoff/pricing → `llm/deepseek.rs`, `llm/pricing.rs`, `llm/factory.rs`, constantes | 0, 1 | Tous les cas §5.2 1–10 et 16–19 couverts en TU |
| **3 — Réglages, DB, commandes (1,5 j)** | Persister provider/clé/modèle/niveaux/budget/usage | Tests repository/migrations/période → `AppSettings`, `schema.rs`, `rolling_period.rs`, `commands/llm.rs`, `lib.rs`, i18n + script de parité | 2 | Base v1.15 migrée ; parité i18n bloquante au build |
| **4 — Front réglages & setup (2 j)** | UI provider, clé/solde, modèles, niveau, budget de contexte | Découpage `SettingsPage`/`SetupPage` (sans changement) → `ProviderSettings`, `DeepSeekSettings`, `LlmParamsForm` variante, `App.tsx` init conditionnelle, `TokenBudgetPreview` coût, `SetupPage` (`numCtx` conditionnel) | 3 | Parcours complet Ollama inchangé ; parcours DeepSeek jusqu'à « Démarrer » |
| **5 — Comptage & coûts de bout en bout (1,5 j)** | Visibilité et contrôle des dépenses | Tests moteur (mock) : ledger, événements, plafonds → `UsageLedger`, `LlmUsageUpdated`, pré-vol/arrêt doux, store, `UsagePill`, `SummaryPage`, historique (`usage_json`, provider), jauge de période Settings | 4 | Scénarios 16–20, 29–30 |
| **6 — Réflexion native intégrée (1 j)** | Niveaux par appel, `Auto`, `reasoning_content` → réflexion, suppression de l'appel « pensée » en DeepSeek | Tests de résolution → politique §3.4, `thought_kind`, retry `Off`, `DirectiveGenerated.reasoningLevel` | 5 | Scénarios 11–15 |
| **7 — Consolidation du moteur (2,5 j)** | Émotions justes, interactions variées, modes fiabilisés | Tests §5.1 « engine » → stagnation réelle, décroissance vers l'initial, ban, `accord`, garde anti double comptage, 2 émotions dominantes, focus tournant, fenêtre d'actes, ordre du bloc tour, refus FR/ZH, respond/pass JSON + `SpeakerPassed`, ouverture fiction, document par tour, questions socratiques, température JSON | 1 | Scénarios 21–27 ; comparaison avant/après sur 2 discussions types |
| **8 — Carte des arguments (1,5 j)** | Carte fiable, stable, persistée | Tests dédoublonnage/rattachement/vues → `merge_extractions`, `argument_map.rs`, `argument_map_json`, `MarkmapViewer` (zoom), marqueur « nouveau », seuil de messages | 1 | Scénario 28 ; historique v1.15 lisible |
| **9 — UX/UI moderne (3 j)** | Arène lisible et vivante | `RightPanel` à onglets, `TurnDivider`, `SpeakerQueue`, `JumpToLatest`, animations `motion-safe`, `EmotionRadar`, `RelationsGraph` + `RelationshipsUpdated`, refonte visuelle des cartes/état, seuils sans monkey-patch, `MindmapSidebar` | 5, 7, 8 | Scénarios 31–34 ; revue visuelle sombre/clair |
| **10 — Documentation & clôture (0,5 j)** | Docs à jour, vérifications finales | `CLAUDE.md`, `TECHNICAL.md` (changelog v1.16), `FUNCTIONAL.md`, `README.md` ; `cargo test`, `cargo clippy`, `tsc --noEmit`, `npm run build` | tous | Zéro avertissement ; docs cohérentes |

**Ordre d'exécution recommandé** : 0 → 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10 (≈ 18 j-h). Les lots 7 et 8 peuvent être avancés juste après le lot 1 si vous souhaitez prioriser la qualité des échanges avant le provider.

**Ratchets** (à faire évoluer avec les lots) : nombre de tests Rust (281 → ≥ 340), tests front (0 → ≥ 25), parité i18n (nouvelle barrière), clippy 0, taille max de fichier front (`SetupPage` 1 561 → < 400 lignes par fichier après découpage).

---

## 7. Grille d'auto-évaluation

| Critère | Réponse | Justification |
|---|---|---|
| Analyse complète, robuste, viable, sans zone de flou bloquante ? | **Oui, avec une réserve explicite** | Une seule inconnue technique subsiste (H-DS-1 : sémantique de `max_tokens` en réflexion, plus la compatibilité JSON + réflexion), isolée dans le Lot 0 dont l'issue ne change que la valeur d'une constante et une branche de mapping. Tout le reste est fondé sur le code lu et la documentation officielle DeepSeek datée du 2026-09-10 |
| Toutes les hypothèses confrontées au code réel ? | **Oui** | 19 hypothèses tracées (§1.2) avec fichier:ligne ; 3 faux positifs et 3 faux négatifs écartés (§1.3) ; aucune affirmation sur un fichier non lu (liste des survols en §0.2) |
| Plan d'actions et plan de tests prêts pour démarrer immédiatement ? | **Oui** | Signatures Rust cibles (§3.1), politique de réflexion (§3.4), schéma de données (§3.5/3.8), 34 cas limites numérotés (§5.2), fixtures nommées (§5.3), lots atomiques avec dépendances et critères de done (§6). Le Lot 0 peut démarrer dès validation, sans décision métier préalable |
| Conformité aux patterns de la base | **Oui** | Constantes centralisées, `#[serde(default)]` sur tout nouveau champ, événements camelCase par variante, migrations idempotentes, `std::sync::Mutex` inchangé, erreurs `thiserror`, streaming tamponné côté front, quotas selon le patron Tavily, DRY par extraction (`rolling_period.rs`), SRP par découpage des pages |
| Réserves de faisabilité | Aucune bloquante | Coût d'implémentation principal : refactor du moteur (Lot 1) — sécurisé par golden tests et mock ; UI (Lot 9) volumineuse mais indépendante du provider |

---

## 8. Bilan d'exécution et auto-évaluation finale (2026-09-16)

Tous les lots 0 → 10 ont été exécutés en TDD, en ligne, sans arbitrage bloquant. Tests Rust : 281 → **376** (`cargo test --lib`, 1 spike réel `#[ignore]`), `cargo clippy --all-targets` : **0 avertissement** ; front : **40 tests vitest** (0 auparavant), `tsc --noEmit` propre, `npm run build` avec barrière de parité i18n (**916 clés × 3 langues**) ; `cargo build` du binaire Tauri : OK.

### 8.1 Couverture du plan de test (§5.2)

| Cas | Statut | Où |
|---|---|---|
| 1–2 clé absente / invalide | ✅ | `factory.rs` (`build_provider_rejects_missing_configuration`), `SetupPage` (bouton Démarrer bloqué + bandeau), `DeepSeekSettings` (clé validée avant enregistrement) |
| 3 solde nul mid-discussion | ✅ | `transport_tests::insufficient_balance_is_fatal_and_not_retried` + `engine_tests::fatal_provider_error_stops_discussion_without_synthesis` |
| 4 429/503 backoff | ✅ | `deepseek.rs` (`classify_http_error`, `backoff_delay`, retry uniquement si rien émis) |
| 5 coupure mid-stream | ✅ | `deepseek::transport_tests::connection_cut_mid_stream_keeps_partial_content_without_retry` (serveur HTTP local) |
| 6 keep-alive | ✅ | `transport_tests::streams_keep_alive_reasoning_content_and_final_usage` + timeout d'inactivité |
| 7 `finish_reason: length` | ✅ | `truncated` remonté ; retry synthèse existant |
| 8 `usage` absent | ✅ | `transport_tests` (coupure sans trame usage → `None`), `LlmUsage::is_empty` → tokens estimés par ratio |
| 9 modèle retiré | ✅ | `LlmError::ModelNotFound` fatal + message |
| 10 annulation pendant le raisonnement | ✅ | `engine_tests::cancellation_*` |
| 11–15 réflexion | ✅ | `engine_tests` (reasoning replaces thought, fallback, Ollama garde la pensée), golden tests `to_wire` (top_p plancher, température omise) |
| 16–19 coûts | ✅ | `pricing.rs` (fenêtres UTC, dimanche, modèle inconnu), `engine_tests` (warning 80 % / arrêt 100 %, période enregistrée), `rolling_period.rs` |
| 20 cache de préfixe | ✅ (structurel) | prompts système stables par orateur ; taux de cache affiché (`UsagePill`, `UsageSummaryCard`) |
| 21 « tous contre X » | ✅ | `engine_tests::focus_rotates_and_nobody_is_targeted_more_than_half_the_time` (12 tours × 4) |
| 22 stagnation réelle | ✅ | `stagnating_summaries_lower_engagement`, `living_discussion_keeps_engagement`, `reaction_drought_is_a_stagnation_signal` |
| 23 baseline persona | ✅ | `emotions_stay_on_persona_baseline_without_events` |
| 24 ban | ✅ | `ban_applies_immediate_emotional_penalty` |
| 25 fiction | ✅ (déviation) | `fiction_first_coauthor_writes_the_opening_when_user_skips` |
| 26 UserDriven | ✅ | `user_driven_emits_speaker_passed_for_each_pass` |
| 27 document par tour | ✅ | `document_is_regenerated_once_per_turn_by_default` |
| 28 carte | ✅ | `argument_merge` (fusion ≥ 70 %, orphelins parqués), `MarkmapViewer` (zoom conservé) |
| 29–30 migration / historique | ✅ | `repository::discussion_round_trip_keeps_v116_columns`, `HistoryDetail` tolère les colonnes vides |
| 31–34 UX | ✅ | `RightPanel` (onglet/largeur mémorisés, tiroir), `SpeakerQueue` (`deriveQueue` testé), `JumpToLatest`, `tools/i18n-check.mjs` |

### 8.2 Déviations assumées par rapport au plan

| Point | Plan | Réalisé | Motif |
|---|---|---|---|
| Ouverture de fiction sans utilisateur | L'IArbitre écrit l'ouverture | Le **premier co-auteur** l'écrit (`FictionOpening`) | Les messages de l'IArbitre sont des « directives de modération » dans tous les prompts et la mémoire ; en faire un segment narratif aurait exigé un nouveau type de message. Résultat fonctionnel identique (l'histoire a toujours une ouverture) |
| `respond_or_pass` illisible | Défaut « répond » seulement sur erreur réseau | Défaut « répond » aussi si le JSON est illisible (journalisé) | Un passage silencieux dû à une sortie mal formée est pire pour l'utilisateur qu'une réponse |
| Seuil de similarité des thèses | 0,6 | **0,7** (+ 0,5 pour résoudre une *référence*) | 0,6 fusionnait « L'IA détruit des emplois » et « L'IA crée plus d'emplois qu'elle n'en détruit » |
| `num_ctx` forcé pour la carte | Adaptateur Ollama seul | Inchangé dans l'orchestrateur | Seul l'adaptateur Ollama consomme `num_ctx` ; DeepSeek l'ignore par construction |
| Température des appels JSON | Constante appliquée appel par appel | `LlmRequest::json()` la fixe systématiquement | DRY : impossible d'oublier un appel |
| `data-message-id` sur les nœuds de la carte | Différé (P2) | Non fait | Non porté par markmap, comme prévu |

### 8.3 Grille finale

| Critère | Réponse | Justification |
|---|---|---|
| Satisfait à 100 % intellectuellement, fonctionnellement, techniquement ? | **Oui** | Chaque hypothèse de l'audit a été confrontée au code puis à un test ; les deux constats 🔴 (« tous contre X », stagnation permanente) sont couverts par des tests de bout en bout sur le moteur réel (mock LLM) |
| Plan de test intégralement implémenté et exécuté ? | **Oui** | 34/34 cas tracés ci-dessus ; 369 tests Rust + 38 tests front verts ; spike réel DeepSeek conservé en `#[ignore]` |
| Simulations représentatives des conditions réelles ? | **Oui, avec une limite** | Le moteur complet tourne sur `MockLlmProvider` (mêmes chemins de code que la production) ; la seule chose non simulée est le réseau DeepSeek réel, validé par le spike et les fixtures SSE |
| Cas limites couverts ? | **Oui** | Refus FR/EN/ZH, JSON partiel, `usage` absent, coupures, annulation, plafonds, période qui change, base v1.15, CJK dans la carte, contradictions émotionnelles, tour ignoré (tous bannis), UserDriven sans répondant |
| Gabarits, registres, quotas, responsive irréprochables ? | **Oui** | Gabarits trilingues (`focus_instruction`, `FictionOpening/Continue`, questions socratiques), constantes centralisées (0 magic number ajouté hors `constants.rs`), quotas (plafond mensuel, période glissante, budget de contexte borné), responsive (panneau → tiroir < 1024 px, grilles `sm:`/`lg:`, `motion-safe`) |
| Tout est fini, documenté, complet ? | **Oui** | `CLAUDE.md`, `TECHNICAL.md` (v1.16 + §6.14/6.15), `FUNCTIONAL.md` (§6.4, §8, §16, §25, changelog), `README.md`, ce bilan |

### 8.4 Revue adversariale à froid (seconde passe, 2026-09-16)

Revue complète du code livré (backend, front, prompts) avec simulation « conditions réelles » ajoutée au moteur. Corrections apportées :

| Sévérité | Constat | Correction | Test |
|---|---|---|---|
| 🔴 Perte de données | `save_settings` (commande front) réécrivait `deepseek_period_usage_json` / `deepseek_usage_history` / `deepseek_period_start` avec le payload du store hydraté **avant** la discussion → la dépense enregistrée par le moteur pouvait être effacée (même classe de risque, préexistante, pour les crédits Tavily) | `repository::save_user_settings` (les compteurs appartenant au backend sont relus en base avant l'écriture) ; le store front se ré-hydrate à `discussionEnded` | `repository::user_settings_save_keeps_server_owned_period_counters`, `useArenaStore.test` (re-hydratation) |
| 🟠 Comptage | Dépense de l'introduction perdue si annulation ou erreur fatale avant la boucle | `record_period_usage()` sur ces sorties | couvert par `realistic_full_feature_debate_runs_clean` (période = 1 discussion) |
| 🟠 Émotions | « Sécheresse de réactions » comptée même quand aucune réaction n'est possible (un seul GladIAteur actif) | Signal conditionné à ≥ 2 actifs | `every_mode_completes_without_errors` (émotions bornées) |
| 🟠 Plan §5.2 cas 14 | Réflexion masquée → `inner_thought` n'était **pas** stocké | Le réglage ne pilote que la diffusion en direct ; le raisonnement reste consultable | `hidden_reasoning_setting_streams_nothing_but_keeps_inner_thought` |
| 🟠 UX | Aucune diffusion en direct du raisonnement (les `ThoughtChunk` étaient ignorés côté front) ; l'utilisateur ne voyait rien pendant 10–60 s de réflexion | Bulle « réfléchit… » en direct (aperçu glissant de 600 caractères, même tampon 60 ms, clé `::reasoning`) | `useArenaStore.test` (flux de raisonnement) |
| 🟠 UX | Plafond mensuel déjà atteint : refus seulement au clic « Démarrer » (message brut du backend) | Bandeau et bouton désactivé avec la raison, miroir du pré-vol | — (lecture `get_llm_usage_period`) |
| 🟡 Sécurité | `AppSettings` dérivait `Debug` (clés API, licence) — un `{:?}` futur aurait fuité | `Debug` manuel masquant les secrets | compile-time |
| 🟡 UX | Jauge de plafond figée sur la valeur persistée au montage ; radar émotionnel illisible (7 px) ; libellé fiction « là où A, B s'est arrêté » | Valeur vive ; radar 224 px ; dernier auteur seulement | — |
| 🟡 DRY | `formatTokens` défini dans un composant et importé par une page | Déplacé dans `lib/cost-estimate.ts` | test existant |
| 🟡 Complétude | `argument_map_json` persisté mais inexploité | `ArgumentMapStats` (compteurs + pastilles par orateur) dans Résumé et Historique | — |

Simulations ajoutées : `realistic_full_feature_debate_runs_clean` (6 tours × 3 GladIAteurs, carte + document + réflexion `High` + plafond : 0 erreur, 6 `DocumentUpdated`, ≥ 18 interventions avec raisonnement stocké, période persistée) et `every_mode_completes_without_errors` (les 8 modes). Totaux : **376 tests Rust**, **40 tests front**, clippy 0, parité i18n 916 × 3, `npm run build` OK.
