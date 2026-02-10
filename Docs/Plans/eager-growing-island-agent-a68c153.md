# Functional Review: Democratic & Authoritarian Turn Distribution Modes

## Summary of Findings

This document is a critical functional review of the plan in `serene-napping-sloth.md` (section "Modes de distribution Democratique et Autoritaire") against the current codebase. The analysis covers 10 edge-case scenarios.

**Verdict: The plan has 4 critical issues, 3 moderate issues, and 3 items that are handled correctly (or need minor clarification).**

---

## Scenario 1: 2 gladiators, democratic mode — guaranteed tie

### Situation
With 2 active gladiators (A, B), each votes for the OTHER participants. Since each only has 1 other participant, both receive exactly 1 vote. Borda count: A gets (N-1)=1 point from B, B gets (N-1)=1 point from A. Permanent tie every single turn.

### Analysis: MODERATE ISSUE

The plan says "En cas d'egalite, IArbitre departage." This means **every single turn** with 2 gladiators in Democratic mode will trigger a tiebreaker LLM call to IArbitre. That is:
- **N+1 LLM calls per turn** (2 votes + 1 tiebreak) instead of the advertised N+0..1
- The tiebreaker call is not "optional" — it is mandatory 100% of the time
- The IArbitre tiebreaker prompt (`build_tiebreaker_prompt`) receives the same 2 names every turn with no differentiating context, so the LLM will likely return a semi-random or biased result

**Degenerate case**: If IArbitre consistently favors one gladiator (LLMs have positional bias — they tend to pick the first name listed), one gladiator always speaks first, creating an unfair structural advantage. The gladiator who speaks first each turn sets the framing; the second always responds reactively.

### Recommendation
For N=2, consider bypassing the Borda vote entirely and using a simple coin-flip (random), or alternating A-first/B-first across turns. The democratic vote adds 3 LLM calls per turn for zero information gain. Alternatively, document this as a known limitation and recommend Democratic mode for 3+ gladiators.

---

## Scenario 2: 3+ gladiators, all vote for the same person

### Situation
N=4 gladiators (A, B, C, D). Gladiators B, C, D all rank A first. A ranks someone else first. Result: A gets max points, but the remaining 3 (B, C, D) could have a 3-way tie at low scores.

### Analysis: WORKS CORRECTLY (with a nuance)

The Borda count handles this fine mathematically. A gets the highest score and speaks first. The remaining gladiators (B, C, D) would have varying scores based on how they were ranked by other voters — a full 3-way tie is unlikely unless votes are perfectly symmetric.

**However**, the plan says the tiebreaker prompt is `build_tiebreaker_prompt(tied_names, topic, lang)`. This prompt receives only the tied names and the topic. It does NOT receive:
- The discussion history / previous turn messages
- Any context about WHY these participants are tied
- The current turn number

Without context, IArbitre's tiebreak decision is essentially random. This is acceptable for occasional 2-way ties but becomes problematic for frequent large ties.

### Recommendation
Pass `messages_history` and `current_turn` to the tiebreaker prompt so IArbitre can make an informed decision based on who has been least heard recently.

---

## Scenario 3: LLM returns names with slight variations

### Situation
Gladiator name: "L'Avocat du Diable". LLM returns variations like:
- "l'avocat du diable" (lowercase)
- "Avocat du Diable" (missing article)
- "L'avocat" (truncated)
- "L'Avocat du diable" (mixed case)

### Analysis: CRITICAL ISSUE

The plan says name resolution uses `eq_ignore_ascii_case` (same pattern as `validate_reactions`). But let's look at what `validate_reactions` actually does in the current code (`json_parser.rs:77-108`):

```rust
// 1. Exact match (case-insensitive, trimmed)
let speaker = known_speakers
    .iter()
    .find(|s| s.to_lowercase().trim() == r_lower)
    // 2. Fallback: known name starts with the LLM-provided string (min 3 chars)
    .or_else(|| {
        if r_lower.len() >= 3 {
            known_speakers
                .iter()
                .find(|s| s.to_lowercase().starts_with(&r_lower))
        } else {
            None
        }
    })?;
```

