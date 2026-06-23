use crate::types::{ComplexityClass, ComplexityProfile};

pub fn analyze_source_code(source: &str, file_ext: &str) -> ComplexityProfile {
    match file_ext {
        "java" | "kt" | "kts" => analyze_jvm(source),
        "js" | "ts" | "mjs" | "mts" => analyze_js_ts(source),
        "py" => analyze_python(source),
        "rs" => analyze_rust(source),
        "go" => analyze_go(source),
        _ => analyze_generic(source),
    }
}

fn count_loops(source: &str) -> (u32, u32) {
    let patterns = [
        (r"\bfor\s*\(", 1u32),
        (r"\bwhile\s*\(", 1),
        (r"\bdo\s*\{", 1),
        (r"\bfor\s+[a-zA-Z_]", 1),
        (r"\bforEach\b", 1),
        (r"\bmap\s*\(", 1),
    ];
    let mut count = 0u32;
    for (pat, weight) in &patterns {
        if let Ok(re) = regex::Regex::new(pat) {
            count += re.find_iter(source).count() as u32 * weight;
        }
    }
    (count, count)
}

fn estimate_max_nesting(source: &str) -> u32 {
    let mut max_depth = 0u32;
    let mut depth = 0u32;

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("#") || trimmed.starts_with("/*") || trimmed.starts_with("*") {
            continue;
        }

        let open = trimmed.matches('{').count();
        let close = trimmed.matches('}').count();

        if open > close {
            depth += (open - close) as u32;
            max_depth = max_depth.max(depth);
        } else if close > open {
            depth = depth.saturating_sub((close - open) as u32);
        }

        if trimmed.contains("if ") || trimmed.contains("for ") || trimmed.contains("while ") || trimmed.contains("catch ") || trimmed.contains("except") {
            if !trimmed.contains('{') && !trimmed.ends_with(':') {
                let nest_est = 1 + count_indent_level(trimmed);
                max_depth = max_depth.max(nest_est);
            }
        }
    }

    max_depth
}

fn count_indent_level(line: &str) -> u32 {
    let spaces = line.chars().take_while(|c| *c == ' ').count();
    (spaces / 4) as u32
}

fn estimate_complexity_class(loop_count: u32, nesting: u32) -> ComplexityClass {
    if loop_count == 0 {
        return ComplexityClass::O1;
    }
    match loop_count {
        1 => {
            if nesting >= 4 {
                ComplexityClass::ON3
            } else if nesting >= 3 {
                ComplexityClass::ON2
            } else {
                ComplexityClass::ON
            }
        }
        2 => {
            if nesting >= 3 {
                ComplexityClass::ON3
            } else {
                ComplexityClass::ON2
            }
        }
        3..=5 => ComplexityClass::ON2,
        6..=10 => ComplexityClass::ON3,
        _ => {
            if nesting >= 5 {
                ComplexityClass::O2N
            } else {
                ComplexityClass::ON3
            }
        }
    }
}

fn count_allocations(source: &str) -> u32 {
    let alloc_patterns = [
        r"\bnew\s+[A-Z]",
        r"\bvec!",
        r"\bVec::new\b",
        r"\bHashMap::new\b",
        r"\bHashSet::new\b",
        r"\bBox::new\b",
        r"\bRc::new\b",
        r"\bArc::new\b",
        r"\bString::new\b",
        r"\bformat!",
        r"\bcollect\b",
        r"\bclone\b",
        r"\bmake\b",
        r"\bappend\b",
        r"\bextend\b",
        r"\bpush\b",
        r"\binsert\b",
        r"\b\[.\]",
    ];
    let mut count = 0u32;
    for pat in &alloc_patterns {
        if let Ok(re) = regex::Regex::new(pat) {
            count += re.find_iter(source).count() as u32;
        }
    }
    count.min(1000)
}

