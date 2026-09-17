//! End-of-turn analysis: emotions, contagion, history, argument map extraction.

use super::*;

impl DiscussionEngine {
    // ── Emotion analysis, contagion, history ──────────────────────────

    /// Emotion phase, part 1: the analyst request (`None` when nothing was said).
    pub(super) fn prepare_emotion_analysis(&self) -> Option<LlmRequest> {
        if self.turn_messages.is_empty() {
            return None;
        }

        // Build participants JSON for the prompt
        let mut participants_info = Vec::new();
        participants_info.push(format!(
            "  \"{}\": {{\"role\": \"IArbitre\", \"engagement\": {}, \"accord\": {}, \"confiance\": {}, \"frustration\": {}, \"curiosite\": {}, \"enthousiasme\": {}}}",
            self.arbitre.config.name,
            self.arbitre.emotions.engagement, self.arbitre.emotions.accord,
            self.arbitre.emotions.confiance, self.arbitre.emotions.frustration,
            self.arbitre.emotions.curiosite, self.arbitre.emotions.enthousiasme,
        ));
        for g in &self.gladiateurs {
            participants_info.push(format!(
                "  \"{}\": {{\"role\": \"GladIAteur\", \"engagement\": {}, \"accord\": {}, \"confiance\": {}, \"frustration\": {}, \"curiosite\": {}, \"enthousiasme\": {}}}",
                g.config.name,
                g.emotions.engagement, g.emotions.accord,
                g.emotions.confiance, g.emotions.frustration,
                g.emotions.curiosite, g.emotions.enthousiasme,
            ));
        }
        let participants_json = format!("{{\n{}\n}}", participants_info.join(",\n"));

        // Build recent context from turn messages
        let recent_context = self.turn_messages.iter()
            .map(|m| format!("[{}] {}", m.speaker_name, truncate_str(&m.content, constants::ORCH_EMOTION_CONTEXT)))
            .collect::<Vec<_>>()
            .join("\n");

        // Build events summary (reactions, bans)
        let mut events = Vec::new();
        for (sid, tally) in &self.turn_reaction_counts {
            let name = self.gladiateurs.iter()
                .find(|g| g.config.id == *sid)
                .map(|g| g.config.name.as_str())
                .unwrap_or(sid);
            if tally.likes > 0 { events.push(format!("{} received {} approval(s)", name, tally.likes)); }
            if tally.dislikes > 0 { events.push(format!("{} received {} disapproval(s)", name, tally.dislikes)); }
            if tally.questions > 0 { events.push(format!("{} raised {} question(s)", name, tally.questions)); }
            if tally.laughs > 0 { events.push(format!("{} made the room laugh {} time(s)", name, tally.laughs)); }
        }
        if !self.turn_audience_targets.is_empty() {
            let mut names: Vec<&str> = self.turn_audience_targets.iter().map(String::as_str).collect();
            names.sort_unstable();
            events.push(format!("the audience reacted to {}", names.join(", ")));
        }
        for g in &self.gladiateurs {
            if g.ban_issued_this_turn {
                events.push(format!("{} was banned this turn", g.config.name));
            }
        }
        let events_summary = if events.is_empty() {
            "No notable events".to_string()
        } else {
            events.join(", ")
        };

        let prompt = prompt_builder::build_emotion_analysis_prompt(
            &participants_json,
            &recent_context,
            &events_summary,
            &self.config.discussion_language,
        );

        let sys_prompt = match self.config.discussion_language.as_str() {
            "en" => "You are an emotion analyst. Base your analysis strictly on the exchanges and reactions provided. Do not invent events. Respond only with JSON.",
            "zh" => "你是情绪分析师。严格根据提供的交流和反应进行分析。不要捏造事件。仅用JSON回复。",
            _ => "Tu es un analyste émotionnel. Base ton analyse strictement sur les échanges et réactions fournis. N'invente pas d'événements. Réponds uniquement en JSON.",
        };

        Some(LlmRequest::new(sys_prompt, &prompt, &self.arbitre.config.llm_params, CallKind::Emotion).json())
    }

    /// Names the analysts may address (moderator first; the audience member once they spoke).
    pub(super) fn known_participant_names(&self) -> Vec<String> {
        std::iter::once(self.arbitre.config.name.clone())
            .chain(self.gladiateurs.iter().map(|g| g.config.name.clone()))
            .chain(self.user_has_spoken.then(|| self.config.user_name.clone()))
            .collect()
    }

