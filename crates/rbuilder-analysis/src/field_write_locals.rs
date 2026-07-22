//! Best-effort local/param type recovery for field-write indexing (Layer F5).
//!
//! Bound resolution only: formal parameters and explicitly typed locals inside the
//! target function. No full type inference / reflection.

use std::collections::HashMap;
use tree_sitter::{Node, Parser};

/// Merge typed locals + formals for `function_name` into `env` (name → bare type).
pub fn merge_local_types(
    language: &str,
    source: &str,
    function_name: &str,
    env: &mut HashMap<String, String>,
) {
    match language {
        "java" => merge_with_grammar(
            &tree_sitter_java::LANGUAGE.into(),
            source,
            function_name,
            env,
            visit_java,
        ),
        "csharp" => merge_with_grammar(
            &tree_sitter_c_sharp::LANGUAGE.into(),
            source,
            function_name,
            env,
            visit_csharp,
        ),
        "go" => merge_with_grammar(
            &tree_sitter_go::LANGUAGE.into(),
            source,
            function_name,
            env,
            visit_go,
        ),
        "rust" => merge_with_grammar(
            &tree_sitter_rust::LANGUAGE.into(),
            source,
            function_name,
            env,
            visit_rust,
        ),
        "python" => merge_with_grammar(
            &tree_sitter_python::LANGUAGE.into(),
            source,
            function_name,
            env,
            visit_python,
        ),
        "javascript" => merge_with_grammar(
            &tree_sitter_javascript::LANGUAGE.into(),
            source,
            function_name,
            env,
            visit_javascript,
        ),
        "typescript" => {
            let lang = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
            merge_with_grammar(&lang, source, function_name, env, visit_typescript)
        }
        "c" => merge_with_grammar(
            &tree_sitter_c::LANGUAGE.into(),
            source,
            function_name,
            env,
            visit_c_family,
        ),
        "cpp" => merge_with_grammar(
            &tree_sitter_cpp::LANGUAGE.into(),
            source,
            function_name,
            env,
            visit_c_family,
        ),
        _ => {}
    }
}

fn merge_with_grammar(
    language: &tree_sitter::Language,
    source: &str,
    function_name: &str,
    env: &mut HashMap<String, String>,
    visit: fn(Node, &[u8], &str, &mut HashMap<String, String>, bool),
) {
    let mut parser = Parser::new();
    if parser.set_language(language).is_err() {
        return;
    }
    let Some(tree) = parser.parse(source, None) else {
        return;
    };
    visit(tree.root_node(), source.as_bytes(), function_name, env, false);
}

fn normalize_type_name(name: &str) -> String {
    let bare = name.split('<').next().unwrap_or(name).trim();
    let bare = bare.trim_start_matches('*').trim().trim_start_matches('&');
    bare.rsplit('.')
        .next()
        .unwrap_or(bare)
        .rsplit("::")
        .next()
        .unwrap_or(bare)
        .trim()
        .to_string()
}

fn insert_ty(env: &mut HashMap<String, String>, name: &str, ty: &str) {
    let n = name.trim();
    if n.is_empty() || n == "_" || n == "self" || n == "this" {
        return;
    }
    env.entry(n.to_string())
        .or_insert_with(|| normalize_type_name(ty));
}

fn text_of(node: Node, source: &[u8]) -> Option<String> {
    node.utf8_text(source).ok().map(|s| s.to_string())
}

fn walk_children(
    node: Node,
    source: &[u8],
    function_name: &str,
    env: &mut HashMap<String, String>,
    in_target: bool,
    visit: fn(Node, &[u8], &str, &mut HashMap<String, String>, bool),
    reset_kinds: &[&str],
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if reset_kinds.iter().any(|k| child.kind() == *k) {
            visit(child, source, function_name, env, false);
        } else {
            visit(child, source, function_name, env, in_target);
        }
    }
}

