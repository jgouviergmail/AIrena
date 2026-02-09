use std::collections::{HashMap, HashSet};

use rand::seq::SliceRandom;
use tokio_util::sync::CancellationToken;

use crate::engine::json_parser;
use crate::engine::prompt_builder;
use crate::models::discussion::TurnDistribution;
use crate::models::gladiateur::GladIAteurState;
use crate::models::settings::LlmParams;
use crate::ollama::client::OllamaClient;

/// Determine the speaking order for a turn
pub fn determine_speaker_order(
    gladiateurs: &[GladIAteurState],
    distribution: &TurnDistribution,
) -> Vec<usize> {
    let active_indices: Vec<usize> = gladiateurs
        .iter()
        .enumerate()
        .filter(|(_, g)| !g.is_banned())
        .map(|(i, _)| i)
        .collect();

    match distribution {
        TurnDistribution::Sequential => {
            let mut sorted = active_indices;
            sorted.sort_by_key(|&i| gladiateurs[i].config.intervention_number);
            sorted
        }
        TurnDistribution::Random => {
            let mut shuffled = active_indices;
            let mut rng = rand::thread_rng();
            shuffled.shuffle(&mut rng);
            shuffled
        }
        // Democratic/Authoritarian use async functions; sync fallback = sequential
        _ => {
            let mut sorted = active_indices;
            sorted.sort_by_key(|&i| gladiateurs[i].config.intervention_number);
            sorted
        }
    }
}

/// Count active (non-banned) gladiators
pub fn active_count(gladiateurs: &[GladIAteurState]) -> usize {
    gladiateurs.iter().filter(|g| !g.is_banned()).count()
}

/// Decrement ban counters and return IDs of gladiators whose bans were lifted
pub fn decrement_bans(gladiateurs: &mut [GladIAteurState]) -> Vec<(String, String)> {
    let mut lifted = Vec::new();

    for g in gladiateurs.iter_mut() {
        if g.ban_issued_this_turn {
            g.ban_issued_this_turn = false;
            continue;
        }
        if g.ban_remaining_turns > 0 {
            g.ban_remaining_turns -= 1;
            if g.ban_remaining_turns == 0 {
                lifted.push((g.config.id.clone(), g.config.name.clone()));
            }
        }
    }

    lifted
}

/// Context needed for async turn determination (democratic/authoritarian).
/// All fields are owned to avoid borrow checker issues across .await in the orchestrator.
pub struct AsyncTurnContext {
    pub ollama_client: OllamaClient,
    pub cancel_token: CancellationToken,
    pub arbitre_system_prompt: String,
    pub arbitre_llm_params: LlmParams,
    pub discussion_summary: String,
    pub topic: String,
    pub current_turn: u32,
    pub discussion_language: String,
}