    /// Emotion phase, part 2: apply the bounded deltas (reactions and bans were already rule-applied).
    pub(super) fn apply_emotion_analysis(&mut self, analysis: json_parser::EmotionAnalysis, channel: &Channel<ArenaEvent>) {
        self.llm_stagnation_flag = analysis.stagnating.unwrap_or(false);
        let deltas: HashMap<String, EmotionDelta> = analysis
            .deltas
            .iter()
            .map(|(name, d)| (name.clone(), emotion_engine::clamp_delta(d, constants::EMOTION_LLM_DELTA_CAP)))
            .collect();

        // Apply to arbitre
        if let Some(delta) = deltas.get(&self.arbitre.config.name) {
            let prev = self.arbitre.emotions.clone();
            emotion_engine::apply_llm_delta(&mut self.arbitre.emotions, delta);
            let (arb_id, current) = (self.arbitre.config.id.clone(), self.arbitre.emotions.clone());
            self.emit_threshold_events(channel, &arb_id, &prev, &current);
            Self::emit_emotion_updated(channel, &self.arbitre.config.id, &self.arbitre.emotions, &self.config.discussion_language);
        }

        // Apply to gladiateurs
        for i in 0..self.gladiateurs.len() {
            let Some(delta) = deltas.get(&self.gladiateurs[i].config.name) else { continue };
            let prev = self.gladiateurs[i].emotions.clone();
            emotion_engine::apply_llm_delta(&mut self.gladiateurs[i].emotions, delta);
            let (sid, current) = (self.gladiateurs[i].config.id.clone(), self.gladiateurs[i].emotions.clone());
            self.emit_threshold_events(channel, &sid, &prev, &current);
            Self::emit_emotion_updated(channel, &sid, &current, &self.config.discussion_language);
        }
    }

    /// Emotion phase, parse step: an answer that yields nothing is counted (the rule-based values stay).
    pub(super) fn apply_emotion_raw(&mut self, raw: &str, channel: &Channel<ArenaEvent>) {
        if json_parser::parse_json_response::<serde_json::Value>(raw).is_err() {
            self.diagnostics.note_parse_failure(CallKind::Emotion);
            tracing::warn!(turn = self.current_turn, preview = %truncate_str(raw, 200), "Emotion analysis unusable — keeping rule-based values");
            return;
        }
        let analysis = json_parser::parse_emotion_analysis(raw, &self.known_participant_names());
        self.apply_emotion_analysis(analysis, channel);
    }

    /// Apply emotional contagion: compute average, move everyone toward it (order-independent).
    pub(super) fn apply_emotional_contagion(&mut self, channel: &Channel<ArenaEvent>) {
        // Group average: the active gladiateurs (the moderator does not take part
        // in the exchange, so it only receives the mood — it does not shape it)
        let mut profiles: Vec<&EmotionalProfile> = Vec::new();
        if constants::EMOTION_CONTAGION_INCLUDE_ARBITRE {
            profiles.push(&self.arbitre.emotions);
        }
        for g in &self.gladiateurs {
            if !g.is_banned() {
                profiles.push(&g.emotions);
            }
        }
        if profiles.len() < 2 {
            return; // No contagion with < 2 participants
        }

        let avg = emotion_engine::compute_average(&profiles);

        // Apply to arbitre
        emotion_engine::apply_contagion(&avg, &mut self.arbitre.emotions);
        Self::emit_emotion_updated(channel, &self.arbitre.config.id, &self.arbitre.emotions, &self.config.discussion_language);

        // Apply to non-banned gladiateurs
        for g in &mut self.gladiateurs {
            if !g.is_banned() {
                emotion_engine::apply_contagion(&avg, &mut g.emotions);
                Self::emit_emotion_updated(channel, &g.config.id, &g.emotions, &self.config.discussion_language);
            }
        }
    }

    /// Record emotion history snapshots and emit EmotionHistoryUpdate events.
    pub(super) fn record_emotion_history(&mut self, channel: &Channel<ArenaEvent>) {
        let turn = self.current_turn;

        // Arbitre
        self.arbitre.emotion_history.push(EmotionSnapshot {
            turn,
            emotions: self.arbitre.emotions.clone(),
        });
        if self.arbitre.emotion_history.len() > constants::ORCH_MAX_EMOTION_HISTORY {
            self.arbitre.emotion_history.remove(0);
        }
        let _ = channel.send(ArenaEvent::EmotionHistoryUpdate {
            speaker_id: self.arbitre.config.id.clone(),
            history: self.arbitre.emotion_history.clone(),
        });

        // Gladiateurs
        for g in &mut self.gladiateurs {
            g.emotion_history.push(EmotionSnapshot {
                turn,
                emotions: g.emotions.clone(),
            });
            if g.emotion_history.len() > constants::ORCH_MAX_EMOTION_HISTORY {
                g.emotion_history.remove(0);
            }
            let _ = channel.send(ArenaEvent::EmotionHistoryUpdate {
                speaker_id: g.config.id.clone(),
                history: g.emotion_history.clone(),
            });
        }
    }

