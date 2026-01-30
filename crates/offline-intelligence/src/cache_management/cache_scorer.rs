use regex::Regex;
use std::collections::HashMap;
use lazy_static::lazy_static;
lazy_static! {
    static ref KEY_PATTERNS: HashMap<&'static str, Regex> = {
        let mut m = HashMap::new();


        m.insert(
            "system_prompt",
            Regex::new(r"system|instruction|prompt|assistant_role").unwrap(),
        );


        m.insert(
            "code_related",
            Regex::new(r"def |function |class |import |return |print |code|program|algorithm|python|rust|javascript|java|c\+\+|sql|```").unwrap(),
        );


        m.insert(
            "important_concept",
            Regex::new(r"important|critical|crucial|essential|must|need|require|urgent|asap|priority|key|main|primary").unwrap(),
        );


        m.insert(
            "question",
            Regex::new(r"what|how|why|when|where|who|explain|describe|can you|could you|would you|should").unwrap(),
        );


        m.insert(
            "numeric",
            Regex::new(r"\d+|date|time|age|year|month|day|hour|minute|second").unwrap(),
        );

        m
    };
}
/
pub struct CacheEntryParams<'a> {
    pub key_hash: &'a str,
    pub key_data: Option<&'a [u8]>,
    pub key_type: &'a str,
    pub layer_index: i32,
    pub head_index: Option<i32>,
    pub access_count: i32,
    pub last_accessed_seconds_ago: f32,
    pub value_size_bytes: usize,
}
/
pub struct CacheEntryScorer {
    key_engagement: HashMap<String, f32>,
    config: CacheScoringConfig,
}
#[derive(Debug, Clone)]
pub struct CacheScoringConfig {
    pub recency_weight: f32,
    pub access_count_weight: f32,
    pub key_pattern_weight: f32,
    pub layer_weight: f32,
    pub head_weight: f32,
    pub value_size_weight: f32,
    pub engagement_decay: f32,
    pub min_engagement: f32,
    pub max_engagement: f32,
}
impl Default for CacheScoringConfig {
    fn default() -> Self {
        Self {
            recency_weight: 0.3,
            access_count_weight: 0.2,
            key_pattern_weight: 0.25,
            layer_weight: 0.1,
            head_weight: 0.05,
            value_size_weight: 0.1,
            engagement_decay: 0.95,
            min_engagement: 0.1,
            max_engagement: 1.0,
        }
    }
}
impl CacheEntryScorer {
    /
    pub fn new(config: CacheScoringConfig) -> Self {
        Self {
            key_engagement: HashMap::new(),
            config,
        }
    }
    /
    pub fn score_entry(&self, params: CacheEntryParams) -> f32 {
        let mut score = 0.0;
        score += self.score_recency(params.last_accessed_seconds_ago);
        score += self.score_access_count(params.access_count);
        score += self.score_key_patterns(params.key_data, params.key_type);
        score += self.score_layer_position(params.layer_index);
        score += self.score_head_position(params.head_index);
        score += self.score_value_size(params.value_size_bytes);
        score += self.score_key_engagement(params.key_hash);
        score.clamp(0.0, 1.0)
    }
    fn score_recency(&self, seconds_ago: f32) -> f32 {
        let recency_factor = 1.0 / (1.0 + seconds_ago / 3600.0);
        recency_factor * self.config.recency_weight
    }
    fn score_access_count(&self, access_count: i32) -> f32 {
        let normalized = (access_count as f32).min(100.0) / 100.0;
        normalized * self.config.access_count_weight
    }
    fn score_key_patterns(&self, key_data: Option<&[u8]>, key_type: &str) -> f32 {

        let mut pattern_score: f32 = 0.0;


        match key_type {
            "attention_key" | "attention_value" => pattern_score += 0.1,
            "ffn_key" | "ffn_value" => pattern_score += 0.05,
            _ => {}
        }


        if let Some(data) = key_data {
            if let Ok(key_str) = std::str::from_utf8(data) {
                for (pattern_name, regex) in KEY_PATTERNS.iter() {
                    if regex.is_match(key_str) {
                        let weight = match *pattern_name {
                            "system_prompt" => 0.8,
                            "code_related" => 0.7,
                            "important_concept" => 0.9,
                            "question" => 0.6,
                            "numeric" => 0.5,
                            _ => 0.3,
                        };
                        pattern_score += weight;
                    }
                }
            }
        }

        pattern_score.min(1.0) * self.config.key_pattern_weight
    }
    fn score_layer_position(&self, layer_index: i32) -> f32 {

        let layer_factor = if layer_index < 10 {
            0.9
        } else if layer_index < 20 {
            0.7
        } else {
            0.5
        };
        layer_factor * self.config.layer_weight
    }
    fn score_head_position(&self, head_index: Option<i32>) -> f32 {
        if let Some(head) = head_index {

            let head_factor = if head < 4 { 0.8 } else { 0.5 };
            head_factor * self.config.head_weight
        } else {
            0.0
        }
    }
    fn score_value_size(&self, size_bytes: usize) -> f32 {

        let size_factor = (size_bytes as f32).min(10000.0) / 10000.0;
        size_factor * self.config.value_size_weight
    }
    fn score_key_engagement(&self, key_hash: &str) -> f32 {
        self.key_engagement.get(key_hash).map_or(0.0, |&e| e * 0.3)
    }
    pub fn update_engagement(&mut self, key_hash: &str, was_retrieved: bool) {
        let engagement_increase = if was_retrieved { 0.15 } else { 0.05 };

        let current = self.key_engagement.entry(key_hash.to_string()).or_insert(0.3);
        *current = (*current + engagement_increase)
            .min(self.config.max_engagement)
            .max(self.config.min_engagement);


        self.decay_other_keys(key_hash);
    }
    fn decay_other_keys(&mut self, current_key: &str) {
        for (key, engagement) in self.key_engagement.iter_mut() {
            if *key != current_key {
                *engagement = (*engagement * self.config.engagement_decay)
                    .max(self.config.min_engagement);
            }
        }
    }
    /
    pub fn should_preserve_entry(
        &self,
        importance_score: f32,
        key_type: &str,
        layer_index: i32,
        config_threshold: f32,
    ) -> bool {
        let base_preservation = match key_type {
            "attention_key" | "attention_value" => 0.8,
            "ffn_key" | "ffn_value" => 0.6,
            _ => 0.5,
        };

        let layer_factor = if layer_index < 8 { 1.2 } else { 1.0 };
        let combined_score = importance_score * layer_factor;

        combined_score >= config_threshold || base_preservation >= 0.7
    }
    /
    pub fn extract_keywords(&self, key_data: Option<&[u8]>) -> Vec<String> {
        let mut keywords = Vec::new();

        if let Some(data) = key_data {
            if let Ok(key_str) = std::str::from_utf8(data) {

                let words: Vec<&str> = key_str.split_whitespace().collect();
                for word in words.iter().filter(|w| w.len() > 3) {
                    let word_lower = word.to_lowercase();


                    if !self.is_stop_word(&word_lower) {
                        keywords.push(word_lower);
                    }
                }
            }
        }

        keywords.dedup();
        keywords.truncate(5);
        keywords
    }

    fn is_stop_word(&self, word: &str) -> bool {
        let stop_words = [
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
            "of", "with", "by", "is", "am", "are", "was", "were", "be", "been",
            "being", "have", "has", "had", "do", "does", "did", "will", "would",
            "shall", "should", "may", "might", "must", "can", "could", "this",
            "that", "these", "those", "it", "its", "it's",
        ];
        stop_words.contains(&word)
    }
}
/
impl crate::cache_management::cache_extractor::CacheEntryScorer for CacheEntryScorer {
    fn extract_keywords(&self, key_data: Option<&[u8]>) -> Vec<String> {

        self.extract_keywords(key_data)
    }
}

