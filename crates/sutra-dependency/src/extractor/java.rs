use regex::Regex;

use super::ImportExtractor;
use crate::types::{ExtractedImport, ImportKind};

pub struct JavaExtractor;

impl ImportExtractor for JavaExtractor {
    fn language() -> &'static str {
        "java"
    }

    fn supports(path: &str) -> bool {
        path.ends_with(".java")
    }

    fn extract(source: &str, file_path: &str) -> super::ExtractResult {
        let module_name = crate::module_name_from_path(file_path);
        let mut imports = Vec::new();

        let import_re = Regex::new(r"^import\s+(?:static\s+)?([a-zA-Z_][\w.]*(?:\.[\w*]+)?)\s*;").unwrap();
        let package_re = Regex::new(r"^package\s+([a-zA-Z_][\w.]*)\s*;").unwrap();

        let mut package = String::new();
        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*") {
                continue;
            }
            if let Some(caps) = package_re.captures(trimmed) {
                package = caps.get(1).unwrap().as_str().to_string();
            }
            if let Some(caps) = import_re.captures(trimmed) {
                let raw = caps.get(1).unwrap().as_str();
                if raw.ends_with(".*") {
                    let base = raw.trim_end_matches(".*");
                    imports.push(ExtractedImport {
                        module: base.to_string(),
                        line: 0,
                        kind: ImportKind::Static,
                    });
                } else {
                    imports.push(ExtractedImport {
                        module: raw.to_string(),
                        line: 0,
                        kind: ImportKind::Static,
                    });
                }
            }
        }

        let effective_module = if package.is_empty() {
            module_name.clone()
        } else {
            format!("{}.{}", package, module_name)
        };

        super::ExtractResult {
            imports,
            module_name: effective_module,
            language: "java".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_import() {
        let source = r#"
import java.util.List;

public class Test {}
"#;
        let result = JavaExtractor::extract(source, "Test.java");
        assert_eq!(result.imports.len(), 1);
        assert_eq!(result.imports[0].module, "java.util.List");
    }

    #[test]
    fn test_multiple_imports() {
        let source = r#"
import java.util.List;
import java.util.ArrayList;
import java.io.File;
"#;
        let result = JavaExtractor::extract(source, "Test.java");
        assert_eq!(result.imports.len(), 3);
    }

    #[test]
    fn test_wildcard_import() {
        let source = r#"
import java.util.*;
"#;
        let result = JavaExtractor::extract(source, "Test.java");
        assert_eq!(result.imports.len(), 1);
        assert_eq!(result.imports[0].module, "java.util");
    }

    #[test]
    fn test_package_detection() {
        let source = r#"
package com.example.app;

import com.example.app.service.UserService;
"#;
        let result = JavaExtractor::extract(source, "Test.java");
        assert!(result.module_name.contains("com.example.app"));
    }

    #[test]
    fn test_comments_ignored() {
        let source = r#"
// import java.util.List;
/* import java.io.File; */
"#;
        let result = JavaExtractor::extract(source, "Test.java");
        assert!(result.imports.is_empty());
    }

    #[test]
    fn test_empty_source() {
        let result = JavaExtractor::extract("", "Empty.java");
        assert!(result.imports.is_empty());
    }

    #[test]
    fn test_static_import() {
        let source = r#"
import static java.lang.Math.PI;
"#;
        let result = JavaExtractor::extract(source, "Test.java");
        assert_eq!(result.imports.len(), 1);
        assert_eq!(result.imports[0].module, "java.lang.Math.PI");
    }

    #[test]
    fn test_no_imports() {
        let source = r#"
public class Simple {
    public void run() {}
}
"#;
        let result = JavaExtractor::extract(source, "Simple.java");
        assert!(result.imports.is_empty());
    }
}
