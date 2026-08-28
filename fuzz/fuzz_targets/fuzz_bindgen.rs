#![no_main]

use libfuzzer_sys::fuzz_target;
use agam_ffi::bindgen::{BindgenConfig, CHeaderParser};

fuzz_target!(|data: &[u8]| {
    if let Ok(c_header) = std::str::from_utf8(data) {
        let config = BindgenConfig {
            library_name: "fuzz_lib".to_string(),
            type_prefix: None,
            allowlist_functions: vec![],
            allowlist_types: vec![],
        };
        let parser = CHeaderParser::new(config);
        let _ = parser.generate_agam_module(c_header);
    }
});