/// Determine speaker order via democratic voting (Borda count).
/// Each active gladiator votes to rank the others. Ties are broken by the IArbitre.
/// Falls back to sequential order if LLM calls fail.
pub async fn determine_order_democratic(
    gladiateurs: &[GladIAteurState],
    ctx: &AsyncTurnContext,
) -> Vec<usize> {
    let active: Vec<(usize, String)> = gladiateurs
        .iter()
        .enumerate()
        .filter(|(_, g)| !g.is_banned())
        .map(|(i, g)| (i, g.config.name.clone()))
        .collect();

    // Early return for trivial cases
    if active.len() <= 1 {
        return active.iter().map(|(i, _)| *i).collect();
    }

    // N=2: guaranteed tie every time → delegate to authoritarian (1 LLM call vs 3)
    if active.len() == 2 {
        tracing::info!("Democratic mode with 2 participants: delegating to authoritarian");
        return determine_order_authoritarian(gladiateurs, ctx).await;
    }

    let active_names: Vec<String> = active.iter().map(|(_, n)| n.clone()).collect();

    // Build vote futures for all active gladiators in parallel
    let vote_futures: Vec<_> = active
        .iter()
        .map(|(idx, voter_name)| {
            let other_names: Vec<String> = active_names
                .iter()
                .filter(|n| n.as_str() != voter_name.as_str())
                .cloned()
                .collect();

            let prompt = prompt_builder::build_democratic_vote_prompt(
                voter_name,
                &other_names,
                &ctx.topic,
                &ctx.discussion_summary,
                &ctx.discussion_language,
            );
            let request = ctx.ollama_client.build_request(
                &gladiateurs[*idx].config.system_prompt,
                &prompt,
                &gladiateurs[*idx].config.llm_params,
                true, // json_format
            );
            let client = ctx.ollama_client.clone();
            let cancel = ctx.cancel_token.clone();

            async move {
                match client.chat(&request, cancel).await {
                    Ok(raw) => json_parser::parse_vote(&raw),
                    Err(e) => {
                        tracing::warn!("Democratic vote failed: {e}");
                        Vec::new()
                    }
                }
            }
        })
        .collect();

    // Execute all votes in parallel
    let votes: Vec<Vec<String>> = futures_util::future::join_all(vote_futures).await;

    // Tally votes using Borda count: 1st place = N-1 points, 2nd = N-2, etc.
    let n = active_names.len();
    let mut scores: HashMap<String, usize> = HashMap::new();
    for name in &active_names {
        scores.insert(name.clone(), 0);
    }

    let mut total_recognized = 0usize;
    for ranking in &votes {
        let mut seen = HashSet::new();
        for (pos, llm_name) in ranking.iter().enumerate() {
            if let Some(matched) = json_parser::match_speaker_name(llm_name, &active_names) {
                // Deduplicate: ignore repeated names within a single voter's ranking
                if seen.insert(matched.clone()) {
                    let points = n.saturating_sub(1).saturating_sub(pos);
                    *scores.entry(matched.clone()).or_default() += points;
                    total_recognized += 1;
                }
            }
        }
    }

    // Zero-information detection: if no votes were recognized, fall back immediately
    if total_recognized == 0 {
        tracing::warn!("Democratic voting: zero recognized votes, falling back to sequential");
        return determine_speaker_order(gladiateurs, &TurnDistribution::Sequential);
    }

    // Sort active indices by score (descending)
    let mut scored_indices: Vec<(usize, usize)> = active
        .iter()
        .map(|(idx, name)| {
            let score = scores.get(name).copied().unwrap_or(0);
            (*idx, score)
        })
        .collect();
    scored_indices.sort_by(|a, b| b.1.cmp(&a.1));

    // Group by score to detect ties
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut current_score = usize::MAX;
    for (idx, score) in &scored_indices {
        if *score != current_score {
            groups.push(vec![*idx]);
            current_score = *score;
        } else if let Some(last) = groups.last_mut() {
            last.push(*idx);
        }
    }

    // Build final order, breaking ties via IArbitre
    let mut final_order: Vec<usize> = Vec::new();
    for group in groups {
        if group.len() == 1 {
            final_order.push(group[0]);
        } else {
            // Tie-breaking: ask IArbitre to order the tied participants
            let tied_names: Vec<String> = group
                .iter()
                .map(|i| gladiateurs[*i].config.name.clone())
                .collect();

            let prompt = prompt_builder::build_tiebreak_prompt(
                &tied_names,
                &ctx.topic,
                &ctx.discussion_summary,
                ctx.current_turn,
                &ctx.discussion_language,
            );
            let request = ctx.ollama_client.build_request(
                &ctx.arbitre_system_prompt,
                &prompt,
                &ctx.arbitre_llm_params,
                true,
            );
            let tiebreak_order = match ctx
                .ollama_client
                .chat(&request, ctx.cancel_token.clone())
                .await
            {
                Ok(raw) => json_parser::parse_authoritarian_order(&raw),
                Err(e) => {
                    tracing::warn!("Tiebreak failed: {e}");
                    Vec::new()
                }
            };

            // Map tiebreak names back to indices
            let mut resolved: Vec<usize> = Vec::new();
            for name in &tiebreak_order {
                if let Some(matched) = json_parser::match_speaker_name(name, &tied_names) {
                    if let Some(idx) = group
                        .iter()
                        .find(|i| gladiateurs[**i].config.name == *matched)
                    {
                        if !resolved.contains(idx) {
                            resolved.push(*idx);
                        }
                    }
                }
            }
            // Append any remaining tied indices not resolved by the tiebreak
            for idx in &group {
                if !resolved.contains(idx) {
                    resolved.push(*idx);
                }
            }
            final_order.extend(resolved);
        }
    }

    if final_order.is_empty() {
        tracing::warn!("Democratic voting produced empty order, falling back to sequential");
        return determine_speaker_order(gladiateurs, &TurnDistribution::Sequential);
    }

    final_order
}

