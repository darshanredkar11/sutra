use crate::ir::{IrNode, LoopKind, ResourceKind};
use crate::parser::Parser;
use tree_sitter::{Parser as TsParser, Node};

pub struct PythonParser;

impl Parser for PythonParser {
    fn parse(source: &str) -> Vec<IrNode> {
        let mut parser = TsParser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .expect("Failed to load Python grammar");
        let tree = parser.parse(source, None);
        match tree {
            Some(t) => parse_module(t.root_node(), source),
            None => vec![],
        }
    }

    fn language_name() -> &'static str {
        "python"
    }
}

fn parse_module(node: Node, source: &str) -> Vec<IrNode> {
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
        "function_definition" => parse_function(node, source),
        "class_definition" => parse_class(node, source),
        "if_statement" => parse_if(node, source),
        "for_statement" => parse_for(node, source),
        "while_statement" => parse_while(node, source),
        "assignment" => parse_assignment(node, source),
        "expression_statement" => parse_expression_statement(node, source),
        "return_statement" => parse_return(node, source),
        "try_statement" => parse_try(node, source),
        "with_statement" => parse_with(node, source),
        _ => {
            let text = node_text(node, source);
            if !text.trim().is_empty() {
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

fn node_text(node: Node, source: &str) -> String {
    let start = node.byte_range().start;
    let end = node.byte_range().end;
    source[start..end].to_string()
}

fn parse_function(node: Node, source: &str) -> Vec<IrNode> {
    let mut body = Vec::new();
    let mut name = String::new();
    let mut params = Vec::new();
    let line = node.start_position().row + 1;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => name = node_text(child, source),
            "parameters" => {
                let mut pc = child.walk();
                for param in child.children(&mut pc) {
                    if param.kind() == "identifier" {
                        params.push(node_text(param, source));
                    }
                }
            }
            "block" => {
                let mut bc = child.walk();
                for stmt in child.children(&mut bc) {
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
        if child.kind() == "block" {
            let mut bc = child.walk();
            for stmt in child.children(&mut bc) {
                nodes.extend(parse_statement(stmt, source));
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
            "block" => {
                let mut bc = child.walk();
                for stmt in child.children(&mut bc) {
                    then_branch.extend(parse_statement(stmt, source));
                }
            }
            "elif_clause" | "else_clause" => {
                let mut cc = child.walk();
                for sub in child.children(&mut cc) {
                    if sub.kind() == "block" {
                        let mut bc = sub.walk();
                        for stmt in sub.children(&mut bc) {
                            else_branch.extend(parse_statement(stmt, source));
                        }
                    } else if sub.kind() == "if_statement" {
                        else_branch.extend(parse_if(sub, source));
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
        if child.kind() == "condition" || child.kind() == "binary_operator" || child.kind() == "comparison_operator" || child.kind() == "boolean_operator" || child.kind() == "call" || child.kind() == "identifier"
        {
            let text = node_text(child, source);
            if !text.is_empty() {
                return text;
            }
        }
    }
    // fallback: grab the parenthesized expression after 'if'
    let full = node_text(node, source);
    let idx = full.find("if ");
    if let Some(pos) = idx {
        let rest = &full[pos + 3..];
        if let Some(colon) = rest.find(':') {
            return rest[..colon].trim().to_string();
        }
    }
    String::new()
}

fn parse_for(node: Node, source: &str) -> Vec<IrNode> {
    let condition = extract_loop_condition(node, source);
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
    let condition = extract_loop_condition(node, source);
    let line = node.start_position().row + 1;
    let body = extract_body(node, source);
    vec![IrNode::Loop {
        kind: LoopKind::While,
        condition,
        body,
        line,
    }]
}

fn extract_loop_condition(node: Node, source: &str) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" | "binary_operator" | "comparison_operator" | "call" => {
                return node_text(child, source);
            }
            _ => {}
        }
    }
    node_text(node, source)
}

fn extract_body(node: Node, source: &str) -> Vec<IrNode> {
    let mut body = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "block" {
            let mut bc = child.walk();
            for stmt in child.children(&mut bc) {
                body.extend(parse_statement(stmt, source));
            }
        }
    }
    body
}

fn parse_assignment(node: Node, source: &str) -> Vec<IrNode> {
    let line = node.start_position().row + 1;
    let text = node_text(node, source);

    // Detect resource allocation patterns
    if text.contains("open(") {
        if let Some(target) = text.split('=').next() {
            let target = target.trim().to_string();
            return vec![
                IrNode::Alloc {
                    target: target.clone(),
                    resource: ResourceKind::File,
                    line,
                },
                IrNode::Assignment {
                    target,
                    source: text.split('=').nth(1).unwrap_or("").trim().to_string(),
                    line,
                },
            ];
        }
    }

    let mut parts = text.splitn(2, '=');
    let target = parts.next().unwrap_or("").trim().to_string();
    let source = parts.next().unwrap_or("").trim().to_string();

    vec![IrNode::Assignment {
        target,
        source,
        line,
    }]
}

fn parse_expression_statement(node: Node, source: &str) -> Vec<IrNode> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "call" => return parse_call(child, source),
            "assignment" => return parse_assignment(child, source),
            _ => {}
        }
    }
    // Fallback: extract the full text as a call/assignment
    let text = node_text(node, source);
    if text.contains('(') && text.contains(')') {
        vec![IrNode::Call {
            name: text.split('(').next().unwrap_or("").trim().to_string(),
            args: vec![],
            line: node.start_position().row + 1,
        }]
    } else {
        vec![]
    }
}

fn parse_call(node: Node, source: &str) -> Vec<IrNode> {
    let line = node.start_position().row + 1;
    let text = node_text(node, source);
    let name = text.split('(').next().unwrap_or("").to_string();
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

    let mut nodes = vec![IrNode::Call {
        name: name.clone(),
        args: args.clone(),
        line,
    }];

    // Track resource operations: .close(), .release(), .disconnect()
    let lower = name.to_lowercase();
    if lower.contains("open") || lower.contains("connect") || lower.contains("acquire") {
        if let Some(first_arg) = args.first() {
            nodes.insert(
                0,
                IrNode::Alloc {
                    target: first_arg.clone(),
                    resource: ResourceKind::Unknown(name.clone()),
                    line,
                },
            );
        }
    }

    nodes
}

fn parse_try(node: Node, source: &str) -> Vec<IrNode> {
    let mut nodes = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "block" => {
                let mut bc = child.walk();
                for stmt in child.children(&mut bc) {
                    nodes.extend(parse_statement(stmt, source));
                }
            }
            "finally_clause" | "except_clause" | "else_clause" => {
                // These clauses contain their own inner block
                let mut cc = child.walk();
                for sub in child.children(&mut cc) {
                    if sub.kind() == "block" {
                        let mut bc = sub.walk();
                        for stmt in sub.children(&mut bc) {
                            nodes.extend(parse_statement(stmt, source));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    nodes
}

fn parse_with(node: Node, source: &str) -> Vec<IrNode> {
    let mut nodes = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "block" => {
                let mut bc = child.walk();
                for stmt in child.children(&mut bc) {
                    nodes.extend(parse_statement(stmt, source));
                }
            }
            _ => {}
        }
    }
    nodes
}

fn parse_return(node: Node, source: &str) -> Vec<IrNode> {
    let line = node.start_position().row + 1;
    let mut cursor = node.walk();
    let mut value = None;
    for child in node.children(&mut cursor) {
        if child.kind() != "return" {
            value = Some(node_text(child, source));
        }
    }
    vec![IrNode::Return { value, line }]
}
