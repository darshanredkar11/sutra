use crate::types::{RequestWeight, Runtime};

pub fn estimate_request_weight(schema_json: &str, runtime: Runtime) -> RequestWeight {
    let raw_bytes = estimate_raw_bytes(schema_json);
    let expansion_factor = runtime.avg_expansion();
    let runtime_bytes = raw_bytes * expansion_factor;
    let temp_allocations = estimate_temp_allocations(schema_json) as f64;
    let total_bytes = runtime_bytes + temp_allocations;

    RequestWeight {
        raw_bytes,
        expansion_factor,
        runtime_bytes,
        temp_allocations,
        total_bytes,
    }
}

fn estimate_raw_bytes(json: &str) -> f64 {
    let trimmed = json.trim();
    if trimmed.is_empty() {
        return 64.0;
    }

    let mut total = 0.0;
    let mut i = 0;
    let bytes = trimmed.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'"' {
            i += 1;
            let mut key_len = 0u32;
            while i < bytes.len() && bytes[i] != b'"' {
                key_len += 1;
                i += 1;
            }
            total += key_len as f64;
        } else if bytes[i] == b':' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'"' {
                i += 1;
                let mut val_len = 0u32;
                while i < bytes.len() && bytes[i] != b'"' {
                    val_len += 1;
                    i += 1;
                }
                total += val_len as f64;
            } else if i < bytes.len() && (bytes[i] == b't' || bytes[i] == b'f' || bytes[i] == b'n') {
                total += 4.0;
                while i < bytes.len() && bytes[i] != b',' && bytes[i] != b'}' && bytes[i] != b']' {
                    i += 1;
                }
            } else {
                let mut num_len = 0u32;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'.' || bytes[i] == b'-') {
                    num_len += 1;
                    i += 1;
                }
                total += num_len as f64;
            }
        } else {
            i += 1;
        }
    }

    total.max(64.0)
}

fn estimate_temp_allocations(json: &str) -> u32 {
    let mut count = 0u32;
    let mut depth = 0u32;
    for b in json.bytes() {
        match b {
            b'{' | b'[' => {
                depth += 1;
                if depth > 1 {
                    count += 1;
                }
            }
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }
    }
    count * 64
}

pub fn estimate_weight_from_file_length(file_len: usize) -> RequestWeight {
    let raw_bytes = file_len as f64;
    RequestWeight {
        raw_bytes,
        expansion_factor: 3.0,
        runtime_bytes: raw_bytes * 3.0,
        temp_allocations: 256.0,
        total_bytes: raw_bytes * 3.0 + 256.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_raw_bytes_empty() {
        let w = estimate_request_weight("", Runtime::Rust);
        assert!(w.raw_bytes >= 64.0);
    }

    #[test]
    fn test_estimate_raw_bytes_bool_null_only() {
        let json = r#"{"a":true,"b":false,"c":null}"#;
        let w = estimate_request_weight(json, Runtime::Rust);
        assert!(w.raw_bytes >= 64.0);
    }

    #[test]
    fn test_estimate_raw_bytes_numbers_only() {
        let json = r#"{"a":1,"b":3.14,"c":-42}"#;
        let w = estimate_request_weight(json, Runtime::Rust);
        assert!(w.raw_bytes >= 64.0);
    }

    #[test]
    fn test_estimate_raw_bytes_unicode() {
        let json = r#"{"message":"héllo wörld 🔥"}"#;
        let w = estimate_request_weight(json, Runtime::Rust);
        assert!(w.raw_bytes >= 64.0);
    }

    #[test]
    fn test_estimate_raw_bytes_simple() {
        let json = r#"{"name":"darshan","items":[1,2,3]}"#;
        let w = estimate_request_weight(json, Runtime::Rust);
        assert!(w.raw_bytes > 0.0);
        assert!(w.total_bytes > 0.0);
    }

    #[test]
    fn test_estimate_raw_bytes_nested() {
        let json = r#"{"user":{"id":1,"name":"test","items":[1,2,3]}}"#;
        let w = estimate_request_weight(json, Runtime::Jvm);
        assert!(w.raw_bytes > 0.0);
        assert!((w.expansion_factor - 6.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_estimate_temp_allocations() {
        assert_eq!(estimate_temp_allocations(r#"{"a":{"b":1}}"#), 64);
        assert_eq!(estimate_temp_allocations(r#"{"a":1}"#), 0);
    }

    #[test]
    fn test_estimate_temp_allocations_deeply_nested() {
        let json = r#"{"a":{"b":{"c":{"d":1}}}}"#;
        let count = estimate_temp_allocations(json);
        assert_eq!(count, 192); // 3 nested depths * 64
    }

    #[test]
    fn test_estimate_temp_allocations_arrays_nested() {
        let json = r#"{"a":[1,{"b":2}],"c":[[1,2],3]}"#;
        let count = estimate_temp_allocations(json);
        assert!(count > 0);
    }

    #[test]
    fn test_estimate_temp_allocations_no_nesting_zero() {
        assert_eq!(estimate_temp_allocations(r#"42"#), 0);
        assert_eq!(estimate_temp_allocations(r#""hello""#), 0);
        assert_eq!(estimate_temp_allocations(""), 0);
    }

    #[test]
    fn test_runtime_expansion_differences() {
        let json = r#"{"data":"value"}"#;
        let w_jvm = estimate_request_weight(json, Runtime::Jvm);
        let w_rust = estimate_request_weight(json, Runtime::Rust);
        assert!(w_jvm.runtime_bytes > w_rust.runtime_bytes);
    }

    #[test]
    fn test_estimate_weight_from_file_length() {
        let w = estimate_weight_from_file_length(4096);
        assert_eq!(w.raw_bytes, 4096.0);
        assert_eq!(w.expansion_factor, 3.0);
        assert_eq!(w.temp_allocations, 256.0);
        assert_eq!(w.total_bytes, 4096.0 * 3.0 + 256.0);
    }

    #[test]
    fn test_estimate_weight_from_file_length_zero() {
        let w = estimate_weight_from_file_length(0);
        assert_eq!(w.raw_bytes, 0.0);
        assert_eq!(w.total_bytes, 256.0);
    }

    #[test]
    fn test_estimate_request_weight_all_runtimes() {
        let json = r#"{"key":"value"}"#;
        for runtime in &[Runtime::Rust, Runtime::Go, Runtime::Jvm, Runtime::NodeJs, Runtime::Python] {
            let w = estimate_request_weight(json, *runtime);
            assert!(w.total_bytes > 0.0);
            assert!(w.expansion_factor > 0.0);
        }
    }
}
