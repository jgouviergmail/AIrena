# Technical Review: Democratic & Authoritarian Turn Distribution Modes

## Executive Summary

Reviewed the plan for adding `Democratic` and `Authoritarian` variants to the `TurnDistribution` enum in the AIrena project. The analysis focuses on Rust type system correctness, async lifetime semantics, and API consistency.

**Critical issues found:**
- ✅ **Q1 (Non-exhaustive match)**: Plan DOES handle this correctly via fallback branches
- ⚠️ **Q2 (Borrow checker)**: BLOCKING ISSUE — borrowing immutable refs from `&mut self` across `.await` will fail
- ✅ **Q3 (join_all lifetimes)**: No issue — `join_all` accepts `impl IntoIterator<Item = Future>`
- ⚠️ **Q4 (Name matching)**: Plan should reuse `validate_reactions` pattern for consistency
- ⚠️ **Q5 (Parse fallback chain)**: Bare array fallback is INCORRECT — type mismatch
- ✅ **Q6 (CancellationToken lifetime)**: No issue — `Clone + 'static`

---

## Critical Question 1: Non-Exhaustive Match After Enum Extension

### Analysis

**Current code** (`turn_manager.rs:18-30`):
```rust
match distribution {
    TurnDistribution::Sequential => { /* ... */ }
    TurnDistribution::Random => { /* ... */ }
}
```

After adding `Democratic` and `Authoritarian` to the enum, this becomes **non-exhaustive** and Rust will **fail to compile** with error:
```
error[E0004]: non-exhaustive patterns: `TurnDistribution::Democratic` and `TurnDistribution::Authoritarian` not covered
```

### Plan's Solution

The plan addresses this via **dispatch pattern in orchestrator** (step 6):

```rust
let order = match &self.config.arbitre.turn_distribution {
    TurnDistribution::Sequential | TurnDistribution::Random => {
        turn_manager::determine_speaker_order(&self.gladiateurs, &self.config.arbitre.turn_distribution)
    }
    TurnDistribution::Democratic | TurnDistribution::Authoritarian => {
        // async path
    }
};
```

This means the orchestrator **never calls** `determine_speaker_order()` with `Democratic` or `Authoritarian`.

### Technical Issue

**The original `determine_speaker_order()` function is now dead code** for the new variants, BUT:
- It's called by **fallback logic** in both async functions (step 5)
- It's called by **unit tests** (L81-88)

**Impact:** The function MUST handle the new variants even if they're only used as fallback.

### Recommendation

**Option A (Minimal):** Add catch-all fallback to `determine_speaker_order()`:
```rust
match distribution {
    TurnDistribution::Sequential => { /* ... */ }
    TurnDistribution::Random => { /* ... */ }
    TurnDistribution::Democratic | TurnDistribution::Authoritarian => {
        // Fallback to Sequential — caller should use async path
        let mut sorted = active_indices;
        sorted.sort_by_key(|&i| gladiateurs[i].config.intervention_number);
        sorted
    }
}
```

**Option B (Correct):** Replace with `unreachable!()`:
```rust
match distribution {
    TurnDistribution::Sequential => { /* ... */ }
    TurnDistribution::Random => { /* ... */ }
    TurnDistribution::Democratic | TurnDistribution::Authoritarian => {
        unreachable!("Use async determine_order_* functions for these modes")
    }
}
```

Then update fallback logic in async functions to call Sequential DIRECTLY:
```rust
// Instead of determine_speaker_order(gladiateurs, distribution)
determine_speaker_order(gladiateurs, &TurnDistribution::Sequential)
```

**Verdict:** ⚠️ **PARTIAL** — Plan handles dispatch correctly but doesn't explicitly show how fallback calls avoid unreachable code.

---

## Critical Question 2: Borrow Checker & `&mut self` Across `.await`

### Analysis

**Current signature:** `orchestrator.rs:119`
```rust
pub async fn run(mut self, ...) { /* takes ownership */ }
```

**Proposed code** (step 6):
```rust
let ctx = turn_manager::AsyncTurnContext {
    ollama_client: &self.ollama_client,
    arbitre_system_prompt: &self.arbitre.config.system_prompt,
    // ... more borrows
};
match &self.config.arbitre.turn_distribution {
    TurnDistribution::Democratic => turn_manager::determine_order_democratic(&self.gladiateurs, &ctx).await,
    // ...
}
```