    /// Handle manual emotion adjustment from the frontend
    pub(super) fn handle_adjust_emotion(
        &mut self,
        speaker_id: &str,
        axis: &str,
        value: u8,
        channel: &Channel<ArenaEvent>,
    ) {
        let value = value.min(100);

        // Try matching arbitre
        if speaker_id == self.arbitre.config.id {
            Self::set_emotion_axis(&mut self.arbitre.emotions, axis, value);
            Self::emit_emotion_updated(channel, &self.arbitre.config.id, &self.arbitre.emotions, &self.config.discussion_language);
            return;
        }

        // Try matching gladiateurs
        for g in &mut self.gladiateurs {
            if g.config.id == speaker_id {
                Self::set_emotion_axis(&mut g.emotions, axis, value);
                Self::emit_emotion_updated(channel, &g.config.id, &g.emotions, &self.config.discussion_language);
                return;
            }
        }
    }

    /// Emit threshold-crossing events for axes that newly crossed HIGH or LOW boundaries.
    /// Threshold crossings for the UI, plus one stage direction for the first
    /// crossing (emotion-driven, capped per speaker and turn).
    pub(super) fn emit_threshold_events(
        &mut self,
        channel: &Channel<ArenaEvent>,
        speaker_id: &str,
        prev: &EmotionalProfile,
        current: &EmotionalProfile,
    ) {
        // v1.20.4 — the theatre follows the movements too: entering the notable
        // zone away from the persona's baseline counts as a crossing (UI flash,
        // timeline mark, stage direction) like the absolute thresholds
        let mut crossed = emotion_engine::detect_thresholds(prev, current);
        let baseline = self.baseline_of(speaker_id);
        for moved in self.shift_zones.crossings(speaker_id, current, &baseline) {
            if !crossed.iter().any(|(axis, _, _)| *axis == moved.0) {
                crossed.push(moved);
            }
        }
        for (axis, direction, value) in &crossed {
            let _ = channel.send(ArenaEvent::EmotionalThresholdCrossed {
                speaker_id: speaker_id.to_string(),
                axis: axis.clone(),
                direction: direction.clone(),
                value: *value,
            });
        }
        if let Some((axis, direction, _)) = stage_directions::most_dramatic(&crossed) {
            self.emit_stage_direction(speaker_id, StageCue::Threshold { axis, direction }, channel);
        }
    }

    /// The profile a speaker started with (the moderator starts neutral).
    fn baseline_of(&self, speaker_id: &str) -> EmotionalProfile {
        self.gladiateurs
            .iter()
            .find(|g| g.config.id == speaker_id)
            .map(|g| g.initial_emotions.clone())
            .unwrap_or_default()
    }

    pub(super) fn emit_emotion_updated(
        channel: &Channel<ArenaEvent>,
        speaker_id: &str,
        emotions: &EmotionalProfile,
        lang: &str,
    ) {
        let mood = prompt_builder::summarize_emotional_state(emotions, lang);
        let _ = channel.send(ArenaEvent::EmotionUpdated {
            speaker_id: speaker_id.to_string(),
            emotions: emotions.clone(),
            mood_summary: Some(mood),
        });
    }

    pub(super) fn set_emotion_axis(emotions: &mut EmotionalProfile, axis: &str, value: u8) {
        match axis {
            "engagement" => emotions.engagement = value,
            "accord" => emotions.accord = value,
            "confiance" => emotions.confiance = value,
            "frustration" => emotions.frustration = value,
            "curiosite" => emotions.curiosite = value,
            "enthousiasme" => emotions.enthousiasme = value,
            _ => {}
        }
    }

