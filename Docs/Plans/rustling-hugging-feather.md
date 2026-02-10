# Plan: Discussion History & Post-Discussion Navigation

## Context
All discussion data (messages, synthesis, emotions) is currently **ephemeral** — stored only in Zustand stores, lost on page reset or app restart. The DB has only `settings` and `predefined_profiles` tables. The user requested:
1. **Post-discussion navigation**: toggle between full discussion view and synthesis after a discussion ends
2. **Persistent history**: save discussions to SQLite, browse past discussions, delete individually or all

## Prerequisite: Clean up debug logging
Remove debug `console.warn` from reaction investigation:
- [useArenaStore.ts](src/stores/useArenaStore.ts) lines 140-166: remove verbose reaction logging in `reactionEmitted` handler
- [MessageBubble.tsx](src/components/discussion/MessageBubble.tsx) line 136: remove `console.warn` for reaction rendering

---

## Critical Design Decisions (from 3-iteration review)

### Save trigger: `discussionEnded` handler in useArenaStore (NOT SummaryPage mount)
- **Why not SummaryPage mount?** User could click "Home"/"New Discussion" (resetting stores) before save completes. Also, refreshing page at /summary loses all data.
- **Why not Rust engine?** The engine's `messages_history` does NOT include reactions (reactions are emitted as separate events and only applied in the frontend Zustand store). Saving from frontend captures complete messages with reactions.
- **How it works**: In the `discussionEnded` handler, synchronously capture all data from `useArenaStore.getState()` + `useSetupStore.getState()` + `useSettingsStore.getState()`, then fire-and-forget async `saveDiscussionHistory()`. Data is serialized immediately by Tauri IPC — safe even if stores reset afterward.

### Participant metadata: `participants_json` column on discussions table
- Stores `[{id, name, role, emoji}]` for all participants (gladiateurs + arbitre + user)
- Enables emojiMap reconstruction and participant name highlighting in history view
- JSON column (not join table) — participant data is always loaded with its discussion, never queried independently

### Message ordering: `sort_order INTEGER` column
- Monotonically increasing integer based on array index at save time
- More reliable than timestamp (sub-ms precision issues within same turn)