/// Determine speaker order via IArbitre decision.
/// The IArbitre decides the full speaking order based on discussion context.
/// Falls back to sequential order if the LLM call fails.
pub async fn determine_order_authoritarian(
    gladiateurs: &[GladIAteurState],
    ctx: &AsyncTurnContext,
) -> Vec<usize> {
    let active: Vec<(usize, String)> = gladiateurs
        .iter()
        .enumerate()
        .filter(|(_, g)| !g.is_banned())
        .map(|(i, g)| (i, g.config.name.clone()))
        .collect();

    // Early return for trivial cases
    if active.len() <= 1 {
        return active.iter().map(|(i, _)| *i).collect();
    }

    let active_names: Vec<String> = active.iter().map(|(_, n)| n.clone()).collect();

    let prompt = prompt_builder::build_authoritarian_order_prompt(
        &active_names,
        &ctx.topic,
        &ctx.discussion_summary,
        ctx.current_turn,
        &ctx.discussion_language,
    );
    let request = ctx.ollama_client.build_request(
        &ctx.arbitre_system_prompt,
        &prompt,
        &ctx.arbitre_llm_params,
        true, // json_format
    );

    let ordered_names = match ctx
        .ollama_client
        .chat(&request, ctx.cancel_token.clone())
        .await
    {
        Ok(raw) => json_parser::parse_authoritarian_order(&raw),
        Err(e) => {
            tracing::warn!("Authoritarian ordering failed: {e}, falling back to sequential");
            return determine_speaker_order(gladiateurs, &TurnDistribution::Sequential);
        }
    };

    if ordered_names.is_empty() {
        tracing::warn!("Authoritarian ordering returned empty, falling back to sequential");
        return determine_speaker_order(gladiateurs, &TurnDistribution::Sequential);
    }

    // Map names back to indices
    let mut result: Vec<usize> = Vec::new();
    for name in &ordered_names {
        if let Some(matched) = json_parser::match_speaker_name(name, &active_names) {
            if let Some((idx, _)) = active.iter().find(|(_, n)| n == matched) {
                if !result.contains(idx) {
                    result.push(*idx);
                }
            }
        }
    }

    // Safety net: append any active indices not mentioned by the LLM (by intervention_number)
    let mut missing: Vec<usize> = active
        .iter()
        .filter(|(idx, _)| !result.contains(idx))
        .map(|(idx, _)| *idx)
        .collect();
    missing.sort_by_key(|&i| gladiateurs[i].config.intervention_number);
    result.extend(missing);

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::gladiateur::GladIAteurConfig;
    use crate::models::settings::LlmParams;

    fn all_banned(gladiateurs: &[GladIAteurState]) -> bool {
        gladiateurs.iter().all(|g| g.is_banned())
    }

    fn make_gladiateur(id: &str, number: u32, banned: u32) -> GladIAteurState {
        let mut state = GladIAteurState::new(GladIAteurConfig {
            id: id.to_string(),
            name: id.to_string(),
            intervention_number: number,
            system_prompt: String::new(),
            llm_params: LlmParams::default(),
            emoji: None,
            initial_emotions: None,
        }, None);
        state.ban_remaining_turns = banned;
        state
    }

    #[test]
    fn test_sequential_order_excludes_banned() {
        let gladiateurs = vec![
            make_gladiateur("A", 3, 0),
            make_gladiateur("B", 1, 2), // banned
            make_gladiateur("C", 2, 0),
        ];
        let order = determine_speaker_order(&gladiateurs, &TurnDistribution::Sequential);
        assert_eq!(order, vec![2, 0]); // C(2), A(3) — B is banned
    }

    #[test]
    fn test_all_banned() {
        let gladiateurs = vec![make_gladiateur("A", 1, 1), make_gladiateur("B", 2, 3)];
        assert!(all_banned(&gladiateurs));
    }

    #[test]
    fn test_decrement_bans_lifts_correctly() {
        let mut gladiateurs = vec![
            make_gladiateur("A", 1, 1), // will be lifted
            make_gladiateur("B", 2, 3), // still banned
        ];
        let lifted = decrement_bans(&mut gladiateurs);
        assert_eq!(lifted.len(), 1);
        assert_eq!(lifted[0].0, "A");
        assert_eq!(gladiateurs[0].ban_remaining_turns, 0);
        assert_eq!(gladiateurs[1].ban_remaining_turns, 2);
    }

    #[test]
    fn test_decrement_skips_ban_issued_this_turn() {
        let mut gladiateurs = vec![make_gladiateur("A", 1, 2)];
        gladiateurs[0].ban_issued_this_turn = true;
        let lifted = decrement_bans(&mut gladiateurs);
        assert!(lifted.is_empty());
        assert_eq!(gladiateurs[0].ban_remaining_turns, 2); // unchanged
        assert!(!gladiateurs[0].ban_issued_this_turn); // reset
    }
}
