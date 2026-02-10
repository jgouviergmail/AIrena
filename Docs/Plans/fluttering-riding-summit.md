# Plan : Améliorations UX/UI + vérifications système émotionnel

## Contexte

Le système émotionnel avancé (plan précédent) est implémenté et fonctionnel. L'utilisateur demande :
1. Vérifier que le système d'émotions fonctionne correctement (pas d'erreurs silencieuses)
2. Améliorations UX/UI sur toutes les pages (SettingsPage, SetupPage 4 steps, ArenaPage, SummaryPage)
3. Sidebar émotionnelle redimensionnable avec noms complets et barres remplies

---

## Phase 1 — Vérification système émotionnel

### 1.1 Vérification code (pas de modifications)
- Relire les logs de la dernière discussion pour identifier d'éventuelles erreurs silencieuses
- Vérifier : émission initiale (tous participants), rule-based (likes/dislikes/bans), LLM analysis (graceful fallback), contagion, thresholds, history snapshots
- Signaler tout problème trouvé à l'utilisateur

### 1.2 Optimisations potentielles identifiées (à confirmer)
- Les suggestions seront listées dans un rapport séparé après analyse

---

## Phase 2 — SettingsPage (`src/pages/SettingsPage.tsx`)

**Objectif** : Mieux organisée et plus jolie visuellement (fonctionnel inchangé)

### Modifications :
- Ajouter des **icônes** devant chaque titre de section :
  - Général → `Settings` (lucide)
  - Connexion Ollama → `Server` (lucide)
  - Recherche Internet → `Globe` (lucide, déjà importé)
- Ajouter une **ligne de séparation** sous chaque titre de section (border-b dans `Section` component)
- **Sections en cartes** : Envelopper chaque section dans un `rounded-xl border border-border bg-card/50 p-5` pour un rendu plus structuré
- Améliorer l'espacement et la hiérarchie visuelle des champs
- **Aucun changement fonctionnel**

### Fichiers modifiés :
- `src/pages/SettingsPage.tsx`

---

## Phase 3 — SetupPage Step 1 (`StepTopic`)

**Fichier** : `src/pages/SetupPage.tsx` (fonction `StepTopic`, lignes 203-285)

### Modifications :
1. **Réordonner** : Langue de la discussion en PREMIER (avant le sujet)
2. **Renommer** : "Timeout intervention (secondes)" → "Timeout intervention utilisateur (secondes)"
3. **Repositionner** : Timeout sur une ligne sous maxTurns (pleine largeur au lieu de grid 2 cols)
4. **Icônes** devant chaque titre :
   - Langue → `Globe` (déjà importé)
   - Sujet → `MessageSquare` (lucide)
   - Nombre de tours → `Repeat` (lucide)
   - Timeout → `Clock` (lucide)
5. **Lignes de séparation** sous chaque titre (border-b avec marge)

### i18n :
- `setup.userTimeout` : "Timeout intervention (secondes)" → "Timeout intervention utilisateur (secondes)" (fr)
- Équivalents en/zh

### Fichiers modifiés :
- `src/pages/SetupPage.tsx`
- `src/i18n/locales/fr.json`
- `src/i18n/locales/en.json`
- `src/i18n/locales/zh.json`

---

## Phase 4 — SetupPage Step 2 (`StepArbitre`)

**Fichier** : `src/pages/SetupPage.tsx` (fonction `StepArbitre`, lignes 287-515)

### Modifications :
1. **Recherche web intro** : Remplacer la ligne switch+label actuelle (lignes 470-494) par :
   - Un **titre** : "Utiliser des informations internet pour introduire" avec icône `Globe`
   - Le **switch** en dessous, avec labels "non" / "oui"
2. **Icônes** devant chaque titre :
   - Profil → `UserCircle` (lucide)
   - Nom → `Tag` (lucide)
   - Prompt → `FileText` (lucide)
   - Distribution → `Shuffle` (lucide)
   - Recherche web → `Globe`
   - Params LLM → `Sliders` (lucide)
3. **Lignes de séparation** sous chaque titre

### i18n :
- Ajouter `setup.arbitreWebSearchTitle` : "Utiliser des informations internet pour introduire" (fr)
- Ajouter `setup.switchNo` / `setup.switchYes` : "Non" / "Oui" (fr)
- Équivalents en/zh

