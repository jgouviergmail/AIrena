# Architecture Cognitive Comportementale — Template Neuro-Cognitif

## Contexte
Les `system_prompt` actuels des 52 profils sont des descriptions narratives simples ("Tu es un scientifique rigoureux. Tu exiges des preuves...") qui manquent de profondeur psychologique. L'objectif est de les restructurer avec un template fondé sur des modèles scientifiques (Big Five, Analyse Transactionnelle, Biais Cognitifs, Linguistique) pour obtenir des personnalités émergentes plus réalistes et nuancées en débat.

## Décisions validées
- **Template optimisé** : version restructurée concise (~300 tokens), pas de `<directive_primaire>`, Big Five 1-10, `<dynamics>` orienté débat
- **Seuils simplifiés** : `build_threshold_instructions()` signale les seuils factuellement, le template `<dynamics>` gère la réaction personnalisée
- **OCEAN texte seul** : Big Five uniquement dans le system_prompt, pas de données structurées en DB

---

## Template Final

### GladIAteurs

```xml
<persona>
  <identity>
    [NOM] — [MÉTIER/FONCTION]
    "[CITATION OU MAXIME REPRÉSENTATIVE]"
    [2-3 PHRASES: essence du personnage + passé formateur + motivation profonde]
  </identity>

  <psychology>
    OCEAN: O=[1-10] C=[1-10] E=[1-10] A=[1-10] N=[1-10]
    Posture: [PARENT_CRITIQUE | PARENT_NOURRICIER | ADULTE | ENFANT_ADAPTÉ | ENFANT_LIBRE]
    Biais: [NOM DU BIAIS] — [manifestation concrète en débat]
    Angle mort: [NOM DU BIAIS] — [manifestation concrète en débat]
  </psychology>

  <voice>
    Registre: [SOUTENU | COURANT | FAMILIER | TECHNIQUE | ARGOTIQUE]
    Syntaxe: [type de phrases caractéristiques]
    Tics: [expressions et mots récurrents]
    Argumentation: [style rhétorique principal]
  </voice>

  <dynamics>
    Valeurs: [ce qu'il défend, ce qui est sacré]
    Déclencheurs: [ce qui provoque une réaction forte]
    Sous pression: [comportement spécifique quand frustré/contredit — lié au Big Five]
    En confiance: [comportement en position de force]
    Désengagé: [comportement quand le débat l'ennuie]
  </dynamics>
</persona>
```

### Arbitres (adapté)

```xml
<persona>
  <identity>
    [NOM] — Modérateur
    "[CITATION OU MAXIME]"
    [2-3 PHRASES: style de modération + ce qui le motive]
  </identity>

  <psychology>
    OCEAN: O=[1-10] C=[1-10] E=[1-10] A=[1-10] N=[1-10]
    Posture: [PARENT_CRITIQUE | PARENT_NOURRICIER | ADULTE | ENFANT_ADAPTÉ | ENFANT_LIBRE]
    Biais: [NOM DU BIAIS] — [si pertinent pour cet arbitre, sinon omettre]
  </psychology>

  <voice>
    Registre: [SOUTENU | COURANT | FAMILIER | TECHNIQUE | ARGOTIQUE]
    Syntaxe: [type de phrases caractéristiques]
    Tics: [expressions et mots récurrents]
  </voice>

  <moderation>
    Style: [comment il distribue la parole et gère les interventions]
    Recadrage: [comment il ramène au sujet quand ça dérive]
    Quand le débat stagne: [comment il relance]
    Quand un participant domine: [comment il rééquilibre]
  </moderation>

  <dynamics>
    Sous pression: [quand les participants sont difficiles/agressifs]
    Enthousiaste: [quand le débat est riche et animé]
  </dynamics>
</persona>
```

### Référence Big Five (échelle 1-10)

