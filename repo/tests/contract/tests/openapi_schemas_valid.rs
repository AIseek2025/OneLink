use std::fs;

#[test]
fn all_openapi_yaml_files_are_valid() {
    let dir = "../../platform/contracts/openapi";
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot read {dir}: {e}"));
    let mut count = 0;
    for entry in entries {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "yaml") {
            continue;
        }
        let content =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {:?}: {e}", path));
        serde_yaml::from_str::<serde_json::Value>(&content)
            .unwrap_or_else(|e| panic!("invalid YAML in {:?}: {e}", path));
        count += 1;
    }
    assert!(
        count >= 4,
        "expected at least 4 OpenAPI schemas, found {count}"
    );
}
