//! Dramaturgy (v1.18): acts, scene events, coalitions, stage blocks — one `impl DiscussionEngine` block.

use super::*;

impl DiscussionEngine {
    // ── Dramaturgy (v1.18) ────────────────────────────────────────────

    /// Stage the turn: announce a new act, maybe break the routine with a scene
    /// event, maybe let two allies relay each other. Returns the (possibly
    /// reordered or shortened) speaker order.
    pub(super) async fn stage_turn(&mut self, order: Vec<usize>, channel: &Channel<ArenaEvent>) -> Vec<usize> {
        self.turn_scene_event = None;
        self.turn_coalition = None;
        let lang = self.config.discussion_language.clone();

        // Act of the turn
        let position = TurnPosition {
            turn: self.current_turn,
            max_turns: self.config.max_turns,
            stagnating: self.is_stagnating(),
            stop_requested: self.status == DiscussionStatus::StopRequested,
        };
        if let Some(act) = dramaturgy::resolve_act(&self.config.discussion_mode, position) {
            if self.current_act != Some(act) {
                self.current_act = Some(act);
                tracing::info!(turn = self.current_turn, act = ?act, "Act started");
                let _ = channel.send(ArenaEvent::ActStarted { turn: self.current_turn, act, title: act.title(&lang).to_string() });
                let line = self.voice_announcement(act.announcement(&lang)).await;
                self.emit_arbitre_line(MessageKind::ActAnnouncement, &line, channel);
            }
        }

        // Roles and hats of the turn (v1.19)
        self.refresh_roles(channel);

        // Scene event — a crisis cell lives on its dispatches instead of the random policy
        let mut order = order;
        if self.config.discussion_mode == DiscussionMode::CrisisCell {
            if let Some(event) = self.next_dispatch_event() {
                self.announce_scene_event(event, channel).await;
            }
        } else if self.config.features.scene_events {
            if let Some(kind) = self.draw_scene_event(order.len()) {
                order = self.materialise_scene_event(kind, order, channel).await;
            }
        }

        // Coalition (never on top of a scene event: one twist per turn)
        if self.turn_scene_event.is_none() && self.config.features.coalitions {
            order = self.try_coalition(order, channel);
        }
        order
    }

    /// A soft stop requested during the turn makes the remaining speakers close:
    /// the closing act is announced at once (v1.18).
    pub(super) async fn refresh_act_on_stop(&mut self, channel: &Channel<ArenaEvent>) {
        if self.status != DiscussionStatus::StopRequested {
            return;
        }
        let Some(closing) = dramaturgy::mode_script(&self.config.discussion_mode).last().map(|a| a.key) else { return };
        if self.current_act != Some(closing) {
            self.current_act = Some(closing);
            let lang = self.config.discussion_language.clone();
            tracing::info!(turn = self.current_turn, act = ?closing, "Closing act (stop requested)");
            let _ = channel.send(ArenaEvent::ActStarted { turn: self.current_turn, act: closing, title: closing.title(&lang).to_string() });
            let line = self.voice_announcement(closing.announcement(&lang)).await;
            self.emit_arbitre_line(MessageKind::ActAnnouncement, &line, channel);
        }
    }

    /// The act and scene instructions of a speaker, or `None` when the turn is plain.
    pub(super) fn stage_block_for(&self, speaker_name: &str) -> Option<StageBlock> {
        let lang = &self.config.discussion_language;
        let mut lines: Vec<String> = Vec::new();
        let mut closing = false;
        if let Some(act) = self.current_act {
            lines.push(act.speaker_instruction(lang).to_string());
            closing = dramaturgy::mode_script(&self.config.discussion_mode).last().is_some_and(|last| last.key == act);
        }
        if let Some(instruction) = self.turn_scene_event.as_ref().and_then(|e| e.speaker_instruction(speaker_name, lang)) {
            lines.push(instruction);
        }
        (!lines.is_empty()).then(|| StageBlock { text: lines.join("\n"), closing })
    }

    /// Scene-event policy for this turn (test hook first).
    pub(super) fn draw_scene_event(&mut self, active_count: usize) -> Option<SceneEventKind> {
        #[cfg(test)]
        if let Some(kind) = self.forced_scene_event.take() {
            return Some(kind);
        }
        #[cfg(test)]
        if self.random_staging_disabled {
            return None;
        }
        let ctx = SceneContext {
            mode: self.config.discussion_mode.clone(),
            turn: self.current_turn,
            max_turns: self.config.max_turns,
            stop_requested: self.status == DiscussionStatus::StopRequested,
            last_event_turn: self.last_scene_event_turn,
            stagnating: self.is_stagnating(),
            search_available: self.surprise_fact_available(),
            audience_enabled: self.config.features.audience_reactions,
            active_count,
            base_probability: self.tuning.scene_event_base_probability,
            stagnation_boost: self.tuning.scene_event_stagnation_boost,
            used_kinds: self.scene_kinds_used.clone(),
        };
        scene_events::pick_scene_event(&mut rand::thread_rng(), &ctx)
    }

