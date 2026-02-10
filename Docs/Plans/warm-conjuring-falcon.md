# Plan : Recherche Internet via Tavily API

## Contexte

Les IA d'AIrena sont limitées par leurs données d'entraînement. L'intégration de Tavily Search permet aux agents de rechercher des informations récentes/factuelles sur internet pendant les discussions, rendant les débats plus argumentés et à jour.

**API Tavily** : POST `https://api.tavily.com/search`, auth Bearer `tvly-*`, 1 credit/recherche basic, 1000 crédits gratuits/mois.

---

## 1. Rust : Module Tavily (nouveau)

### 1.1 `src-tauri/src/tavily/client.rs` (nouveau)

```rust
#[derive(Clone)]
pub struct TavilyClient {
    http_client: reqwest::Client, // réutilise reqwest (déjà en dep)
    api_key: String,
}

impl TavilyClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
            api_key: api_key.to_string(),
        }
    }
}
```

- `search(&self, query: &str, cancel: CancellationToken) -> Result<TavilySearchResponse, TavilyError>`
  - POST `https://api.tavily.com/search`
  - Headers: `Authorization: Bearer {api_key}`, `Content-Type: application/json`
  - Body: `{ query, search_depth: "basic", max_results: 5, include_answer: true }`
  - **IMPORTANT** : vérifier le status HTTP AVANT de parser le JSON body
    - 200 → parser `TavilySearchResponse`
    - 401 → `TavilyError::InvalidKey`
    - 429 → `TavilyError::RateLimit`
    - 432 → `TavilyError::QuotaExceeded`
    - autre → `TavilyError::Http(status, body_text)`
  - Timeout : 15s via reqwest client builder (PAS de `.timeout()` par requête)
  - Annulation via `tokio::select!` avec `cancel.cancelled()`

### 1.2 `src-tauri/src/tavily/mod.rs` (nouveau)

```rust
pub mod client;
pub mod error;

#[derive(Debug, serde::Deserialize)]
pub struct TavilySearchResponse {
    #[serde(default)]
    pub answer: Option<String>,
    #[serde(default)]
    pub results: Vec<TavilyResult>,
}

#[derive(Debug, serde::Deserialize)]
pub struct TavilyResult {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub score: f64,
}
```

### 1.3 `src-tauri/src/tavily/error.rs` (nouveau)

```rust
#[derive(Debug, thiserror::Error)]
pub enum TavilyError {
    #[error("Invalid API key")]
    InvalidKey,
    #[error("Rate limit exceeded")]
    RateLimit,
    #[error("Monthly quota exceeded")]
    QuotaExceeded,
    #[error("HTTP error {0}: {1}")]
    Http(u16, String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Cancelled")]
    Cancelled,
}
```

### 1.4 `src-tauri/src/lib.rs` — Ajouter `mod tavily`

---

## 2. Rust : Modèles & Config

### 2.1 `src-tauri/src/models/settings.rs` — Étendre AppSettings

Ajouter les champs à la struct :
```rust
pub tavily_api_key: String,           // clé API (vide = désactivé)
pub tavily_period_start: String,      // ISO date du début de période (YYYY-MM-DD)
pub tavily_usage_count: u32,          // compteur crédits période courante
pub tavily_usage_history: String,     // JSON: [{periodStart, periodEnd, usageCount}]
```

**Mettre à jour `impl Default for AppSettings`** — ajouter :
```rust
tavily_api_key: String::new(),
tavily_period_start: String::new(),
tavily_usage_count: 0,
tavily_usage_history: "[]".to_string(),
```

### 2.2 `src-tauri/src/models/discussion.rs` — WebSearchConfig

Nouvelle struct :
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSearchConfig {
    pub enabled: bool,
    pub max_per_discussion: u32,
    pub max_per_turn: u32,
    pub directive: String,
}
impl Default for WebSearchConfig {
    fn default() -> Self {
        Self { enabled: false, max_per_discussion: 5, max_per_turn: 2, directive: String::new() }
    }
}
```

### 2.3 `src-tauri/src/models/gladiateur.rs` — Ajouter web_search

```rust
// GladIAteurConfig — ajouter le champ :
#[serde(default)]
pub web_search: Option<WebSearchConfig>,

