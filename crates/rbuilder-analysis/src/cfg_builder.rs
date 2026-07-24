//! CFG construction from tree-sitter ASTs.

use crate::cfg::{BasicBlock, BlockId, CfgEdgeType, ControlFlowGraph, Statement, StatementKind};
use crate::def_use::extract_def_use;
use crate::language_profile::{function_kinds_for, parse_source};
use rbuilder_error::{Error, Result};
use rbuilder_plugin_helpers::extract_name_from_node;
use std::collections::{HashMap, HashSet};
use tree_sitter::{Node, Tree};
use uuid::Uuid;

/// Build a CFG for a named function in source text.
pub fn build_cfg_for_function(
    language: &str,
    source: &str,
    function_name: &str,
) -> Result<ControlFlowGraph> {
    let bytes = source.as_bytes();
    let tree = parse_language(language, bytes)?;
    let function_kinds = function_kinds_for(language)?;
    let func_node =
        find_function_by_name(tree.root_node(), bytes, function_name, function_kinds)
            .ok_or_else(|| Error::NotFound(format!("function '{function_name}' not found")))?;
    build_cfg_from_function_node(language, func_node, bytes)
}

/// Parsed source file with function name → byte span index (one tree-sitter parse per file).
pub struct ParsedSourceFile {
    tree: Tree,
    locations: HashMap<String, FunctionLocation>,
}

impl ParsedSourceFile {
    /// Parse and index all functions in a source file.
    pub fn parse(language: &str, source: &[u8]) -> Result<Self> {
        let (tree, locations) = index_function_locations(language, source)?;
        Ok(Self { tree, locations })
    }

    /// Whether `function_name` was found in the indexed file.
    pub fn contains(&self, function_name: &str) -> bool {
        self.locations.contains_key(function_name)
    }

    /// Build CFG for a named function using the cached parse tree.
    pub fn build_cfg(
        &self,
        language: &str,
        source: &[u8],
        function_name: &str,
    ) -> Result<ControlFlowGraph> {
        build_cfg_for_function_in_tree(language, &self.tree, source, function_name)
    }
}

/// Byte span of a function body in a parsed source file.
#[derive(Debug, Clone, Copy)]
pub struct FunctionLocation {
    /// Inclusive start byte offset in the source buffer.
    pub start_byte: usize,
    /// Exclusive end byte offset in the source buffer.
    pub end_byte: usize,
}

/// Build a CFG for a named function in an already-parsed tree (no re-parse).
pub fn build_cfg_for_function_in_tree(
    language: &str,
    tree: &Tree,
    source: &[u8],
    function_name: &str,
) -> Result<ControlFlowGraph> {
    let function_kinds = function_kinds_for(language)?;
    let func_node = find_function_by_name(tree.root_node(), source, function_name, function_kinds)
        .ok_or_else(|| Error::NotFound(format!("function '{function_name}' not found")))?;
    build_cfg_from_function_node(language, func_node, source)
}

/// Parse a source file once and index function names to source byte spans.
pub fn index_function_locations(
    language: &str,
    source: &[u8],
) -> Result<(Tree, HashMap<String, FunctionLocation>)> {
    let tree = parse_language(language, source)?;
    let function_kinds = function_kinds_for(language)?;
    let mut index = HashMap::new();
    collect_function_locations(tree.root_node(), source, function_kinds, &mut index);
    Ok((tree, index))
}

fn collect_function_locations(
    node: Node<'_>,
    source: &[u8],
    function_kinds: &[&str],
    out: &mut HashMap<String, FunctionLocation>,
) {
    if function_kinds.contains(&node.kind()) {
        if let Ok(Some(func_name)) = extract_name_from_node(node, source) {
            out.entry(func_name).or_insert(FunctionLocation {
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
            });
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_function_locations(child, source, function_kinds, out);
    }
}

fn parse_language(language: &str, source: &[u8]) -> Result<Tree> {
    parse_source(language, source)
}

fn find_function_by_name<'a>(
    node: Node<'a>,
    source: &[u8],
    name: &str,
    function_kinds: &[&str],
) -> Option<Node<'a>> {
    if function_kinds.contains(&node.kind()) {
        if let Ok(Some(func_name)) = extract_name_from_node(node, source) {
            if func_name == name {
                return Some(node);
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = find_function_by_name(child, source, name, function_kinds) {
            return Some(found);
        }
    }
    None
}

fn build_cfg_from_function_node(
    language: &str,
    func_node: Node,
    source: &[u8],
) -> Result<ControlFlowGraph> {
    let mut cfg = ControlFlowGraph::new();
    let mut builder = CfgBuilder::new(&mut cfg, language);
    builder.build_function(func_node, source)?;
    Ok(cfg)
}

struct CfgBuilder<'a> {
    cfg: &'a mut ControlFlowGraph,
    current_block: BlockId,
    language: &'a str,
    /// Innermost-first stack of `for` / `switch` / `select` break targets.
    breakable_stack: Vec<BreakableContext>,
    /// False after return/unreachable branch terminates the current path.
    flow_active: bool,
    /// Go `Label:` entry blocks (created eagerly so forward `goto` resolves).
    label_blocks: HashMap<String, BlockId>,
    /// Set when the current switch case ended with `fallthrough`.
    pending_fallthrough: bool,
    /// Label to attach to the next pushed breakable (`Outer: for` / `Outer: switch`).
    pending_breakable_label: Option<String>,
    /// Function-scoped `defer` stack (static approx; LIFO on return/panic).
    defer_stack: Vec<DeferredCall>,
    /// Enclosing `finally` bodies (Java/C#-style), LIFO on return/throw.
    finally_stack: Vec<Vec<DeferredCall>>,
    /// Catch entry blocks for enclosing tries (Exception edges from throw sites).
    try_catch_stack: Vec<Vec<BlockId>>,
    /// When true, leaving a switch case falls into the next case (Java classic switch).
    switch_implicit_fallthrough: bool,
}

struct BreakableContext {
    /// Block after the loop/switch/select.
    exit: BlockId,
    /// `continue` target (update/header). `None` for switch/select.
    continue_target: Option<BlockId>,
    /// Optional label from `Label: for` / `Label: switch`.
    label: Option<String>,
}

#[derive(Clone)]
struct DeferredCall {
    text: String,
    line: usize,
}

impl<'a> CfgBuilder<'a> {
    fn new(cfg: &'a mut ControlFlowGraph, language: &'a str) -> Self {
        let entry = cfg.entry;
        Self {
            cfg,
            current_block: entry,
            language,
            breakable_stack: Vec::new(),
            flow_active: true,
            label_blocks: HashMap::new(),
            pending_fallthrough: false,
            pending_breakable_label: None,
            defer_stack: Vec::new(),
            finally_stack: Vec::new(),
            try_catch_stack: Vec::new(),
            switch_implicit_fallthrough: false,
        }
    }

    fn build_function(&mut self, func_node: Node, source: &[u8]) -> Result<()> {
        let body =
            function_body_node(func_node, self.language).ok_or_else(|| Error::ParseError {
                file: "source".into(),
                line: func_node.start_position().row + 1,
                message: "Function has no body".to_string(),
            })?;
        self.visit_block(body, source)?;
        if self.flow_active {
            if !self.defer_stack.is_empty() {
                self.unwind_defers_to_exit(CfgEdgeType::Return);
            } else if self.cfg.exits.is_empty() {
                let exit = self.new_block();
                self.cfg
                    .add_edge(self.current_block, exit, CfgEdgeType::Next);
                self.cfg.exits.push(exit);
            }
        }
        self.cfg.prune_unreachable_blocks();
        Ok(())
    }

    fn new_block(&mut self) -> BlockId {
        let id = Uuid::new_v4();
        self.cfg.add_block(BasicBlock {
            id,
            statements: Vec::new(),
            start_line: 0,
            end_line: 0,
        });
        id
    }

    fn add_statement(&mut self, node: Node, source: &[u8], kind: StatementKind) -> Result<()> {
        let line = node.start_position().row + 1;
        let text = node.utf8_text(source)?.trim().to_string();
        let (defined_vars, used_vars) = extract_def_use(node, source);
        let stmt = Statement {
            kind,
            line,
            text,
            defined_vars,
            used_vars,
        };
        self.add_statement_to_current(stmt);
        Ok(())
    }

    fn add_statement_to_current(&mut self, stmt: Statement) {
        let kind = stmt.kind;
        let block = self
            .cfg
            .blocks
            .get_mut(&self.current_block)
            .expect("current block");
        if block.start_line == 0 || stmt.line < block.start_line {
            block.start_line = stmt.line;
        }
        if stmt.line > block.end_line {
            block.end_line = stmt.line;
        }
        block.statements.push(stmt);
        // Conservative: any non-terminal stmt in a try may throw → catch.
        if !matches!(kind, StatementKind::Return | StatementKind::Jump) {
            self.wire_try_exception_from_current();
        }
    }

    fn wire_try_exception_from_current(&mut self) {
        let from = self.current_block;
        let Some(handlers) = self.try_catch_stack.last() else {
            return;
        };
        let handlers = handlers.clone();
        for h in handlers {
            let already = self.cfg.edges.iter().any(|e| {
                e.from == from && e.to == h && e.edge_type == CfgEdgeType::Exception
            });
            if !already {
                self.cfg.add_edge(from, h, CfgEdgeType::Exception);
            }
        }
    }