### Edge cases handled
- **Force-stop** (no synthesis): saves with empty synthesis — UI shows "No synthesis"
- **Force-stop** (no messages): skips save if `messages.length === 0`
- **Idempotency**: `ON CONFLICT(id) DO NOTHING` prevents duplicate saves
- **Reactions**: captured from frontend store (engine doesn't track them on messages)
- **Ban notifications**: stored via `is_ban_notification` column, rendered correctly by MessageBubble
- **Inner thoughts**: stored via `inner_thought` column, toggle works in ReadOnlyFeed

---

## Part 1: Backend — Database & Models

### 1A. Schema — [schema.rs](src-tauri/src/db/schema.rs)
Add to `SCHEMA` constant (idempotent with `IF NOT EXISTS`):

```sql
CREATE TABLE IF NOT EXISTS discussions (
    id TEXT PRIMARY KEY,
    topic TEXT NOT NULL,
    discussion_language TEXT NOT NULL DEFAULT 'fr',
    model_name TEXT NOT NULL DEFAULT '',
    participants_json TEXT NOT NULL DEFAULT '[]',
    total_turns INTEGER NOT NULL DEFAULT 0,
    synthesis TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS discussion_messages (
    id TEXT PRIMARY KEY,
    discussion_id TEXT NOT NULL,
    turn_number INTEGER NOT NULL DEFAULT 0,
    speaker_id TEXT NOT NULL,
    speaker_name TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL DEFAULT '',
    inner_thought TEXT,
    reactions_json TEXT NOT NULL DEFAULT '[]',
    is_ban_notification INTEGER NOT NULL DEFAULT 0,
    timestamp TEXT NOT NULL DEFAULT '',
    sort_order INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (discussion_id) REFERENCES discussions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_dm_discussion_id
    ON discussion_messages(discussion_id, sort_order);
```

Key details:
- `PRAGMA foreign_keys=ON` already in existing schema — CASCADE works
- `role TEXT` stores serde output: `"IArbitre"`, `"GladIAteur"`, or `"user"` (per-variant rename)
- `reactions_json` / `participants_json`: JSON-serialized arrays
- `timestamp TEXT`: ISO 8601 / RFC 3339 format (matches chrono serde)

### 1B. Models — **NEW** [models/history.rs](src-tauri/src/models/history.rs)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticipantInfo {
    pub id: String,
    pub name: String,
    pub role: String,      // "IArbitre", "GladIAteur", "user"
    pub emoji: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveDiscussionRequest {
    pub id: String,
    pub topic: String,
    pub discussion_language: String,
    pub model_name: String,
    pub participants: Vec<ParticipantInfo>,
    pub total_turns: u32,
    pub synthesis: String,
    pub created_at: String,
    pub messages: Vec<Message>,  // reuse existing Message type from models/message.rs
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionSummary {
    pub id: String,
    pub topic: String,
    pub discussion_language: String,
    pub model_name: String,
    pub participants: Vec<ParticipantInfo>,
    pub total_turns: u32,
    pub has_synthesis: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionDetail {
    pub id: String,
    pub topic: String,
    pub discussion_language: String,
    pub model_name: String,
    pub participants: Vec<ParticipantInfo>,
    pub total_turns: u32,
    pub synthesis: String,
    pub created_at: String,
    pub messages: Vec<Message>,  // reuse existing Message type
}
```

Add `pub mod history;` to [models/mod.rs](src-tauri/src/models/mod.rs).

### 1C. Repository — [repository.rs](src-tauri/src/db/repository.rs)

5 new functions following existing `db.call(|conn| { ... })` pattern:

1. **`save_discussion(db, request: SaveDiscussionRequest)`** — Single transaction:
   - `INSERT INTO discussions` (serialize participants to JSON via `serde_json::to_string`)
   - Loop `INSERT INTO discussion_messages` (serialize reactions to JSON, use enumerate index for sort_order)
   - Use `ON CONFLICT(id) DO NOTHING` for idempotency
   - All data moved into closure (Send + 'static)

2. **`list_discussions(db)`** → `Vec<DiscussionSummary>`:
   - SELECT from discussions only, ORDER BY created_at DESC
   - Parse `participants_json` with `serde_json::from_str`
   - `has_synthesis` = `!synthesis.is_empty()`

3. **`get_discussion(db, id)`** → `Option<DiscussionDetail>`:
   - SELECT discussion + SELECT messages WHERE discussion_id = ? ORDER BY sort_order
   - Parse `reactions_json` per message with `serde_json::from_str`
   - Reconstruct `Message` structs (parse role as `SpeakerRole`, timestamp as `DateTime<Utc>`)

4. **`delete_discussion(db, id)`** — DELETE FROM discussions WHERE id = ? (CASCADE handles messages)

5. **`delete_all_discussions(db)`** — DELETE FROM discussions

### 1D. Commands — **NEW** [commands/history.rs](src-tauri/src/commands/history.rs)

```rust
#[tauri::command]
pub async fn save_discussion_history(request: SaveDiscussionRequest, state: State<'_, AppState>) -> Result<(), CommandError>

#[tauri::command]
pub async fn list_discussion_history(state: State<'_, AppState>) -> Result<Vec<DiscussionSummary>, CommandError>

#[tauri::command]
pub async fn get_discussion_history(id: String, state: State<'_, AppState>) -> Result<Option<DiscussionDetail>, CommandError>

#[tauri::command]
pub async fn delete_discussion_history(id: String, state: State<'_, AppState>) -> Result<(), CommandError>

#[tauri::command]
pub async fn delete_all_discussion_history(state: State<'_, AppState>) -> Result<(), CommandError>
```

Pattern: `let db = state.db.clone();` BEFORE any `.await` (Tauri v2 lifetime constraint).

### 1E. Error — [error.rs](src-tauri/src/error.rs)
Add `#[error("History error: {0}")] History(String)` to `CommandError`.

### 1F. Registration — [lib.rs](src-tauri/src/lib.rs) + [commands/mod.rs](src-tauri/src/commands/mod.rs)
Add `pub mod history;` and register 5 commands in `generate_handler![]`.

---

## Part 2: Frontend — Types & API

### 2A. Types — [types.ts](src/lib/types.ts)

```typescript
export interface ParticipantInfo {
  id: string;
  name: string;
  role: SpeakerRole;
  emoji: string;
}

export interface DiscussionSummary {
  id: string;
  topic: string;
  discussionLanguage: string;
  modelName: string;
  participants: ParticipantInfo[];
  totalTurns: number;
  hasSynthesis: boolean;
  createdAt: string;
}

export interface DiscussionDetail {
  id: string;
  topic: string;
  discussionLanguage: string;
  modelName: string;
  participants: ParticipantInfo[];
  totalTurns: number;
  synthesis: string;
  createdAt: string;
  messages: Message[];
}

export interface SaveDiscussionRequest {
  id: string;
  topic: string;
  discussionLanguage: string;
  modelName: string;
  participants: ParticipantInfo[];
  totalTurns: number;
  synthesis: string;
  createdAt: string;
  messages: Message[];
}
```

### 2B. Tauri API — [tauri-api.ts](src/lib/tauri-api.ts)

```typescript
export async function saveDiscussionHistory(request: SaveDiscussionRequest): Promise<void>
export async function listDiscussionHistory(): Promise<DiscussionSummary[]>
export async function getDiscussionHistory(id: string): Promise<DiscussionDetail | null>
export async function deleteDiscussionHistory(id: string): Promise<void>
export async function deleteAllDiscussionHistory(): Promise<void>
```

---

## Part 3: Auto-Save on Discussion End

### [useArenaStore.ts](src/stores/useArenaStore.ts) — `discussionEnded` handler

```typescript
case "discussionEnded": {
    stopSynthBuffering();

    // Capture data BEFORE setting status (stores still populated)
    const arenaState = useArenaStore.getState();
    const setupState = useSetupStore.getState();
    const settingsState = useSettingsStore.getState();

    if (arenaState.discussionId && arenaState.messages.length > 0) {
        const participants: ParticipantInfo[] = [
            { id: setupState.arbitre.id, name: setupState.arbitre.name,
              role: "IArbitre", emoji: ROLE_EMOJIS.IArbitre },
            ...setupState.gladiateurs.map(g => ({
                id: g.id, name: g.name, role: "GladIAteur" as SpeakerRole,
                emoji: g.emoji ?? getProfileEmoji(g.name, g.systemPrompt),
            })),
            { id: "user", name: settingsState.settings.username,
              role: "user", emoji: ROLE_EMOJIS.user },
        ];

        saveDiscussionHistory({
            id: arenaState.discussionId,
            topic: setupState.topic,
            discussionLanguage: setupState.discussionLanguage,
            modelName: settingsState.settings.ollamaModel,
            participants,
            totalTurns: arenaState.currentTurn,
            synthesis: arenaState.synthesis,
            createdAt: new Date().toISOString(),
            messages: arenaState.messages,
        }).catch(err => console.error("Failed to save discussion history:", err));
    }

    set({ status: "ended" });
    break;
}
```

New imports in useArenaStore: `saveDiscussionHistory` from tauri-api, `useSetupStore`/`useSettingsStore` (`.getState()`), `getProfileEmoji`/`ROLE_EMOJIS` from profile-emoji, `ParticipantInfo`/`SaveDiscussionRequest` from types.

---

## Part 4: Post-Discussion Navigation (SummaryPage)

### 4A. Tab toggle — [SummaryPage.tsx](src/pages/SummaryPage.tsx)

```typescript
const [tab, setTab] = useState<"synthesis" | "discussion">("synthesis");
```

- Two tab buttons above content area
- "Synthesis" tab: existing synthesis view (unchanged)
- "Full Discussion" tab: `<ReadOnlyFeed messages={messages} participants={participants} />`
  - Build `participants` from stores (same logic as save)
- Add "History" button in actions bar (navigates to `/history`)

### 4B. ReadOnlyFeed — **NEW** [ReadOnlyFeed.tsx](src/components/discussion/ReadOnlyFeed.tsx)

Props: `messages: Message[]`, `participants: ParticipantInfo[]`

Internally computes:
- `emojiMap: Map<string, string>` from participants (role-based for arbitre/user, stored emoji for gladiateurs)
- `participantNames: string[]` for name highlighting

Reuses existing `MessageBubble` component — supports reactions, inner thoughts, ban notifications, name highlighting all via existing props. No streaming, no token buffer, no active speaker.

---

## Part 5: History Pages

### 5A. History list — **NEW** [HistoryPage.tsx](src/pages/HistoryPage.tsx)

- `useEffect` → `listDiscussionHistory()` on mount
- Each card: topic (truncated), date (formatted), participant emojis, turn count, synthesis indicator
- Click → `navigate("/history/" + id)`
- Delete button per entry with `window.confirm()` (same pattern as `hardStopConfirm` in ArenaPage)
- "Delete All" button with `window.confirm()`
- Empty state message
- TopBar with title

### 5B. History detail — **NEW** [HistoryDetailPage.tsx](src/pages/HistoryDetailPage.tsx)

- `useParams()` → `id` from route
- `useEffect` → `getDiscussionHistory(id)` on mount → local state
- Same layout as SummaryPage: stat cards (topic, turns, participants) + tab toggle
- "Discussion" tab uses `<ReadOnlyFeed messages={detail.messages} participants={detail.participants} />`
- "Back to History" + "Delete" buttons
- Loading state while fetching

### 5C. Router — [App.tsx](src/App.tsx)

```tsx
const HistoryPage = lazy(() => import("@/pages/HistoryPage"));
const HistoryDetailPage = lazy(() => import("@/pages/HistoryDetailPage"));
// Inside Routes, under AppShell:
<Route path="/history" element={<HistoryPage />} />
<Route path="/history/:id" element={<HistoryDetailPage />} />
```

### 5D. Sidebar — [Sidebar.tsx](src/components/layout/Sidebar.tsx)

Add to `navItems`:
```typescript
{ key: "history", path: "/history", icon: History, labelKey: "nav.history" },
```

Import `History` from lucide-react. Update `isArenaActive` if needed (history routes should not show arena indicator).

### 5E. HomePage — [HomePage.tsx](src/pages/HomePage.tsx)

Add "History" button next to existing buttons (uses `History` icon from lucide-react).

---

## Part 6: i18n — [fr.json](src/i18n/locales/fr.json), [en.json](src/i18n/locales/en.json), [zh.json](src/i18n/locales/zh.json)

New keys (French shown, translate for EN and ZH):
```json
{
  "nav": { "history": "Historique" },
  "home": { "history": "Historique" },
  "history": {
    "title": "Historique des discussions",
    "empty": "Aucune discussion enregistrée",
    "delete": "Supprimer",
    "deleteAll": "Tout supprimer",
    "deleteConfirm": "Supprimer cette discussion ?",
    "deleteAllConfirm": "Supprimer tout l'historique ?",
    "turns": "{{count}} tour(s)",
    "noSynthesis": "Sans synthèse",
    "back": "Retour à l'historique"
  },
  "summary": {
    "tabSynthesis": "Synthèse",
    "tabDiscussion": "Discussion complète",
    "viewHistory": "Historique"
  }
}
```

---

## File Change Summary

| File | Action | Description |
|------|--------|-------------|
| `src-tauri/src/db/schema.rs` | MODIFY | Add 2 tables + 1 index |
| `src-tauri/src/db/repository.rs` | MODIFY | Add 5 history functions |
| `src-tauri/src/models/history.rs` | **NEW** | ParticipantInfo, SaveDiscussionRequest, DiscussionSummary, DiscussionDetail |
| `src-tauri/src/models/mod.rs` | MODIFY | `pub mod history` |
| `src-tauri/src/commands/history.rs` | **NEW** | 5 Tauri commands |
| `src-tauri/src/commands/mod.rs` | MODIFY | `pub mod history` |
| `src-tauri/src/error.rs` | MODIFY | `History(String)` variant |
| `src-tauri/src/lib.rs` | MODIFY | Register 5 commands |
| `src/lib/types.ts` | MODIFY | 4 new interfaces |
| `src/lib/tauri-api.ts` | MODIFY | 5 new API functions |
| `src/stores/useArenaStore.ts` | MODIFY | Save logic in `discussionEnded` + remove debug logging |
| `src/components/discussion/ReadOnlyFeed.tsx` | **NEW** | Read-only message feed |
| `src/components/discussion/MessageBubble.tsx` | MODIFY | Remove debug logging |
| `src/pages/SummaryPage.tsx` | MODIFY | Tab toggle + history button |
| `src/pages/HistoryPage.tsx` | **NEW** | History list |
| `src/pages/HistoryDetailPage.tsx` | **NEW** | History detail |
| `src/pages/HomePage.tsx` | MODIFY | History button |
| `src/App.tsx` | MODIFY | 2 routes |
| `src/components/layout/Sidebar.tsx` | MODIFY | History nav item |
| `src/i18n/locales/fr.json` | MODIFY | History + summary keys |
| `src/i18n/locales/en.json` | MODIFY | History + summary keys |
| `src/i18n/locales/zh.json` | MODIFY | History + summary keys |

---

## Implementation Order

1. Clean up debug logging (useArenaStore.ts, MessageBubble.tsx)
2. Backend: schema + models + repository + commands + error + registration
3. Frontend: types + Tauri API functions
4. Auto-save in `discussionEnded` handler (useArenaStore.ts)
5. ReadOnlyFeed component
6. SummaryPage tab toggle + history button
7. HistoryPage + HistoryDetailPage
8. Router + Sidebar + HomePage updates
9. i18n (all 3 languages)
10. Build & test

## Verification

1. `cargo build` — Rust compiles
2. `npm run build` — frontend compiles
3. Start discussion → end normally → verify SummaryPage shows both tabs (synthesis + discussion)
4. Navigate to `/history` → verify discussion appears with correct metadata + emojis
5. Click entry → verify full discussion with reactions, inner thoughts, ban notifications
6. Delete one → verify removed
7. New discussion → verify old history persists
8. Force-stop a discussion → verify saved with empty synthesis
9. Delete All → verify cleared
