# Plan : Dynamic Behavioral Directive — Meta-orchestrateur de comportement

## Contexte

L'analyse approfondie des logs de discussion a identifie 5 causes racines de monotonie dans les debats :
1. **Instructions finales statiques** — Le meme texte ("Lance-toi dans le debat...") est injecte a chaque tour pour chaque participant
2. **Emotions descriptives, pas prescriptives** — "Tu es frustre a 75/100" mais aucun pont vers un comportement concret
3. **Pas de self-memory** — Le speaker ne sait pas ce qu'il a deja dit, donc se repete
4. **Pas de tracking relationnel** — Allies/rivaux emergent des likes/dislikes mais ne sont jamais exploites
5. **Tics verbaux omnipresents** — Les "Non mais allo ?!" et "Mais pourquoi ?" deviennent des crutches

**Solution** : Un systeme de "Dynamic Behavioral Directive" en 5 couches qui remplace l'instruction finale statique par une directive unique, contextuelle, par speaker par tour.

## Architecture

```
Orchestrator (turn loop)
  |
  +-- build_speaker_turn_context() --> SpeakerTurnContext
  |     |-- emotions (from GladIAteurState.emotions)
  |     |-- relationships (from cumulative_reactions)
  |     |-- own_previous_messages (from speaker_own_messages)
  |     |-- parsed_dynamics (from dynamics_cache)
  |     |-- ocean (from system_prompt)
  |     |-- turn_position, total_turns, speakers_this_turn, etc.
  |
  +-- build_dynamic_directive(context) --> DirectiveOutput  [only if emotion_driven=true]
  |     |-- Layer 1: Emotion->Behavior bridge
  |     |-- Layer 2: Relationship hints (turns 2+)
  |     |-- Layer 3: Speech act selection (turns 2+)
  |     |-- Layer 4: Self-memory anti-repetition (turns 2+)
  |     |-- Layer 5: Situational awareness
  |
  +-- inject into prompt_builder (replaces static final instruction)
  +-- emit DirectiveGenerated event (for UI visualization)
```

## Fichiers a creer

| Fichier | Role |
|---------|------|
| `src-tauri/src/engine/dynamics_parser.rs` | Parse `<dynamics>` XML from system_prompt |
| `src-tauri/src/engine/directive_builder.rs` | 5-layer directive builder (coeur du systeme) |

## Fichiers a modifier

| Fichier | Modification |
|---------|-------------|
| `src-tauri/src/engine/mod.rs` | Register `dynamics_parser`, `directive_builder` |
| `src-tauri/src/engine/orchestrator.rs` | Add tracking fields, call directive builder, emit event |
| `src-tauri/src/engine/prompt_builder.rs` | Accept `dynamic_directive` param, replace static instruction, add tic rarefaction |
| `src-tauri/src/models/events.rs` | Add `DirectiveGenerated` event variant |
| `src/lib/types.ts` | Add `DirectiveOutput` interface, event type |
| `src/stores/useArenaStore.ts` | Handle `directiveGenerated` event, store directives |
| `src/components/emotion/ParticipantEmotionCard.tsx` | Add collapsible directive panel |
| `src/i18n/locales/fr.json` | Labels directive panel |
| `src/i18n/locales/en.json` | Labels directive panel |
| `src/i18n/locales/zh.json` | Labels directive panel |

## 1. dynamics_parser.rs — Parse `<dynamics>` XML

Parse la section `<dynamics>` du system_prompt de chaque participant au demarrage de la discussion. Resultat cache dans un `HashMap<String, ParsedDynamics>` sur l'orchestrator.

```rust
pub struct ParsedDynamics {
    pub values: String,
    pub triggers: String,
    pub under_pressure: String,
    pub confident: String,
    pub disengaged: String,
    // Arbitre-specific
    pub enthusiastic: Option<String>,
}

pub fn parse_dynamics(system_prompt: &str) -> Option<ParsedDynamics>
```

**Strategie** : Regex `<dynamics>([\s\S]*?)</dynamics>` puis extraction par labels trilingues (meme pattern que les LABELS dans persona-parser.ts cote frontend). Retourne `None` si pas de section `<dynamics>`.