    fn visit_block(&mut self, node: Node, source: &[u8]) -> Result<BlockId> {
        if is_block_like(node.kind()) {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.is_named() {
                    self.visit_statement(child, source)?;
                }
            }
        } else {
            self.visit_statement(node, source)?;
        }
        Ok(self.current_block)
    }

    fn visit_statement(&mut self, node: Node, source: &[u8]) -> Result<()> {
        if !self.flow_active {
            return Ok(());
        }
        match node.kind() {
            // Rust + Python conditionals
            "if_statement" | "if_expression" => self.visit_if(node, source),
            "while_statement" | "while_expression" => self.visit_while(node, source),
            "do_statement" => self.visit_do(node, source),
            "for_statement" | "for_expression" | "for_in_expression" | "foreach_statement"
            | "for_range_loop" => self.visit_for(node, source),
            "enhanced_for_statement" => self.visit_enhanced_for(node, source),
            "loop_expression" => self.visit_loop(node, source),

            // Returns
            "return_statement" | "return_expression" => self.visit_return(node, source),
            "yield_statement" => self.visit_return(node, source),
            "throw_statement" => self.visit_throw(node, source),

            // Jumps
            "break_expression" | "break_statement" => self.visit_break(node, source),
            "continue_expression" | "continue_statement" => self.visit_continue(node, source),
            "goto_statement" => self.visit_goto(node, source),
            "fallthrough_statement" => self.visit_fallthrough(node, source),
            "labeled_statement" | "empty_labeled_statement" => {
                self.visit_labeled_statement(node, source)
            }

            // Declarations / assignments
            "let_declaration" | "let_statement" => {
                self.add_statement(node, source, StatementKind::Declaration)?;
                Ok(())
            }
            "short_var_declaration"
            | "var_declaration"
            | "const_declaration"
            | "variable_declaration"
            | "local_declaration_statement"
            | "local_variable_declaration"
            | "declaration" => {
                self.add_statement(node, source, StatementKind::Declaration)?;
                Ok(())
            }
            "assignment"
            | "assignment_expression"
            | "assignment_statement"
            | "augmented_assignment"
            | "augmented_assignment_expression"
            | "compound_assignment_expr" => {
                self.add_statement(node, source, StatementKind::Assignment)?;
                Ok(())
            }
            "inc_statement" | "dec_statement" | "send_statement" | "update_expression" => {
                self.add_statement(node, source, StatementKind::Expression)?;
                Ok(())
            }

            // Expression statement
            "expression_statement" => self.visit_expression_stmt(node, source),

            // Rust match
            "match_expression" => self.visit_match(node, source),

            // Switch (Go + Java `switch_expression`)
            "switch_statement"
            | "type_switch_statement"
            | "expression_switch_statement"
            | "switch_expression" => self.visit_switch(node, source),
            "select_statement" => self.visit_select(node, source),

            // Go concurrency helpers
            "defer_statement" => self.visit_defer(node, source),
            "go_statement" => {
                self.add_statement(node, source, StatementKind::Expression)?;
                Ok(())
            }

            // try / try-with-resources
            "try_statement" | "try_with_resources_statement" => self.visit_try(node, source),
            "assert_statement" => {
                self.add_statement(node, source, StatementKind::Branch)?;
                Ok(())
            }

            // Block / body wrapper
            k if is_block_like(k) => {
                self.visit_block(node, source)?;
                Ok(())
            }

            // Default: treat as expression
            _ if node.is_named() => {
                let kind = Self::classify_expression(node, source);
                self.add_statement(node, source, kind)?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn visit_expression_stmt(&mut self, node: Node, source: &[u8]) -> Result<()> {
        let inner = node.named_child(0).unwrap_or(node);
        match inner.kind() {
            "if_statement" | "if_expression" | "while_statement" | "while_expression"
            | "for_statement" | "for_expression" | "for_in_expression" | "loop_expression"
            | "match_expression" | "return_statement" | "return_expression" => {
                self.visit_statement(inner, source)
            }
            _ => {
                let kind = Self::classify_expression(inner, source);
                self.add_statement(inner, source, kind)?;
                if is_panic_call(inner, source) {
                    self.unwind_defers_to_exit(CfgEdgeType::Exception);
                }
                Ok(())
            }
        }
    }

    fn classify_expression(node: Node, _source: &[u8]) -> StatementKind {
        match node.kind() {
            "call_expression" | "function_call" | "method_call" | "invocation_expression" => {
                StatementKind::FunctionCall
            }
            "assignment_expression" | "assignment" | "augmented_assignment" => {
                StatementKind::Assignment
            }
            "let_declaration" | "let_statement" => StatementKind::Declaration,
            "return_expression" | "return_statement" => StatementKind::Return,
            "if_expression" | "if_statement" => StatementKind::Branch,
            _ => StatementKind::Expression,
        }
    }

    fn visit_if(&mut self, node: Node, source: &[u8]) -> Result<()> {
        // Go: `if init; cond` — init runs in the current block before the condition.
        if let Some(init) = node.child_by_field_name("initializer") {
            self.visit_statement(init, source)?;
        }

        let cond_block = self.new_block();
        self.cfg
            .add_edge(self.current_block, cond_block, CfgEdgeType::Next);
        self.current_block = cond_block;

        let true_block = self.new_block();
        let false_block = self.new_block();

        if let Some(cond) = node
            .child_by_field_name("condition")
            .or_else(|| node.child_by_field_name("operand"))
        {
            self.wire_condition(cond, source, true_block, false_block)?;
        } else {
            self.add_statement_to_current(Statement {
                kind: StatementKind::Branch,
                line: node.start_position().row + 1,
                text: "if".to_string(),
                defined_vars: HashSet::new(),
                used_vars: HashSet::new(),
            });
            self.cfg
                .add_edge(cond_block, true_block, CfgEdgeType::IfTrue);
            self.cfg
                .add_edge(cond_block, false_block, CfgEdgeType::IfFalse);
        }

        let true_end;
        let true_reaches;
        self.flow_active = true;
        self.current_block = true_block;
        if let Some(consequence) = node
            .child_by_field_name("consequence")
            .or_else(|| node.child_by_field_name("body"))
        {
            self.visit_block(consequence, source)?;
        }
        true_end = self.current_block;
        true_reaches = self.flow_active;

        let mut false_end = false_block;
        let false_reaches;
        self.flow_active = true;
        self.current_block = false_block;
        if let Some(alternative) = node
            .child_by_field_name("alternative")
            .or_else(|| node.child_by_field_name("else"))
        {
            self.visit_block(alternative, source)?;
            false_end = self.current_block;
            false_reaches = self.flow_active;
        } else if let Some(else_clause) = node.child_by_field_name("else_clause") {
            self.visit_block(else_clause, source)?;
            false_end = self.current_block;
            false_reaches = self.flow_active;
        } else {
            false_reaches = true;
        }

        let merge = self.new_block();
        if true_reaches {
            self.cfg.add_edge(true_end, merge, CfgEdgeType::Next);
        }
        if false_reaches {
            self.cfg.add_edge(false_end, merge, CfgEdgeType::Next);
        }
        self.flow_active = true_reaches || false_reaches;
        self.current_block = merge;
        Ok(())
    }

    fn visit_while(&mut self, node: Node, source: &[u8]) -> Result<()> {
        let header = self.new_block();
        self.cfg
            .add_edge(self.current_block, header, CfgEdgeType::Next);
        self.add_statement_to_current(Statement {
            kind: StatementKind::Branch,
            line: node.start_position().row + 1,
            text: node
                .child_by_field_name("condition")
                .and_then(|c| c.utf8_text(source).ok())
                .unwrap_or("while")
                .trim()
                .to_string(),
            defined_vars: HashSet::new(),
            used_vars: HashSet::new(),
        });

        let body = self.new_block();
        self.cfg.add_edge(header, body, CfgEdgeType::IfTrue);
        let exit = self.new_block();
        self.cfg.add_edge(header, exit, CfgEdgeType::IfFalse);

        self.breakable_stack.push(BreakableContext {
            exit,
            continue_target: Some(header),
            label: self.pending_breakable_label.take(),
        });

        self.current_block = body;
        if let Some(body_node) = node.child_by_field_name("body") {
            self.visit_block(body_node, source)?;
        }
        self.cfg
            .add_edge(self.current_block, header, CfgEdgeType::Jump);
        self.breakable_stack.pop();

        self.current_block = exit;
        Ok(())
    }

    fn visit_do(&mut self, node: Node, source: &[u8]) -> Result<()> {
        let body = self.new_block();
        self.cfg
            .add_edge(self.current_block, body, CfgEdgeType::Next);

        let header = self.new_block();
        let exit = self.new_block();

        self.breakable_stack.push(BreakableContext {
            exit,
            continue_target: Some(header),
            label: self.pending_breakable_label.take(),
        });

        self.current_block = body;
        if let Some(body_node) = node.child_by_field_name("body") {
            self.visit_block(body_node, source)?;
        }
        self.cfg
            .add_edge(self.current_block, header, CfgEdgeType::Next);

        self.add_statement_to_current(Statement {
            kind: StatementKind::Branch,
            line: node.start_position().row + 1,
            text: node
                .child_by_field_name("condition")
                .and_then(|c| c.utf8_text(source).ok())
                .unwrap_or("do")
                .trim()
                .to_string(),
            defined_vars: HashSet::new(),
            used_vars: HashSet::new(),
        });
        self.cfg.add_edge(header, body, CfgEdgeType::IfTrue);
        self.cfg.add_edge(header, exit, CfgEdgeType::IfFalse);
        self.breakable_stack.pop();

        self.current_block = exit;
        Ok(())
    }

    fn visit_for(&mut self, node: Node, source: &[u8]) -> Result<()> {
        // Go: for_clause / range_clause. Java/C-like: fields init/condition/update.
        let mut for_clause = None;
        let mut range_clause = None;
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "for_clause" => for_clause = Some(child),
                "range_clause" => range_clause = Some(child),
                _ => {}
            }
        }

        // Init
        if let Some(clause) = for_clause {
            if let Some(init) = clause
                .child_by_field_name("initializer")
                .or_else(|| clause.child_by_field_name("init"))
            {
                self.visit_statement(init, source)?;
            }
        } else if let Some(init) = node
            .child_by_field_name("init")
            .or_else(|| node.child_by_field_name("initializer"))
        {
            self.visit_statement(init, source)?;
        } else if let Some(range) = range_clause {
            self.add_statement(range, source, StatementKind::Expression)?;
        }

        let header = self.new_block();
        self.cfg
            .add_edge(self.current_block, header, CfgEdgeType::Next);
        self.current_block = header;

        let body = self.new_block();
        let exit = self.new_block();

        // Condition — prefer explicit fields; Go while-style bare expr only if no init/update fields.
        let cond_node = for_clause
            .and_then(|c| c.child_by_field_name("condition"))
            .or_else(|| node.child_by_field_name("condition"))
            .or_else(|| {
                // Go `for cond {}` — single expression child, not a declaration/update.
                if for_clause.is_some() || range_clause.is_some() {
                    return None;
                }
                if node.child_by_field_name("init").is_some()
                    || node.child_by_field_name("update").is_some()
                {
                    return None;
                }
                let mut c = node.walk();
                for child in node.children(&mut c) {
                    if child.is_named()
                        && child.kind() != "block"
                        && !is_block_like(child.kind())
                        && !matches!(
                            child.kind(),
                            "local_variable_declaration"
                                | "update_expression"
                                | "assignment_expression"
                        )
                    {
                        return Some(child);
                    }
                }
                None
            });

        if let Some(cond) = cond_node {
            self.wire_condition(cond, source, body, exit)?;
        } else {
            let cond_text = if let Some(range) = range_clause {
                range
                    .utf8_text(source)
                    .ok()
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| "range".to_string())
            } else {
                "true".to_string()
            };
            self.add_statement_to_current(Statement {
                kind: StatementKind::Branch,
                line: node.start_position().row + 1,
                text: cond_text,
                defined_vars: HashSet::new(),
                used_vars: HashSet::new(),
            });
            self.cfg.add_edge(header, body, CfgEdgeType::IfTrue);
            self.cfg.add_edge(header, exit, CfgEdgeType::IfFalse);
        }

        // Updates (Go for_clause.update or Java field "update" — possibly multiple).
        let mut update_nodes: Vec<Node> = Vec::new();
        if let Some(clause) = for_clause {
            if let Some(u) = clause.child_by_field_name("update") {
                update_nodes.push(u);
            }
        } else {
            let mut c = node.walk();
            let mut idx = 0u32;
            for child in node.children(&mut c) {
                if node.field_name_for_child(idx) == Some("update") {
                    update_nodes.push(child);
                }
                idx += 1;
            }
        }

        let update_block = if update_nodes.is_empty() {
            None
        } else {
            Some(self.new_block())
        };
        let continue_target = update_block.unwrap_or(header);
        self.breakable_stack.push(BreakableContext {
            exit,
            continue_target: Some(continue_target),
            label: self.pending_breakable_label.take(),
        });

        self.flow_active = true;
        self.current_block = body;
        if let Some(body_node) = node.child_by_field_name("body") {
            self.visit_block(body_node, source)?;
        }
        if self.flow_active {
            self.cfg
                .add_edge(self.current_block, continue_target, CfgEdgeType::Jump);
        }

        if let Some(update_id) = update_block {
            self.flow_active = true;
            self.current_block = update_id;
            for u in update_nodes {
                self.visit_statement(u, source)?;
            }
            if self.flow_active {
                self.cfg
                    .add_edge(self.current_block, header, CfgEdgeType::Next);
            }
        }

        self.breakable_stack.pop();
        self.flow_active = true;
        self.current_block = exit;
        Ok(())
    }

    fn visit_enhanced_for(&mut self, node: Node, source: &[u8]) -> Result<()> {
        if let Some(value) = node.child_by_field_name("value") {
            self.add_statement(value, source, StatementKind::Expression)?;
        }
        let header = self.new_block();
        self.cfg
            .add_edge(self.current_block, header, CfgEdgeType::Next);
        self.current_block = header;
        let header_text = node
            .child_by_field_name("value")
            .and_then(|v| v.utf8_text(source).ok())
            .map(|s| format!("for-each {s}"))
            .unwrap_or_else(|| "for-each".to_string());
        self.add_statement_to_current(Statement {
            kind: StatementKind::Branch,
            line: node.start_position().row + 1,
            text: header_text,
            defined_vars: HashSet::new(),
            used_vars: HashSet::new(),
        });
        let body = self.new_block();
        let exit = self.new_block();
        self.cfg.add_edge(header, body, CfgEdgeType::IfTrue);
        self.cfg.add_edge(header, exit, CfgEdgeType::IfFalse);
        self.breakable_stack.push(BreakableContext {
            exit,
            continue_target: Some(header),
            label: self.pending_breakable_label.take(),
        });
        self.flow_active = true;
        self.current_block = body;
        if let Some(body_node) = node.child_by_field_name("body") {
            self.visit_block(body_node, source)?;
        }
        if self.flow_active {
            self.cfg
                .add_edge(self.current_block, header, CfgEdgeType::Jump);
        }
        self.breakable_stack.pop();
        self.flow_active = true;
        self.current_block = exit;
        Ok(())
    }

    fn visit_loop(&mut self, node: Node, source: &[u8]) -> Result<()> {
        let header = self.new_block();
        self.cfg
            .add_edge(self.current_block, header, CfgEdgeType::Next);
        let body = self.new_block();
        self.cfg.add_edge(header, body, CfgEdgeType::IfTrue);
        let exit = self.new_block();
        self.cfg.add_edge(header, exit, CfgEdgeType::IfFalse);

        self.breakable_stack.push(BreakableContext {
            exit,
            continue_target: Some(header),
            label: self.pending_breakable_label.take(),
        });

        self.current_block = body;
        if let Some(body_node) = node.child_by_field_name("body") {
            self.visit_block(body_node, source)?;
        }
        self.cfg
            .add_edge(self.current_block, header, CfgEdgeType::Jump);
        self.breakable_stack.pop();

        self.current_block = exit;
        Ok(())
    }

    fn visit_return(&mut self, node: Node, source: &[u8]) -> Result<()> {
        // Java: `return switch (...) { ... };` — lower the switch CFG, then exit.
        if let Some(sw) = {
            let mut found = None;
            let mut c = node.walk();
            for ch in node.children(&mut c) {
                if ch.kind() == "switch_expression" {
                    found = Some(ch);
                    break;
                }
            }
            found
        } {
            self.visit_switch(sw, source)?;
            if !self.flow_active {
                return Ok(());
            }
            self.add_statement_to_current(Statement {
                kind: StatementKind::Return,
                line: node.start_position().row + 1,
                text: "return".to_string(),
                defined_vars: HashSet::new(),
                used_vars: HashSet::new(),
            });
            self.unwind_finallies(source)?;
            self.unwind_defers_to_exit(CfgEdgeType::Return);
            return Ok(());
        }

        self.add_statement(node, source, StatementKind::Return)?;
        self.unwind_finallies(source)?;
        self.unwind_defers_to_exit(CfgEdgeType::Return);
        Ok(())
    }

    fn visit_throw(&mut self, node: Node, source: &[u8]) -> Result<()> {
        self.add_statement(node, source, StatementKind::Jump)?;
        // Inside try: route to catch entries (finally runs when leaving via catch/completion).
        if let Some(handlers) = self.try_catch_stack.last() {
            let handlers = handlers.clone();
            for h in handlers {
                self.cfg
                    .add_edge(self.current_block, h, CfgEdgeType::Exception);
            }
            self.flow_active = false;
            self.current_block = self.new_block();
            return Ok(());
        }
        self.unwind_finallies(source)?;
        self.unwind_defers_to_exit(CfgEdgeType::Exception);
        Ok(())
    }

    /// Emit enclosing `finally` bodies (outermost last) before a terminal exit.
    fn unwind_finallies(&mut self, _source: &[u8]) -> Result<()> {
        let frames: Vec<Vec<DeferredCall>> = self.finally_stack.iter().rev().cloned().collect();
        for stmts in frames {
            for d in stmts {
                let b = self.new_block();
                self.cfg
                    .add_edge(self.current_block, b, CfgEdgeType::Next);
                self.current_block = b;
                self.add_statement_to_current(Statement {
                    kind: StatementKind::Expression,
                    line: d.line,
                    text: d.text,
                    defined_vars: HashSet::new(),
                    used_vars: HashSet::new(),
                });
            }
        }
        Ok(())
    }

    fn visit_defer(&mut self, node: Node, source: &[u8]) -> Result<()> {
        self.add_statement(node, source, StatementKind::Expression)?;
        let deferred = node
            .named_child(0)
            .and_then(|n| n.utf8_text(source).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                node.utf8_text(source)
                    .unwrap_or("defer")
                    .trim()
                    .trim_start_matches("defer")
                    .trim()
                    .to_string()
            });
        self.defer_stack.push(DeferredCall {
            text: deferred,
            line: node.start_position().row + 1,
        });
        Ok(())
    }

    /// Route current block through deferred calls (LIFO) then a terminal exit edge.
    fn unwind_defers_to_exit(&mut self, terminal: CfgEdgeType) {
        let deferred: Vec<_> = self.defer_stack.iter().rev().cloned().collect();
        let mut prev = self.current_block;
        for d in deferred {
            let b = self.new_block();
            self.cfg.add_edge(prev, b, CfgEdgeType::Next);
            self.current_block = b;
            self.add_statement_to_current(Statement {
                kind: StatementKind::FunctionCall,
                line: d.line,
                text: d.text,
                defined_vars: HashSet::new(),
                used_vars: HashSet::new(),
            });
            prev = b;
        }
        let exit = self.new_block();
        self.cfg.add_edge(prev, exit, terminal);
        self.cfg.exits.push(exit);
        self.flow_active = false;
    }

    fn jump_label_of(node: Node, source: &[u8]) -> Option<String> {
        let mut c = node.walk();
        for ch in node.children(&mut c) {
            if ch.kind() == "label_name" {
                return ch.utf8_text(source).ok().map(|s| s.trim().to_string());
            }
        }
        // `break Outer` — second named child may be the label
        if let Some(n) = node.named_child(0) {
            if n.kind() == "label_name" || n.kind() == "identifier" {
                return n.utf8_text(source).ok().map(|s| s.trim().to_string());
            }
        }
        None
    }

    fn find_breakable(&self, label: Option<&str>) -> Option<&BreakableContext> {
        match label {
            Some(l) => self
                .breakable_stack
                .iter()
                .rev()
                .find(|c| c.label.as_deref() == Some(l)),
            None => self.breakable_stack.last(),
        }
    }

    fn visit_break(&mut self, node: Node, source: &[u8]) -> Result<()> {
        self.add_statement(node, source, StatementKind::Jump)?;
        let label = Self::jump_label_of(node, source);
        if let Some(ctx) = self.find_breakable(label.as_deref()) {
            let exit = ctx.exit;
            self.cfg
                .add_edge(self.current_block, exit, CfgEdgeType::Jump);
        }
        self.flow_active = false;
        self.current_block = self.new_block();
        Ok(())
    }

    fn visit_continue(&mut self, node: Node, source: &[u8]) -> Result<()> {
        self.add_statement(node, source, StatementKind::Jump)?;
        let label = Self::jump_label_of(node, source);
        if let Some(ctx) = self.find_breakable(label.as_deref()) {
            if let Some(cont) = ctx.continue_target {
                self.cfg
                    .add_edge(self.current_block, cont, CfgEdgeType::Jump);
            }
        }
        self.flow_active = false;
        self.current_block = self.new_block();
        Ok(())
    }

    fn ensure_label_block(&mut self, name: &str) -> BlockId {
        if let Some(id) = self.label_blocks.get(name) {
            return *id;
        }
        let id = self.new_block();
        self.label_blocks.insert(name.to_string(), id);
        id
    }

    fn visit_goto(&mut self, node: Node, source: &[u8]) -> Result<()> {
        self.add_statement(node, source, StatementKind::Jump)?;
        let label = {
            let mut found = None;
            if let Some(n) = node.named_child(0) {
                found = n.utf8_text(source).ok().map(|s| s.trim().to_string());
            }
            if found.is_none() {
                let mut c = node.walk();
                for ch in node.children(&mut c) {
                    if ch.kind() == "label_name" {
                        found = ch.utf8_text(source).ok().map(|s| s.trim().to_string());
                        break;
                    }
                }
            }
            found.unwrap_or_default()
        };
        if !label.is_empty() {
            let target = self.ensure_label_block(&label);
            self.cfg
                .add_edge(self.current_block, target, CfgEdgeType::Jump);
        }
        self.flow_active = false;
        self.current_block = self.new_block();
        Ok(())
    }

    fn visit_fallthrough(&mut self, node: Node, source: &[u8]) -> Result<()> {
        self.add_statement(node, source, StatementKind::Jump)?;
        self.pending_fallthrough = true;
        self.flow_active = false;
        Ok(())
    }

    fn visit_labeled_statement(&mut self, node: Node, source: &[u8]) -> Result<()> {
        let label = node
            .child_by_field_name("label")
            .or_else(|| {
                // Java: `identifier ':' statement` (no label field).
                node.named_child(0).filter(|n| {
                    matches!(n.kind(), "identifier" | "label_name")
                })
            })
            .and_then(|n| n.utf8_text(source).ok())
            .unwrap_or("")
            .trim()
            .to_string();
        if label.is_empty() {
            return Ok(());
        }
        let label_block = self.ensure_label_block(&label);
        if self.flow_active {
            self.cfg
                .add_edge(self.current_block, label_block, CfgEdgeType::Next);
        }
        self.current_block = label_block;
        self.flow_active = true;

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if !child.is_named() {
                continue;
            }
            if child.kind() == "label_name" {
                continue;
            }
            if node.child_by_field_name("label").is_some_and(|l| l.id() == child.id()) {
                continue;
            }
            // Java label identifier is the first named child — skip it.
            if child.kind() == "identifier" {
                if let Ok(t) = child.utf8_text(source) {
                    if t.trim() == label {
                        continue;
                    }
                }
            }
            let attach_label = matches!(
                child.kind(),
                "for_statement"
                    | "for_expression"
                    | "enhanced_for_statement"
                    | "while_statement"
                    | "while_expression"
                    | "do_statement"
                    | "loop_expression"
                    | "switch_statement"
                    | "type_switch_statement"
                    | "expression_switch_statement"
                    | "switch_expression"
                    | "select_statement"
            );
            if attach_label {
                self.pending_breakable_label = Some(label.clone());
            }
            self.visit_statement(child, source)?;
            if attach_label {
                self.pending_breakable_label = None;
            }
        }
        Ok(())
    }

    /// Lower `&&` / `||` into short-circuit CFG edges ending at `true_dest` / `false_dest`.
    fn wire_condition(
        &mut self,
        cond: Node,
        source: &[u8],
        true_dest: BlockId,
        false_dest: BlockId,
    ) -> Result<()> {
        // Unwrap Java/C-style `(expr)`.
        let cond = {
            let mut c = cond;
            while c.kind() == "parenthesized_expression" {
                if let Some(inner) = c.named_child(0) {
                    c = inner;
                } else {
                    break;
                }
            }
            c
        };

        if let Some(op) = logical_operator(cond, source) {
            let left = cond
                .child_by_field_name("left")
                .ok_or_else(|| Error::ParseError {
                    file: "source".into(),
                    line: cond.start_position().row + 1,
                    message: "logical expression missing left".into(),
                })?;
            let right = cond
                .child_by_field_name("right")
                .ok_or_else(|| Error::ParseError {
                    file: "source".into(),
                    line: cond.start_position().row + 1,
                    message: "logical expression missing right".into(),
                })?;
            let mid = self.new_block();
            if op == "&&" {
                self.wire_condition(left, source, mid, false_dest)?;
                self.flow_active = true;
                self.current_block = mid;
                self.wire_condition(right, source, true_dest, false_dest)?;
            } else {
                self.wire_condition(left, source, true_dest, mid)?;
                self.flow_active = true;
                self.current_block = mid;
                self.wire_condition(right, source, true_dest, false_dest)?;
            }
            return Ok(());
        }

        let text = cond.utf8_text(source)?.trim().to_string();
        if is_constant_false(&text) {
            self.add_statement_to_current(Statement {
                kind: StatementKind::Branch,
                line: cond.start_position().row + 1,
                text,
                defined_vars: HashSet::new(),
                used_vars: HashSet::new(),
            });
            self.cfg
                .add_edge(self.current_block, false_dest, CfgEdgeType::IfFalse);
            return Ok(());
        }
        if is_constant_true(&text) {
            self.add_statement_to_current(Statement {
                kind: StatementKind::Branch,
                line: cond.start_position().row + 1,
                text,
                defined_vars: HashSet::new(),
                used_vars: HashSet::new(),
            });
            self.cfg
                .add_edge(self.current_block, true_dest, CfgEdgeType::IfTrue);
            return Ok(());
        }

        self.add_statement_to_current(Statement {
            kind: StatementKind::Branch,
            line: cond.start_position().row + 1,
            text,
            defined_vars: HashSet::new(),
            used_vars: HashSet::new(),
        });
        let from = self.current_block;
        self.cfg.add_edge(from, true_dest, CfgEdgeType::IfTrue);
        self.cfg.add_edge(from, false_dest, CfgEdgeType::IfFalse);
        Ok(())
    }

    fn visit_match(&mut self, node: Node, source: &[u8]) -> Result<()> {
        self.add_statement(node, source, StatementKind::Branch)?;
        let cond_block = self.current_block;
        let merge = self.new_block();
        let mut arms = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "match_arm" || child.kind() == "match_arm_pattern" {
                arms.push(child);
            }
        }

        if arms.is_empty() {
            if let Some(body) = node.child_by_field_name("body") {
                arms.push(body);
            }
        }

        for arm in arms {
            let arm_block = self.new_block();
            self.cfg
                .add_edge(cond_block, arm_block, CfgEdgeType::IfTrue);
            self.current_block = arm_block;
            self.visit_block(arm, source)?;
            self.cfg
                .add_edge(self.current_block, merge, CfgEdgeType::Next);
        }

        let default_block = self.new_block();
        self.cfg
            .add_edge(cond_block, default_block, CfgEdgeType::IfFalse);
        self.cfg.add_edge(default_block, merge, CfgEdgeType::Next);

        self.current_block = merge;
        Ok(())
    }

    fn visit_switch(&mut self, node: Node, source: &[u8]) -> Result<()> {
        // Go: `switch init; value` — init runs before the branch.
        if let Some(init) = node.child_by_field_name("initializer") {
            self.visit_statement(init, source)?;
        } else {
            // type_switch_statement nests initializer under the header child.
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(init) = child.child_by_field_name("initializer") {
                    self.visit_statement(init, source)?;
                    break;
                }
            }
        }

        let cond_block = self.new_block();
        self.cfg
            .add_edge(self.current_block, cond_block, CfgEdgeType::Next);
        self.current_block = cond_block;

        let branch_text = node
            .child_by_field_name("value")
            .or_else(|| node.child_by_field_name("condition"))
            .or_else(|| node.child_by_field_name("operand"))
            .and_then(|c| c.utf8_text(source).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| node.kind().to_string());
        self.add_statement_to_current(Statement {
            kind: StatementKind::Branch,
            line: node.start_position().row + 1,
            text: branch_text,
            defined_vars: HashSet::new(),
            used_vars: HashSet::new(),
        });

        let merge = self.new_block();
        self.breakable_stack.push(BreakableContext {
            exit: merge,
            continue_target: None,
            label: self.pending_breakable_label.take(),
        });
        let mut cases = Vec::new();
        collect_switch_cases(node, &mut cases);

        if cases.is_empty() {
            if let Some(body) = node.child_by_field_name("body") {
                self.current_block = cond_block;
                self.visit_block(body, source)?;
            }
            self.breakable_stack.pop();
            self.current_block = merge;
            self.flow_active = true;
            return Ok(());
        }

        let mut has_default = false;
        let mut any_reaches_merge = false;
        let mut fallthrough_from: Option<BlockId> = None;
        let prev_implicit = self.switch_implicit_fallthrough;
        for case in &cases {
            // Java classic groups fall through by default; arrow rules / Go cases do not.
            self.switch_implicit_fallthrough = matches!(case.kind(), "switch_block_statement_group");
            let is_default = is_switch_default_case(*case, source);
            if is_default {
                has_default = true;
            }

            let case_block = self.new_block();
            let edge = if is_default {
                CfgEdgeType::IfFalse
            } else {
                CfgEdgeType::IfTrue
            };
            self.cfg.add_edge(cond_block, case_block, edge);
            if let Some(src) = fallthrough_from.take() {
                self.cfg.add_edge(src, case_block, CfgEdgeType::Jump);
            }
            self.flow_active = true;
            self.pending_fallthrough = false;
            self.current_block = case_block;
            self.visit_case_body(*case, source)?;

            let implicit = self.switch_implicit_fallthrough && self.flow_active;
            if self.pending_fallthrough || implicit {
                fallthrough_from = Some(self.current_block);
                self.pending_fallthrough = false;
            } else if self.flow_active {
                self.cfg
                    .add_edge(self.current_block, merge, CfgEdgeType::Next);
                any_reaches_merge = true;
            }
        }
        self.switch_implicit_fallthrough = prev_implicit;

        // Trailing fallthrough with no next case → merge.
        if let Some(src) = fallthrough_from {
            self.cfg.add_edge(src, merge, CfgEdgeType::Next);
            any_reaches_merge = true;
        }

        if !has_default {
            let default_block = self.new_block();
            self.cfg
                .add_edge(cond_block, default_block, CfgEdgeType::IfFalse);
            self.cfg.add_edge(default_block, merge, CfgEdgeType::Next);
            any_reaches_merge = true;
        }

        self.breakable_stack.pop();
        self.flow_active = any_reaches_merge;
        self.current_block = merge;
        Ok(())
    }

    /// Lower switch/select case bodies.
    fn visit_case_body(&mut self, case: Node, source: &[u8]) -> Result<()> {
        if let Some(body) = case.child_by_field_name("body") {
            self.visit_block(body, source)?;
            return Ok(());
        }
        match case.kind() {
            "switch_block_statement_group" => {
                let mut c = case.walk();
                for child in case.children(&mut c) {
                    if !child.is_named() {
                        continue;
                    }
                    // Skip labels (`case 1:` / `default:`).
                    if child.kind() == "switch_label" {
                        continue;
                    }
                    self.visit_statement(child, source)?;
                }
            }
            "switch_rule" => {
                let mut c = case.walk();
                for child in case.children(&mut c) {
                    if !child.is_named() {
                        continue;
                    }
                    if child.kind() == "switch_label" {
                        continue;
                    }
                    self.visit_statement(child, source)?;
                }
            }
            _ => {
                let mut cursor = case.walk();
                for child in case.children(&mut cursor) {
                    if child.kind() == "statement_list" || is_block_like(child.kind()) {
                        self.visit_block(child, source)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn visit_select(&mut self, node: Node, source: &[u8]) -> Result<()> {
        self.visit_switch(node, source)
    }

    fn visit_try(&mut self, node: Node, source: &[u8]) -> Result<()> {
        // Snapshot finally + resource closes before body so return/throw can unwind them.
        let mut finally_snapshot: Option<Vec<DeferredCall>> = None;
        let mut catch_nodes: Vec<Node> = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "finally_clause" => {
                    finally_snapshot = Some(snapshot_block_stmts(child, source));
                }
                "except_clause" | "except_handler" | "catch_clause" => {
                    catch_nodes.push(child);
                }
                _ => {}
            }
        }
        let resource_closes = snapshot_resource_closes(node, source);

        // Push order: user finally first, then resource closes → LIFO runs closes then finally.
        let pushed_finally = finally_snapshot.is_some();
        let pushed_closes = !resource_closes.is_empty();
        if let Some(ref snap) = finally_snapshot {
            self.finally_stack.push(snap.clone());
        }
        if pushed_closes {
            self.finally_stack.push(resource_closes.clone());
        }

        // Pre-create catch entries so body statements can Exception-edge to them.
        let catch_entries: Vec<BlockId> = catch_nodes.iter().map(|_| self.new_block()).collect();
        if !catch_entries.is_empty() {
            self.try_catch_stack.push(catch_entries.clone());
        }

        // Resource declarations run before the try body.
        if let Some(resources) = node.child_by_field_name("resources") {
            let mut c = resources.walk();
            for child in resources.children(&mut c) {
                if child.kind() == "resource" {
                    self.add_statement(child, source, StatementKind::Declaration)?;
                }
            }
        }

        let try_block = self.new_block();
        self.cfg
            .add_edge(self.current_block, try_block, CfgEdgeType::Next);
        self.current_block = try_block;
        // Always link try entry → catch (covers return-only / empty bodies).
        for &h in &catch_entries {
            self.cfg.add_edge(try_block, h, CfgEdgeType::Exception);
        }
        if let Some(body) = node.child_by_field_name("body") {
            self.visit_block(body, source)?;
        }
        if !catch_entries.is_empty() {
            self.try_catch_stack.pop();
        }
        let try_reached_end = self.flow_active;
        let mut try_end = self.current_block;
        let merge = self.new_block();

        for (i, catch_node) in catch_nodes.iter().enumerate() {
            let handler = catch_entries[i];
            self.flow_active = true;
            self.current_block = handler;
            if let Some(block) = catch_node.child_by_field_name("body") {
                self.visit_block(block, source)?;
            } else if catch_node.kind() == "catch_clause" {
                if let Some(block) = find_child_kind(*catch_node, "block") {
                    self.visit_block(block, source)?;
                }
            }
            if self.flow_active {
                self.unwind_finallies(source)?;
                if self.flow_active {
                    self.cfg
                        .add_edge(self.current_block, merge, CfgEdgeType::Next);
                }
            }
        }

        if try_reached_end {
            self.flow_active = true;
            self.current_block = try_end;
            // Same order as unwind_finallies: reverse stack = closes then user finally.
            if pushed_closes {
                for d in &resource_closes {
                    let b = self.new_block();
                    self.cfg
                        .add_edge(self.current_block, b, CfgEdgeType::Next);
                    self.current_block = b;
                    self.add_statement_to_current(Statement {
                        kind: StatementKind::Expression,
                        line: d.line,
                        text: d.text.clone(),
                        defined_vars: HashSet::new(),
                        used_vars: HashSet::new(),
                    });
                }
            }
            if let Some(ref stmts) = finally_snapshot {
                for d in stmts {
                    let b = self.new_block();
                    self.cfg
                        .add_edge(self.current_block, b, CfgEdgeType::Next);
                    self.current_block = b;
                    self.add_statement_to_current(Statement {
                        kind: StatementKind::Expression,
                        line: d.line,
                        text: d.text.clone(),
                        defined_vars: HashSet::new(),
                        used_vars: HashSet::new(),
                    });
                }
            }
            try_end = self.current_block;
            self.cfg.add_edge(try_end, merge, CfgEdgeType::Next);
        }

        if pushed_closes {
            self.finally_stack.pop();
        }
        if pushed_finally {
            self.finally_stack.pop();
        }

        self.flow_active = true;
        self.current_block = merge;
        Ok(())
    }
}

fn snapshot_resource_closes(node: Node, source: &[u8]) -> Vec<DeferredCall> {
    let Some(resources) = node.child_by_field_name("resources") else {
        return Vec::new();
    };
    let mut names: Vec<(String, usize)> = Vec::new();
    let mut c = resources.walk();
    for child in resources.children(&mut c) {
        if child.kind() != "resource" {
            continue;
        }
        let line = child.start_position().row + 1;
        let name = child
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(source).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && s != "_")
            .or_else(|| {
                // `try (alreadyOpen)` / `try (obj.field)` — no `name` field.
                if child.child_by_field_name("type").is_some() {
                    return None;
                }
                child
                    .named_child(0)
                    .and_then(|n| n.utf8_text(source).ok())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            });
        if let Some(name) = name {
            names.push((name, line));
        }
    }
    // Close in reverse declaration order (JLS).
    names
        .into_iter()
        .rev()
        .map(|(name, line)| DeferredCall {
            text: format!("{name}.close()"),
            line,
        })
        .collect()
}

fn snapshot_block_stmts(node: Node, source: &[u8]) -> Vec<DeferredCall> {
    let mut out = Vec::new();
    let block = find_child_kind(node, "block").unwrap_or(node);
    let mut stack = vec![block];
    while let Some(n) = stack.pop() {
        if matches!(
            n.kind(),
            "expression_statement"
                | "method_invocation"
                | "return_statement"
                | "local_variable_declaration"
                | "throw_statement"
        ) {
            if let Ok(t) = n.utf8_text(source) {
                out.push(DeferredCall {
                    text: t.trim().to_string(),
                    line: n.start_position().row + 1,
                });
            }
            continue;
        }
        let mut c = n.walk();
        let children: Vec<_> = n.children(&mut c).filter(|ch| ch.is_named()).collect();
        for ch in children.into_iter().rev() {
            stack.push(ch);
        }
    }
    out
}

fn is_switch_default_case(case: Node, source: &[u8]) -> bool {
    if matches!(
        case.kind(),
        "default_case" | "default_statement" | "switch_default"
    ) {
        return true;
    }
    let mut c = case.walk();
    for child in case.children(&mut c) {
        if child.kind() == "switch_label" {
            if let Ok(t) = child.utf8_text(source) {
                if t.trim().starts_with("default") {
                    return true;
                }
            }
        }
    }
    false
}

fn find_child_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == kind {
            return Some(child);
        }
        if let Some(found) = find_child_kind(child, kind) {
            return Some(found);
        }
    }
    None
}