fn visit_java(
    node: Node,
    source: &[u8],
    function_name: &str,
    env: &mut HashMap<String, String>,
    in_target: bool,
) {
    let kind = node.kind();
    let mut now_in = in_target;
    if kind == "method_declaration" || kind == "constructor_declaration" {
        let name = if kind == "constructor_declaration" {
            find_ancestor_name(node, source, "class_declaration").unwrap_or_default()
        } else {
            node.child_by_field_name("name")
                .and_then(|n| text_of(n, source))
                .unwrap_or_default()
        };
        now_in = name == function_name;
    }
    if now_in && kind == "local_variable_declaration" {
        collect_java_style_local(node, source, env);
    }
    if now_in && kind == "formal_parameter" {
        if let (Some(name), Some(ty)) = (
            node.child_by_field_name("name").and_then(|n| text_of(n, source)),
            node.child_by_field_name("type").and_then(|n| text_of(n, source)),
        ) {
            insert_ty(env, &name, &ty);
        }
    }
    walk_children(
        node,
        source,
        function_name,
        env,
        now_in,
        visit_java,
        &["method_declaration", "constructor_declaration"],
    );
}

fn collect_java_style_local(node: Node, source: &[u8], env: &mut HashMap<String, String>) {
    let mut ty = None;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if matches!(
            child.kind(),
            "type_identifier"
                | "generic_type"
                | "integral_type"
                | "floating_point_type"
                | "boolean_type"
                | "array_type"
                | "nullable_type"
                | "predefined_type"
        ) {
            ty = text_of(child, source);
        }
        if child.kind() == "variable_declarator" {
            if let (Some(name), Some(t)) = (
                child
                    .child_by_field_name("name")
                    .and_then(|n| text_of(n, source)),
                ty.as_ref(),
            ) {
                insert_ty(env, &name, t);
            }
        }
    }
}

fn visit_csharp(
    node: Node,
    source: &[u8],
    function_name: &str,
    env: &mut HashMap<String, String>,
    in_target: bool,
) {
    let kind = node.kind();
    let mut now_in = in_target;
    if matches!(
        kind,
        "method_declaration" | "local_function_statement" | "constructor_declaration"
    ) {
        let name = if kind == "constructor_declaration" {
            find_ancestor_name(node, source, "class_declaration")
                .or_else(|| find_ancestor_name(node, source, "struct_declaration"))
                .unwrap_or_default()
        } else {
            node.child_by_field_name("name")
                .and_then(|n| text_of(n, source))
                .unwrap_or_default()
        };
        now_in = name == function_name;
    }
    if now_in && matches!(kind, "local_declaration_statement" | "variable_declaration") {
        // local_declaration_statement wraps variable_declaration
        if kind == "variable_declaration" || kind == "local_declaration_statement" {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "variable_declaration" {
                    collect_csharp_var_decl(child, source, env);
                }
            }
            if kind == "variable_declaration" {
                collect_csharp_var_decl(node, source, env);
            }
        }
    }
    if now_in && kind == "parameter" {
        if let (Some(name), Some(ty)) = (
            node.child_by_field_name("name").and_then(|n| text_of(n, source)),
            node.child_by_field_name("type").and_then(|n| text_of(n, source)),
        ) {
            insert_ty(env, &name, &ty);
        }
    }
    walk_children(
        node,
        source,
        function_name,
        env,
        now_in,
        visit_csharp,
        &[
            "method_declaration",
            "local_function_statement",
            "constructor_declaration",
        ],
    );
}

fn collect_csharp_var_decl(node: Node, source: &[u8], env: &mut HashMap<String, String>) {
    let ty = node
        .child_by_field_name("type")
        .and_then(|n| text_of(n, source));
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "variable_declarator" {
            if let (Some(name), Some(t)) = (
                child
                    .child_by_field_name("name")
                    .and_then(|n| text_of(n, source)),
                ty.as_ref(),
            ) {
                insert_ty(env, &name, t);
            }
        }
    }
}