// GladIAteurState — ajouter le champ :
pub web_searches_used_discussion: u32,
```
**Modifier `GladIAteurState::new()`** — initialiser `web_searches_used_discussion: 0`
**Import** : `use super::discussion::WebSearchConfig;` dans gladiateur.rs

### 2.4 `src-tauri/src/models/iarbitre.rs` — Idem

```rust
// IArbitreConfig — ajouter le champ :
#[serde(default)]
pub web_search: Option<WebSearchConfig>,

// IArbitreState — ajouter le champ :
pub web_searches_used_discussion: u32,
```
**Modifier `IArbitreState::new()`** — initialiser `web_searches_used_discussion: 0`
**Import** : `use super::discussion::WebSearchConfig;` dans iarbitre.rs

### 2.5 `src-tauri/src/models/events.rs` — Nouvel événement

```rust
/// Recherche web effectuée — émis UNE SEULE FOIS par speaker par tour (batché)
#[serde(rename_all = "camelCase")]
WebSearchPerformed {
    speaker_id: String,
    speaker_name: String,
    queries: Vec<String>,       // toutes les queries exécutées
    results_count: u32,         // nombre total de résultats obtenus
    searches_used_discussion: u32, // compteur mis à jour pour ce speaker
}
```

**Supprimé** : `WebSearchBudget` — le frontend calcule le budget directement depuis `useSetupStore` (somme des maxPerDiscussion).

---

## 3. Rust : Settings DB & Period Management

### 3.1 `src-tauri/src/db/repository.rs` — Étendre get/save_settings

Ajouter les 4 nouvelles clés au match de `get_settings()` :
```rust
"tavily_api_key" => settings.tavily_api_key = value,
"tavily_period_start" => settings.tavily_period_start = value,
"tavily_usage_count" => settings.tavily_usage_count = value.parse().unwrap_or(0),
"tavily_usage_history" => settings.tavily_usage_history = value,
```

Ajouter aux pairs de `save_settings()` :
```rust
("tavily_api_key", settings.tavily_api_key.clone()),
("tavily_period_start", settings.tavily_period_start.clone()),
("tavily_usage_count", settings.tavily_usage_count.to_string()),
("tavily_usage_history", settings.tavily_usage_history.clone()),
```

**Auto-set period_start dans `save_settings()` (backend)** : si `tavily_api_key` est non-vide ET `tavily_period_start` est vide → `period_start = chrono::Local::now().format("%Y-%m-%d").to_string()`. Pas besoin de comparer avec l'ancienne valeur — si la clé est renseignée et la date absente, on la set.

### 3.2 Logique de période glissante — helper dédié

Nouvelle fonction `check_and_reset_tavily_period()` dans `repository.rs` :

```
1. Lire settings
2. Si api_key vide OU period_start vide → retour
3. Parser period_start en NaiveDate
4. Si now < period_start + 1 mois → retour (période en cours)
5. Archiver UNE SEULE entrée (la période écoulée avec son compteur courant)
   history.push({period_start, period_start + 1 mois, usage_count})
   NOTE: PAS d'entrées vides intermédiaires si l'app a été dormante
6. Avancer period_start de N mois pour tomber dans le mois courant
7. Remettre usage_count à 0
8. Sauvegarder en DB
```

### 3.3 Incrément atomique et lecture du compteur — dans l'engine

L'engine reçoit `db: Connection` (clone depuis `AppState.db` — `tokio_rusqlite::Connection` est Arc-interne, `.clone()` est cheap).

**Lecture du compteur courant** :
```rust
pub async fn get_tavily_usage(db: &Connection) -> Result<u32, tokio_rusqlite::Error> {
    db.call(|conn| {
        let count: u32 = conn.query_row(
            "SELECT COALESCE(CAST(value AS INTEGER), 0) FROM settings
             WHERE key = 'tavily_usage_count'",
            [], |row| row.get(0),
        ).unwrap_or(0);
        Ok(count)
    }).await
}
```

**Incrément atomique** via SQL unique (pas de read-modify-write = pas de race condition) :
```rust
pub async fn increment_tavily_usage(db: &Connection) -> Result<u32, tokio_rusqlite::Error> {
    db.call(|conn| {
        conn.execute(
            "UPDATE settings SET value = CAST(value AS INTEGER) + 1
             WHERE key = 'tavily_usage_count'",
            [],
        )?;
        let count: u32 = conn.query_row(
            "SELECT CAST(value AS INTEGER) FROM settings
             WHERE key = 'tavily_usage_count'",
            [], |row| row.get(0),
        )?;
        Ok(count)
    }).await
}
```

---

## 4. Rust : Prompt Builder

### 4.1 `src-tauri/src/engine/prompt_builder.rs` — Nouvelles fonctions

#### `build_web_search_decision_prompt()`
```
Signature : pub fn build_web_search_decision_prompt(
    topic: &str, recent_context: &str, search_directive: &str,
    searches_remaining: u32, discussion_language: &str,
) -> String