    /// A surprise fact needs a knowledge source with quota left (Wikipedia or web) or imported documents.
    pub(super) fn surprise_fact_available(&self) -> bool {
        self.can_search_wiki().0
            || (self.tavily_client.is_some() && self.config.web_search_pool.saturating_sub(self.web_searches_used_pool) > 0)
            || self.rag_store.as_ref().is_some_and(|s| !s.is_empty())
    }

    /// Build the event (participants, fact…), announce it and apply its effects on the order.
    pub(super) async fn materialise_scene_event(&mut self, kind: SceneEventKind, order: Vec<usize>, channel: &Channel<ArenaEvent>) -> Vec<usize> {
        let name_of = |i: usize, glads: &[GladIAteurState]| glads[i].config.name.clone();
        let (event, new_order): (SceneEvent, Vec<usize>) = match kind {
            SceneEventKind::SurpriseFact => match self.fetch_surprise_fact(channel).await {
                Some((fact, source)) => (SceneEvent::SurpriseFact { fact, source }, order),
                None => {
                    tracing::info!("Surprise fact unavailable — format constraint instead");
                    (SceneEvent::FormatConstraint { constraint: scene_events::draw_constraint(&mut rand::thread_rng()) }, order)
                }
            },
            SceneEventKind::FormatConstraint => (SceneEvent::FormatConstraint { constraint: scene_events::draw_constraint(&mut rand::thread_rng()) }, order),
            SceneEventKind::ForcedSteelman => (SceneEvent::ForcedSteelman, order),
            SceneEventKind::AudienceQuestion => {
                let Some(target) = self.audience_question_target(&order) else { return order };
                // No usable question from the exchanges: no event at all (never a generic one)
                let Some(question) = self.generate_audience_question(target).await else { return order };
                (SceneEvent::AudienceQuestion { target: name_of(target, &self.gladiateurs), question }, order)
            }
            SceneEventKind::HotSeat => {
                let Some(target) = self.hot_seat_target(&order) else { return order };
                let mut reordered: Vec<usize> = order.iter().copied().filter(|i| *i != target).collect();
                reordered.push(target);
                (SceneEvent::HotSeat { target: name_of(target, &self.gladiateurs) }, reordered)
            }
            SceneEventKind::Duel => {
                let Some((a, b)) = self.duel_pair(&order) else {
                    tracing::info!("Duel impossible (fewer than three active speakers) — no scene event");
                    return order;
                };
                (SceneEvent::Duel { a: name_of(a, &self.gladiateurs), b: name_of(b, &self.gladiateurs) }, vec![a, b])
            }
        };
        self.scene_kinds_used.insert(kind);
        self.announce_scene_event(event, channel).await;
        new_order
    }

    /// The room's question to the speaker at `target` (v1.20.3): written by the
    /// moderator from the summary, the target's stance and what they still owe.
    pub(super) async fn generate_audience_question(&mut self, target: usize) -> Option<String> {
        let g = &self.gladiateurs[target].config;
        let lang = self.config.discussion_language.clone();
        let position = self.arbitre.memory.positional_map.get(&g.name).map(|p| p.stance.clone());
        let loops: Vec<String> = prompt_builder::format_open_loop_lines(&self.open_loops.for_speaker(&g.id), &lang);
        let input = prompt_builder::AudienceQuestionInput {
            topic: &self.config.topic,
            target: &g.name,
            summary: &self.arbitre.memory.contextual_summary,
            target_position: position.as_deref(),
            open_loops: &loops,
        };
        let (system, user) = prompt_builder::build_audience_question_prompt(&input, &lang);
        let mut params = self.arbitre.config.llm_params.clone();
        params.num_predict = constants::AUDIENCE_QUESTION_NUM_PREDICT;
        let request = LlmRequest::new(&system, &user, &params, CallKind::AudienceQuestion).json().speaker(&self.arbitre.config.id);
        match self.chat_text(&request).await {
            Ok(raw) => {
                let question = json_parser::parse_audience_question(&raw);
                if question.is_none() {
                    self.diagnostics.note_parse_failure(CallKind::AudienceQuestion);
                    tracing::warn!(preview = %truncate_str(raw.trim(), 120), "Audience question unusable — no scene event");
                }
                question
            }
            Err(LlmError::Cancelled) => None,
            Err(e) => {
                tracing::warn!(error = %e, "Audience question call failed — no scene event");
                None
            }
        }
    }

