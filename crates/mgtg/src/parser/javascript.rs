use crate::ir::{IrNode, LoopKind, ResourceKind};
use crate::parser::Parser;
use tree_sitter::{Parser as TsParser, Node};

pub struct JsParser;

impl Parser for JsParser {
    fn parse(source: &str) -> Vec<IrNode> {
        let mut parser = TsParser::new();
        parser
            .set_language(&tree_sitter_javascript::LANGUAGE.into())
            .expect("Failed to load JavaScript grammar");
        let tree = parser.parse(source, None);
        match tree {
            Some(t) => parse_program(t.root_node(), source),
            None => vec![],
        }
    }

    fn language_name() -> &'static str {
        "javascript"
    }
}

fn parse_program(node: Node, source: &str) -> Vec<IrNode> {
    let mut nodes = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        nodes.extend(parse_statement(child, source));
    }
    nodes
}

fn parse_statement(node: Node, source: &str) -> Vec<IrNode> {
    let kind = node.kind();
    match kind {
        "function_declaration" | "function" | "function_expression" => parse_function(node, source),
        "method_definition" => parse_method(node, source),
        "class_declaration" => parse_class(node, source),
        "if_statement" => parse_if(node, source),
        "for_statement" | "for_in_statement" | "for_of_statement" => parse_for(node, source),
        "while_statement" => parse_while(node, source),
        "do_statement" => parse_do_while(node, source),
        "assignment_expression" | "variable_declaration" | "lexical_declaration" => parse_assignment(node, source),
        "expression_statement" => parse_expression_statement(node, source),
        "return_statement" => parse_return(node, source),
        "ternary_expression" => parse_ternary(node, source),
        "arrow_function" => parse_closure(node, source),
        _ => {
            let text = node_text(node, source);
            if !text.trim().is_empty() && !is_syntactic_meta(kind) {
                vec![IrNode::Unresolved {
                    text,
                    line: node.start_position().row + 1,
                }]
            } else {
                vec![]
            }
        }
    }
}

fn is_syntactic_meta(kind: &str) -> bool {
    matches!(
        kind,
        ";" | "{" | "}" | "(" | ")" | "[" | "]" | "," | ":" | "."
    )
}

fn node_text(node: Node, source: &str) -> String {
    let start = node.byte_range().start;
    let end = node.byte_range().end;
    source[start..end].to_string()
}

fn parse_function(node: Node, source: &str) -> Vec<IrNode> {
    let mut name = String::new();
    let mut params = Vec::new();
    let mut body = Vec::new();
    let line = node.start_position().row + 1;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => name = node_text(child, source),
            "formal_parameters" => {
                let mut pc = child.walk();
                for param in child.children(&mut pc) {
                    if param.kind() == "identifier" {
                        params.push(node_text(param, source));
                    }
                }
            }
            "statement_block" => {
                let mut sc = child.walk();
                for stmt in child.children(&mut sc) {
                    body.extend(parse_statement(stmt, source));
                }
            }
            _ => {}
        }
    }

    vec![IrNode::Function {
        name,
        params,
        body,
        line,
    }]
}

fn parse_method(node: Node, source: &str) -> Vec<IrNode> {
    let mut name = String::new();
    let mut params = Vec::new();
    let mut body = Vec::new();
    let line = node.start_position().row + 1;
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "property_identifier" => name = node_text(child, source),
            "formal_parameters" => {
                let mut pc = child.walk();
                for param in child.children(&mut pc) {
                    if param.kind() == "identifier" {
                        params.push(node_text(param, source));
                    }
                }
            }
            "statement_block" => {
                let mut sc = child.walk();
                for stmt in child.children(&mut sc) {
                    body.extend(parse_statement(stmt, source));
                }
            }
            _ => {}
        }
    }

    vec![IrNode::Function {
        name,
        params,
        body,
        line,
    }]
}

fn parse_class(node: Node, source: &str) -> Vec<IrNode> {
    let mut nodes = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "class_body" {
            let mut cc = child.walk();
            for member in child.children(&mut cc) {
                nodes.extend(parse_statement(member, source));
            }
        }
    }
    nodes
}

