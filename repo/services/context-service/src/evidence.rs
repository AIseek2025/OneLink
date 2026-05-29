//! L1 evidence assembly: structured scoring, optional graph expansion, optional rerank second-pass.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::app_state::MemoryArtifactRecord;
use crate::l1_policy;
use crate::memory_store::{MemoryEntityLinkRecord, MemorySearchItem, MemorySummaryRecord};
use crate::policy::PolicyConfigStore;
use crate::store::{MemoryBackend, UserMemorySnapshot};

#[derive(Debug, Default)]
pub(crate) struct L1Evidence {
    pub selected_summary_ids: Vec<String>,
    pub selected_memory_ids: Vec<String>,
    pub evidence_count: usize,
    pub summary_hits: usize,
    pub artifact_hits: usize,
    pub entity_hits: usize,
    pub conflict_count: usize,
    pub top_confidence: f64,
    pub estimated_tokens: usize,
    #[allow(dead_code)]
    pub memory_context: String,
    pub items: Vec<MemorySearchItem>,
    pub top_evidence_preference_polarity: String,
}

#[derive(Debug, Default)]
struct QueryIntent {
    wants_current: bool,
    wants_past: bool,
    wants_update: bool,
    wants_location: bool,
    wants_preference: bool,
    wants_connection: bool,
    wants_remote: bool,
}

