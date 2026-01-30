use crate::memory::Message;
use crate::memory_db::MemoryDatabase;
use std::sync::Arc;
use tracing::{debug, info};
/
#[derive(Debug, Clone)]
pub struct RetrievalPlan {
    /
    pub needs_retrieval: bool,

    /
    pub use_tier1: bool,
    pub use_tier2: bool,
    pub use_tier3: bool,

    /
    pub cross_session_search: bool,

    /
    pub semantic_search: bool,
    pub keyword_search: bool,
    pub temporal_search: bool,

    /
    pub max_messages: usize,
    pub max_tokens: usize,

    /
    pub target_compression: f32,

    /
    pub search_topics: Vec<String>,
}
impl Default for RetrievalPlan {
    fn default() -> Self {
        Self {
            needs_retrieval: false,
            use_tier1: true,
            use_tier2: false,
            use_tier3: false,
            cross_session_search: false,
            semantic_search: false,
            keyword_search: false,
            temporal_search: false,
            max_messages: 100,
            max_tokens: 4000,
            target_compression: 0.3,
            search_topics: Vec::new(),
        }
    }
}
/
pub struct RetrievalPlanner {
    database: Arc<MemoryDatabase>,
    recent_threshold_messages: usize,
    max_retrieval_time_ms: u64,
}
impl RetrievalPlanner {
    /
    pub fn new(database: Arc<MemoryDatabase>) -> Self {
        Self {
            database,
            recent_threshold_messages: 20,
            max_retrieval_time_ms: 200,
        }
    }

    /
    pub async fn create_plan(
        &self,
        session_id: &str,
        current_messages: &[Message],
        max_context_tokens: usize,
        user_query: Option<&str>,
        has_past_refs: bool,
    ) -> anyhow::Result<RetrievalPlan> {
        let mut plan = RetrievalPlan {
            max_tokens: max_context_tokens,
            ..Default::default()
        };


        let mut has_past_references_in_query = false;
        if let Some(query) = user_query {

            if self.is_cross_session_query(query, session_id) {
                plan.needs_retrieval = true;
                plan.cross_session_search = true;
                plan.search_topics = self.extract_topics_from_query(query);
            }


            has_past_references_in_query = self.has_past_references_in_text(query);
        }

        if !has_past_references_in_query && has_past_refs {
            has_past_references_in_query = true;
        }

        if !plan.needs_retrieval && !self.needs_retrieval(current_messages, max_context_tokens) {

            if has_past_references_in_query {
                plan.needs_retrieval = true;
                debug!("Retrieval needed: query asks for past content");
            } else {
                debug!("No retrieval needed - within context limits and no past references");
                return Ok(plan);
            }
        }

        plan.needs_retrieval = true;


        plan.use_tier1 = true;


        let analysis = self.analyze_conversation(current_messages, user_query).await?;


        self.plan_tier_usage(&mut plan, &analysis, session_id, has_past_references_in_query).await?;


        self.plan_search_strategies(&mut plan, &analysis, user_query);


        if plan.search_topics.is_empty() {
            plan.search_topics = analysis.extracted_topics;
        }


        self.adjust_limits(&mut plan, current_messages, max_context_tokens);

        info!(
            "Created retrieval plan: Tiers({}{}{}), CrossSession({}), Search({}{}{}), PastRefs={}",
            if plan.use_tier1 { "1" } else { "" },
            if plan.use_tier2 { "2" } else { "" },
            if plan.use_tier3 { "3" } else { "" },
            plan.cross_session_search,
            if plan.semantic_search { "S" } else { "" },
            if plan.keyword_search { "K" } else { "" },
            if plan.temporal_search { "T" } else { "" },
            has_past_references_in_query
        );

        Ok(plan)
    }

    /
    fn needs_retrieval(&self, messages: &[Message], max_tokens: usize) -> bool {
        if messages.len() <= 1 {
            return false;
        }


        let estimated_tokens: usize = messages.iter()
            .map(|m| m.content.len() / 4)
            .sum();

        estimated_tokens > max_tokens
    }
    /
    fn is_cross_session_query(&self, query: &str, _current_session_id: &str) -> bool {
        let cross_session_patterns = [
            "previously", "before", "earlier", "last time", "yesterday",
            "do you remember", "we discussed", "we talked about",
            "what did we talk", "remember when", "recall",
        ];

        let query_lower = query.to_lowercase();


        cross_session_patterns.iter().any(|pattern| query_lower.contains(pattern))
    }

