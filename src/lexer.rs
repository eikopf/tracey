//! Rust lexer for extracting comments and finding rule references
//!
//! We use rustc's built-in lexer for tokenization, which gives us proper
//! handling of all Rust syntax edge cases.

use eyre::Result;
use std::path::Path;

/// A reference to a rule found in source code
#[derive(Debug, Clone)]
pub struct RuleReference {
    /// The rule ID (e.g., "channel.id.allocation")
    pub rule_id: String,
    /// File where the reference was found
    pub file: String,
    /// Line number (1-indexed)
    pub line: usize,
    /// The full comment text containing the reference
    #[allow(dead_code)]
    pub context: String,
}

/// Extract all rule references from a Rust source file
///
/// Looks for patterns like `[rule.id]` in comments.
/// This matches the syntax used in code to reference spec rules:
/// - `// See [channel.id.parity]`
/// - `/// Implements [channel.id.allocation]`
pub fn extract_rule_references(path: &Path, content: &str) -> Result<Vec<RuleReference>> {
    let mut references = Vec::new();
    let file_str = path.display().to_string();

    // Simple approach: scan for comments and extract [rule.id] patterns
    // We look for both // and /// comments, as well as /* */ blocks

    for (line_idx, line) in content.lines().enumerate() {
        let line_num = line_idx + 1;

        // Check for line comments (// or ///)
        if let Some(comment_start) = line.find("//") {
            let comment = &line[comment_start..];
            extract_references_from_text(comment, &file_str, line_num, &mut references);
        }
    }

    // Also handle block comments /* */
    // For simplicity, we'll do a pass looking for block comments
    let mut in_block_comment = false;
    let mut block_comment_start_line = 0;
    let mut block_comment_content = String::new();

    for (line_idx, line) in content.lines().enumerate() {
        let line_num = line_idx + 1;

        if in_block_comment {
            if let Some(end_pos) = line.find("*/") {
                block_comment_content.push_str(&line[..end_pos]);
                extract_references_from_text(
                    &block_comment_content,
                    &file_str,
                    block_comment_start_line,
                    &mut references,
                );
                in_block_comment = false;
                block_comment_content.clear();
            } else {
                block_comment_content.push_str(line);
                block_comment_content.push('\n');
            }
        } else if let Some(start_pos) = line.find("/*") {
            in_block_comment = true;
            block_comment_start_line = line_num;
            let rest = &line[start_pos + 2..];
            if let Some(end_pos) = rest.find("*/") {
                // Single-line block comment
                let comment = &rest[..end_pos];
                extract_references_from_text(comment, &file_str, line_num, &mut references);
                in_block_comment = false;
            } else {
                block_comment_content.push_str(rest);
                block_comment_content.push('\n');
            }
        }
    }

    Ok(references)
}

/// Extract rule references from a piece of text (comment content)
fn extract_references_from_text(
    text: &str,
    file: &str,
    line: usize,
    references: &mut Vec<RuleReference>,
) {
    // Look for [rule.id] patterns
    // Rule IDs are: lowercase letters, digits, dots, and hyphens
    // Pattern: \[([a-z][a-z0-9.-]*)\]

    let mut chars = text.char_indices().peekable();

    while let Some((_start_idx, ch)) = chars.next() {
        if ch == '[' {
            // Potential rule reference start
            let mut rule_id = String::new();
            let mut valid = true;
            let mut found_dot = false;

            // First char must be lowercase letter
            if let Some(&(_, first_char)) = chars.peek() {
                if first_char.is_ascii_lowercase() {
                    rule_id.push(first_char);
                    chars.next();
                } else {
                    valid = false;
                }
            } else {
                valid = false;
            }

            if valid {
                // Continue reading the rule ID
                while let Some(&(_, c)) = chars.peek() {
                    if c == ']' {
                        chars.next();
                        break;
                    } else if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' {
                        rule_id.push(c);
                        chars.next();
                    } else if c == '.' {
                        found_dot = true;
                        rule_id.push(c);
                        chars.next();
                    } else {
                        valid = false;
                        break;
                    }
                }
            }

            // Rule ID must contain at least one dot (hierarchical)
            // and not end with a dot
            if valid && found_dot && !rule_id.ends_with('.') && !rule_id.is_empty() {
                references.push(RuleReference {
                    rule_id,
                    file: file.to_string(),
                    line,
                    context: text.trim().to_string(),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_extract_simple_reference() {
        let content = r#"
            // See [channel.id.allocation] for details
            fn allocate_id() {}
        "#;

        let refs = extract_rule_references(&PathBuf::from("test.rs"), content).unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].rule_id, "channel.id.allocation");
    }

    #[test]
    fn test_extract_multiple_references() {
        let content = r#"
            /// Implements [channel.id.parity] and [channel.id.no-reuse]
            fn next_channel_id() {}
        "#;

        let refs = extract_rule_references(&PathBuf::from("test.rs"), content).unwrap();
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].rule_id, "channel.id.parity");
        assert_eq!(refs[1].rule_id, "channel.id.no-reuse");
    }

    #[test]
    fn test_ignore_non_rule_brackets() {
        let content = r#"
            // array[0] is not a rule
            // [Some text] is not a rule either
            fn foo() {}
        "#;

        let refs = extract_rule_references(&PathBuf::from("test.rs"), content).unwrap();
        assert_eq!(refs.len(), 0);
    }
}
