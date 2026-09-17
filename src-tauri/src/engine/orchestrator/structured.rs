//! Structured modes (v1.19), long memory (v1.20) and hidden agendas (v1.19).

use super::*;

impl DiscussionEngine {
    // ===== Structured modes: roles, dispatches, votes, outcomes (v1.19) =====

    /// Modes where the participants play a role or wear a hat.
    pub(super) fn mode_has_roles(mode: &DiscussionMode) -> bool {
        matches!(mode, DiscussionMode::Trial | DiscussionMode::OxfordDebate | DiscussionMode::SixHats)
    }

    /// Roles of a trial or an Oxford debate: the configured ones when valid, dealt from the casting order otherwise.
    pub(super) fn deal_fixed_roles(config: &DiscussionConfig) -> HashMap<String, String> {
        config
            .gladiateurs
            .iter()
            .enumerate()
            .filter_map(|(i, g)| mode_roles::resolve_role(&config.discussion_mode, g.mode_role.as_deref(), i).map(|r| (g.id.clone(), r.to_string())))
            .collect()
    }

    /// The hats rotate every turn; the fixed roles are announced once, on turn 1.
    pub(super) fn refresh_roles(&mut self, channel: &Channel<ArenaEvent>) {
        if self.config.discussion_mode == DiscussionMode::SixHats {
            self.roles = self.gladiateurs.iter().enumerate().map(|(i, g)| (g.config.id.clone(), mode_roles::hat_for(self.current_turn, i).to_string())).collect();
        } else if self.current_turn != 1 || self.roles.is_empty() {
            return;
        }
        let lang = &self.config.discussion_language;
        let roles: Vec<RoleAssignment> = self
            .gladiateurs
            .iter()
            .filter_map(|g| {
                let role = self.roles.get(&g.config.id)?;
                Some(RoleAssignment { speaker_id: g.config.id.clone(), role: role.clone(), label: mode_roles::role_label(role, lang).to_string() })
            })
            .collect();
        tracing::info!(turn = self.current_turn, ?roles, "Roles dealt");
        let _ = channel.send(ArenaEvent::RolesAssigned { turn: self.current_turn, roles });
    }

    /// The persona, followed by its memories of past discussions (v1.20) and
    /// its role or hat block in the structured modes.
    pub(super) fn system_prompt_for(&self, glad_idx: usize) -> String {
        let g = &self.gladiateurs[glad_idx].config;
        let mut system = g.system_prompt.clone();
        if let Some(memories) = self.memory_blocks.get(&g.id) {
            system.push_str("\n\n");
            system.push_str(memories);
        }
        if let Some(cast) = self.cast_blocks.get(&g.id).filter(|c| !c.is_empty()) {
            system.push_str("\n\n");
            system.push_str(cast);
        }
        if let Some(role) = self.roles.get(&g.id) {
            system.push_str("\n\n");
            system.push_str(&mode_roles::role_block(&self.config.discussion_mode, role, &self.config.discussion_language));
        }
        system
    }

    // ===== Long memory (v1.20) =====

    /// Recall the closest past recaps of every speaker built from a catalogue profile.
    pub(super) async fn recall_memories(&mut self) {
        if !self.persona_memory_enabled {
            return;
        }
        let lang = self.config.discussion_language.clone();
        let topic = self.config.topic.clone();
        let profiles: Vec<(String, String)> = self
            .gladiateurs
            .iter()
            .filter_map(|g| g.config.source_profile_id.clone().map(|p| (g.config.id.clone(), p)))
            .collect();
        for (speaker_id, profile_id) in profiles {
            match repository::recall_persona_memories(&self.db, &profile_id, &topic, constants::PERSONA_MEMORY_MAX_RECAPS).await {
                Ok(memories) if !memories.is_empty() => {
                    tracing::info!(speaker = %speaker_id, profile = %profile_id, recalled = memories.len(), "Persona memories recalled");
                    self.memory_blocks.insert(speaker_id, prompt_builder::build_memories_block(&memories, &lang));
                }
                Ok(_) => {}
                Err(e) => tracing::warn!(speaker = %speaker_id, error = %e, "Persona memories unavailable"),
            }
        }
    }