    pub(super) fn create_message(
        &self,
        speaker_id: &str,
        speaker_name: &str,
        role: SpeakerRole,
        content: &str,
    ) -> Message {
        Message {
            id: uuid::Uuid::new_v4().to_string(),
            discussion_id: self.discussion_id.clone(),
            turn_number: self.current_turn,
            speaker_id: speaker_id.to_string(),
            speaker_name: speaker_name.to_string(),
            role,
            content: content.to_string(),
            inner_thought: None,
            thought_kind: ThoughtKind::Persona,
            reactions: Vec::new(),
            is_ban_notification: false,
            kind: MessageKind::Normal,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Build read-only document context for synthesis, or None if document format is disabled.
    pub(super) fn build_document_context_for_synthesis(&self) -> Option<String> {
        if self.config.document_format == DocumentFormat::None {
            return None;
        }
        Some(prompt_builder::build_document_context_readonly(
            &self.document_content,
            self.config.document_format.as_extension(),
            &self.config.discussion_language,
            &self.config.discussion_mode,
        ))
    }

    /// Request regenerating the document from one contribution (pass 2).
    pub(super) fn document_update_request(&self, speaker_name: &str, discussion_text: &str, llm_params: &LlmParams) -> LlmRequest {
        tracing::info!(speaker = %speaker_name, "Pass 2: generating document update");

        let (sys, usr) = prompt_builder::build_document_update_prompt(
            &self.document_content,
            self.config.document_format.as_extension(),
            discussion_text,
            &self.config.discussion_mode,
            &self.config.discussion_language,
            &self.config.topic,
        );

        // Use speaker's LLM params with enough tokens for the full document.
        // The document grows over turns — estimate current tokens (≈ chars/CHARS_PER_TOKEN_ESTIMATE for multilingual)
        // and set num_predict to 2× current size + padding, so the LLM can reproduce + extend.
        let mut params = llm_params.clone();
        let estimated_doc_tokens = (self.document_content.len() / constants::CHARS_PER_TOKEN_ESTIMATE) as i32;
        params.num_predict = params.num_predict.max(estimated_doc_tokens * 2 + constants::ORCH_DOC_TOKEN_PADDING).max(constants::ORCH_DOC_MIN_NUM_PREDICT);

        LlmRequest::new(&sys, &usr, &params, CallKind::DocumentUpdate)
    }

    /// The document carried by a pass-2 answer: the `<document>` block when the
    /// model wrapped it, else the whole text; `None` when empty.
    pub(super) fn document_from_response(raw: &str) -> Option<String> {
        if raw.trim().is_empty() {
            return None;
        }
        let doc = match json_parser::extract_and_strip_document(raw) {
            (_, Some(extracted)) => extracted,
            _ => raw.trim().to_string(),
        };
        tracing::info!(len = doc.len(), preview = %truncate_str(&doc, 200), "Document update generated");
        Some(doc)
    }

    /// Sequential pass 2 (per-intervention granularity): call, then read the document.
    pub(super) async fn generate_document_update(&self, speaker_name: &str, discussion_text: &str, llm_params: &LlmParams) -> Option<String> {
        if self.config.document_format == DocumentFormat::None || self.cancel_token.is_cancelled() {
            return None;
        }
        let request = self.document_update_request(speaker_name, discussion_text, llm_params);
        match self.chat_text(&request).await {
            Ok(raw) => {
                let doc = Self::document_from_response(&raw);
                if doc.is_none() {
                    tracing::warn!(speaker = %speaker_name, "Pass 2 returned empty — document unchanged");
                }
                doc
            }
            Err(LlmError::Cancelled) => None,
            Err(e) => {
                tracing::warn!(speaker = %speaker_name, error = %e, "Pass 2 document update failed — document unchanged");
                None
            }
        }
    }

    /// Regenerate the document from one contribution and broadcast it.
    pub(super) async fn apply_document_update(
        &mut self,
        speaker_id: &str,
        speaker_name: &str,
        contribution: &str,
        llm_params: &LlmParams,
        channel: &Channel<ArenaEvent>,
    ) {
        if let Some(updated_doc) = self.generate_document_update(speaker_name, contribution, llm_params).await {
            self.document_content = updated_doc.clone();
            let _ = channel.send(ArenaEvent::DocumentUpdated {
                speaker_id: speaker_id.to_string(),
                speaker_name: speaker_name.to_string(),
                content: updated_doc,
                format: self.config.document_format.as_extension().to_string(),
            });
        }
    }

    /// Document phase, part 1 (turn granularity): every contribution of the turn in
    /// ONE request, attributed to the moderator (who consolidates the group's work).
    pub(super) fn prepare_document_turn(&mut self) -> Option<LlmRequest> {
        if self.config.document_update_granularity != DocumentUpdateGranularity::Turn
            || self.turn_document_contributions.is_empty()
            || self.config.document_format == DocumentFormat::None
        {
            return None;
        }
        let contributions = std::mem::take(&mut self.turn_document_contributions);
        let combined = contributions
            .iter()
            .map(|(name, text)| format!("--- {name} ---\n{text}"))
            .collect::<Vec<_>>()
            .join("\n\n");
        let arb_name = self.arbitre.config.name.clone();
        let params = self.arbitre.config.llm_params.clone();
        Some(self.document_update_request(&arb_name, &combined, &params))
    }

    /// Document phase, part 2: the new document (when the answer holds one) is stored and announced.
    pub(super) fn apply_document_result(&mut self, speaker_id: &str, speaker_name: &str, raw: &str, channel: &Channel<ArenaEvent>) {
        let Some(updated_doc) = Self::document_from_response(raw) else {
            tracing::warn!(speaker = %speaker_name, "Document update returned nothing usable — keeping the previous document");
            return;
        };
        self.document_content = updated_doc.clone();
        let _ = channel.send(ArenaEvent::DocumentUpdated {
            speaker_id: speaker_id.to_string(),
            speaker_name: speaker_name.to_string(),
            content: updated_doc,
            format: self.config.document_format.as_extension().to_string(),
        });
    }

    /// Ask a gladiateur whether they want to respond in UserDriven mode.
    /// Returns true if the speaker wants to respond, false if they pass.
    pub(super) async fn ask_respond_or_pass(&self, glad_idx: usize) -> bool {
        let recent = self.build_recent_exchanges(glad_idx);
        let prompt = mode_prompts::build_respond_or_pass_prompt(
            &self.config.topic,
            &recent,
            &self.gladiateurs[glad_idx].config.name,
            &self.config.discussion_language,
        );
        let mut params = self.gladiateurs[glad_idx].config.llm_params.clone();
        params.num_predict = constants::ORCH_NUM_PREDICT_RESPOND_PASS; // Short response only
        let request = LlmRequest::new(
            &self.gladiateurs[glad_idx].config.system_prompt,
            &prompt,
            &params,
            CallKind::RespondOrPass,
        )
        .json()
        .speaker(&self.gladiateurs[glad_idx].config.id);
        match self.chat_text(&request).await {
            Ok(raw) => {
                // Parse {"respond": true/false}
                if let Ok(val) = json_parser::parse_json_response::<serde_json::Value>(&raw) {
                    val.get("respond").and_then(|v| v.as_bool()).unwrap_or(true)
                } else {
                    true // Default to responding if parsing fails
                }
            }
            Err(e) => {
                tracing::warn!("respond-or-pass LLM failed for {}: {e}", self.gladiateurs[glad_idx].config.name);
                true // Default to responding on error
            }
        }
    }

    /// Generate a Socratic question from IArbitre.
    pub(super) async fn generate_socratic_question(&self) -> Option<String> {
        let recent = self.turn_messages.iter()
            .chain(self.messages_history.iter().rev().take(constants::ORCH_RECENT_MESSAGES_TAKE))
            .map(|m| format!("{}: {}", m.speaker_name, truncate_str(&m.content, constants::ORCH_SOCRATIC_CONTEXT)))
            .collect::<Vec<_>>()
            .join("\n");
        let prompt = mode_prompts::build_socratic_question_prompt(
            &self.config.topic,
            &recent,
            &self.config.discussion_language,
            &self.socratic_questions,
        );
        let mut params = self.arbitre.config.llm_params.clone();
        params.num_predict = constants::ORCH_NUM_PREDICT_SOCRATIC; // Short question
        let request = LlmRequest::new(
            &self.arbitre.config.system_prompt,
            &prompt,
            &params,
            CallKind::Socratic,
        )
        .speaker(&self.arbitre.config.id);
        // The full question is emitted as MessageComplete at the call site
        match self.chat_text(&request).await {
            Ok(text) => {
                if text.is_empty() { None } else { Some(text) }
            }
            Err(e) => {
                tracing::warn!("Socratic question generation failed: {e}");
                None
            }
        }
    }

    // ── Argument map extraction ────────────────────────────────────────

    /// Argument-map phase, part 1: the extraction request (`None` when the turn is too thin).
    pub(super) fn prepare_argument_extraction(&self) -> Option<LlmRequest> {
        let substantive = self.turn_messages.iter().filter(|m| m.role != SpeakerRole::Arbitre).count();
        if substantive < constants::ARGMAP_MIN_TURN_MESSAGES {
            tracing::info!(substantive, "Argument map extraction skipped (too few messages)");
            return None;
        }

        // Build recent context from turn messages
        let recent_context: String = self
            .turn_messages
            .iter()
            .map(|m| format!("[{}] {}", m.speaker_name, truncate_str(&m.content, constants::ARGMAP_CONTEXT_CHARS)))
            .collect::<Vec<_>>()
            .join("\n");

        // Reactions flagged as strong points or questions point the extractor at what mattered
        let hints = reactions::argument_hints(&self.turn_messages, &self.config.discussion_language);
        let recent_context = if hints.is_empty() { recent_context } else { format!("{recent_context}\n{}", hints.join("\n")) };

        // Build existing theses + arguments context for the extraction prompt, followed by
        // the objections still unanswered (an answer nests under its objection, v1.20.1)
        let mut existing_context = self.build_existing_arguments_context();
        let objections_block = self.build_open_objections_block();
        if !objections_block.is_empty() {
            existing_context.push('\n');
            existing_context.push_str(&objections_block);
        }

        let prompt = prompt_builder::build_argument_extraction_prompt(
            &recent_context,
            &existing_context,
            &self.config.topic,
            &self.config.discussion_language,
        );

        let sys_prompt = match self.config.discussion_language.as_str() {
            "en" => "You are an argument analyst. Extract theses, supporting arguments, counter-arguments and evidence from the exchanges. Respond only with JSON.",
            "zh" => "你是论证分析师。从对话中提取论点、支持论据、反驳和证据。仅用JSON回复。",
            _ => "Tu es un analyste d'argumentation. Extrais les thèses, arguments de soutien, contre-arguments et preuves des échanges. Réponds uniquement en JSON.",
        };

        // Use dedicated params: generous context + num_predict for the long extraction prompt
        let mut params = self.arbitre.config.llm_params.clone();
        params.num_predict = params.num_predict.max(constants::ARGMAP_NUM_PREDICT);
        params.num_ctx = params.num_ctx.max(constants::ARGMAP_NUM_CTX);

        // .json() pins the structured-output temperature
        Some(LlmRequest::new(sys_prompt, &prompt, &params, CallKind::ArgumentMap).json())
    }

    /// Argument-map phase, part 2: parse (one sequential retry on a malformed
    /// non-empty answer), merge into the accumulated map and announce it.
    pub(super) async fn apply_argument_extraction(&mut self, raw: &str, request: &LlmRequest, channel: &Channel<ArenaEvent>) {
        tracing::info!(turn = self.current_turn, raw_len = raw.len(), raw_preview = %truncate_str(raw, 300), "Argument map LLM response received");
        let known_names = self.known_participant_names();
        let mut extractions = json_parser::parse_argument_extraction(raw, &known_names);
        // Retry once if parse failed on non-empty response (malformed JSON from LLM)
        if extractions.is_empty() && !raw.is_empty() && !self.cancel_token.is_cancelled() {
            self.diagnostics.note_parse_failure(CallKind::ArgumentMap);
            self.diagnostics.note_retry();
            tracing::info!(turn = self.current_turn, raw_len = raw.len(), "Argument map parse failed on non-empty response — retrying");
            let mut retry_req = request.clone();
            retry_req.params.temperature = (retry_req.params.temperature + constants::TEMP_DIFFICULTY_BOOST).min(constants::TEMP_MAX);
            match self.chat_text(&retry_req).await {
                Ok(retry_raw) if !retry_raw.is_empty() => {
                    tracing::info!(turn = self.current_turn, raw_len = retry_raw.len(), "Argument map retry response received");
                    extractions = json_parser::parse_argument_extraction(&retry_raw, &known_names);
                    if extractions.is_empty() {
                        self.diagnostics.note_parse_failure(CallKind::ArgumentMap);
                        tracing::warn!(turn = self.current_turn, raw = %truncate_str(&retry_raw, constants::ARGMAP_RAW_LOG_MAX_CHARS), "Argument map answer unusable twice — raw retry answer");
                    }
                }
                Ok(_) => {}
                Err(LlmError::Cancelled) => return,
                Err(e) => tracing::warn!(error = %e, "Argument map retry failed"),
            }
        }
        if extractions.is_empty() {
            tracing::info!(turn = self.current_turn, "No arguments extracted this turn (parse returned empty)");
            return;
        }

        tracing::info!(turn = self.current_turn, extractions_count = extractions.len(), "Argument map extractions parsed, merging");
        let speakers: HashMap<String, String> = std::iter::once((self.arbitre.config.name.clone(), self.arbitre.config.id.clone()))
            .chain(self.gladiateurs.iter().map(|g| (g.config.name.clone(), g.config.id.clone())))
            .chain(self.user_has_spoken.then(|| (self.config.user_name.clone(), constants::USER_SPEAKER_ID.to_string())))
            .collect();
        let report = argument_merge::merge_extractions(
            &mut self.argument_map,
            extractions,
            &speakers,
            &self.config.discussion_language,
        );
        if report.new_node_ids.is_empty() && report.dropped == 0 {
            tracing::info!(turn = self.current_turn, "Argument map unchanged this turn");
            return;
        }
        self.open_objection_loops(&report.new_node_ids);

        let new_ids: HashSet<String> = report.new_node_ids.iter().cloned().collect();
        let md = self.argument_map.to_markdown(&self.config.topic, &new_ids);
        let md_speaker = self.argument_map.to_markdown_by_speaker(&self.config.topic, &new_ids, &self.config.discussion_language);
        let depth = self.argument_map.depth_stats();
        tracing::info!(
            turn = self.current_turn,
            theses = self.argument_map.theses_count(),
            arguments = self.argument_map.arguments_count(),
            new_nodes = report.new_node_ids.len(),
            deduplicated = report.deduplicated_theses,
            unattached = report.unattached,
            dropped = report.dropped,
            ?depth,
            "Argument map merged"
        );
        let _ = channel.send(ArenaEvent::ArgumentMapUpdated {
            markdown: md,
            markdown_by_speaker: md_speaker,
            theses_count: self.argument_map.theses_count() as u32,
            arguments_count: self.argument_map.arguments_count() as u32,
            map: self.argument_map.clone(),
            new_node_ids: report.new_node_ids,
            dropped_count: report.dropped,
            depth,
        });
    }

    /// The most recent counter-argument `speaker_id` still owes an answer to, as
    /// "text (by)" — argument map on and argumentative mode only (v1.20.1).
    pub(super) fn unanswered_objection_for(&self, speaker_id: &str) -> Option<String> {
        if !self.argument_map_enabled || !self.config.discussion_mode.rewards_depth() {
            return None;
        }
        self.argument_map
            .unanswered_objections()
            .into_iter()
            .rev()
            .find(|o| o.debtor_id == speaker_id)
            .map(|o| format!("{} ({})", o.text, o.by_name))
    }

    /// New counter-arguments become open loops of the speakers they hit (one per
    /// debtor and turn): the answer is expected in front of them, never forced.
    pub(super) fn open_objection_loops(&mut self, new_ids: &[String]) {
        if !self.config.discussion_mode.rewards_depth() {
            return;
        }
        let mut served: HashSet<String> = HashSet::new();
        let objections = self.argument_map.unanswered_objections();
        for o in objections.iter().rev().filter(|o| new_ids.contains(&o.argument_id)) {
            if !self.gladiateurs.iter().any(|g| g.config.id == o.debtor_id) || !served.insert(o.debtor_id.clone()) {
                continue;
            }
            tracing::info!(debtor = %o.debtor_id, by = %o.by_name, objection = %o.text, "Objection opened as a loop");
            self.open_loops.push(&o.debtor_id, OpenLoop::new(OpenLoopKind::Objection, &o.text, &o.by_name, self.current_turn));
        }
    }

    /// Moderator hint (v1.20.1): the objection the speaker who just spoke still owes
    /// an answer to, so the moderator asks for depth when the intervention ignored it.
    pub(super) fn depth_hint_for(&self, speaker_id: &str) -> Option<String> {
        let objection = self.unanswered_objection_for(speaker_id)?;
        Some(match self.config.discussion_language.as_str() {
            "en" => format!("Depth: an objection is still unanswered — \"{objection}\". If this intervention ignores it, ask in one sentence for an answer on the merits (never impose a topic)."),
            "zh" => format!("深度：仍有一条反驳未被回应——\"{objection}\"。如果这次发言忽视了它，用一句话要求就实质作出回应（绝不强加话题）。"),
            _ => format!("Profondeur : une objection reste sans réponse — « {objection} ». Si cette intervention l'ignore, demande en une phrase d'y répondre sur le fond (sans imposer de sujet)."),
        })
    }

    /// "[État du débat]" (v1.20.3): the theses on the table, the most argued first,
    /// each with its owner and how many objections wait on it; then the newest
    /// unanswered objections. `None` without a map or before anything was mapped.
    pub(super) fn debate_state_block(&self) -> Option<String> {
        if !self.argument_map_enabled || self.argument_map.theses.is_empty() {
            return None;
        }
        let lang = self.config.discussion_language.as_str();
        let objections = self.argument_map.unanswered_objections();
        let mut theses: Vec<&crate::models::argument_map::ThesisNode> = self.argument_map.theses.iter().filter(|t| !t.label.is_empty()).collect();
        theses.sort_by_key(|t| std::cmp::Reverse(t.arguments.iter().map(|a| a.count_all()).sum::<usize>()));
        let thesis_lines: Vec<String> = theses
            .iter()
            .take(constants::DEBATE_STATE_MAX_THESES)
            .map(|t| {
                let args = t.arguments.iter().map(|a| a.count_all()).sum::<usize>();
                let open = objections.iter().filter(|o| o.thesis_label == t.label).count();
                match lang {
                    "en" => format!("- {} ({}): {} argument(s), {} open objection(s)", t.label, t.speaker_name, args, open),
                    "zh" => format!("- {}（{}）：{}条论据，{}条未回应的反驳", t.label, t.speaker_name, args, open),
                    _ => format!("- {} ({}) : {} argument(s), {} objection(s) ouverte(s)", t.label, t.speaker_name, args, open),
                }
            })
            .collect();
        let name_of = |id: &str| self.gladiateurs.iter().find(|g| g.config.id == id).map(|g| g.config.name.clone()).unwrap_or_else(|| id.to_string());
        let objection_lines: Vec<String> = objections
            .iter()
            .rev()
            .take(constants::DEBATE_STATE_MAX_OBJECTIONS)
            .map(|o| match lang {
                "en" => format!("- \"{}\" ({} to {})", o.text, o.by_name, name_of(&o.debtor_id)),
                "zh" => format!("- \"{}\"（{}对{}）", o.text, o.by_name, name_of(&o.debtor_id)),
                _ => format!("- « {} » ({} à {})", o.text, o.by_name, name_of(&o.debtor_id)),
            })
            .collect();
        let mut block = thesis_lines.join("\n");
        if !objection_lines.is_empty() {
            let header = match lang {
                "en" => "Unanswered objections:",
                "zh" => "未回应的反驳：",
                _ => "Objections sans réponse :",
            };
            block.push_str(&format!("\n{header}\n{}", objection_lines.join("\n")));
        }
        Some(block)
    }

    /// The objections still unanswered, for the extraction prompt (v1.20.1): the
    /// extractor is told whom they hit, so a speaker's answer is nested under the
    /// objection (`targets_argument`) instead of lying flat on the thesis.
    pub(super) fn build_open_objections_block(&self) -> String {
        if !self.config.discussion_mode.rewards_depth() {
            return String::new();
        }
        let objections = self.argument_map.unanswered_objections();
        if objections.is_empty() {
            return String::new();
        }
        let lang = self.config.discussion_language.as_str();
        let name_of = |id: &str| self.gladiateurs.iter().find(|g| g.config.id == id).map(|g| g.config.name.clone()).unwrap_or_else(|| id.to_string());
        let lines: Vec<String> = objections
            .iter()
            .rev()
            .take(constants::ARGMAP_PROMPT_MAX_OBJECTIONS)
            .map(|o| match lang {
                "en" => format!("- \"{}\" (objection from {} to {})", o.text, o.by_name, name_of(&o.debtor_id)),
                "zh" => format!("- \"{}\"（{}对{}的反驳）", o.text, o.by_name, name_of(&o.debtor_id)),
                _ => format!("- « {} » (objection de {} à {})", o.text, o.by_name, name_of(&o.debtor_id)),
            })
            .collect();
        let header = match lang {
            "en" => "Objections still unanswered — when a speaker answers one in these exchanges, attach the answer to it (targets_argument = its exact text):",
            "zh" => "尚未回应的反驳——当有发言者在这些交流中作出回应时，把回应挂在该反驳之下（targets_argument = 其原文）：",
            _ => "Objections encore sans réponse — si un intervenant y répond dans ces échanges, rattache sa réponse à l'objection (targets_argument = son texte exact) :",
        };
        format!("{header}\n{}\n", lines.join("\n"))
    }

    /// Format existing theses and their arguments (all depths) for the extraction prompt.
    /// Arguments are rendered recursively with indentation, truncated for prompt compactness,
    /// and capped at `ARGMAP_PROMPT_MAX_EXISTING_ARGUMENTS` total argument lines.
    pub(super) fn build_existing_arguments_context(&self) -> String {
        if self.argument_map.theses.is_empty() {
            return String::new();
        }
        let mut lines = Vec::new();
        let mut arg_count = 0;

        for thesis in &self.argument_map.theses {
            lines.push(format!("  - {}", thesis.label));
            Self::collect_argument_lines(
                &thesis.arguments,
                &mut lines,
                &mut arg_count,
                2, // starting indent level (2 = "    - ")
            );
        }

        lines.join("\n")
    }

    /// Recursively collect argument lines with indentation for the extraction prompt.
    pub(super) fn collect_argument_lines(
        arguments: &[ArgumentNode],
        lines: &mut Vec<String>,
        arg_count: &mut usize,
        indent_level: usize,
    ) {
        for arg in arguments {
            if *arg_count >= constants::ARGMAP_PROMPT_MAX_EXISTING_ARGUMENTS {
                return;
            }
            let indent = "  ".repeat(indent_level);
            let icon = ArgumentMap::arg_icon(&arg.arg_type);
            let label = truncate_at_word_boundary(&arg.label, constants::ARGMAP_PROMPT_LABEL_CHARS);
            lines.push(format!("{indent}- {icon} {}: {label}", arg.speaker_name));
            *arg_count += 1;

            Self::collect_argument_lines(&arg.children, lines, arg_count, indent_level + 1);
        }
    }
}