NOTE: Le system prompt du SPEAKER est passé séparément à ollama_client.build_request()
LLM params override : cloner les llm_params du speaker, puis :
  params.temperature = 0.3;
  params.num_predict = 100;

Prompt (trilingue FR/EN/ZH) :
  "Tu as accès à la recherche internet. [directive utilisateur]
   Sujet du débat : [topic]
   Contexte récent : [résumé court ~500 chars max]
   Recherches restantes : N

   As-tu besoin d'informations factuelles récentes ou spécialisées ?
   Si oui, fournis 1 à 3 requêtes de recherche courtes et pertinentes.
   Réponds UNIQUEMENT avec ce JSON :
   {\"needs_search\": true/false, \"queries\": [\"requête 1\"]}"
```

#### `default_search_directive()`
```rust
pub fn default_search_directive(lang: &str) -> &'static str {
    match lang {
        "en" => "Use internet search when you need recent information, precise factual data, or knowledge about an unfamiliar domain.",
        "zh" => "当你需要最新信息、精确的事实数据或不熟悉领域的知识时，使用互联网搜索。",
        _ => "Utilise la recherche internet lorsque tu as besoin d'informations récentes, de données factuelles précises, ou de connaissances sur un domaine que tu ne maîtrises pas.",
    }
}
```

#### Struct `SearchDecisionResponse` — définie dans `json_parser.rs` (pattern existant : VoteResponse, AuthoritarianOrderResponse)
```rust
#[derive(Default, serde::Deserialize)]
pub(crate) struct SearchDecisionResponse {
    #[serde(default)]
    pub needs_search: bool,
    #[serde(default)]
    pub queries: Vec<String>,
}
```
Parser avec `json_parser::parse_json_response::<SearchDecisionResponse>(&raw).unwrap_or_default()`
`#[derive(Default)]` fournit le fallback : `needs_search = false`, `queries = vec![]`

#### `build_search_results_context()`
```
Signature : pub fn build_search_results_context(
    results: &[(String, TavilySearchResponse)],
    discussion_language: &str,
) -> String

Import nécessaire : use crate::tavily::TavilySearchResponse;
(engine → tavily : pas de dépendance circulaire)

TRONCATION OBLIGATOIRE : max 300 chars par résultat, max 2000 chars total.
Utiliser truncate_str() existant (use super::truncate_str as truncate).

Format trilingue :
  [Résultats de recherche internet]
  Requête : "..."
  Résumé : [answer Tavily, tronqué 500 chars]
  Sources :
  1. "titre" (domain.com) : "extrait..." (tronqué 300 chars)
  2. ...
```

#### `build_datetime_context()` (BONUS)
```rust
pub fn build_datetime_context(discussion_language: &str) -> String {
    let now = chrono::Local::now();
    // %:z donne "+01:00" — PAS %Z qui donne "Romance Standard Time" sur Windows
    let datetime = now.format("%Y-%m-%d %H:%M:%S %:z").to_string();
    match discussion_language {
        "en" => format!("[Current date and time] {}", datetime),
        "zh" => format!("[当前日期和时间] {}", datetime),
        _ => format!("[Date et heure actuelles] {}", datetime),
    }
}
```

#### Modifier les signatures des fonctions existantes :

- `build_intervention_prompt(...)` → ajouter `web_search_results: Option<&str>`
  - Injecter après le contexte mémoire, avant les instructions finales
  - Injecter `build_datetime_context()` au début du user_msg

- `build_thought_prompt(...)` → ajouter `web_search_results: Option<&str>`
  - Injecter comme contexte additionnel

- `build_introduction_prompt(...)` → ajouter `web_search_results: Option<&str>`
  - Injecter `build_datetime_context()` au début

