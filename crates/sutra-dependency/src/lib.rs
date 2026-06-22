pub mod architecture;
pub mod engine;
pub mod extractor;
pub mod graph;
pub mod output;
pub mod persist;
pub mod types;

mod module_name {
    use std::path::Path;

    pub fn from_path(file_path: &str) -> String {
        let path = Path::new(file_path);
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy())
            .unwrap_or_default()
            .to_string();

        if stem == "__init__" {
            let parent = path
                .parent()
                .map(|p| p.to_string_lossy())
                .unwrap_or_default()
                .to_string();
            let parent = parent.replace(std::path::MAIN_SEPARATOR, ".");
            if parent.is_empty() || parent == "." {
                return String::new();
            }
            return parent;
        }

        let without_ext = path.with_extension("");
        let components: Vec<String> = without_ext
            .components()
            .filter_map(|c| {
                let s = c.as_os_str().to_str()?;
                if s == "." || s == ".." || s == "src" || s == "lib" {
                    return None;
                }
                Some(s.to_string())
            })
            .collect();

        if components.is_empty() {
            return stem;
        }
        components.join(".")
    }
}

use module_name::from_path as module_name_from_path;

pub use engine::DependencyEngine;