    /// Log, emit and install the event of the turn (the moderator announces it in
    /// the feed, in its own voice).
    pub(super) async fn announce_scene_event(&mut self, event: SceneEvent, channel: &Channel<ArenaEvent>) {
        let lang = self.config.discussion_language.clone();
        tracing::info!(turn = self.current_turn, event = ?event, "Scene event");
        self.last_scene_event_turn = Some(self.current_turn);
        let _ = channel.send(ArenaEvent::SceneEventTriggered { turn: self.current_turn, event: event.clone(), participants: event.participants() });
        let line = self.voice_announcement(&event.announcement(&lang)).await;
        self.emit_arbitre_line(MessageKind::SceneEvent, &line, channel);
        self.turn_scene_event = Some(event);
    }

    /// One fact about the topic from Wikipedia (free) or the web (quota attributed to the moderator).
    pub(super) async fn fetch_surprise_fact(&mut self, channel: &Channel<ArenaEvent>) -> Option<(String, Option<String>)> {
        let (arb_id, arb_name) = (self.arbitre.config.id.clone(), self.arbitre.config.name.clone());
        let query = truncate_str(&self.config.topic, constants::ORCH_TOPIC_FOR_SEARCH).to_string();
        if self.can_search_wiki().0 {
            let (_, articles) = self.process_wiki_search_intro(&query, &arb_id, &arb_name, channel).await;
            if let Some(a) = articles.into_iter().find(|a| !a.snippet.trim().is_empty()) {
                self.wiki_searches_used_pool += 1;
                return Some((truncate_at_sentence_boundary(&a.snippet, constants::SURPRISE_FACT_MAX_CHARS), Some(a.title)));
            }
        }
        if self.tavily_client.is_some() && self.config.web_search_pool.saturating_sub(self.web_searches_used_pool) > 0 {
            let global_usage = repository::get_tavily_usage(&self.db).await.unwrap_or(0);
            let (can, max_queries) = self.can_search_web(global_usage);
            if can {
                let params = self.arbitre.config.llm_params.clone();
                let (_, count, _, sources) = self
                    .process_web_search(&self.arbitre.config.system_prompt.clone(), &arb_id, &arb_name, max_queries, "", "", Some(vec![query]), &params, channel, &[], &[])
                    .await;
                self.web_searches_used_pool += count;
                self.register_sources(&arb_name, SourceKind::Web, sources.iter().map(|s| (s.title.clone(), s.url.clone())));
                if let Some(s) = sources.into_iter().find(|s| !s.snippet.trim().is_empty()) {
                    return Some((truncate_at_sentence_boundary(&s.snippet, constants::SURPRISE_FACT_MAX_CHARS), Some(s.domain)));
                }
            }
        }
        None
    }

    /// Hot seat: the speaker who drew the most reactions last time (the first of
    /// the order when nobody did); `None` on an empty order.
    pub(super) fn hot_seat_target(&self, order: &[usize]) -> Option<usize> {
        // `max_by_key` keeps the last maximum: reverse so ties favour the first of the order
        order.iter().rev().copied().max_by_key(|i| {
            let id = &self.gladiateurs[*i].config.id;
            self.last_reactions_received.get(id).map_or(0, |t| t.likes + t.dislikes + t.questions + t.laughs)
        })
    }

    /// Question from the room: the speaker with the most open loops, else the hot-seat candidate.
    pub(super) fn audience_question_target(&self, order: &[usize]) -> Option<usize> {
        let most_loops = order.iter().rev().copied().max_by_key(|i| self.open_loops.for_speaker(&self.gladiateurs[*i].config.id).len());
        match most_loops {
            Some(i) if !self.open_loops.for_speaker(&self.gladiateurs[i].config.id).is_empty() => Some(i),
            _ => self.hot_seat_target(order),
        }
    }

    /// Duel: the most tense pair of the order (rivals first), else the first two.
    pub(super) fn duel_pair(&self, order: &[usize]) -> Option<(usize, usize)> {
        if order.len() < constants::SCENE_EVENT_MIN_ACTIVE_FOR_DUEL {
            return None;
        }
        let mut best: Option<(usize, usize, u8)> = None;
        for (x, &a) in order.iter().enumerate() {
            for &b in &order[x + 1..] {
                let rank = match self.relationship_scores.classify_pair(&self.gladiateurs[a].config.id, &self.gladiateurs[b].config.id, &self.tuning) {
                    Some(RelationshipKind::Rival) => 3,
                    Some(RelationshipKind::Tense) => 2,
                    None => 1,
                    Some(RelationshipKind::Ally) => 0,
                };
                if best.is_none_or(|(_, _, r)| rank > r) {
                    best = Some((a, b, rank));
                }
            }
        }
        best.map(|(a, b, _)| (a, b))
    }