fn visit_go(
    node: Node,
    source: &[u8],
    function_name: &str,
    env: &mut HashMap<String, String>,
    in_target: bool,
) {
    let kind = node.kind();
    let mut now_in = in_target;
    if matches!(kind, "function_declaration" | "method_declaration") {
        let name = node
            .child_by_field_name("name")
            .and_then(|n| text_of(n, source))
            .unwrap_or_default();
        now_in = name == function_name;
    }
    if now_in && kind == "parameter_declaration" {
        let ty = node
            .child_by_field_name("type")
            .and_then(|n| text_of(n, source));
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                if let (Ok(name), Some(t)) = (child.utf8_text(source), ty.as_ref()) {
                    insert_ty(env, name, t);
                }
            }
        }
    }
    if now_in && kind == "var_spec" {
        let ty = node
            .child_by_field_name("type")
            .and_then(|n| text_of(n, source));
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                if let (Ok(name), Some(t)) = (child.utf8_text(source), ty.as_ref()) {
                    insert_ty(env, name, t);
                }
            }
        }
    }
    walk_children(
        node,
        source,
        function_name,
        env,
        now_in,
        visit_go,
        &["function_declaration", "method_declaration"],
    );
}

fn visit_rust(
    node: Node,
    source: &[u8],
    function_name: &str,
    env: &mut HashMap<String, String>,
    in_target: bool,
) {
    let kind = node.kind();
    let mut now_in = in_target;
    if kind == "function_item" {
        let name = node
            .child_by_field_name("name")
            .and_then(|n| text_of(n, source))
            .unwrap_or_default();
        now_in = name == function_name;
    }
    if now_in && kind == "parameter" {
        let pat = node.child_by_field_name("pattern");
        let ty = node
            .child_by_field_name("type")
            .and_then(|n| text_of(n, source));
        if let (Some(pat), Some(t)) = (pat, ty) {
            if let Ok(name) = pat.utf8_text(source) {
                // Strip ref patterns like `&mut order`
                let name = name
                    .trim()
                    .trim_start_matches("&mut ")
                    .trim_start_matches('&')
                    .trim();
                insert_ty(env, name, &t);
            }
        }
    }
    if now_in && kind == "let_declaration" {
        let ty = node
            .child_by_field_name("type")
            .and_then(|n| text_of(n, source));
        if let (Some(pat), Some(t)) = (node.child_by_field_name("pattern"), ty) {
            if let Ok(name) = pat.utf8_text(source) {
                insert_ty(env, name.trim(), &t);
            }
        }
    }
    walk_children(
        node,
        source,
        function_name,
        env,
        now_in,
        visit_rust,
        &["function_item"],
    );
}

fn visit_python(
    node: Node,
    source: &[u8],
    function_name: &str,
    env: &mut HashMap<String, String>,
    in_target: bool,
) {
    let kind = node.kind();
    let mut now_in = in_target;
    if kind == "function_definition" {
        let name = node
            .child_by_field_name("name")
            .and_then(|n| text_of(n, source))
            .unwrap_or_default();
        now_in = name == function_name;
    }
    if now_in && matches!(kind, "typed_parameter" | "typed_default_parameter") {
        // typed_parameter: identifier + type
        let mut name = None;
        let mut ty = None;
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" if name.is_none() => name = text_of(child, source),
                "type" => ty = text_of(child, source),
                _ => {
                    if child.kind() == "identifier" && name.is_none() {
                        name = text_of(child, source);
                    }
                }
            }
        }
        if let (Some(name), Some(ty)) = (name, ty) {
            insert_ty(env, &name, &ty);
        }
    }
    if now_in && kind == "assignment" {
        // `other: OrderDTO = order` — left may be typed via annotation sibling; skip untyped.
        if let Some(left) = node.child_by_field_name("left") {
            if left.kind() == "identifier" {
                // look for type on annotated assignment: actually python uses `annotated_assignment`?
            }
        }
    }
    if now_in && kind == "annotated_assignment" {
        let name = node
            .child_by_field_name("left") // or name?
            .and_then(|n| text_of(n, source));
        let ty = node
            .child_by_field_name("type")
            .and_then(|n| text_of(n, source));
        if let (Some(name), Some(ty)) = (name, ty) {
            insert_ty(env, &name, &ty);
        }
    }
    walk_children(
        node,
        source,
        function_name,
        env,
        now_in,
        visit_python,
        &["function_definition"],
    );
}

