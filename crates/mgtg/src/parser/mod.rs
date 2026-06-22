pub mod python;
pub mod javascript;

use crate::ir::IrNode;

pub trait Parser {
    fn parse(source: &str) -> Vec<IrNode>;
    fn language_name() -> &'static str;
}

pub fn detect_language(path: &str) -> Option<&'static str> {
    if path.ends_with(".py") {
        Some("python")
    } else if path.ends_with(".js")
        || path.ends_with(".jsx")
        || path.ends_with(".mjs")
        || path.ends_with(".cjs")
        || path.ends_with(".ts")
        || path.ends_with(".tsx")
    {
        Some("javascript")
    } else {
        None
    }
}

pub fn parse_file(path: &str, source: &str) -> Option<(Vec<IrNode>, &'static str)> {
    let lang = detect_language(path)?;
    let nodes = match lang {
        "python" => python::PythonParser::parse(source),
        "javascript" => javascript::JsParser::parse(source),
        _ => return None,
    };
    Some((nodes, lang))
}
