mod python;
mod javascript;
mod java;
mod golang;

use crate::types::ExtractedImport;

pub struct ExtractResult {
    pub imports: Vec<ExtractedImport>,
    pub module_name: String,
    pub language: String,
}

pub trait ImportExtractor {
    fn language() -> &'static str;
    fn extract(source: &str, file_path: &str) -> ExtractResult;
    fn supports(path: &str) -> bool;
}

pub fn detect_language(path: &str) -> Option<&'static str> {
    let path = path.to_lowercase();
    if path.ends_with(".py") {
        Some("python")
    } else if path.ends_with(".js")
        || path.ends_with(".jsx")
        || path.ends_with(".mjs")
        || path.ends_with(".cjs")
    {
        Some("javascript")
    } else if path.ends_with(".ts") || path.ends_with(".tsx") {
        Some("javascript")
    } else if path.ends_with(".java") {
        Some("java")
    } else if path.ends_with(".go") {
        Some("golang")
    } else {
        None
    }
}

pub fn extract_imports(source: &str, path: &str) -> Option<ExtractResult> {
    let lang = detect_language(path)?;
    let result = match lang {
        "python" => python::PythonExtractor::extract(source, path),
        "javascript" => javascript::JsExtractor::extract(source, path),
        "java" => java::JavaExtractor::extract(source, path),
        "golang" => golang::GoExtractor::extract(source, path),
        _ => return None,
    };
    Some(result)
}

pub fn relative_to_absolute(relative: &str, current_module: &str) -> String {
    if !relative.starts_with('.') {
        return relative.to_string();
    }
    let depth = relative.chars().take_while(|c| *c == '.').count();
    let remainder = relative.trim_start_matches('.');
    let remainder = if remainder.starts_with('/') || remainder.is_empty() {
        ""
    } else if remainder.starts_with('.') {
        ""
    } else {
        remainder
    };

    let mut parts: Vec<&str> = current_module.split('.').collect();
    for _ in 0..depth {
        parts.pop();
    }
    if !remainder.is_empty() {
        parts.push(remainder);
    }
    parts.join(".")
}

pub fn module_name_from_path(file_path: &str) -> String {
    let path = std::path::Path::new(file_path);
    let stem = path.file_stem().map(|s| s.to_string_lossy()).unwrap_or_default().to_string();
    if stem == "__init__" {
        let parent = path.parent().map(|p| p.to_string_lossy()).unwrap_or_default();
        let parent = parent.replace(std::path::MAIN_SEPARATOR, ".");
        if parent.is_empty() || parent == "." {
            return String::new();
        }
        return parent;
    }
    let without_ext = path.with_extension("");
    let parts: Vec<&str> = without_ext.components().filter_map(|c| {
        let s = c.as_os_str().to_str()?;
        if s == "." || s == ".." || s == "src" || s == "lib" {
            return None;
        }
        Some(s)
    }).collect();
    if parts.is_empty() {
        return stem.to_string();
    }
    parts.join(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_language_py() {
        assert_eq!(detect_language("foo.py"), Some("python"));
        assert_eq!(detect_language("path/to/script.py"), Some("python"));
    }

    #[test]
    fn test_detect_language_js() {
        assert_eq!(detect_language("file.js"), Some("javascript"));
        assert_eq!(detect_language("file.jsx"), Some("javascript"));
        assert_eq!(detect_language("file.mjs"), Some("javascript"));
        assert_eq!(detect_language("file.cjs"), Some("javascript"));
        assert_eq!(detect_language("file.ts"), Some("javascript"));
        assert_eq!(detect_language("file.tsx"), Some("javascript"));
    }

    #[test]
    fn test_detect_language_java() {
        assert_eq!(detect_language("Main.java"), Some("java"));
    }

    #[test]
    fn test_detect_language_go() {
        assert_eq!(detect_language("main.go"), Some("golang"));
    }

    #[test]
    fn test_detect_language_unsupported() {
        assert_eq!(detect_language("file.rs"), None);
        assert_eq!(detect_language("file.c"), None);
        assert_eq!(detect_language("file.rb"), None);
    }

    #[test]
    fn test_relative_to_absolute_no_dot() {
        assert_eq!(relative_to_absolute("os.path", "myapp.utils"), "os.path");
    }

    #[test]
    fn test_relative_to_absolute_single_dot() {
        assert_eq!(relative_to_absolute(".utils.helpers", "myapp.models"), "myapp.utils.helpers");
    }

    #[test]
    fn test_relative_to_absolute_double_dot() {
        assert_eq!(relative_to_absolute("..models.user", "myapp.services.auth"), "myapp.models.user");
    }

    #[test]
    fn test_relative_to_absolute_triple_dot() {
        assert_eq!(relative_to_absolute("...core.base", "a.b.c.d"), "a.core.base");
    }

    #[test]
    fn test_relative_to_absolute_trailing_dots() {
        assert_eq!(relative_to_absolute("..", "myapp.utils.files"), "myapp");
    }

    #[test]
    fn test_module_name_from_path_simple() {
        assert!(module_name_from_path("src/main.py").contains("main"));
    }

    #[test]
    fn test_module_name_from_path_init() {
        let name = module_name_from_path("myapp/__init__.py");
        assert_eq!(name, "myapp");
    }

    #[test]
    fn test_module_name_from_path_nested() {
        let name = module_name_from_path("src/services/user_service.py");
        assert!(name.contains("user_service"));
        assert!(name.contains("services"));
    }

    #[test]
    fn test_empty_source_no_imports() {
        let result = extract_imports("", "empty.py");
        assert!(result.is_some());
        assert!(result.unwrap().imports.is_empty());
    }

    #[test]
    fn test_source_with_only_comments() {
        let source = "# this is a comment\n# another comment\n# import os\n";
        let result = extract_imports(source, "comments.py");
        assert!(result.is_some());
        assert!(result.unwrap().imports.is_empty());
    }

    #[test]
    fn test_malformed_imports_handled_gracefully() {
        let source = "impot os\nfrom import\nimport\nfrom x import\n";
        let result = extract_imports(source, "malformed.py");
        assert!(result.is_some());
        assert!(result.unwrap().imports.is_empty());
    }

    #[test]
    fn test_very_long_import_path() {
        let long_module = "a".repeat(500);
        let source = format!("import {}", long_module);
        let result = extract_imports(&source, "long_import.py");
        assert!(result.is_some());
        let imports = result.unwrap().imports;
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].module.len(), 500);
    }

    #[test]
    fn test_unicode_import_paths() {
        let source = "const mod = require(\"some_路径/file\");\n";
        let result = extract_imports(source, "unicode.js");
        assert!(result.is_some());
        let imports = result.unwrap().imports;
        assert!(!imports.is_empty());
        assert!(imports[0].module.contains("路径"));
    }
}