- `build_synthesis_prompt(...)` → ajouter `web_search_results: Option<&str>`
  - Injecter `build_datetime_context()` au début

---

## 5. Rust : Orchestrator

### 5.1 `src-tauri/src/engine/orchestrator.rs`

#### Nouveaux champs dans `DiscussionEngine` :
```rust
tavily_client: Option<TavilyClient>,  // None si pas de clé API
db: tokio_rusqlite::Connection,       // pour MAJ compteur (clone, Arc-interne)
```

#### Modifier `DiscussionEngine::new()` :
- Accepter `tavily_api_key: Option<&str>` et `db: Connection`
- Créer `TavilyClient::new(key)` si clé non vide
- Initialiser `web_searches_used_discussion: 0` dans chaque GladIAteurState et IArbitreState

#### Helper de quota partagé (DRY) + 2 wrappers :

```rust
/// Logique partagée — calcule le max de queries autorisées
fn compute_max_queries(
    web_search: Option<&WebSearchConfig>,
    searches_used_discussion: u32,
    global_usage: u32,
    has_tavily: bool,
) -> (bool, u32) {
    let config = match web_search {
        Some(ws) if ws.enabled && has_tavily => ws,
        _ => return (false, 0),
    };
    let remaining_disc = config.max_per_discussion
        .saturating_sub(searches_used_discussion);
    let max_queries = remaining_disc.min(config.max_per_turn);
    let global_remaining = 1000u32.saturating_sub(global_usage);
    let max_queries = max_queries.min(global_remaining);
    (max_queries > 0, max_queries)
}

fn can_search_gladiateur(&self, glad_idx: usize, global_usage: u32) -> (bool, u32) {
    compute_max_queries(
        self.gladiateurs[glad_idx].config.web_search.as_ref(),
        self.gladiateurs[glad_idx].web_searches_used_discussion,
        global_usage,
        self.tavily_client.is_some(),
    )
}

fn can_search_arbitre(&self, global_usage: u32) -> (bool, u32) {
    compute_max_queries(
        self.arbitre.config.web_search.as_ref(),
        self.arbitre.web_searches_used_discussion,
        global_usage,
        self.tavily_client.is_some(),
    )
}
```

#### Nouvelle méthode : `process_web_search()`

**NOTE** : `&self` (pas `&mut self`) — aucun champ de DiscussionEngine n'est muté.
L'incrément du compteur de discussion se fait au call site après retour.