fn visit_javascript(
    node: Node,
    source: &[u8],
    function_name: &str,
    env: &mut HashMap<String, String>,
    in_target: bool,
) {
    // JS has no static types — copy-assign inference only: `const other = order` when order already typed.
    visit_js_ts_like(node, source, function_name, env, in_target, false);
}

fn visit_typescript(
    node: Node,
    source: &[u8],
    function_name: &str,
    env: &mut HashMap<String, String>,
    in_target: bool,
) {
    visit_js_ts_like(node, source, function_name, env, in_target, true);
}

fn visit_js_ts_like(
    node: Node,
    source: &[u8],
    function_name: &str,
    env: &mut HashMap<String, String>,
    in_target: bool,
    with_types: bool,
) {
    let kind = node.kind();
    let mut now_in = in_target;
    if matches!(
        kind,
        "function_declaration" | "method_definition" | "function_expression"
    ) {
        let name = if kind == "method_definition" {
            let raw = node
                .child_by_field_name("name")
                .and_then(|n| text_of(n, source))
                .unwrap_or_default();
            if raw == "constructor" {
                find_ancestor_name(node, source, "class_declaration").unwrap_or(raw)
            } else {
                raw
            }
        } else {
            node.child_by_field_name("name")
                .and_then(|n| text_of(n, source))
                .unwrap_or_default()
        };
        now_in = name == function_name;
    }
    if now_in
        && with_types
        && matches!(
            kind,
            "required_parameter" | "optional_parameter" | "formal_parameter"
        )
    {
        let name = node
            .child_by_field_name("pattern")
            .or_else(|| node.child_by_field_name("name"))
            .and_then(|n| text_of(n, source));
        let ty = node
            .child_by_field_name("type")
            .and_then(|n| text_of(n, source))
            .or_else(|| find_child_kind_text(node, source, "type_annotation"));
        if let (Some(name), Some(ty)) = (name, ty) {
            insert_ty(env, &name, &ty);
        }
    }
    if now_in && kind == "lexical_declaration" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() != "variable_declarator" {
                continue;
            }
            let name = child
                .child_by_field_name("name")
                .and_then(|n| text_of(n, source));
            let ty = if with_types {
                child
                    .child_by_field_name("type")
                    .and_then(|n| text_of(n, source))
                    .or_else(|| find_child_kind_text(child, source, "type_annotation"))
            } else {
                None
            };
            // Copy inference: `const other = order` when order known
            if ty.is_none() {
                if let Some(value) = child.child_by_field_name("value") {
                    if value.kind() == "identifier" {
                        if let (Some(name), Ok(rhs)) = (name.as_ref(), value.utf8_text(source)) {
                            if let Some(t) = env.get(rhs).cloned() {
                                insert_ty(env, name, &t);
                            }
                        }
                    }
                }
            } else if let (Some(name), Some(t)) = (name, ty) {
                insert_ty(env, &name, &t);
            }
        }
    }
    let visit = if with_types {
        visit_typescript
    } else {
        visit_javascript
    };
    walk_children(
        node,
        source,
        function_name,
        env,
        now_in,
        visit,
        &[
            "function_declaration",
            "method_definition",
            "function_expression",
        ],
    );
}