    /
    pub fn has_past_references_in_text(&self, text: &str) -> bool {
        let reference_patterns = [
            "earlier", "before", "previous", "last time", "yesterday",
            "we discussed", "we talked about", "remember", "recall",
            "did we talk", "have we discussed", "what did we say",
            "what was said", "mentioned earlier", "previously mentioned",
        ];

        let text_lower = text.to_lowercase();
        reference_patterns.iter().any(|p| text_lower.contains(p))
    }
    /
    fn extract_topics_from_query(&self, query: &str) -> Vec<String> {
        let words: Vec<&str> = query.split_whitespace().collect();
        if words.len() < 3 {
            return vec![query.to_string()];
        }


        let topic = words.iter()
            .rev()
            .take(4)
            .rev()
            .copied()
            .collect::<Vec<&str>>()
            .join(" ");

        vec![topic]
    }

    /
    async fn analyze_conversation(
        &self,
        messages: &[Message],
        user_query: Option<&str>,
    ) -> anyhow::Result<ConversationAnalysis> {
        let mut analysis = ConversationAnalysis {
            extracted_topics: self.extract_topics(messages),
            has_past_references: self.has_past_references_in_messages(messages),
            ..Default::default()
        };


        if let Some(query) = user_query {
            analysis.requires_specific_details = self.requires_specific_details(query);
            analysis.query_complexity = self.assess_query_complexity(query);
        }


        analysis.conversation_length = messages.len();
        analysis.recency_pattern = self.analyze_recency_pattern(messages);

        Ok(analysis)
    }

    /
    async fn plan_tier_usage(
        &self,
        plan: &mut RetrievalPlan,
        analysis: &ConversationAnalysis,
        session_id: &str,
        has_past_references_in_query: bool,
    ) -> anyhow::Result<()> {
        let has_summaries = self.database.summaries
            .get_session_summaries(session_id)
            .map(|summaries| !summaries.is_empty())
            .unwrap_or_else(|e| {
                debug!("Database error checking summaries: {}", e);
                false
            });

        plan.use_tier2 = has_summaries;


        let has_db_messages = self.check_if_session_has_db_messages(session_id).await?;



        if has_past_references_in_query && has_db_messages {
            plan.use_tier3 = true;
            debug!("Query asks for past content, using Tier 3 (database)");
        }


        if analysis.requires_specific_details && has_db_messages {
            plan.use_tier3 = true;
            debug!("Specific details requested, using Tier 3");
        }


        if plan.cross_session_search {
            plan.use_tier3 = true;
            debug!("Cross-session search, using Tier 3");
        }


        if analysis.conversation_length > 30 && has_db_messages && !plan.use_tier3 {
            plan.use_tier3 = true;
            debug!("Long conversation ({} messages), using Tier 3", analysis.conversation_length);
        }


        if analysis.has_past_references && has_db_messages && !plan.use_tier3 {
            plan.use_tier3 = true;
            debug!("Past references in messages, using Tier 3");
        }

        if analysis.conversation_length > 100 {
            plan.target_compression = 0.2;
        }

        Ok(())
    }

    /
    async fn check_if_session_has_db_messages(&self, session_id: &str) -> anyhow::Result<bool> {

        match self.database.conversations.get_session_messages(session_id, Some(1), Some(0)) {
            Ok(messages) => Ok(!messages.is_empty()),
            Err(e) => {
                debug!("Error checking DB for session {}: {}", session_id, e);
                Ok(false)
            }
        }
    }

    /
    fn plan_search_strategies(
        &self,
        plan: &mut RetrievalPlan,
        analysis: &ConversationAnalysis,
        user_query: Option<&str>,
    ) {

        plan.semantic_search = analysis.query_complexity > 0.5 || (analysis.extracted_topics.is_empty() && !plan.cross_session_search);


        plan.keyword_search = analysis.requires_specific_details
            || analysis.has_past_references
            || plan.cross_session_search
            || !analysis.extracted_topics.is_empty();


        plan.temporal_search = self.has_temporal_references(user_query.unwrap_or(""));
    }

    /
    fn adjust_limits(
        &self,
        plan: &mut RetrievalPlan,
        current_messages: &[Message],
        max_context_tokens: usize,
    ) {
        let current_tokens: usize = current_messages.iter()
            .map(|m| m.content.len() / 4)
            .sum();

        let available_for_retrieval = max_context_tokens.saturating_sub(current_tokens);


        let estimated_messages = available_for_retrieval / 50;
        plan.max_messages = estimated_messages.clamp(10, 100);
    }

