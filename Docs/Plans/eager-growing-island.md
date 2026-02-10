# Plan: Modes de distribution des tours — Démocratique & Autoritaire

## Contexte
Actuellement, AIrena propose 2 modes de distribution des tours : **Séquentiel** (ordre fixe) et **Aléatoire** (shuffle). On ajoute 2 nouveaux modes qui utilisent le LLM pour déterminer l'ordre de parole :
- **Démocratique** : vote masqué des IA participantes (Borda count) → IArbitre tranche les égalités
- **Autoritaire** : IArbitre décide seul l'ordre de parole

**Contrainte clé** : Chaque IA active (non-bannie) DOIT parler à chaque tour — les votes/décisions ne déterminent que l'ORDRE.

---

## Fichiers à modifier

| Fichier | Changement |
|---------|------------|
| `src-tauri/src/models/discussion.rs` | +2 variants enum `TurnDistribution` |
| `src-tauri/src/models/events.rs` | +1 event `DeterminingOrder` (avec `#[serde(rename_all = "camelCase")]`) |
| `src-tauri/src/ollama/client.rs` | Ajouter `#[derive(Clone)]` à `OllamaClient` |
| `src-tauri/src/engine/prompt_builder.rs` | +3 fonctions prompt (vote, ordre autoritaire, tiebreak) |
| `src-tauri/src/engine/json_parser.rs` | +2 structs, +2 fonctions parse, +1 helper `match_speaker_name` |
| `src-tauri/src/engine/turn_manager.rs` | +2 fonctions async, +1 struct `AsyncTurnContext`, modifier match existant |
| `src-tauri/src/engine/orchestrator.rs` | Dispatch async + check annulation post-vote |
| `src/lib/types.ts` | Étendre type union + event |
| `src/pages/SetupPage.tsx` | Grille 2×2 de boutons avec descriptions |
| `src/i18n/locales/{en,fr,zh}.json` | Nouvelles clés (modes + descriptions + event) |
| `src/stores/useArenaStore.ts` | Handler event `determiningOrder` (minimal) |

---

## Étapes d'implémentation

### 1. Rust — Enum `TurnDistribution` (`src-tauri/src/models/discussion.rs:6-11`)

```rust
pub enum TurnDistribution {
    Sequential,
    Random,
    Democratic,
    Authoritarian,
}
```

### 2. Rust — Match exhaustif `determine_speaker_order` (`src-tauri/src/engine/turn_manager.rs:18-30`)

**⚠️ CRITIQUE** : Le match existant ne gère que Sequential/Random → erreur de compilation.
Ajouter un bras wildcard pour les nouveaux variants :

```rust
match distribution {
    TurnDistribution::Sequential => { /* inchangé */ }
    TurnDistribution::Random => { /* inchangé */ }
    // Democratic/Authoritarian use async functions; sync fallback = sequential
    _ => {
        let mut sorted = active_indices;
        sorted.sort_by_key(|&i| gladiateurs[i].config.intervention_number);
        sorted
    }
}
```

### 3a. Rust — `Clone` pour `OllamaClient` (`src-tauri/src/ollama/client.rs:16`)

**⚠️ PRÉREQUIS** : `OllamaClient` ne dérive pas `Clone`. Nécessaire pour `AsyncTurnContext` owned.
Tous les champs sont Clone (`reqwest::Client` = Arc-based, `String`).

```rust
#[derive(Clone)]
pub struct OllamaClient { ... }
```

### 3b. Rust — Event `DeterminingOrder` (`src-tauri/src/models/events.rs`)