/// `retrieval_modes` 为 policy 过滤后的实际模式列表。
#[allow(clippy::too_many_arguments)]
pub(crate) async fn collect_l1_evidence(
    store: &MemoryBackend,
    policy: &PolicyConfigStore,
    user_id: &str,
    query: &str,
    memory_limit: usize,
    summary_limit: usize,
    retrieval_modes: &[String],
    graph_enabled: bool,
    rerank_enabled: bool,
) -> L1Evidence {
    let normalized_query = query.trim().to_lowercase();
    let intent = parse_query_intent(query);
    let query_polarity_opt = l1_policy::query_preference_polarity(query);
    let snap = match store.user_snapshot(user_id).await {
        Ok(s) => s,
        Err(err) => {
            tracing::warn!(error = %err, user_id, "user_snapshot failed; using empty L1 bundle");
            UserMemorySnapshot::default()
        }
    };
    let summaries = &snap.summaries;
    let artifacts = &snap.artifacts;
    let entities = &snap.entities;
    let entity_links = &snap.entity_links;
    let mut matched_entity_ids: Vec<String> = entities
        .values()
        .filter(|entity| {
            entity.user_id == user_id
                && (normalized_query.contains(&entity.name.to_lowercase())
                    || entity.name.to_lowercase().contains(&normalized_query))
        })
        .map(|entity| entity.id.clone())
        .collect();

    if intent.wants_location && !l1_policy::query_names_any_city(query) {
        for entity in entities.values() {
            if entity.user_id == user_id
                && entity.entity_type == "location"
                && !matched_entity_ids.iter().any(|id| id == &entity.id)
            {
                matched_entity_ids.push(entity.id.clone());
            }
        }
    }

    let mut ranked_summaries: Vec<(f64, MemorySummaryRecord)> = summaries
        .values()
        .filter(|summary| summary.user_id == user_id)
        .cloned()
        .map(|summary| {
            let summary_entity_hit = summary.memory_ids.iter().any(|mid| {
                entity_links.values().any(|link| {
                    link.user_id == user_id
                        && link.memory_id == *mid
                        && matched_entity_ids.iter().any(|eid| eid == &link.entity_id)
                })
            });
            (
                score_match(
                    &normalized_query,
                    &summary.summary_text,
                    &summary.keywords,
                    &summary.temporal_state,
                    summary.supersedes_previous,
                    "summary",
                    summary_entity_hit,
                    &intent,
                    &summary.preference_polarity,
                    query_polarity_opt,
                    policy.importance_score_default,
                    &summary.updated_at,
                    0,
                    policy,
                ),
                summary,
            )
        })
        .collect();
    ranked_summaries.sort_by(|left, right| {
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(Ordering::Equal)
            .then_with(|| right.1.updated_at.cmp(&left.1.updated_at))
    });

    let mut ranked_artifacts: Vec<(f64, MemoryArtifactRecord)> = artifacts
        .values()
        .filter(|artifact| artifact.user_id == user_id)
        .cloned()
        .map(|artifact| {
            let linked_entity_hit = entity_links.values().any(|link| {
                link.user_id == user_id
                    && link.memory_id == artifact.memory_id
                    && matched_entity_ids
                        .iter()
                        .any(|entity_id| entity_id == &link.entity_id)
            });
            (
                score_match(
                    &normalized_query,
                    &artifact.content,
                    &artifact.keywords,
                    &artifact.temporal_state,
                    artifact.supersedes_previous,
                    &artifact.network_type,
                    linked_entity_hit,
                    &intent,
                    &artifact.preference_polarity,
                    query_polarity_opt,
                    artifact.importance_score,
                    &artifact.last_accessed_at,
                    artifact.access_count,
                    policy,
                ) + artifact.confidence / l1_policy::SCORE_CONFIDENCE_DIVISOR,
                artifact,
            )
        })
        .collect();
    ranked_artifacts.sort_by(|left, right| {
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(Ordering::Equal)
            .then_with(|| right.1.created_at.cmp(&left.1.created_at))
    });

    let graph_requested = graph_enabled && retrieval_modes.iter().any(|m| m == "graph");
    let graph_mem: HashSet<String> = if graph_requested {
        entity_links
            .values()
            .filter(|l| {
                l.user_id == user_id && matched_entity_ids.iter().any(|e| e == &l.entity_id)
            })
            .map(|l| l.memory_id.clone())
            .collect()
    } else {
        HashSet::new()
    };

    let take_sum = if summary_limit == 0 { 0 } else { summary_limit };
    let mut sum_top: Vec<(f64, MemorySummaryRecord)> =
        ranked_summaries.iter().take(take_sum).cloned().collect();

    if graph_requested && !graph_mem.is_empty() && summary_limit > 0 {
        let top_sum_ids: HashSet<String> =
            sum_top.iter().map(|(_, s)| s.summary_id.clone()).collect();
        let mut seen = top_sum_ids.clone();
        let mut sum_extras: Vec<(f64, MemorySummaryRecord)> = vec![];
        for summary in summaries.values() {
            if summary.user_id != user_id {
                continue;
            }
            if seen.contains(&summary.summary_id) {
                continue;
            }
            let overlap = summary
                .memory_ids
                .iter()
                .filter(|m| graph_mem.contains(*m))
                .count();
            if overlap == 0 {
                continue;
            }
            let sc = (l1_policy::SCORE_GRAPH_EXPANSION_BASE + 0.04 * overlap as f64)
                .min(l1_policy::SCORE_MAX);
            sum_extras.push((sc, summary.clone()));
            seen.insert(summary.summary_id.clone());
        }
        sum_top.extend(sum_extras);
        sum_top.sort_by(|left, right| {
            right
                .0
                .partial_cmp(&left.0)
                .unwrap_or(Ordering::Equal)
                .then_with(|| right.1.updated_at.cmp(&left.1.updated_at))
        });
        sum_top.truncate(summary_limit);
    }

    let take_mem = if memory_limit == 0 { 0 } else { memory_limit };
    let mut art_top: Vec<(f64, MemoryArtifactRecord)> =
        ranked_artifacts.iter().take(take_mem).cloned().collect();

    if graph_requested && !matched_entity_ids.is_empty() && memory_limit > 0 {
        let mut seen: HashSet<String> = art_top.iter().map(|(_, a)| a.memory_id.clone()).collect();
        let mut extras: Vec<(f64, MemoryArtifactRecord)> = vec![];
        for link in entity_links.values() {
            if link.user_id != user_id {
                continue;
            }
            if !matched_entity_ids.iter().any(|e| e == &link.entity_id) {
                continue;
            }
            if seen.contains(&link.memory_id) {
                continue;
            }
            if let Some(a) = artifacts.get(&link.memory_id) {
                let sc = (l1_policy::SCORE_GRAPH_EXPANSION_BASE
                    + link.confidence / l1_policy::SCORE_CONFIDENCE_DIVISOR)
                    .min(l1_policy::SCORE_MAX);
                extras.push((sc, a.clone()));
                seen.insert(link.memory_id.clone());
            }
        }
        art_top.extend(extras);
        art_top.sort_by(|left, right| {
            right
                .0
                .partial_cmp(&left.0)
                .unwrap_or(Ordering::Equal)
                .then_with(|| right.1.created_at.cmp(&left.1.created_at))
        });
        art_top.truncate(memory_limit);
    }

    let selected_summaries: Vec<MemorySummaryRecord> =
        sum_top.into_iter().map(|(_, s)| s).collect();
    let selected_artifacts: Vec<MemoryArtifactRecord> =
        art_top.into_iter().map(|(_, a)| a).collect();

    let mut items = vec![];
    for summary in &selected_summaries {
        let summary_entity_hit = summary.memory_ids.iter().any(|mid| {
            entity_links.values().any(|link| {
                link.user_id == user_id
                    && link.memory_id == *mid
                    && matched_entity_ids.iter().any(|eid| eid == &link.entity_id)
            })
        });
        let confidence = score_match(
            &normalized_query,
            &summary.summary_text,
            &summary.keywords,
            &summary.temporal_state,
            summary.supersedes_previous,
            "summary",
            summary_entity_hit,
            &intent,
            &summary.preference_polarity,
            query_polarity_opt,
            policy.importance_score_default,
            &summary.updated_at,
            0,
            policy,
        );
        items.push(MemorySearchItem {
            memory_id: summary.summary_id.clone(),
            network_type: "summary".to_string(),
            memory_level: "working".to_string(),
            content: summary.summary_text.clone(),
            confidence,
            preference_polarity: normalized_stored_polarity(&summary.preference_polarity),
        });
    }
    for artifact in &selected_artifacts {
        let linked_entity_hit = entity_links.values().any(|link| {
            link.user_id == user_id
                && link.memory_id == artifact.memory_id
                && matched_entity_ids
                    .iter()
                    .any(|entity_id| entity_id == &link.entity_id)
        });
        let confidence = score_match(
            &normalized_query,
            &artifact.content,
            &artifact.keywords,
            &artifact.temporal_state,
            artifact.supersedes_previous,
            &artifact.network_type,
            linked_entity_hit,
            &intent,
            &artifact.preference_polarity,
            query_polarity_opt,
            artifact.importance_score,
            &artifact.last_accessed_at,
            artifact.access_count,
            policy,
        ) + artifact.confidence / l1_policy::SCORE_CONFIDENCE_DIVISOR;
        items.push(MemorySearchItem {
            memory_id: artifact.memory_id.clone(),
            network_type: artifact.network_type.clone(),
            memory_level: artifact.memory_level.clone(),
            content: artifact.content.clone(),
            confidence,
            preference_polarity: normalized_stored_polarity(&artifact.preference_polarity),
        });
    }

    let rerank_requested = rerank_enabled && retrieval_modes.iter().any(|m| m == "rerank");
    if rerank_requested {
        rerank_second_pass(
            &mut items,
            &normalized_query,
            &matched_entity_ids,
            entity_links,
            user_id,
        );
    }

    let conflict_count = selected_summaries
        .iter()
        .map(|summary| l1_policy::detect_conflict_count(&summary.summary_text))
        .sum::<usize>()
        + selected_artifacts
            .iter()
            .map(|artifact| l1_policy::detect_conflict_count(&artifact.content))
            .sum::<usize>();
    let summary_preview = selected_summaries
        .iter()
        .take(2)
        .map(|summary| summary.summary_text.clone())
        .collect::<Vec<_>>()
        .join(" | ");
    let artifact_preview = selected_artifacts
        .iter()
        .take(3)
        .map(|artifact| format!("[{}] {}", artifact.network_type, artifact.content))
        .collect::<Vec<_>>()
        .join(" | ");
    let top_confidence = items
        .iter()
        .map(|item| item.confidence)
        .fold(0.0_f64, f64::max);
    let top_evidence_preference_polarity = top_polarity_for_selection(
        &normalized_query,
        query_polarity_opt,
        &intent,
        policy,
        user_id,
        &selected_summaries,
        &selected_artifacts,
        &matched_entity_ids,
        entity_links,
    );
    let top_activation = selected_artifacts
        .iter()
        .map(|artifact| {
            activation_factor(
                &artifact.network_type,
                artifact.importance_score,
                &artifact.last_accessed_at,
                artifact.access_count,
                policy,
            )
        })
        .fold(0.0_f64, f64::max);
    let query_polarity_hint = query_polarity_opt.unwrap_or("none");
    let memory_context = format!(
        "summary_hits={}; artifact_hits={}; entity_hits={}; top_confidence={:.2}; activation_top={:.2}; query_polarity_hint={}; pref_top={}; summaries={}; artifacts={}",
        selected_summaries.len(),
        selected_artifacts.len(),
        matched_entity_ids.len(),
        top_confidence,
        top_activation,
        query_polarity_hint,
        top_evidence_preference_polarity,
        summary_preview,
        artifact_preview
    );
    let estimated_tokens = estimate_token_count(
        selected_summaries
            .iter()
            .map(|summary| summary.summary_text.as_str())
            .chain(
                selected_artifacts
                    .iter()
                    .map(|artifact| artifact.content.as_str()),
            ),
    );

    L1Evidence {
        selected_summary_ids: selected_summaries
            .iter()
            .map(|summary| summary.summary_id.clone())
            .collect(),
        selected_memory_ids: selected_artifacts
            .iter()
            .map(|artifact| artifact.memory_id.clone())
            .collect(),
        evidence_count: selected_summaries.len() + selected_artifacts.len(),
        summary_hits: selected_summaries.len(),
        artifact_hits: selected_artifacts.len(),
        entity_hits: matched_entity_ids.len(),
        conflict_count,
        top_confidence,
        estimated_tokens,
        memory_context,
        items,
        top_evidence_preference_polarity,
    }
}