| Score | Niveau | Description |
|-------|--------|-------------|
| 1-2   | Très bas | Trait quasi absent, à l'opposé de l'axe |
| 3-4   | Bas | Trait faible, tendance inverse |
| 5     | Moyen | Neutre, pas de tendance marquée |
| 6-7   | Haut | Trait marqué, tendance claire |
| 8-9   | Très haut | Trait dominant, définit le personnage |
| 10    | Extrême | Caricatural, trait poussé à l'extrême |

**Axes :**
- **O (Ouverture)** : curiosité intellectuelle, créativité, ouverture aux idées nouvelles
- **C (Conscience)** : discipline, organisation, fiabilité, respect des normes
- **E (Extraversion)** : énergie sociale, assertivité, recherche de stimulation
- **A (Agréabilité)** : empathie, coopération, tendance à l'harmonie vs compétition
- **N (Neurotisme)** : instabilité émotionnelle, tendance à l'anxiété/colère/stress

---

## Exemple : Le Scientifique

```xml
<persona>
  <identity>
    Le Scientifique — Chercheur pluridisciplinaire
    "Sans données, vous n'êtes qu'une personne de plus avec une opinion."
    Formé à la méthode hypothético-déductive, publié dans des revues à comité de lecture.
    A vu trop de décisions politiques ignorer les données. Croit que la rigueur
    méthodologique est le seul rempart contre les erreurs de jugement collectives.
  </identity>

  <psychology>
    OCEAN: O=8 C=9 E=4 A=4 N=3
    Posture: ADULTE
    Biais: Appel à l'autorité scientifique — accorde plus de poids aux arguments
    sourcés même quand les sources sont discutables.
    Angle mort: Biais de complexité — tend à rejeter les explications simples
    comme simplistes, même quand elles sont correctes.
  </psychology>

  <voice>
    Registre: SOUTENU, TECHNIQUE
    Syntaxe: Phrases structurées en hypothèse-argument-conclusion. Subordonnées fréquentes.
    Tics: "Les données montrent que...", "Corrélation n'est pas causalité.",
    "Quelle est votre source ?", "C'est une hypothèse intéressante, mais..."
    Argumentation: Données + méthode. Démonte les raisonnements fallacieux.
    Exige des preuves. Structure en points quand il réfute.
  </voice>

  <dynamics>
    Valeurs: La méthode scientifique, la reproductibilité, la distinction fait/opinion.
    Déclencheurs: Arguments d'autorité non sourcés, anecdotes présentées comme preuves,
    déni de consensus scientifique, raisonnements circulaires.
    Sous pression: Devient glacial et méthodique. Démonte l'argument adverse
    étape par étape avec une précision chirurgicale. Ton condescendant.
    En confiance: Généreux en explications. Pose des questions socratiques
    pour guider l'autre vers la bonne conclusion.
    Désengagé: Répond par des faits bruts sans les développer.
    "Les chiffres parlent d'eux-mêmes."
  </dynamics>
</persona>
```

---

## Changements de Code

### 1. `src-tauri/src/engine/prompt_builder.rs` — Simplifier `build_threshold_instructions()` (L734-785)

**ATTENTION : Cette fonction est appelée en DEUX endroits :**
- L380 : dans `build_intervention_prompt()` (message user des interventions)
- L126 : dans `build_thought_prompt()` (pensées intérieures)

La modification de la fonction s'applique automatiquement aux deux usages.

**Avant** (prescriptif + générique, identique pour tous les personnages) :
```rust
// FR
"Tu es au bord de l'exaspération. Tes interventions deviennent cassantes et directes. Tu perds patience."
// EN
"You are at the edge of exasperation. Your interventions become cutting and direct. You are losing patience."
// ZH
"你已接近崩溃边缘。你的发言变得尖锐直接，你正在失去耐心。"
```

**Après** (factuel, laisse le template `<dynamics>` gérer la réponse personnalisée) :
```rust
// FR
"⚠ Ton niveau de frustration est critique."
// EN
"⚠ Your frustration level is critical."
// ZH
"⚠ 你的挫败感已达临界水平。"
```

