# AIrena — Documentation Fonctionnelle

> **Version** : 1.13
> **Dernière mise à jour** : 2026-02-27
> **Auteur** : jgouv

---

## Table des matières

1. [Présentation du produit](#1-présentation-du-produit)
2. [Concepts clés](#2-concepts-clés)
3. [Parcours utilisateur](#3-parcours-utilisateur)
4. [Page d'accueil](#4-page-daccueil)
5. [Configuration d'une discussion (Setup)](#5-configuration-dune-discussion-setup)
   - 5.1 [Étape 1 — Sujet & Langue](#51-étape-1--sujet--langue)
   - 5.2 [Étape 2 — Le modérateur (IArbitre)](#52-étape-2--le-modérateur-iarbitre)
   - 5.3 [Étape 3 — Les participants (GladIAteurs)](#53-étape-3--les-participants-gladiateurs)
   - 5.4 [Étape 4 — Mode & Options](#54-étape-4--mode--options)
6. [Discussion en direct (Arena)](#6-discussion-en-direct-arena)
   - 6.1 [Fil de discussion](#61-fil-de-discussion)
   - 6.2 [Contrôles de discussion](#62-contrôles-de-discussion)
   - 6.3 [Intervention utilisateur](#63-intervention-utilisateur)
   - 6.4 [Sidebar émotionnelle](#64-sidebar-émotionnelle)
   - 6.5 [Sidebar document](#65-sidebar-document)
   - 6.6 [Sidebar carte des arguments](#66-sidebar-carte-des-arguments)
   - 6.7 [Indicateur d'activité](#67-indicateur-dactivité)
7. [Modes de discussion](#7-modes-de-discussion)
8. [Système émotionnel](#8-système-émotionnel)
9. [Personnalités cognitives](#9-personnalités-cognitives)
10. [Recherche de connaissances](#10-recherche-de-connaissances)
11. [Modération](#11-modération)
12. [Mode Think (réflexion)](#12-mode-think-réflexion)
13. [Réactions](#13-réactions)
14. [Synthèse et résumé](#14-synthèse-et-résumé)
15. [Historique des discussions](#15-historique-des-discussions)
16. [Paramètres de l'application](#16-paramètres-de-lapplication)
17. [Profils prédéfinis](#17-profils-prédéfinis)
18. [Profils personnalisés](#18-profils-personnalisés)
19. [Internationalisation](#19-internationalisation)
20. [Thème et apparence](#20-thème-et-apparence)
21. [Export et téléchargement](#21-export-et-téléchargement)
22. [RAG — Enrichissement par documents](#22-rag--enrichissement-par-documents)
23. [Surlignage des modifications (document collaboratif)](#23-surlignage-des-modifications-document-collaboratif)
24. [Carte des arguments (Mindmap)](#24-carte-des-arguments-mindmap)
25. [Glossaire](#25-glossaire)
26. [Changelog](#26-changelog)

---

## 1. Présentation du produit

**AIrena** est une application de bureau qui permet d'organiser des discussions entre plusieurs intelligences artificielles locales. L'utilisateur définit un sujet, sélectionne des personnalités IA (« GladIAteurs ») et un modérateur IA (« IArbitre »), puis observe et participe à un échange structuré en temps réel.

### Proposition de valeur

- **100% local** : fonctionne avec Ollama, aucune donnée n'est envoyée vers le cloud (sauf Tavily, optionnel)
- **Multi-perspectives** : confronte plusieurs points de vue sur un même sujet
- **Interactif** : l'utilisateur peut intervenir dans la discussion à tout moment
- **Intelligent** : système émotionnel, personnalités cognitives, mémoire contextuelle
- **Polyvalent** : 8 modes de discussion, du débat contradictoire à la fiction collaborative
- **Trilingue** : interface disponible en français, anglais et chinois

### Public cible

- Chercheurs et étudiants explorant des sujets complexes
- Professionnels souhaitant confronter des perspectives
- Passionnés d'IA voulant tester les capacités de modèles locaux
- Créatifs en quête d'inspiration collaborative

---

## 2. Concepts clés

### GladIAteur

Un **GladIAteur** est un participant IA dans la discussion. Chaque GladIAteur possède :
- Un **nom** et un **emoji** d'avatar
- Un **system prompt** définissant sa personnalité et son expertise
- Des **paramètres LLM** (température, tokens, etc.)
- Un **profil émotionnel** initial (6 axes)
- Un **numéro d'intervention** déterminant son ordre de passage (mode séquentiel)

Il existe des profils prédéfinis (Le Scientifique, Le Philosophe, L'Avocat du Diable…) et l'utilisateur peut créer ses propres profils.

### IArbitre

L'**IArbitre** est le modérateur IA. Il a un rôle unique :
- **Introduction** : présente le sujet et les règles
- **Modération** : évalue chaque intervention et peut émettre des commentaires ou des bans
- **Synthèse** : résume la discussion à la fin
- **Ordre de passage** : en mode autoritaire, il décide seul de l'ordre des intervenants

### Tour

Un **tour** est un cycle complet où chaque GladIAteur non-banni prend la parole une fois (dans un ordre défini par le mode de distribution). À chaque tour :
1. L'ordre des orateurs est déterminé
2. Chaque orateur intervient à son tour
3. Les autres réagissent (like/dislike)
4. Les émotions sont mises à jour
5. Le modérateur évalue les interventions
6. La mémoire est actualisée

### Discussion

Une **discussion** est une séquence complète : introduction → tours → synthèse. Elle est identifiée par un UUID unique et peut être sauvegardée dans l'historique.

---

## 3. Parcours utilisateur

```
┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│  Accueil  │────►│  Setup   │────►│  Arena   │────►│ Synthèse │
│           │     │ (4 étapes)│     │ (direct) │     │          │
└──────────┘     └──────────┘     └──────────┘     └────┬─────┘
      │                                                  │
      │           ┌──────────┐     ┌──────────┐          │
      ├──────────►│Historique│────►│  Détail  │          │
      │           └──────────┘     └──────────┘          │
      │                                                  │
      │           ┌──────────┐                           │
      └──────────►│Paramètres│                           │
                  └──────────┘                           │
                                                         │
                  ┌───────────────────────────────────────┘
                  │ Sauvegarde automatique dans l'historique
                  ▼
```

### Flux principal

1. L'utilisateur arrive sur la **page d'accueil**
2. Il clique sur « Nouvelle discussion »
3. Il configure la discussion en **4 étapes** (sujet, modérateur, participants, options)
4. La **discussion se lance** en temps réel dans l'Arena
5. Il observe, intervient s'il le souhaite, ajuste les émotions
6. La discussion se termine par une **synthèse** de l'IArbitre
7. Il consulte le **résumé** avec statistiques et options de téléchargement
8. La discussion est **automatiquement sauvegardée** dans l'historique

---

## 4. Page d'accueil

La page d'accueil offre trois actions principales :

| Action | Description |
|---|---|
| **Nouvelle discussion** | Redirige vers l'assistant de configuration |
| **Historique** | Accède aux discussions passées |
| **Paramètres** | Configure l'application (Ollama, langue, thème) |

Si une discussion est en cours, un indicateur visuel le signale et le bouton « Nouvelle discussion » est désactivé.

---

## 5. Configuration d'une discussion (Setup)

L'assistant de configuration guide l'utilisateur en 4 étapes avec navigation avant/arrière. Chaque étape affiche un titre descriptif (ex. « Étape 1 / 4 — Paramétrage général de la discussion »).

### 5.1 Étape 1 — Paramétrage général

| Champ | Type | Description |
|---|---|---|
| **Sujet** | Texte libre | Le thème de la discussion (obligatoire) |
| **Langue de discussion** | Sélecteur | Français, Anglais ou Chinois — détermine la langue des prompts et des instructions internes |
| **Nombre max de tours** | Nombre | Limite le nombre de tours (illimité si vide) |
| **Carte des arguments** | Toggle | Active la génération automatique d'une mindmap des thèses et arguments (voir [section 24](#24-carte-des-arguments-mindmap)) |

### 5.2 Étape 2 — Le modérateur (IArbitre)

| Champ | Type | Description |
|---|---|---|
| **Profil** | Sélecteur | Choix parmi les profils IArbitre prédéfinis ou personnalisés |
| **Nom** | Texte | Nom affiché du modérateur |
| **System prompt** | Textarea | Personnalité et instructions du modérateur |
| **Mode de distribution** | Sélecteur | Comment l'ordre de passage est déterminé (voir ci-dessous) |
| **Recherche web (intro)** | Toggle | Active la recherche Tavily avant l'introduction |
| **Recherche Wikipedia (intro)** | Toggle | Active la recherche Wikipedia avant l'introduction |
| **Paramètres LLM** | Sliders | Température, top-p, top-k, tokens max, contexte, repeat penalty |

#### Modes de distribution des tours

| Mode | Fonctionnement |
|---|---|
| **Séquentiel** | Ordre fixe basé sur le numéro d'intervention de chaque participant |
| **Aléatoire** | Mélange aléatoire à chaque tour |
| **Démocratique** | Chaque GladIAteur vote pour classer les autres (vote Borda masqué) ; l'IArbitre départage les ex-æquo |
| **Autoritaire** | L'IArbitre seul décide de l'ordre de passage à chaque tour |

### 5.3 Étape 3 — Les participants (GladIAteurs)

L'utilisateur ajoute entre 2 et N GladIAteurs depuis les profils prédéfinis ou en créant des personnalités sur mesure.

| Champ | Type | Description |
|---|---|---|
| **Profil** | Sélecteur | Profil prédéfini ou personnalisé |
| **Nom** | Texte | Nom affiché du participant |
| **Emoji** | Sélecteur | Avatar emoji (40+ choix ou saisie libre) |
| **System prompt** | Textarea | Personnalité, expertise, style discursif |
| **Numéro d'intervention** | Nombre | Ordre de passage en mode séquentiel |
| **Émotions initiales** | Sliders (optionnel) | 6 axes émotionnels (0-100) |
| **Paramètres LLM** | Sliders | Ajustement individuel (température, tokens…) |

**Réorganisation** : glisser-déposer pour réordonner les participants.

### 5.4 Étape 4 — Mode & Options

#### Mode de discussion

8 modes disponibles, organisés en grille 2×2 par catégorie :

| Mode | Description | Posture attendue |
|---|---|---|
| **Débat** | Discussion contradictoire classique | Argumenter, défendre sa position, répondre aux contre-arguments |
| **Brainstorming** | Génération d'idées créatives | Proposer sans critiquer, enrichir les idées des autres |
| **Co-Construction** | Élaboration collaborative d'un livrable | Converger vers un output commun (document texte, markdown ou CSV) |
| **Guidé par l'utilisateur** | L'utilisateur dirige chaque tour | Les participants répondent librement aux orientations |
| **Socratique** | Questionnement philosophique | Poser des questions profondes, réfléchir |
| **Tutoriel** | Panel d'experts enseignants | Pédagogie, vulgarisation, exemples |
| **Revue critique** | Critique équilibrée | Identifier forces et axes d'amélioration |
| **Fiction collaborative** | Co-création narrative | Développer personnages, intrigue, univers |

#### Format de document (Co-Construction uniquement)

| Format | Description |
|---|---|
| **Aucun** | Pas de document collaboratif |
| **Texte brut** (.txt) | Document plain-text |
| **Markdown** (.md) | Document structuré avec formatage |
| **CSV** (.csv) | Tableau à colonnes (séparateur `;`) |

#### Options avancées

| Option | Type | Description |
|---|---|---|
| **Timeout intervention** | Secondes | Temps accordé à l'utilisateur pour intervenir |
| **Pool recherche web** | Nombre | Nombre total de recherches Tavily autorisées dans la discussion (0 = désactivé) |
| **Pool recherche Wikipedia** | Nombre | Nombre total de recherches Wikipedia autorisées (0 = désactivé) |

---

## 6. Discussion en direct (Arena)

### 6.1 Fil de discussion

Le fil de discussion affiche en temps réel :

- **Introduction** de l'IArbitre (message initial)
- **Messages** de chaque GladIAteur avec :
  - Badge de l'orateur (emoji + nom + rôle)
  - Contenu textuel (avec surlignage des noms de participants)
  - Réactions reçues (likes/dislikes)
  - Icône de pensée (si mode think activé)
  - Badge de recherche web/Wikipedia (si utilisée)
- **Messages de modération** (commentaires ou notifications de ban)
- **Messages utilisateur** (intervention directe)
- **Bulle de streaming** : pendant qu'un orateur parle, son texte apparaît progressivement

#### Indicateur de tour et mode

En haut du fil, un indicateur affiche :
- Le numéro de tour courant
- L'état (en cours, pause, détermination de l'ordre…)
- Le nombre de recherches web/Wikipedia effectuées
- Le **badge du mode de discussion** courant (si différent de « Débat »)

### 6.2 Contrôles de discussion

| Bouton | Action | Condition |
|---|---|---|
| **Pause** | Met la discussion en pause | Discussion en cours |
| **Reprendre** | Reprend depuis la pause | Discussion en pause |
| **Arrêt doux** | Termine le tour courant puis s'arrête | Discussion en cours |
| **Arrêt forcé** | Arrêt immédiat (confirmation requise) | Discussion en cours/pause |
| **Intervenir** | Signale que l'utilisateur veut prendre la parole | Discussion en cours |

### 6.3 Intervention utilisateur

Lorsque l'utilisateur demande à intervenir :

1. Le système attend la fin de l'orateur courant
2. Un panneau de saisie apparaît avec :
   - Une zone de texte
   - Un compteur de temps restant (timeout configurable)
   - Un bouton « Envoyer »
   - Un bouton « Passer » (annuler l'intervention)
3. Le message de l'utilisateur est traité comme celui d'un GladIAteur
4. Les autres participants réagissent à l'intervention

### 6.4 Sidebar émotionnelle

La sidebar droite affiche le profil émotionnel de chaque participant :

- **6 sliders colorés** (un par axe émotionnel, 0-100)
- **Emoji d'humeur** : visage correspondant à l'état dominant
- **Sparklines** : mini-graphiques montrant l'évolution au fil des tours
- **Résumé d'humeur** : texte décrivant l'état émotionnel (généré par LLM)
- **Directive cognitive** : acte de parole sélectionné et influence émotionnelle
- **Statut de ban** : si un participant est banni, durée restante affichée

L'utilisateur peut **ajuster manuellement** les émotions via les sliders (drag).

**Animation flash** : quand un axe franchit un seuil critique (≥85 ou ≤15), la carte du participant flashe brièvement.

### 6.5 Sidebar document

Visible uniquement en mode **Co-Construction** avec un format de document sélectionné :

- Affiche le document collaboratif en cours d'élaboration
- **Rendu markdown** pour les formats `.md` et `.txt`
- **Tableau** pour le format `.csv`
- Badge du format (`.md`, `.txt`, `.csv`)
- Badge « Dernière modification par : [nom] »
- Repliable/dépliable

Le document est mis à jour par chaque GladIAteur à son tour, qui contribue au livrable commun.

### 6.6 Sidebar carte des arguments

Visible uniquement si la **carte des arguments** est activée dans le setup :

- Affiche une **mindmap interactive** (arbre de thèses et d'arguments) générée automatiquement à partir des interventions
- **Toggle de vue** : bascule entre vue par thèse (thesis-centric) et vue par GladIAteur (speaker-centric)
- **Badge de compteurs** : nombre de thèses (T) et d'arguments (A) extraits
- **Légende** : ✅ Pour (soutien), ❌ Contre (opposition), 📊 Preuves
- Repliable/dépliable (texte vertical quand replié)
- Redimensionnable via séparateur draggable

La carte est mise à jour après chaque tour. Voir la [section 24](#24-carte-des-arguments-mindmap) pour les détails fonctionnels.

### 6.7 Indicateur d'activité

La barre de titre de l'Arena affiche en temps réel l'activité du moteur de discussion :

| Type d'activité | Indicateur |
|---|---|
| **Réflexion** | `[nom] réfléchit…` |
| **Écriture** | `[nom] écrit…` |
| **Réactions** | `Collecte des réactions…` |
| **Recherche web** | `[nom] recherche sur le web…` |
| **Recherche Wikipedia** | `[nom] recherche sur Wikipedia…` |
| **RAG** | `[nom] consulte les documents…` |
| **Émotions** | `Mise à jour des émotions…` |
| **Carte des arguments** | `Analyse des arguments…` |
| **Synthèse** | `Synthèse en cours…` |
| **Ordre des tours** | `Détermination de l'ordre…` |

Un point lumineux animé (pulse) accompagne le texte pour signaler visuellement l'activité en cours.

---

## 7. Modes de discussion

### Débat (par défaut)

Le mode classique : chaque participant défend sa position, répond aux contre-arguments et tente de convaincre. Le modérateur veille à l'équité et à la qualité argumentative.

### Brainstorming (Idéation)

Mode créatif : les participants génèrent des idées sans critique. L'accent est mis sur la quantité, la diversité et le rebond sur les propositions des autres. Le modérateur encourage l'exploration.

### Co-Construction

Mode collaboratif : les participants convergent vers un livrable commun. Chacun enrichit un document partagé (texte, markdown ou CSV). Le modérateur facilite l'intégration des contributions.

### Guidé par l'utilisateur

L'utilisateur oriente chaque tour en donnant obligatoirement son input au début. Chaque GladIAteur décide ensuite individuellement (via LLM) s'il souhaite répondre ou passer. Seuls les participants répondants prennent la parole. Si tous passent, le tour est automatiquement sauté. Idéal pour explorer un sujet de manière dirigée.

> **Note** : En mode Guidé par l'utilisateur, le mode de distribution des tours est automatiquement désactivé (chaque GladIAteur choisit librement de répondre).

### Socratique

Mode philosophique : l'accent est sur les questions plutôt que les réponses. Les participants pratiquent la maïeutique, explorent les présupposés et approfondissent la réflexion. L'IArbitre pose une nouvelle question d'approfondissement à chaque tour (à partir du tour 2), guidant progressivement vers les dimensions les plus profondes du sujet.

### Tutoriel

Mode pédagogique : les participants agissent comme un panel d'experts enseignants. L'accent est sur la clarté, les exemples et la vulgarisation. Le modérateur s'assure que les explications sont accessibles.

### Revue critique

Mode analytique : chaque participant évalue un sujet ou une proposition en identifiant les forces et les axes d'amélioration. Critique constructive et équilibrée.

### Fiction collaborative

Mode créatif narratif : les participants co-créent une histoire en relais. L'utilisateur écrit l'ouverture de l'histoire au premier tour, puis chaque GladIAteur développe la suite à tour de rôle (toujours en ordre séquentiel, pas de randomisation). Le modérateur maintient la cohérence de l'intrigue, des personnages et de l'univers narratif.

> **Note** : En mode Fiction collaborative, le mode de distribution des tours est automatiquement forcé en séquentiel. Les actes de parole sont remappés en actions narratives (ex. Challenge → introduction de conflit, SteelMan → développement de profondeur personnage, Anecdote → scène vivante).

---

## 8. Système émotionnel

Chaque participant possède un profil émotionnel sur 6 axes indépendants (0-100) :

| Axe | Description | Baseline |
|---|---|---|
| **Engagement** | Intérêt et implication dans la discussion | 50 |
| **Accord** | Alignement avec les positions des autres | 50 |
| **Confiance** | Assurance dans ses propres vues | 50 |
| **Frustration** | Irritation et tension | 10 |
| **Curiosité** | Ouverture intellectuelle et désir d'explorer | 50 |
| **Enthousiasme** | Énergie et dynamisme | 50 |

### Mécanismes de mise à jour

Les émotions évoluent automatiquement selon les événements :

| Événement | Effet |
|---|---|
| Likes reçus | Confiance ↑, Engagement ↑ |
| Dislikes reçus | Frustration ↑, Confiance ↓ |
| Contradiction (2+ dislikes) | Frustration ↑, Engagement ↑ |
| Soutien (2+ likes) | Enthousiasme ↑, Confiance ↑ |
| Ban reçu | Frustration ↑↑, Engagement ↓ |
| Stagnation | Engagement ↓, Curiosité ↓ |
| Décroissance naturelle | Tous les axes tendent vers 50 |

### Seuils d'alerte

- **Seuil haut** (≥85) : état extrême signalé (ex. frustration très élevée)
- **Seuil bas** (≤15) : état critique signalé (ex. engagement très faible)

### Influence sur le comportement (optionnel)

Si l'option **« Émotions influencent le comportement »** est activée dans les paramètres, les émotions modifient les directives cognitives :
- Un participant frustré sera plus agressif ou défensif
- Un participant enthousiaste sera plus volubile et engagé
- Un participant désengagé sera plus laconique

Si désactivée, les émotions sont toujours tracées et affichées mais n'influencent pas le contenu généré.

---

## 9. Personnalités cognitives

Chaque GladIAteur possède une personnalité cognitive extraite de son system prompt via des blocs XML `<dynamics>`. Ce système influence le comportement discursif à travers 5 couches :

### Couche 1 — Valeurs et déclencheurs

Définies dans le system prompt :
- **Valeurs** : convictions fondamentales du personnage
- **Déclencheurs** : ce qui le provoque ou le motive
- **Sous pression** : comportement en situation de stress
- **En confiance** : comportement quand rassuré

### Couche 2 — Relations

Le système évalue dynamiquement les relations entre participants :
- **Allié** : soutien fréquent (nombreux likes mutuels)
- **Rival** : opposition fréquente (nombreux dislikes)
- **Tendu** : relation ambivalente

### Couche 3 — Actes de parole

À chaque tour, un acte de parole est sélectionné aléatoirement avec pondération émotionnelle :

| Acte | Description |
|---|---|
| **Challenge** | Remettre en question un argument |
| **SteelMan** | Renforcer l'argument d'un autre avant de répondre |
| **Anecdote** | Illustrer par un exemple concret |
| **Question** | Poser une question exploratoire |
| **Provocation** | Pousser la discussion vers un terrain inconfortable |
| **Concession** | Reconnaître un point de l'adversaire |
| **Redirect** | Réorienter la discussion |
| **Humor** | Alléger l'atmosphère |
| **Appeal** | Faire appel aux valeurs partagées |
| **Synthesis** | Résumer et structurer le débat |

### Couche 4 — Anti-répétition

L'historique des messages précédents du participant est injecté pour éviter qu'il ne répète les mêmes arguments ou exemples.

### Couche 5 — Conscience situationnelle

Le système prend en compte :
- L'ambiance générale du groupe (frustration/engagement moyen)
- La position dans la discussion (début, milieu, fin)
- Le retour après un ban
- La proximité de la fin de discussion

---

## 10. Recherche de connaissances

### Wikipedia (gratuit, toujours disponible)

Les participants peuvent rechercher des informations sur Wikipedia pour enrichir leurs interventions.

| Caractéristique | Valeur |
|---|---|
| **Langues** | Français, Anglais, Chinois (fallback → anglais) |
| **Résultats** | 3 max par recherche, extraits de 500 caractères |
| **Pool** | Configurable par discussion (0 = désactivé) |
| **Décision** | Le LLM décide s'il a besoin d'une recherche |
| **Filtrage** | Score par mots-clés, pénalisation des pages de désambiguïsation |
| **Événement** | `WikiSearchPerformed` avec URL des articles |

### Recherche web Tavily (optionnelle, clé API requise)

Recherche web en temps réel via l'API Tavily.

| Caractéristique | Valeur |
|---|---|
| **API** | Tavily (`api.tavily.com`) |
| **Profondeur** | Basic (5 résultats max) |
| **Quota** | 1000 crédits/mois (tier gratuit), suivi automatique |
| **Pool** | Configurable par discussion (0 = désactivé) |
| **Configuration** | Clé API dans les paramètres |

### Recherche d'introduction

L'IArbitre peut effectuer une recherche web et/ou Wikipedia **avant** son introduction, pour ancrer la discussion dans des faits actuels.

---

## 11. Modération

L'IArbitre évalue chaque intervention des GladIAteurs et peut prendre trois actions :

| Action | Description |
|---|---|
| **Aucune** | L'intervention est conforme |
| **Commentaire** | L'IArbitre émet une remarque (rappel à l'ordre, suggestion) |
| **Ban** | Le GladIAteur est temporairement exclu (1-3 tours) |

### Critères de modération

Les critères varient selon le mode de discussion :
- **Débat** : respect des règles argumentatives, pertinence
- **Brainstorming** : pas de critique négative, contribution constructive
- **Co-Construction** : contribution au document commun, intégration
- **Socratique** : qualité des questions, profondeur réflexive
- **Tutorial** : clarté pédagogique, accessibilité
- **Revue critique** : équilibre forces/faiblesses, constructivité
- **Fiction collaborative** : cohérence narrative, contribution créative

### Système de ban

- **Durée** : 1 à 3 tours (décidée par l'IArbitre)
- **Notification** : message visible dans le fil et badge dans la sidebar émotionnelle
- **Impact** : le participant banni ne parle pas pendant la durée du ban
- **Levée** : automatique à expiration ; notifié par un message

---

## 12. Mode Think (réflexion)

Le mode think permet aux modèles Ollama compatibles de « réfléchir » avant de répondre. La réflexion interne est séparée du contenu visible.

### Fonctionnement

- **Activation** : heuristique probabiliste (pas à chaque tour)
  - Base : 20% de chance
  - +15% si frustration > 70
  - +10% si engagement > 70
  - +15% en fin de discussion
  - +10% si contredit
  - Maximum : 60%
  - Jamais au tour 1
- **Affichage** : icône cliquable sur le message pour voir/masquer la pensée interne
- **Compatibilité** : si le modèle ne supporte pas le mode think, il est ignoré silencieusement

### Intérêt

Quand le mode think est activé, le LLM peut structurer sa réflexion avant de formuler sa réponse. Cela peut améliorer la qualité argumentative, notamment dans les situations complexes ou émotionnellement chargées.

---

## 13. Réactions

Après chaque intervention d'un GladIAteur, les autres participants émettent une réaction :

| Réaction | Signification |
|---|---|
| **Like** 👍 | Accord, approbation, intérêt |
| **Dislike** 👎 | Désaccord, critique, rejet |

Chaque réaction est accompagnée d'une **justification courte** (1 phrase maximum) expliquant la raison du like ou dislike.

La **sémantique des réactions varie selon le mode** de discussion :
- **Débat** : like = « argument pertinent/convaincant », dislike = « argument faible/hors-sujet »
- **Brainstorming** : like = « idée originale/prometteuse », dislike = « idée déjà vue/hors-sujet »
- **Fiction** : like = « récit captivant/cohérent », dislike = « rupture de continuité/incohérence »
- etc.

Les réactions sont :
- Générées par le LLM pour chaque participant non-orateur
- Affichées sous le message correspondant avec la justification
- Utilisées pour mettre à jour les émotions
- Utilisées pour évaluer les relations entre participants
- Sauvegardées dans l'historique

---

## 14. Synthèse et résumé

### Synthèse automatique

À la fin de la discussion (dernier tour ou arrêt doux), l'IArbitre produit une synthèse :
- Résumé des positions de chaque participant
- Points de convergence et de divergence
- Conclusions ou questions ouvertes
- Adaptée au mode de discussion (ex. en co-construction, synthèse du livrable)

La synthèse est streamée en temps réel (token par token). Le budget de tokens pour la synthèse est doublé par rapport aux interventions normales (`SYNTHESIS_NUM_PREDICT = 4096`) pour éviter toute troncature sur les discussions riches. En cas de synthèse tronquée (absence de conclusion), un retry automatique est effectué avec un budget encore doublé.

### Page de résumé

Après la synthèse, l'utilisateur accède à la page de résumé avec :

| Élément | Description |
|---|---|
| **Onglet Synthèse** | Texte complet de la synthèse |
| **Onglet Discussion** | Fil complet en lecture seule |
| **Onglet Carte des arguments** | Mindmap interactive des thèses organisées par thèse (vue thesis-centric, si activée) |
| **Onglet Carte par GladIAteurs** | Mindmap des arguments organisés par orateur (vue speaker-centric, si activée) |
| **Badge de mode** | Mode de discussion affiché (si différent de « Débat ») |
| **Statistiques** | Nombre de tours, modèle utilisé, participants (composant `StatCard` partagé) |
| **Liste des participants** | Emojis et noms de tous les participants avec badge IArbitre |
| **Téléchargement document** | Export du document collaboratif (si co-construction) |
| **Téléchargement cartes (MD)** | Export des 2 cartes des arguments en markdown via sélection de dossier (si activées) |
| **Téléchargement cartes (SVG)** | Export des 2 cartes en SVG vectoriel complet via sélection de dossier (si activées) |
| **Nouvelle discussion** | Retour à la configuration |
| **Voir dans l'historique** | Navigation vers l'historique |

---

## 15. Historique des discussions

### Liste des discussions

La page d'historique affiche toutes les discussions sauvegardées avec :
- Sujet et date
- Participants (emojis)
- Nombre de tours
- Mode de discussion
- Indicateur de synthèse disponible
- Boutons de suppression (individuel et global)

### Détail d'une discussion

La vue détaillée permet de :
- Relire l'intégralité de la discussion en mode lecture seule
- Consulter la synthèse
- Consulter les deux cartes des arguments (par thèse et par GladIAteur, si activées lors de la discussion)
- Voir les réactions, les bans, les recherches
- Télécharger les cartes en markdown ou SVG (multi-fichier via sélection de dossier)
- Consulter le document collaboratif (si co-construction)

### Sauvegarde automatique

Chaque discussion terminée est automatiquement sauvegardée dans la base de données locale. La sauvegarde inclut :
- Tous les messages avec réactions
- La synthèse
- Les métadonnées (participants, mode, format, modèle)
- Le document collaboratif (si applicable)
- Les deux cartes des arguments en markdown (par thèse + par GladIAteur, si activées)

---

## 16. Paramètres de l'application

| Paramètre | Type | Description |
|---|---|---|
| **Nom d'utilisateur** | Texte | Nom affiché lors des interventions |
| **Langue** | Sélecteur | Langue de l'interface (FR/EN/ZH) |
| **Thème** | Toggle | Sombre ou clair |
| **URL Ollama** | URL | Adresse du serveur Ollama (défaut : `http://localhost:11434`) |
| **Modèle Ollama** | Sélecteur | Modèle LLM à utiliser (parmi ceux installés) |
| **Modèle d'embeddings** | Sélecteur | Modèle Ollama pour les embeddings RAG (optionnel, défaut : modèle LLM principal) |
| **Émotions influencent le comportement** | Toggle | Les émotions modifient-elles les directives ? |
| **Clé API Tavily** | Texte | Clé pour la recherche web (optionnel) |

### Pré-chargement du modèle

Un bouton permet de pré-charger le modèle Ollama sélectionné en mémoire, évitant les délais de chargement au premier tour de la discussion.

### Vérification de la connexion

Un bouton « Vérifier la connexion » teste la connectivité avec le serveur Ollama et affiche un indicateur vert/rouge.

---

## 17. Profils prédéfinis

### GladIAteurs (97 profils prédéfinis)

Les profils sont organisés par catégorie :

| Catégorie | Exemples |
|---|---|
| **Experts & Sciences** | Le Scientifique, Le Philosophe, Le Critique, L'Historien, Le Biologiste, Le Géographe, Le Mathématicien, Le Physicien, Le Chimiste, Le Climatologue, Le Géopoliticien |
| **IT & Tech** | Le Hackeur White Hat, Le Hackeur Red Hat, L'Expert IA, L'Informaticien, Le DEV Frontend, Le DEV Backend, Le DEV Architecte, La Data Analyste, Le DEV UX/UI, Le DevOps, Le RSSI, Le Marketing Digital |
| **Personnalités historiques** | Socrate, Nietzsche, Voltaire, Machiavel, Sun Tzu, Napoléon, Darwin, Einstein, Marx, Churchill, Platon, Aristote, Descartes, Kant, Simone de Beauvoir, Marie Curie, Tesla, Galilée, Newton, Léonard de Vinci |
| **Écrivains** | Victor Hugo, Shakespeare, Baudelaire, Dostoïevski, Oscar Wilde |
| **Mode** | Coco Chanel, Yves Saint Laurent, Karl Lagerfeld, Alexander McQueen, Vivienne Westwood |
| **Archétypes** | L'Avocat du Diable, Le Créatif, L'Optimiste, Le Pessimiste, Le Pragmatique, La Féministe, Le Masculiniste, Le Complotiste, L'Humoriste, Le Naïf, Le Psycho-rigide |
| **Figures** | Dieu, Satan, Bouddha, Krishna, La Singularité, L'Extra-terrestre, Le Chien, Le Chat |
| **Métiers** | Le Médecin, Le Psychologue, Le Comptable, Le Financier, Le Tradeur, Le Policier, Le Gendarme, Le Politicien, Le Fiscaliste, Le Dirigeant |
| **Sociaux** | Le Pilier de Bar, Le Mafieux, La Starlette de Télé-réalité, Le Startuper, La Fashion-Victim, Le Techno-Addict |
| **Politiques** | Le Mec de Gauche, Le Mec de Droite, L'Anarchiste, Le Fasciste, Le Mec d'Extrême Droite |

Chaque profil prédéfini inclut :
- Un system prompt détaillé avec personnalité, expertise et blocs `<dynamics>`
- Des émotions initiales adaptées à la personnalité
- Des traits OCEAN (Big Five)
- Des tics de langage et un style argumentatif propre
- Des réactions sous pression, en confiance et en désengagement

### IArbitres (10 profils prédéfinis)

| Profil | Style |
|---|---|
| **Le Modérateur Impartial** | Neutre, rigoureux, équitable — gardien de l'équité |
| **Le Provocateur** | Piquant, stimulant — provoque les positions pour les tester |
| **Le Maïeuticien** | Socratique — guide par les questions, jamais par les ordres |
| **Le Juge Strict** | Autoritaire, intransigeant — application stricte des règles |
| **L'Animateur TV** | Showman, dramatique — met en scène le débat comme un spectacle |
| **Le Thérapeute** | Empathique, doux — cherche les émotions derrière les arguments |
| **Le Roi Philosophe** | Sage, érudit — élève le débat vers les principes fondamentaux |
| **L'Agent du Chaos** | Imprévisible, absurde — chaos productif, questions déstabilisantes |
| **Le Directeur Scientifique** | Méthodique, exigeant — revue par les pairs, preuves obligatoires |
| **La Grand-mère** | Bienveillante, terre-à-terre — bon sens et sagesse populaire |

---

## 18. Profils personnalisés

L'utilisateur peut créer ses propres profils de GladIAteur et d'IArbitre :

### Création

1. Accéder aux paramètres ou à l'étape correspondante du setup
2. Cliquer sur « Nouveau profil »
3. Renseigner :
   - Nom
   - Personnalité (courte description)
   - System prompt (instructions détaillées)
   - Catégorie
   - Émotions initiales (optionnel)
4. Sauvegarder

### Gestion

- Les profils personnalisés peuvent être modifiés ou supprimés
- Les profils prédéfinis (builtin) ne peuvent pas être modifiés ni supprimés
- Les profils sont persistés dans la base de données SQLite locale

---

## 19. Internationalisation

L'application est disponible en 3 langues :

| Langue | Code | Statut |
|---|---|---|
| **Français** | `fr` | Langue par défaut et de fallback |
| **Anglais** | `en` | Traduction complète |
| **Chinois** | `zh` | Traduction complète |

### Portée de la traduction

- **Interface** : tous les textes, boutons, labels, messages d'erreur
- **Profils prédéfinis** : noms et system prompts localisés
- **Instructions internes** : les prompts envoyés au LLM sont dans la langue de discussion
- **Modération** : messages de ban/levée de ban trilingues

### Langue de l'interface vs langue de discussion

- La **langue de l'interface** (paramètres) détermine les textes affichés
- La **langue de discussion** (setup) détermine la langue des prompts et instructions envoyés au LLM

---

## 20. Thème et apparence

### Thème sombre (par défaut)

Fond sombre avec texte clair. Optimisé pour une utilisation prolongée et une bonne lisibilité.

### Thème clair

Fond clair avec texte sombre. Alternative classique.

### Personnalisation

Le thème est persisté dans les paramètres et s'applique immédiatement au basculement.

### Éléments visuels

- **Emojis d'avatar** : chaque participant a un emoji distinctif
- **Couleurs de rôle** : l'IArbitre et les GladIAteurs ont des couleurs de badge distinctes
- **Sidebar repliable** : optimise l'espace de lecture
- **Sidebar redimensionnable** : glisser pour ajuster la largeur

---

## 21. Export et téléchargement

### Depuis la page de résumé

| Action | Description |
|---|---|
| Télécharger le document | Exporte le document collaboratif (si co-construction) |
| Exporter les cartes en texte (.md) | Exporte les 2 cartes des arguments en markdown dans un dossier choisi (si activées) |
| Exporter les cartes en image (.svg) | Exporte les 2 cartes en SVG vectoriel complet dans un dossier choisi (si activées) |

### Depuis l'historique

Mêmes options de téléchargement disponibles depuis la vue détaillée d'une discussion (cartes MD + SVG multi-fichiers).

### Format d'export

Les exports de document et de fichier unique utilisent le sélecteur de fichier natif Windows (dialogue « Enregistrer sous »). Les exports multi-fichiers (cartes des arguments) utilisent un sélecteur de **dossier** puis écrivent les fichiers dans le dossier choisi.

L'export SVG de la carte des arguments est disponible depuis **tous les onglets** (synthèse, discussion ou carte). Le SVG exporté contient l'**intégralité** de la carte (toutes les thèses et arguments), indépendamment de la portion visible à l'écran (zoom/pan). Deux fichiers SVG sont générés : « Carte des arguments » (vue par thèse) et « Carte des arguments par gladiateurs » (vue par orateur).

---

## 22. RAG — Enrichissement par documents

**RAG (Retrieval-Augmented Generation)** permet aux participants d'accéder à des connaissances extraites de documents que vous importez dans l'application.

### Fonctionnement

1. **Import de documents** : vous importez vos documents (PDF, TXT, MD, CSV, Word) via les paramètres
2. **Traitement automatique** : l'application découpe les documents en morceaux (chunks), génère des embeddings et construit un index de recherche
3. **Recherche contextuelle** : pendant la discussion, les participants peuvent rechercher des informations pertinentes dans votre base documentaire
4. **Injection dans le contexte** : les passages les plus pertinents sont automatiquement injectés dans le contexte de l'IA avant sa réponse

### Formats supportés

| Format | Extension | Notes |
|---|---|---|
| **PDF** | `.pdf` | Extraction de texte brut (pas d'images) |
| **Texte** | `.txt` | UTF-8 |
| **Markdown** | `.md` | UTF-8 |
| **CSV** | `.csv` | Converti en tableau formaté |
| **Word** | `.docx` | Extraction basique (pas de formatage complexe) |

### Utilisation

#### Import de documents

1. Ouvrir les **Paramètres**
2. Section **RAG** (en bas de page)
3. Cliquer sur « **Importer un document** »
4. Sélectionner un fichier (max 10 MB)
5. Attendre le traitement (parsing → chunking → embeddings)

La liste des documents importés s'affiche avec :
- Nom du fichier
- Format détecté
- Nombre de morceaux (chunks)
- Nombre de caractères
- Bouton de suppression

#### Utilisation pendant la discussion

Une fois des documents importés, les participants peuvent automatiquement rechercher des informations pertinentes. Quand un participant utilise le RAG, un événement `RAG Context Injecté` apparaît dans le fil avec :
- Le nom du participant
- Les passages utilisés (nom de fichier, aperçu, score de pertinence)

#### Gestion des documents

- **Supprimer un document** : clic sur l'icône poubelle à côté du document
- **Vider le store RAG** : bouton « Tout supprimer » en bas de la liste

### Configuration

| Paramètre | Valeur | Description |
|---|---|---|
| **Modèle d'embeddings** | (settings) | Modèle Ollama utilisé pour générer les embeddings. Par défaut, utilise le modèle LLM principal. |
| **Taille max fichier** | 10 MB | Limite de taille pour l'import |
| **Taille des chunks** | ~800 caractères | Taille cible d'un morceau de texte |
| **Top résultats** | 5 | Nombre de passages retournés par recherche |

### Points importants

- **Mémoire** : les documents restent en mémoire pendant toute la session. À la fermeture de l'application, il faut les réimporter.
- **Cohérence** : si vous changez le modèle d'embeddings, vous devez vider le store RAG et réimporter tous les documents.
- **Performance** : le premier import peut prendre du temps (chargement du modèle). Les imports suivants sont plus rapides.
- **Pertinence** : la qualité des résultats dépend du modèle d'embeddings utilisé et de la qualité des documents.

---

## 23. Surlignage des modifications (document collaboratif)

En mode **Co-Construction**, chaque participant contribue à un document partagé. Le système de diff surligne visuellement les modifications apportées par chaque participant.

### Fonctionnement

Après chaque contribution :
1. Le nouveau contenu du document est comparé avec la version précédente
2. Les changements sont identifiés selon le format du document
3. Les modifications sont surlignées dans la **Sidebar Document**

### Stratégies de diff par format

| Format | Niveau de diff | Rendu |
|---|---|---|
| **Texte (.txt)** | Mot par mot | Les mots ajoutés apparaissent en surbrillance jaune |
| **Markdown (.md)** | Ligne par ligne | Les lignes ajoutées/modifiées apparaissent avec un fond coloré |
| **CSV (.csv)** | Cellule par cellule | Les cellules modifiées sont surlignées dans le tableau |

### Comportement

- **Durée du surlignage** : permanent (visible jusqu'à la prochaine modification)
- **Indication visuelle** : badge « Dernière modification par : [nom] » au-dessus du document
- **Historique** : pas de diff cumulatif — seule la dernière modification est surlignée

### Cas d'usage

- **Suivi des contributions** : voir rapidement qui a ajouté quoi
- **Validation collective** : identifier les sections à discuter
- **Cohérence narrative** : repérer les incohérences entre contributions (fiction collaborative)

---

## 24. Carte des arguments (Mindmap)

La **carte des arguments** est une fonctionnalité optionnelle qui génère automatiquement une visualisation en arbre (mindmap) des thèses et arguments avancés pendant la discussion.

### Activation

La carte des arguments est activée via un toggle dans l'étape 1 du setup (« Paramétrage général »). Quand elle est activée, le moteur extrait les thèses et arguments de chaque intervention à partir du tour 2.

### Structure de la carte

La carte existe en **deux vues** complémentaires :

#### Vue thesis-centric (par thèse)

Chaque thèse est une branche principale, les arguments sont imbriqués récursivement :

```
# [Sujet de discussion]
## [Thèse 1] ([Orateur])
- ✅ [Orateur A]: [Argument de soutien]
  - ❌ [Orateur B]: [Contre-argument]
    - ✅ [Orateur A]: [Réfutation]
      - 📊 [Orateur A]: [Preuve]
## [Thèse 2] ([Orateur])
- ✅ [Orateur B]: [Support]
```

#### Vue speaker-centric (par GladIAteur)

Les arguments sont regroupés par orateur, avec les sous-arguments imbriqués :

```
# [Sujet de discussion]
## [Orateur 1]
### [Thèse 1]
- ✅ [Argument de soutien]
  - ❌ [Sous-contre-argument]
### → [Thèse d'un autre orateur]
- ❌ [Contre-argument]
## [Orateur 2]
### [Thèse 2]
...
```

### Arguments récursifs

Les arguments peuvent avoir des **sous-arguments** imbriqués (contre-arguments, réfutations, preuves). La profondeur est limitée à 4 niveaux (`ARGMAP_MAX_ARGUMENT_DEPTH`). Si le LLM cible un argument existant via `targets_argument`, le nouveau noeud est attaché en enfant de cet argument. Si la cible n'est pas trouvée ou que la profondeur max est atteinte, l'argument est rattaché directement à la thèse (fallback plat).

### Types d'arguments

| Type | Icône | Description |
|---|---|---|
| **Support** | ✅ | Argument en faveur d'une thèse ou d'un argument parent |
| **Counter** | ❌ | Contre-argument s'opposant à une thèse ou un argument parent |
| **Evidence** | 📊 | Preuve, donnée factuelle ou exemple concret |

### Mécanisme d'extraction

1. Après chaque tour (à partir du tour 2), le LLM analyse les interventions
2. Il identifie les nouvelles thèses et arguments en les rattachant aux thèses existantes ou en en créant de nouvelles
3. **Ciblage d'argument** : le LLM peut spécifier `targets_argument` pour créer un sous-argument (contre-argument à un argument existant, réfutation, etc.)
4. La carte est fusionnée avec la version précédente (les thèses existantes sont enrichies, pas dupliquées)
5. Le résultat est converti en deux markdowns hiérarchiques : vue par thèse et vue par orateur
6. Les deux markdowns sont émis et persistés séparément

### Affichage

- **Arena** : sidebar droite avec rendu interactif via [markmap](https://markmap.js.org/) et toggle thesis/speaker view
- **Résumé / Historique** : deux onglets dédiés (« Carte des arguments » et « Carte par GladIAteurs ») avec la même visualisation
- **Export** : téléchargement multi-fichier en markdown (.md) ou en SVG (.svg) via sélection de dossier

### Validation des labels

Le système vérifie la qualité des labels extraits par le LLM :
- **Longueur minimale** : les labels de thèse doivent contenir au moins 8 caractères (`ARGMAP_MIN_THESIS_LABEL_CHARS`) — filtre les indices numériques et les labels trop courts
- **Anti-numérique** : les labels purement numériques (ex. « 4 », « 9 ») sont rejetés — le prompt interdit explicitement l'utilisation de numéros ou d'indices pour référencer les thèses
- **Résolution automatique** : si le LLM retourne un index numérique (ex. « 3 »), le système tente de le résoudre vers la thèse existante correspondante
- Les labels sont tronqués proprement (aux limites de mots) si nécessaire

### Contraintes

- Les labels sont courts (thèses ≤ 200 caractères, arguments ≤ 400 caractères)
- Maximum 20 thèses et 100 arguments
- Profondeur max de sous-arguments : 4 niveaux (`ARGMAP_MAX_ARGUMENT_DEPTH`)
- Les labels sont en langage naturel dans la langue de discussion (pas d'identifiants techniques ni de numéros)
- Les deux cartes (par thèse + par orateur) sont persistées dans l'historique sous forme markdown

---

## 25. Glossaire

| Terme | Définition |
|---|---|
| **AIrena** | Nom de l'application (contraction de « AI » et « Arena ») |
| **GladIAteur** | Participant IA dans une discussion (contraction de « Gladiateur » et « IA ») |
| **IArbitre** | Modérateur IA d'une discussion (contraction de « IA » et « Arbitre ») |
| **Ollama** | Serveur local pour exécuter des modèles LLM |
| **LLM** | Large Language Model — modèle de langage à grande échelle |
| **Tour** | Cycle complet où chaque participant non-banni prend la parole |
| **Ban** | Exclusion temporaire d'un participant par le modérateur |
| **Synthèse** | Résumé structuré produit par l'IArbitre en fin de discussion |
| **Mode think** | Capacité du LLM à « réfléchir » en interne avant de répondre |
| **Directive** | Instruction comportementale générée pour chaque tour (acte de parole) |
| **Acte de parole** | Stratégie discursive choisie (Challenge, SteelMan, Anecdote…) |
| **Justification** | Courte explication (1 phrase) accompagnant chaque réaction like/dislike |
| **Pool** | Quota de recherches (web ou Wikipedia) disponible pour une discussion |
| **Co-Construction** | Mode où les participants élaborent un document collaboratif |
| **Vote Borda** | Méthode de vote par classement utilisée en mode démocratique |
| **Tavily** | Service de recherche web par API (optionnel) |
| **System prompt** | Instructions de personnalité et de comportement envoyées au LLM |
| **Token** | Unité de texte minimale traitée par un LLM |
| **Streaming** | Affichage progressif du texte token par token |
| **NDJSON** | Newline-Delimited JSON — format de streaming utilisé par Ollama |
| **RAG** | Retrieval-Augmented Generation — enrichissement des réponses par recherche documentaire |
| **Embedding** | Représentation vectorielle d'un texte pour la recherche sémantique |
| **Chunk** | Morceau de texte découpé depuis un document (pour le RAG) |
| **BM25** | Algorithme de recherche lexicale (tf-idf amélioré) |
| **Similarité cosine** | Mesure de proximité sémantique entre deux embeddings |
| **Diff** | Différence calculée entre deux versions d'un texte |
| **Carte des arguments** | Visualisation arborescente (mindmap) des thèses et arguments d'une discussion |
| **Mindmap** | Représentation graphique en arbre, utilisée pour la carte des arguments |
| **Markmap** | Bibliothèque JavaScript de rendu de mindmaps depuis du markdown |

---

## 26. Changelog

### v1.14 (2026-02-27) — Arguments récursifs & Double vue carte

**Nouvelles fonctionnalités** :
- **Arguments récursifs** : les arguments peuvent avoir des sous-arguments imbriqués (contre-arguments, réfutations, preuves) jusqu'à 4 niveaux de profondeur
- **Double vue carte des arguments** : vue par thèse (thesis-centric) et vue par GladIAteur (speaker-centric), dans la sidebar Arena, le Résumé et l'Historique
- **Toggle de vue** dans la sidebar carte (Arena) pour basculer entre les deux vues
- **Onglets séparés** dans le Résumé et l'Historique : « Carte des arguments » + « Carte par GladIAteurs »
- **Export multi-fichier** : les exports MD et SVG génèrent 2 fichiers (un par vue) dans un dossier choisi par l'utilisateur

**Améliorations** :
- `targets_argument` dans l'extraction LLM — permet au LLM de cibler un argument existant pour créer un sous-argument
- Persistance des deux markdowns en base de données (nouvelle colonne `argument_map_md_by_speaker`)
- Composant `StatCard` partagé entre SummaryPage et HistoryDetailPage (DRY)
- Unicité des recherches + anti-répétition + détection du mode think

### v1.13 (2026-02-24) — Synthèse élargie & Optimisation contexte

**Améliorations** :
- **Synthèse élargie** : `SYNTHESIS_NUM_PREDICT = 4096` (2× le budget par défaut) pour des synthèses plus complètes ; retry automatique avec budget doublé en cas de troncature
- **Optimisation du contexte** : système de waterfall adaptatif pour la gestion du budget de tokens dans les prompts
- **Export SVG complet** : l'export de la carte des arguments capture l'intégralité du contenu (pas seulement la partie visible) via `getBBox()` sur l'élément SVG principal

### v1.12 (2026-02-22) — Optimisation contexte & Waterfall

**Améliorations** :
- **Budget de contexte adaptatif** : système de waterfall qui alloue dynamiquement les tokens entre les sections du prompt (mémoire, directives, RAG, recherche) en fonction du `num_ctx` disponible
- **Constantes Token Budget** : 16 nouvelles constantes `BUDGET_*` (floors + ceilings) pour un contrôle fin de l'allocation par section

### v1.11.2 (2026-02-19) — Fix synthèse

**Corrections** :
- Fix de la synthèse tronquée : augmentation du `num_predict` pour la synthèse

### v1.11.1 (2026-02-18) — Fix mineur

**Corrections** :
- Corrections mineures de stabilité

### v1.11 (2026-02-17) — Illustration

**Nouvelles fonctionnalités** :
- Illustration de la page d'accueil

### v1.10 (2026-02-14) — Carte des arguments & Améliorations UX

**Nouvelles fonctionnalités** :
- **Carte des arguments (Mindmap)** : visualisation interactive en arbre des thèses et arguments extraits automatiquement de la discussion
  - Extraction LLM après chaque tour (fusion incrémentale)
  - Sidebar dédiée dans l'Arena avec compteurs (thèses/arguments) et légende
  - Onglet dans le Résumé et l'Historique
  - Export en markdown (.md) et SVG (.svg)
  - Persistance en base de données
- **Indicateur d'activité** : affichage en temps réel de l'état du moteur (réflexion, écriture, recherche, émotions, synthèse…) dans la barre de titre
- **Titres des étapes Setup** : chaque étape affiche un titre descriptif (ex. « Étape 1 / 4 — Paramétrage général de la discussion »)

**Améliorations** :
- Centralisation de toutes les constantes et magic numbers dans `constants.rs`
- Labels de la carte en langage naturel dans la langue de discussion (pas d'identifiants techniques)
- Réorganisation du setup : le toggle « Carte des arguments » et le « Nombre de tours » sont dans l'étape 1

### v1.9.1 (2026-02-13) — RAG v2

**Améliorations** :
- Optimisation du parsing PDF (meilleure gestion des erreurs et des fichiers corrompus)
- Amélioration de la pertinence des résultats de recherche RAG
- Messages d'erreur plus clairs lors de l'import de documents

### v1.9 (2026-02-13) — RAG (Enrichissement par documents)

**Nouvelles fonctionnalités** :
- **Système RAG complet** : importez vos documents (PDF, TXT, MD, CSV, DOCX) pour enrichir les discussions
- Interface de gestion des documents RAG dans les Paramètres
- Recherche hybride (lexicale BM25 + sémantique par embeddings)
- Événement « RAG Context Injecté » dans le fil de discussion
- Configuration du modèle d'embeddings (par défaut : modèle LLM principal)
- Support de documents jusqu'à 10 MB
- Découpage intelligent en chunks avec overlap

**Expérience utilisateur** :
- Indicateurs visuels des passages RAG utilisés (nom fichier, aperçu, score)
- Gestion facile : import, suppression unitaire ou globale
- Feedback temps réel pendant le traitement des documents

### v1.8.1 (2026-02-13) — Surlignage des modifications

**Nouvelles fonctionnalités** :
- **Surlignage visuel des modifications** dans le document collaboratif (mode Co-Construction)
- Diff intelligent par format :
  - Texte : surlignage des mots ajoutés
  - Markdown : surlignage des lignes modifiées
  - CSV : surlignage des cellules modifiées
- Badge « Dernière modification par : [nom] » au-dessus du document
- Indication visuelle immédiate des contributions de chaque participant

**Améliorations** :
- Performance améliorée pour le rendu des gros documents markdown
- Scroll automatique dans la sidebar document

### v1.8 (2026-02-13) — Profils enrichis

**Nouvelles fonctionnalités** :
- 10 nouveaux profils GladIAteur (Sciences avancées, Arts, Sports)
- 2 nouveaux profils IArbitre

**Améliorations** :
- Interface de sélection de profils avec filtrage par catégorie
- Support des émotions initiales personnalisées par profil

### v1.7 (2026-02-13) — Documentation

**Documentation** :
- Documentation technique complète (architecture, patterns, API)
- Documentation fonctionnelle complète (guide utilisateur, modes, fonctionnalités)

### v1.6 (2026-02-13) — Types de discussions

**Nouvelles fonctionnalités** :
- 8 modes de discussion : Débat, Brainstorming, Co-Construction, Guidé par l'utilisateur, Socratique, Tutoriel, Revue critique, Fiction collaborative
- Chaque mode définit des instructions spécifiques pour l'introduction, les interventions, la synthèse et la modération
- Mode **Guidé par l'utilisateur** : input obligatoire + chaque GladIAteur choisit de répondre ou passer
- Mode **Fiction collaborative** : ouverture par l'utilisateur au tour 1, relais séquentiel, actes de parole narratifs
- Mode **Socratique** : l'IArbitre pose une nouvelle question d'approfondissement chaque tour
- Mode **Co-Construction** avec document collaboratif partagé (texte, markdown, CSV) et sidebar dédiée
- Réactions enrichies : chaque like/dislike est accompagné d'une justification courte
- Sémantique des réactions adaptée au mode (débat ≠ brainstorming ≠ fiction)
- Rendu markdown enrichi : titres, italique, blocs de code, tableaux, règles horizontales
- Support du rendu LaTeX (formules mathématiques) via KaTeX
- Téléchargement de documents via dialogue natif Windows « Enregistrer sous »
- Badge de mode affiché dans l'Arena et le résumé
- Navigation sécurisée : bouton Setup désactivé pendant la discussion active
- 50+ nouvelles traductions (FR/EN/ZH) pour les modes, formats et documents

### v1.5 — Wikipedia

**Nouvelles fonctionnalités** :
- Intégration de la recherche Wikipedia pour enrichir les interventions
- Support multilingue (français, anglais, chinois avec fallback)
- Filtrage intelligent des résultats (pénalisation des pages de désambiguïsation)
- Pool de quotas configurable par discussion
- Badges de recherche sur les messages et indicateur de compteur global
- URLs des articles Wikipedia affichées

### v1.4 — Personnalités cognitives

**Nouvelles fonctionnalités** :
- Système de directives cognitives à 5 couches
- 10 actes de parole avec pondération émotionnelle dynamique
- Extraction automatique des traits de personnalité depuis les system prompts XML
- Anti-répétition basée sur l'historique personnel
- Conscience situationnelle (ambiance du groupe, bans, position dans la discussion)

### v1.3 — Émotions

**Nouvelles fonctionnalités** :
- Système émotionnel à 6 axes (engagement, accord, confiance, frustration, curiosité, enthousiasme)
- Émotions initiales personnalisables par participant
- Mise à jour automatique basée sur les réactions et événements
- Sidebar émotionnelle avec sliders interactifs et sparklines
- Seuils d'alerte avec animation flash
- Option pour activer/désactiver l'influence des émotions sur le comportement

### v1.2 — Internet

**Nouvelles fonctionnalités** :
- Recherche web Tavily (API optionnelle)
- Suivi automatique des quotas mensuels
- Décision de recherche par le LLM
- Configuration de la clé API dans les paramètres

### v1.1 — Distribution des tours & Build

**Nouvelles fonctionnalités** :
- Mode de distribution démocratique (vote Borda masqué)
- Mode de distribution autoritaire (IArbitre décide seul)
- Sauvegarde des profils personnalisés
- Build exécutable Windows (installateurs MSI/NSIS)

**Corrections** :
- Fix affichage des émotions

### v1.0 — Release initiale

**Fonctionnalités** :
- Architecture complète Tauri v2 + React 19
- Moteur de discussion avec streaming temps réel
- Mode think (réflexion interne des IA)
- Système de réactions (like/dislike)
- Modération et système de bans
- Mémoire contextuelle et carte positionnelle
- Sauvegarde automatique et historique des discussions
- Internationalisation français/anglais/chinois
- Thème sombre et clair
- 97 profils GladIAteur et 10 profils IArbitre prédéfinis

### v0.2 — Prototype

- Première version fonctionnelle du moteur de discussion

### v0.1 — Init

- Structure initiale du projet
