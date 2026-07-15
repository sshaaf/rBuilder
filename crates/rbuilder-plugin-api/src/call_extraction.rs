//! Shared helpers for extracting `Calls` relations from tree-sitter ASTs.

use crate::{Relation, RelationType, SourceLocation, Symbol, SymbolType};
use std::path::Path;
use tree_sitter::Node;

/// Call node kinds per language grammar.
pub const RUST_CALL_KINDS: &[&str] = &["call_expression", "macro_invocation"];
pub const GO_CALL_KINDS: &[&str] = &["call_expression"];
pub const PYTHON_CALL_KINDS: &[&str] = &["call"];
pub const CSHARP_CALL_KINDS: &[&str] = &["invocation_expression"];
pub const C_CALL_KINDS: &[&str] = &["call_expression"];
pub const CPP_CALL_KINDS: &[&str] = &["call_expression"];
pub const JS_CALL_KINDS: &[&str] = &["call_expression"];
pub const TS_CALL_KINDS: &[&str] = &["call_expression"];

/// Find the innermost function symbol containing `node`.
pub fn containing_function<'a>(node: Node, symbols: &'a [Symbol]) -> Option<&'a Symbol> {
    let line = node.start_position().row + 1;
    symbols
        .iter()
        .filter(|s| s.symbol_type == SymbolType::Function)
        .filter(|s| line >= s.location.start_line && line <= s.location.end_line)
        .min_by_key(|s| s.location.end_line - s.location.start_line)
}

/// Best-effort callee name from a call expression subtree (iterative; bounded depth).
pub fn callee_name(root: Node, source: &[u8]) -> Option<String> {
    const MAX_DEPTH: usize = 512;
    let mut stack = vec![(root, 0usize)];

    while let Some((node, depth)) = stack.pop() {
        if depth > MAX_DEPTH {
            continue;
        }

        match node.kind() {
            "identifier" | "type_identifier" => {
                return node.utf8_text(source).ok().map(str::to_string);
            }
            "field_expression" | "selector_expression" | "attribute" => {
                if let Some(n) = node
                    .child_by_field_name("field")
                    .or_else(|| node.child_by_field_name("attribute"))
                    .or_else(|| node.child_by_field_name("name"))
                {
                    stack.push((n, depth + 1));
                }
            }
            "scoped_identifier" | "qualified_type" => {
                if let Some(n) = node.child_by_field_name("name") {
                    stack.push((n, depth + 1));
                }
            }
            "parenthesized_expression" => {
                if let Some(inner) = node.named_child(0) {
                    stack.push((inner, depth + 1));
                }
            }
            "invocation_expression" => {
                if let Some(n) = node.named_child(0) {
                    stack.push((n, depth + 1));
                }
            }
            _ => {
                if let Some(func) = node.child_by_field_name("function") {
                    stack.push((func, depth + 1));
                } else if let Some(name) = node.child_by_field_name("name") {
                    stack.push((name, depth + 1));
                }
            }
        }
    }
    None
}

/// Push a `Calls` relation when `node` is a recognized call site.
pub fn push_call_relation(
    node: Node,
    source: &[u8],
    file_path: &Path,
    symbols: &[Symbol],
    call_kinds: &[&str],
    language: &str,
    relations: &mut Vec<Relation>,
) {
    if !call_kinds.contains(&node.kind()) {
        return;
    }

    let callee = node
        .child_by_field_name("function")
        .or_else(|| node.child_by_field_name("macro"))
        .or_else(|| node.child_by_field_name("name"))
        .and_then(|n| callee_name(n, source))
        .or_else(|| callee_name(node, source));

    let Some(callee) = callee else {
        return;
    };
    if callee.is_empty() {
        return;
    }

    let Some(from_fn) = containing_function(node, symbols) else {
        return;
    };

    let from = from_fn
        .qualified_name
        .clone()
        .unwrap_or_else(|| from_fn.name.clone());

    let local_target = symbols
        .iter()
        .find(|s| s.name == callee && s.symbol_type == SymbolType::Function)
        .and_then(|s| s.qualified_name.clone())
        .unwrap_or_else(|| callee.clone());

    relations.push(Relation {
        from,
        to: local_target,
        relation_type: RelationType::Calls,
        location: SourceLocation {
            file: file_path.to_string_lossy().to_string(),
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
            start_column: node.start_position().column,
            end_column: node.end_position().column,
        },
        metadata: serde_json::json!({ "language": language }),
        to_qualified_hint: None,
        to_type_hint: None,
    });
}

/// Iterative tree walk that records call relations (heap stack; bounded depth).
pub fn walk_calls(
    root: Node,
    source: &[u8],
    file_path: &Path,
    symbols: &[Symbol],
    call_kinds: &[&str],
    language: &str,
    relations: &mut Vec<Relation>,
) {
    const MAX_DEPTH: usize = 2048;
    let mut stack = vec![(root, 0usize)];

    while let Some((node, depth)) = stack.pop() {
        if depth > MAX_DEPTH {
            tracing::warn!(
                file = ?file_path,
                depth = depth,
                "AST depth limit exceeded during walk_calls; skipping deep branches"
            );
            continue;
        }

        push_call_relation(
            node, source, file_path, symbols, call_kinds, language, relations,
        );

        let mut cursor = node.walk();
        let children: Vec<Node> = node.children(&mut cursor).collect();
        for child in children.into_iter().rev() {
            stack.push((child, depth + 1));
        }
    }
}
