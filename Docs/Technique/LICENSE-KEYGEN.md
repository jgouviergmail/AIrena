# AIrena License Key Generator

> **Version** : 1.0
> **Dernière mise à jour** : 2026-02-15
> **Script** : `tools/keygen.mjs`
> **Prérequis** : Node.js 18+

---

## Table des matières

1. [Vue d'ensemble](#1-vue-densemble)
2. [Prérequis](#2-prérequis)
3. [Première utilisation](#3-première-utilisation)
4. [Commandes](#4-commandes)
   - 4.1 [init — Initialiser les clés cryptographiques](#41-init--initialiser-les-clés-cryptographiques)
   - 4.2 [generate — Générer une clé de licence](#42-generate--générer-une-clé-de-licence)
   - 4.3 [inspect — Inspecter une clé existante](#43-inspect--inspecter-une-clé-existante)
5. [Paramètres de licence](#5-paramètres-de-licence)
6. [Architecture cryptographique](#6-architecture-cryptographique)
7. [Fichiers et sécurité](#7-fichiers-et-sécurité)
8. [Intégration avec l'application](#8-intégration-avec-lapplication)
9. [Exemples courants](#9-exemples-courants)
10. [Dépannage](#10-dépannage)

---

## 1. Vue d'ensemble

Le script `tools/keygen.mjs` est un outil standalone en ligne de commande permettant de générer et inspecter les clés de licence AIrena. Il utilise exclusivement les APIs cryptographiques natives de Node.js (`node:crypto`) et ne nécessite aucune dépendance npm.

Chaque clé de licence encode :
- L'adresse email du titulaire
- Un horodatage de création
- Une durée de validité (en heures)
- Un nonce aléatoire anti-rejeu

La clé est signée (Ed25519), chiffrée (AES-256-GCM), puis encodée en Base64 avec le préfixe `AIRENA-`.

---

## 2. Prérequis

| Composant | Version minimale | Vérification |
|-----------|-----------------|--------------|
| Node.js   | 18.0+           | `node --version` |

Aucune installation de dépendances n'est nécessaire. Le script utilise uniquement les modules natifs de Node.js :
- `node:crypto` (Ed25519, AES-256-GCM, SHA-256)
- `node:fs` (lecture/écriture du fichier de clés)
- `node:path`, `node:url` (résolution de chemins)

---

## 3. Première utilisation

Lors de la **toute première exécution** (quelle que soit la commande), le script détecte l'absence du fichier `tools/.keys.json` et génère automatiquement :

1. Une **paire de clés Ed25519** (signature asymétrique)
2. Une **clé AES-256** (chiffrement symétrique)

```bash
node tools/keygen.mjs init
```

**Sortie attendue :**

```
Generating new keypair + AES key...

Keys saved to tools/.keys.json

=== Paste these into src-tauri/src/constants.rs ===

pub const LICENSE_ED25519_PUBLIC_KEY_HEX: &str = "758f08...fd4";
pub const LICENSE_AES_KEY_HEX: &str = "ab0bed...bbe";
```

**Action requise** : Copier les deux constantes affichées dans `src-tauri/src/constants.rs` (section `// -- License --`), puis recompiler l'application Rust.

> **IMPORTANT** : Cette opération ne doit être effectuée qu'**une seule fois**. Toute régénération des clés invalide l'ensemble des licences existantes.

---

## 4. Commandes

### 4.1 `init` — Initialiser les clés cryptographiques

```bash
node tools/keygen.mjs init
```

Génère la paire Ed25519 + clé AES si `tools/.keys.json` n'existe pas. Si le fichier existe déjà, cette commande est un no-op (les clés existantes sont conservées).

**Quand l'utiliser** : Uniquement lors de la configuration initiale du projet, ou si `tools/.keys.json` a été supprimé intentionnellement pour révoquer toutes les clés existantes.

---

### 4.2 `generate` — Générer une clé de licence

```bash
node tools/keygen.mjs generate --email <email> --duration <heures>
```

| Paramètre    | Obligatoire | Description |
|-------------|-------------|-------------|
| `--email`    | Oui         | Adresse email du titulaire de la licence |
| `--duration` | Oui         | Durée de validité en **heures** (entier positif) |

**Exemple :**

```bash
node tools/keygen.mjs generate --email utilisateur@exemple.com --duration 720
```

**Sortie :**

```
=== Generated License Key ===

AIRENA-cHR5c-GxhbG-9hZC1-0ZXN0-LWtle-Q1234-ABCDE-...

Email:            utilisateur@exemple.com
Duration:         720h (30.0 days)
Max discussions:  1500
Expires at:       2026-03-17T14:30:00.000Z
SHA-256:          a1b2c3d4e5f6...
```

La clé affichée est prête à être copiée et saisie dans l'application (Paramètres > Licence).

#### Durées courantes

| Usage | Durée | Commande |
|-------|-------|----------|
| Test rapide (1 jour) | 24h | `--duration 24` |
| Essai (1 semaine) | 168h | `--duration 168` |
| Licence mensuelle | 720h | `--duration 720` |
| Licence trimestrielle | 2160h | `--duration 2160` |
| Licence annuelle | 8760h | `--duration 8760` |

---

### 4.3 `inspect` — Inspecter une clé existante

```bash
node tools/keygen.mjs inspect <CLÉ>
```

Déchiffre, vérifie la signature et affiche les métadonnées d'une clé de licence.

**Exemple :**

```bash
node tools/keygen.mjs inspect "AIRENA-cHR5c-GxhbG-9hZC1-0ZXN0-LWtle-Q1234-ABCDE-..."
```

> **Note** : Encadrer la clé avec des guillemets pour éviter que le shell n'interprète les tirets comme des options.

**Sortie :**

```
=== License Key Inspection ===

Signature:        VALID
Version:          1
Email:            utilisateur@exemple.com
Created:          2026-02-15T14:30:00.000Z
Duration:         720h (30.0 days)
Expires:          2026-03-17T14:30:00.000Z
Max discussions:  1500
Nonce:            a1b2c3d4e5f6g7h8
Status:           ACTIVE
SHA-256:          a1b2c3d4e5f6...
```

| Champ | Description |
|-------|-------------|
| **Signature** | `VALID` si la signature Ed25519 est vérifiée, `INVALID` sinon |
| **Version** | Version du format de payload (actuellement `1`) |
| **Email** | Adresse email encodée dans la clé |
| **Created** | Date/heure de génération (UTC) |
| **Duration** | Durée de validité totale |
| **Expires** | Date/heure d'expiration (UTC) |
| **Max discussions** | Nombre maximum de discussions autorisées |
| **Nonce** | Identifiant aléatoire anti-rejeu (16 caractères hex) |
| **Status** | `ACTIVE` si non expirée, `EXPIRED` si la date d'expiration est passée |
| **SHA-256** | Hash de la clé brute (utilisé comme identifiant interne pour le suivi du compteur) |

---

## 5. Paramètres de licence

### Quota de discussions

Le nombre maximum de discussions autorisées par licence est calculé selon la formule :

```
max_discussions = ceil(50 * duration_hours / 24)
```

Soit **50 discussions par tranche de 24 heures** de validité (arrondi au supérieur).

| Durée | Max discussions |
|-------|----------------|
| 24h (1 jour) | 50 |
| 168h (1 semaine) | 350 |
| 720h (30 jours) | 1 500 |
| 2160h (90 jours) | 4 500 |
| 8760h (365 jours) | 18 250 |

La constante `LICENSE_DISCUSSIONS_PER_DAY` dans `src-tauri/src/constants.rs` permet de modifier ce ratio (défaut : `50`).

### Protection anti-horloge

L'application détecte les reculs d'horloge système (manipulation manuelle ou changement de fuseau). Une tolérance de **2 heures** (`LICENSE_CLOCK_TOLERANCE_SECS = 7200`) est appliquée pour couvrir les transitions d'heure d'été/hiver.

Si l'horloge système recule de plus de 2 heures par rapport au dernier contrôle, la licence est considérée invalide (`Clock moved backward`).

### Gestion du compteur

Le compteur de discussions est lié au **SHA-256 de la clé brute** :
- Si l'utilisateur saisit une **nouvelle clé** (hash différent), le compteur repart automatiquement à 1.
- Si la même clé est conservée, le compteur s'incrémente normalement.
- Le compteur est incrémenté **après** la validation Ollama et **avant** le lancement du moteur de discussion, garantissant qu'un crédit n'est consommé que si toutes les vérifications pré-vol réussissent.

---

## 6. Architecture cryptographique

### Flux de génération

```
                      keygen.mjs (generate)
                      ─────────────────────

  ┌─────────────────────────────────────────────────────┐
  │ 1. Payload JSON                                     │
  │    {"v":1, "e":"email", "t":unix_ts,                │
  │     "d":hours, "n":"random_hex"}                    │
  └────────────────────┬────────────────────────────────┘
                       │
                       ▼
  ┌─────────────────────────────────────────────────────┐
  │ 2. Ed25519 sign(private_key, payload_bytes)         │
  │    → signature (64 bytes)                           │
  └────────────────────┬────────────────────────────────┘
                       │
                       ▼
  ┌─────────────────────────────────────────────────────┐
  │ 3. plaintext = payload_bytes + signature            │
  └────────────────────┬────────────────────────────────┘
                       │
                       ▼
  ┌─────────────────────────────────────────────────────┐
  │ 4. AES-256-GCM encrypt(aes_key, nonce, plaintext)   │
  │    → ciphertext + auth_tag (16 bytes)               │
  └────────────────────┬────────────────────────────────┘
                       │
                       ▼
  ┌─────────────────────────────────────────────────────┐
  │ 5. blob = nonce(12) + ciphertext + auth_tag(16)     │
  └────────────────────┬────────────────────────────────┘
                       │
                       ▼
  ┌─────────────────────────────────────────────────────┐
  │ 6. Base64 Standard → segments de 5 chars + "-"      │
  │    → "AIRENA-cHR5c-GxhbG-9hZC1-..."                │
  └─────────────────────────────────────────────────────┘
```

### Flux de vérification (application Rust)

Le flux inverse est exécuté dans `src-tauri/src/license.rs` :

```
  AIRENA-cHR5c-GxhbG-...
    │
    ▼ Strip "AIRENA-" + supprimer tirets
    │
    ▼ Base64 Standard decode → blob
    │
    ▼ blob[0..12] = nonce, blob[12..] = ciphertext+tag
    │
    ▼ AES-256-GCM decrypt(aes_key, nonce, ciphertext+tag) → plaintext
    │
    ▼ plaintext[0..len-64] = payload, plaintext[len-64..] = signature
    │
    ▼ Ed25519 verify(public_key, payload, signature)
    │
    ▼ JSON parse → LicensePayload {v, e, t, d, n}
    │
    ▼ Vérifications : version, expiration, horloge, quota
```

### Algorithmes utilisés

| Algorithme | Usage | Taille de clé |
|-----------|-------|---------------|
| **Ed25519** | Signature asymétrique (intégrité + authenticité) | 256 bits |
| **AES-256-GCM** | Chiffrement authentifié (confidentialité + intégrité) | 256 bits |
| **SHA-256** | Hash de la clé brute (identifiant compteur) | 256 bits |
| **Base64 Standard** | Encodage binaire → texte | N/A |

### Pourquoi Base64 Standard et non URL-safe ?

L'alphabet Base64 standard (`A-Za-z0-9+/=`) ne contient **pas** le caractère `-`. Les tirets de segmentation (`AIRENA-XXXXX-XXXXX-...`) sont donc non-ambigus et peuvent être supprimés sans risque lors du décodage.

---

## 7. Fichiers et sécurité

### Fichiers impliqués

| Fichier | Contenu | Git |
|---------|---------|-----|
| `tools/keygen.mjs` | Script de génération (code source) | Versionné |
| `tools/.keys.json` | Clé privée Ed25519 + clé AES-256 | **EXCLU** (`.gitignore`) |
| `src-tauri/src/constants.rs` | Clé publique Ed25519 + clé AES-256 (hex) | Versionné |

### Structure de `tools/.keys.json`

```json
{
  "ed25519_public_hex": "758f08...fd4",
  "ed25519_private_der_b64": "MC4CAQ...==",
  "aes_key_hex": "ab0bed...bbe"
}
```

| Champ | Format | Usage |
|-------|--------|-------|
| `ed25519_public_hex` | Hex (64 chars = 32 bytes) | Vérification de signature (aussi dans `constants.rs`) |
| `ed25519_private_der_b64` | Base64 (PKCS#8 DER) | Signature des clés (keygen uniquement) |
| `aes_key_hex` | Hex (64 chars = 32 bytes) | Chiffrement/déchiffrement (aussi dans `constants.rs`) |

### Bonnes pratiques de sécurité

1. **Ne jamais committer `tools/.keys.json`** dans le dépôt Git. Le fichier est déjà listé dans `.gitignore`.
2. **Stocker une sauvegarde** de `.keys.json` dans un coffre-fort (gestionnaire de mots de passe, KMS, etc.). La perte de ce fichier rend impossible la génération de nouvelles clés compatibles.
3. **Régénérer les clés** (`rm tools/.keys.json && node tools/keygen.mjs init`) si une compromission est suspectée. Cette action **invalide toutes les clés existantes** et nécessite de mettre à jour `constants.rs` + recompiler l'application.
4. **Les constantes dans `constants.rs`** (clé publique + clé AES) sont embarquées dans le binaire compilé. La clé AES est un secret partagé qui assure l'opacité de la clé de licence (l'utilisateur ne peut pas lire le payload en clair).

---

## 8. Intégration avec l'application

### Flux utilisateur

```
  Administrateur                       Utilisateur final
  ──────────────                       ─────────────────
  node tools/keygen.mjs generate
    --email user@example.com
    --duration 720
       │
       ▼
  Clé AIRENA-XXXXX-... ──────────────► Saisie dans Paramètres > Licence
                                          │
                                          ▼
                                       Clic "Valider"
                                          │
                                          ▼
                                       Backend : decode + verify + check
                                          │
                                          ▼
                                       Badge vert "Licence active"
                                       + date d'expiration
                                          │
                                          ▼
                                       Clic "Sauvegarder"
                                          │
                                          ▼
                                       Clé persistée en DB (settings)
                                          │
                                          ▼
                                       Nouvelle discussion débloquée
```

### Points de contrôle dans l'application

| Page | Comportement sans licence valide |
|------|-------------------------------|
| **Accueil** (HomePage) | Bannière d'avertissement + bouton "Nouvelle discussion" grisé |
| **Configuration** (SetupPage) | Bannière rouge + bouton "Démarrer" grisé |
| **Paramètres** (SettingsPage) | Section Licence avec saisie + validation + badge de statut |
| **Backend** (start_discussion) | Gate Rust : rejet avec `CommandError::License` avant tout traitement |

### Constantes modifiables

Toutes les constantes de licence sont dans `src-tauri/src/constants.rs` :

| Constante | Valeur | Description |
|-----------|--------|-------------|
| `LICENSE_VERSION` | `1` | Version du payload (pour évolution future) |
| `LICENSE_CLOCK_TOLERANCE_SECS` | `7200` | Tolérance anti-horloge (2h) |
| `LICENSE_DISCUSSIONS_PER_DAY` | `50` | Quota par 24h de validité |
| `LICENSE_ED25519_PUBLIC_KEY_HEX` | `"758f..."` | Clé publique Ed25519 (vérification) |
| `LICENSE_AES_KEY_HEX` | `"ab0b..."` | Clé AES-256-GCM (déchiffrement) |

---

## 9. Exemples courants

### Générer une clé de test (24 heures)

```bash
node tools/keygen.mjs generate --email dev@airena.local --duration 24
```

Résultat : licence valide 24h, 50 discussions max.

### Générer une clé mensuelle

```bash
node tools/keygen.mjs generate --email client@entreprise.com --duration 720
```

Résultat : licence valide 30 jours, 1 500 discussions max.

### Générer une clé annuelle

```bash
node tools/keygen.mjs generate --email premium@entreprise.com --duration 8760
```

Résultat : licence valide 365 jours, 18 250 discussions max.

### Vérifier l'état d'une clé

```bash
node tools/keygen.mjs inspect "AIRENA-cHR5c-GxhbG-9hZC1-0ZXN0-LWtle-Q1234-..."
```

Vérifie la signature, affiche email, dates, statut (ACTIVE/EXPIRED).

### Régénérer les clés (révocation totale)

```bash
rm tools/.keys.json
node tools/keygen.mjs init
```

Puis copier les nouvelles constantes dans `src-tauri/src/constants.rs` et recompiler.

> **ATTENTION** : Cette opération invalide **toutes** les clés précédemment générées.

---

## 10. Dépannage

### `ERROR: Key must start with AIRENA-`

La clé fournie à `inspect` ne commence pas par le préfixe attendu. Vérifier que la clé complète a bien été copiée.

### `ERROR: Key too short`

La clé est tronquée. Taille minimale du blob décodé : 93 octets (12 nonce + 16 tag + 64 signature + 1 payload). S'assurer que la clé n'a pas été coupée lors du copier-coller.

### `ERROR: Decryption failed (invalid or tampered key)`

Causes possibles :
- La clé a été générée avec un jeu de clés cryptographiques différent (autre `tools/.keys.json`)
- La clé a été modifiée manuellement (le chiffrement authentifié GCM détecte toute altération)
- Les constantes dans `constants.rs` ne correspondent pas au `.keys.json` utilisé pour la génération

**Solution** : Régénérer une clé avec le même `tools/.keys.json` que celui utilisé pour compiler l'application.

### `ERROR: Decrypted content too short`

Erreur interne : le contenu déchiffré fait moins de 65 octets (64 signature + 1 payload minimum). Clé corrompue ou incompatibilité de version.

### L'application affiche "Clé invalide" alors que `inspect` dit VALID

Vérifier que :
1. Les constantes hex dans `constants.rs` correspondent au `tools/.keys.json` actuel
2. L'application a été recompilée après la mise à jour des constantes (`npm run tauri build`)
3. La clé n'a pas expiré (vérifier le champ `Expires` dans `inspect`)
4. Le quota de discussions n'est pas épuisé

### L'application affiche "Clock moved backward"

L'horloge système a reculé de plus de 2 heures depuis le dernier contrôle de licence. Remettre l'horloge à l'heure correcte et relancer l'application.
