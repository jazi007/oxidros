//! Integration tests for node name, namespace, and topic remapping.

use oxidros_zenoh::Context;
use ros2args::names::NameKind;
use ros2args::{RemapRule, Ros2Args};

/// Helper to create Ros2Args with remap rules
fn args_with_remaps(rules: Vec<RemapRule>) -> Ros2Args {
    Ros2Args {
        remap_rules: rules,
        ..Default::default()
    }
}

// ============================================================================
// Node name remapping tests
// ============================================================================

#[test]
fn test_node_name_remapping_via_node_methods() {
    let args = args_with_remaps(vec![RemapRule::new_global(
        "__node".to_string(),
        "remapped_node".to_string(),
    )]);

    let ctx = Context::with_args(args).expect("Failed to create context");
    let node = ctx
        .create_node("original_node", None)
        .expect("Failed to create node");

    // name() should return the effective (remapped) name
    assert_eq!(node.name(), "remapped_node");
}

#[test]
fn test_node_namespace_remapping_via_node_methods() {
    let args = args_with_remaps(vec![RemapRule::new_global(
        "__ns".to_string(),
        "/remapped_ns".to_string(),
    )]);

    let ctx = Context::with_args(args).expect("Failed to create context");
    let node = ctx
        .create_node("my_node", Some("/original_ns"))
        .expect("Failed to create node");

    // namespace() should return the effective (remapped) namespace
    assert_eq!(node.namespace(), "/remapped_ns");
}

#[test]
fn test_node_fqn_with_both_remappings() {
    let args = args_with_remaps(vec![
        RemapRule::new_global("__node".to_string(), "new_node".to_string()),
        RemapRule::new_global("__ns".to_string(), "/new_ns".to_string()),
    ]);

    let ctx = Context::with_args(args).expect("Failed to create context");
    let node = ctx
        .create_node("old_node", Some("/old_ns"))
        .expect("Failed to create node");

    assert_eq!(node.name(), "new_node");
    assert_eq!(node.namespace(), "/new_ns");
    assert_eq!(node.fully_qualified_name(), "/new_ns/new_node");
}

#[test]
fn test_node_no_remapping() {
    let args = Ros2Args::default();

    let ctx = Context::with_args(args).expect("Failed to create context");
    let node = ctx
        .create_node("my_node", Some("/my_ns"))
        .expect("Failed to create node");

    assert_eq!(node.name(), "my_node");
    assert_eq!(node.namespace(), "/my_ns");
    assert_eq!(node.fully_qualified_name(), "/my_ns/my_node");
}

// ============================================================================
// Topic name remapping tests
// ============================================================================

#[test]
fn test_topic_remapping_absolute_name() {
    let args = args_with_remaps(vec![RemapRule::new_global(
        "/chatter".to_string(),
        "/remapped_chatter".to_string(),
    )]);

    let ctx = Context::with_args(args).expect("Failed to create context");
    let node = ctx
        .create_node("my_node", None)
        .expect("Failed to create node");

    let result = node
        .expand_and_remap_name("/chatter", NameKind::Topic)
        .expect("Failed to expand name");

    assert_eq!(result, "/remapped_chatter");
}

#[test]
fn test_topic_remapping_relative_name() {
    let args = args_with_remaps(vec![RemapRule::new_global(
        "chatter".to_string(),
        "remapped_chatter".to_string(),
    )]);

    let ctx = Context::with_args(args).expect("Failed to create context");
    let node = ctx
        .create_node("my_node", Some("/my_ns"))
        .expect("Failed to create node");

    let result = node
        .expand_and_remap_name("chatter", NameKind::Topic)
        .expect("Failed to expand name");

    // Relative names are expanded with namespace, then remapping is applied
    assert_eq!(result, "/my_ns/remapped_chatter");
}