    /// Two allies of the order relay each other (probability `COALITION_PROBABILITY`):
    /// the follower is moved right after the leader.
    pub(super) fn try_coalition(&mut self, order: Vec<usize>, channel: &Channel<ArenaEvent>) -> Vec<usize> {
        if order.len() < constants::COALITION_MIN_ACTIVE || !scene_events::eligible_mode(&self.config.discussion_mode) {
            return order;
        }
        let pair = order.iter().enumerate().find_map(|(x, &a)| {
            order[x + 1..].iter().find(|&&b| {
                self.relationship_scores.classify_pair(&self.gladiateurs[a].config.id, &self.gladiateurs[b].config.id, &self.tuning) == Some(RelationshipKind::Ally)
            }).map(|&b| (a, b))
        });
        let Some((a, b)) = pair else { return order };
        #[cfg(test)]
        let forced = self.coalitions_forced;
        #[cfg(not(test))]
        let forced = false;
        #[cfg(test)]
        if !forced && self.random_staging_disabled {
            return order;
        }
        if !forced && !rand::thread_rng().gen_bool(self.tuning.coalition_probability) {
            return order;
        }
        let (a_id, b_id) = (self.gladiateurs[a].config.id.clone(), self.gladiateurs[b].config.id.clone());
        let (a_name, b_name) = (self.gladiateurs[a].config.name.clone(), self.gladiateurs[b].config.name.clone());
        let mut reordered: Vec<usize> = Vec::with_capacity(order.len());
        for &i in &order {
            if i == b {
                continue;
            }
            reordered.push(i);
            if i == a {
                reordered.push(b);
            }
        }
        tracing::info!(turn = self.current_turn, leader = %a_name, follower = %b_name, "Coalition formed");
        let _ = channel.send(ArenaEvent::CoalitionFormed { turn: self.current_turn, a: a_id.clone(), b: b_id.clone(), a_name, b_name });
        self.turn_coalition = Some((a_id, b_id));
        reordered
    }

    /// The speaker's part in this turn's coalition, if any.
    pub(super) fn coalition_role_for(&self, speaker_id: &str) -> Option<CoalitionRole> {
        let (a, b) = self.turn_coalition.as_ref()?;
        let name = |id: &str| self.gladiateurs.iter().find(|g| g.config.id == id).map(|g| g.config.name.clone()).unwrap_or_default();
        if speaker_id == a {
            Some(CoalitionRole::Leader { partner: name(b) })
        } else if speaker_id == b {
            Some(CoalitionRole::Follower { partner: name(a) })
        } else {
            None
        }
    }

    /// Test hooks (v1.18): force the next scene event / every eligible coalition.
    #[cfg(test)]
    pub fn force_scene_event(&mut self, kind: SceneEventKind) {
        self.forced_scene_event = Some(kind);
    }

    #[cfg(test)]
    pub fn force_coalitions(&mut self) {
        self.coalitions_forced = true;
    }

    /// Test hook: deterministic runs — the dice of the staging never roll.
    #[cfg(test)]
    pub fn disable_random_staging(&mut self) {
        self.random_staging_disabled = true;
    }

    pub(super) fn emit_arbitre_message(&mut self, content: &str, channel: &Channel<ArenaEvent>) {
        let arb_id = self.arbitre.config.id.clone();
        let arb_name = self.arbitre.config.name.clone();
        let msg = self.create_message(&arb_id, &arb_name, SpeakerRole::Arbitre, content);
        let _ = channel.send(ArenaEvent::MessageComplete {
            message: msg.clone(),
        });
        self.turn_messages.push(msg.clone());
        self.messages_history.push(msg);
    }

    pub(super) fn emit_ban_notification(&mut self, content: &str, channel: &Channel<ArenaEvent>) {
        let arb_id = self.arbitre.config.id.clone();
        let arb_name = self.arbitre.config.name.clone();
        let mut msg = self.create_message(&arb_id, &arb_name, SpeakerRole::Arbitre, content);
        msg.is_ban_notification = true;
        msg.kind = MessageKind::BanNotification;
        let _ = channel.send(ArenaEvent::MessageComplete {
            message: msg.clone(),
        });
        self.turn_messages.push(msg.clone());
        self.messages_history.push(msg);
    }

