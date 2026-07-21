//! Shared repo-walking helpers.
//!
//! Every `sutra-repair-*` engine (and `sutra-rse`) needs to enumerate a
//! repo's source files by extension. Each engine used to carry its own
//! copy-pasted `walkdir::WalkDir` with a hand-rolled, inconsistent
//! vendor/build-dir exclusion list -- some skipped `target/`, none skipped
//! `build/`, so generated code (`mobile/shared/build/generated/sqldelight/
//! .../*.kt`, `target/debug/build/*/out/*.rs`) still got scanned and
//! produced findings against files nobody wrote or can act on.

/// Directory names never worth descending into: their contents are
/// vendored, generated, or build-output, not code the repo's authors wrote.
pub const VENDOR_DIRS: [&str; 8] = [
    "node_modules",
    "vendor",
    "target",
    "dist",
    "build",
    ".git",
    "venv",
    "__pycache__",
];

/// Recursively list every file under `repo_path` whose extension is in
/// `extensions`, skipping [`VENDOR_DIRS`] and any other hidden directory.
pub fn discover_source_files(repo_path: &str, extensions: &[&str]) -> Vec<String> {
    walkdir::WalkDir::new(repo_path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            if !e.file_type().is_dir() {
                return true;
            }
            match e.file_name().to_str() {
                Some(name) => !name.starts_with('.') && !VENDOR_DIRS.contains(&name),
                None => true,
            }
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|x| x.to_str())
                .map(|ext| extensions.contains(&ext))
                .unwrap_or(false)
        })
        .map(|e| e.path().to_string_lossy().into_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_source_files_skips_vendor_dirs() {
        let dir = std::env::temp_dir().join(format!(
            "sutra-common-fs-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::create_dir_all(dir.join("target/debug/build")).unwrap();
        std::fs::create_dir_all(dir.join("node_modules/pkg")).unwrap();
        std::fs::create_dir_all(dir.join("mobile/build/generated")).unwrap();
        std::fs::write(dir.join("src/main.rs"), "fn main() {}").unwrap();
        std::fs::write(dir.join("target/debug/build/out.rs"), "// generated").unwrap();
        std::fs::write(dir.join("node_modules/pkg/index.js"), "// vendored").unwrap();
        std::fs::write(dir.join("mobile/build/generated/Gen.kt"), "// generated").unwrap();

        let files = discover_source_files(dir.to_str().unwrap(), &["rs", "js", "kt"]);
        assert_eq!(files.len(), 1, "expected only src/main.rs, got {:?}", files);
        assert!(files[0].ends_with("src/main.rs"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_discover_source_files_filters_by_extension() {
        let dir = std::env::temp_dir().join(format!(
            "sutra-common-fs-test-ext-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.rs"), "").unwrap();
        std::fs::write(dir.join("b.txt"), "").unwrap();

        let files = discover_source_files(dir.to_str().unwrap(), &["rs"]);
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("a.rs"));

        std::fs::remove_dir_all(&dir).ok();
    }
}