```rust
async fn process_web_search(
    &self,                          // PAS &mut self — lecture seule
    system_prompt: &str,            // system prompt du speaker (in-character)
    speaker_id: &str,
    speaker_name: &str,
    max_queries: u32,
    search_directive: &str,
    recent_context: &str,           // construit par le caller (build_recent_exchanges ou custom)
    forced_queries: Option<Vec<String>>,  // Some → skip LLM decision (IArbitre direct search)
    llm_params: &LlmParams,        // speaker's params (clonés et overridés pour decision)
    searches_used_so_far: u32,      // compteur discussion courant (pour l'event)
    channel: &Channel<ArenaEvent>,
) -> (Option<String>, u32) {   // (search_context, queries_executed_count)

    // 1. DÉTERMINER LES QUERIES
    let queries: Vec<String> = if let Some(forced) = forced_queries {
        // IArbitre direct search — pas de décision LLM
        forced.into_iter().take(max_queries as usize).collect()
    } else {
        // Décision LLM (non-streaming, JSON)
        let mut decision_params = llm_params.clone();
        decision_params.temperature = 0.3;
        decision_params.num_predict = 100;

        let prompt = prompt_builder::build_web_search_decision_prompt(
            &self.config.topic, recent_context, search_directive,
            max_queries, &self.config.discussion_language,
        );
        let request = self.ollama_client.build_request(
            system_prompt, &prompt, &decision_params, true, // json_mode
        );
        let raw = match self.ollama_client.chat(&request, self.cancel_token.clone()).await {
            Ok(r) => r,
            Err(_) => return (None, 0),  // LLM error → no search
        };
        let decision = json_parser::parse_json_response::<SearchDecisionResponse>(&raw)
            .unwrap_or_default();

        if !decision.needs_search || decision.queries.is_empty() {
            return (None, 0);
        }
        decision.queries.into_iter().take(max_queries as usize).collect()
    };

    if queries.is_empty() { return (None, 0); }

    // 2. EXÉCUTER CHAQUE RECHERCHE
    let tavily = match self.tavily_client.as_ref() {
        Some(c) => c,
        None => return (None, 0),   // pas de ? — return type est tuple, pas Result
    };
    let mut all_results: Vec<(String, TavilySearchResponse)> = Vec::new();
    let mut executed_count = 0u32;

    for query in &queries {
        if self.cancel_token.is_cancelled() { break; }

        match tavily.search(query, self.cancel_token.clone()).await {
            Ok(response) => {
                all_results.push((query.clone(), response));
                executed_count += 1;
                let _ = repository::increment_tavily_usage(&self.db).await;
            }
            Err(TavilyError::QuotaExceeded) => {
                tracing::warn!("Tavily quota exceeded — stopping all searches");
                break;
            }
            Err(TavilyError::InvalidKey) => {
                tracing::error!("Tavily API key invalid — stopping all searches");
                break;  // toutes les suivantes échoueront aussi
            }
            Err(TavilyError::Cancelled) => break,
            Err(e) => {
                tracing::warn!(query = %query, error = %e, "Tavily search failed — skipping");
                continue;
            }
        }
    }

    if executed_count == 0 { return (None, 0); }

    // 3. ÉMETTRE EVENT (batché, queries = seulement celles réussies)
    let executed_queries: Vec<String> = all_results.iter().map(|(q, _)| q.clone()).collect();
    let total_results: u32 = all_results.iter().map(|(_, r)| r.results.len() as u32).sum();
    let _ = channel.send(ArenaEvent::WebSearchPerformed {
        speaker_id: speaker_id.to_string(),
        speaker_name: speaker_name.to_string(),
        queries: executed_queries,
        results_count: total_results,     // informatif pour futur affichage détaillé
        searches_used_discussion: searches_used_so_far + executed_count,
    });

    // 4. FORMATER pour injection prompt (tronqué max 2000 chars)
    let lang = &self.config.discussion_language;
    (Some(prompt_builder::build_search_results_context(&all_results, lang)), executed_count)
}
```

#### Flow modifié par gladiateur :

```
Ordre :
1. Réactions (existant)
2. *** RECHERCHE WEB *** (nouveau — si activé + quotas OK)
3. Pensée/Think (existant — enrichi avec résultats web)
4. Intervention (existant — enrichi avec résultats web)
5. Émotions (existant)
6. Modération (existant)
```

```rust
// C.2.5 WEB SEARCH
let web_search_context: Option<String> = {
    let global_usage = repository::get_tavily_usage(&self.db).await.unwrap_or(0);
    let (can, max_q) = self.can_search_gladiateur(glad_idx, global_usage);
    if can {
        let ws = self.gladiateurs[glad_idx].config.web_search.as_ref().unwrap();
        let directive = if ws.directive.is_empty() {
            prompt_builder::default_search_directive(&self.config.discussion_language).to_string()
        } else { ws.directive.clone() };
        let recent = truncate(
            &self.build_recent_exchanges(glad_idx), 500
        ).to_string();
        let used_so_far = self.gladiateurs[glad_idx].web_searches_used_discussion;
        let (ctx, count) = self.process_web_search(
            &self.gladiateurs[glad_idx].config.system_prompt,
            &speaker_id, &speaker_name, max_q, &directive,
            &recent,                            // recent_context
            None,                               // forced_queries: None → LLM décide
            &self.gladiateurs[glad_idx].config.llm_params,
            used_so_far,                        // searches_used_so_far
            &channel,
        ).await;
        self.gladiateurs[glad_idx].web_searches_used_discussion += count;
        ctx
    } else { None }
};

// C.3 + C.4 — passer web_search_context aux 3 chemins
let (thought, content) = if use_think {
    let (t, c) = self.process_intervention_think(
        glad_idx, web_search_context.as_deref(), &channel
    ).await;
    // ... fallback si think mode échoue ...
} else {
    let thought = self.process_thought(
        glad_idx, web_search_context.as_deref(), &channel
    ).await;
    let content = self.process_intervention(
        glad_idx, thought.as_deref(), web_search_context.as_deref(), &channel
    ).await;
    (thought, content)
};
```

#### IArbitre : recherche pendant introduction et synthèse