    /// One `Recap` call per speaker built from a profile, together; each usable
    /// answer is emitted as `PersonaRecapReady` (the frontend persists it with
    /// the discussion so the row cascades with it).
    pub(super) async fn write_recaps(&mut self, channel: &Channel<ArenaEvent>) {
        // A hard stop wants the end now, an exhausted budget forbids more billed calls: no recap
        if !self.persona_memory_enabled || self.llm.has_fatal_error() || self.status == DiscussionStatus::ForceStopRequested || self.budget_exceeded_sent {
            return;
        }
        let lang = self.config.discussion_language.clone();
        let known = self.known_participant_names();
        let edges = self.relationship_scores.edges(&self.tuning);
        let name_of = |id: &str| self.gladiateurs.iter().find(|g| g.config.id == id).map(|g| g.config.name.clone());
        let requests: Vec<(usize, LlmRequest)> = (0..self.gladiateurs.len())
            .filter(|&i| self.gladiateurs[i].config.source_profile_id.is_some())
            .map(|i| {
                let g = &self.gladiateurs[i].config;
                let own: Vec<String> = self
                    .messages_history
                    .iter()
                    .rev()
                    .filter(|m| m.speaker_id == g.id && m.kind == MessageKind::Normal)
                    .take(constants::RECAP_OWN_MESSAGES_MAX)
                    .map(|m| truncate_at_word_boundary(&m.content, constants::RECAP_OWN_MESSAGE_MAX_CHARS))
                    .collect();
                let mut allies = Vec::new();
                let mut rivals = Vec::new();
                for e in edges.iter().filter(|e| e.a == g.id || e.b == g.id) {
                    let other = if e.a == g.id { &e.b } else { &e.a };
                    match (e.kind.as_deref(), name_of(other)) {
                        (Some("ally"), Some(n)) => allies.push(n),
                        (Some("rival"), Some(n)) => rivals.push(n),
                        _ => {}
                    }
                }
                let input = RecapInput { speaker_name: &g.name, topic: &self.config.topic, summary: &self.arbitre.memory.contextual_summary, own_messages: &own, allies: &allies, rivals: &rivals };
                let prompt = prompt_builder::build_recap_prompt(&input, &lang);
                let mut params = g.llm_params.clone();
                params.num_predict = constants::RECAP_NUM_PREDICT;
                (i, LlmRequest::new(&g.system_prompt, &prompt, &params, CallKind::Recap).json().speaker(&g.id))
            })
            .collect();
        if requests.is_empty() {
            return;
        }
        let limit = self.llm.capabilities().max_parallel_calls.max(1);
        let futures: Vec<_> = requests
            .into_iter()
            .map(|(i, request)| {
                let llm = Arc::clone(&self.llm);
                let cancel = self.cancel_token.clone();
                async move { (i, llm.chat(&request, cancel).await.map(|r| r.content)) }
            })
            .collect();
        for (i, raw) in run_bounded(futures, limit).await {
            let g = &self.gladiateurs[i].config;
            match raw.ok().and_then(|r| json_parser::parse_recap(&r, &known)) {
                Some(recap) => {
                    let _ = channel.send(ArenaEvent::PersonaRecapReady {
                        recap: PersonaRecapRecord { speaker_id: g.id.clone(), speaker_name: g.name.clone(), profile_id: g.source_profile_id.clone().unwrap_or_default(), recap },
                    });
                }
                None => {
                    self.diagnostics.note_parse_failure(CallKind::Recap);
                    tracing::warn!(speaker = %g.name, "Recap unusable — nothing remembered");
                }
            }
        }
    }

    /// The gladiateurs playing `role` (casting order).
    pub(super) fn speakers_with_role(&self, role: &str) -> Vec<usize> {
        (0..self.gladiateurs.len()).filter(|i| self.roles.get(&self.gladiateurs[*i].config.id).is_some_and(|r| r == role)).collect()
    }

    /// Crisis cell: one JSON call writes the dispatches (one per turn) before the first turn.
    pub(super) async fn generate_crisis_dispatches(&mut self) {
        if self.config.discussion_mode != DiscussionMode::CrisisCell {
            return;
        }
        let count = self.config.max_turns.unwrap_or(constants::CRISIS_DISPATCH_DEFAULT_COUNT).clamp(1, constants::CRISIS_DISPATCH_MAX_COUNT);
        let (system, user) = prompt_builder::build_dispatches_prompt(&self.config.topic, count, &self.config.discussion_language);
        let mut params = self.arbitre.config.llm_params.clone();
        params.num_predict = constants::CRISIS_DISPATCHES_NUM_PREDICT;
        let request = LlmRequest::new(&system, &user, &params, CallKind::CrisisDispatches).json().speaker(&self.arbitre.config.id);
        match self.chat_text(&request).await {
            Ok(raw) => {
                let dispatches = json_parser::parse_dispatches(&raw, count as usize);
                if dispatches.is_empty() {
                    self.diagnostics.note_parse_failure(CallKind::CrisisDispatches);
                    tracing::warn!(preview = %truncate_str(raw.trim(), 120), "Dispatches answer unusable — the crisis cell runs without dispatches");
                }
                tracing::info!(count = dispatches.len(), "Crisis dispatches ready");
                self.crisis_dispatches = dispatches.into();
            }
            Err(LlmError::Cancelled) => {}
            Err(e) => tracing::warn!(error = %e, "Dispatches call failed — the crisis cell runs without dispatches"),
        }
    }