#[test]
fn test_topic_remapping_private_name() {
    let args = args_with_remaps(vec![RemapRule::new_global(
        "~/private_topic".to_string(),
        "/global_topic".to_string(),
    )]);

    let ctx = Context::with_args(args).expect("Failed to create context");
    let node = ctx
        .create_node("my_node", Some("/my_ns"))
        .expect("Failed to create node");

    let result = node
        .expand_and_remap_name("~/private_topic", NameKind::Topic)
        .expect("Failed to expand name");

    assert_eq!(result, "/global_topic");
}

#[test]
fn test_topic_no_remapping() {
    let args = Ros2Args::default();

    let ctx = Context::with_args(args).expect("Failed to create context");
    let node = ctx
        .create_node("my_node", Some("/my_ns"))
        .expect("Failed to create node");

    // Relative name expands to /my_ns/chatter
    let result = node
        .expand_and_remap_name("chatter", NameKind::Topic)
        .expect("Failed to expand name");
    assert_eq!(result, "/my_ns/chatter");

    // Private name expands to /my_ns/my_node/data
    let result = node
        .expand_and_remap_name("~/data", NameKind::Topic)
        .expect("Failed to expand name");
    assert_eq!(result, "/my_ns/my_node/data");

    // Absolute name stays as-is
    let result = node
        .expand_and_remap_name("/absolute", NameKind::Topic)
        .expect("Failed to expand name");
    assert_eq!(result, "/absolute");
}

#[test]
fn test_topic_node_specific_remapping() {
    let args = args_with_remaps(vec![
        RemapRule::new_node_specific(
            "target_node".to_string(),
            "/topic_a".to_string(),
            "/remapped_a".to_string(),
        ),
        RemapRule::new_node_specific(
            "other_node".to_string(),
            "/topic_a".to_string(),
            "/other_remap".to_string(),
        ),
    ]);

    let ctx = Context::with_args(args).expect("Failed to create context");

    // Create target_node - should get the remapping
    let target = ctx
        .create_node("target_node", None)
        .expect("Failed to create node");
    let result = target
        .expand_and_remap_name("/topic_a", NameKind::Topic)
        .expect("Failed to expand name");
    assert_eq!(result, "/remapped_a");

    // Create unrelated_node - should NOT get any remapping
    let unrelated = ctx
        .create_node("unrelated_node", None)
        .expect("Failed to create node");
    let result = unrelated
        .expand_and_remap_name("/topic_a", NameKind::Topic)
        .expect("Failed to expand name");
    assert_eq!(result, "/topic_a"); // No change
}

// ============================================================================
// Combined node + topic remapping tests
// ============================================================================

#[test]
fn test_private_topic_uses_effective_node_name() {
    // When node name is remapped, private topics should use the remapped name
    let args = args_with_remaps(vec![RemapRule::new_global(
        "__node".to_string(),
        "renamed_node".to_string(),
    )]);

    let ctx = Context::with_args(args).expect("Failed to create context");
    let node = ctx
        .create_node("original_node", Some("/ns"))
        .expect("Failed to create node");

    // ~/data should expand using the effective name
    let result = node
        .expand_and_remap_name("~/data", NameKind::Topic)
        .expect("Failed to expand name");

    assert_eq!(result, "/ns/renamed_node/data");
}

#[test]
fn test_topic_remapping_still_matches_original_node_name() {
    // Node-specific topic rules should match against the ORIGINAL node name
    let args = args_with_remaps(vec![
        RemapRule::new_global("__node".to_string(), "renamed_node".to_string()),
        RemapRule::new_node_specific(
            "original_node".to_string(), // Rule targets original name
            "/topic".to_string(),
            "/remapped_topic".to_string(),
        ),
    ]);

    let ctx = Context::with_args(args).expect("Failed to create context");
    let node = ctx
        .create_node("original_node", None)
        .expect("Failed to create node");

    // Node name should be remapped
    assert_eq!(node.name(), "renamed_node");

    // But topic rule should still match (using original name for matching)
    let result = node
        .expand_and_remap_name("/topic", NameKind::Topic)
        .expect("Failed to expand name");
    assert_eq!(result, "/remapped_topic");
}