fn collect_switch_cases<'a>(node: Node<'a>, cases: &mut Vec<Node<'a>>) {
    match node.kind() {
        "expression_case" | "type_case" | "case_clause" | "default_case" | "default_statement"
        | "case_statement" | "communication_case" | "switch_section" | "switch_case"
        | "switch_default" | "switch_block_statement_group" | "switch_rule" => cases.push(node),
        _ => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.is_named() {
                    collect_switch_cases(child, cases);
                }
            }
        }
    }
}

fn is_block_like(kind: &str) -> bool {
    matches!(
        kind,
        "block"
            | "statement_block"
            | "statement_list"
            | "compound_statement"
            | "source_file"
            | "function_body"
            | "block_expression"
    )
}

fn function_body_node<'a>(func_node: Node<'a>, language: &str) -> Option<Node<'a>> {
    if let Some(body) = func_node.child_by_field_name("body") {
        return Some(body);
    }
    match language.to_lowercase().as_str() {
        "rust" | "rs" => func_node.child_by_field_name("block"),
        _ => {
            let mut cursor = func_node.walk();
            for child in func_node.children(&mut cursor) {
                if is_block_like(child.kind()) {
                    return Some(child);
                }
            }
            None
        }
    }
}

fn is_constant_false(text: &str) -> bool {
    matches!(text, "false" | "False" | "0")
}