    /// Step mode (v1.20.1): before a speaker talks, wait for the audience's cue
    /// (`NextSpeaker`), serving the other commands meanwhile. A cue received
    /// ahead is consumed at once; leaving the mode, a stop or a closed channel
    /// releases the wait.
    pub(super) async fn await_cue(&mut self, glad_idx: usize, cmd_rx: &mut mpsc::Receiver<EngineCommand>, channel: &Channel<ArenaEvent>) {
        if !self.step_mode || self.cancel_token.is_cancelled() {
            return;
        }
        if self.cues_pending > 0 {
            self.cues_pending -= 1;
            return;
        }
        let speaker_id = self.gladiateurs[glad_idx].config.id.clone();
        let speaker_name = self.gladiateurs[glad_idx].config.name.clone();
        tracing::info!(speaker = %speaker_name, turn = self.current_turn, "Step mode: waiting for the audience's cue");
        let _ = channel.send(ArenaEvent::AwaitingCue { speaker_id, speaker_name });
        let cancel = self.cancel_token.clone();
        loop {
            tokio::select! {
                cmd = cmd_rx.recv() => match cmd {
                    Some(EngineCommand::NextSpeaker) => return,
                    Some(EngineCommand::SetStepMode { enabled }) => {
                        self.set_step_mode(enabled);
                        if !enabled {
                            return;
                        }
                    }
                    Some(EngineCommand::ForceStop) => {
                        self.status = DiscussionStatus::ForceStopRequested;
                        return;
                    }
                    Some(EngineCommand::Stop) => {
                        self.status = DiscussionStatus::StopRequested;
                        return;
                    }
                    Some(EngineCommand::Pause) => {
                        self.status = DiscussionStatus::Paused;
                        let _ = channel.send(ArenaEvent::PauseConfirmed);
                    }
                    Some(EngineCommand::Resume) => {
                        self.status = DiscussionStatus::Active;
                        let _ = channel.send(ArenaEvent::ResumeConfirmed);
                    }
                    Some(EngineCommand::AdjustEmotion { speaker_id, axis, value }) => self.handle_adjust_emotion(&speaker_id, &axis, value, channel),
                    Some(EngineCommand::AudienceReaction { message_id, reaction_type }) => self.handle_audience_reaction(&message_id, reaction_type, channel),
                    Some(EngineCommand::UserWantsToIntervene) => self.user_intervention_pending = true,
                    Some(_) => {}
                    None => return,
                },
                _ = cancel.cancelled() => return,
            }
        }
    }