fn count_branches(source: &str) -> u32 {
    let branch_patterns = [
        r"\bif\b",
        r"\belse\s+if\b",
        r"\belse\b",
        r"\bmatch\b",
        r"\bswitch\b",
        r"\bcase\b",
        r"\bcond\b",
        r"\?\s*[a-zA-Z_]+\s*:",
        r"\|\|",
        r"&&",
    ];
    let mut count = 0u32;
    for pat in &branch_patterns {
        if let Ok(re) = regex::Regex::new(pat) {
            count += re.find_iter(source).count() as u32;
        }
    }
    count
}

fn count_functions(source: &str) -> u32 {
    let fn_patterns = [
        r"\bfn\s+[a-zA-Z_]\w*\s*\(",
        r"\bdef\s+[a-zA-Z_]\w*\s*\(",
        r"\bfunction\s+[a-zA-Z_]\w*\s*\(",
        r"\bpublic\s+\w+\s+\w+\s*\(",
        r"\bprivate\s+\w+\s+\w+\s*\(",
        r"\bprotected\s+\w+\s+\w+\s*\(",
        r"\bfun\s+[a-zA-Z_]\w*\s*\(",
        r"\bfunc\s+[a-zA-Z_]\w*\s*\(",
        r"\basync\s+fn\s+[a-zA-Z_]\w*\s*\(",
        r"\bdef\s+[a-zA-Z_]\w*\s*\(self",
    ];
    let mut count = 0u32;
    for pat in &fn_patterns {
        if let Ok(re) = regex::Regex::new(pat) {
            count += re.find_iter(source).count() as u32;
        }
    }
    count
}

fn analyze_jvm(source: &str) -> ComplexityProfile {
    let (loop_count, _) = count_loops(source);
    let nesting = estimate_max_nesting(source);
    ComplexityProfile {
        time_complexity: estimate_complexity_class(loop_count, nesting),
        loop_depth: nesting,
        allocation_count: count_allocations(source),
        branch_count: count_branches(source),
        function_count: count_functions(source),
    }
}

fn analyze_js_ts(source: &str) -> ComplexityProfile {
    let (loop_count, _) = count_loops(source);
    let nesting = estimate_max_nesting(source);
    ComplexityProfile {
        time_complexity: estimate_complexity_class(loop_count, nesting),
        loop_depth: nesting,
        allocation_count: count_allocations(source) + (source.matches("await ").count() as u32),
        branch_count: count_branches(source) + (source.matches(".then(").count() as u32),
        function_count: count_functions(source),
    }
}

fn analyze_python(source: &str) -> ComplexityProfile {
    let (loop_count, _) = count_loops(source);
    let nesting = estimate_max_nesting_python(source);
    ComplexityProfile {
        time_complexity: estimate_complexity_class(loop_count, nesting),
        loop_depth: nesting,
        allocation_count: count_allocations(source) + (source.matches("yield").count() as u32),
        branch_count: count_branches(source),
        function_count: count_functions(source),
    }
}

fn estimate_max_nesting_python(source: &str) -> u32 {
    let mut max_depth = 0u32;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let depth = count_indent_level(line);
        max_depth = max_depth.max(depth);
    }
    (max_depth / 2).max(1)
}

fn analyze_rust(source: &str) -> ComplexityProfile {
    let (loop_count, _) = count_loops(source);
    let nesting = estimate_max_nesting(source);
    ComplexityProfile {
        time_complexity: estimate_complexity_class(loop_count, nesting),
        loop_depth: nesting,
        allocation_count: count_allocations(source),
        branch_count: count_branches(source) + (source.matches("unwrap(").count() as u32) + (source.matches("expect(").count() as u32),
        function_count: count_functions(source),
    }
}

fn analyze_go(source: &str) -> ComplexityProfile {
    let (loop_count, _) = count_loops(source);
    let nesting = estimate_max_nesting(source);
    ComplexityProfile {
        time_complexity: estimate_complexity_class(loop_count, nesting),
        loop_depth: nesting,
        allocation_count: count_allocations(source) + (source.matches("make(").count() as u32),
        branch_count: count_branches(source) + (source.matches("err != nil").count() as u32),
        function_count: count_functions(source),
    }
}

