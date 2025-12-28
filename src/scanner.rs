//! File system scanner for Rust files

use crate::lexer::{extract_rule_references, RuleReference};
use eyre::Result;
use std::path::Path;
use walkdir::WalkDir;

/// Scan a directory for Rust files and extract all rule references
pub fn scan_directory(
    root: &Path,
    include_patterns: &[String],
    exclude_patterns: &[String],
) -> Result<Vec<RuleReference>> {
    let mut all_references = Vec::new();

    for entry in WalkDir::new(root)
        .follow_links(true)
        .into_iter()
        .filter_entry(|e| !is_excluded(e.path(), root, exclude_patterns))
    {
        let entry = entry?;
        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        // Only process .rs files that match include patterns
        if path.extension().is_some_and(|ext| ext == "rs")
            && is_included(path, root, include_patterns)
        {
            let content = std::fs::read_to_string(path)?;
            let refs = extract_rule_references(path, &content)?;
            all_references.extend(refs);
        }
    }

    Ok(all_references)
}

/// Check if a path matches any include pattern
fn is_included(path: &Path, root: &Path, patterns: &[String]) -> bool {
    // If no patterns specified, include everything
    if patterns.is_empty() {
        return true;
    }

    let relative = path.strip_prefix(root).unwrap_or(path);
    let relative_str = relative.to_string_lossy();

    for pattern in patterns {
        if matches_glob(&relative_str, pattern) {
            return true;
        }
    }

    false
}

/// Check if a path matches any exclude pattern
fn is_excluded(path: &Path, root: &Path, patterns: &[String]) -> bool {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let relative_str = relative.to_string_lossy();

    for pattern in patterns {
        if matches_glob(&relative_str, pattern) {
            return true;
        }
    }

    false
}

/// Simple glob matching (supports * and **)
fn matches_glob(path: &str, pattern: &str) -> bool {
    // Handle the common case of **/*.rs
    if pattern == "**/*.rs" {
        return path.ends_with(".rs");
    }

    // Handle target/** exclusion
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return path.starts_with(prefix);
    }

    // Fallback: simple contains check for the non-wildcard parts
    let parts: Vec<&str> = pattern.split('*').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return true;
    }

    let mut remaining = path;
    for part in parts {
        if let Some(idx) = remaining.find(part) {
            remaining = &remaining[idx + part.len()..];
        } else {
            return false;
        }
    }

    true
}