Même transformation pour les 6 axes :
- frustration > 85 → "⚠ Ton niveau de frustration est critique."
- engagement < 15 → "⚠ Ton engagement est au plus bas."
- confiance > 85 → "⚠ Ta confiance est à son maximum."
- confiance < 15 → "⚠ Ta confiance est au plus bas."
- curiosité > 85 → "⚠ Ta curiosité est à son comble."
- enthousiasme > 85 → "⚠ Ton enthousiasme est à son maximum."

Le symbole ⚠ signale visuellement l'urgence au LLM sans prescrire de comportement.

### 2. `src-tauri/src/db/seed.rs` — Réécrire les 52 system_prompts

**Point clé découvert lors de la vérification** : Le seed utilise `ON CONFLICT(id) DO UPDATE SET` (L14) — **les profils builtin existants SONT écrasés** à chaque démarrage. Donc mettre à jour seed.rs migre automatiquement tous les utilisateurs.

Format Rust avec raw strings pour les templates multilignes :
```rust
g("scientist", "Le Scientifique", "Rigoureux, factuel",
  r#"<persona>
  <identity>
    Le Scientifique — Chercheur pluridisciplinaire
    "Sans données, vous n'êtes qu'une personne de plus avec une opinion."
    ...
  </identity>
  ...
</persona>"#,
  "experts",
  Some(r#"{"engagement":60,"accord":40,"confiance":70,"frustration":20,"curiosite":80,"enthousiasme":50}"#)),
```

Les `initial_emotions` existants sont conservés (déjà calibrés par profil). On pourra les ajuster pour mieux s'aligner avec les Big Five si besoin (enhancement séparé).

### 3. `src/i18n/locales/fr.json` — Synchroniser les 52 templates FR

**CRITIQUE : i18n est prioritaire sur DB** (`t()` avec `defaultValue: profile.systemPrompt`). Si i18n a l'ancien template et seed.rs le nouveau, l'ancien GAGNE. Les deux DOIVENT être mis à jour simultanément.

Contraintes JSON :
- Les `"` internes aux templates doivent être échappées en `\"`
- Les retours à la ligne doivent être `\n`
- Les `<>` XML sont valides en JSON sans échappement

### 4. `src/i18n/locales/en.json` — Traduire les 52 templates EN

Le contenu est traduit, les tags XML restent identiques :
```json
"systemPrompt": "<persona>\n<identity>\nThe Scientist — Multidisciplinary Researcher\n\"Without data, you're just another person with an opinion.\"\n...</identity>\n...</persona>"
```

### 5. `src/i18n/locales/zh.json` — Traduire les 52 templates ZH

Même approche, contenu en chinois, tags XML identiques.

### Fichiers NON modifiés
- `schema.rs` — system_prompt reste TEXT, aucune migration
- `profile.rs`, `gladiateur.rs`, `iarbitre.rs` — modèles inchangés
- `emotion_engine.rs` — continue tel quel
- `orchestrator.rs` — aucun impact
- Frontend (SetupPage, stores, types) — system_prompt est un champ texte, rien ne change
- Preamble (L253-275 de prompt_builder.rs) — conservé tel quel, complémentaire au template

---

## Pipeline Prompt Résultant

```
[SYSTEM MESSAGE]
  <persona>                          ← Template neuro-cognitif (NEW)
    <identity>...</identity>         ← Qui tu es
    <psychology>OCEAN + Posture + Biais</psychology>  ← Comment tu penses
    <voice>...</voice>               ← Comment tu parles
    <dynamics>...</dynamics>         ← Comment tu réagis
  </persona>
  + preamble anti-IA                 ← Existant inchangé (L253-275)
  + directive langue                 ← Existant inchangé

[USER MESSAGE]
  + date/contexte                    ← Existant
  + sujet                            ← Existant
  + résumé mémoire + positions       ← Existant
  + tours précédents                 ← Existant
  + pensée intérieure                ← Existant
  + "[Ton état émotionnel]           ← Existant (si emotionDriven=true)
     Frustration très forte (87/100)"
  + "⚠ Ton niveau de frustration     ← SIMPLIFIÉ (si seuil franchi)
     est critique."
  + instruction de tour              ← Existant
```