fn analyze_generic(source: &str) -> ComplexityProfile {
    let (loop_count, _) = count_loops(source);
    let nesting = estimate_max_nesting(source);
    ComplexityProfile {
        time_complexity: estimate_complexity_class(loop_count, nesting),
        loop_depth: nesting,
        allocation_count: count_allocations(source),
        branch_count: count_branches(source),
        function_count: count_functions(source),
    }
}

pub fn detect_endpoints(source: &str, file_ext: &str) -> Vec<(String, String)> {
    let mut endpoints = Vec::new();
    let patterns: &[(&str, &str)] = match file_ext {
        "java" | "kt" | "kts" => &[
            (r#"(?:@(?:GetMapping|PostMapping|PutMapping|DeleteMapping|RequestMapping)\s*\(\s*[""'])([^""']+)"#, "HTTP"),
            (r#"(?:@Path\s*\(\s*[""'])([^""']+)"#, "HTTP"),
        ],
        "py" => &[
            (r#"(?:@(?:app|router)\.(?:get|post|put|delete|route)\s*\(\s*[""'])([^""']+)"#, "HTTP"),
            (r#"(?:@(?:app|router)\.(?:get|post|put|delete|route)\s*\(\s*[""'])([^""']+)"#, "HTTP"),
        ],
        "js" | "ts" | "mjs" | "mts" => &[
            (r#"(?:\.(?:get|post|put|delete|patch)\s*\(\s*[""'])([^""']+)"#, "HTTP"),
            (r#"(?:router\.(?:get|post|put|delete|patch)\s*\(\s*[""'])([^""']+)"#, "HTTP"),
        ],
        "rs" => &[
            (r#"(?:\.route\s*\(\s*[""'])([^""']+)"#, "HTTP"),
            (r#"(?:#\[(?:get|post|put|delete)\s*\(\s*[""'])([^""']+)"#, "HTTP"),
        ],
        "go" => &[
            (r#"(?:\.(?:GET|POST|PUT|DELETE|HandleFunc)\s*\(\s*[""'])([^""']+)"#, "HTTP"),
        ],
        _ => &[],
    };

    for (pat, method) in patterns {
        if let Ok(re) = regex::Regex::new(pat) {
            for cap in re.captures_iter(source) {
                endpoints.push((cap[1].to_string(), method.to_string()));
            }
        }
    }

    endpoints
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_loops_empty() {
        assert_eq!(count_loops("").0, 0);
    }

    #[test]
    fn test_count_loops_simple() {
        assert_eq!(count_loops("while(true) { for(int j=0;;) {} }").0, 2);
    }

    #[test]
    fn test_count_loops_for_each_and_map_counted() {
        let src = "items.forEach(x => f(x)); items.map(x => x + 1);";
        assert_eq!(count_loops(src).0, 2);
    }

    #[test]
    fn test_estimate_complexity_no_loops() {
        assert_eq!(estimate_complexity_class(0, 0), ComplexityClass::O1);
    }

    #[test]
    fn test_estimate_complexity_nested() {
        assert_eq!(estimate_complexity_class(1, 3), ComplexityClass::ON2);
        assert_eq!(estimate_complexity_class(2, 0), ComplexityClass::ON2);
    }

    #[test]
    fn test_estimate_complexity_on3_with_deep_nest() {
        assert_eq!(estimate_complexity_class(1, 4), ComplexityClass::ON3);
        assert_eq!(estimate_complexity_class(2, 3), ComplexityClass::ON3);
    }

    #[test]
    fn test_estimate_complexity_o2n_with_many_loops() {
        assert_eq!(estimate_complexity_class(11, 4), ComplexityClass::ON3);
        assert_eq!(estimate_complexity_class(20, 5), ComplexityClass::O2N);
    }

    #[test]
    fn test_estimate_complexity_range_on2() {
        assert_eq!(estimate_complexity_class(3, 0), ComplexityClass::ON2);
        assert_eq!(estimate_complexity_class(5, 0), ComplexityClass::ON2);
        assert_eq!(estimate_complexity_class(6, 0), ComplexityClass::ON3);
        assert_eq!(estimate_complexity_class(10, 0), ComplexityClass::ON3);
    }

    #[test]
    fn test_count_allocations_new_and_vec() {
        let src = "let v = vec![1,2,3]; let b = Box::new(42); let s = String::new();";
        let n = count_allocations(src);
        assert!(n >= 3, "expected >= 3 allocations, got {}", n);
    }

    #[test]
    fn test_count_allocations_capped_at_1000() {
        let src = "new A ".repeat(2000);
        assert_eq!(count_allocations(&src), 1000);
    }

    #[test]
    fn test_count_allocations_no_match_zero() {
        assert_eq!(count_allocations("fn foo() -> i32 { 42 }"), 0);
    }

    #[test]
    fn test_count_branches_if_else() {
        let src = "if a { 1 } else if b { 2 } else { 3 }";
        assert!(count_branches(src) >= 3);
    }

    #[test]
    fn test_count_branches_match_switch() {
        let src = "match x { 1 => true, _ => false } switch(y) { case 1: break; }";
        assert!(count_branches(src) >= 3);
    }

    #[test]
    fn test_count_branches_ternary_and_ops() {
        let src = "let x = a ? b : c; if d && e || f { }";
        assert_eq!(count_branches(src), 4);
    }

    #[test]
    fn test_count_branches_empty_zero() {
        assert_eq!(count_branches("fn foo() { }"), 0);
    }

    #[test]
    fn test_count_functions_all_patterns() {
        let src = "fn foo() {} def bar(): pass function baz() {} async fn qux() {}";
        assert_eq!(count_functions(src), 5);
    }

    #[test]
    fn test_count_functions_java_methods() {
        let src = "public void foo() {} private int bar() {} protected String baz() {}";
        assert_eq!(count_functions(src), 3);
    }

    #[test]
    fn test_count_functions_empty_zero() {
        assert_eq!(count_functions("let x = 42;"), 0);
    }

    #[test]
    fn test_estimate_max_nesting_c_style() {
        let src = "if a {\n  if b {\n    if c {\n    }\n  }\n}";
        assert_eq!(estimate_max_nesting(src), 3);
    }

    #[test]
    fn test_estimate_max_nesting_python_style() {
        let src = "if a:\n    if b:\n        print(1)";
        let depth = estimate_max_nesting_python(src);
        assert_eq!(depth, 1);
    }

    #[test]
    fn test_estimate_max_nesting_indent_after_keyword() {
        let src = "if a\n    for b\n        while c";
        let depth = estimate_max_nesting(src);
        assert!(depth > 0);
    }

    #[test]
    fn test_analyze_source_code_rust() {
        let src = "fn process(items: &[i32]) { for i in items { println!(\"{}\", i); } }";
        let profile = analyze_source_code(src, "rs");
        assert_eq!(profile.time_complexity, ComplexityClass::ON);
        assert!(profile.function_count > 0);
    }

    #[test]
    fn test_analyze_source_code_python() {
        let src = "def process(items):\n    for item in items:\n        print(item)";
        let profile = analyze_source_code(src, "py");
        assert_eq!(profile.time_complexity, ComplexityClass::ON);
        assert!(profile.function_count > 0);
    }

    #[test]
    fn test_analyze_source_code_java() {
        let src = "class Foo { public void run() { for(int i=0;i<10;i++) { System.out.println(i); } } }";
        let profile = analyze_source_code(src, "java");
        assert_eq!(profile.time_complexity, ComplexityClass::ON);
        assert!(profile.function_count > 0);
    }

    #[test]
    fn test_analyze_source_code_go() {
        let src = "func process(items []int) { for _, i := range items { fmt.Println(i) } }";
        let profile = analyze_source_code(src, "go");
        assert_eq!(profile.time_complexity, ComplexityClass::ON);
        assert!(profile.function_count > 0);
    }

    #[test]
    fn test_analyze_source_code_javascript() {
        let src = "function process(items) { for (let i of items) { console.log(i); } }";
        let profile = analyze_source_code(src, "js");
        assert_eq!(profile.time_complexity, ComplexityClass::ON);
        assert!(profile.function_count > 0);
    }

    #[test]
    fn test_analyze_source_code_generic_fallback() {
        let src = "for i in 1..10 { puts(i); }";
        let profile = analyze_source_code(src, "rb");
        assert_eq!(profile.time_complexity, ComplexityClass::ON);
    }

    #[test]
    fn test_analyze_source_code_empty() {
        let profile = analyze_source_code("", "rs");
        assert_eq!(profile.time_complexity, ComplexityClass::O1);
        assert_eq!(profile.loop_depth, 0);
        assert_eq!(profile.function_count, 0);
    }

    #[test]
    fn test_analyze_source_code_js_ts_await_counts() {
        let src = "async function get() { let r = await fetch('/api'); return r.json(); }";
        let profile = analyze_source_code(src, "ts");
        assert!(profile.allocation_count > 0);
    }

    #[test]
    fn test_analyze_source_code_go_err_check_adds_branches() {
        let src = "func foo() error { _, err := do(); if err != nil { return err }; return nil }";
        let profile = analyze_source_code(src, "go");
        assert!(profile.branch_count > 0);
    }

    #[test]
    fn test_analyze_source_code_rust_unwrap_adds_branches() {
        let src = "fn foo() { let x = bar().unwrap(); let y = baz().expect(\"msg\"); }";
        let profile = analyze_source_code(src, "rs");
        assert!(profile.branch_count > 0 || profile.allocation_count > 0);
    }

    #[test]
    fn test_detect_endpoints_rust() {
        let src = r#"#[get("/api/users")]"#;
        let eps = detect_endpoints(src, "rs");
        assert!(!eps.is_empty());
    }

    #[test]
    fn test_detect_endpoints_python() {
        let src = r#"@app.get("/health")"#;
        let eps = detect_endpoints(src, "py");
        assert!(!eps.is_empty());
    }

    #[test]
    fn test_detect_endpoints_java() {
        let src = r#"@GetMapping("/api/users") public List<User> getUsers() {}"#;
        let eps = detect_endpoints(src, "java");
        assert!(!eps.is_empty());
    }

    #[test]
    fn test_detect_endpoints_javascript() {
        let src = r#"app.get("/api/users", (req, res) => {});"#;
        let eps = detect_endpoints(src, "js");
        assert!(!eps.is_empty());
    }

    #[test]
    fn test_detect_endpoints_go() {
        let src = r#"http.HandleFunc("/api/users", handler)"#;
        let eps = detect_endpoints(src, "go");
        assert!(!eps.is_empty());
    }

    #[test]
    fn test_detect_endpoints_unsupported_lang_empty() {
        let src = r#"route("/api", handler)"#;
        let eps = detect_endpoints(src, "rb");
        assert!(eps.is_empty());
    }

    #[test]
    fn test_detect_endpoints_no_match_empty() {
        let src = "fn foo() { let x = 42; }";
        let eps = detect_endpoints(src, "rs");
        assert!(eps.is_empty());
    }

    #[test]
    fn test_estimate_complexity_ologn_via_single_loop_low_nest() {
        assert_eq!(estimate_complexity_class(1, 1), ComplexityClass::ON);
        assert_eq!(estimate_complexity_class(1, 2), ComplexityClass::ON);
    }
}
