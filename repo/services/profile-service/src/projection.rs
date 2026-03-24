//! Phase A/B：从 context `memory/resolve` 返回的原始记忆行启发式映射为结构化画像事实（profile-service 内聚）。
//! Phase B：事实可信度、溯源字段；收紧 communication_preference；增强 location 模式。

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

pub const FACT_INTEREST: &str = "interest";
pub const FACT_GOAL: &str = "goal";
pub const FACT_LOCATION: &str = "location";
pub const FACT_COMMUNICATION_PREFERENCE: &str = "communication_preference";

/// 持久化在 ProfileDoc 中的事实（按 fact_type + 规范化 value 去重）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredFact {
    pub fact_type: String,
    pub value: String,
    /// 溯源：context memory 行 id（resolve 返回的 memory_id）
    pub source_memory_id: String,
    /// 溯源：若上游写入记忆时携带了来源消息 id，则透传（否则为 None）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_message_id: Option<String>,
    /// 启发式置信度 0.0~1.0（MVP；非模型打分）
    pub confidence: f64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct TraitSnapshot {
    pub interest_tags: Vec<String>,
    pub connection_goal_tags: Vec<String>,
    pub location_label: Option<String>,
    pub communication_preferences: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MemoryResolveInput {
    pub memory_id: String,
    pub content: String,
    pub network_type: String,
    pub keywords: Vec<String>,
    pub temporal_state: Option<String>,
    pub preference_polarity: Option<String>,
    pub source_message_id: Option<String>,
}

pub fn normalize_fact_value(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn fact_key(f: &StoredFact) -> String {
    format!("{}::{}", f.fact_type, normalize_fact_value(&f.value))
}

/// 合并去重：同一 fact_type + 规范化 value 只保留一条（**先出现优先**，保留其溯源与 confidence）。
pub fn merge_facts_dedupe(target: &mut Vec<StoredFact>, incoming: Vec<StoredFact>) {
    let mut existing: HashSet<String> = target.iter().map(fact_key).collect();
    for f in incoming {
        let k = fact_key(&f);
        if existing.insert(k) {
            target.push(f);
        }
    }
}

fn push_fact(
    out: &mut Vec<StoredFact>,
    fact_type: &str,
    value: String,
    memory_id: &str,
    source_message_id: Option<&str>,
    confidence: f64,
) {
    let v = normalize_fact_value(&value);
    if v.is_empty() {
        return;
    }
    let conf = confidence.clamp(0.0, 1.0);
    let smid = source_message_id
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    out.push(StoredFact {
        fact_type: fact_type.to_string(),
        value: v,
        source_memory_id: memory_id.to_string(),
        source_message_id: smid,
        confidence: conf,
    });
}

fn opt_msg<'a>(input: &'a MemoryResolveInput) -> Option<&'a str> {
    input
        .source_message_id
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
}

fn content_suggests_goal(c: &str) -> bool {
    c.contains("希望认识")
        || c.contains("想认识")
        || c.contains("要认识")
        || (c.contains("投资人")
            && (c.contains("希望") || c.contains("认识") || c.contains("想")))
        || (c.contains("合伙人") && (c.contains("希望") || c.contains("认识")))
        || c.contains("连接目标")
}

fn content_suggests_interest(c: &str, network_type: &str) -> bool {
    if content_suggests_goal(c) && c.len() < 48 && !c.contains("兴趣") && !c.contains("关注") {
        return false;
    }
    c.contains("兴趣")
        || c.contains("关注")
        || c.contains("创业")
        || c.contains("人工智能")
        || c.contains(" AI ")
        || c.starts_with("关注 AI")
        || c.contains("对 AI")
        || (network_type == "opinion" && (c.contains("主题") || c.contains("话题")))
}

/// 常见城市名（用于子串匹配）；非常见地名由「我在X」等模式抽取。
const KNOWN_CITIES: &[&str] = &[
    "上海", "北京", "深圳", "杭州", "成都", "广州", "苏州", "南京", "武汉", "西安",
];