/// 第二阶段：与 L1 `score_match` 独立；引入共现实体加权、同 network_type 拥挤惩罚、跨条目共现惩罚。
fn rerank_second_pass(
    items: &mut Vec<MemorySearchItem>,
    normalized_query: &str,
    matched_entity_ids: &[String],
    entity_links: &HashMap<String, MemoryEntityLinkRecord>,
    user_id: &str,
) {
    if items.len() <= 1 {
        return;
    }
    let old = std::mem::take(items);
    let rerank_scores: Vec<f64> = old
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let mut s = item.confidence;
            let entity_touch = entity_links
                .values()
                .filter(|l| {
                    l.user_id == user_id
                        && l.memory_id == item.memory_id
                        && matched_entity_ids.iter().any(|e| e == &l.entity_id)
                })
                .count() as f64;
            s += 0.03 * entity_touch;
            let dup_nt = old
                .iter()
                .filter(|x| x.network_type == item.network_type)
                .count();
            if dup_nt > 2 {
                s -= 0.015 * (dup_nt - 2) as f64;
            }
            let qtok = query_tokens(normalized_query);
            let content_lower = item.content.to_lowercase();
            let unique_focus = qtok
                .iter()
                .filter(|t| content_lower.contains(t.as_str()))
                .count() as f64;
            s += 0.02 * unique_focus;
            let mut cross = 0usize;
            for (j, other) in old.iter().enumerate() {
                if i == j {
                    continue;
                }
                if content_token_overlap(&item.content, &other.content) >= 3 {
                    cross += 1;
                }
            }
            s -= 0.01 * cross as f64;
            s.min(l1_policy::SCORE_MAX)
        })
        .collect();
    let mut pairs: Vec<(usize, f64)> = (0..old.len()).map(|i| (i, rerank_scores[i])).collect();
    pairs.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(Ordering::Equal)
            .then_with(|| old[b.0].memory_id.cmp(&old[a.0].memory_id))
    });
    *items = pairs
        .into_iter()
        .map(|(idx, sc)| {
            let mut it = old[idx].clone();
            it.confidence = sc;
            it
        })
        .collect();
}

