use regex::Regex;

use super::ImportExtractor;
use crate::types::{ExtractedImport, ImportKind};

fn resolve_node_module(import_path: &str, current_file: &str) -> String {
    if import_path.starts_with('.') || import_path.starts_with('/') {
        let base = std::path::Path::new(current_file)
            .parent()
            .unwrap_or(std::path::Path::new(""));
        let mut resolved = base.join(import_path);
        resolved = if resolved.extension().is_none() {
            resolved.with_extension("")
        } else {
            resolved
        };
        let s = resolved.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/");
        let parts: Vec<&str> = s.split('/').collect();
        let mut cleaned: Vec<&str> = Vec::new();
        for part in parts {
            match part {
                "." => {}
                ".." => { cleaned.pop(); }
                p => { cleaned.push(p); }
            }
        }
        cleaned.join("/")
    } else {
        import_path.to_string()
    }
}

pub struct JsExtractor;

impl ImportExtractor for JsExtractor {
    fn language() -> &'static str {
        "javascript"
    }

    fn supports(path: &str) -> bool {
        path.ends_with(".js")
            || path.ends_with(".jsx")
            || path.ends_with(".mjs")
            || path.ends_with(".cjs")
            || path.ends_with(".ts")
            || path.ends_with(".tsx")
    }

    fn extract(source: &str, file_path: &str) -> super::ExtractResult {
        let module_name = crate::module_name_from_path(file_path);
        let mut imports = Vec::new();

        let es_import = Regex::new(
            r#"import\s+(?:\{[^}]*\}\s*from\s+|\*\s*as\s+\w+\s+from\s+|\w+\s+from\s+)?["']([^"']+)["']"#,
        )
        .unwrap();
        let es_dynamic = Regex::new(r#"import\s*\(\s*["']([^"']+)["']\s*\)"#).unwrap();
        let require_re =
            Regex::new(r#"(?:const|let|var)\s+\w+\s*=\s*require\s*\(\s*["']([^"']+)["']\s*\)"#)
                .unwrap();
        let require_direct = Regex::new(r#"require\s*\(\s*["']([^"']+)["']\s*\)"#).unwrap();
        let export_from =
            Regex::new(r#"export\s+\{[^}]*\}\s*from\s+["']([^"']+)["']"#).unwrap();

        for (i, line) in source.lines().enumerate() {
            let line_num = (i + 1) as u32;
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") {
                continue;
            }

            if let Some(caps) = es_import.captures(trimmed) {
                let raw = caps.get(1).unwrap().as_str();
                imports.push(ExtractedImport {
                    module: resolve_node_module(raw, file_path),
                    line: line_num,
                    kind: ImportKind::Static,
                });
            } else if let Some(caps) = es_dynamic.captures(trimmed) {
                let raw = caps.get(1).unwrap().as_str();
                imports.push(ExtractedImport {
                    module: resolve_node_module(raw, file_path),
                    line: line_num,
                    kind: ImportKind::Dynamic,
                });
            } else if let Some(caps) = require_re.captures(trimmed) {
                let raw = caps.get(1).unwrap().as_str();
                imports.push(ExtractedImport {
                    module: resolve_node_module(raw, file_path),
                    line: line_num,
                    kind: ImportKind::Static,
                });
            } else if let Some(caps) = require_direct.captures(trimmed) {
                let raw = caps.get(1).unwrap().as_str();
                imports.push(ExtractedImport {
                    module: resolve_node_module(raw, file_path),
                    line: line_num,
                    kind: ImportKind::Static,
                });
            } else if let Some(caps) = export_from.captures(trimmed) {
                let raw = caps.get(1).unwrap().as_str();
                imports.push(ExtractedImport {
                    module: resolve_node_module(raw, file_path),
                    line: line_num,
                    kind: ImportKind::ReExport,
                });
            }
        }

        super::ExtractResult {
            imports,
            module_name,
            language: "javascript".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_es_import_default() {
        let source = r#"import express from "express";"#;
        let result = JsExtractor::extract(source, "test.js");
        assert_eq!(result.imports.len(), 1);
        assert_eq!(result.imports[0].module, "express");
        assert_eq!(result.imports[0].kind, ImportKind::Static);
    }

    #[test]
    fn test_es_import_named() {
        let source = r#"import { useState, useEffect } from "react";"#;
        let result = JsExtractor::extract(source, "test.js");
        assert_eq!(result.imports.len(), 1);
        assert_eq!(result.imports[0].module, "react");
    }

    #[test]
    fn test_es_import_namespace() {
        let source = r#"import * as React from "react";"#;
        let result = JsExtractor::extract(source, "test.js");
        assert_eq!(result.imports.len(), 1);
        assert_eq!(result.imports[0].module, "react");
    }

    #[test]
    fn test_es_import_relative() {
        let source = r#"import { helper } from "./utils/helper";"#;
        let result = JsExtractor::extract(source, "src/app.js");
        assert_eq!(result.imports.len(), 1);
        assert!(result.imports[0].module.contains("src/utils/helper"));
    }

    #[test]
    fn test_require() {
        let source = r#"const fs = require("fs");"#;
        let result = JsExtractor::extract(source, "test.js");
        assert_eq!(result.imports.len(), 1);
        assert_eq!(result.imports[0].module, "fs");
    }

    #[test]
    fn test_dynamic_import() {
        let source = r#"const mod = await import("./lazy.js");"#;
        let result = JsExtractor::extract(source, "test.js");
        assert_eq!(result.imports.len(), 1);
        assert_eq!(result.imports[0].kind, ImportKind::Dynamic);
    }

    #[test]
    fn test_export_from() {
        let source = r#"export { foo, bar } from "./module";"#;
        let result = JsExtractor::extract(source, "test.js");
        assert_eq!(result.imports.len(), 1);
        assert_eq!(result.imports[0].kind, ImportKind::ReExport);
    }

    #[test]
    fn test_require_direct_no_var() {
        let source = r#"require("./setup");"#;
        let result = JsExtractor::extract(source, "test.js");
        assert_eq!(result.imports.len(), 1);
    }

    #[test]
    fn test_comments_ignored() {
        let source = r#"// import fs from "fs";"#;
        let result = JsExtractor::extract(source, "test.js");
        assert!(result.imports.is_empty());
    }

    #[test]
    fn test_empty_source() {
        let result = JsExtractor::extract("", "empty.js");
        assert!(result.imports.is_empty());
    }

    #[test]
    fn test_tsx_support() {
        let source = r#"import React from "react";"#;
        let result = JsExtractor::extract(source, "component.tsx");
        assert!(!result.imports.is_empty());
        assert_eq!(result.language, "javascript");
    }
}
