//! Web search, Wikipedia and RAG retrieval for the speakers.

use super::*;

impl DiscussionEngine {
    // ===== Web Search =====

    /// Check if the web search pool has remaining credits.
    /// Returns (can_search, max_queries_this_turn).
    /// Pool is shared between all gladiateurs, max 1 per gladiateur per turn.
    pub(super) fn can_search_web(&self, global_usage: u32) -> (bool, u32) {
        let pool = self.config.web_search_pool;
        if pool == 0 || self.tavily_client.is_none() {
            return (false, 0);
        }
        let pool_remaining = pool.saturating_sub(self.web_searches_used_pool);
        let max_queries = pool_remaining.min(1);
        let global_remaining = constants::TAVILY_FREE_MONTHLY_QUOTA.saturating_sub(global_usage);
        let max_queries = max_queries.min(global_remaining);
        (max_queries > 0, max_queries)
    }

    /// Execute web search for a speaker. Returns (formatted_context, queries_executed_count).
    /// Uses `&self` — no fields mutated; counter increment happens at call site.
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn process_web_search(
        &self,
        system_prompt: &str,
        speaker_id: &str,
        speaker_name: &str,
        max_queries: u32,
        search_directive: &str,
        recent_context: &str,
        forced_queries: Option<Vec<String>>,
        llm_params: &LlmParams,
        channel: &Channel<ArenaEvent>,
        past_queries: &[String],
        other_queries: &[(String, String)],
    ) -> (Option<String>, u32, Vec<String>, Vec<WebSourceInfo>) {
        // 1. Determine queries
        let queries: Vec<String> = if let Some(forced) = forced_queries {
            forced.into_iter().take(max_queries as usize).collect()
        } else {
            // LLM decision (non-streaming, JSON)
            let prompt = prompt_builder::build_web_search_decision_prompt(
                &self.config.topic,
                recent_context,
                search_directive,
                max_queries,
                &self.config.discussion_language,
                past_queries,
                other_queries,
            );
            let request = LlmRequest::new(system_prompt, &prompt, llm_params, CallKind::SearchDecision)
                .json()
                .speaker(speaker_id);
            let raw = match self.chat_text(&request).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(error = %e, "Web search decision LLM call failed — skipping search");
                    return (None, 0, Vec::new(), Vec::new());
                }
            };
            let decision = json_parser::parse_json_response::<json_parser::SearchDecisionResponse>(&raw)
                .unwrap_or_default();