/// 取文本中**最靠左出现**的已知城市，避免「现在在苏州，下周去上海」误取后者。
fn best_known_city_in(text: &str) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    for city in KNOWN_CITIES {
        if let Some(pos) = text.find(*city) {
            let replace = match best {
                None => true,
                Some((p0, _)) => pos < p0,
            };
            if replace {
                best = Some((pos, (*city).to_string()));
            }
        }
    }
    best.map(|(_, c)| c)
}

fn content_suggests_location(c: &str, network_type: &str, keywords: &[String]) -> bool {
    if network_type == "world" || c.contains("所在城市") || c.contains("当前所在") {
        return true;
    }
    if best_known_city_in(c).is_some() {
        return true;
    }
    if keywords
        .iter()
        .any(|k| KNOWN_CITIES.iter().any(|city| k.as_str() == *city))
    {
        return true;
    }
    location_pattern_prefix(c).is_some()
}

/// 返回 (抽取值, confidence)。
fn extract_location_value(c: &str, keywords: &[String]) -> Option<(String, f64)> {
    if let Some(city) = best_known_city_in(c) {
        return Some((city, 0.88));
    }
    for k in keywords {
        for city in KNOWN_CITIES {
            if k.as_str() == *city {
                return Some(((*city).to_string(), 0.85));
            }
        }
    }
    if c.contains("所在城市是") {
        if let Some(idx) = c.find("所在城市是") {
            let tail = &c[idx + "所在城市是".len()..];
            if let Some(city) = best_known_city_in(tail) {
                return Some((city, 0.82));
            }
        }
    }
    if let Some((v, conf)) = extract_location_from_spoken_patterns(c) {
        return Some((v, conf));
    }
    None
}

fn location_pattern_prefix(c: &str) -> Option<&'static str> {
    const PREFIXES: &[&str] = &[
        "我现在在",
        "目前人在",
        "我现在人在",
        "人在",
        "现在在",
        "我在",
        "常驻在",
        "常驻",
        "base在",
        "base 在",
        "Base在",
        "Base 在",
        "BASE在",
        "BASE 在",
    ];
    PREFIXES.iter().copied().find(|p| c.contains(*p))
}

const LOCATION_DELIMS: &[char] = &[
    '，', '。', '；', '、', ',', ';', '\n', '\r', ' ', '做', '的', '了', '和', '与', '及', '后',
];

fn take_location_span(rest: &str) -> String {
    let rest = rest.trim_start();
    let end_byte = rest
        .char_indices()
        .find(|(_, ch)| LOCATION_DELIMS.contains(ch))
        .map(|(i, _)| i)
        .unwrap_or(rest.len());
    let raw = rest.get(..end_byte).unwrap_or(rest);
    raw.chars().take(32).collect::<String>().trim().to_string()
}

fn extract_location_from_spoken_patterns(c: &str) -> Option<(String, f64)> {
    let ordered: &[(&str, f64)] = &[
        ("我现在在", 0.78),
        ("我现在人在", 0.78),
        ("目前人在", 0.76),
        ("现在在", 0.75),
        ("人在", 0.72),
        ("我在", 0.72),
        ("常驻在", 0.8),
        ("常驻", 0.78),
        ("base在", 0.7),
        ("base 在", 0.7),
        ("Base在", 0.7),
        ("Base 在", 0.7),
        ("BASE在", 0.7),
        ("BASE 在", 0.7),
    ];
    for (prefix, conf) in ordered {
        if let Some(pos) = c.find(*prefix) {
            let after = &c[pos + prefix.len()..];
            let span = take_location_span(after);
            if span.len() < 2 {
                continue;
            }
            if let Some(city) = best_known_city_in(&span) {
                return Some((city, conf + 0.1));
            }
            // 2–24 字以内的地名片段（启发式；可测试）
            let nchars = span.chars().count();
            if (2..=24).contains(&nchars) {
                return Some((normalize_fact_value(&span), *conf));
            }
        }
    }
    None
}

