//! Fails when a workspace crate breaks the hexagonal dependency direction.

use std::path::Path;

use cargo_metadata::MetadataCommand;

const DOMAIN: &str = "domain";
const APPLICATION: &str = "application";
const ADAPTER_PREFIX: &str = "adapter-";
const SHARED_ADAPTER: &str = "adapter-fs";

/// Returns why `from -> to` is forbidden, or `None` when the edge is allowed.
/// Both ends are workspace crates.
fn violation(from: &str, to: &str) -> Option<&'static str> {
    match from {
        DOMAIN => Some("domain must not depend on any workspace crate"),
        APPLICATION if to != DOMAIN => Some("application may depend on domain only"),
        adapter
            if adapter.starts_with(ADAPTER_PREFIX)
                && to.starts_with(ADAPTER_PREFIX)
                && (to != SHARED_ADAPTER || adapter == SHARED_ADAPTER) =>
        {
            Some("an adapter may depend on adapter-fs only among adapters")
        }
        _ => None,
    }
}

#[test]
fn workspace_respects_hexagonal_dependency_direction() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml");
    let metadata = MetadataCommand::new()
        .manifest_path(manifest)
        .no_deps()
        .exec()
        .expect("cargo metadata should succeed");

    let members: Vec<&str> = metadata
        .workspace_packages()
        .iter()
        .map(|package| package.name.as_str())
        .collect();

    let violations: Vec<String> = metadata
        .workspace_packages()
        .iter()
        .flat_map(|package| {
            package
                .dependencies
                .iter()
                .filter(|dependency| members.contains(&dependency.name.as_str()))
                .filter_map(|dependency| {
                    violation(&package.name, &dependency.name)
                        .map(|reason| format!("{} -> {}: {reason}", package.name, dependency.name))
                })
        })
        .collect();

    assert!(
        violations.is_empty(),
        "forbidden dependencies:\n{}",
        violations.join("\n")
    );
}

#[test]
fn rules_reject_forbidden_edges_and_accept_allowed_ones() {
    assert!(violation("domain", "application").is_some());
    assert!(violation("application", "adapter-fs").is_some());
    assert!(violation("adapter-claude", "adapter-codex").is_some());
    assert!(violation("adapter-fs", "adapter-claude").is_some());

    assert!(violation("application", "domain").is_none());
    assert!(violation("adapter-claude", "adapter-fs").is_none());
    assert!(violation("adapter-codex", "application").is_none());
}