**⚠️ IMPORTANT** : Le fichier utilise maintenant `#[serde(rename_all = "camelCase")]` sur CHAQUE variant individuellement (pas juste sur l'enum). Le nouveau variant doit suivre ce pattern :

```rust
/// Turn order is being determined (democratic/authoritarian modes)
#[serde(rename_all = "camelCase")]
DeterminingOrder { turn_number: u32 },
```

Placer après `TurnSkipped` (L37-41), avant `SpeakerActive` (L42-44).

### 4. Rust — Helper `match_speaker_name` (`src-tauri/src/engine/json_parser.rs`)

Extraire un helper réutilisable basé sur le pattern de `validate_reactions` (L77-94) :

```rust
/// Match a LLM-returned name against a list of known names.
/// 3 layers: exact case-insensitive → prefix match (min 3 chars) → contains match (min 5 chars)
pub fn match_speaker_name<'a>(llm_name: &str, known_names: &'a [String]) -> Option<&'a String> {
    let llm_lower = llm_name.to_lowercase().trim().to_string();
    // 1. Exact (case-insensitive, trimmed)
    known_names.iter().find(|s| s.to_lowercase().trim() == llm_lower)
        // 2. Prefix (known starts with LLM name, min 3 chars)
        .or_else(|| {
            if llm_lower.len() >= 3 {
                known_names.iter().find(|s| s.to_lowercase().starts_with(&llm_lower))
            } else { None }
        })
        // 3. Contains (either direction, min 5 chars) — handles "Avocat du Diable" vs "L'Avocat du Diable"
        .or_else(|| {
            if llm_lower.len() >= 5 {
                known_names.iter().find(|s| {
                    let s_lower = s.to_lowercase();
                    s_lower.contains(&llm_lower) || llm_lower.contains(&s_lower)
                })
            } else { None }
        })
}
```

Refactorer `validate_reactions` pour utiliser ce helper aussi (DRY).

### 5. Rust — Parse vote/order (`src-tauri/src/engine/json_parser.rs`)

```rust
#[derive(Debug, Deserialize)]
struct VoteResponse { #[serde(default)] ranking: Vec<String> }

#[derive(Debug, Deserialize)]
struct AuthoritarianOrderResponse { #[serde(default)] order: Vec<String> }

pub fn parse_vote(raw: &str) -> Vec<String> {
    parse_json_response::<VoteResponse>(raw).map(|r| r.ranking).ok()
        .or_else(|| parse_json_response::<Vec<String>>(raw).ok())
        .unwrap_or_default()
}

pub fn parse_authoritarian_order(raw: &str) -> Vec<String> {
    parse_json_response::<AuthoritarianOrderResponse>(raw).map(|r| r.order).ok()
        .or_else(|| parse_json_response::<Vec<String>>(raw).ok())
        .unwrap_or_default()
}
```

### 6. Rust — Prompts (`src-tauri/src/engine/prompt_builder.rs`)

3 fonctions trilinguales (FR/EN/ZH) :

**a) `build_democratic_vote_prompt(voter_name, other_active_names, topic, discussion_summary, discussion_language)`**
- Demande au gladiateur de classer les AUTRES participants actifs
- Inclut le topic explicitement (pas juste le summary, crucial pour tour 1 quand summary est vide)
- Format JSON : `{"ranking": ["premier", "deuxième", ...]}`
- Instruction : inclure TOUS les participants listés

**b) `build_authoritarian_order_prompt(active_names, topic, discussion_summary, current_turn, discussion_language)`**
- Demande à IArbitre de décider l'ordre complet
- Format JSON : `{"order": ["premier", "deuxième", ...]}`

**c) `build_tiebreak_prompt(tied_names, topic, discussion_summary, current_turn, discussion_language)`**
- Demande à IArbitre de départager les ex-aequo
- Inclut le contexte de discussion + topic + numéro de tour
- Format JSON : `{"order": ["premier", "deuxième", ...]}`

### 7. Rust — Turn Manager async (`src-tauri/src/engine/turn_manager.rs`)

**Struct de contexte (champs OWNED, pas de borrows problématiques) :**
```rust
pub struct AsyncTurnContext {
    pub ollama_client: OllamaClient,          // Clone (uses Arc<reqwest::Client> internally)
    pub cancel_token: CancellationToken,      // Clone + 'static
    pub arbitre_system_prompt: String,         // Cloned
    pub arbitre_llm_params: LlmParams,        // Cloned
    pub discussion_summary: String,            // Cloned
    pub topic: String,                         // Cloned
    pub current_turn: u32,
    pub discussion_language: String,           // Cloned
}
```

**⚠️ CRITIQUE** : Pas de lifetime `'a` — tous les champs sont owned. Élimine tout problème de borrow checker avec `&mut self` dans l'orchestrateur.

**a) `determine_order_democratic(gladiateurs: &[GladIAteurState], ctx: &AsyncTurnContext) -> Vec<usize>`**