fn parse_if(node: Node, source: &str) -> Vec<IrNode> {
    let condition = extract_condition(node, source);
    let line = node.start_position().row + 1;
    let mut then_branch = Vec::new();
    let mut else_branch = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "statement_block" | "expression_statement" if then_branch.is_empty() => {
                let mut sc = child.walk();
                for stmt in child.children(&mut sc) {
                    then_branch.extend(parse_statement(stmt, source));
                }
                if then_branch.is_empty() {
                    then_branch.extend(parse_statement(child, source));
                }
            }
            "else_clause" => {
                let mut ec = child.walk();
                for sub in child.children(&mut ec) {
                    if sub.kind() == "statement_block" {
                        let mut sc = sub.walk();
                        for stmt in sub.children(&mut sc) {
                            else_branch.extend(parse_statement(stmt, source));
                        }
                    } else {
                        else_branch.extend(parse_statement(sub, source));
                    }
                }
            }
            _ => {}
        }
    }

    vec![IrNode::Conditional {
        condition,
        then_branch,
        else_branch,
        line,
    }]
}

fn extract_condition(node: Node, source: &str) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "parenthesized_expression" => {
                let text = node_text(child, source);
                // strip parens
                let inner = text.trim().strip_prefix('(').and_then(|s| s.strip_suffix(')')).unwrap_or(&text);
                return inner.trim().to_string();
            }
            _ => {}
        }
    }
    node_text(node, source)
}

fn parse_for(node: Node, source: &str) -> Vec<IrNode> {
    let condition = node_text(node, source);
    let line = node.start_position().row + 1;
    let body = extract_body(node, source);
    vec![IrNode::Loop {
        kind: LoopKind::For,
        condition,
        body,
        line,
    }]
}

fn parse_while(node: Node, source: &str) -> Vec<IrNode> {
    let condition = extract_condition(node, source);
    let line = node.start_position().row + 1;
    let body = extract_body(node, source);
    vec![IrNode::Loop {
        kind: LoopKind::While,
        condition,
        body,
        line,
    }]
}

fn parse_do_while(node: Node, source: &str) -> Vec<IrNode> {
    let line = node.start_position().row + 1;
    let body = extract_body(node, source);
    vec![IrNode::Loop {
        kind: LoopKind::DoWhile,
        condition: node_text(node, source),
        body,
        line,
    }]
}

fn extract_body(node: Node, source: &str) -> Vec<IrNode> {
    let mut body = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "statement_block" {
            let mut sc = child.walk();
            for stmt in child.children(&mut sc) {
                body.extend(parse_statement(stmt, source));
            }
        }
    }
    body
}

fn parse_assignment(node: Node, source: &str) -> Vec<IrNode> {
    let line = node.start_position().row + 1;
    let text = node_text(node, source);
    let mut nodes = Vec::new();

    // Detect resource patterns
    let lower = text.to_lowercase();
    if lower.contains("open(") || lower.contains("fopen") || lower.contains("fs.open") || lower.contains("new socket") || lower.contains("new websocket") {
        if let Some(target) = text.split(&['=', ':'][..]).next() {
            let target = target.trim().to_string();
            nodes.push(IrNode::Alloc {
                target: target.clone(),
                resource: ResourceKind::File,
                line,
            });
        }
    }

    if let Some(eq_pos) = text.find('=') {
        let target = text[..eq_pos].trim().to_string();
        let source = text[eq_pos + 1..].trim().to_string();
        nodes.push(IrNode::Assignment { target, source, line });
    }

    if nodes.is_empty() {
        nodes.push(IrNode::Unresolved { text, line });
    }
    nodes
}

fn parse_expression_statement(node: Node, source: &str) -> Vec<IrNode> {
    let _line = node.start_position().row + 1;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "call_expression" {
            return parse_call(child, source);
        }
        if child.kind() == "assignment_expression" {
            return parse_assignment(child, source);
        }
        // Handle method calls like file.close()
        if child.kind() == "member_expression" {
            return parse_member_call(child, source);
        }
    }
    vec![]
}