## 2. directive_builder.rs — Coeur du systeme

### Structures

```rust
pub struct SpeakerTurnContext {
    pub speaker_name: String,
    pub speaker_id: String,
    pub emotions: EmotionalProfile,
    pub relationships: Vec<RelationshipHint>,
    pub own_previous_messages: Vec<String>,  // last 2 messages, truncated 200 chars
    pub dynamics: Option<ParsedDynamics>,
    pub ocean: Option<OceanScores>,         // reuse existing parse_ocean_values
    pub turn_number: u32,
    pub total_turns: u32,
    pub speakers_this_turn: Vec<String>,
    pub is_first_speaker_this_turn: bool,
    pub was_recently_banned: bool,
    pub group_avg_frustration: u8,
    pub group_avg_engagement: u8,
    pub discussion_language: String,
}

pub struct RelationshipHint {
    pub other_name: String,
    pub kind: RelationshipKind,
}

pub enum RelationshipKind {
    Ally,           // mutual likes >= 2
    Rival,          // mutual dislikes >= 2
    Tense,          // asymmetric (I like them, they dislike me, or vice versa)
    Neutral,
}

pub enum SpeechAct {
    Challenge,       // Conteste un argument precis
    SteelMan,        // Reformule l'argument adverse en plus fort avant de repondre
    Anecdote,        // Illustre par une histoire personnelle
    Question,        // Pose une question ouverte a un participant
    Provocation,     // Lance une pique deliberee
    Concession,      // Admet un point de l'adversaire
    Redirect,        // Change d'angle sur le meme sujet
    Humor,           // Desamorce par l'humour
    Appeal,          // Appel aux valeurs/emotions du groupe
    Synthesis,       // Resume le debat + nouvelle position
}

pub struct DirectiveOutput {
    pub directive_text: String,           // Full text injected into prompt
    pub speech_act: String,               // Selected act name (for UI)
    pub emotion_behavior: Option<String>, // Layer 1 output (for UI)
    pub relationship_summary: String,     // Layer 2 output (for UI)
}
```

### build_dynamic_directive(ctx: &SpeakerTurnContext) -> DirectiveOutput

**IMPORTANT** : Les 5 couches ne sont actives que si `emotion_driven = true` dans les settings. Quand `emotion_driven = false`, l'instruction finale statique actuelle est conservée telle quelle (aucune directive dynamique). Cela permet a l'utilisateur de choisir entre le mode "brut" (LLM libre) et le mode "orchestré" (meta-orchestrateur actif).

**Layer 1 : Emotion -> Behavior Bridge**
- Map les emotions HIGH (>70) et LOW (<30) vers les champs dynamics du profil
- Exemples :
  - frustration HIGH + dynamics.under_pressure -> "Tu es sous pression : [dynamics.under_pressure]"
  - confiance HIGH + dynamics.confident -> "Tu es en confiance : [dynamics.confident]"
  - engagement LOW + dynamics.disengaged -> "Tu te desengages : [dynamics.disengaged]"
  - curiosite HIGH + dynamics.triggers -> "Ta curiosite est eveille : [dynamics.triggers]"
- Si pas de dynamics parsees, genere une directive generique basee sur l'emotion dominante
- Priority : frustration > engagement > confiance > curiosite > enthousiasme > accord

**Layer 2 : Relationship Hints** (tours 2+)
- Construit a partir de `cumulative_reactions` (HashMap<(speaker_id, target_id), (likes, dislikes)>)
- Seuils : Ally = mutual_likes >= 2, Rival = mutual_dislikes >= 2, Tense = asymmetric
- Injecte : "Tu as un allie dans ce debat : [nom]. Vous avez tendance a vous soutenir."
  ou "Tu as un rival : [nom]. Vos desaccords se multiplient."
  ou "La relation avec [nom] est tendue — il/elle t'a critique mais tu l'as apprecie."
- Si aucune relation notable : omis

**Layer 3 : Speech Act Selection** (tours 2+)
- Pool de 10 actes de parole avec poids de base egal (10% chacun)
- Modificateurs OCEAN :
  - E >= 7: +5% Provocation, +5% Humor
  - A >= 7: +5% SteelMan, +5% Concession
  - O >= 7: +5% Question, +5% Redirect
  - N >= 7: +5% Appeal, +3% Anecdote
  - C >= 7: +5% Synthesis, +3% Challenge
