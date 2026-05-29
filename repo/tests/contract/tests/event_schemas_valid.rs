use std::fs;

#[test]
fn all_event_json_schemas_are_valid() {
    let dir = "../../data-platform/event-schemas";
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot read {dir}: {e}"));
    let mut count = 0;
    for entry in entries {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let content =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {:?}: {e}", path));
        serde_json::from_str::<serde_json::Value>(&content)
            .unwrap_or_else(|e| panic!("invalid JSON in {:?}: {e}", path));
        count += 1;
    }
    assert!(
        count >= 8,
        "expected at least 8 event schemas, found {count}"
    );
}
