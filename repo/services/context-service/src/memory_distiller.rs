//! Memory distillation / consolidation placeholder.

use serde_json::Value;

use crate::l1_policy::{
    detect_conflict_count, infer_preference_polarity, infer_temporal_state,
    supersedes_previous_from_input, KEYWORD_SEED_PHRASES,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsolidationMode {
    Incremental,
    Replay,
}

impl ConsolidationMode {
    pub fn supports_replay(self) -> bool {
        matches!(self, Self::Incremental | Self::Replay)
    }
}

#[derive(Debug, Clone, Default)]
pub struct DistilledEntity {
    pub entity_type: String,
    pub name: String,
    pub relationship: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Default)]
pub struct DistilledMemoryView {
    pub summary_text: String,
    pub keywords: Vec<String>,
    pub entities: Vec<DistilledEntity>,
    pub conflict_count: usize,
    pub temporal_state: String,
    pub supersedes_previous: bool,
    pub preference_polarity: String,
}

pub fn distill_message(user_message: &str, candidates: &[Value]) -> DistilledMemoryView {
    let normalized = user_message.trim();
    let mut keywords = collect_keywords(normalized);
    let mut summary_parts: Vec<String> = candidates
        .iter()
        .filter_map(|candidate| {
            candidate
                .get("content")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string())
        })
        .collect();
    summary_parts.truncate(3);

    if summary_parts.is_empty() && !normalized.is_empty() {
        summary_parts.push(normalized.chars().take(80).collect());
    }

    let entities = infer_entities(normalized, &keywords);
    for entity in &entities {
        if !keywords.iter().any(|keyword| keyword == &entity.name) {
            keywords.push(entity.name.clone());
        }
    }

    let preference_polarity = infer_preference_polarity(normalized).to_string();

    DistilledMemoryView {
        summary_text: summary_parts.join("；"),
        keywords,
        entities,
        conflict_count: detect_conflict_count(normalized),
        temporal_state: infer_temporal_state(normalized).to_string(),
        supersedes_previous: supersedes_previous_from_input(normalized),
        preference_polarity,
    }
}

fn collect_keywords(input: &str) -> Vec<String> {
    let mut keywords = vec![];
    for phrase in KEYWORD_SEED_PHRASES {
        if input.contains(phrase) && !keywords.iter().any(|existing| existing == phrase) {
            keywords.push(phrase.to_string());
        }
    }

    for word in input
        .split(|ch: char| !ch.is_alphanumeric() && !matches!(ch, '_' | '-'))
        .filter(|word| word.len() >= 4)
    {
        if !keywords.iter().any(|existing| existing == word) {
            keywords.push(word.to_string());
        }
    }

    keywords.truncate(8);
    keywords
}

fn infer_entities(input: &str, keywords: &[String]) -> Vec<DistilledEntity> {
    let mut entities = vec![];
    if input.contains("AI") || input.contains("人工智能") {
        entities.push(DistilledEntity {
            entity_type: "topic".to_string(),
            name: "AI".to_string(),
            relationship: "interested_in".to_string(),
            confidence: 0.84,
        });
    }
    if input.contains("创业") || keywords.iter().any(|keyword| keyword == "startup") {
        entities.push(DistilledEntity {
            entity_type: "topic".to_string(),
            name: "创业".to_string(),
            relationship: "interested_in".to_string(),
            confidence: 0.82,
        });
    }
    if input.contains("投资人") || input.contains("投资") {
        entities.push(DistilledEntity {
            entity_type: "persona".to_string(),
            name: "投资人".to_string(),
            relationship: "seeking_connection".to_string(),
            confidence: 0.78,
        });
    }
    if input.contains("技术合伙人") || input.contains("合伙人") {
        entities.push(DistilledEntity {
            entity_type: "persona".to_string(),
            name: "技术合伙人".to_string(),
            relationship: "seeking_connection".to_string(),
            confidence: 0.79,
        });
    }
    if input.contains("远程办公") || input.contains("远程") {
        entities.push(DistilledEntity {
            entity_type: "work_mode".to_string(),
            name: "远程办公".to_string(),
            relationship: "current_mode".to_string(),
            confidence: 0.76,
        });
    }
    for city in ["上海", "北京", "深圳", "杭州"] {
        if input.contains(city) {
            entities.push(DistilledEntity {
                entity_type: "location".to_string(),
                name: city.to_string(),
                relationship: "mentioned_location".to_string(),
                confidence: 0.72,
            });
        }
    }
    entities
}