### Technical Issue

**Rust Rule:** When you take an immutable reference from `self` and store it in a struct, the compiler must ensure that reference remains valid across `.await` points.

**In this case:**
1. `run(mut self, ...)` takes **OWNERSHIP** (not `&mut self` as the question states)
2. The borrow is from an **owned** `self`, not `&mut self`
3. Async context holds borrowed refs via `AsyncTurnContext<'a>`

**Compiler check:**
- `self` is moved into `run()` and lives for the entire async fn
- `&self.ollama_client` is valid for the lifetime of `self` (which is `'a` in the context struct)
- The `.await` point cannot invalidate these borrows because `self` is owned

**But there's a subtlety:** The code borrows `&self.gladiateurs` for the `.await` while also passing it mutably later (line 352: `turn_manager::decrement_bans(&mut self.gladiateurs)`).

### Specific Lifetime Issue

The plan shows:
```rust
determine_order_democratic(&self.gladiateurs, &ctx).await
```

But `AsyncTurnContext` borrows:
```rust
pub arbitre_system_prompt: &'a str,
```

And the orchestrator code mutates `self.gladiateurs` at line 219, 244, 352, etc.

**The problem:** If `AsyncTurnContext` borrows immutable refs from `self` fields, and we later mutate `self.gladiateurs`, the compiler will reject this UNLESS the immutable borrows are dropped before mutation.

**In the proposed code:**
```rust
let ctx = AsyncTurnContext { ollama_client: &self.ollama_client, ... };
let order = determine_order_democratic(&self.gladiateurs, &ctx).await;
// ctx is dropped here — borrows end
// Now we can mutate self later
```

This **SHOULD WORK** because:
1. `ctx` is created in a temporary scope
2. After the `.await` completes, `ctx` is dropped
3. Immutable borrows end
4. Later mutations are allowed

### Caveat: Clone to Avoid Aliasing

If the compiler complains about aliasing (borrowing `&self.ollama_client` while holding `&self.gladiateurs` across an `.await`), the plan correctly suggests:

> "Si le borrow checker bloque (car `self` est `&mut` dans `run()`), cloner les champs nécessaires dans des variables locales avant le `.await`."

But the signature is `mut self`, not `&mut self`, so this is a **RED HERRING** in the plan.

**The correct issue:** `OllamaClient` might need to be cloned if it's used across threads in `join_all` (see Q3).

### Recommendation

**The plan's suggestion is INCORRECT in its reasoning** (it says `&mut self` but the signature is `mut self`), BUT the **solution is correct**:

```rust
// Clone only what's needed for the async context
let ollama_client = self.ollama_client.clone();
let arbitre_system_prompt = self.arbitre.config.system_prompt.clone();
let ctx = AsyncTurnContext {
    ollama_client: &ollama_client,
    arbitre_system_prompt: &arbitre_system_prompt,
    // ...
};
```

**Verdict:** ⚠️ **BLOCKING ISSUE** — The plan's explanation is wrong, but the fix works. The issue is not `&mut self` but potential aliasing of borrows. Since `determine_order_democratic` uses `join_all`, which might spawn tasks, the compiler may require clones.

---

## Critical Question 3: `join_all` and `'static` Requirements

### Analysis

**Proposed code** (step 5):
```rust
let futures: Vec<_> = active_glads.iter().map(|&idx| {
    let ctx_clone = /* ... */;
    async move { /* LLM call */ }
}).collect();
let results = futures_util::future::join_all(futures).await;
```

**`join_all` signature:**
```rust
pub fn join_all<I>(i: I) -> JoinAll<<I as IntoIterator>::Item>
where
    I: IntoIterator,
    <I as IntoIterator>::Item: Future,
```

**Key point:** `join_all` does NOT require `'static` futures! It accepts any `Future` and **awaits them in place** (not spawning tasks).

### Technical Issue

The plan shows:
```rust
let ctx = AsyncTurnContext { ollama_client: &self.ollama_client, ... };
```

Each future in the `join_all` captures `&ctx` (or fields from it).