    /
    fn extract_topics(&self, messages: &[Message]) -> Vec<String> {
        let mut topics = Vec::new();

        for message in messages.iter().rev().filter(|m| m.role == "user").take(3) {
            let words: Vec<&str> = message.content.split_whitespace().collect();

            for i in 0..words.len().saturating_sub(2) {
                let word_lower = words[i].to_lowercase();

                if word_lower == "about" || word_lower == "regarding" {
                    let topic = words[i + 1..].iter()
                        .take(3)
                        .copied()
                        .collect::<Vec<&str>>()
                        .join(" ");

                    if !topic.is_empty() {
                        topics.push(topic);
                    }
                }

                if ["what", "how", "why", "when", "where", "who"].contains(&word_lower.as_str()) {
                    let topic = words[i + 1..].iter()
                        .take(4)
                        .copied()
                        .collect::<Vec<&str>>()
                        .join(" ");

                    if !topic.is_empty() {
                        topics.push(topic);
                    }
                }
            }
        }

        topics.dedup();
        topics.truncate(3);

        topics
    }

    /
    fn has_past_references_in_messages(&self, messages: &[Message]) -> bool {
        let reference_patterns = [
            "earlier", "before", "previous", "last time", "yesterday",
            "we discussed", "we talked about", "remember", "recall",
        ];

        for message in messages.iter().rev().take(5) {
            let content_lower = message.content.to_lowercase();
            if reference_patterns.iter().any(|p| content_lower.contains(p)) {
                return true;
            }
        }

        false
    }

    /
    fn requires_specific_details(&self, query: &str) -> bool {
        let detail_patterns = [
            "exactly", "specifically", "in detail", "step by step",
            "the code", "the number", "the date", "the name",
            "show me", "give me", "tell me",
        ];

        let query_lower = query.to_lowercase();
        detail_patterns.iter().any(|p| query_lower.contains(p))
    }

    /
    fn assess_query_complexity(&self, query: &str) -> f32 {
        let words: Vec<&str> = query.split_whitespace().collect();

        if words.len() < 3 {
            return 0.2;
        }

        let mut complexity = 0.0;
        complexity += (words.len() as f32).min(50.0) / 100.0;

        let clause_count = query.split(&[',', ';', '&']).count();
        complexity += (clause_count as f32).min(5.0) / 10.0;

        let technical_terms = ["code", "function", "algorithm", "parameter", "variable"];
        for term in technical_terms {
            if query.to_lowercase().contains(term) {
                complexity += 0.2;
            }
        }

        complexity.min(1.0)
    }

    /
    fn analyze_recency_pattern(&self, messages: &[Message]) -> RecencyPattern {
        if messages.len() < 5 {
            return RecencyPattern::RecentOnly;
        }

        let recent_topics = self.extract_topics(&messages[messages.len().saturating_sub(5)..]);
        let older_topics = self.extract_topics(&messages[..messages.len().saturating_sub(5)]);

        let overlap = recent_topics.iter()
            .filter(|topic| older_topics.contains(topic))
            .count();

        match overlap {
            0 => RecencyPattern::TopicJumping,
            1 => RecencyPattern::Mixed,
            _ => RecencyPattern::TopicContinuation,
        }
    }

    /
    fn has_temporal_references(&self, query: &str) -> bool {
        let temporal_patterns = [
            "yesterday", "today", "tomorrow", "last week", "last month",
            "earlier", "before", "previously", "in the past",
        ];

        let query_lower = query.to_lowercase();
        temporal_patterns.iter().any(|p| query_lower.contains(p))
    }
}
/
#[derive(Debug, Default)]
struct ConversationAnalysis {
    extracted_topics: Vec<String>,
    has_past_references: bool,
    requires_specific_details: bool,
    query_complexity: f32,
    conversation_length: usize,
    recency_pattern: RecencyPattern,
}
/
#[derive(Debug, Clone, PartialEq, Default)]
enum RecencyPattern {
    #[default]
    RecentOnly,
    TopicContinuation,
    TopicJumping,
    Mixed,
}
impl Clone for RetrievalPlanner {
    fn clone(&self) -> Self {
        Self {
            database: self.database.clone(),
            recent_threshold_messages: self.recent_threshold_messages,
            max_retrieval_time_ms: self.max_retrieval_time_ms,
        }
    }
}
