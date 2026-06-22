use regex::Regex;

use super::ImportExtractor;
use crate::types::{ExtractedImport, ImportKind};

pub struct GoExtractor;

impl ImportExtractor for GoExtractor {
    fn language() -> &'static str {
        "golang"
    }

    fn supports(path: &str) -> bool {
        path.ends_with(".go")
    }

    fn extract(source: &str, file_path: &str) -> super::ExtractResult {
        let module_name = crate::module_name_from_path(file_path);
        let mut imports = Vec::new();

        let single_import = Regex::new(r#"^\s*import\s+("(?:[^"\\]|\\.)+")"#).unwrap();
        let named_import = Regex::new(r#"^\s*import\s+(\w+)\s+("(?:[^"\\]|\\.)+")"#).unwrap();
        let import_line =
            Regex::new(r#"^\s*("(?:[^"\\]|\\.)+")"#).unwrap();
        let named_import_line =
            Regex::new(r#"^\s*(\w+)\s+("(?:[^"\\]|\\.)+")"#).unwrap();

        let mut in_group = false;
        let mut group_text = String::new();

        for line in source.lines() {
            let trimmed = line;

            if trimmed.trim().is_empty() {
                continue;
            }

            if !in_group {
                if let Some(caps) = named_import.captures(trimmed) {
                    let raw = caps.get(2).unwrap().as_str();
                    let path = &raw[1..raw.len() - 1];
                    imports.push(ExtractedImport {
                        module: path.to_string(),
                        line: 0,
                        kind: ImportKind::Static,
                    });
                    continue;
                }
                if let Some(caps) = single_import.captures(trimmed) {
                    let raw = caps.get(1).unwrap().as_str();
                    let path = &raw[1..raw.len() - 1];
                    imports.push(ExtractedImport {
                        module: path.to_string(),
                        line: 0,
                        kind: ImportKind::Static,
                    });
                    continue;
                }
                if trimmed.trim() == "import (" {
                    in_group = true;
                    group_text.clear();
                    continue;
                }
            } else {
                if trimmed.trim() == ")" {
                    in_group = false;
                    for group_line in group_text.lines() {
                        let gl = group_line.trim();
                        if gl.is_empty() {
                            continue;
                        }
                        if let Some(caps) = named_import_line.captures(gl) {
                            let raw = caps.get(2).unwrap().as_str();
                            let path = &raw[1..raw.len() - 1];
                            imports.push(ExtractedImport {
                                module: path.to_string(),
                                line: 0,
                                kind: ImportKind::Static,
                            });
                        } else if let Some(caps) = import_line.captures(gl) {
                            let raw = caps.get(1).unwrap().as_str();
                            let path = &raw[1..raw.len() - 1];
                            imports.push(ExtractedImport {
                                module: path.to_string(),
                                line: 0,
                                kind: ImportKind::Static,
                            });
                        }
                    }
                    continue;
                }
                group_text.push_str(trimmed);
                group_text.push('\n');
            }
        }

        super::ExtractResult {
            imports,
            module_name,
            language: "golang".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_import() {
        let source = r#"import "fmt""#;
        let result = GoExtractor::extract(source, "main.go");
        assert_eq!(result.imports.len(), 1);
        assert_eq!(result.imports[0].module, "fmt");
    }

    #[test]
    fn test_named_import() {
        let source = r#"import math "math""#;
        let result = GoExtractor::extract(source, "main.go");
        assert_eq!(result.imports.len(), 1);
        assert_eq!(result.imports[0].module, "math");
    }

    #[test]
    fn test_grouped_imports() {
        let source = r#"import (
    "fmt"
    "os"
    "net/http"
)"#;
        let result = GoExtractor::extract(source, "main.go");
        assert_eq!(result.imports.len(), 3);
        assert_eq!(result.imports[0].module, "fmt");
        assert_eq!(result.imports[1].module, "os");
        assert_eq!(result.imports[2].module, "net/http");
    }

    #[test]
    fn test_grouped_named_imports() {
        let source = r#"import (
    myfmt "fmt"
    "os"
)"#;
        let result = GoExtractor::extract(source, "main.go");
        assert_eq!(result.imports.len(), 2);
        assert_eq!(result.imports[0].module, "fmt");
        assert_eq!(result.imports[1].module, "os");
    }

    #[test]
    fn test_mixed_imports() {
        let source = r#"import "fmt"
import "os"
import (
    "strings"
    "strconv"
)"#;
        let result = GoExtractor::extract(source, "main.go");
        assert_eq!(result.imports.len(), 4);
    }

    #[test]
    fn test_empty_source() {
        let result = GoExtractor::extract("", "empty.go");
        assert!(result.imports.is_empty());
    }

    #[test]
    fn test_no_imports() {
        let source = r#"package main

func main() {}
"#;
        let result = GoExtractor::extract(source, "main.go");
        assert!(result.imports.is_empty());
    }

    #[test]
    fn test_import_with_dot() {
        let source = r#"import "github.com/gin-gonic/gin""#;
        let result = GoExtractor::extract(source, "main.go");
        assert!(result.imports[0].module.contains("gin"));
    }

    #[test]
    fn test_grouped_with_comments() {
        let source = r#"import (
    "fmt"
    // "os"
    "net/http"
)"#;
        let result = GoExtractor::extract(source, "main.go");
        assert_eq!(result.imports.len(), 2);
        assert_eq!(result.imports[0].module, "fmt");
        assert_eq!(result.imports[1].module, "net/http");
    }
}