This matching has TWO strategies:
1. **Exact case-insensitive match**: "l'avocat du diable" matches "L'Avocat du Diable" -- OK
2. **Prefix match** (fallback): known name `starts_with` the LLM string (min 3 chars)

Now consider the variants:
- "l'avocat du diable" -> Exact match via lowercase. **OK**
- "L'Avocat du diable" -> `to_lowercase()` gives "l'avocat du diable", exact match. **OK**
- "L'avocat" -> Prefix match: "l'avocat du diable".starts_with("l'avocat") = **true**. **OK**
- "Avocat du Diable" -> "avocat du diable" does NOT equal "l'avocat du diable", and "l'avocat du diable".starts_with("avocat du diable") = **false**. **FAILS**

**The critical issue**: The plan says to use `eq_ignore_ascii_case`, but the actual working code uses `to_lowercase()` + exact match with a `starts_with` fallback. The plan should use `to_lowercase()` (which handles Unicode properly for French characters) rather than `eq_ignore_ascii_case` (which only handles ASCII a-z/A-Z). Furthermore:

- `eq_ignore_ascii_case` will NOT handle the missing-article case ("Avocat du Diable" without the "L'"). This is a common LLM behavior for French names with articles.
- The `starts_with` fallback in the current code goes the WRONG direction for vote parsing. It checks if the KNOWN name starts with the LLM string. So "L'avocat" matches because "l'avocat du diable" starts with "l'avocat". But "Avocat du Diable" fails because "l'avocat du diable" does NOT start with "avocat du diable".

**Additional concern for Democratic mode**: The `DemocraticVote` struct has `ranking: Vec<String>`. If name resolution fails for a name in the ranking, that vote position is lost. With Borda count, a missing vote distorts the point distribution — some gladiators get fewer points than they should.

### Recommendation
1. Use `to_lowercase()` not `eq_ignore_ascii_case` (consistency with existing code + Unicode correctness)
2. Add a `contains` fallback: if exact and prefix both fail, check if the known name `contains` the LLM string or vice versa (minimum 4 chars to avoid false positives)
3. When a name in a vote ranking cannot be resolved, skip that position but DO NOT shift the remaining positions up (otherwise a gladiator ranked 3rd gets 2nd-place points)

---

## Scenario 4: LLM returns completely wrong names

### Situation
LLM returns `{"ranking": ["Alice", "Bob"]}` but no gladiator has those names.

### Analysis: HANDLED BY DESIGN (but verify fallback)

The plan says: "Noms non reconnus ignores" for Authoritarian mode. For Democratic mode, if all names in a vote are unrecognized, the voter's entire vote is effectively null.

In the worst case: ALL voters return garbage names. Every vote is null. All gladiators have 0 Borda points. This is an N-way tie. The tiebreaker prompt fires with ALL gladiators listed.

**Concern**: The plan's fallback is "si un appel LLM echoue, tracing::warn + fallback sequentiel." But a vote that parses successfully as JSON but contains wrong names is NOT an LLM failure — the JSON parse succeeds, the name resolution silently fails. The plan needs to distinguish between:
- LLM call failure (network/timeout) -> fallback to sequential
- LLM returns valid JSON with garbage names -> 0-point vote (silent)
- LLM returns invalid JSON -> parse failure -> fallback to sequential

The current handling where garbage names produce 0-point votes and eventually trigger a tiebreaker is functionally correct but wasteful. If ALL votes resolve to 0 points, the system should detect this and fall back to sequential instead of making an additional tiebreaker call.

### Recommendation
Add a heuristic: if total recognized votes across all voters is 0, fall back to sequential immediately instead of calling the tiebreaker with all names.

---

## Scenario 5: All voting LLM calls fail (Ollama overloaded)

### Situation
Democratic mode, all N vote calls fail (timeout, connection error, etc.).

### Analysis: MODERATE ISSUE

The plan says "si un appel LLM echoue, tracing::warn + fallback sequentiel." But the plan does not specify the granularity of this fallback:

