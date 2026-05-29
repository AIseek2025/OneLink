use std::fs;

const PROMOTION_RECORD_PATH: &str = "../../../docs/autopilot/phase_promotion_phase1_to_phase2.md";
const AUDIT_REPORT_9_PATH: &str = "../../../reports/audit_report_iteration_9.md";

#[test]
fn phase_1_to_2_promotion_record_exists() {
    assert!(
        fs::metadata(PROMOTION_RECORD_PATH).is_ok(),
        "Phase 1 → Phase 2 promotion record must exist at docs/autopilot/phase_promotion_phase1_to_phase2.md"
    );
}

#[test]
fn phase_1_promotion_record_references_approved_audit() {
    let content = fs::read_to_string(PROMOTION_RECORD_PATH)
        .unwrap_or_else(|e| panic!("cannot read promotion record: {e}"));

    assert!(
        content.contains("approved"),
        "Promotion record must reference the approved audit conclusion"
    );
    assert!(
        content.contains("audit_report_iteration_9"),
        "Promotion record must reference audit_report_iteration_9.md as the approval source"
    );
    assert!(
        content.contains("work_report_iteration_9"),
        "Promotion record must reference work_report_iteration_9.md as the evidence source"
    );
}

#[test]
fn phase_1_audit_report_9_concludes_approved() {
    let content = fs::read_to_string(AUDIT_REPORT_9_PATH)
        .unwrap_or_else(|e| panic!("cannot read audit report 9: {e}"));

    assert!(
        content.contains("approved"),
        "audit_report_iteration_9.md must conclude 'approved'"
    );
    assert!(
        content.contains("是否允许进入下一 phase") && content.contains("yes"),
        "audit_report_iteration_9.md must explicitly allow entering next phase"
    );
}

#[test]
fn phase_1_promotion_record_covers_all_done_criteria() {
    let content = fs::read_to_string(PROMOTION_RECORD_PATH)
        .unwrap_or_else(|e| panic!("cannot read promotion record: {e}"));

    assert!(
        content.contains("P0-1") || content.contains("Security Baseline"),
        "Promotion record must cover P0-1 Security Baseline"
    );
    assert!(
        content.contains("P0-2") || content.contains("Persistence Baseline"),
        "Promotion record must cover P0-2 Persistence Baseline"
    );
    assert!(
        content.contains("P0-3") || content.contains("Automated Quality Baseline"),
        "Promotion record must cover P0-3 Automated Quality Baseline"
    );
}

#[test]
fn phase_1_promotion_record_references_persistence_smoke_evidence() {
    let content = fs::read_to_string(PROMOTION_RECORD_PATH)
        .unwrap_or_else(|e| panic!("cannot read promotion record: {e}"));

    assert!(
        content.contains("persistence_smoke") || content.contains("PERSISTENCE SMOKE"),
        "Promotion record must reference real Postgres persistence smoke evidence"
    );
}

#[test]
fn phase_1_promotion_record_satisfies_promotion_rule() {
    let content = fs::read_to_string(PROMOTION_RECORD_PATH)
        .unwrap_or_else(|e| panic!("cannot read promotion record: {e}"));

    assert!(
        content.contains("work_report_iteration_9"),
        "Promotion record must reference latest work_report (iteration 9)"
    );
    assert!(
        content.contains("audit_report_iteration_9"),
        "Promotion record must reference latest audit_report (iteration 9)"
    );
    assert!(
        content.contains("approved"),
        "Promotion record must state current phase pass conclusion"
    );
    assert!(
        content.contains("phase_handoff_iteration_10") || content.contains("Phase 2"),
        "Promotion record must reference next phase handoff"
    );
}

#[test]
fn phase_1_promotion_record_has_reverification_status() {
    let content = fs::read_to_string(PROMOTION_RECORD_PATH)
        .unwrap_or_else(|e| panic!("cannot read promotion record: {e}"));

    assert!(
        content.contains("Re-verification and Current Status"),
        "Promotion record must have a re-verification section acknowledging current governance state"
    );
    assert!(
        content.contains("historical fact"),
        "Promotion record must clarify that the original promotion is a historical fact"
    );
    assert!(
        content.contains("iteration 19") || content.contains("iteration 21"),
        "Promotion record must reference iteration 19 or 21 re-verification evidence"
    );
}

#[test]
fn phase_1_promotion_record_acknowledges_governance_conflict() {
    let content = fs::read_to_string(PROMOTION_RECORD_PATH)
        .unwrap_or_else(|e| panic!("cannot read promotion record: {e}"));

    assert!(
        content.contains("Governance Conflict Acknowledgment"),
        "Promotion record must have a Governance Conflict Acknowledgment section"
    );
    assert!(
        content.contains("needs_followup"),
        "Promotion record must reference the needs_followup audit conclusion that triggered the conflict"
    );
    assert!(
        content.contains("suspended")
            || content.contains("RESOLVED")
            || content.contains("resolved"),
        "Promotion record must acknowledge the governance conflict (suspended or resolved status)"
    );
    assert!(
        content.contains("audit_report_iteration_21") || content.contains("iteration 21"),
        "Promotion record must reference the iteration 21 audit that resolved the governance conflict"
    );
}
