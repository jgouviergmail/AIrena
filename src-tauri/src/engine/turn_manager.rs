use rand::seq::SliceRandom;

use crate::models::discussion::TurnDistribution;
use crate::models::gladiateur::GladIAteurState;

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
        });
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