- Modificateurs emotionnels :
  - frustration > 70: +8% Challenge, +5% Provocation
  - confiance > 70: +5% SteelMan, +5% Provocation
  - curiosite > 70: +8% Question, +5% Redirect
  - engagement < 30: +10% Humor, +5% Provocation (provoque pour re-engager)
- Anti-repetition : si `last_speech_act` == acte selectionne, re-roll une fois
- Selection aleatoire ponderee via `rand::distributions::WeightedIndex`
- Injecte : "Pour cette intervention, privilegie l'approche : [description de l'acte]"

**Layer 4 : Self-Memory Anti-Repetition** (tours 2+)
- Injecte les 2 derniers messages du speaker (tronques a 200 chars chacun)
- "Tes interventions precedentes : [msg1] / [msg2]. IMPORTANT : trouve de nouvelles formulations, de nouveaux angles. Ne repete PAS tes arguments."
- Budget : ~150 tokens (400 chars max)

**Layer 5 : Situational Awareness**
- Humeur de groupe : "L'ambiance est tendue (frustration moyenne: X/100)" ou "L'ambiance est detendue"
- Position dans le tour : "Tu es le premier a parler ce tour" / "Tu parles apres [noms]"
- Retour de ban : "Tu reviens apres avoir ete banni. Montre que tu as pris du recul."
- Proximite de fin : (reutilise `build_end_awareness()` existant)
- Turn 1 special : "C'est l'ouverture du debat. Pose ta position clairement." (pas de layers 2-4)

### Assemblage final
```
[Layer 5: Situational awareness]
[Layer 1: Emotion->behavior bridge] (si emotion_driven)
[Layer 2: Relationship hints] (si turns 2+)
[Layer 3: Speech act] (si turns 2+)
[Layer 4: Self-memory] (si turns 2+)
```

Total estime : ~400 tokens (safe dans les 8192 num_ctx)

## 3. orchestrator.rs — Nouveaux champs et integration

### Nouveaux champs sur DiscussionEngine
```rust
// Cumulative reactions: (speaker_id, target_speaker_id) -> (likes, dislikes)
cumulative_reactions: HashMap<(String, String), (u32, u32)>,

// Speaker's own messages for self-memory: speaker_id -> Vec<String> (last 2)
speaker_own_messages: HashMap<String, Vec<String>>,

// Parsed dynamics cache: speaker_id -> ParsedDynamics
dynamics_cache: HashMap<String, ParsedDynamics>,

// Last speech act per speaker: speaker_id -> SpeechAct
last_speech_acts: HashMap<String, SpeechAct>,
```

### Initialisation (dans `start()`)
- Parser `dynamics` de chaque gladiateur et de l'arbitre au demarrage
- Stocker dans `dynamics_cache`

### Integration dans la boucle de tour
1. **Apres `process_reactions()`** : mettre a jour `cumulative_reactions` (incrementer likes/dislikes)
2. **Avant `build_intervention_prompt()`** : appeler `build_speaker_turn_context()` puis `build_dynamic_directive()`
3. **Apres generation** : stocker le message dans `speaker_own_messages` (garder les 2 derniers)
4. **Emettre** `DirectiveGenerated` event avec le `DirectiveOutput`

### build_speaker_turn_context() -> SpeakerTurnContext
Construit le contexte complet pour un speaker donne en aggregeant :
- Emotions depuis `gladiateur.emotions`
- Relations depuis `cumulative_reactions`
- Messages precedents depuis `speaker_own_messages`
- Dynamics depuis `dynamics_cache`
- OCEAN depuis `parse_ocean_values(&gladiateur.config.system_prompt)`
- Metriques de groupe (moyenne frustration/engagement sur tous les gladiateurs actifs)

## 4. prompt_builder.rs — Modifications

### Signature de build_intervention_prompt
Ajouter parametre `dynamic_directive: Option<&str>` en fin de signature.