fn visit_c_family(
    node: Node,
    source: &[u8],
    function_name: &str,
    env: &mut HashMap<String, String>,
    in_target: bool,
) {
    let kind = node.kind();
    let mut now_in = in_target;
    if kind == "function_definition" {
        let name = function_declarator_name(node, source).unwrap_or_default();
        now_in = name == function_name;
    }
    if now_in && kind == "parameter_declaration" {
        let ty = node
            .child_by_field_name("type")
            .and_then(|n| text_of(n, source));
        let name = node
            .child_by_field_name("declarator")
            .and_then(|d| declarator_ident(d, source));
        if let (Some(name), Some(ty)) = (name, ty) {
            insert_ty(env, &name, &ty);
        }
    }
    if now_in && kind == "declaration" {
        let ty = node
            .child_by_field_name("type")
            .and_then(|n| text_of(n, source));
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "init_declarator" || child.kind().contains("declarator") {
                if let (Some(name), Some(t)) = (declarator_ident(child, source), ty.as_ref()) {
                    insert_ty(env, &name, t);
                }
            }
        }
    }
    walk_children(
        node,
        source,
        function_name,
        env,
        now_in,
        visit_c_family,
        &["function_definition"],
    );
}

fn function_declarator_name(node: Node, source: &[u8]) -> Option<String> {
    let decl = node.child_by_field_name("declarator")?;
    declarator_ident(decl, source)
}

fn declarator_ident(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" => text_of(node, source),
        "pointer_declarator"
        | "function_declarator"
        | "array_declarator"
        | "parenthesized_declarator"
        | "reference_declarator" => node
            .child_by_field_name("declarator")
            .and_then(|d| declarator_ident(d, source)),
        "init_declarator" => node
            .child_by_field_name("declarator")
            .and_then(|d| declarator_ident(d, source)),
        _ => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(n) = declarator_ident(child, source) {
                    return Some(n);
                }
            }
            None
        }
    }
}

fn first_named_child_text(node: Node, source: &[u8]) -> Option<String> {
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();
    children
        .into_iter()
        .find(|ch| ch.is_named())
        .and_then(|ch| text_of(ch, source))
}

fn find_child_kind_text(node: Node, source: &[u8], kind: &str) -> Option<String> {
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();
    children
        .into_iter()
        .find(|ch| ch.kind() == kind)
        .and_then(|ch| {
            if kind == "type_annotation" {
                first_named_child_text(ch, source)
            } else {
                text_of(ch, source)
            }
        })
}

fn find_ancestor_name(node: Node, source: &[u8], kind: &str) -> Option<String> {
    let mut cur = node;
    while let Some(parent) = cur.parent() {
        if parent.kind() == kind {
            return parent
                .child_by_field_name("name")
                .and_then(|n| text_of(n, source));
        }
        cur = parent;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_locals_merge() {
        let source = r#"
public class OrderProcessor {
    public void process(OrderDTO order) {
        OrderDTO other = order;
        other.status = "X";
    }
}
"#;
        let mut env = HashMap::new();
        merge_local_types("java", source, "process", &mut env);
        assert_eq!(env.get("order").map(String::as_str), Some("OrderDTO"));
        assert_eq!(env.get("other").map(String::as_str), Some("OrderDTO"));
    }

    #[test]
    fn csharp_locals_merge() {
        let source = r#"
class OrderProcessor {
  public void Process(OrderDTO order) {
    OrderDTO other = order;
    other.status = "X";
  }
}
"#;
        let mut env = HashMap::new();
        merge_local_types("csharp", source, "Process", &mut env);
        assert_eq!(env.get("order").map(String::as_str), Some("OrderDTO"));
        assert_eq!(env.get("other").map(String::as_str), Some("OrderDTO"));
    }

    #[test]
    fn go_params_merge() {
        let source = r#"
package demo
func Process(order *OrderDTO) {
  order.Status = "X"
}
"#;
        let mut env = HashMap::new();
        merge_local_types("go", source, "Process", &mut env);
        assert!(
            env.get("order").map(|s| s.contains("OrderDTO")).unwrap_or(false),
            "env={env:?}"
        );
    }
}
