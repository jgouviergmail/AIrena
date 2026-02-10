# Plan: Intégration Wikipedia pour IArbitre et GladIAteurs

## Context

AIrena permet déjà aux participants (IArbitre et GladIAteurs) de rechercher des informations sur internet via l'API Tavily. L'utilisateur souhaite ajouter une source complémentaire : **Wikipedia** — encyclopédique, gratuite, sans clé API ni quota. Les deux sources (internet et Wikipedia) doivent pouvoir être activées indépendamment ou simultanément. Le paramétrage est identique : max 1 recherche par tour par gladiateur, budget configurable par discussion, option d'introduction pour l'IArbitre.

**API Wikipedia retenue** : `generator=search` + `prop=extracts` en un seul appel HTTP GET (pas d'auth, pas de clé, multilingue fr/en/zh). Validé par tests live sur les 3 langues.

---

## Fichiers à créer (3)

### 1. `src-tauri/src/wikipedia/mod.rs`
Structs de réponse — `#[derive(Debug, Clone, serde::Deserialize)]` + `#[serde(default)]` sur tous les champs :
- `WikiSearchResponse { query: Option<WikiQuery> }`
- `WikiQuery { pages: Vec<WikiPage> }`
- `WikiPage { title: String, pageid: u64, index: i32, extract: String }`
  - `index` = rang de pertinence (pour tri)
  - `title` = utilisé pour construire l'URL Wikipedia cliquable

### 2. `src-tauri/src/wikipedia/client.rs`
Client HTTP miroir de `tavily/client.rs` mais simplifié :
- `WikiClient` — `#[derive(Clone)]`, pas de clé API
- `WikiClient::new()` — reqwest avec timeout 10s + User-Agent via `format!("AIrena/{} (https://github.com/...) reqwest", env!("CARGO_PKG_VERSION"))`
- `search(query, discussion_language, cancel_token)` → `Result<WikiSearchResponse, WikiError>`
- URL : `https://{lang}.wikipedia.org/w/api.php?action=query&generator=search&gsrsearch={q}&gsrlimit=3&prop=extracts&exintro=1&explaintext=1&exchars=500&format=json&formatversion=2&maxlag=5`
- Mapping langue : "fr"→fr, "en"→en, "zh"→zh
- **Fallback zh→en** : si `discussion_language == "zh"` et la réponse a `query.pages` vide (ou `query` est `None`), refaire automatiquement la même requête sur `en.wikipedia.org`. Log un `tracing::info!("zh Wikipedia returned no results, falling back to en")`.
- Note: réponses erreur maxlag (HTTP 200 avec JSON `error`) gérées gracieusement — `query: Option` sera `None` → skip ou fallback

### 3. `src-tauri/src/wikipedia/error.rs`
Enum simplifié (pas de InvalidKey/QuotaExceeded/RateLimit — Wikipedia est gratuit) :
- `Http(u16, String)`, `Network(String)`, `Cancelled`

---

## Fichiers à modifier — Backend Rust (8)

### 4. `src-tauri/src/lib.rs`
- Ajouter `mod wikipedia;` à côté de `mod tavily;` (ligne 8)

### 5. `src-tauri/src/models/iarbitre.rs`
- Ajouter champ `wiki_search_intro: bool` avec `#[serde(default)]` dans `IArbitreConfig` (après `web_search_intro`)

### 6. `src-tauri/src/models/discussion.rs`
- Ajouter champ `wiki_search_max_per_gladiateur: u32` avec `#[serde(default)]` dans `DiscussionConfig` (après `web_search_max_per_gladiateur`)

### 7. `src-tauri/src/models/gladiateur.rs`
- Ajouter champ `wiki_searches_used_discussion: u32` dans `GladIAteurState` (après `web_searches_used_discussion`)
- Initialiser à `0` dans `GladIAteurState::new()`

### 8. `src-tauri/src/models/events.rs`
- Ajouter variant `WikiSearchPerformed` avec `#[serde(rename_all = "camelCase")]` :
  ```rust
  WikiSearchPerformed {
      speaker_id: String,
      speaker_name: String,
      queries: Vec<String>,
      results_count: u32,
      searches_used_discussion: u32,
      /// URLs des articles trouvés (pour liens cliquables dans le feed)
      article_urls: Vec<String>,
  }
  ```
  Note : `article_urls` = `format!("https://{lang}.wikipedia.org/wiki/{}", title.replace(' ', "_"))` pour chaque page trouvée

### 9. `src-tauri/src/engine/orchestrator.rs`

**Struct** :
- Ajouter champ `wiki_client: WikiClient` dans `DiscussionEngine` (toujours créé, pas optionnel — gratuit)
- Ajouter champ `wiki_topic_cache: Option<WikiSearchResponse>` — cache de la première recherche forcée sur le topic (partagé entre gladiateurs)
- Initialiser `WikiClient::new()` et `wiki_topic_cache: None` dans `new()`

**Renommage** : Paramètre `web_search_results` → `search_results` dans `process_thought()`, `process_intervention()`, `process_intervention_think()` (reçoit désormais le contexte combiné web+wiki)

**Introduction** (après bloc `intro_web_search` ~ligne 222-245) :
- Ajouter bloc `intro_wiki_search` parallélisé :
  - Si les DEUX sont actifs (web_search_intro + wiki_search_intro) : `tokio::join!` pour exécuter les 2 HTTP en parallèle
  - Si un seul : exécution directe
  - Si wiki trouvé : mettre en cache dans `self.wiki_topic_cache`
- Combiner les 2 contextes :
  ```rust
  let combined = match (&intro_web_search, &intro_wiki_search) {
      (Some(w), Some(wiki)) => Some(format!("{w}\n\n{wiki}")),
      (Some(w), None) => Some(w.clone()),
      (None, Some(wiki)) => Some(wiki.clone()),
      (None, None) => None,
  };
  ```

**Per-turn** (après bloc `web_search_context` ~ligne 432-470) :
- Déterminer quelles sources sont actives pour ce gladiateur ce tour-ci
- **First search (forced, first turn pour chaque gladiateur)** :
  - Web : toujours appel HTTP (topic)
  - Wiki : vérifier `wiki_topic_cache` — si présent, réutiliser directement (évite N-1 appels redondants). Sinon appel HTTP puis mise en cache.
  - Si les 2 actifs : `tokio::join!` (web HTTP + wiki cache/HTTP)
- **Tours suivants (LLM décide)** :
  - Si les 2 sources actives : **1 prompt combiné** via `build_combined_search_decision_prompt()` → `CombinedSearchDecision` → HTTP parallèles via `tokio::join!`
  - Si une seule source active : prompt individuel (web existant ou nouveau wiki) → `SearchDecisionResponse`
- Combiner contextes avant passage à `process_thought()` / `process_intervention()` / `process_intervention_think()`

**Nouvelles méthodes** :
- `can_wiki_gladiateur(glad_idx) -> (bool, u32)` — vérifie `wiki_search_max_per_gladiateur > 0` et budget restant, max 1/tour (pas de quota global, plus simple que `can_search_gladiateur`)
- `process_wiki_search(...)` — miroir simplifié de `process_web_search()` :
  - Utilise `WikiClient` au lieu de `TavilyClient`
  - Pas d'`increment_tavily_usage`
  - Émet `WikiSearchPerformed` (avec `article_urls`)
  - Utilise `build_wiki_results_context()` pour le formatage
- `execute_search_decisions(web_can, wiki_can, glad_idx, channel)` — orchestration combinée :
  1. Détermine les requêtes (forcées/cache ou LLM)
  2. Si les 2 actives : 1 prompt combiné puis HTTP parallèles via `tokio::join!`
  3. Si 1 seule : prompt individuel puis HTTP
  4. Retourne `(Option<String>, Option<String>, u32, u32)` → (web_context, wiki_context, web_count, wiki_count)

**Tests** : Ajouter `check_wiki_quota()` dans le bloc `#[cfg(test)]` (miroir simplifié de `check_quota` sans `global_usage` ni `has_tavily`)

### 10. `src-tauri/src/engine/json_parser.rs`

- Ajouter struct `CombinedSearchDecision` avec `#[derive(Debug, Default, serde::Deserialize)]` + `#[serde(default)]` :
  ```rust
  pub(crate) struct CombinedSearchDecision {
      #[serde(default)]
      pub needs_web_search: bool,
      #[serde(default)]
      pub web_queries: Vec<String>,
      #[serde(default)]
      pub needs_wiki_search: bool,
      #[serde(default)]
      pub wiki_queries: Vec<String>,
  }
  ```
- Tests de désérialisation : complet, partiel, garbage, markdown-wrapped

### 11. `src-tauri/src/engine/prompt_builder.rs`

**Import** : Ajouter `use crate::wikipedia::WikiSearchResponse;`

**Modification header existant** (`build_search_results_context`) :
- Changer les headers pour être plus descriptifs :
  - FR : `[Résultats internet — actualité, données récentes, vérifications]`
  - EN : `[Internet results — current data, recent news, fact-checking]`
  - ZH : `[互联网结果 — 最新数据、近期新闻、事实核查]`
- Pas de changement de signature — `MAX_SEARCH_CONTEXT_LEN` (2000) reste inchangé pour chaque source (LLM locaux, pas de contrainte de coût token).

4 nouvelles fonctions :

- `default_wiki_directive(lang) -> &'static str` — directive trilingual orientée "définitions encyclopédiques, contexte historique, concepts scientifiques"
- `build_wiki_search_decision_prompt(topic, recent_context, directive, remaining, lang) -> String` — orienté Wikipedia : **demande un terme/concept encyclopédique, PAS une question**. 1 requête max. Utilisé quand SEUL wiki est actif.
  - FR : *"Fournis un terme ou concept encyclopédique (titre d'article Wikipédia), PAS une question."*
  - EN : *"Provide an encyclopedic term or concept (Wikipedia article title), NOT a question."*
  - ZH : *"提供一个百科术语或概念（维基百科文章标题），而不是问题。"*
- `build_combined_search_decision_prompt(topic, recent_context, web_directive, wiki_directive, web_remaining, wiki_remaining, lang) -> String` — prompt unique avec 2 sources :
  - Explique la nature de chaque source (internet=actualité, wiki=encyclopédique)
  - Demande le format JSON `CombinedSearchDecision`
  - Précise que les requêtes wiki doivent être des concepts, les requêtes internet des questions/recherches
- `build_wiki_results_context(results: &[(String, WikiSearchResponse)], lang) -> String` :
  - Header : `[Résultats Wikipédia — contexte encyclopédique, définitions, faits établis]` (trilingual)
  - Pages triées par `index`
  - Chaque résultat : `N. "Titre" : extrait...` (titre tronqué 100 chars, extrait 400 chars)
  - Troncation via `floor_char_boundary` à `MAX_SEARCH_CONTEXT_LEN` (2000 chars, même budget que internet)

---

## Fichiers à modifier — Frontend (7)

### 12. `src/lib/types.ts`
- Ajouter `wikiSearchIntro?: boolean` dans `IArbitreConfig` (après `webSearchIntro`)
- Ajouter `wikiSearchMaxPerGladiateur: number` dans `DiscussionConfig` (après `webSearchMaxPerGladiateur`)
- Ajouter variant `wikiSearchPerformed` dans le type union `ArenaEvent` :
  ```typescript
  | {
      type: "wikiSearchPerformed";
      data: {
        speakerId: string;
        speakerName: string;
        queries: string[];
        resultsCount: number;
        searchesUsedDiscussion: number;
        articleUrls: string[];
      };
    }
  ```

### 13. `src/stores/useSetupStore.ts`
- Ajouter state `wikiSearchMaxPerGladiateur: number` (default `0`)
- Ajouter setter `setWikiSearchMaxPerGladiateur` (avec clamp sur `maxTurns`)
- Auto-clamp dans `setMaxTurns` (même pattern que `webSearchMaxPerGladiateur`)
- Inclure dans `buildConfig()` et `reset()`

### 14. `src/stores/useArenaStore.ts`
- Ajouter dans le state + initialState :
  - `wikiSearchCount: number` (0)
  - `_pendingWikiCount: number` (0)
  - `wikiSearchesPerMessage: Record<string, number>` ({})
  - `wikiArticleUrlsPerMessage: Record<string, string[]>` ({}) — pour les liens cliquables
- Handler `wikiSearchPerformed` :
  - Incrémenter `wikiSearchCount` + `_pendingWikiCount`
  - Stocker `articleUrls` dans un buffer temporaire (même pattern que `_pendingSearchCount`)
- Dans `messageComplete` : stocker `_pendingWikiCount` dans `wikiSearchesPerMessage[msg.id]` + urls dans `wikiArticleUrlsPerMessage[msg.id]`, reset
- Dans `speakerActive` : reset `_pendingWikiCount` et buffer urls

### 15. `src/pages/SetupPage.tsx`

**StepArbitre** (après toggle `webSearchIntro` ~ligne 488-518) :
- Toggle `wikiSearchIntro` — toujours visible (pas de condition sur clé API), icône `BookOpen` (lucide-react), couleur verte

**StepGladiateurs** (après input `webSearchMaxPerGladiateur` ~ligne 706-737) :
- Input numérique `wikiSearchMaxPerGladiateur` — même layout, icône `BookOpen`, toujours visible
- Affichage budget total : `wikiSearchMaxPerGladiateur * gladiateurs.length`

**StepSummary** (après web search budget ~ligne 990) :
- Ligne résumé Wikipedia si `wikiSearchMaxPerGladiateur > 0`

### 16. `src/components/discussion/MessageBubble.tsx`
- Ajouter props `wikiSearchCount?: number` et `wikiArticleUrls?: string[]`
- Ajouter badge `BookOpen` vert à côté du badge `Globe` bleu existant, avec **liens cliquables** :
  ```tsx
  {(wikiSearchCount ?? 0) > 0 && (
    <span
      className="inline-flex items-center gap-1 rounded-full bg-green-500/10 px-1.5 py-0.5 text-[10px] font-medium text-green-600 cursor-pointer"
      title={wikiArticleUrls?.join("\n") ?? `${wikiSearchCount} Wikipedia`}
      onClick={() => wikiArticleUrls?.[0] && window.open(wikiArticleUrls[0], "_blank")}
    >
      <BookOpen className="h-3 w-3" />
    </span>
  )}
  ```
  - Tooltip : liste des URLs au survol
  - Clic : ouvre le premier article dans le navigateur (via Tauri `shell.open` ou `window.open`)

### 17. `src/components/discussion/DiscussionFeed.tsx`
- Extraire `wikiSearchesPerMessage` et `wikiArticleUrlsPerMessage` depuis `useArenaStore`
- Passer `wikiSearchCount` et `wikiArticleUrls` à `MessageBubble`

### 18. `src/i18n/locales/fr.json`, `en.json`, `zh.json`
Nouvelles clés i18n (section `setup`) :
| Clé | FR | EN | ZH |
|-----|----|----|-----|
| `setup.wikiSearch` | Wikipédia | Wikipedia | 维基百科 |
| `setup.arbitreWikiSearchIntro` | 1 recherche Wikipédia pour l'introduction | 1 Wikipedia search for introduction | 介绍使用1次维基百科搜索 |
| `setup.arbitreWikiSearchTitle` | Utiliser Wikipédia pour l'introduction | Use Wikipedia for introduction | 使用维基百科进行介绍 |
| `setup.wikiSearchMaxPerGladiateur` | Recherches Wikipédia par GladIAteur | Wikipedia searches per GladIAteur | 每位GladIAteur的维基百科搜索次数 |
| `setup.wikiSearchMaxPerGladiateurDesc` | Max 1 par tour, maximum {{max}} | Max 1 per turn, maximum {{max}} | 每轮最多1次，最多{{max}}次 |
| `setup.wikiSearchBudget` | Budget Wikipédia : {{count}} recherche(s) | Wikipedia budget: {{count}} search(es) | 维基百科预算：{{count}}次搜索 |

---

## Décisions clés

| # | Décision | Choix | Raison |
|---|----------|-------|--------|
| 1 | Module séparé `wikipedia/` | Oui | Miroir de `tavily/`, SRP, client HTTP différent (GET vs POST, pas d'auth) |
| 2 | `WikiClient` toujours créé (pas `Option`) | Oui | Gratuit, pas de condition sur clé API |
| 3 | 2 badges distincts (Globe bleu + BookOpen vert) | Oui | Distinguer visuellement les sources dans le feed |
| 4 | Prompt combiné quand les 2 sources actives | Oui | Économise 1 appel LLM/gladiateur/tour ≈ 2 min sur 40 tours |
| 5 | HTTP parallèles via `tokio::join!` | Oui | Économise ~2-3s/tour + ~2s à l'introduction |
| 6 | Requêtes wiki = concepts, pas questions | Oui | Wikipedia search fonctionne mieux avec des titres d'articles |
| 7 | Headers enrichis dans les contextes | Oui | Aide le LLM à pondérer actualité vs. encyclopédique |
| 8 | Pas de budget adaptatif (2000/source chacun) | Oui | LLM locaux, pas de contrainte de coût token — maximise l'info disponible |
| 9 | Fallback zh→en | Oui | Wikipedia chinois plus petit, fallback transparent |
| 10 | Cache première recherche topic | Oui | Évite N-1 appels HTTP identiques pour les gladiateurs |
| 11 | Liens Wikipedia cliquables | Oui | L'utilisateur peut vérifier les sources encyclopédiques |
| 12 | Renommer `web_search_results` → `search_results` | Oui | Paramètre reçoit désormais du contenu combiné |
| 13 | User-Agent dynamique `CARGO_PKG_VERSION` | Oui | Respecte la politique Wikimedia |
| 14 | Pas de page Settings | Correct | Pas de clé API, pas de quota |

---

## Vérification

1. **Compilation** : `cargo build` — zéro erreur
2. **Clippy** : `cargo clippy` — zéro warning
3. **Tests Rust** : `cargo test` — tous les 38 tests existants passent + nouveaux tests :
   - Désérialisation `WikiSearchResponse` (réponse réelle + champs manquants + réponse erreur maxlag)
   - Désérialisation `CombinedSearchDecision` (complet, partiel, garbage, markdown-wrapped)
   - `can_wiki_gladiateur` (budget 0 / budget > 0 / épuisé / max 1 par tour)
   - `build_wiki_results_context` (formatage, tri par index, troncation UTF-8 via `floor_char_boundary`)
   - `default_wiki_directive` (3 langues)
   - `WikiClient::wiki_lang` mapping (fr, en, zh, default)
4. **TypeScript** : `npx tsc --noEmit` — zéro erreur
5. **Test manuel** :
   - Setup : activer Wikipedia seul → badge BookOpen vert, recherche intro + par tour
   - Setup : activer Internet seul → comportement inchangé, badge Globe bleu
   - Setup : activer les deux → 2 badges, prompt combiné, HTTP parallèles
   - Setup : désactiver les deux → aucune recherche
   - Cliquer le badge BookOpen → ouvre l'article Wikipedia dans le navigateur
   - Vérifier fallback zh→en si le topic est de niche
   - Vérifier cache : avec 3+ gladiateurs, la première recherche wiki est faite 1 seule fois
   - Vérifier les 3 langues (fr/en/zh) dans les résultats
   - Vérifier StepSummary : budgets internet + Wikipedia affichés