### Remplacement de l'instruction finale statique
- Si `dynamic_directive` est `Some(text)` (emotion_driven=true) : injecter le texte tel quel
- Si `None` (emotion_driven=false, ou fallback) : garder l'instruction statique actuelle

### Tic rarefaction dans le preamble
Ajouter dans la section preamble (L265-287) :
```
"Tes tics verbaux sont des ponctuations OCCASIONNELLES, pas des bequilles.
 Utilise-les au maximum 1 fois par intervention, jamais en debut de phrase."
```
(trilingue FR/EN/ZH)

### Rendre parse_ocean_values public
Changer `fn parse_ocean_values` -> `pub fn parse_ocean_values` pour reutilisation par le directive builder.

## 5. events.rs — Nouveau variant

```rust
DirectiveGenerated {
    speaker_id: String,
    speaker_name: String,
    speech_act: String,
    emotion_behavior: Option<String>,
    relationship_summary: String,
}
```

Avec `#[serde(rename_all = "camelCase")]` sur ce variant.

## 6. Frontend — UI "Coulisses du debat"

### types.ts
```typescript
interface DirectiveData {
  speechAct: string;
  emotionBehavior: string | null;
  relationshipSummary: string;
}
```

### useArenaStore.ts
- Nouveau state : `directives: Map<string, DirectiveData>` (speaker_id -> last directive)
- Handler `directiveGenerated` : met a jour la map

### ParticipantEmotionCard.tsx
Ajouter une section collapsible "Coulisses" sous les barres d'emotion existantes :
- Icone theatre (drama masks) + chevron toggle
- Contenu :
  - **Acte de parole** : badge colore avec le nom de l'acte (ex: "Provocation", "Concession")
  - **Comportement** : texte court du Layer 1 (si emotion_driven)
  - **Relations** : texte court du Layer 2
- Replie par defaut, se deplie au clic
- Pas de nouveau fichier necessaire — inline dans ParticipantEmotionCard (~40 lignes)

### i18n (3 fichiers)
Ajouter bloc `"directive"` avec ~15 cles :
- Titres : backstage, speechAct, behavior, relationships
- Noms des 10 speech acts
- Labels: ally, rival, tense, neutral

## 7. Ordre d'implementation

1. **`dynamics_parser.rs`** — Fonctions pures, aucune dependance
2. **`directive_builder.rs`** — Depend de dynamics_parser + models (emotion, memory)
3. **`engine/mod.rs`** — Register modules
4. **`orchestrator.rs`** — Nouveaux champs, integration dans la boucle
5. **`prompt_builder.rs`** — Nouveau parametre, tic rarefaction, remplacement instruction statique
6. **`events.rs`** — Nouveau variant
7. **`types.ts`** — Interface DirectiveData
8. **`useArenaStore.ts`** — Handler directiveGenerated
9. **`ParticipantEmotionCard.tsx`** — Section collapsible
10. **`i18n`** — Labels dans les 3 locales

## 8. Edge cases

| Cas | Comportement |
|-----|-------------|
| Tour 1 | Layers 2-4 desactivees, Layer 5 seule + instruction d'ouverture |
| Pas de `<dynamics>` dans le prompt | Layer 1 utilise directives generiques basees sur emotion dominante |
| emotion_driven = false | TOUTES les couches desactivees, instruction statique conservee |
| Aucune relation notable | Layer 2 omise |
| Gladiateurs dupliques (meme profil) | Chacun a ses propres emotions/relations/speech acts — divergence naturelle |
| Arbitre (IArbitre) | Pas de directive dynamique (l'arbitre a son propre prompt de moderation) |
| Speech act identique au dernier | Re-roll une fois, accepte si re-roll identique aussi |

## 9. Verification

- `cargo clippy` — zero warnings
- `cargo test` — tous les tests existants passent
- `npx tsc --noEmit` — zero erreurs TypeScript
- Test manuel : lancer une discussion 5+ tours, verifier :
  - Les directives varient entre les tours dans les logs
  - Le panneau "Coulisses" affiche les actes de parole
  - Les gladiateurs ne repetent plus les memes formulations
  - Les relations (ally/rival) emergent apres 2-3 tours de likes/dislikes mutuels
  - Le pont emotion->comportement active les textes `<dynamics>` du profil