/// 仅当存在**显式**沟通/渠道偏好措辞时返回事实文案（不把普通情绪/极性当作沟通偏好）。
fn explicit_communication_preference_value(c: &str) -> Option<String> {
    let c = c.trim();
    if c.is_empty() {
        return None;
    }
    if c.contains("不要推销")
        || c.contains("别推销")
        || c.contains("讨厌推销")
        || c.contains("不要推销式")
        || c.contains("推销式")
    {
        return Some(c.to_string());
    }
    if c.contains("拐弯抹角")
        && (c.contains("不喜欢")
            || c.contains("讨厌")
            || c.contains("别")
            || c.contains("不要"))
    {
        return Some(c.to_string());
    }
    if c.contains("直接一点") || c.contains("直接一些") || c.contains("直接些") {
        return Some(c.to_string());
    }
    if (c.contains("喜欢") || c.contains("希望") || c.contains("想要") || c.contains("更"))
        && c.contains("直接")
        && (c.contains("沟通") || c.contains("说话") || c.contains("表达") || c.contains("点"))
    {
        return Some(c.to_string());
    }
    if c.contains("沟通方式")
        || c.contains("邮件联系")
        || c.contains("不要打电话")
        || c.contains("别打电话")
    {
        return Some(c.to_string());
    }
    if c.contains("微信") && (c.contains("联系") || c.contains("加我") || c.contains("用微信")) {
        return Some(c.to_string());
    }
    None
}

fn infer_facts_one(input: &MemoryResolveInput) -> Vec<StoredFact> {
    let mut out = Vec::new();
    let c = input.content.trim();
    if c.is_empty() {
        return out;
    }
    let mid = input.memory_id.as_str();
    let nt = input.network_type.as_str();
    let msg = opt_msg(input);

    if content_suggests_goal(c) {
        push_fact(
            &mut out,
            FACT_GOAL,
            c.to_string(),
            mid,
            msg,
            if c.contains("希望认识") || c.contains("想认识") {
                0.8
            } else {
                0.68
            },
        );
    }
    if content_suggests_interest(c, nt) {
        let conf = if c.contains("兴趣") || c.contains("关注") {
            0.78
        } else {
            0.65
        };
        push_fact(&mut out, FACT_INTEREST, c.to_string(), mid, msg, conf);
    }
    if content_suggests_location(c, nt, &input.keywords) {
        let (loc, lconf) = extract_location_value(c, &input.keywords).unwrap_or_else(|| {
            (
                c.chars().take(120).collect::<String>(),
                if nt == "world" { 0.5 } else { 0.42 },
            )
        });
        push_fact(&mut out, FACT_LOCATION, loc, mid, msg, lconf);
    }
    if let Some(v) = explicit_communication_preference_value(c) {
        push_fact(
            &mut out,
            FACT_COMMUNICATION_PREFERENCE,
            v,
            mid,
            msg,
            0.86,
        );
    }

    let _ = input.preference_polarity.as_deref();
    let _ = input.temporal_state.as_deref();

    let mut deduped = Vec::new();
    merge_facts_dedupe(&mut deduped, out);
    deduped
}

pub fn facts_from_resolved_items(items: &[MemoryResolveInput]) -> Vec<StoredFact> {
    let mut acc = Vec::new();
    for item in items {
        let one = infer_facts_one(item);
        merge_facts_dedupe(&mut acc, one);
    }
    acc
}

pub fn traits_from_facts(facts: &[StoredFact]) -> TraitSnapshot {
    let mut interest_tags = Vec::new();
    let mut connection_goal_tags = Vec::new();
    let mut communication_preferences = Vec::new();
    let mut location_label: Option<String> = None;

    for f in facts {
        match f.fact_type.as_str() {
            FACT_INTEREST => {
                if !interest_tags.contains(&f.value) {
                    interest_tags.push(f.value.clone());
                }
            }
            FACT_GOAL => {
                if !connection_goal_tags.contains(&f.value) {
                    connection_goal_tags.push(f.value.clone());
                }
            }
            FACT_LOCATION => {
                if location_label.is_none() {
                    location_label = Some(f.value.clone());
                }
            }
            FACT_COMMUNICATION_PREFERENCE => {
                if !communication_preferences.contains(&f.value) {
                    communication_preferences.push(f.value.clone());
                }
            }
            _ => {}
        }
    }

    interest_tags.truncate(12);
    connection_goal_tags.truncate(8);
    communication_preferences.truncate(8);

    TraitSnapshot {
        interest_tags,
        connection_goal_tags,
        location_label,
        communication_preferences,
    }
}

