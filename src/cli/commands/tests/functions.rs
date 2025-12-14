//! Functions tests for CLI commands

use crate::functions::registry;

// =========================================================================
// Registry Tests
// =========================================================================

#[test]
fn test_registry_counts() {
    // Registry is the single source of truth
    assert_eq!(
        registry::count_enterprise(),
        173,
        "Enterprise should have 173 functions"
    );
    assert_eq!(registry::count_demo(), 48, "Demo should have 48 functions");
}

#[test]
fn test_registry_demo_functions_are_scalar() {
    // All demo functions must work with v1.0.0 (scalar-only)
    for func in registry::demo_functions() {
        assert!(
            func.scalar,
            "Demo function {} must be scalar (v1.0.0 compatible)",
            func.name
        );
    }
}