fn content_token_overlap(a: &str, b: &str) -> usize {
    let la = a.to_lowercase();
    let lb = b.to_lowercase();
    query_tokens(&la)
        .into_iter()
        .filter(|t| t.len() >= 2 && lb.contains(t.as_str()))
        .count()
}

#[inline]
fn normalized_stored_polarity(raw: &str) -> String {
    if raw.is_empty() {
        "neutral".to_string()
    } else {
        raw.to_string()
    }
}

#[allow(clippy::too_many_arguments)]
fn top_polarity_for_selection(
    normalized_query: &str,
    query_polarity_opt: Option<&str>,
    intent: &QueryIntent,
    policy: &PolicyConfigStore,
    user_id: &str,
    selected_summaries: &[MemorySummaryRecord],
    selected_artifacts: &[MemoryArtifactRecord],
    matched_entity_ids: &[String],
    entity_links: &HashMap<String, MemoryEntityLinkRecord>,
) -> String {
    let mut best_score = -1.0_f64;
    let mut best_pol = "neutral".to_string();
    for summary in selected_summaries {
        let summary_entity_hit = summary.memory_ids.iter().any(|mid| {
            entity_links.values().any(|link| {
                link.user_id == user_id
                    && link.memory_id == *mid
                    && matched_entity_ids.iter().any(|eid| eid == &link.entity_id)
            })
        });
        let sc = score_match(
            normalized_query,
            &summary.summary_text,
            &summary.keywords,
            &summary.temporal_state,
            summary.supersedes_previous,
            "summary",
            summary_entity_hit,
            intent,
            &summary.preference_polarity,
            query_polarity_opt,
            policy.importance_score_default,
            &summary.updated_at,
            0,
            policy,
        );
        if sc > best_score {
            best_score = sc;
            best_pol = normalized_stored_polarity(&summary.preference_polarity);
        }
    }
    for artifact in selected_artifacts {
        let linked_entity_hit = entity_links.values().any(|link| {
            link.user_id == user_id
                && link.memory_id == artifact.memory_id
                && matched_entity_ids
                    .iter()
                    .any(|entity_id| entity_id == &link.entity_id)
        });
        let sc = score_match(
            normalized_query,
            &artifact.content,
            &artifact.keywords,
            &artifact.temporal_state,
            artifact.supersedes_previous,
            &artifact.network_type,
            linked_entity_hit,
            intent,
            &artifact.preference_polarity,
            query_polarity_opt,
            artifact.importance_score,
            &artifact.last_accessed_at,
            artifact.access_count,
            policy,
        ) + artifact.confidence / l1_policy::SCORE_CONFIDENCE_DIVISOR;
        if sc > best_score {
            best_score = sc;
            best_pol = normalized_stored_polarity(&artifact.preference_polarity);
        }
    }
    best_pol
}