```
1. Collecter active_indices + active_names (filtrer is_banned)
2. EARLY RETURN si ≤ 1 actif → retourner directement
3. ⚠️ CAS N=2 : tie garanti → skip votes, appeler directement determine_order_authoritarian
   (évite 3 LLM calls inutiles : 2 votes triviales + 1 tiebreak)
4. Pour chaque actif : prompt de vote excluant son propre nom
   - Utilise le system_prompt du GLADIATEUR (pas celui de l'arbitre)
   - json_format = true
5. futures_util::future::join_all → votes en parallèle
6. Scoring Borda : 1er = N-1 points, 2e = N-2, etc.
   - Résolution des noms via match_speaker_name() contre active_names uniquement
7. ⚠️ DÉTECTION ZÉRO-INFO : si total_recognized_votes == 0 → fallback séquentiel
8. Tri par score décroissant → groupes par score
9. Groupes >1 membre → appel tiebreak à IArbitre
10. Filet de sécurité : tout actif non résolu appendu par intervention_number
11. Fallback total si vide → determine_speaker_order(Sequential)
```

**b) `determine_order_authoritarian(gladiateurs: &[GladIAteurState], ctx: &AsyncTurnContext) -> Vec<usize>`**

```
1. Collecter active_indices + active_names (filtrer is_banned)
2. EARLY RETURN si ≤ 1 actif
3. Un seul appel LLM non-streaming à IArbitre (json_format = true)
4. Parse → résolution noms via match_speaker_name() contre active_names
5. Participants manquants appendus par intervention_number (déterministe)
6. Fallback erreur LLM → determine_speaker_order(Sequential)
```

### 8. Rust — Orchestrateur (`src-tauri/src/engine/orchestrator.rs:207-211`)

**⚠️ CRITIQUE** : Cloner les champs AVANT le `.await` pour éviter les problèmes de borrow checker :

```rust
// Determine speaker order
let order = match &self.config.arbitre.turn_distribution {
    TurnDistribution::Sequential | TurnDistribution::Random => {
        turn_manager::determine_speaker_order(
            &self.gladiateurs,
            &self.config.arbitre.turn_distribution,
        )
    }
    TurnDistribution::Democratic | TurnDistribution::Authoritarian => {
        let _ = channel.send(ArenaEvent::DeterminingOrder {
            turn_number: self.current_turn,
        });

        // Clone fields into owned context to avoid borrow issues across .await
        let ctx = turn_manager::AsyncTurnContext {
            ollama_client: self.ollama_client.clone(),
            cancel_token: self.cancel_token.clone(),
            arbitre_system_prompt: self.arbitre.config.system_prompt.clone(),
            arbitre_llm_params: self.arbitre.config.llm_params.clone(),
            discussion_summary: self.arbitre.memory.contextual_summary.clone(),
            topic: self.config.topic.clone(),
            current_turn: self.current_turn,
            discussion_language: self.config.discussion_language.clone(),
        };

        match &self.config.arbitre.turn_distribution {
            TurnDistribution::Democratic => {
                turn_manager::determine_order_democratic(&self.gladiateurs, &ctx).await
            }
            TurnDistribution::Authoritarian => {
                turn_manager::determine_order_authoritarian(&self.gladiateurs, &ctx).await
            }
            _ => unreachable!(),
        }
    }
};

// ⚠️ CRITIQUE : Check annulation APRÈS le vote, AVANT TurnStarted
// Empêche l'émission d'un TurnStarted fantôme si force-stop pendant le vote
if self.cancel_token.is_cancelled() { break; }
if self.process_commands(&mut cmd_rx, &channel).await { break; }
```

### 9. TypeScript — Types (`src/lib/types.ts`)

**L34** : `turnDistribution: "sequential" | "random" | "democratic" | "authoritarian";`

**ArenaEvent** : `| { type: "determiningOrder"; data: { turnNumber: number } }`

### 10. TypeScript — i18n (3 fichiers)

**fr.json** dans `"setup"` :
```json
"democratic": "Démocratique",
"democraticDesc": "Les participants votent pour décider de l'ordre de parole. En cas d'égalité, l'IArbitre tranche.",
"authoritarian": "Autoritaire",
"authoritarianDesc": "L'IArbitre décide seul de l'ordre de parole selon ses propres critères.",
"sequentialDesc": "Ordre fixe basé sur le numéro d'intervention",
"randomDesc": "Ordre aléatoire à chaque tour"
```

**en.json** dans `"setup"` :
```json
"democratic": "Democratic",
"democraticDesc": "Participants vote to decide speaking order. In case of a tie, the moderator decides.",
"authoritarian": "Authoritarian",
"authoritarianDesc": "The moderator alone decides the speaking order based on their own criteria.",
"sequentialDesc": "Fixed order based on position number",
"randomDesc": "Random order each turn"
```