### Fichiers modifiés :
- `src/pages/SetupPage.tsx`
- `src/i18n/locales/fr.json`
- `src/i18n/locales/en.json`
- `src/i18n/locales/zh.json`

---

## Phase 5 — SetupPage Step 3 (`StepGladiateurs`)

**Fichier** : `src/pages/SetupPage.tsx` (fonction `StepGladiateurs`, lignes 519-850)

### Modifications :
1. **Renommer** : "Recherches par GladIAteur (discussion entière)" → "Nombre de recherches internet par GladIAteur (discussion entière)"
2. **Réordonner icônes** par GladIAteur : expand (ChevronDown) AVANT save (Save)
   - Actuellement (ligne 768-813) : reset → save → expand → delete
   - Nouveau : reset → expand → save → delete
3. **Icônes** devant chaque titre :
   - Comportement émotionnel → `Heart` (lucide)
   - Recherches internet → `Globe` (remplacer l'icône `Search` actuelle)
   - Choisir un profil → `Users` (lucide)
4. **Lignes de séparation** sous chaque titre

### i18n :
- `setup.webSearchMaxPerGladiateur` : "Recherches par GladIAteur..." → "Nombre de recherches internet par GladIAteur (discussion entière)" (fr)
- Équivalents en/zh

### Fichiers modifiés :
- `src/pages/SetupPage.tsx`
- `src/i18n/locales/fr.json`
- `src/i18n/locales/en.json`
- `src/i18n/locales/zh.json`

---

## Phase 6 — SetupPage Step 4 (`StepSummary`)

**Fichier** : `src/pages/SetupPage.tsx` (fonction `StepSummary`, lignes 852-904)

### Modifications :
- Remplacer l'affichage actuel des gladiateurs (noms séparés par virgule, ligne 891) par une **liste verticale** avec emoji devant chaque nom :
  ```tsx
  <div className="space-y-1">
    {gladiateurs.map((g) => (
      <div key={g.id} className="flex items-center gap-2 text-sm text-foreground">
        <span>{g.emoji ?? getProfileEmoji(g.name, g.systemPrompt)}</span>
        <span className="font-medium">{g.name}</span>
      </div>
    ))}
  </div>
  ```
- Importer `getProfileEmoji` dans le composant (déjà importé au niveau du fichier)

### Fichiers modifiés :
- `src/pages/SetupPage.tsx`

---

## Phase 7 — ArenaPage : Sidebar redimensionnable

**Objectif** : Séparateur déplaçable entre discussion et sidebar émotionnelle

### 7.1 Composant `ResizeDivider.tsx` (NEW)
- `src/components/layout/ResizeDivider.tsx`
- Barre verticale draggable (4px, hover→primary, cursor col-resize)
- Props : `onResize: (deltaX: number) => void`
- Gère mousedown → mousemove → mouseup (cleanup on unmount)
- Pas de nouvelle dépendance (CSS + événements natifs)

### 7.2 ArenaPage layout
- Remplacer la sidebar fixe `w-[280px]` par un state `sidebarWidth` (default 280, min 200, max 500)
- Insérer `ResizeDivider` entre le contenu principal et la sidebar
- Passer `width` en prop à `EmotionSidebar` (au lieu de `w-[280px]` hardcodé)
- Layout : `flex-1` (main) + `ResizeDivider` + `EmotionSidebar` (style width)

### 7.3 EmotionSidebar : width dynamique
- Remplacer `w-[280px]` par `style={{ width: ${width}px }}` passé en prop
- **Labels complets** : Augmenter `w-[60px]` → `w-[80px]` dans EmotionAxisSlider pour afficher les noms complets ("Engagement", "Frustration"...)
- Le redimensionnement permet à l'utilisateur d'élargir davantage si besoin

### Fichiers modifiés :
- `src/components/layout/ResizeDivider.tsx` (NEW)
- `src/pages/ArenaPage.tsx`
- `src/components/emotion/EmotionSidebar.tsx`
- `src/components/emotion/EmotionAxisSlider.tsx`

---

## Phase 8 — ArenaPage : Description émotionnelle brève

**Objectif** : Entre le nom du GladIAteur et les barres, afficher UNE phrase résumant son état émotionnel

### 8.1 Backend : Générer la description (`src-tauri/src/engine/prompt_builder.rs`)
- Nouvelle fn `pub fn summarize_emotional_state(emotions: &EmotionalProfile, lang: &str) -> String`
- Logique : trouver l'axe dominant (le plus éloigné de 50), puis générer une phrase courte :
  - frustration ≥ 70 : "Tendu et agacé" / "Tense and irritated" / "紧张且烦躁"
  - enthousiasme ≥ 70 : "Enthousiasmé" / "Enthusiastic" / "热情高涨"
  - engagement ≤ 30 : "Détaché" / "Detached" / "超然"
  - curiosite ≥ 70 : "Très curieux" / "Very curious" / "非常好奇"
  - confiance ≥ 70 : "Très confiant" / "Very confident" / "非常自信"
  - confiance ≤ 30 : "Hésitant" / "Hesitant" / "犹豫不决"
  - accord ≤ 30 : "En désaccord" / "Disagreeing" / "不同意"
  - accord ≥ 70 : "En accord" / "In agreement" / "赞同"
  - Défaut (tout neutre) : "Neutre" / "Neutral" / "中立"
- Combine les 2 axes les plus extrêmes (max 2 descripteurs) séparés par ", "

### 8.2 Event extension
- Ajouter `mood_summary: Option<String>` au variant `EmotionUpdated` dans `events.rs`
- L'orchestrateur calcule et passe le résumé à chaque émission `EmotionUpdated`

### 8.3 Frontend
- Types : Ajouter `moodSummary?: string` à l'event data `emotionUpdated` dans `types.ts`
- Store : Stocker `moodSummary: Map<string, string>` dans useArenaStore
- Handler `emotionUpdated` : mettre à jour `moodSummary` map
- `ParticipantEmotionCard` : Afficher `moodSummary` en italique sous le nom, avant les sliders

### Fichiers modifiés :
- `src-tauri/src/engine/prompt_builder.rs`
- `src-tauri/src/models/events.rs`
- `src-tauri/src/engine/orchestrator.rs` (toutes les émissions EmotionUpdated)
- `src/lib/types.ts`
- `src/stores/useArenaStore.ts`
- `src/components/emotion/ParticipantEmotionCard.tsx`

---

## Phase 9 — Barres d'émotion remplies selon le score

**Objectif** : Le slider natif `<input type="range">` ne montre pas visuellement le remplissage. Remplacer par une barre visuelle remplie + thumb draggable.

### 9.1 EmotionAxisSlider : Barre remplie
- Remplacer `<input type="range">` par un div custom avec :
  - Background track : `bg-muted rounded-full h-1.5`
  - Fill bar : `style={{ width: ${value}% }}` avec `background: oklch(0.65 0.18 ${hue})`
  - Thumb invisible `<input type="range">` en overlay absolu (pour garder l'interaction native)
- Le range input est en `opacity-0 absolute inset-0 cursor-pointer` (invisible mais garde drag/click)
- La barre visuelle reflète `displayValue`

### Fichiers modifiés :
- `src/components/emotion/EmotionAxisSlider.tsx`

---

## Phase 10 — SummaryPage

**Fichier** : `src/pages/SummaryPage.tsx`

### Modifications :
1. **Stats enrichies** :
   - Afficher la liste des participants avec emojis (comme step 4 setup) au lieu d'un simple nombre
   - Ajouter le modèle utilisé dans les stats
   - Ajouter la durée (calculée entre discussionStarted et end, ou à défaut nombre de tours × estimation)
2. **Synthèse formatée** :
   - Remplacer le `<p className="whitespace-pre-wrap">` simple par un rendu markdown basique :
     - `**texte**` → bold
     - `- item` → liste à puces
     - `\n\n` → paragraphes séparés
   - Pas de dépendance markdown lourde — regex simple pour bold + listes
3. **Style amélioré** :
   - Topic affiché plus grand (text-base au lieu de text-sm dans StatCard)
   - Meilleur espacement vertical
   - Boutons d'action avec plus de contraste visuel

### Fichiers modifiés :
- `src/pages/SummaryPage.tsx`

---

## Phase 11 — i18n consolidé

### Nouvelles clés à ajouter dans les 3 fichiers de locale :

**`fr.json`** :
```json
"setup.userTimeout": "Timeout intervention utilisateur (secondes)",
"setup.arbitreWebSearchTitle": "Utiliser des informations internet pour introduire",
"setup.switchNo": "Non",
"setup.switchYes": "Oui",
"setup.webSearchMaxPerGladiateur": "Nombre de recherches internet par GladIAteur (discussion entière)",
"summary.model": "Modèle",
"summary.participantsList": "Participants"
```

**`en.json`** :
```json
"setup.userTimeout": "User intervention timeout (seconds)",
"setup.arbitreWebSearchTitle": "Use internet information for introduction",
"setup.switchNo": "No",
"setup.switchYes": "Yes",
"setup.webSearchMaxPerGladiateur": "Number of internet searches per GladIAteur (entire discussion)",
"summary.model": "Model",
"summary.participantsList": "Participants"
```

**`zh.json`** :
```json
"setup.userTimeout": "用户干预超时（秒）",
"setup.arbitreWebSearchTitle": "使用互联网信息进行介绍",
"setup.switchNo": "否",
"setup.switchYes": "是",
"setup.webSearchMaxPerGladiateur": "每个 GladIAteur 的互联网搜索次数（整个讨论）",
"summary.model": "模型",
"summary.participantsList": "参与者"
```

### Fichiers modifiés :
- `src/i18n/locales/fr.json`
- `src/i18n/locales/en.json`
- `src/i18n/locales/zh.json`

---

## Résumé des fichiers modifiés

| Fichier | Phase | Action |
|---------|-------|--------|
| `src/pages/SettingsPage.tsx` | 2 | Icônes, séparateurs, cartes sections |
| `src/pages/SetupPage.tsx` | 3-6 | Steps 1-4 : réordonnancement, icônes, séparateurs, labels, liste gladiateurs |
| `src/pages/ArenaPage.tsx` | 7 | Sidebar redimensionnable (state width + ResizeDivider) |
| `src/pages/SummaryPage.tsx` | 10 | Stats enrichies, synthèse formatée, style amélioré |
| `src/components/layout/ResizeDivider.tsx` | 7 | NEW — Barre draggable verticale |
| `src/components/emotion/EmotionSidebar.tsx` | 7 | Width dynamique (prop), retire w-[280px] |
| `src/components/emotion/EmotionAxisSlider.tsx` | 7,9 | Labels plus larges, barre remplie custom |
| `src/components/emotion/ParticipantEmotionCard.tsx` | 8 | Affiche moodSummary |
| `src/stores/useArenaStore.ts` | 8 | moodSummary map |
| `src/lib/types.ts` | 8 | moodSummary dans EmotionUpdated event |
| `src-tauri/src/engine/prompt_builder.rs` | 8 | summarize_emotional_state() |
| `src-tauri/src/models/events.rs` | 8 | mood_summary dans EmotionUpdated |
| `src-tauri/src/engine/orchestrator.rs` | 8 | Passer mood_summary dans toutes les émissions |
| `src/i18n/locales/fr.json` | 3-5,11 | Nouveaux labels |
| `src/i18n/locales/en.json` | 3-5,11 | Nouveaux labels |
| `src/i18n/locales/zh.json` | 3-5,11 | Nouveaux labels |

---

## Ordre d'implémentation

1. **i18n** (Phase 11) — Ajouter toutes les nouvelles clés en premier
2. **SettingsPage** (Phase 2) — Standalone, pas de dépendances
3. **SetupPage Steps 1-4** (Phases 3-6) — Purement frontend
4. **Barres d'émotion remplies** (Phase 9) — EmotionAxisSlider, standalone
5. **Sidebar redimensionnable** (Phase 7) — ResizeDivider + ArenaPage + EmotionSidebar
6. **Description émotionnelle** (Phase 8) — Backend + frontend, plus complexe
7. **SummaryPage** (Phase 10) — Standalone
8. **Vérification** (Phase 1) — Analyse des logs et tests

---

## Vérification

1. `cargo clippy` — 0 warnings
2. `cargo test` — tous tests passent
3. `npx tsc --noEmit` — 0 erreurs TypeScript
4. Tests manuels :
   - SettingsPage : sections avec icônes et cartes, visuellement amélioré
   - SetupPage step 1 : langue en premier, timeout renommé et repositionné, icônes + séparateurs
   - SetupPage step 2 : titre web search + switch dessous, icônes + séparateurs
   - SetupPage step 3 : label web search renommé, expand avant save, icônes + séparateurs
   - SetupPage step 4 : gladiateurs en liste avec emojis
   - ArenaPage : sidebar redimensionnable, noms d'émotions complets, barres remplies, description émotionnelle
   - SummaryPage : stats enrichies, synthèse mieux formatée
   - Pas d'erreurs silencieuses dans la console