#[allow(clippy::too_many_arguments)]
fn score_match(
    query: &str,
    content: &str,
    keywords: &[String],
    temporal_state: &str,
    supersedes_previous: bool,
    network_type: &str,
    linked_entity_hit: bool,
    intent: &QueryIntent,
    preference_polarity: &str,
    query_polarity_opt: Option<&str>,
    importance_score: f64,
    last_accessed_at: &str,
    access_count: u32,
    policy: &PolicyConfigStore,
) -> f64 {
    if query.is_empty() {
        return l1_policy::SCORE_EMPTY_QUERY;
    }
    let content_lower = content.to_lowercase();
    let mut score: f64 = if content_lower.contains(query) {
        l1_policy::SCORE_CONTENT_CONTAINS_QUERY
    } else {
        l1_policy::SCORE_CONTENT_BASE
    };
    if token_overlap_count(query, &content_lower) > 0 {
        score += l1_policy::SCORE_TOKEN_OVERLAP;
    }
    for keyword in keywords {
        if query.contains(&keyword.to_lowercase())
            || content_lower.contains(&keyword.to_lowercase())
        {
            score += l1_policy::SCORE_KEYWORD_HIT;
        }
    }
    if intent.wants_current && matches!(temporal_state, "current" | "updated") {
        score += l1_policy::SCORE_TEMPORAL_CURRENT_MATCH;
    } else if intent.wants_current && temporal_state == "past" {
        score += l1_policy::SCORE_TEMPORAL_CURRENT_PAST_PENALTY;
    }
    if intent.wants_past && temporal_state == "past" {
        score += l1_policy::SCORE_TEMPORAL_PAST_MATCH;
    } else if intent.wants_past && matches!(temporal_state, "current" | "updated") {
        score += l1_policy::SCORE_TEMPORAL_PAST_CURRENT_PENALTY;
    }
    if intent.wants_update && supersedes_previous {
        score += l1_policy::SCORE_UPDATE_SUPERSEDES;
    }
    if intent.wants_location && l1_policy::contains_known_city(&content_lower) {
        score += l1_policy::SCORE_LOCATION_CITY;
    }
    if intent.wants_preference && network_type == "opinion" {
        score += l1_policy::SCORE_PREFERENCE_OPINION;
    }
    if intent.wants_connection && (content.contains("投资人") || content.contains("合伙人")) {
        score += l1_policy::SCORE_CONNECTION;
    }
    if intent.wants_remote && content.contains("远程") {
        score += l1_policy::SCORE_REMOTE;
    }
    if linked_entity_hit {
        score += l1_policy::SCORE_ENTITY_LINK;
    }
    if supersedes_previous && !intent.wants_past {
        score += l1_policy::SCORE_SUPERSEDES_GENERAL;
    }

    let stored = if preference_polarity.is_empty() {
        "neutral"
    } else {
        preference_polarity
    };
    if let Some(qp) = query_polarity_opt {
        if qp != "neutral" && stored == qp {
            score += l1_policy::SCORE_PREFERENCE_ALIGN;
        } else if qp != "neutral" && stored != "neutral" && stored != qp {
            score += l1_policy::SCORE_PREFERENCE_MISMATCH;
        }
    }

    if network_type != "summary" {
        let activation = activation_factor(
            network_type,
            importance_score,
            last_accessed_at,
            access_count,
            policy,
        );
        score += policy.score_activation_weight * activation.min(1.0);
    }

    score.min(l1_policy::SCORE_MAX)
}