**Option A**: Each individual vote failure is independent. If voter A fails but voter B succeeds, do you use partial votes + tiebreaker? Or do you require ALL votes to succeed?

**Option B**: If ANY vote fails, the entire Democratic round falls back to sequential.

**Option C**: If ALL votes fail, fall back to sequential.

The plan is ambiguous. Given that the OllamaClient has retry logic (3 attempts with exponential backoff), a single vote failure after retries suggests a real problem. But partial results could work — if 3 of 4 voters succeed, the Borda count is still meaningful (the missing voter's preferences are simply absent).

**Sequential fallback correctness**: The current `determine_speaker_order` returns `Vec<usize>` (indices into the gladiateurs array). The new `determine_speaker_order_llm` must return the same `Vec<usize>` type. If it falls back to sequential, it should call the existing `determine_speaker_order(&gladiateurs, &TurnDistribution::Sequential)` — this correctly filters banned gladiators and sorts by `intervention_number`. This part appears sound.

### Recommendation
Use Option C (all-or-nothing): if ALL votes fail, fall back to sequential. If at least some votes succeed, use partial results. The plan should explicitly state this behavior.

---

## Scenario 6: Authoritarian mode — LLM returns partial list

### Situation
4 active gladiators (A, B, C, D). IArbitre returns `{"order": ["B", "D"]}` — only 2 of 4.

### Analysis: HANDLED BY DESIGN

The plan explicitly says: "Noms non reconnus ignores, participants manquants ajoutes a la fin." So the result would be [B, D, A, C] (or [B, D, C, A] depending on how "added at the end" is ordered).

**The question is: in what order are the missing participants added?**

The plan does not specify. Options:
- Sequential order (by `intervention_number`) — deterministic, predictable
- Random order — avoids positional bias
- Alphabetical — arbitrary but deterministic

### Recommendation
Use sequential order (`intervention_number`) for missing participants. This is deterministic, testable, and consistent with the Sequential fallback. The plan should explicitly state this.

---

## Scenario 7: Discussion force-stopped during voting phase

### Situation
Democratic mode, 3 vote LLM calls are in progress. User clicks force-stop.

### Analysis: CRITICAL ISSUE

Looking at the orchestrator's main loop (`orchestrator.rs:194-255`), the flow is:

```rust
// Line 199: check commands (including ForceStop)
if self.process_commands(&mut cmd_rx, &channel).await { break; }

// Line 208: determine speaker order (currently sync, plan makes it async)
let order = turn_manager::determine_speaker_order(...);
```

With the plan's change, line 208 becomes an `.await` point:
```rust
let order = turn_manager::determine_speaker_order_llm(...).await;
```

**The problem**: `process_commands` is checked BEFORE the voting phase. If the user sends ForceStop DURING the voting phase (while `determine_speaker_order_llm` is awaiting LLM responses), the ForceStop command sits in the `cmd_rx` channel unprocessed until the voting completes.

The `cancel_token` IS passed to `determine_speaker_order_llm` per the plan, and the OllamaClient checks for cancellation during streaming. So individual LLM calls would be cancelled. BUT:

1. The plan proposes making N sequential vote calls (one per gladiator). If gladiator 1's vote is cancelled, does the function immediately return the sequential fallback? Or does it try gladiator 2's vote?
2. The `cancel_token` cancels the HTTP stream, but `determine_speaker_order_llm` needs to explicitly check `cancel_token.is_cancelled()` between vote calls.

**Force-stop propagation path**:
- `force_stop_discussion` command -> sends `EngineCommand::ForceStop` via mpsc AND calls `cancel_token.cancel()`
- The `cancel_token.cancel()` will interrupt any in-flight OllamaClient call
- The OllamaClient returns `Err(OllamaError::Cancelled)`
- `determine_speaker_order_llm` receives the error and should return the sequential fallback
- Back in the orchestrator, `process_commands` runs next and picks up the ForceStop

**This works IF** `determine_speaker_order_llm` correctly handles `OllamaError::Cancelled` by returning immediately (not retrying). The plan says "si un appel LLM echoue, tracing::warn + fallback sequentiel" — Cancelled is an error, so it would fall back to sequential. The orchestrator would then proceed to `process_commands` which would detect ForceStop and break.

**Verdict**: Functionally correct, but with unnecessary latency. After cancellation, the function falls back to sequential, then the orchestrator determines the speaker order, emits TurnStarted, THEN checks for ForceStop and breaks. There is a brief window where the UI might see a TurnStarted event for a turn that never executes.

### Recommendation
After `determine_speaker_order_llm` returns, add an explicit cancellation check before emitting TurnStarted:
```rust
let order = determine_speaker_order_llm(...).await;
if self.cancel_token.is_cancelled() || self.process_commands(...).await { break; }
// ... then emit TurnStarted
```

---

## Scenario 8: Only 1 active gladiator (others banned)

### Situation
3 gladiators total, 2 are banned. Only gladiator C is active.

### Analysis: CRITICAL ISSUE

The plan does not address this case explicitly. Let's trace the behavior:

**Democratic mode with 1 active gladiator**:
- The algorithm says "Pour chaque gladiateur actif: appel LLM pour voter"
- Gladiator C is the only active one. C votes for... the OTHER participants. But the other participants are banned.
- **Question**: Does the vote prompt include banned gladiators or only active ones?

The plan says the vote lists "AUTRES participants" without specifying active-only. If banned gladiators are included in the vote, C votes for banned gladiators who cannot speak — the votes are meaningless. If banned gladiators are excluded, C has nobody to vote for (empty ranking).

**With 1 active gladiator, there is no choice to make**. The only valid order is [C]. Making an LLM call to determine this is wasteful and could fail.

**Authoritarian mode with 1 active gladiator**:
- IArbitre is asked to order 1 participant. The result is trivially [C].
- Still 1 unnecessary LLM call.

### Recommendation
Add an early return at the top of `determine_speaker_order_llm`:
```rust
if active_indices.len() <= 1 {
    return active_indices; // No vote/order needed
}
```
This handles both the 0-active (all banned, already handled by orchestrator) and 1-active cases efficiently.

---

## Scenario 9: Banned gladiator's name returned in votes

### Situation
Gladiator B is banned. Gladiator A votes and returns `{"ranking": ["B", "C", "D"]}`. B is banned and should not speak this turn.

### Analysis: CRITICAL ISSUE

The plan says nothing about filtering banned gladiators from vote results. The plan says:
- "Pour chaque gladiateur actif: appel LLM" (only active gladiators VOTE)
- But it does not say the vote prompt should exclude banned gladiators from the candidate list

**Two sub-questions**:

**Q1: Are banned gladiators included in the vote prompt's candidate list?**
The plan's `build_democratic_vote_prompt` takes `other_names` as a parameter. If the caller passes ALL gladiator names (including banned ones), the LLM will rank banned gladiators. If it passes only active ones, this scenario is avoided.

**Q2: If banned names appear in vote results, are they filtered?**
The plan says nothing about this. If banned gladiator B receives votes and gets the highest Borda score, the algorithm would rank B first in the speaking order — but B is banned. The orchestrator currently filters banned gladiators at the `determine_speaker_order` level (line 11-16 of turn_manager.rs). But in the planned `determine_speaker_order_llm`, the Borda count operates on names, not indices. If the function resolves names to indices and only considers active indices, banned names are automatically excluded.

**The crucial implementation detail**: The plan's algorithm resolves names to gladiator indices. If the name resolution only looks up ACTIVE gladiators, banned names are silently dropped. If it looks up ALL gladiators, a banned gladiator could end up in the result — but the orchestrator iterates over the returned order and each gladiator goes through the full speech cycle, so a banned gladiator being in the order would cause them to speak when they shouldn't.

### Recommendation
1. The vote prompt should ONLY include active (non-banned) gladiator names as candidates
2. Name resolution in `determine_speaker_order_llm` should only map to active gladiator indices
3. Add an explicit assertion/filter: the returned `Vec<usize>` should only contain indices of active gladiators

---

## Scenario 10: Mixed mode during ongoing discussion

### Situation
Can the turn distribution mode change mid-discussion?

### Analysis: SAFE — NOT POSSIBLE

Looking at the code:
- `TurnDistribution` is stored in `IArbitreConfig` which is part of `DiscussionConfig`
- `DiscussionConfig` is passed to `DiscussionEngine::new()` and stored as `self.config`
- `self.config` is immutable throughout the discussion (no mutation path exists)
- The orchestrator reads `self.config.arbitre.turn_distribution` at each turn (line 210)
- There is no `EngineCommand` variant to change the turn distribution

The turn distribution is set at discussion creation time and cannot be changed mid-discussion. This is correct and safe.

### Recommendation
None needed. This is already correctly handled by the immutable config design.

---

## Summary Table

| # | Scenario | Severity | Status |
|---|----------|----------|--------|
| 1 | 2 gladiators, democratic mode | Moderate | Guaranteed tie every turn, wasteful tiebreaker calls, potential bias |
| 2 | All vote for same person | Low | Works, but tiebreaker lacks context |
| 3 | Name variations | Critical | Missing-article case fails; `eq_ignore_ascii_case` insufficient for French |
| 4 | Completely wrong names | Moderate | Silently produces 0-point votes; should detect and fall back |
| 5 | All voting calls fail | Moderate | Fallback granularity unspecified (partial vs all-or-nothing) |
| 6 | Authoritarian partial list | Low | Handled, but missing-participant ordering unspecified |
| 7 | Force-stop during voting | Critical | Cancellation works but may emit spurious TurnStarted event |
| 8 | Only 1 active gladiator | Critical | Unnecessary LLM calls; edge case with empty vote candidates |
| 9 | Banned gladiator in votes | Critical | Plan does not specify whether banned names are excluded from prompts/resolution |
| 10 | Mixed mode mid-discussion | Safe | Immutable config prevents this |

---

## Top-Priority Fixes for the Plan

### Fix 1: Early return for <= 1 active gladiators (Scenarios 8, 9)
```rust
// At the top of determine_speaker_order_llm:
let active_indices: Vec<usize> = gladiateurs.iter().enumerate()
    .filter(|(_, g)| !g.is_banned())
    .map(|(i, _)| i)
    .collect();
if active_indices.len() <= 1 {
    return active_indices;
}
```

### Fix 2: Only use active gladiator names in all prompts (Scenarios 8, 9)
The vote prompt and authoritarian prompt must receive ONLY the names of active (non-banned) gladiators. Name resolution must also only consider active gladiators.

### Fix 3: Robust name matching (Scenario 3)
Replace `eq_ignore_ascii_case` with the same pattern used in `validate_reactions`:
```rust
// 1. Exact case-insensitive match via to_lowercase()
// 2. Prefix fallback (min 3 chars): known_name.starts_with(llm_name)
// 3. NEW: contains fallback (min 5 chars): known_name.contains(llm_name) || llm_name.contains(known_name)
```

### Fix 4: Add cancellation check after voting (Scenario 7)
```rust
let order = match &self.config.arbitre.turn_distribution {
    // ... Democratic/Authoritarian
};
// Check cancellation/commands AFTER potentially long async voting
if self.cancel_token.is_cancelled() { break; }
if self.process_commands(&mut cmd_rx, &channel).await { break; }
// THEN emit TurnStarted
```

### Fix 5: Detect zero-information votes (Scenario 4)
If all votes resolve to 0 recognized names, skip the tiebreaker and fall back to sequential immediately.

### Fix 6: Document N=2 limitation (Scenario 1)
Either add special handling for N=2 (random fallback) or document that Democratic mode is designed for 3+ gladiators and degrades to "IArbitre decides" with 2.

### Fix 7: Specify fallback granularity (Scenario 5)
Explicitly state: "If ALL vote calls fail, fall back to sequential. If at least 1 vote succeeds, use partial results for Borda count."

### Fix 8: Specify missing-participant ordering (Scenario 6)
"Participants not named by IArbitre in Authoritarian mode are appended in sequential order (by intervention_number)."