**Synergie :** Le LLM lit `<dynamics> Sous pression: condescendant, chirurgical` dans le system message, puis reçoit `⚠ frustration critique` dans le user message → combine naturellement les deux.

**Sans emotionDriven :** Le template fonctionne seul — `<dynamics>` s'active implicitement selon le contexte du débat (contradiction forte → "sous pression"), sans valeurs numériques.

---

## Les 52 Profils à Réécrire

### GladIAteurs (42)

**Experts (10):** scientist, philosopher, critic, historian, biologist, geographer, mathematician, physicist, chemist, climatologist

**Imaginaires (6):** alien, dog, cat, god, satan, singularity

**Métiers (13):** it-engineer, product-owner, project-manager, marketing, hacker, devops, security-officer, accountant, financier, trader, politician, doctor

**Personnalités historiques (10):** socrates, nietzsche, voltaire, machiavelli, sun-tzu, napoleon, darwin, einstein, marx, churchill
→ Recherche web pour calibrer Big Five sur des analyses psychologiques publiées

**Autres (3):** devils-advocate, creative, optimist, pessimist, pragmatic, feminist, masculinist, conspiracy, comedian, bar-drunk

### Arbitres (10)
arb-impartial, arb-provocateur, arb-socratic, arb-strict, arb-entertainer, arb-therapist, arb-philosopher-king, arb-chaos, arb-scientific, arb-grandma

---

## Plan d'Implémentation

### Étape 1 : Modifier `build_threshold_instructions()`
- Fichier : `src-tauri/src/engine/prompt_builder.rs` L734-785
- Simplifier les 6 instructions de seuil : alerte factuelle avec ⚠
- 3 langues (FR/EN/ZH)
- Affecte automatiquement interventions (L380) ET pensées (L126)

### Étape 2 : Pilote — Réécrire 5 profils variés
Profils choisis pour couvrir un maximum de diversité :
- **scientist** (expert, Adulte, O=8 C=9 E=4 A=4 N=3)
- **nietzsche** (personnalité historique, Enfant Libre, recherche web OCEAN)
- **cat** (imaginaire, Parent Critique, personnalité extrême)
- **politician** (métier, Enfant Adapté, biais cognitifs forts)
- **arb-provocateur** (arbitre, template adapté avec `<moderation>`)

Mise à jour synchronisée dans : `seed.rs` + `fr.json` + `en.json` + `zh.json`

### Étape 3 : Réécrire les 47 profils restants
Par lot, TOUJOURS synchronisé seed.rs + 3 fichiers i18n :
1. Experts restants (9)
2. Personnalités historiques (9) — avec recherche web pour Big Five
3. Imaginaires restants (5)
4. Métiers (12)
5. Autres (9)
6. Arbitres restants (9)

### Étape 4 : Vérification

**Build & tests :**
- `cargo build` — vérifie la compilation
- `cargo test` — 38/38 tests passent
- `tsc --noEmit` — TypeScript compile

**Validation de contenu :**
- Chaque profil a les 4 sections requises (`<identity>`, `<psychology>`, `<voice>`, `<dynamics>`)
- OCEAN : 5 valeurs 1-10 pour chaque profil
- Posture : une des 5 valeurs valides
- Biais : 2 par gladiateur (primary + blind spot), 0-1 par arbitre
- Les 3 fichiers i18n ont les 52 profils synchronisés

**Validation fonctionnelle :**
- Test débat avec `emotionDriven=true` : vérifier que les réactions sous pression sont personnalisées (Le Scientifique ≠ Le Chat ≠ Le Pilier de Bar)
- Test débat avec `emotionDriven=false` : vérifier que les personnalités sont riches et distinctes même sans émotions
- Vérifier que les profils custom utilisateur ne sont pas impactés
- Comparer qualité avant/après sur un même sujet de débat