fn is_constant_true(text: &str) -> bool {
    matches!(text, "true" | "True" | "1")
}

fn is_panic_call(node: Node, source: &[u8]) -> bool {
    let call = if node.kind() == "call_expression" {
        node
    } else {
        return false;
    };
    let func = call
        .child_by_field_name("function")
        .or_else(|| call.named_child(0));
    func.and_then(|f| f.utf8_text(source).ok())
        .is_some_and(|s| s.trim() == "panic")
}

fn logical_operator<'a>(node: Node<'a>, source: &'a [u8]) -> Option<&'a str> {
    if node.kind() != "binary_expression" {
        return None;
    }
    let op = node.child_by_field_name("operator")?;
    let text = op.utf8_text(source).ok()?;
    if text == "&&" || text == "||" {
        Some(text)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::CfgEdgeType;

    #[test]
    fn test_rust_if_else_cfg() {
        let code = r#"
fn example(x: i32) -> i32 {
    if x > 0 {
        return x;
    } else {
        return -x;
    }
}
"#;
        let cfg = build_cfg_for_function("rust", code, "example").unwrap();
        assert!(cfg.blocks.len() >= 4);
        let if_true = cfg
            .edges
            .iter()
            .filter(|e| e.edge_type == CfgEdgeType::IfTrue)
            .count();
        assert!(if_true >= 1);
    }

    #[test]
    fn test_rust_loop_has_cycle() {
        let code = r#"
fn loop_example(n: i32) -> i32 {
    let mut sum = 0;
    for i in 0..n {
        sum += i;
    }
    sum
}
"#;
        let cfg = build_cfg_for_function("rust", code, "loop_example").unwrap();
        assert!(cfg.has_cycle());
    }

    #[test]
    fn test_python_if_cfg() {
        let code = r#"
def example(x):
    if x > 0:
        return x
    else:
        return -x
"#;
        let cfg = build_cfg_for_function("python", code, "example").unwrap();
        assert!(cfg.blocks.len() >= 4);
    }

    #[test]
    fn test_go_method_if_cfg() {
        let code = r#"
package handler

func (h *AuthHandler) Login(c *gin.Context) {
    if c == nil {
        return
    }
    return
}
"#;
        let cfg = build_cfg_for_function("go", code, "Login").unwrap();
        assert!(cfg.blocks.len() >= 4);
        let if_true = cfg
            .edges
            .iter()
            .filter(|e| e.edge_type == CfgEdgeType::IfTrue)
            .count();
        assert!(if_true >= 1);
    }

    #[test]
    fn test_go_for_loop_has_cycle() {
        let code = r#"
package demo

func Sum(n int) int {
    total := 0
    for i := 0; i < n; i++ {
        total += i
    }
    return total
}
"#;
        let cfg = build_cfg_for_function("go", code, "Sum").unwrap();
        assert!(cfg.has_cycle());
    }

    fn go_stmt_texts(cfg: &ControlFlowGraph) -> Vec<String> {
        let mut out = Vec::new();
        for b in cfg.blocks.values() {
            for s in &b.statements {
                out.push(s.text.clone());
            }
        }
        out
    }

    fn go_stmt_kinds(cfg: &ControlFlowGraph) -> Vec<StatementKind> {
        let mut out = Vec::new();
        for b in cfg.blocks.values() {
            for s in &b.statements {
                out.push(s.kind);
            }
        }
        out
    }

    #[test]
    fn test_go_if_initializer_before_condition() {
        let code = r#"
package demo

func IfInit(err error) int {
    if e := err; e != nil {
        return 1
    }
    return 0
}
"#;
        let cfg = build_cfg_for_function("go", code, "IfInit").unwrap();
        let texts = go_stmt_texts(&cfg);
        assert!(
            texts.iter().any(|t| t.contains("e := err") || t.contains("e:=err")),
            "if initializer must be its own CFG statement, got {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("e != nil") || t.contains("e!=nil")),
            "condition should appear as branch text, got {texts:?}"
        );
        assert!(
            !texts.iter().any(|t| t.trim_start().starts_with("if e :=")),
            "whole if_statement should not be one Branch blob, got {texts:?}"
        );
        assert!(
            cfg.edges
                .iter()
                .any(|e| e.edge_type == CfgEdgeType::Return),
            "return in then-branch must create Return edge"
        );
    }

    #[test]
    fn test_go_for_clause_init_cond_update_and_continue() {
        let code = r#"
package demo

func Sum(n int) int {
    total := 0
    for i := 0; i < n; i++ {
        if i == 1 {
            continue
        }
        total += i
    }
    return total
}
"#;
        let cfg = build_cfg_for_function("go", code, "Sum").unwrap();
        let texts = go_stmt_texts(&cfg);
        assert!(
            texts.iter().any(|t| t.contains("i := 0") || t.contains("i:=0")),
            "for init must be visited, got {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("i < n") || t.contains("i<n")),
            "for condition must be header branch, got {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("i++")),
            "for update must appear, got {texts:?}"
        );
        assert!(cfg.has_cycle(), "for must still cycle");

        // continue must target update (or a block that reaches update), not skip update forever.
        let continue_blocks: Vec<_> = cfg
            .blocks
            .values()
            .filter(|b| {
                b.statements
                    .iter()
                    .any(|s| s.kind == StatementKind::Jump && s.text.contains("continue"))
            })
            .collect();
        assert!(!continue_blocks.is_empty(), "expected continue statement");
        let update_blocks: Vec<_> = cfg
            .blocks
            .values()
            .filter(|b| b.statements.iter().any(|s| s.text.contains("i++")))
            .map(|b| b.id)
            .collect();
        assert!(!update_blocks.is_empty());
        let mut reaches_update = false;
        for b in &continue_blocks {
            for e in &cfg.edges {
                if e.from == b.id && update_blocks.contains(&e.to) {
                    reaches_update = true;
                }
            }
        }
        assert!(
            reaches_update,
            "continue must jump to update block containing i++"
        );
    }

    #[test]
    fn test_go_switch_case_bodies_emit_returns() {
        let code = r#"
package demo

func Pick(x int) int {
    switch x {
    case 1:
        return 10
    case 2:
        return 20
    default:
        return 0
    }
}
"#;
        let cfg = build_cfg_for_function("go", code, "Pick").unwrap();
        let kinds = go_stmt_kinds(&cfg);
        assert!(
            kinds.iter().filter(|k| **k == StatementKind::Return).count() >= 3,
            "each case return must be a Return statement, got {kinds:?}"
        );
        let returns = cfg
            .edges
            .iter()
            .filter(|e| e.edge_type == CfgEdgeType::Return)
            .count();
        assert!(
            returns >= 3,
            "expected Return edges from case bodies, got {returns}"
        );
        assert!(
            !go_stmt_texts(&cfg)
                .iter()
                .any(|t| t.trim_start().starts_with("case 1:")),
            "case arms must not remain opaque Expression blobs"
        );
    }

    #[test]
    fn test_go_switch_initializer_visited() {
        let code = r#"
package demo

func Pick(err error) int {
    switch e := err; e {
    case nil:
        return 0
    default:
        return 1
    }
}
"#;
        let cfg = build_cfg_for_function("go", code, "Pick").unwrap();
        let texts = go_stmt_texts(&cfg);
        assert!(
            texts.iter().any(|t| t.contains("e := err") || t.contains("e:=err")),
            "switch initializer must be visited, got {texts:?}"
        );
        assert!(
            go_stmt_kinds(&cfg)
                .iter()
                .any(|k| *k == StatementKind::Return),
            "case body returns must be lowered"
        );
    }

    fn block_ids_containing(cfg: &ControlFlowGraph, needle: &str) -> Vec<uuid::Uuid> {
        cfg.blocks
            .values()
            .filter(|b| b.statements.iter().any(|s| s.text.contains(needle)))
            .map(|b| b.id)
            .collect()
    }

    fn has_edge_from_text_to_text(
        cfg: &ControlFlowGraph,
        from_needle: &str,
        to_needle: &str,
    ) -> bool {
        let froms = block_ids_containing(cfg, from_needle);
        let tos = block_ids_containing(cfg, to_needle);
        cfg.edges.iter().any(|e| {
            froms.contains(&e.from)
                && tos.contains(&e.to)
                && matches!(e.edge_type, CfgEdgeType::Jump | CfgEdgeType::Next)
        })
    }

    #[test]
    fn test_go_fallthrough_edges_to_next_case() {
        let code = r#"
package demo

func Ft(x int) int {
    switch x {
    case 1:
        x = x + 1
        fallthrough
    case 2:
        return x
    default:
        return 0
    }
}
"#;
        let cfg = build_cfg_for_function("go", code, "Ft").unwrap();
        assert!(
            go_stmt_texts(&cfg).iter().any(|t| t.contains("fallthrough")),
            "fallthrough must be a CFG statement"
        );
        assert!(
            has_edge_from_text_to_text(&cfg, "fallthrough", "return x")
                || has_edge_from_text_to_text(&cfg, "x = x + 1", "return x"),
            "fallthrough must connect case 1 body into case 2 body, edges={:?}",
            cfg.edges
                .iter()
                .map(|e| format!("{:?}", e.edge_type))
                .collect::<Vec<_>>()
        );
        // Case 1 must not only merge-exit past case 2 without a jump into it.
        let ft_blocks = block_ids_containing(&cfg, "fallthrough");
        let merge_only = cfg.edges.iter().any(|e| {
            ft_blocks.contains(&e.from) && e.edge_type == CfgEdgeType::Next
        });
        assert!(
            !merge_only || has_edge_from_text_to_text(&cfg, "fallthrough", "return x"),
            "fallthrough should Jump into next case, not only Next-merge"
        );
    }

    #[test]
    fn test_go_goto_and_label() {
        let code = r#"
package demo

func Jump(flag bool) int {
    if flag {
        goto Done
    }
    x := 1
Done:
    return x
}
"#;
        let cfg = build_cfg_for_function("go", code, "Jump").unwrap();
        assert!(
            go_stmt_texts(&cfg).iter().any(|t| t.contains("goto Done")),
            "goto must appear as Jump statement, got {:?}",
            go_stmt_texts(&cfg)
        );
        let froms = block_ids_containing(&cfg, "goto Done");
        let tos = block_ids_containing(&cfg, "return x");
        let jump = cfg.edges.iter().any(|e| {
            froms.contains(&e.from) && tos.contains(&e.to) && e.edge_type == CfgEdgeType::Jump
        });
        assert!(
            jump,
            "goto Done must Jump to the labeled return block, edges={:?}",
            cfg.edges
        );
        assert!(
            go_stmt_kinds(&cfg)
                .iter()
                .any(|k| *k == StatementKind::Return),
            "labeled return must be lowered as Return, not opaque label blob"
        );
    }

    #[test]
    fn test_go_short_circuit_and_or() {
        let code = r#"
package demo

func AndOr(a, b, c bool) int {
    if a && b {
        return 1
    }
    if a || c {
        return 2
    }
    return 0
}
"#;
        let cfg = build_cfg_for_function("go", code, "AndOr").unwrap();
        let texts = go_stmt_texts(&cfg);
        assert!(
            texts.iter().any(|t| t == "a" || t.trim() == "a"),
            "&& left operand should be its own branch, got {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t == "b" || t.trim() == "b"),
            "&& right operand should be its own branch, got {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t == "c" || t.trim() == "c"),
            "|| right operand should be its own branch, got {texts:?}"
        );
        assert!(
            !texts.iter().any(|t| t.contains("a && b")),
            "whole a && b must not remain a single Branch blob, got {texts:?}"
        );
        let if_true = cfg
            .edges
            .iter()
            .filter(|e| e.edge_type == CfgEdgeType::IfTrue)
            .count();
        // a&&b and a||c each need multiple conditional edges
        assert!(
            if_true >= 4,
            "short-circuit should create multiple IfTrue edges, got {if_true}"
        );
    }

    fn can_reach(cfg: &ControlFlowGraph, from: uuid::Uuid, to: uuid::Uuid) -> bool {
        use std::collections::{HashSet, VecDeque};
        let mut seen = HashSet::new();
        let mut q = VecDeque::from([from]);
        while let Some(n) = q.pop_front() {
            if n == to {
                return true;
            }
            if !seen.insert(n) {
                continue;
            }
            for e in &cfg.edges {
                if e.from == n {
                    q.push_back(e.to);
                }
            }
        }
        false
    }

    #[test]
    fn test_go_labeled_break_and_continue() {
        let code = r#"
package demo

func Nested() int {
    n := 0
Outer:
    for i := 0; i < 3; i++ {
        for j := 0; j < 3; j++ {
            if j == 1 {
                continue Outer
            }
            if j == 2 {
                break Outer
            }
            n++
        }
    }
    return n
}
"#;
        let cfg = build_cfg_for_function("go", code, "Nested").unwrap();
        let cont = block_ids_containing(&cfg, "continue Outer");
        let brk = block_ids_containing(&cfg, "break Outer");
        let updates = block_ids_containing(&cfg, "i++");
        assert!(!cont.is_empty() && !brk.is_empty() && !updates.is_empty());

        // continue Outer must reach outer update (i++), not only inner j++.
        let reaches_outer_update = cont.iter().any(|c| {
            updates.iter().any(|u| can_reach(&cfg, *c, *u))
                && cfg.edges.iter().any(|e| {
                    e.from == *c
                        && e.edge_type == CfgEdgeType::Jump
                        && updates.contains(&e.to)
                })
        });
        assert!(
            reaches_outer_update,
            "continue Outer must Jump directly to outer i++ update"
        );

        // break Outer must not Jump to an inner-only exit that still runs outer update.
        // It should Jump to a block from which i++ is NOT reached (loop exit).
        let break_skips_update = brk.iter().any(|b| {
            cfg.edges.iter().any(|e| {
                e.from == *b
                    && e.edge_type == CfgEdgeType::Jump
                    && !updates.iter().any(|u| can_reach(&cfg, e.to, *u) && e.to != *u)
                    && updates.iter().all(|u| e.to != *u)
            })
        });
        assert!(
            break_skips_update,
            "break Outer must Jump to outer exit (not outer update)"
        );
    }

    #[test]
    fn test_go_defer_runs_before_return_exit() {
        let code = r#"
package demo

func cleanup() {}

func WithDefer() {
    defer cleanup()
    return
}
"#;
        let cfg = build_cfg_for_function("go", code, "WithDefer").unwrap();
        let texts = go_stmt_texts(&cfg);
        assert!(
            texts.iter().any(|t| t.contains("defer")),
            "defer must be recorded, got {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("cleanup")),
            "deferred call must appear on unwind path, got {texts:?}"
        );
        let rets = block_ids_containing(&cfg, "return");
        let cleanups: Vec<_> = cfg
            .blocks
            .values()
            .filter(|b| {
                b.statements.iter().any(|s| {
                    s.text.contains("cleanup") && !s.text.contains("defer")
                })
            })
            .map(|b| b.id)
            .collect();
        assert!(!rets.is_empty() && !cleanups.is_empty());
        // From return block, must reach a cleanup invocation before / on the way to a Return edge target.
        let ok = rets.iter().any(|r| {
            cleanups.iter().any(|c| can_reach(&cfg, *r, *c) || {
                // cleanup block may be after return stmt via Next, then Return edge
                cfg.edges.iter().any(|e| e.from == *r && e.to == *c)
            })
        });
        assert!(ok, "return must route into deferred cleanup before exit");
        assert!(
            cfg.edges
                .iter()
                .any(|e| e.edge_type == CfgEdgeType::Return),
            "must still have a Return edge to exit"
        );
    }

    #[test]
    fn test_go_defer_lifo_and_panic_unwind() {
        let code = r#"
package demo

func a() {}
func b() {}

func PanicDefer() {
    defer a()
    defer b()
    panic("boom")
}
"#;
        let cfg = build_cfg_for_function("go", code, "PanicDefer").unwrap();
        let texts = go_stmt_texts(&cfg);
        assert!(
            texts.iter().any(|t| t.contains("panic")),
            "panic must appear, got {texts:?}"
        );
        // LIFO: b before a on unwind — find path panic -> b-call -> a-call
        let panic_blocks = block_ids_containing(&cfg, "panic");
        let b_blocks: Vec<_> = cfg
            .blocks
            .values()
            .filter(|b| {
                b.statements
                    .iter()
                    .any(|s| s.text.contains("b()") && !s.text.contains("defer"))
            })
            .map(|b| b.id)
            .collect();
        let a_blocks: Vec<_> = cfg
            .blocks
            .values()
            .filter(|b| {
                b.statements
                    .iter()
                    .any(|s| s.text.contains("a()") && !s.text.contains("defer"))
            })
            .map(|b| b.id)
            .collect();
        assert!(!panic_blocks.is_empty() && !b_blocks.is_empty() && !a_blocks.is_empty());
        let lifo = panic_blocks.iter().any(|p| {
            b_blocks.iter().any(|b| {
                can_reach(&cfg, *p, *b)
                    && a_blocks
                        .iter()
                        .any(|a| can_reach(&cfg, *b, *a))
            })
        });
        assert!(
            lifo,
            "panic must unwind defers LIFO (b then a), texts={texts:?}"
        );
    }

    #[test]
    fn test_go_switch_cfg() {
        let code = r#"
package demo

func Pick(x int) int {
    switch x {
    case 1:
        return 10
    case 2:
        return 20
    default:
        return 0
    }
}
"#;
        let cfg = build_cfg_for_function("go", code, "Pick").unwrap();
        assert!(cfg.blocks.len() >= 5);
        let branches = cfg
            .edges
            .iter()
            .filter(|e| e.edge_type == CfgEdgeType::IfTrue)
            .count();
        assert!(branches >= 2);
    }

    #[test]
    fn test_go_select_cfg() {
        let code = r#"
package demo

func Wait(ch chan int, done chan struct{}) int {
    select {
    case v := <-ch:
        return v
    default:
        return -1
    }
}
"#;
        let cfg = build_cfg_for_function("go", code, "Wait").unwrap();
        assert!(cfg.blocks.len() >= 4);
    }

    #[test]
    fn test_go_range_for_cfg() {
        let code = r#"
package demo

func Keys(m map[string]int) int {
    n := 0
    for k := range m {
        n += len(k)
    }
    return n
}
"#;
        let cfg = build_cfg_for_function("go", code, "Keys").unwrap();
        assert!(cfg.has_cycle());
    }

    #[test]
    fn test_csharp_if_else_cfg() {
        let code = r#"
public class Demo {
    public int Abs(int x) {
        if (x > 0) {
            return x;
        }
        return -x;
    }
}
"#;
        let cfg = build_cfg_for_function("csharp", code, "Abs").unwrap();
        assert!(cfg.blocks.len() >= 4);
    }

    #[test]
    fn test_csharp_loop_has_cycle() {
        let code = r#"
public class Demo {
    public int Sum(int n) {
        var total = 0;
        for (int i = 0; i < n; i++) {
            total += i;
        }
        return total;
    }
}
"#;
        let cfg = build_cfg_for_function("csharp", code, "Sum").unwrap();
        assert!(cfg.has_cycle());
    }

    #[test]
    fn test_c_if_else_cfg() {
        let code = r#"
int abs_val(int x) {
    if (x > 0) {
        return x;
    }
    return -x;
}
"#;
        let cfg = build_cfg_for_function("c", code, "abs_val").unwrap();
        assert!(cfg.blocks.len() >= 4);
    }

    #[test]
    fn test_c_for_loop_has_cycle() {
        let code = r#"
int sum_n(int n) {
    int total = 0;
    for (int i = 0; i < n; i++) {
        total += i;
    }
    return total;
}
"#;
        let cfg = build_cfg_for_function("c", code, "sum_n").unwrap();
        assert!(cfg.has_cycle());
    }

    #[test]
    fn test_c_switch_cfg() {
        let code = r#"
int classify(int x) {
    switch (x) {
        case 1: return 10;
        case 2: return 20;
        default: return 0;
    }
}
"#;
        let cfg = build_cfg_for_function("c", code, "classify").unwrap();
        assert!(cfg.blocks.len() >= 4);
    }

    #[test]
    fn test_cpp_if_else_cfg() {
        let code = r#"
int abs_val(int x) {
    if (x > 0) {
        return x;
    }
    return -x;
}
"#;
        let cfg = build_cfg_for_function("cpp", code, "abs_val").unwrap();
        assert!(cfg.blocks.len() >= 4);
    }

    #[test]
    fn test_cpp_range_for_has_cycle() {
        let code = r#"
int sum_vec(int* arr, int n) {
    int total = 0;
    for (int i = 0; i < n; i++) {
        total += arr[i];
    }
    return total;
}
"#;
        let cfg = build_cfg_for_function("cpp", code, "sum_vec").unwrap();
        assert!(cfg.has_cycle());
    }

    #[test]
    fn test_javascript_switch_cfg() {
        let code = r#"
function classify(v) {
    switch (v) {
        case 1:
            return "one";
        case 2:
            return "two";
        default:
            return "other";
    }
}
"#;
        let cfg = build_cfg_for_function("javascript", code, "classify").unwrap();
        assert!(cfg.blocks.len() >= 5);
    }

    #[test]
    fn test_unsupported_language() {
        let result = build_cfg_for_function("brainfuck", "+++", "main");
        assert!(matches!(result, Err(Error::UnsupportedLanguage(_))));
    }

    #[test]
    fn test_function_not_found() {
        let code = "fn other() {}";
        let result = build_cfg_for_function("rust", code, "missing");
        assert!(matches!(result, Err(Error::NotFound(_))));
    }

    fn java_texts(cfg: &ControlFlowGraph) -> Vec<String> {
        cfg.blocks
            .values()
            .flat_map(|b| b.statements.iter().map(|s| s.text.clone()))
            .collect()
    }

    fn java_kinds(cfg: &ControlFlowGraph) -> Vec<StatementKind> {
        cfg.blocks
            .values()
            .flat_map(|b| b.statements.iter().map(|s| s.kind))
            .collect()
    }

    fn java_block_ids(cfg: &ControlFlowGraph, needle: &str) -> Vec<uuid::Uuid> {
        cfg.blocks
            .values()
            .filter(|b| b.statements.iter().any(|s| s.text.contains(needle)))
            .map(|b| b.id)
            .collect()
    }

    #[test]
    fn test_java_if_else_cfg() {
        let code = r#"
public class Demo {
    public int abs(int x) {
        if (x > 0) {
            return x;
        }
        return -x;
    }
}
"#;
        let cfg = build_cfg_for_function("java", code, "abs").unwrap();
        assert!(cfg.blocks.len() >= 4);
        assert!(cfg
            .edges
            .iter()
            .any(|e| e.edge_type == CfgEdgeType::IfTrue));
        assert!(
            java_kinds(&cfg)
                .iter()
                .filter(|k| **k == StatementKind::Return)
                .count()
                >= 2
        );
    }

    #[test]
    fn test_java_short_circuit_and() {
        let code = r#"
public class Demo {
    public int sc(boolean a, boolean b) {
        if (a && b) {
            return 1;
        }
        return 0;
    }
}
"#;
        let cfg = build_cfg_for_function("java", code, "sc").unwrap();
        let texts = java_texts(&cfg);
        assert!(
            texts.iter().any(|t| t.trim() == "a" || t == "a"),
            "&& left must be its own branch, got {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.trim() == "b" || t == "b"),
            "&& right must be its own branch, got {texts:?}"
        );
        assert!(
            !texts.iter().any(|t| t.contains("a && b")),
            "must not keep (a && b) as one blob, got {texts:?}"
        );
    }

    #[test]
    fn test_java_for_clause_init_cond_update() {
        let code = r#"
public class Demo {
    public int sum(int n) {
        int total = 0;
        for (int i = 0; i < n; i++) {
            if (i == 1) {
                continue;
            }
            total += i;
        }
        return total;
    }
}
"#;
        let cfg = build_cfg_for_function("java", code, "sum").unwrap();
        let texts = java_texts(&cfg);
        assert!(
            texts.iter().any(|t| t.contains("i = 0") || t.contains("i=0")),
            "for init, got {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("i < n") || t.contains("i<n")),
            "for condition, got {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("i++")),
            "for update, got {texts:?}"
        );
        assert!(cfg.has_cycle());
        let cont = java_block_ids(&cfg, "continue");
        let updates = java_block_ids(&cfg, "i++");
        assert!(
            cont.iter().any(|c| {
                cfg.edges
                    .iter()
                    .any(|e| e.from == *c && updates.contains(&e.to) && e.edge_type == CfgEdgeType::Jump)
            }),
            "continue must Jump to i++"
        );
    }

    #[test]
    fn test_java_enhanced_for_has_cycle() {
        let code = r#"
public class Demo {
    public int sum(int[] a) {
        int t = 0;
        for (int v : a) {
            t += v;
        }
        return t;
    }
}
"#;
        let cfg = build_cfg_for_function("java", code, "sum").unwrap();
        assert!(cfg.has_cycle(), "enhanced for must cycle");
        assert!(
            !java_texts(&cfg)
                .iter()
                .any(|t| t.trim_start().starts_with("for (int v")),
            "enhanced for must not remain opaque blob, got {:?}",
            java_texts(&cfg)
        );
        assert!(
            java_texts(&cfg).iter().any(|t| t.contains("t += v") || t.contains("t+=v")),
            "body must be lowered"
        );
    }

    #[test]
    fn test_java_switch_cases_emit_returns() {
        let code = r#"
public class Demo {
    public int pick(int x) {
        switch (x) {
            case 1:
                return 10;
            case 2:
                return 20;
            default:
                return 0;
        }
    }
}
"#;
        let cfg = build_cfg_for_function("java", code, "pick").unwrap();
        assert!(
            java_kinds(&cfg)
                .iter()
                .filter(|k| **k == StatementKind::Return)
                .count()
                >= 3,
            "case returns must lower, kinds={:?}",
            java_kinds(&cfg)
        );
        assert!(
            !java_texts(&cfg)
                .iter()
                .any(|t| t.trim_start().starts_with("switch")),
            "switch must not be opaque blob"
        );
        let returns = cfg
            .edges
            .iter()
            .filter(|e| e.edge_type == CfgEdgeType::Return)
            .count();
        assert!(returns >= 3, "Return edges from cases, got {returns}");
    }

    #[test]
    fn test_java_switch_arrow_rules() {
        let code = r#"
public class Demo {
    public int pick(int x) {
        return switch (x) {
            case 1 -> 10;
            case 2 -> 20;
            default -> 0;
        };
    }
}
"#;
        let cfg = build_cfg_for_function("java", code, "pick").unwrap();
        // Arrow arms should appear as distinct branch targets (expressions or returns).
        assert!(
            cfg.edges
                .iter()
                .filter(|e| e.edge_type == CfgEdgeType::IfTrue)
                .count()
                >= 2,
            "arrow switch needs arm fan-out"
        );
        assert!(
            !java_texts(&cfg)
                .iter()
                .any(|t| t.contains("case 1 ->") && t.contains("default")),
            "arrow switch must not stay one blob, got {:?}",
            java_texts(&cfg)
        );
    }

    #[test]
    fn test_java_labeled_break_continue() {
        let code = r#"
public class Demo {
    public int nested() {
        int n = 0;
        outer:
        for (int i = 0; i < 3; i++) {
            for (int j = 0; j < 3; j++) {
                if (j == 1) {
                    continue outer;
                }
                if (j == 2) {
                    break outer;
                }
                n++;
            }
        }
        return n;
    }
}
"#;
        let cfg = build_cfg_for_function("java", code, "nested").unwrap();
        let cont = java_block_ids(&cfg, "continue outer");
        let brk = java_block_ids(&cfg, "break outer");
        let updates = java_block_ids(&cfg, "i++");
        assert!(!cont.is_empty() && !brk.is_empty() && !updates.is_empty());
        assert!(
            cont.iter().any(|c| {
                cfg.edges.iter().any(|e| {
                    e.from == *c && updates.contains(&e.to) && e.edge_type == CfgEdgeType::Jump
                })
            }),
            "continue outer must Jump to outer i++"
        );
        assert!(
            brk.iter().any(|b| {
                cfg.edges.iter().any(|e| {
                    e.from == *b
                        && e.edge_type == CfgEdgeType::Jump
                        && updates.iter().all(|u| e.to != *u)
                })
            }),
            "break outer must Jump to outer exit, not i++"
        );
    }

    #[test]
    fn test_java_try_catch_exception_edge() {
        let code = r#"
public class Demo {
    public int twc() {
        try {
            return 1;
        } catch (Exception e) {
            return 0;
        }
    }
}
"#;
        let cfg = build_cfg_for_function("java", code, "twc").unwrap();
        assert!(
            cfg.edges
                .iter()
                .any(|e| e.edge_type == CfgEdgeType::Exception),
            "try/catch needs Exception edge"
        );
        assert!(
            java_kinds(&cfg)
                .iter()
                .filter(|k| **k == StatementKind::Return)
                .count()
                >= 2
        );
    }

    #[test]
    fn test_java_try_with_resources() {
        let code = r#"
public class Demo {
    public int twr() throws Exception {
        try (java.io.StringReader r = new java.io.StringReader("")) {
            return 1;
        } catch (Exception e) {
            return 0;
        }
    }
}
"#;
        let cfg = build_cfg_for_function("java", code, "twr").unwrap();
        assert!(
            !java_texts(&cfg)
                .iter()
                .any(|t| t.trim_start().starts_with("try (")),
            "try-with-resources must not be opaque, got {:?}",
            java_texts(&cfg)
        );
        assert!(
            cfg.edges
                .iter()
                .any(|e| e.edge_type == CfgEdgeType::Exception)
                || java_kinds(&cfg)
                    .iter()
                    .any(|k| *k == StatementKind::Return),
            "twr should lower body/catch"
        );
    }

    #[test]
    fn test_java_try_with_resources_close_on_return() {
        let code = r#"
public class Demo {
    public int twr() throws Exception {
        try (java.io.StringReader r = new java.io.StringReader("");
             java.io.StringReader s = new java.io.StringReader("")) {
            return 1;
        } finally {
            System.out.println("userFinally");
        }
    }
}
"#;
        let cfg = build_cfg_for_function("java", code, "twr").unwrap();
        let texts = java_texts(&cfg);
        assert!(
            texts.iter().any(|t| t.contains("s.close()")),
            "later resource must close first (LIFO), got {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("r.close()")),
            "earlier resource must close, got {texts:?}"
        );
        assert!(
            texts
                .iter()
                .any(|t| t.contains("userFinally") || t.contains("println")),
            "user finally must still run, got {texts:?}"
        );
        let ret = java_block_ids(&cfg, "return 1");
        let close_s = java_block_ids(&cfg, "s.close()");
        assert!(!ret.is_empty() && !close_s.is_empty());
        assert!(
            ret.iter().any(|r| {
                close_s.iter().any(|c| {
                    cfg.edges.iter().any(|e| e.from == *r && e.to == *c)
                        || {
                            use std::collections::{HashSet, VecDeque};
                            let mut seen = HashSet::new();
                            let mut q = VecDeque::from([*r]);
                            while let Some(n) = q.pop_front() {
                                if n == *c {
                                    return true;
                                }
                                if !seen.insert(n) {
                                    continue;
                                }
                                for e in &cfg.edges {
                                    if e.from == n {
                                        q.push_back(e.to);
                                    }
                                }
                            }
                            false
                        }
                })
            }),
            "return must reach resource close before exit"
        );
    }

    #[test]
    fn test_java_try_exception_from_body_statement() {
        let code = r#"
public class Demo {
    public int twc() {
        try {
            System.out.println("mayThrow");
            return 1;
        } catch (Exception e) {
            return 0;
        }
    }
}
"#;
        let cfg = build_cfg_for_function("java", code, "twc").unwrap();
        let throw_sites = java_block_ids(&cfg, "mayThrow");
        let catch_rets = java_block_ids(&cfg, "return 0");
        assert!(!throw_sites.is_empty(), "body call must lower");
        assert!(!catch_rets.is_empty(), "catch return must lower");
        assert!(
            throw_sites.iter().any(|from| {
                cfg.edges.iter().any(|e| {
                    e.from == *from
                        && e.edge_type == CfgEdgeType::Exception
                        && {
                            // Exception target can reach catch return
                            use std::collections::{HashSet, VecDeque};
                            let mut seen = HashSet::new();
                            let mut q = VecDeque::from([e.to]);
                            while let Some(n) = q.pop_front() {
                                if catch_rets.contains(&n) {
                                    return true;
                                }
                                if !seen.insert(n) {
                                    continue;
                                }
                                for edge in &cfg.edges {
                                    if edge.from == n {
                                        q.push_back(edge.to);
                                    }
                                }
                            }
                            false
                        }
                })
            }),
            "Exception edge must leave the body statement block, not only try entry"
        );
    }

    #[test]
    fn test_java_finally_on_return() {
        let code = r#"
public class Demo {
    public int withFinally() {
        try {
            return 1;
        } finally {
            System.out.println("cleanup");
        }
    }
}
"#;
        let cfg = build_cfg_for_function("java", code, "withFinally").unwrap();
        let texts = java_texts(&cfg);
        assert!(
            texts.iter().any(|t| t.contains("cleanup") || t.contains("println")),
            "finally body must run on return path, got {texts:?}"
        );
        let ret = java_block_ids(&cfg, "return 1");
        let cleanup = java_block_ids(&cfg, "cleanup");
        let cleanup2 = java_block_ids(&cfg, "println");
        let cleans: Vec<_> = cleanup.into_iter().chain(cleanup2).collect();
        assert!(!ret.is_empty() && !cleans.is_empty());
        assert!(
            ret.iter().any(|r| {
                cleans.iter().any(|c| {
                    cfg.edges.iter().any(|e| e.from == *r && e.to == *c)
                        || {
                            use std::collections::{HashSet, VecDeque};
                            let mut seen = HashSet::new();
                            let mut q = VecDeque::from([*r]);
                            while let Some(n) = q.pop_front() {
                                if n == *c {
                                    return true;
                                }
                                if !seen.insert(n) {
                                    continue;
                                }
                                for e in &cfg.edges {
                                    if e.from == n {
                                        q.push_back(e.to);
                                    }
                                }
                            }
                            false
                        }
                })
            }),
            "return must reach finally cleanup"
        );
    }

    #[test]
    fn test_java_throw_exits() {
        let code = r#"
public class Demo {
    public void boom() {
        throw new RuntimeException("x");
    }
}
"#;
        let cfg = build_cfg_for_function("java", code, "boom").unwrap();
        assert!(
            java_texts(&cfg).iter().any(|t| t.contains("throw")),
            "throw must appear"
        );
        assert!(
            cfg.edges.iter().any(|e| {
                matches!(
                    e.edge_type,
                    CfgEdgeType::Exception | CfgEdgeType::Return | CfgEdgeType::Jump
                )
            }),
            "throw must terminate with Exception/Return/Jump edge"
        );
    }
}

