use regex::Regex;

use super::ImportExtractor;
use crate::extractor::relative_to_absolute;
use crate::types::{ExtractedImport, ImportKind};

pub struct PythonExtractor;

impl ImportExtractor for PythonExtractor {
    fn language() -> &'static str {
        "python"
    }

    fn supports(path: &str) -> bool {
        path.ends_with(".py")
    }

    fn extract(source: &str, file_path: &str) -> super::ExtractResult {
        let module_name = crate::module_name_from_path(file_path);
        let mut imports = Vec::new();

        let import_re = Regex::new(r"^import\s+([a-zA-Z_][\w.]*(?:\s*,\s*[a-zA-Z_][\w.]*)*)").unwrap();
        let from_import_re = Regex::new(r"^from\s+(\.[.\w]*|[a-zA-Z_][\w.]*)\s+import\s+(.+)$").unwrap();

        for (i, line) in source.lines().enumerate() {
            let line_num = (i + 1) as u32;
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if let Some(caps) = import_re.captures(trimmed) {
                let modules_str = caps.get(1).unwrap().as_str();
                for mod_name in modules_str.split(',') {
                    let mod_name = mod_name.trim();
                    let resolved = if mod_name.starts_with('.') {
                        relative_to_absolute(mod_name, &module_name)
                    } else {
                        mod_name.to_string()
                    };
                    imports.push(ExtractedImport {
                        module: resolved,
                        line: line_num,
                        kind: ImportKind::Static,
                    });
                }
            } else if let Some(caps) = from_import_re.captures(trimmed) {
                let from_module = caps.get(1).unwrap().as_str();
                let target = caps.get(2).unwrap().as_str();
                let resolved = if from_module.starts_with('.') {
                    relative_to_absolute(from_module, &module_name)
                } else {
                    from_module.to_string()
                };
                imports.push(ExtractedImport {
                    module: resolved.clone(),
                    line: line_num,
                    kind: ImportKind::Static,
                });
                for symbol in target.split(',') {
                    let symbol = symbol.trim().split(" as ").next().unwrap_or("").trim();
                    if !symbol.is_empty() && !symbol.starts_with('*') && symbol.chars().all(|c| c.is_alphanumeric() || c == '_') {
                        imports.push(ExtractedImport {
                            module: format!("{}.{}", resolved, symbol),
                            line: line_num,
                            kind: ImportKind::Static,
                        });
                    }
                }
            }
        }

        super::ExtractResult {
            imports,
            module_name,
            language: "python".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_import() {
        let source = "import os\nimport sys\n";
        let result = PythonExtractor::extract(source, "test.py");
        assert_eq!(result.imports.len(), 2);
        assert_eq!(result.imports[0].module, "os");
        assert_eq!(result.imports[1].module, "sys");
    }

    #[test]
    fn test_multi_import() {
        let source = "import os, sys, json\n";
        let result = PythonExtractor::extract(source, "test.py");
        assert_eq!(result.imports.len(), 3);
        assert_eq!(result.imports[0].module, "os");
        assert_eq!(result.imports[1].module, "sys");
        assert_eq!(result.imports[2].module, "json");
    }

    #[test]
    fn test_from_import() {
        let source = "from django.db import models\n";
        let result = PythonExtractor::extract(source, "test.py");
        assert_eq!(result.imports.len(), 2);
        assert_eq!(result.imports[0].module, "django.db");
        assert_eq!(result.imports[1].module, "django.db.models");
    }

    #[test]
    fn test_multi_from_import() {
        let source = "from os.path import join, exists, split\n";
        let result = PythonExtractor::extract(source, "test.py");
        assert_eq!(result.imports.len(), 4);
        assert_eq!(result.imports[0].module, "os.path");
    }

    #[test]
    fn test_relative_import() {
        let source = "from .models import User\nfrom ..utils import helpers\n";
        let result = PythonExtractor::extract(source, "myapp/views.py");
        assert_eq!(result.imports.len(), 4);
        assert!(result.imports.iter().any(|i| i.module == "myapp.models"));
        assert!(result.imports.iter().any(|i| i.module == "utils"));
    }

    #[test]
    fn test_import_with_alias_ignores_alias() {
        let source = "import numpy as np\n";
        let result = PythonExtractor::extract(source, "test.py");
        assert_eq!(result.imports.len(), 1);
        assert_eq!(result.imports[0].module, "numpy");
    }

    #[test]
    fn test_comments_ignored() {
        let source = "# import os\n# from foo import bar\n";
        let result = PythonExtractor::extract(source, "test.py");
        assert!(result.imports.is_empty());
    }

    #[test]
    fn test_empty_source() {
        let result = PythonExtractor::extract("", "empty.py");
        assert!(result.imports.is_empty());
    }

    #[test]
    fn test_dotted_import() {
        let source = "import django.db.models\n";
        let result = PythonExtractor::extract(source, "test.py");
        assert_eq!(result.imports.len(), 1);
        assert_eq!(result.imports[0].module, "django.db.models");
    }

    #[test]
    fn test_import_star_ignored_as_symbol() {
        let source = "from os import *\n";
        let result = PythonExtractor::extract(source, "test.py");
        assert_eq!(result.imports.len(), 1);
        assert_eq!(result.imports[0].module, "os");
    }

    #[test]
    fn test_import_within_function() {
        let source = "def foo():\n    import math\n    return math.pi\n";
        let result = PythonExtractor::extract(source, "test.py");
        assert_eq!(result.imports.len(), 1);
        assert_eq!(result.imports[0].module, "math");
    }

    #[test]
    fn test_module_name_for_init() {
        let result = PythonExtractor::extract("import os", "myapp/__init__.py");
        assert_eq!(result.module_name, "myapp");
    }

    #[test]
    fn test_line_numbers() {
        let source = "import os\n\nimport sys\n";
        let result = PythonExtractor::extract(source, "test.py");
        assert_eq!(result.imports[0].line, 1);
        assert_eq!(result.imports[1].line, 3);
    }
}