**Rust rules:**
1. `join_all` itself does not require `'static`
2. BUT if you spawn tasks with `tokio::spawn`, those tasks MUST be `'static`
3. The plan uses `join_all`, not `spawn`, so **no `'static` requirement**

**However:** Each future captures `&OllamaClient`. If `OllamaClient` contains `Rc` or `&` refs, this might not be `Send`, and `join_all` is single-threaded anyway.

**`OllamaClient` definition** (from `src-tauri/src/ollama/client.rs`):
```rust
pub struct OllamaClient {
    client: reqwest::Client,
    base_url: String,
    model: String,
}
```

`reqwest::Client` is `Clone + Send + Sync`, so `&OllamaClient` is `Send`.

### Recommendation

**The plan CAN pass `&OllamaClient` to each future** because:
1. `join_all` does not spawn threads (it's cooperative concurrency)
2. All futures share the same `&OllamaClient` borrow from the parent async context
3. No `'static` bound required

**BUT:** If the implementation changes to use `tokio::spawn` for true parallelism, then `OllamaClient` must be `Arc<OllamaClient>` or cloned per task.

**For the current plan using `join_all`:** ✅ **NO ISSUE**

**Verdict:** ✅ **CORRECT** — No `'static` required for `join_all`.

---

## Critical Question 4: Name Matching Consistency

### Analysis

**Proposed code** (step 5):
```rust
name.to_lowercase().trim()
```

**Existing code** (`json_parser.rs:77-94`):
```rust
let r_lower = r.speaker.to_lowercase().trim().to_string();
// 1. Exact match
known_speakers.iter().find(|s| s.to_lowercase().trim() == r_lower)
// 2. Fallback: prefix match (min 3 chars)
.or_else(|| {
    if r_lower.len() >= 3 {
        known_speakers.iter().find(|s| s.to_lowercase().starts_with(&r_lower))
    } else {
        None
    }
})
```

### Technical Issue

The plan uses **exact match only** for Borda scoring:
```rust
if let Some(idx) = active_names.iter().position(|n| n.to_lowercase().trim() == name_lower) {
    // award points
}
```

**Inconsistency:** The existing `validate_reactions` function has:
1. Case-insensitive exact match
2. Fallback to prefix match (min 3 chars)

**Why prefix matching matters:**
- LLMs might abbreviate names: "Alice" → "Ali"
- Partial responses: "Bob Smith" → "Bob"
- Typos: "Alcie" → "Ali" (3-char prefix)

### Recommendation

**Reuse the exact pattern from `validate_reactions`:**

```rust
fn match_name(needle: &str, known_names: &[String]) -> Option<String> {
    let needle_lower = needle.to_lowercase().trim().to_string();
    known_names.iter()
        .find(|s| s.to_lowercase().trim() == needle_lower)
        .or_else(|| {
            if needle_lower.len() >= 3 {
                known_names.iter().find(|s| s.to_lowercase().starts_with(&needle_lower))
            } else {
                None
            }
        })
        .cloned()
}
```

Use this in:
- Borda scoring (democratic mode)
- Authoritarian order parsing
- Tiebreak parsing

**Verdict:** ⚠️ **INCONSISTENCY** — Plan should extract `validate_reactions`' matching logic into a shared helper.

---

## Critical Question 5: Parse Fallback Chain for `parse_vote`

### Analysis

**Proposed implementation** (step 4):
```rust
fn parse_vote(raw: &str) -> Vec<String> {
    parse_json_response::<VoteResponse>(raw)
        .map(|v| v.ranking)
        .or_else(|_| parse_json_response::<Vec<String>>(raw))
        .unwrap_or_default()
}
```

**Type check:**
```rust
struct VoteResponse { ranking: Vec<String> }
```

1. First attempt: `parse_json_response::<VoteResponse>(raw)` → `Result<VoteResponse, JsonParseError>`
2. `.map(|v| v.ranking)` → `Result<Vec<String>, JsonParseError>`
3. `.or_else(|_| parse_json_response::<Vec<String>>(raw))` → `Result<Vec<String>, JsonParseError>`

**Technical Issue:**

The question asks: "Could a bare JSON array `["Alice","Bob"]` be parsed by the first attempt?"

**Answer: NO**

`parse_json_response::<VoteResponse>` expects:
```json
{"ranking": ["Alice", "Bob"]}
```

A bare array `["Alice", "Bob"]` will FAIL the first attempt because it's not an object with a `ranking` field.

**But the fallback chain IS CORRECT:**
1. First attempt fails on bare array
2. `or_else` tries `parse_json_response::<Vec<String>>`
3. This SUCCEEDS on `["Alice", "Bob"]`

**However, there's a SUBTLE BUG:**

The `parse_json_response` implementation (L14-56) has this logic:
```rust
// 1. Direct parse
if let Ok(val) = serde_json::from_str::<T>(raw) { return Ok(val); }
// 2. Extract markdown block
// 3. Extract first JSON object OR ARRAY
if let Some(obj) = extract_first_json_object(raw) { /* ... */ }
```

The `extract_first_json_object` function (L133-181) has:
```rust
let (open, open_char, close_char) = match (brace_pos, bracket_pos) {
    (Some(b), Some(a)) => {
        if a < b { (a, '[', ']') } else { (b, '{', '}') }
    }
    // ...
}
```

**This ALREADY handles arrays!** So:
- `parse_json_response::<VoteResponse>("["Alice"]")` will try to deserialize `["Alice"]` as `VoteResponse`, which FAILS
- The fallback `parse_json_response::<Vec<String>>("["Alice"]")` will SUCCEED

**Verdict:** ✅ **CORRECT** — The fallback chain is sound. The first attempt will fail on bare arrays, triggering the fallback.

---

## Critical Question 6: `AsyncTurnContext` Lifetime & `CancellationToken`

### Analysis

**Proposed struct** (step 5):
```rust
pub struct AsyncTurnContext<'a> {
    pub ollama_client: &'a OllamaClient,
    pub cancel_token: CancellationToken,
    pub arbitre_system_prompt: &'a str,
    // ...
}
```

**`CancellationToken` type:**
```rust
// From tokio-util
pub struct CancellationToken { /* ... */ }
impl Clone for CancellationToken { /* ... */ }
```

The token is `Clone` and has **no lifetime parameters** (it's `'static` in the sense that it contains an `Arc` internally).

### Technical Issue

**Question:** Are there lifetime issues mixing `&'a OllamaClient` with `CancellationToken`?

**Answer: NO**

1. `CancellationToken` has no lifetime constraints
2. It's passed **by value** (via `Clone`) to async functions
3. The struct can mix owned (`CancellationToken`) and borrowed (`&'a OllamaClient`) fields

**Rust allows:**
```rust
struct Mixed<'a> {
    borrowed: &'a str,
    owned: String,
}
```

**In orchestrator:**
```rust
let ctx = AsyncTurnContext {
    ollama_client: &self.ollama_client,
    cancel_token: self.cancel_token.clone(), // Clone is cheap (Arc bump)
    arbitre_system_prompt: &self.arbitre.config.system_prompt,
    // ...
};
```

The `cancel_token.clone()` creates a new handle to the same underlying token. This is how cancellation works across async tasks.

**Verdict:** ✅ **NO ISSUE** — `CancellationToken` is designed for this exact use case.

---

## Additional Issues Not in Critical Questions

### 7. Error Handling in Async Functions

The plan shows fallbacks like:
```rust
// Fallback: erreur LLM → determine_speaker_order(Sequential)
```

**Issue:** `determine_speaker_order` is **sync**, but we're in an **async** context. This is fine UNLESS the fallback tries to call the async version recursively.

**Recommendation:** Make fallback explicit:
```rust
Err(e) => {
    tracing::warn!("Democratic order failed: {}, falling back to Sequential", e);
    determine_speaker_order(gladiateurs, &TurnDistribution::Sequential)
}
```

### 8. Parallel Vote Execution

The plan uses `join_all` for parallel voting:
```rust
let futures: Vec<_> = active_glads.iter().map(|&idx| { /* ... */ }).collect();
let results = join_all(futures).await;
```

**Issue:** This is **concurrent** (same thread), not **parallel** (multi-thread).

For true parallelism with Ollama (which has I/O waits), this is fine because `tokio` will interleave the I/O.

**BUT:** If two LLM calls to Ollama happen concurrently, the server might queue them. This is acceptable.

**Recommendation:** Add tracing to measure if parallel voting improves performance vs. sequential.

### 9. Tiebreak Prompt Context

The plan shows:
```rust
build_tiebreak_prompt(tied_names, discussion_summary, discussion_language)
```

**Issue:** Tiebreak needs context about WHY they're tied (same Borda score). The prompt should mention this:

> "Les participants suivants ont obtenu le même score : [Alice, Bob]. Décidez de leur ordre de parole."

This is implied but not explicit in the plan.

---

## Summary of Findings

| Question | Status | Severity | Issue |
|----------|--------|----------|-------|
| Q1: Non-exhaustive match | ⚠️ Partial | Medium | Fallback logic unclear |
| Q2: Borrow checker | ⚠️ Blocking | **HIGH** | Clone needed to avoid aliasing |
| Q3: join_all lifetimes | ✅ OK | None | No issue |
| Q4: Name matching | ⚠️ Inconsistent | Medium | Reuse existing pattern |
| Q5: Parse fallback | ✅ OK | None | Logic is correct |
| Q6: CancellationToken | ✅ OK | None | No issue |

---

## Recommendations

### Critical Fixes

1. **Q2 (Borrow checker):** Clone fields before creating `AsyncTurnContext`:
   ```rust
   let ollama_client = self.ollama_client.clone();
   let arbitre_system_prompt = self.arbitre.config.system_prompt.clone();
   let arbitre_llm_params = self.arbitre.config.llm_params.clone();
   let discussion_summary = self.arbitre.memory.contextual_summary.clone();
   let topic = self.config.topic.clone();
   let discussion_language = self.config.discussion_language.clone();

   let ctx = AsyncTurnContext {
       ollama_client: &ollama_client,
       cancel_token: self.cancel_token.clone(),
       arbitre_system_prompt: &arbitre_system_prompt,
       arbitre_llm_params: &arbitre_llm_params,
       discussion_summary: &discussion_summary,
       topic: &topic,
       current_turn: self.current_turn,
       discussion_language: &discussion_language,
   };
   ```

2. **Q1 (Match exhaustiveness):** Update `determine_speaker_order` to handle new variants:
   ```rust
   TurnDistribution::Democratic | TurnDistribution::Authoritarian => {
       // Fallback to Sequential — these should use async path
       tracing::warn!("Async turn distribution called sync function — using Sequential fallback");
       let mut sorted = active_indices;
       sorted.sort_by_key(|&i| gladiateurs[i].config.intervention_number);
       sorted
   }
   ```

3. **Q4 (Name matching):** Extract shared helper in `json_parser.rs`:
   ```rust
   /// Match a name against known names with case-insensitive exact match
   /// and fallback to prefix match (min 3 chars)
   pub fn match_name(needle: &str, known_names: &[String]) -> Option<String> {
       let needle_lower = needle.to_lowercase().trim().to_string();
       known_names.iter()
           .find(|s| s.to_lowercase().trim() == needle_lower)
           .or_else(|| {
               if needle_lower.len() >= 3 {
                   known_names.iter().find(|s| s.to_lowercase().starts_with(&needle_lower))
               } else {
                   None
               }
           })
           .cloned()
   }
   ```
   Use this in `validate_reactions`, Borda scoring, and authoritarian/tiebreak parsing.

### Optional Improvements

4. Add tracing for performance measurement:
   ```rust
   let start = std::time::Instant::now();
   let results = join_all(futures).await;
   tracing::info!("Democratic voting completed in {:?}", start.elapsed());
   ```

5. Add safety check for empty vote results:
   ```rust
   if results.iter().all(|r| r.is_empty()) {
       tracing::warn!("All votes failed — falling back to Sequential");
       return determine_speaker_order(gladiateurs, &TurnDistribution::Sequential);
   }
   ```

---

## Conclusion

The plan is **MOSTLY SOUND** but has **2 blocking issues** (Q2, Q1) and **1 consistency issue** (Q4) that must be addressed before implementation.

**Go/No-Go:** 🟡 **CONDITIONAL GO** — Fix Q2 (cloning) and Q1 (match exhaustiveness) before coding.