fn parse_member_call(node: Node, source: &str) -> Vec<IrNode> {
    let text = node_text(node, source);
    let line = node.start_position().row + 1;
    // e.g. file.close() or socket.disconnect()
    if let Some(dot) = text.find('.') {
        let object = text[..dot].trim().to_string();
        let rest = &text[dot + 1..];
        if let Some(paren) = rest.find('(') {
            let method = rest[..paren].trim().to_string();
            let args_str = if let Some(cp) = rest.find(')') {
                rest[paren + 1..cp].trim().to_string()
            } else {
                String::new()
            };
            let args: Vec<String> = if args_str.is_empty() {
                vec![]
            } else {
                args_str.split(',').map(|s| s.trim().to_string()).collect()
            };
            return vec![IrNode::Call {
                name: format!("{}.{}", object, method),
                args,
                line,
            }];
        }
    }
    vec![IrNode::Unresolved { text, line }]
}

fn parse_call(node: Node, source: &str) -> Vec<IrNode> {
    let line = node.start_position().row + 1;
    let text = node_text(node, source);
    let mut nodes = Vec::new();

    // Extract function name and args
    let name = if let Some(paren) = text.find('(') {
        text[..paren].trim().to_string()
    } else {
        text.clone()
    };

    let args_str = if let Some(paren) = text.find('(') {
        let rest = &text[paren + 1..];
        if let Some(closing) = rest.rfind(')') {
            rest[..closing].to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let args: Vec<String> = if args_str.is_empty() {
        vec![]
    } else {
        args_str.split(',').map(|s| s.trim().to_string()).collect()
    };

    nodes.push(IrNode::Call {
        name: name.clone(),
        args: args.clone(),
        line,
    });

    let lower = name.to_lowercase();
    if lower.contains("open") || lower.contains("connect") || lower.contains("fopen") {
        nodes.insert(
            0,
            IrNode::Alloc {
                target: args.first().cloned().unwrap_or_default(),
                resource: ResourceKind::File,
                line,
            },
        );
    }

    nodes
}

fn parse_return(node: Node, source: &str) -> Vec<IrNode> {
    let line = node.start_position().row + 1;
    let mut cursor = node.walk();
    let mut value = None;
    let mut extra_nodes = Vec::new();
    for child in node.children(&mut cursor) {
        if child.kind() != "return" {
            if child.kind() == "function" || child.kind() == "function_expression" {
                extra_nodes.extend(parse_function(child, source));
                value = Some(node_text(child, source));
            } else if child.kind() == "arrow_function" {
                extra_nodes.extend(parse_closure(child, source));
                value = Some(node_text(child, source));
            } else {
                value = Some(node_text(child, source));
            }
        }
    }
    let mut result = extra_nodes;
    result.push(IrNode::Return { value, line });
    result
}

fn parse_ternary(node: Node, source: &str) -> Vec<IrNode> {
    let line = node.start_position().row + 1;
    let text = node_text(node, source);

    // Extract condition from ternary: cond ? then : else
    let mut parts = text.splitn(3, '?');
    let condition = parts.next().unwrap_or("").trim().to_string();

    vec![IrNode::Conditional {
        condition,
        then_branch: vec![],
        else_branch: vec![],
        line,
    }]
}

fn parse_closure(node: Node, source: &str) -> Vec<IrNode> {
    let line = node.start_position().row + 1;
    let captures = Vec::new();
    let mut body = Vec::new();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "formal_parameters" => {
                // params are local, not captures; actual free-variable analysis is separate
            }
            "statement_block" => {
                let mut sc = child.walk();
                for stmt in child.children(&mut sc) {
                    body.extend(parse_statement(stmt, source));
                }
            }
            _ => {}
        }
    }

    vec![IrNode::Closure {
        captures,
        body,
        line,
    }]
}