**zh.json** dans `"setup"` :
```json
"democratic": "民主",
"democraticDesc": "参与者投票决定发言顺序。平局时由主持人裁决。",
"authoritarian": "权威",
"authoritarianDesc": "主持人根据自己的标准独自决定发言顺序。",
"sequentialDesc": "按编号固定顺序",
"randomDesc": "每轮随机顺序"
```

Dans `"arena"` des 3 fichiers : `"determiningOrder": "Détermination de l'ordre de parole..."` / `"Determining speaking order..."` / `"正在确定发言顺序..."`

### 11. TypeScript — SetupPage (`src/pages/SetupPage.tsx:393-413`)

Grille 2×2 avec titre + description pour chaque mode :

```tsx
<div className="grid grid-cols-2 gap-2">
  {(["sequential", "random", "democratic", "authoritarian"] as const).map((dist) => (
    <button key={dist} onClick={() => updateArbitre({ turnDistribution: dist })}
      className={cn(
        "rounded-md border px-3 py-2 text-left transition-colors",
        arbitre.turnDistribution === dist
          ? "border-primary bg-primary/10 text-primary"
          : "border-border text-muted-foreground hover:bg-accent",
      )}>
      <div className="text-sm font-medium">{t(`setup.${dist}`)}</div>
      <div className="mt-0.5 text-xs opacity-70">{t(`setup.${dist}Desc`)}</div>
    </button>
  ))}
</div>
```

La ligne résumé L590-593 `t(`setup.${arbitre.turnDistribution}`)` fonctionne automatiquement.

### 12. TypeScript — Arena Store (`src/stores/useArenaStore.ts`)

```typescript
case "determiningOrder":
  break; // État transitoire, TurnStarted suit immédiatement après
```

---

## Points critiques validés par la revue (3 itérations)

| # | Problème identifié | Solution intégrée |
|---|---|---|
| P0-1 | Match non-exhaustif compile error | Bras wildcard → séquentiel (étape 2) |
| P0-2 | Borrow checker `&mut self` + `.await` | AsyncTurnContext OWNED (pas de lifetimes) (étape 7) + Clone OllamaClient (étape 3a) |
| P0-3 | ≤1 actif → LLM call inutile | Early return dans les 2 fonctions async (étape 7) |
| P0-4 | Bannés dans prompts/résolution | Filtrer is_banned AVANT tout, résoudre contre actifs uniquement (étape 7) |
| P0-5 | Force-stop pendant vote → TurnStarted fantôme | Check annulation entre vote et TurnStarted (étape 8) |
| P0-6 | N=2 → tie garanti en Démocratique | Shortcut vers authoritarian pour N=2 (étape 7) |
| P1-1 | Name matching fragile | Helper `match_speaker_name` 3 couches (étape 4) |
| P1-2 | Votes zéro-info → tiebreak inutile | Détection total_recognized == 0 → fallback (étape 7) |
| P1-3 | Tour 1 summary vide | Topic explicite dans tous les prompts (étape 6) |
| P2-1 | Votes partiels | Utiliser résultats partiels si ≥1 vote OK (étape 7) |
| P2-2 | Participants manquants en Autoritaire | Appendus par intervention_number (étape 7) |
| P0-7 | `OllamaClient` ne dérive pas Clone | Ajouter `#[derive(Clone)]` (étape 3a) |
| P0-8 | events.rs : `#[serde(rename_all)]` par variant | Attribut ajouté sur `DeterminingOrder` (étape 3b) |
| P2-3 | Tests turn_manager : `GladIAteurConfig` a `emoji` | Inclure `emoji: None` dans les helpers de test |

---

## Vérification

1. `cargo check` — compilation OK (match exhaustif, pas de borrow issues)
2. `cargo test` — tests existants passent + nouveaux tests :
   - `parse_vote`, `parse_authoritarian_order` (json_parser)
   - `match_speaker_name` avec cas exact/prefix/contains/échec
3. `npm run build` — build frontend OK
4. Test manuel :
   - Setup → grille 2×2 → vérifier descriptions affichées pour chaque mode
   - Discussion 3 gladiateurs en mode démocratique → tous parlent, ordre varie
   - Discussion 2 gladiateurs en mode démocratique → shortcut autoritaire, pas de tie loop
   - Discussion en mode autoritaire → ordre change selon contexte
   - Force-stop pendant vote → pas de TurnStarted fantôme
   - Gladiateur banni → pas inclus dans votes/résolution