            if !decision.needs_search || decision.queries.is_empty() {
                return (None, 0, Vec::new(), Vec::new());
            }
            // Hard dedup: filter out near-duplicates of speaker's own past queries
            let deduped: Vec<String> = decision.queries
                .into_iter()
                .take(max_queries as usize)
                .filter(|q| {
                    if is_duplicate_query(q, past_queries) {
                        tracing::info!(speaker = %speaker_name, query = %q, "Web query is near-duplicate of own past — skipping");
                        false
                    } else {
                        true
                    }
                })
                .collect();
            if deduped.is_empty() {
                tracing::info!(speaker = %speaker_name, "All web queries are near-duplicates — skipping web search");
                return (None, 0, Vec::new(), Vec::new());
            }
            deduped
        };

        if queries.is_empty() {
            return (None, 0, Vec::new(), Vec::new());
        }

        // 2. Execute each search
        let tavily = match self.tavily_client.as_ref() {
            Some(c) => c,
            None => return (None, 0, Vec::new(), Vec::new()),
        };
        let mut all_results: Vec<(String, crate::tavily::TavilySearchResponse)> = Vec::new();
        let mut executed_count = 0u32;

        for query in &queries {
            if self.cancel_token.is_cancelled() {
                break;
            }

            match tavily.search(query, self.cancel_token.clone()).await {
                Ok(response) => {
                    all_results.push((query.clone(), response));
                    executed_count += 1;
                    if let Err(e) = repository::increment_tavily_usage(&self.db).await {
                        tracing::warn!(error = %e, "Failed to increment Tavily usage counter");
                    }
                }
                Err(TavilyError::QuotaExceeded) => {
                    tracing::warn!("Tavily quota exceeded — stopping all searches");
                    break;
                }
                Err(TavilyError::InvalidKey) => {
                    tracing::error!("Tavily API key invalid — stopping all searches");
                    break;
                }
                Err(TavilyError::Cancelled) => break,
                Err(e) => {
                    tracing::warn!(query = %query, error = %e, "Tavily search failed — skipping");
                    continue;
                }
            }
        }

        if executed_count == 0 {
            return (None, 0, Vec::new(), Vec::new());
        }

        // 3. Emit batched event (with the sources actually rendered into the prompt)
        let executed_queries: Vec<String> = all_results.iter().map(|(q, _)| q.clone()).collect();
        let total_results: u32 = all_results.iter().map(|(_, r)| r.results.len() as u32).sum();
        let results: Vec<WebSourceInfo> = all_results
            .iter()
            .flat_map(|(_, r)| r.results.iter().take(constants::SEARCH_WEB_RENDER_LIMIT))
            .filter(|r| !r.url.is_empty())
            .map(|r| WebSourceInfo {
                title: r.title.clone(),
                url: r.url.clone(),
                domain: source::domain_of(&r.url),
                snippet: source::snippet_of(&r.content),
            })
            .collect();
        let _ = channel.send(ArenaEvent::WebSearchPerformed {
            speaker_id: speaker_id.to_string(),
            speaker_name: speaker_name.to_string(),
            queries: executed_queries.clone(),
            results_count: total_results,
            pool_used: self.web_searches_used_pool + executed_count,
            results: results.clone(),
        });

        // 4. Format for prompt injection
        let lang = &self.config.discussion_language;
        let ctx = prompt_builder::build_search_results_context(&all_results, lang);
        tracing::info!(
            speaker = %speaker_name,
            ctx_len = ctx.len(),
            ctx_preview = %truncate_str(&ctx, 300),
            "Web search context injected into prompt"
        );
        (
            Some(ctx),
            executed_count,
            executed_queries,
            results,
        )
    }

    /// Remember references injected into a prompt (for the synthesis "Sources" block).
    pub(super) fn register_sources(&mut self, speaker_name: &str, kind: SourceKind, items: impl IntoIterator<Item = (String, String)>) {
        let turn = self.current_turn;
        self.sources_registry.extend(items.into_iter().map(|(title, url)| SourceRecord {
            kind,
            turn,
            speaker_name: speaker_name.to_string(),
            title,
            url,
        }));
    }

    // ===== Search helpers =====

    /// Ask the LLM to pick a search query for forced first-turn search.
    /// Returns the LLM-chosen query, or `fallback` if LLM fails/returns empty.
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn pick_forced_query(
        &self,
        system_prompt: &str,
        prompt: &str,
        llm_params: &LlmParams,
        fallback: String,
        speaker_id: &str,
        speaker_name: &str,
        search_type: &str,
    ) -> String {
        let request = LlmRequest::new(system_prompt, prompt, llm_params, CallKind::SearchDecision)
            .json()
            .speaker(speaker_id);
        let decision = match self.chat_text(&request).await {
            Ok(raw) => {
                tracing::info!(speaker = %speaker_name, search_type, raw = %raw, "Forced search query LLM response");
                json_parser::parse_json_response::<json_parser::SearchDecisionResponse>(&raw)
                    .unwrap_or_default()
            }
            Err(e) => {
                tracing::warn!(speaker = %speaker_name, search_type, error = %e, "Forced search query LLM call failed");
                json_parser::SearchDecisionResponse::default()
            }
        };
        decision.queries.into_iter().next().unwrap_or(fallback)
    }

    // ===== Wikipedia Search =====

    /// Wikipedia article URL (title with underscores, percent-encoded for accents and punctuation).
    pub(super) fn wiki_article_url(wiki_lang: &str, title: &str) -> String {
        format!("https://{}.wikipedia.org/wiki/{}", wiki_lang, urlencoding::encode(&title.replace(' ', "_")))
    }

    /// Build clickable Wikipedia article URLs from a search response.
    pub(super) fn build_article_urls(response: &crate::wikipedia::WikiSearchResponse, wiki_lang: &str) -> Vec<String> {
        response.query.as_ref()
            .map(|q| q.pages.iter().map(|p| Self::wiki_article_url(wiki_lang, &p.title)).collect())
            .unwrap_or_default()
    }

    /// Title, link and excerpt of the articles rendered into the prompt (relevance order, same cap).
    pub(super) fn build_wiki_articles(response: &crate::wikipedia::WikiSearchResponse, wiki_lang: &str) -> Vec<WikiSourceInfo> {
        let Some(q) = response.query.as_ref() else { return Vec::new() };
        let mut pages: Vec<&crate::wikipedia::WikiPage> = q.pages.iter().filter(|p| !p.extract.is_empty()).collect();
        pages.sort_by_key(|p| p.index);
        pages
            .into_iter()
            .take(constants::WIKI_RESULTS_LIMIT as usize)
            .map(|p| WikiSourceInfo {
                title: p.title.clone(),
                url: Self::wiki_article_url(wiki_lang, &p.title),
                snippet: source::snippet_of(&p.extract),
            })
            .collect()
    }

    /// Check if the wiki search pool has remaining credits.
    /// Returns (can_search, max_queries_this_turn).
    /// Pool is shared between all gladiateurs, max 1 per gladiateur per turn.
    pub(super) fn can_search_wiki(&self) -> (bool, u32) {
        let pool = self.config.wiki_search_pool;
        if pool == 0 {
            return (false, 0);
        }
        let remaining = pool.saturating_sub(self.wiki_searches_used_pool);
        let max_queries = remaining.min(1);
        (max_queries > 0, max_queries)
    }

    /// Raw Wikipedia search for the moderator (intro, surprise fact): returns the
    /// formatted context and the articles found, emits WikiSearchPerformed.
    pub(super) async fn process_wiki_search_intro(
        &mut self,
        query: &str,
        speaker_id: &str,
        speaker_name: &str,
        channel: &Channel<ArenaEvent>,
    ) -> (Option<String>, Vec<WikiSourceInfo>) {
        let lang = self.config.discussion_language.clone();
        tracing::info!(query = %query, lang = %lang, speaker = %speaker_name, "Wikipedia intro search");
        match self.wiki_client.search(query, &lang, self.cancel_token.clone()).await {
            Ok((response, actual_lang)) => {
                let has_results = response.query.as_ref().is_some_and(|q| !q.pages.is_empty());
                if has_results {
                    let article_urls = Self::build_article_urls(&response, &actual_lang);
                    let articles = Self::build_wiki_articles(&response, &actual_lang);
                    let results_count = response.query.as_ref().map(|q| q.pages.len() as u32).unwrap_or(0);

                    tracing::info!(speaker = %speaker_name, urls = ?article_urls, "Wikipedia intro results found");

                    // Emit event so the UI shows the wiki badge on the intro message
                    let _ = channel.send(ArenaEvent::WikiSearchPerformed {
                        speaker_id: speaker_id.to_string(),
                        speaker_name: speaker_name.to_string(),
                        queries: vec![query.to_string()],
                        results_count,
                        pool_used: 0, // IArbitre intro does not consume pool
                        article_urls,
                        articles: articles.clone(),
                    });
                    self.register_sources(speaker_name, SourceKind::Wiki, articles.iter().map(|a| (a.title.clone(), a.url.clone())));

                    let results = vec![(query.to_string(), response)];
                    let ctx = prompt_builder::build_wiki_results_context(&results, &lang);
                    tracing::info!(
                        speaker = %speaker_name,
                        ctx_len = ctx.len(),
                        ctx_preview = %truncate_str(&ctx, 300),
                        "Wikipedia intro context injected into prompt"
                    );
                    (Some(ctx), articles)
                } else {
                    tracing::info!("Wikipedia intro search returned no results");
                    (None, Vec::new())
                }
            }
            Err(crate::wikipedia::error::WikiError::Cancelled) => (None, Vec::new()),
            Err(e) => {
                tracing::warn!(error = %e, "Wikipedia intro search failed");
                (None, Vec::new())
            }
        }
    }

    /// Execute Wikipedia search for a gladiateur. Returns (formatted_context, queries_executed_count).
    pub(super) async fn process_wiki_search(
        &mut self,
        glad_idx: usize,
        query: String,
        channel: &Channel<ArenaEvent>,
    ) -> (Option<String>, u32) {
        let lang = self.config.discussion_language.clone();
        let speaker_id = self.gladiateurs[glad_idx].config.id.clone();
        let speaker_name = self.gladiateurs[glad_idx].config.name.clone();

        tracing::info!(speaker = %speaker_name, query = %query, lang = %lang, "Wikipedia search for gladiateur");

        let (response, actual_lang) = match self.wiki_client.search(&query, &lang, self.cancel_token.clone()).await {
            Ok(r) => r,
            Err(crate::wikipedia::error::WikiError::Cancelled) => return (None, 0),
            Err(e) => {
                tracing::warn!(query = %query, error = %e, "Wikipedia search failed");
                return (None, 0);
            }
        };

        let has_results = response.query.as_ref().is_some_and(|q| !q.pages.is_empty());
        if !has_results {
            tracing::info!(speaker = %speaker_name, query = %query, "Wikipedia returned no results");
            return (None, 0);
        }

        let article_urls = Self::build_article_urls(&response, &actual_lang);
        let articles = Self::build_wiki_articles(&response, &actual_lang);

        tracing::info!(speaker = %speaker_name, urls = ?article_urls, "Wikipedia results found");

        let results_count = response.query.as_ref().map(|q| q.pages.len() as u32).unwrap_or(0);

        // Emit event
        let _ = channel.send(ArenaEvent::WikiSearchPerformed {
            speaker_id: speaker_id.to_string(),
            speaker_name: speaker_name.to_string(),
            queries: vec![query.clone()],
            results_count,
            pool_used: self.wiki_searches_used_pool + 1,
            article_urls,
            articles: articles.clone(),
        });
        self.register_sources(&speaker_name, SourceKind::Wiki, articles.into_iter().map(|a| (a.title, a.url)));

        // Format for prompt injection
        let all_results = vec![(query, response)];
        let ctx = prompt_builder::build_wiki_results_context(&all_results, &lang);
        tracing::info!(
            speaker = %speaker_name,
            ctx_len = ctx.len(),
            ctx_preview = %truncate_str(&ctx, 300),
            "Wikipedia context injected into prompt"
        );
        (Some(ctx), 1)
    }

    // ── RAG knowledge base query ──────────────────────────────────────

    /// Query the RAG store for relevant chunks and emit event.
    /// Returns formatted context string for prompt injection, or None.
    /// Uses per-speaker cache with TTL to avoid redundant queries.
    pub(super) async fn process_rag_query(
        &mut self,
        speaker_id: &str,
        speaker_name: &str,
        glad_idx: usize,
        channel: &Channel<ArenaEvent>,
    ) -> Option<String> {
        if self.rag_store.as_ref().is_none_or(|s| s.is_empty()) {
            return None;
        }

        // 1. Cache hit check
        if let Some(entry) = self.rag_cache.get(speaker_id) {
            if self.current_turn.saturating_sub(entry.cached_at_turn) < constants::RAG_CACHE_TTL_TURNS {
                tracing::info!(
                    speaker = %speaker_name,
                    turn = self.current_turn,
                    cached_at = entry.cached_at_turn,
                    "RAG cache hit"
                );
                let _ = channel.send(ArenaEvent::RagContextInjected {
                    speaker_id: speaker_id.to_string(),
                    speaker_name: speaker_name.to_string(),
                    chunks: entry.chunks.clone(),
                    cached: true,
                });
                return Some(entry.context_text.clone());
            }
        }

        // 2. Deferred embeddings (no-op if ready); without them the store answers lexically (BM25)
        if let Some(ref mut store) = self.rag_store {
            store.ensure_embeddings().await;
        }

        // 3. Build query context
        let topic = truncate_str(&self.config.topic, constants::ORCH_TOPIC_FOR_SEARCH);
        let recent_raw = self.build_recent_exchanges(glad_idx);
        let recent = truncate_str(&recent_raw, constants::ORCH_RECENT_FOR_SEARCH);
        let context = format!("{topic}\n\n{recent}");

        // 4. Query RAG store — bind result to release borrow before cache insert
        let result = {
            let rag_store = self.rag_store.as_ref()?;
            let lang = &self.config.discussion_language;
            rag_store
                .query(
                    &context,
                    lang,
                    self.llm.as_ref(),
                    &self.gladiateurs[glad_idx].config.llm_params,
                    self.cancel_token.clone(),
                )
                .await
        };

        // 5. Process result + cache insert
        match result {
            Ok((ctx_text, chunks)) if !chunks.is_empty() => {
                tracing::info!(
                    speaker = %speaker_name,
                    turn = self.current_turn,
                    chunk_count = chunks.len(),
                    ctx_len = ctx_text.len(),
                    "RAG context injected"
                );
                let _ = channel.send(ArenaEvent::RagContextInjected {
                    speaker_id: speaker_id.to_string(),
                    speaker_name: speaker_name.to_string(),
                    chunks: chunks.clone(),
                    cached: false,
                });
                let refs: Vec<(String, String)> = chunks.iter().map(rag_source_ref).collect();
                self.register_sources(speaker_name, SourceKind::Rag, refs);
                self.rag_cache.insert(speaker_id.to_string(), RagCacheEntry {
                    cached_at_turn: self.current_turn,
                    context_text: ctx_text.clone(),
                    chunks,
                });
                Some(ctx_text)
            }
            Ok(_) => None, // Empty results — not cached, will retry next turn
            Err(LlmError::Cancelled) => None,
            Err(e) => {
                tracing::warn!(
                    speaker = %speaker_name,
                    error = %e,
                    "RAG query failed — continuing without knowledge base"
                );
                None
            }
        }
    }
}
