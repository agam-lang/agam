#![no_main]

use libfuzzer_sys::fuzz_target;
use agam_lexer::tokenize;
use agam_parser::parse;
use agam_errors::SourceId;

fuzz_target!(|data: &[u8]| {
    if let Ok(src) = std::str::from_utf8(data) {
        let tokens = tokenize(src, SourceId(0));
        let _ = parse(tokens, SourceId(0));
    }
});
