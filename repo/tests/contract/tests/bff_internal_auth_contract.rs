use std::collections::HashSet;
use std::fs;

#[test]
fn bff_contract_has_no_internal_routes() {
    let path = "../../platform/contracts/openapi/bff.yaml";
    let content = fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
    let doc: serde_yaml::Value =
        serde_yaml::from_str(&content).unwrap_or_else(|e| panic!("invalid YAML in {path}: {e}"));
    let paths = doc
        .get("paths")
        .unwrap_or_else(|| panic!("bff.yaml missing 'paths' key"));
    let path_keys: HashSet<String> = paths
        .as_mapping()
        .expect("paths must be a mapping")
        .keys()
        .map(|k| k.as_str().unwrap().to_string())
        .collect();
    assert!(
        !path_keys.is_empty(),
        "bff.yaml must define at least 1 path"
    );
    let internal_paths: Vec<&str> = path_keys
        .iter()
        .filter(|p| p.starts_with("/internal/"))
        .map(String::as_str)
        .collect();
    assert!(
        internal_paths.is_empty(),
        "BFF public contract must not expose internal routes, found: {internal_paths:?}"
    );
}