    /// The dispatch of this turn, as a scene event (none once the list is exhausted).
    pub(super) fn next_dispatch_event(&mut self) -> Option<SceneEvent> {
        let total = self.crisis_dispatches.len() as u32 + self.current_turn.saturating_sub(1);
        let text = self.crisis_dispatches.pop_front()?;
        Some(SceneEvent::Dispatch { text, index: self.current_turn, total })
    }

    /// Oxford debate: ask the audience to vote and wait for it (or the user
    /// turn timeout); other commands keep being served meanwhile.
    pub(super) async fn collect_audience_vote(&mut self, phase: VotePhase, cmd_rx: &mut mpsc::Receiver<EngineCommand>, channel: &Channel<ArenaEvent>) {
        if self.config.discussion_mode != DiscussionMode::OxfordDebate || self.cancel_token.is_cancelled() {
            return;
        }
        let timeout_secs = self.config.user_intervention_timeout_secs;
        tracing::info!(?phase, timeout_secs, "Audience vote requested");
        let _ = channel.send(ArenaEvent::AudienceVoteRequested { phase, timeout_secs });
        let timeout = tokio::time::sleep(Duration::from_secs(timeout_secs));
        tokio::pin!(timeout);
        loop {
            tokio::select! {
                cmd = cmd_rx.recv() => match cmd {
                    Some(EngineCommand::AudienceVote { choice }) => {
                        let choice = choice.trim().to_lowercase();
                        if choice != VOTE_FOR && choice != VOTE_AGAINST {
                            tracing::warn!(choice, "Audience vote ignored: unknown choice");
                            continue;
                        }
                        match phase {
                            VotePhase::Before => self.vote_before = Some(choice.clone()),
                            VotePhase::After => self.vote_after = Some(choice.clone()),
                        }
                        tracing::info!(?phase, choice, "Audience vote recorded");
                        let _ = channel.send(ArenaEvent::AudienceVoteRecorded { phase, choice });
                        return;
                    }
                    // The audience declines to vote: the window closes at once
                    Some(EngineCommand::SkipUserTurn) => {
                        tracing::info!(?phase, "Audience vote declined");
                        return;
                    }
                    Some(EngineCommand::ForceStop) => {
                        self.status = DiscussionStatus::ForceStopRequested;
                        return;
                    }
                    Some(EngineCommand::Stop) => {
                        self.status = DiscussionStatus::StopRequested;
                        return;
                    }
                    Some(EngineCommand::AdjustEmotion { speaker_id, axis, value }) => self.handle_adjust_emotion(&speaker_id, &axis, value, channel),
                    Some(EngineCommand::AudienceReaction { message_id, reaction_type }) => self.handle_audience_reaction(&message_id, reaction_type, channel),
                    Some(EngineCommand::UserWantsToIntervene) => self.user_intervention_pending = true,
                    Some(_) => {}
                    None => return,
                },
                _ = &mut timeout => {
                    tracing::info!(?phase, "Audience vote: no vote before the timeout");
                    return;
                }
            }
        }
    }

    /// The mode's result: the jurors' verdict (or the moderator's), the
    /// parties' decisions, the audience swing. `None` for the other modes.
    pub(super) async fn resolve_outcome(&mut self) -> Option<ModeOutcome> {
        match self.config.discussion_mode {
            DiscussionMode::Trial => Some(self.resolve_verdict().await),
            DiscussionMode::Negotiation => Some(self.resolve_agreement().await),
            DiscussionMode::OxfordDebate => Some(ModeOutcome::AudienceSwing {
                before: self.vote_before.clone(),
                after: self.vote_after.clone(),
                winner: ModeOutcome::swing_winner(self.vote_before.as_deref(), self.vote_after.as_deref()),
            }),
            _ => None,
        }
    }