**Introduction** : Pas de décision LLM — recherche directe sur le topic (la query EST le sujet).
```rust
// Avant l'introduction, si web search activé pour l'arbitre :
let intro_web_search: Option<String> = {
    let global_usage = repository::get_tavily_usage(&self.db).await.unwrap_or(0);
    let (can, max_q) = self.can_search_arbitre(global_usage);
    if can {
        // Topic tronqué à 200 chars (Tavily limite les queries longues)
        let topic_query = truncate(&self.config.topic, 200).to_string();
        let used_so_far = self.arbitre.web_searches_used_discussion;
        let (ctx, count) = self.process_web_search(
            &self.arbitre.config.system_prompt,
            &self.arbitre.config.id, &self.arbitre.config.name,
            max_q.min(1),   // 1 seule recherche pour l'intro
            "",             // pas de directive
            "",             // pas de recent_context
            Some(vec![topic_query]),  // forced_queries → skip LLM decision
            &self.arbitre.config.llm_params,
            used_so_far,    // searches_used_so_far
            &channel,
        ).await;
        self.arbitre.web_searches_used_discussion += count;
        ctx
    } else { None }
};
// Passer intro_web_search.as_deref() à build_introduction_prompt()
```

**Synthèse** : Idem — recherche sur le topic si quota restant, `forced_queries = Some(vec![topic])`.

Vérifier `can_search_arbitre()` avant chaque recherche.

#### Modifier signatures des méthodes existantes :
- `process_thought(&self, glad_idx: usize, web_search: Option<&str>, channel: &Channel<ArenaEvent>) -> Option<String>`
- `process_intervention(&self, glad_idx: usize, thought: Option<&str>, web_search: Option<&str>, channel: &Channel<ArenaEvent>) -> Option<String>`
- `process_intervention_think(&self, glad_idx: usize, web_search: Option<&str>, channel: &Channel<ArenaEvent>) -> (Option<String>, Option<String>)`
- `generate_synthesis(&self, web_search: Option<&str>, channel: &Channel<ArenaEvent>)`

**NOTE** : `process_web_search` est `&self`, les autres restent `&self` aussi. L'introduction IArbitre utilise `&mut self` uniquement APRÈS le retour de `process_web_search` pour `+= count`.

---

## 6. Rust : Commande discussion.rs

### `src-tauri/src/commands/discussion.rs` — Modifier `start_discussion`

```rust
let settings = state.get_settings().await?;

// Vérifier/reset la période Tavily avant le démarrage
if !settings.tavily_api_key.is_empty() {
    let _ = repository::check_and_reset_tavily_period(&state.db).await;
}

let tavily_key = if settings.tavily_api_key.is_empty() { None }
    else { Some(settings.tavily_api_key.clone()) };
let db_clone = state.db.clone();  // tokio_rusqlite::Connection est Arc-interne

tauri::async_runtime::spawn(async move {
    let mut engine = DiscussionEngine::new(
        config, id_clone, &ollama_url, &ollama_model,
        tavily_key.as_deref(), db_clone,
    );
    // ...
});
```

---

## 7. Frontend : Types

### `src/lib/types.ts`

```typescript
export interface WebSearchConfig {
  enabled: boolean;
  maxPerDiscussion: number;
  maxPerTurn: number;
  directive: string;
}

export const DEFAULT_WEB_SEARCH_CONFIG: WebSearchConfig = {
  enabled: false, maxPerDiscussion: 5, maxPerTurn: 2, directive: "",
};

// Modifier GladIAteurConfig — ajouter :
webSearch?: WebSearchConfig;

// Modifier IArbitreConfig — ajouter :
webSearch?: WebSearchConfig;

// Modifier AppSettings — ajouter :
tavilyApiKey: string;
tavilyPeriodStart: string;
tavilyUsageCount: number;
tavilyUsageHistory: string;

// Nouveau type
export interface TavilyPeriodHistory {
  periodStart: string;
  periodEnd: string;
  usageCount: number;
}

// Nouveau ArenaEvent dans l'union type
| { type: "webSearchPerformed"; data: {
    speakerId: string; speakerName: string;
    queries: string[]; resultsCount: number;
    searchesUsedDiscussion: number;
  }}

// Directive par défaut (constante)
export const DEFAULT_WEB_SEARCH_DIRECTIVE: Record<string, string> = {
  fr: "Utilise la recherche internet lorsque tu as besoin d'informations récentes, de données factuelles précises, ou de connaissances sur un domaine que tu ne maîtrises pas.",
  en: "Use internet search when you need recent information, precise factual data, or knowledge about an unfamiliar domain.",
  zh: "当你需要最新信息、精确的事实数据或不熟悉领域的知识时，使用互联网搜索。",
};
```