    pub(super) async fn handle_user_intervention(
        &mut self,
        cmd_rx: &mut mpsc::Receiver<EngineCommand>,
        channel: &Channel<ArenaEvent>,
    ) {
        tracing::info!(
            discussion_id = %self.discussion_id,
            turn = self.current_turn,
            "User intervention: waiting for user input"
        );
        let _ = channel.send(ArenaEvent::UserTurnReady);

        let timeout = tokio::time::sleep(Duration::from_secs(
            self.config.user_intervention_timeout_secs,
        ));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                cmd = cmd_rx.recv() => {
                    match cmd {
                        Some(EngineCommand::SubmitUserMessage { content }) => {
                            tracing::info!(
                                discussion_id = %self.discussion_id,
                                turn = self.current_turn,
                                content_len = content.len(),
                                "User intervention: message received"
                            );
                            let msg = self.create_message(
                                constants::USER_SPEAKER_ID,
                                &self.config.user_name,
                                SpeakerRole::User,
                                &content,
                            );
                            let _ = channel.send(ArenaEvent::MessageComplete { message: msg.clone() });
                            self.turn_messages.push(msg.clone());
                            self.messages_history.push(msg.clone());
                            self.user_intervention_pending = false;
                            self.user_intervention_handled = true;
                            // The audience is a participant from now on; the next speaker answers them (v1.20.2)
                            self.user_has_spoken = true;
                            self.user_reply_pending = Some(truncate_at_word_boundary(content.trim(), constants::AUDIENCE_MESSAGE_EXCERPT_CHARS));
                            // The participants react to the user's words as they would to a peer's
                            self.process_reaction_round(&msg, channel).await;
                            return;
                        }
                        Some(EngineCommand::SkipUserTurn) => {
                            tracing::info!(
                                discussion_id = %self.discussion_id,
                                turn = self.current_turn,
                                "User intervention: skipped"
                            );
                            self.user_intervention_pending = false;
                            self.user_intervention_handled = true;
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
                        Some(EngineCommand::AdjustEmotion { speaker_id, axis, value }) => {
                            self.handle_adjust_emotion(&speaker_id, &axis, value, channel);
                        }
                        Some(EngineCommand::AudienceReaction { message_id, reaction_type }) => {
                            self.handle_audience_reaction(&message_id, reaction_type, channel);
                        }
                        Some(_) => {}
                        None => return,
                    }
                }
                _ = &mut timeout => {
                    let _ = channel.send(ArenaEvent::UserTurnTimeout);
                    self.user_intervention_pending = false;
                    self.user_intervention_handled = true;
                    return;
                }
            }
        }
    }

    /// End of turn (v1.17): the document integration, the emotion analysis, the
    /// memory update and the argument extraction are prepared, run together
    /// (bounded by `max_parallel_calls`) and applied in the historical order.
    /// On a sequential provider the memory and emotion calls are fused into one
    /// `TurnAnalyst` call. Contagion, room mood, relationship decay and the
    /// history snapshot keep their place between emotions and memory.
    pub(super) async fn run_end_of_turn_calls(&mut self, channel: &Channel<ArenaEvent>) {
        let limit = self.llm.capabilities().max_parallel_calls.max(1);
        let mut jobs: Vec<(EndOfTurnPhase, LlmRequest, bool)> = Vec::new();
        if let Some(request) = self.prepare_document_turn() {
            jobs.push((EndOfTurnPhase::Document, request, false));
        }
        let emotion = self.prepare_emotion_analysis();
        let memory = self.prepare_memory_update();
        match (limit <= 1, emotion, memory) {
            (true, Some(e), Some(m)) => {
                let prompt = prompt_builder::build_turn_analyst_prompt(&m.user, &e.user, &self.config.discussion_language);
                let system = format!("{}\n{}", m.system, e.system);
                jobs.push((EndOfTurnPhase::TurnAnalyst, LlmRequest::new(&system, &prompt, &self.arbitre.config.llm_params, CallKind::TurnAnalyst).json(), true));
            }
            (_, e, m) => {
                if let Some(e) = e {
                    jobs.push((EndOfTurnPhase::Emotion, e, false));
                }
                if let Some(m) = m {
                    jobs.push((EndOfTurnPhase::Memory, m, true));
                }
            }
        }
        if self.argument_map_enabled && !self.cancel_token.is_cancelled() {
            if let Some(request) = self.prepare_argument_extraction() {
                jobs.push((EndOfTurnPhase::ArgumentMap, request, true));
            }
        }

        let futures: Vec<Pin<Box<dyn Future<Output = PhaseOutcome> + Send>>> = jobs
            .into_iter()
            .map(|(phase, request, retry)| {
                Box::pin(run_phase(Arc::clone(&self.llm), self.cancel_token.clone(), phase, request, retry))
                    as Pin<Box<dyn Future<Output = PhaseOutcome> + Send>>
            })
            .collect();
        tracing::info!(turn = self.current_turn, phases = futures.len(), limit, "End-of-turn calls starting");
        let outcomes = run_bounded(futures, limit).await;
        for o in &outcomes {
            self.turn_timer.record(o.phase.name(), o.ms);
            if o.retried {
                self.diagnostics.note_retry();
            }
            if let Err(e) = &o.raw {
                if !matches!(e, LlmError::Cancelled) {
                    tracing::warn!(phase = o.phase.name(), error = %e, "End-of-turn call failed — state unchanged");
                }
            }
        }
        let outcome = |phase: EndOfTurnPhase| outcomes.iter().find(|o| o.phase == phase && o.raw.is_ok());
        let known_names = self.known_participant_names();

        // E.0 Document
        if let Some(o) = outcome(EndOfTurnPhase::Document) {
            let (arb_id, arb_name) = (self.arbitre.config.id.clone(), self.arbitre.config.name.clone());
            self.apply_document_result(&arb_id, &arb_name, o.raw.as_deref().unwrap_or_default(), channel);
        }

        // E.1 Emotions (fused answer split first; the memory half waits for E.2/E.3)
        let mut fused_memory: Option<MemoryUpdateResponse> = None;
        if let Some(o) = outcome(EndOfTurnPhase::TurnAnalyst) {
            let raw = o.raw.as_deref().unwrap_or_default();
            match json_parser::parse_turn_analyst(raw, &known_names) {
                Some((memory, emotions)) => {
                    self.apply_emotion_analysis(emotions, channel);
                    fused_memory = Some(memory);
                }
                None => {
                    self.diagnostics.note_parse_failure(CallKind::TurnAnalyst);
                    tracing::warn!(turn = self.current_turn, preview = %truncate_str(raw, 200), "Turn analyst answer unusable — memory and emotions unchanged");
                }
            }
        } else if let Some(o) = outcome(EndOfTurnPhase::Emotion) {
            self.apply_emotion_raw(o.raw.as_deref().unwrap_or_default(), channel);
        }

        // E.2 The moderator's drift (v1.20.5), the emotional contagion (order-independent), then the room's temperature
        self.settle_arbitre_emotions(channel);
        self.apply_emotional_contagion(channel);
        self.emit_room_mood(channel);

        // E.2b Relationships fade unless fed again (v1.17)
        self.relationship_scores.decay(self.tuning.relationship_decay_per_turn);
        self.emit_relationships(channel);

        // E.3 Snapshot history + emit EmotionHistoryUpdate
        self.record_emotion_history(channel);

        // E.4 Memory — also refreshes the summary-based stagnation signal
        if let Some(memory) = fused_memory {
            self.apply_memory_update(memory, channel);
        } else if let Some(o) = outcome(EndOfTurnPhase::Memory) {
            self.apply_memory_raw(o.raw.as_deref().unwrap_or_default(), channel);
        }

        // E.5 Argument map
        if let Some(o) = outcome(EndOfTurnPhase::ArgumentMap) {
            let (raw, request) = (o.raw.as_deref().unwrap_or_default().to_string(), o.request.clone());
            self.apply_argument_extraction(&raw, &request, channel).await;
        }
    }

    /// Memory phase, part 1 (no LLM): every participant remembers the turn, then
    /// the update request is built. `None` when nothing was said.
    pub(super) fn prepare_memory_update(&mut self) -> Option<LlmRequest> {
        if self.turn_messages.is_empty() {
            return None;
        }
        let turn = self.current_turn;
        let is_fiction = self.config.discussion_mode == DiscussionMode::CollaborativeFiction;
        for g in &mut self.gladiateurs {
            memory_manager::add_turn_to_memory(&mut g.memory, turn, &self.turn_messages, is_fiction);
        }
        memory_manager::add_turn_to_memory(&mut self.arbitre.memory, turn, &self.turn_messages, is_fiction);

        let contextual_summary = &self.arbitre.memory.contextual_summary;
        let positional_json = memory_manager::positional_map_to_json(&self.arbitre.memory);
        let turn_text = memory_manager::format_turn_messages(&self.turn_messages, is_fiction);
        let prompt = prompt_builder::build_memory_update_prompt(
            contextual_summary,
            &positional_json,
            turn,
            &turn_text,
            &self.config.discussion_language,
            &self.config.discussion_mode,
            self.budget_for(&self.arbitre.config.id),
        );
        let mem_sys = self.memory_summarizer_prompt();
        Some(LlmRequest::new(&mem_sys, &prompt, &self.arbitre.config.llm_params, CallKind::Memory).json())
    }

    /// Memory phase, part 2: apply the parsed update (summary, positions with their
    /// trajectory, open questions) and refresh the summary-based stagnation signal.
    pub(super) fn apply_memory_update(&mut self, parsed: MemoryUpdateResponse, channel: &Channel<ArenaEvent>) {
        // Stagnation signal: the new summary barely differs from the previous one
        if !self.previous_summary.is_empty() && !parsed.summary.is_empty() {
            let similarity = emotion_engine::text_similarity(&self.previous_summary, &parsed.summary);
            self.summary_stagnating = similarity >= constants::EMOTION_STAGNATION_SIMILARITY;
            if self.summary_stagnating {
                tracing::info!(turn = self.current_turn, similarity, "Discussion summary is stagnating");
            }
        }
        if !parsed.summary.is_empty() {
            self.previous_summary = parsed.summary.clone();
        }
        let summary_max = self.budget_for(&self.arbitre.config.id).contextual_summary_chars;
        let positions = self.normalise_position_names(parsed.positions);
        let summary = if parsed.summary.is_empty() { self.arbitre.memory.contextual_summary.clone() } else { parsed.summary.clone() };
        memory_manager::update_from_llm_response(&mut self.arbitre.memory, summary.clone(), positions.clone(), summary_max);
        for g in &mut self.gladiateurs {
            memory_manager::update_from_llm_response(&mut g.memory, summary.clone(), positions.clone(), summary_max);
        }
        let positions = memory_manager::positions_sorted(&self.arbitre.memory);
        if !positions.is_empty() {
            let _ = channel.send(ArenaEvent::PositionsUpdated { positions });
        }
        // Questions the analyst spotted as unanswered → open loops on their addressee
        let names: Vec<String> = self.gladiateurs.iter().map(|g| g.config.name.clone()).collect();
        for q in &parsed.open_questions {
            let Some(to) = json_parser::match_speaker_name(&q.to, &names) else { continue };
            let Some(target_id) = self.gladiateurs.iter().find(|g| g.config.name == *to).map(|g| g.config.id.clone()) else { continue };
            let from = json_parser::match_speaker_name(&q.from, &names).cloned().unwrap_or_else(|| self.arbitre.config.name.clone());
            self.open_loops.push(&target_id, OpenLoop::new(OpenLoopKind::Question, &q.question, &from, self.current_turn));
        }
    }

    /// Memory phase, parse step: a broken answer is counted and leaves the state untouched.
    pub(super) fn apply_memory_raw(&mut self, raw: &str, channel: &Channel<ArenaEvent>) {
        match json_parser::parse_json_response::<MemoryUpdateResponse>(raw) {
            Ok(parsed) => self.apply_memory_update(parsed, channel),
            Err(_) => {
                self.diagnostics.note_parse_failure(CallKind::Memory);
                let end = raw.floor_char_boundary(500);
                tracing::warn!(turn = self.current_turn, raw_len = raw.len(), raw_preview = %&raw[..end], "Failed to parse memory update response");
            }
        }
    }

    /// Stream the synthesis, emit `SynthesisComplete` and return its text (empty on failure).
    pub(super) async fn generate_synthesis(&self, channel: &Channel<ArenaEvent>) -> String {
        let document_context = self.build_document_context_for_synthesis();
        let full_doc_text = self.full_document_for(&self.arbitre.config.id);
        // The moderator judges the agendas without knowing the outcome yet
        let agendas = self.agenda_reveals("");
        let prompt = prompt_builder::build_synthesis_prompt(
            &self.config.topic,
            &self.arbitre.memory,
            &self.config.discussion_language,
            &self.sources_registry,
            &self.config.discussion_mode,
            document_context.as_deref(),
            full_doc_text.as_deref(),
            self.budget_for(&self.arbitre.config.id),
            &agendas,
            self.outcome.as_ref(),
        );
        // Synthesis is a comprehensive summary — use a dedicated, higher token budget.
        let mut synth_params = self.arbitre.config.llm_params.clone();
        synth_params.num_predict = synth_params.num_predict.max(constants::SYNTHESIS_NUM_PREDICT);
        let request = LlmRequest::new(
            &self.arbitre_system,
            &prompt,
            &synth_params,
            CallKind::Synthesis,
        )
        .reasoning(self.synthesis_reasoning_level())
        .pace(self.reasoning_pace)
        .speaker(&self.arbitre.config.id);

        let ch = channel.clone();
        let on_token = move |token: &str| {
            let _ = ch.send(ArenaEvent::SynthesisChunk {
                chunk: token.to_string(),
            });
        };
        let summary = match self
            .llm
            .chat_stream(&request, &on_token, &|_| {}, self.cancel_token.clone())
            .await
        {
            Ok(resp) => resp.content,
            Err(e) => {
                tracing::warn!("Synthesis streaming failed: {}", e);
                String::new()
            }
        };

        // If synthesis is empty (e.g. thinking model exhausted tokens on reasoning),
        // retry once without reasoning, with higher temperature and doubled num_predict.
        if summary.trim().is_empty() && !self.cancel_token.is_cancelled() {
            tracing::warn!("Synthesis was empty — retrying with higher temperature and doubled budget");
            let mut retry_request = request.clone().reasoning(ReasoningLevel::Off);
            retry_request.params.temperature = (retry_request.params.temperature + constants::TEMP_DIFFICULTY_BOOST).min(constants::TEMP_MAX);
            retry_request.params.num_predict = retry_request.params.num_predict.saturating_mul(2);
            match self.chat_text(&retry_request).await {
                Ok(retry_summary) if !retry_summary.trim().is_empty() => {
                    tracing::info!(len = retry_summary.len(), "Synthesis retry succeeded");
                    // Emit all at once since we can't stream the retry
                    let _ = channel.send(ArenaEvent::SynthesisChunk { chunk: retry_summary.clone() });
                    let _ = channel.send(ArenaEvent::SynthesisComplete { summary: retry_summary.clone() });
                    return retry_summary;
                }
                Ok(_) => tracing::error!("Synthesis retry also returned empty"),
                Err(e) => tracing::error!(error = %e, "Synthesis retry failed"),
            }
        }

        let _ = channel.send(ArenaEvent::SynthesisComplete { summary: summary.clone() });
        summary
    }
}