    /// One `Verdict` call per juror, together; the moderator returns it alone
    /// when no juror sat (or none answered usably) and breaks a tie.
    pub(super) async fn resolve_verdict(&mut self) -> ModeOutcome {
        let lang = self.config.discussion_language.clone();
        let jurors = self.speakers_with_role(mode_roles::ROLE_JUROR);
        let mut votes: Vec<VerdictVote> = if jurors.is_empty() {
            Vec::new()
        } else {
            let prompt = prompt_builder::build_verdict_prompt(&self.config.topic, &self.arbitre.memory, &lang, false);
            let requests: Vec<(usize, LlmRequest)> = jurors
                .iter()
                .map(|&i| {
                    let g = &self.gladiateurs[i].config;
                    let mut params = g.llm_params.clone();
                    params.num_predict = constants::VERDICT_NUM_PREDICT;
                    (i, LlmRequest::new(&self.system_prompt_for(i), &prompt, &params, CallKind::Verdict).json().speaker(&g.id))
                })
                .collect();
            let limit = self.llm.capabilities().max_parallel_calls.max(1);
            let futures: Vec<_> = requests
                .into_iter()
                .map(|(i, request)| {
                    let llm = Arc::clone(&self.llm);
                    let cancel = self.cancel_token.clone();
                    async move { (i, llm.chat(&request, cancel).await.map(|r| r.content)) }
                })
                .collect();
            let mut votes = Vec::new();
            for (i, raw) in run_bounded(futures, limit).await {
                let g = &self.gladiateurs[i].config;
                match raw.ok().and_then(|r| json_parser::parse_verdict(&r)) {
                    Some((choice, reason)) => votes.push(VerdictVote { voter_id: g.id.clone(), voter_name: g.name.clone(), choice, reason }),
                    None => {
                        self.diagnostics.note_parse_failure(CallKind::Verdict);
                        tracing::warn!(juror = %g.name, "Verdict unusable — juror abstains");
                    }
                }
            }
            votes
        };
        let mut winner = ModeOutcome::majority(&votes);
        // No jury, a silent jury or a tie: the presiding moderator rules
        let by_arbitre = votes.is_empty();
        if winner.is_none() && !self.cancel_token.is_cancelled() {
            let prompt = prompt_builder::build_verdict_prompt(&self.config.topic, &self.arbitre.memory, &lang, true);
            let mut params = self.arbitre.config.llm_params.clone();
            params.num_predict = constants::VERDICT_NUM_PREDICT;
            let request = LlmRequest::new(&self.arbitre.config.system_prompt, &prompt, &params, CallKind::Verdict).json().speaker(&self.arbitre.config.id);
            match self.chat_text(&request).await.ok().and_then(|r| json_parser::parse_verdict(&r)) {
                Some((choice, reason)) => {
                    winner = Some(choice.clone());
                    votes.push(VerdictVote { voter_id: self.arbitre.config.id.clone(), voter_name: self.arbitre.config.name.clone(), choice, reason });
                }
                None => {
                    self.diagnostics.note_parse_failure(CallKind::Verdict);
                    tracing::warn!("Moderator's verdict unusable — no verdict");
                }
            }
        }
        ModeOutcome::Verdict { votes, winner, by_arbitre }
    }

    /// One `Verdict` call per party: does it sign the deal on the table?
    pub(super) async fn resolve_agreement(&mut self) -> ModeOutcome {
        let prompt = prompt_builder::build_agreement_prompt(&self.config.topic, &self.arbitre.memory, &self.config.discussion_language);
        let requests: Vec<(usize, LlmRequest)> = (0..self.gladiateurs.len())
            .map(|i| {
                let g = &self.gladiateurs[i].config;
                let mut params = g.llm_params.clone();
                params.num_predict = constants::VERDICT_NUM_PREDICT;
                (i, LlmRequest::new(&self.system_prompt_for(i), &prompt, &params, CallKind::Verdict).json().speaker(&g.id))
            })
            .collect();
        let limit = self.llm.capabilities().max_parallel_calls.max(1);
        let futures: Vec<_> = requests
            .into_iter()
            .map(|(i, request)| {
                let llm = Arc::clone(&self.llm);
                let cancel = self.cancel_token.clone();
                async move { (i, llm.chat(&request, cancel).await.map(|r| r.content)) }
            })
            .collect();
        let mut parties = Vec::new();
        for (i, raw) in run_bounded(futures, limit).await {
            let g = &self.gladiateurs[i].config;
            match raw.ok().and_then(|r| json_parser::parse_agreement(&r)) {
                Some((accepts, reason)) => parties.push(PartyDecision { party_id: g.id.clone(), party_name: g.name.clone(), accepts, reason }),
                None => {
                    self.diagnostics.note_parse_failure(CallKind::Verdict);
                    tracing::warn!(party = %g.name, "Agreement answer unusable — counted as a refusal");
                    parties.push(PartyDecision { party_id: g.id.clone(), party_name: g.name.clone(), accepts: false, reason: String::new() });
                }
            }
        }
        let reached = !parties.is_empty() && parties.iter().all(|p| p.accepts);
        ModeOutcome::Agreement { parties, reached }
    }