fn activation_factor(
    network_type: &str,
    importance_score: f64,
    last_accessed_at: &str,
    access_count: u32,
    policy: &PolicyConfigStore,
) -> f64 {
    if network_type == "summary" {
        return 0.0;
    }
    let importance = importance_score.max(0.0);
    let elapsed_hours = elapsed_hours_since(last_accessed_at).unwrap_or(0.0);
    let decay = (1.0 + elapsed_hours).powf(-policy.activation_decay_rate.max(0.0));
    let access_boost = (2.0 + access_count as f64).log2();
    (importance * decay * access_boost).max(0.0)
}

fn elapsed_hours_since(label: &str) -> Option<f64> {
    let touched_ms = label.strip_prefix("unix-ms:")?.parse::<u128>().ok()?;
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis();
    if now_ms <= touched_ms {
        return Some(0.0);
    }
    Some((now_ms - touched_ms) as f64 / 3_600_000.0)
}

fn parse_query_intent(query: &str) -> QueryIntent {
    QueryIntent {
        wants_current: l1_policy::INTENT_WANTS_CURRENT
            .iter()
            .any(|marker| query.contains(*marker)),
        wants_past: l1_policy::INTENT_WANTS_PAST
            .iter()
            .any(|marker| query.contains(*marker)),
        wants_update: l1_policy::INTENT_WANTS_UPDATE
            .iter()
            .any(|marker| query.contains(*marker)),
        wants_location: l1_policy::INTENT_WANTS_LOCATION
            .iter()
            .any(|marker| query.contains(*marker)),
        wants_preference: l1_policy::INTENT_WANTS_PREFERENCE
            .iter()
            .any(|marker| query.contains(*marker)),
        wants_connection: l1_policy::INTENT_WANTS_CONNECTION
            .iter()
            .any(|marker| query.contains(*marker)),
        wants_remote: l1_policy::INTENT_WANTS_REMOTE
            .iter()
            .any(|marker| query.contains(*marker)),
    }
}

fn token_overlap_count(query: &str, content: &str) -> usize {
    query_tokens(query)
        .into_iter()
        .filter(|token| content.contains(token))
        .count()
}

fn query_tokens(query: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for token in query
        .split(|ch: char| !ch.is_alphanumeric() && !matches!(ch, '_' | '-'))
        .filter(|token| token.len() >= 2)
    {
        if !tokens.iter().any(|existing| existing == token) {
            tokens.push(token.to_string());
        }
    }
    for marker in l1_policy::QUERY_TOKEN_EXTRA {
        if query.contains(*marker) && !tokens.iter().any(|existing| existing == *marker) {
            tokens.push((*marker).to_string());
        }
    }
    tokens
}

fn estimate_token_count<'a>(values: impl Iterator<Item = &'a str>) -> usize {
    let total_chars = values.map(|value| value.chars().count()).sum::<usize>();
    (total_chars / 4).max(1)
}