---

## 8. Frontend : Settings Page

### `src/pages/SettingsPage.tsx` — Nouvelle section "Recherche Internet"

Position : après Ollama, avant Profils.
- **Clé API** : input password + toggle reveal + lien `https://www.tavily.com` + note "1000 crédits gratuits/mois"
- **Section conditionnelle** (si clé non vide) :
  - Période : input date (auto-set backend, modifiable)
  - Compteur : input number éditable + "X / 1000" + barre progression
  - Historique : liste scrollable max-h-48, "Du X au Y — Z crédits" par entrée

---

## 9. Frontend : Setup Page

### `src/pages/SetupPage.tsx`

- **StepArbitre** : section collapsible web search (toggle + maxDisc + maxTurn + directive textarea)
  - Caché si pas de clé Tavily → message "Configurez votre clé Tavily"
- **StepGladiateurs** : même section dans chaque carte, collapsible
  - Contrainte : maxPerTurn clamp auto si > maxPerDiscussion
- **StepSummary** :
  - Raccourci global "Appliquer à tous" (toggle + inputs + bouton Appliquer = action one-shot)
  - Indicateur budget = `Σ maxPerDiscussion` de tous agents activés (calculé dynamiquement)
  - Avertissement si budget > 0 + bouton "Ajuster" → retour step 1

### `src/stores/useSetupStore.ts`
- `applyGlobalWebSearch(config: WebSearchConfig)` : applique à arbitre + tous gladiateurs
- `buildConfig()` inclut les webSearch configs via le spread existant

---

## 10. Frontend : Arena

### `src/stores/useArenaStore.ts`

Ajouter à `initialState` :
```typescript
webSearchCount: 0,
_pendingSearchCount: 0,
webSearchesPerMessage: {} as Record<string, number>,
```

Handlers :
- `speakerActive` → reset `_pendingSearchCount: 0`
- `webSearchPerformed` → `webSearchCount += queries.length`, `_pendingSearchCount += queries.length`
- `messageComplete` → si `_pendingSearchCount > 0`, stocker dans `webSearchesPerMessage[msg.id]`, reset
- `reset()` → inclure les 3 champs

### UI Components :
- **TurnIndicator** : icône Globe + "X recherche(s)" si webSearchCount > 0
- **MessageBubble** : prop `searchCount?`, badge `🔍 N` si > 0
- **DiscussionFeed** : lire `webSearchesPerMessage` du store, passer à MessageBubble

---

## 11. i18n (3 langues, structure imbriquée JSON)

Dans `settings` : tavily, tavilyApiKey, tavilyLink, tavilyFreeCredits, tavilyPeriodStart, tavilyUsageCount, tavilyHistory, tavilyNoHistory, tavilyPeriodEntry
Dans `setup` : webSearch, webSearchEnabled, webSearchDisabled, webSearchMaxDiscussion, webSearchMaxTurn, webSearchDirective, webSearchGlobal, webSearchApply, webSearchBudget, webSearchWarning, webSearchAdjust
Dans `arena` : webSearchCount, webSearchBadge

---

## 12. BONUS : Date/heure avec fuseau horaire

`build_datetime_context()` injecté dans build_introduction_prompt, build_intervention_prompt, build_synthesis_prompt, build_thought_prompt.
Format : `[Date et heure actuelles] 2026-02-08 14:30:00 +01:00` (pas `%Z` qui donne "Romance Standard Time" sur Windows).

---

## 13. Fichiers modifiés

