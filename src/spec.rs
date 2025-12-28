//! Spec manifest fetching and parsing
//!
//! Fetches `_rules.json` from a spec URL and parses the rule definitions.

use eyre::{Result, WrapErr};
use serde::Deserialize;
use std::collections::HashMap;

/// A rule definition from the spec manifest
#[derive(Debug, Clone, Deserialize)]
pub struct RuleInfo {
    /// URL fragment to link to this rule
    #[allow(dead_code)]
    pub url: String,
}

/// The spec manifest structure (from _rules.json)
#[derive(Debug, Clone, Deserialize)]
pub struct SpecManifest {
    /// Map of rule IDs to their info
    pub rules: HashMap<String, RuleInfo>,
}

impl SpecManifest {
    /// Fetch a spec manifest from a URL
    pub async fn fetch(url: &str) -> Result<Self> {
        let response = reqwest::get(url)
            .await
            .wrap_err_with(|| format!("Failed to fetch spec manifest from {}", url))?;

        if !response.status().is_success() {
            eyre::bail!(
                "Failed to fetch spec manifest from {}: HTTP {}",
                url,
                response.status()
            );
        }

        let manifest: SpecManifest = response
            .json()
            .await
            .wrap_err_with(|| format!("Failed to parse spec manifest from {}", url))?;

        Ok(manifest)
    }

    /// Get the set of all rule IDs in this manifest
    pub fn rule_ids(&self) -> impl Iterator<Item = &str> {
        self.rules.keys().map(|s| s.as_str())
    }

    /// Check if a rule ID exists in this manifest
    pub fn has_rule(&self, id: &str) -> bool {
        self.rules.contains_key(id)
    }

    /// Get the URL for a rule
    #[allow(dead_code)]
    pub fn get_rule_url(&self, id: &str) -> Option<&str> {
        self.rules.get(id).map(|r| r.url.as_str())
    }
}