#[cfg(test)]
mod evidence_hardening_tests {
    use super::*;
    use crate::app_state::MemoryArtifactRecord;
    use crate::memory_store::{MemoryEntityLinkRecord, MemoryEntityRecord, MemorySearchItem};
    use crate::policy::PolicyConfigStore;
    use crate::store::MemoryBackend;
    use std::collections::HashMap;

    fn artifact(
        memory_id: &str,
        user_id: &str,
        content: &str,
        confidence: f64,
        created_ms: i64,
    ) -> MemoryArtifactRecord {
        MemoryArtifactRecord {
            memory_id: memory_id.into(),
            user_id: user_id.into(),
            conversation_id: String::new(),
            source_message_id: String::new(),
            content: content.into(),
            network_type: "experience".into(),
            evidence_type: "fact".into(),
            memory_level: "persistent".into(),
            source_type: "chat".into(),
            confidence,
            importance_score: 0.5,
            keywords: vec![],
            temporal_state: "timeless".into(),
            supersedes_previous: false,
            preference_polarity: "neutral".into(),
            last_accessed_at: format!("unix-ms:{created_ms}"),
            access_count: 0,
            created_at: format!("unix-ms:{created_ms}"),
        }
    }

    #[tokio::test]
    async fn graph_pulls_artifact_not_in_first_top_k() {
        let uid = "00000000-0000-0000-0000-000000000099";
        let eid = "entity:graph:fixture:gx1";
        let query = "gxfixturetoken";
        let store = MemoryBackend::in_memory();
        let policy = PolicyConfigStore::default();

        // m-a: 首轮最高分（query 字面命中）；m-b：靠 confidence/10 挤进 top-2；m-c：字面弱 + entity link，首轮第三，图扩展分高
        let a = artifact("m-a", uid, query, 0.5, 10);
        let b = artifact("m-b", uid, "noise filler beta gamma delta", 5.0, 20);
        let c = artifact("m-c", uid, "zzz unrelated", 0.5, 30);

        store.insert_artifact(a).await.unwrap();
        store.insert_artifact(b).await.unwrap();
        store.insert_artifact(c).await.unwrap();

        store
            .upsert_entity(MemoryEntityRecord {
                id: eid.into(),
                user_id: uid.into(),
                entity_type: "topic".into(),
                name: "gxfixturetoken".into(),
            })
            .await
            .unwrap();

        for (i, mid) in ["m-a", "m-b", "m-c"].iter().enumerate() {
            store
                .upsert_entity_link(MemoryEntityLinkRecord {
                    id: format!("link-{i}"),
                    user_id: uid.into(),
                    entity_id: eid.into(),
                    memory_id: (*mid).into(),
                    relationship: "mentions".into(),
                    confidence: if *mid == "m-c" { 8.0 } else { 0.1 },
                })
                .await
                .unwrap();
        }

        let without = collect_l1_evidence(
            &store,
            &policy,
            uid,
            query,
            2,
            0,
            &["structured".into(), "graph".into()],
            false,
            false,
        )
        .await;

        let with = collect_l1_evidence(
            &store,
            &policy,
            uid,
            query,
            2,
            0,
            &["structured".into(), "graph".into()],
            true,
            false,
        )
        .await;

        assert!(
            !without.selected_memory_ids.contains(&"m-c".to_string()),
            "without graph, m-c should stay outside top-2 first pass (got {:?})",
            without.selected_memory_ids
        );
        assert!(
            with.selected_memory_ids.contains(&"m-c".to_string()),
            "with graph, m-c should be pulled in via entity_links (got {:?})",
            with.selected_memory_ids
        );
    }

    #[test]
    fn rerank_second_pass_reorders_using_non_score_match_signals() {
        let uid = "u1";
        let eid = "e-rich";
        let mut links = HashMap::new();
        for i in 0..18 {
            links.insert(
                format!("L{i}"),
                MemoryEntityLinkRecord {
                    id: format!("L{i}"),
                    user_id: uid.into(),
                    entity_id: eid.into(),
                    memory_id: "mem-low".into(),
                    relationship: "x".into(),
                    confidence: 0.1,
                },
            );
        }
        let mut items = vec![
            MemorySearchItem {
                memory_id: "mem-high".into(),
                network_type: "experience".into(),
                memory_level: "persistent".into(),
                content: "aa".into(),
                confidence: 0.92,
                preference_polarity: "neutral".into(),
            },
            MemorySearchItem {
                memory_id: "mem-low".into(),
                network_type: "experience".into(),
                memory_level: "persistent".into(),
                content: "aa".into(),
                confidence: 0.5,
                preference_polarity: "neutral".into(),
            },
        ];
        rerank_second_pass(&mut items, "tokone toktwo", &[eid.into()], &links, uid);
        assert_eq!(items[0].memory_id, "mem-low");
        assert_eq!(items[1].memory_id, "mem-high");
    }

