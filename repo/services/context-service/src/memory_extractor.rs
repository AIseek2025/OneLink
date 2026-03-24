//! Memory extraction — MVP 启发式；为后续 ASMR-Lite Search/Reason 路由预留模块边界。

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedArtifactCandidate {
    pub network_type: String,
    pub evidence_type: String,
    pub memory_level: String,
    pub content: String,
    pub source_type: String,
    pub confidence: f64,
}

/// 生成符合 `context.memory.extracted.v1` 的 `artifact_candidates` 数组元素（含扩展数值字段）。
pub fn heuristic_extract(user_message: &str) -> Vec<Value> {
    let t = user_message.trim();
    if t.is_empty() {
        return vec![];
    }

    let mut out: Vec<Value> = vec![];
    let lowered = t.to_lowercase();

    if let Some(city) = extract_city_after_markers(t, &["现在在", "目前在", "当前在"]) {
        out.push(json!({
            "network_type": "world",
            "evidence_type": "fact",
            "memory_level": "persistent",
            "content": format!("当前所在城市是{}", city),
            "content_structured": { "kind": "current_city", "city": city, "temporal_state": "current" },
            "source_type": "chat",
            "confidence": 0.9,
            "importance_score": 0.82,
            "consistency_score": 0.9,
            "entity_refs": [city]
        }));
    }

    if let Some(city) = extract_city_after_markers(t, &["之前在", "以前在", "曾经在"]) {
        out.push(json!({
            "network_type": "experience",
            "evidence_type": "fact",
            "memory_level": "persistent",
            "content": format!("之前所在城市是{}", city),
            "content_structured": { "kind": "past_city", "city": city, "temporal_state": "past" },
            "source_type": "chat",
            "confidence": 0.83,
            "importance_score": 0.73,
            "consistency_score": 0.85,
            "entity_refs": [city]
        }));
    }

    if t.contains("远程办公") || t.contains("远程") {
        let temporal_state = if t.contains("改为") || t.contains("后来") || t.contains("现在")
        {
            "updated"
        } else {
            "current"
        };
        out.push(json!({
            "network_type": "experience",
            "evidence_type": "fact",
            "memory_level": "persistent",
            "content": "当前办公方式偏向远程办公",
            "content_structured": { "kind": "work_mode", "mode": "remote", "temporal_state": temporal_state },
            "source_type": "chat",
            "confidence": 0.81,
            "importance_score": 0.72,
            "consistency_score": 0.84,
            "entity_refs": ["远程办公"]
        }));
    }

    if t.contains("不喜欢被推销") || t.contains("不喜欢推销") || t.contains("不喜欢被推销式沟通")
    {
        out.push(json!({
            "network_type": "opinion",
            "evidence_type": "fact",
            "memory_level": "persistent",
            "content": "不喜欢被推销式沟通",
            "content_structured": { "kind": "communication_preference", "polarity": "negative", "topic": "推销式沟通" },
            "source_type": "chat",
            "confidence": 0.91,
            "importance_score": 0.8,
            "consistency_score": 0.9,
            "entity_refs": ["推销式沟通"]
        }));
    }

    // 显式沟通风格子句：与 profile-service Phase B `explicit_communication_preference_value` 口径对齐。
    // 长句会被拆成多条短 artifact；若不单独抽出含「拐弯抹角 / 希望直接一点」等片段，profile 侧永远收不到可映射文本。
    if let Some(clause) = extract_explicit_communication_clause(t) {
        out.push(json!({
            "network_type": "opinion",
            "evidence_type": "fact",
            "memory_level": "persistent",
            "content": clause,
            "content_structured": { "kind": "communication_preference", "topic": "style" },
            "source_type": "chat",
            "confidence": 0.88,
            "importance_score": 0.76,
            "consistency_score": 0.86,
            "entity_refs": []
        }));
    }

    if t.contains("创业") || lowered.contains("startup") {
        out.push(json!({
            "network_type": "opinion",
            "evidence_type": "inference",
            "memory_level": "persistent",
            "content": "对创业或相关话题表达兴趣",
            "content_structured": {},
            "source_type": "chat",
            "confidence": 0.82,
            "importance_score": 0.74,
            "consistency_score": 0.88,
            "entity_refs": []
        }));
    }

    if t.contains("AI") || t.contains("人工智能") {
        out.push(json!({
            "network_type": "opinion",
            "evidence_type": "fact",
            "memory_level": "persistent",
            "content": "关注 AI / 人工智能相关主题",
            "content_structured": {},
            "source_type": "chat",
            "confidence": 0.79,
            "importance_score": 0.71,
            "consistency_score": 0.86,
            "entity_refs": []
        }));
    }

    if t.contains("投资人") {
        out.push(json!({
            "network_type": "experience",
            "evidence_type": "inference",
            "memory_level": "persistent",
            "content": "希望认识投资人",
            "content_structured": { "kind": "connection_goal", "target": "投资人" },
            "source_type": "chat",
            "confidence": 0.84,
            "importance_score": 0.78,
            "consistency_score": 0.87,
            "entity_refs": ["投资人"]
        }));
    }

    if t.contains("技术合伙人") || t.contains("合伙人") {
        out.push(json!({
            "network_type": "experience",
            "evidence_type": "inference",
            "memory_level": "persistent",
            "content": "希望认识技术合伙人",
            "content_structured": { "kind": "connection_goal", "target": "技术合伙人" },
            "source_type": "chat",
            "confidence": 0.84,
            "importance_score": 0.78,
            "consistency_score": 0.87,
            "entity_refs": ["技术合伙人"]
        }));
    }

    if out.is_empty() {
        let preview: String = t.chars().take(120).collect();
        out.push(json!({
            "network_type": "experience",
            "evidence_type": "fact",
            "memory_level": "working",
            "content": format!("聊天陈述摘要: {}", preview),
            "content_structured": {},
            "source_type": "chat",
            "confidence": 0.55,
            "importance_score": 0.5,
            "consistency_score": 0.6,
            "entity_refs": []
        }));
    }

    out
}