fn short_type_label(fact_type: &str) -> &'static str {
    match fact_type {
        FACT_INTEREST => "兴趣",
        FACT_GOAL => "目标",
        FACT_LOCATION => "地点",
        FACT_COMMUNICATION_PREFERENCE => "沟通",
        _ => "事实",
    }
}

/// 刷新 traits、展示层 headline/bio/highlights，并在空字段时回填 city_level_location。
pub fn refresh_derived_profile_fields(
    facts: &[StoredFact],
    city_level_location: &mut String,
    headline: &mut String,
    bio: &mut String,
    memory_highlights: &mut Vec<String>,
    traits: &mut TraitSnapshot,
) {
    *traits = traits_from_facts(facts);

    if city_level_location.is_empty() {
        if let Some(loc) = traits.location_label.clone() {
            *city_level_location = loc;
        }
    }

    *headline = if facts.is_empty() {
        "记忆已同步".to_string()
    } else {
        format!("记忆已同步 · {} 条画像事实", facts.len())
    };

    let mut bio_parts: Vec<String> = vec![];
    if !traits.interest_tags.is_empty() {
        bio_parts.push(format!("兴趣：{}", traits.interest_tags.join("、")));
    }
    if !traits.connection_goal_tags.is_empty() {
        bio_parts.push(format!("目标：{}", traits.connection_goal_tags.join("、")));
    }
    if !traits.communication_preferences.is_empty() {
        bio_parts.push(format!(
            "沟通：{}",
            traits.communication_preferences.join("、")
        ));
    }
    *bio = if bio_parts.is_empty() {
        "来自聊天的结构化画像投影".to_string()
    } else {
        format!("结构化画像摘要 · {}", bio_parts.join(" · "))
    };

    memory_highlights.clear();
    for f in facts.iter().take(20) {
        memory_highlights.push(format!(
            "[{}] {}",
            short_type_label(&f.fact_type),
            f.value
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_resolve_inputs() -> Vec<MemoryResolveInput> {
        vec![
            MemoryResolveInput {
                memory_id: "m1".into(),
                content: "希望认识投资人".into(),
                network_type: "experience".into(),
                keywords: vec![],
                temporal_state: None,
                preference_polarity: Some("positive".into()),
                source_message_id: Some("msg-m1".into()),
            },
            MemoryResolveInput {
                memory_id: "m2".into(),
                content: "对创业或相关话题表达兴趣".into(),
                network_type: "opinion".into(),
                keywords: vec![],
                temporal_state: None,
                preference_polarity: Some("positive".into()),
                source_message_id: None,
            },
            MemoryResolveInput {
                memory_id: "m3".into(),
                content: "关注 AI / 人工智能相关主题".into(),
                network_type: "opinion".into(),
                keywords: vec![],
                temporal_state: None,
                preference_polarity: Some("positive".into()),
                source_message_id: None,
            },
            MemoryResolveInput {
                memory_id: "m4".into(),
                content: "请不要用推销式沟通，希望直接一点".into(),
                network_type: "opinion".into(),
                keywords: vec![],
                temporal_state: None,
                preference_polarity: None,
                source_message_id: Some("msg-m4".into()),
            },
        ]
    }

    #[test]
    fn explicit_communication_preference_hits() {
        let items = vec![MemoryResolveInput {
            memory_id: "c1".into(),
            content: "不要推销式沟通，我喜欢直接说话".into(),
            network_type: "experience".into(),
            keywords: vec![],
            temporal_state: None,
            preference_polarity: None,
            source_message_id: Some("s1".into()),
        }];
        let facts = facts_from_resolved_items(&items);
        let comm: Vec<_> = facts
            .iter()
            .filter(|f| f.fact_type == FACT_COMMUNICATION_PREFERENCE)
            .collect();
        assert_eq!(comm.len(), 1, "expected one explicit comm fact");
        assert!(comm[0].confidence > 0.8);
        assert_eq!(comm[0].source_memory_id, "c1");
        assert_eq!(comm[0].source_message_id.as_deref(), Some("s1"));
    }

    #[test]
    fn generic_positive_polarity_does_not_emit_communication_preference() {
        let items = vec![MemoryResolveInput {
            memory_id: "p1".into(),
            content: "今天心情很好，一切都很顺利".into(),
            network_type: "experience".into(),
            keywords: vec![],
            temporal_state: None,
            preference_polarity: Some("positive".into()),
            source_message_id: None,
        }];
        let facts = facts_from_resolved_items(&items);
        assert!(
            !facts
                .iter()
                .any(|f| f.fact_type == FACT_COMMUNICATION_PREFERENCE),
            "positive polarity alone must not create communication_preference"
        );
    }

    #[test]
    fn location_pattern_extracts_non_hardcoded_city() {
        let items = vec![MemoryResolveInput {
            memory_id: "l1".into(),
            content: "我现在在苏州，下周去上海".into(),
            network_type: "experience".into(),
            keywords: vec![],
            temporal_state: None,
            preference_polarity: None,
            source_message_id: None,
        }];
        let facts = facts_from_resolved_items(&items);
        let loc = facts.iter().find(|f| f.fact_type == FACT_LOCATION).expect("location");
        assert_eq!(loc.value, "苏州");
        assert!(loc.confidence > 0.7);
    }

    #[test]
    fn location_base_pattern() {
        let items = vec![MemoryResolveInput {
            memory_id: "l2".into(),
            content: "长期base在义乌做外贸".into(),
            network_type: "experience".into(),
            keywords: vec![],
            temporal_state: None,
            preference_polarity: None,
            source_message_id: None,
        }];
        let facts = facts_from_resolved_items(&items);
        let loc = facts.iter().find(|f| f.fact_type == FACT_LOCATION).expect("location");
        assert_eq!(loc.value, "义乌");
    }

    #[test]
    fn merge_duplicate_batch_idempotent_by_fact_key() {
        let batch = vec![StoredFact {
            fact_type: FACT_GOAL.to_string(),
            value: "希望认识投资人".to_string(),
            source_memory_id: "a".to_string(),
            source_message_id: Some("x".to_string()),
            confidence: 0.9,
        }];
        let mut target = Vec::new();
        merge_facts_dedupe(&mut target, batch.clone());
        let n = target.len();
        merge_facts_dedupe(&mut target, batch);
        assert_eq!(target.len(), n);
        assert_eq!(target[0].source_memory_id, "a");
    }

    #[test]
    fn smoke_like_messages_produce_structured_facts_and_traits() {
        let facts = facts_from_resolved_items(&sample_resolve_inputs());
        assert!(
            facts.iter().any(|f| f.fact_type == FACT_GOAL),
            "expected goal fact"
        );
        let interest_n = facts.iter().filter(|f| f.fact_type == FACT_INTEREST).count();
        assert!(
            interest_n >= 2,
            "expected at least two interest facts, got {interest_n}"
        );
        let comm: Vec<_> = facts
            .iter()
            .filter(|f| f.fact_type == FACT_COMMUNICATION_PREFERENCE)
            .collect();
        assert_eq!(comm.len(), 1, "expected one explicit communication fact from m4");
        assert_eq!(comm[0].source_memory_id, "m4");

        let traits = traits_from_facts(&facts);
        assert!(!traits.interest_tags.is_empty());
        assert!(!traits.connection_goal_tags.is_empty());
        assert!(!traits.communication_preferences.is_empty());
        assert!(traits.location_label.is_none());
    }

    #[test]
    fn headline_keeps_benchmark_substring() {
        let facts = facts_from_resolved_items(&sample_resolve_inputs());
        let mut city = String::new();
        let mut headline = String::new();
        let mut bio = String::new();
        let mut highlights = vec![];
        let mut traits = TraitSnapshot::default();
        refresh_derived_profile_fields(
            &facts,
            &mut city,
            &mut headline,
            &mut bio,
            &mut highlights,
            &mut traits,
        );
        assert!(
            headline.contains("记忆已同步"),
            "benchmark-asmr-lite-v1 expects this substring"
        );
    }
}