### Rust (14 fichiers)
| Fichier | Action |
|---------|--------|
| `src-tauri/src/tavily/mod.rs` | **NOUVEAU** — types response |
| `src-tauri/src/tavily/client.rs` | **NOUVEAU** — TavilyClient |
| `src-tauri/src/tavily/error.rs` | **NOUVEAU** — TavilyError |
| `src-tauri/src/lib.rs` | `mod tavily` |
| `src-tauri/src/models/settings.rs` | +4 champs AppSettings |
| `src-tauri/src/models/discussion.rs` | +WebSearchConfig |
| `src-tauri/src/models/gladiateur.rs` | +web_search + counter |
| `src-tauri/src/models/iarbitre.rs` | +web_search + counter |
| `src-tauri/src/models/events.rs` | +WebSearchPerformed |
| `src-tauri/src/db/repository.rs` | extend settings, increment_tavily_usage, check_period |
| `src-tauri/src/engine/prompt_builder.rs` | 3 nouvelles fn, modifier 4 existantes |
| `src-tauri/src/engine/orchestrator.rs` | process_web_search, can_search_*, flow modifié |
| `src-tauri/src/engine/json_parser.rs` | +SearchDecisionResponse (suit le pattern VoteResponse) |
| `src-tauri/src/commands/discussion.rs` | passer clé API + DB |

### Frontend (12 fichiers)
| Fichier | Action |
|---------|--------|
| `src/lib/types.ts` | WebSearchConfig, AppSettings, event, constantes |
| `src/stores/useSetupStore.ts` | applyGlobalWebSearch |
| `src/stores/useArenaStore.ts` | compteurs, handlers, tracking |
| `src/pages/SettingsPage.tsx` | section Tavily |
| `src/pages/SetupPage.tsx` | config web search |
| `src/components/discussion/TurnIndicator.tsx` | compteur |
| `src/components/discussion/MessageBubble.tsx` | badge |
| `src/components/discussion/DiscussionFeed.tsx` | passer searchCount |
| `src/i18n/locales/fr.json` | clés i18n |
| `src/i18n/locales/en.json` | clés i18n |
| `src/i18n/locales/zh.json` | clés i18n |

---

## 14. Ordre d'implémentation

1. Tavily module Rust (client + error + mod)
2. Modèles Rust (settings, WebSearchConfig, events)
3. DB repository (settings étendu, increment atomique, période)
4. Prompt builder (nouvelles fn + datetime + modifier signatures)
5. Orchestrator (process_web_search, can_search_*, flow modifié, signatures)
6. Commande discussion (clé API + DB + check période)
7. Types frontend (mirror Rust)
8. Settings page (section Tavily)
9. Setup page (config web search)
10. Arena store + UI (handlers, compteurs, badges)
11. i18n (3 langues)
12. Tests & validation

---

## 15. Vérification

1. **Cargo clippy** : zéro warning
2. **Cargo test** : tous tests passent (y compris nouveaux tests ci-dessous)
3. **tsc --noEmit** : pas d'erreur TS
4. **Tests unitaires à ajouter** :
   - `json_parser.rs` : test parsing `SearchDecisionResponse` (valide, incomplet, vide, garbage)
   - `prompt_builder.rs` : test `build_datetime_context()` (vérifie format `+XX:XX`, PAS `%Z`)
   - `prompt_builder.rs` : test `build_search_results_context()` (vérifie troncation à 2000 chars)
   - `prompt_builder.rs` : test `default_search_directive()` (3 langues)
   - `orchestrator.rs` : test `compute_max_queries()` (quotas tour, discussion, global, désactivé)
   - `repository.rs` : test `increment_tavily_usage()` + `get_tavily_usage()` (atomicité)
5. **Tests manuels** :
   - Settings : saisir clé → section active, date auto-set
   - Settings : modifier compteur manuellement → persistance
   - Settings : historique scrollable
   - Setup : web search 1 agent → budget affiché
   - Setup : "Appliquer à tous" → tous mis à jour
   - Setup : override individuel après apply global
   - Setup : maxPerTurn > maxPerDiscussion → clamp
   - Discussion : IA décide quand chercher (logs tracing)
   - Discussion : quotas tour + discussion respectés
   - Discussion : épuiser quota agent → arrête de chercher
   - Discussion : Tavily 401/432 → dégradation gracieuse, pas de crash
   - Discussion : compteur TurnIndicator mis à jour
   - Discussion : badge 🔍 sur messages avec recherches
   - Discussion : force-stop pendant recherche → pas de crash
   - Discussion : date/heure dans prompts (logs)
   - Discussion sans clé Tavily : pas de régression
   - Settings après discussion : compteur global incrémenté