    #[test]
    fn activation_boosts_recent_high_access() {
        let policy = PolicyConfigStore::default();
        let intent = QueryIntent::default();
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let one_hour_ago = now_ms.saturating_sub(3_600_000);
        let thirty_days_ago = now_ms.saturating_sub(30 * 24 * 3_600_000);
        let content = "用户关注 AI 创业与产品";
        let query = "创业";
        let high = score_match(
            query,
            content,
            &[],
            "timeless",
            false,
            "experience",
            false,
            &intent,
            "neutral",
            None,
            0.9,
            &format!("unix-ms:{one_hour_ago}"),
            10,
            &policy,
        );
        let low = score_match(
            query,
            content,
            &[],
            "timeless",
            false,
            "experience",
            false,
            &intent,
            "neutral",
            None,
            0.3,
            &format!("unix-ms:{thirty_days_ago}"),
            0,
            &policy,
        );
        assert!(
            high > low,
            "same lexical match: high-activation artifact should score higher: high={high}, low={low}"
        );
    }

    #[test]
    fn activation_does_not_break_zero_access() {
        let policy = PolicyConfigStore::default();
        let intent = QueryIntent::default();
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let s = score_match(
            "创业",
            "用户关注 AI 创业",
            &[],
            "timeless",
            false,
            "experience",
            false,
            &intent,
            "neutral",
            None,
            0.7,
            &format!("unix-ms:{now_ms}"),
            0,
            &policy,
        );
        assert!(s.is_finite());
        assert!(
            s >= 0.0,
            "zero access_count must not yield negative score: {s}"
        );
    }

    #[test]
    fn activation_factor_prefers_recent_high_access_memory() {
        let policy = PolicyConfigStore::default();
        let intent = QueryIntent::default();
        let fresh = score_match(
            "创业",
            "用户关注 AI 创业",
            &[],
            "timeless",
            false,
            "experience",
            false,
            &intent,
            "neutral",
            None,
            0.9,
            "unix-ms:9999999999999",
            8,
            &policy,
        );
        let stale = score_match(
            "创业",
            "用户关注 AI 创业",
            &[],
            "timeless",
            false,
            "experience",
            false,
            &intent,
            "neutral",
            None,
            0.5,
            "unix-ms:1",
            0,
            &policy,
        );
        assert!(
            fresh > stale,
            "fresh/high-access artifact should outrank stale/low-access artifact: fresh={fresh}, stale={stale}"
        );
    }

    #[test]
    fn activation_weight_zero_disables_activation_reordering() {
        let mut enabled = PolicyConfigStore::default();
        let disabled = PolicyConfigStore {
            score_activation_weight: 0.0,
            ..Default::default()
        };
        let intent = QueryIntent::default();
        let with_activation = score_match(
            "创业",
            "用户关注 AI 创业",
            &[],
            "timeless",
            false,
            "experience",
            false,
            &intent,
            "neutral",
            None,
            0.9,
            "unix-ms:9999999999999",
            8,
            &enabled,
        );
        let without_activation = score_match(
            "创业",
            "用户关注 AI 创业",
            &[],
            "timeless",
            false,
            "experience",
            false,
            &intent,
            "neutral",
            None,
            0.9,
            "unix-ms:9999999999999",
            8,
            &disabled,
        );
        let baseline = score_match(
            "创业",
            "用户关注 AI 创业",
            &[],
            "timeless",
            false,
            "experience",
            false,
            &intent,
            "neutral",
            None,
            0.1,
            "unix-ms:1",
            0,
            &disabled,
        );
        assert!(with_activation > without_activation);
        assert_eq!(without_activation, baseline);
        enabled.score_activation_weight = 0.0;
        assert_eq!(without_activation, baseline);
    }
}