    // ===== Hidden agendas (v1.19) =====

    /// Agendas need the feature on and a mode where a secret objective makes
    /// sense — a negotiation always has them: parties without secret interests
    /// have nothing to negotiate.
    pub(super) fn agendas_enabled_for(config: &DiscussionConfig) -> bool {
        (config.features.hidden_agenda || config.discussion_mode == DiscussionMode::Negotiation) && config.discussion_mode.supports_hidden_agenda()
    }

    /// One JSON call per gladiateur, run together (bounded by the provider's
    /// parallelism). A failed, cancelled or empty answer leaves that speaker
    /// without agenda — the discussion goes on unchanged.
    pub(super) async fn generate_agendas(&mut self) {
        if !Self::agendas_enabled_for(&self.config) {
            return;
        }
        let lang = self.config.discussion_language.as_str();
        let names: Vec<String> = self.gladiateurs.iter().map(|g| g.config.name.clone()).collect();
        let requests: Vec<(String, LlmRequest)> = self
            .gladiateurs
            .iter()
            .map(|g| {
                let others: Vec<String> = names.iter().filter(|n| **n != g.config.name).cloned().collect();
                let prompt = prompt_builder::build_agenda_prompt(&self.config.topic, &others, &self.config.discussion_mode, lang);
                let mut params = g.config.llm_params.clone();
                params.num_predict = constants::AGENDA_NUM_PREDICT;
                let request = LlmRequest::new(&g.config.system_prompt, &prompt, &params, CallKind::Agenda).json().speaker(&g.config.id);
                (g.config.id.clone(), request)
            })
            .collect();
        let limit = self.llm.capabilities().max_parallel_calls.max(1);
        let futures: Vec<_> = requests
            .into_iter()
            .map(|(id, request)| {
                let llm = Arc::clone(&self.llm);
                let cancel = self.cancel_token.clone();
                async move { (id, llm.chat(&request, cancel).await.map(|r| r.content)) }
            })
            .collect();
        let started = Instant::now();
        for (id, raw) in run_bounded(futures, limit).await {
            match raw {
                Ok(raw) => match json_parser::parse_agenda(&raw) {
                    Some(agenda) => {
                        tracing::debug!(speaker = %id, objective = %agenda.objective, "Hidden agenda set");
                        self.agendas.insert(id, agenda);
                    }
                    None => {
                        self.diagnostics.note_parse_failure(CallKind::Agenda);
                        tracing::warn!(speaker = %id, preview = %truncate_str(raw.trim(), 120), "Agenda answer unusable — no agenda for this speaker");
                    }
                },
                Err(LlmError::Cancelled) => {}
                Err(e) => tracing::warn!(speaker = %id, error = %e, "Agenda call failed — no agenda for this speaker"),
            }
        }
        tracing::info!(agendas = self.agendas.len(), gladiateurs = self.gladiateurs.len(), ms = started.elapsed().as_millis() as u64, "Hidden agendas ready");
    }

    /// The agendas in casting order, judged against the synthesis (an empty
    /// synthesis leaves every outcome unknown).
    pub(super) fn agenda_reveals(&self, synthesis: &str) -> Vec<AgendaReveal> {
        self.gladiateurs
            .iter()
            .filter_map(|g| {
                let agenda = self.agendas.get(&g.config.id)?.clone();
                let achieved = if synthesis.is_empty() { None } else { json_parser::agenda_outcome(synthesis, &g.config.name) };
                Some(AgendaReveal { speaker_id: g.config.id.clone(), speaker_name: g.config.name.clone(), agenda, achieved })
            })
            .collect()
    }
}
