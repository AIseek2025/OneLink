//! L1 检索与蒸馏共用策略：冲突 marker、意图 marker、评分权重、偏好极性（MVP 单源口径）。

// --- 冲突 / 时间线（与 distiller、`detect_conflicts` 对齐）---

pub const CONFLICT_MARKERS: &[&str] = &[
    "不再",
    "改为",
    "后来",
    "现在",
    "之前",
    "以前",
    "从",
    "已经不",
];

#[inline]
pub fn detect_conflict_count(input: &str) -> usize {
    CONFLICT_MARKERS
        .iter()
        .filter(|marker| input.contains(**marker))
        .count()
}

/// 用户消息是否表达「覆盖/更新」旧事实（写入 supersedes_previous）。
pub const SUPERSEDES_MARKERS: &[&str] = &["改为", "不再", "后来", "从", "已经不"];

#[inline]
pub fn supersedes_previous_from_input(input: &str) -> bool {
    SUPERSEDES_MARKERS.iter().any(|m| input.contains(*m))
}

#[inline]
pub fn infer_temporal_state(input: &str) -> &'static str {
    if supersedes_previous_from_input(input) {
        "updated"
    } else if INTENT_WANTS_CURRENT.iter().any(|m| input.contains(*m)) {
        "current"
    } else if INTENT_WANTS_PAST.iter().any(|m| input.contains(*m)) {
        "past"
    } else {
        "timeless"
    }
}

// --- 偏好极性（用户消息蒸馏 + 查询侧对齐）---

pub const PREFERENCE_NEGATIVE_MARKERS: &[&str] = &["不喜欢", "讨厌", "不想", "不要"];
pub const PREFERENCE_POSITIVE_MARKERS: &[&str] = &["喜欢", "想要", "希望", "感兴趣"];

#[inline]
pub fn infer_preference_polarity(input: &str) -> &'static str {
    if PREFERENCE_NEGATIVE_MARKERS
        .iter()
        .any(|m| input.contains(*m))
    {
        "negative"
    } else if PREFERENCE_POSITIVE_MARKERS
        .iter()
        .any(|m| input.contains(*m))
    {
        "positive"
    } else {
        "neutral"
    }
}

/// 仅在查询看起来在问「偏好/沟通」时返回 Some，供 L1 与存储的 `preference_polarity` 对齐加分。
#[inline]
pub fn query_preference_polarity(query: &str) -> Option<&'static str> {
    let hit = INTENT_WANTS_PREFERENCE.iter().any(|m| query.contains(*m));
    if !hit {
        return None;
    }
    Some(infer_preference_polarity(query))
}

// --- Task router（候选路由）---

pub const ROUTER_EXPLICIT_CONFLICT: &[&str] = &["冲突", "到底", "替代", "覆盖", "更新后"];
pub const ROUTER_MENTIONS_PAST: &[&str] = &["之前", "以前", "曾经"];
pub const ROUTER_MENTIONS_CURRENT: &[&str] = &["现在", "目前", "当前"];
pub const ROUTER_MENTIONS_UPDATE: &[&str] = &["后来", "改为", "不再", "更新", "已经不"];
pub const ROUTER_REASONING_LIKE: &[&str] = &["如何判断", "比较", "更适合", "推荐", "投资人"];

// --- L1 查询意图 ---

pub const INTENT_WANTS_CURRENT: &[&str] = &["现在", "目前", "当前"];
pub const INTENT_WANTS_PAST: &[&str] = &["之前", "以前", "曾经"];
pub const INTENT_WANTS_UPDATE: &[&str] = &["改为", "不再", "后来", "更新", "替代", "变化"];
pub const INTENT_WANTS_LOCATION: &[&str] = &["哪里", "哪座城市", "城市", "在哪"];
pub const INTENT_WANTS_PREFERENCE: &[&str] = &["喜欢", "不喜欢", "偏好", "沟通方式", "讨厌"];
pub const INTENT_WANTS_CONNECTION: &[&str] = &["投资人", "合伙人", "认识谁", "连接谁"];
pub const INTENT_WANTS_REMOTE: &[&str] = &["远程", "办公方式", "工作方式"];

pub const KNOWN_CITIES: &[&str] = &["上海", "北京", "深圳", "杭州"];

/// Distiller 关键词种子（与 L1 `query_tokens` 扩展词表对齐）。
pub const KEYWORD_SEED_PHRASES: &[&str] = &[
    "AI",
    "人工智能",
    "创业",
    "startup",
    "投资",
    "投资人",
    "技术合伙人",
    "远程办公",
    "推销式沟通",
    "招聘",
    "上海",
    "北京",
    "深圳",
    "杭州",
];

pub const QUERY_TOKEN_EXTRA: &[&str] = &[
    "上海",
    "北京",
    "深圳",
    "杭州",
    "远程",
    "投资人",
    "合伙人",
    "推销",
    "沟通",
    "AI",
];

// --- L1 评分权重（`score_match`）---

pub const SCORE_EMPTY_QUERY: f64 = 0.35;
pub const SCORE_CONTENT_CONTAINS_QUERY: f64 = 0.75;
pub const SCORE_CONTENT_BASE: f64 = 0.2;
pub const SCORE_TOKEN_OVERLAP: f64 = 0.12;
pub const SCORE_KEYWORD_HIT: f64 = 0.08;
pub const SCORE_TEMPORAL_CURRENT_MATCH: f64 = 0.14;
pub const SCORE_TEMPORAL_CURRENT_PAST_PENALTY: f64 = -0.08;
pub const SCORE_TEMPORAL_PAST_MATCH: f64 = 0.14;
pub const SCORE_TEMPORAL_PAST_CURRENT_PENALTY: f64 = -0.05;
pub const SCORE_UPDATE_SUPERSEDES: f64 = 0.12;
pub const SCORE_LOCATION_CITY: f64 = 0.12;
pub const SCORE_PREFERENCE_OPINION: f64 = 0.12;
pub const SCORE_CONNECTION: f64 = 0.12;
pub const SCORE_REMOTE: f64 = 0.12;
pub const SCORE_ENTITY_LINK: f64 = 0.16;
pub const SCORE_SUPERSEDES_GENERAL: f64 = 0.04;
pub const SCORE_CONFIDENCE_DIVISOR: f64 = 10.0;
pub const SCORE_MAX: f64 = 0.99;

/// 查询极性与证据 `preference_polarity` 一致（且非 neutral）时加分。
pub const SCORE_PREFERENCE_ALIGN: f64 = 0.10;
/// 双方均为明确极性但相反时轻罚，避免错配排前。
pub const SCORE_PREFERENCE_MISMATCH: f64 = -0.06;

#[inline]
pub fn contains_known_city(text: &str) -> bool {
    KNOWN_CITIES.iter().any(|city| text.contains(*city))
}

#[inline]
pub fn query_names_any_city(query: &str) -> bool {
    KNOWN_CITIES.iter().any(|city| query.contains(*city))
}