/// 从整句中切出**单条**显式沟通偏好子句（按 `；` / `;` 分片），供 profile 映射 `communication_preference`。
fn extract_explicit_communication_clause(t: &str) -> Option<String> {
    for part in t.split(|c| c == '；' || c == ';') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        if clause_is_explicit_communication_preference(p) {
            return Some(p.to_string());
        }
    }
    let whole = t.trim();
    if clause_is_explicit_communication_preference(whole) {
        return Some(whole.to_string());
    }
    None
}

/// 与 `profile-service` `projection::explicit_communication_preference_value` 保持同构，避免泛化极性包装。
fn clause_is_explicit_communication_preference(p: &str) -> bool {
    if p.contains("不要推销")
        || p.contains("别推销")
        || p.contains("讨厌推销")
        || p.contains("不要推销式")
        || p.contains("推销式")
    {
        return true;
    }
    if p.contains("拐弯抹角")
        && (p.contains("不喜欢")
            || p.contains("讨厌")
            || p.contains("别")
            || p.contains("不要"))
    {
        return true;
    }
    if p.contains("直接一点") || p.contains("直接一些") || p.contains("直接些") {
        return true;
    }
    if (p.contains("喜欢") || p.contains("希望") || p.contains("想要") || p.contains("更"))
        && p.contains("直接")
        && (p.contains("沟通") || p.contains("说话") || p.contains("表达") || p.contains("点"))
    {
        return true;
    }
    if p.contains("沟通方式")
        || p.contains("邮件联系")
        || p.contains("不要打电话")
        || p.contains("别打电话")
    {
        return true;
    }
    if p.contains("微信") && (p.contains("联系") || p.contains("加我") || p.contains("用微信")) {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_like_message_emits_explicit_communication_clause_artifact() {
        let msg = "我对 AI 创业很感兴趣，希望认识投资人；沟通上不喜欢拐弯抹角，希望直接一点。";
        let v = heuristic_extract(msg);
        let contents: Vec<&str> = v
            .iter()
            .filter_map(|x| x.get("content").and_then(|c| c.as_str()))
            .collect();
        assert!(
            contents.iter().any(|c| c.contains("拐弯抹角") && c.contains("不喜欢")),
            "expected communication clause artifact, got: {:?}",
            contents
        );
    }
}

fn extract_city_after_markers<'a>(input: &'a str, markers: &[&str]) -> Option<&'a str> {
    const CITIES: [&str; 4] = ["上海", "北京", "深圳", "杭州"];
    for marker in markers {
        if let Some(start) = input.find(marker) {
            let slice = &input[start + marker.len()..];
            for city in CITIES {
                if slice.starts_with(city) {
                    return Some(city);
                }
            }
        }
    }
    None
}
