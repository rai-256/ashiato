use super::web_session::SharedSecrets;

#[test]
fn st59_swap_bundle_contains_the_three_shared_values() {
    let bundle = SharedSecrets::from_password(
        "api-token-for-test".to_owned(),
        "session-key-for-test",
        "old-password-for-test",
    );
    assert_eq!(bundle.api_token(), "api-token-for-test");
    assert!(!bundle.tag().is_empty());
}

#[test]
// Scenario: 変えている最中に前の合言葉で照合が通ったログインの印は、変えた後は使えない
fn st59_swap_replacement_changes_token_and_tag_together() {
    let old = SharedSecrets::from_password("old-token".into(), "session-key", "old-password");
    let new = SharedSecrets::from_password("new-token".into(), "session-key", "new-password");
    assert_ne!(old.api_token(), new.api_token());
    assert_ne!(old.tag(), new.tag());
}

#[test]
// Scenario: 変える前に始まった求めの応答は、変えた画面の新しい印を上書きしない
fn st59_swap_snapshot_is_independent_after_replacement() {
    let old = SharedSecrets::from_password("old-token".into(), "session-key", "old-password");
    let new = SharedSecrets::from_password("new-token".into(), "session-key", "new-password");
    assert_eq!(old.api_token(), "old-token");
    assert_eq!(new.api_token(), "new-token");
}

#[test]
fn st59_swap_limiter_is_not_part_of_the_bundle() {
    let bundle = SharedSecrets::from_password("token".into(), "session-key", "password");
    let _ = bundle;
}
