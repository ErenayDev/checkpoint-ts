use serde_json::Value;
use std::fs;

pub fn find_entry_file() -> Option<String> {
    // Looking for package.json
    if fs::metadata("./package.json").is_ok() {
        let pkg_content = fs::read_to_string("./package.json").ok()?;
        let pkg: Value = serde_json::from_str(&pkg_content).ok()?;
        if let Some(main_field) = pkg.get("main").and_then(|v| v.as_str()) {
            let main_path = main_field.trim();
            if fs::metadata(main_path).is_ok() {
                return Some(main_path.to_string());
            }
        }
    }

    // Looking for index.ts
    if fs::metadata("./index.ts").is_ok() {
        return Some("./index.ts".to_string());
    }

    // Looking for index.js
    if fs::metadata("./index.js").is_ok() {
        return Some("./index.js".to_string());
    }

    // Looking for main.ts
    if fs::metadata("./main.ts").is_ok() {
        return Some("./main.ts".to_string());
    }

    // Looking for main.js
    if fs::metadata("./main.js").is_ok() {
        return Some("./main.js".to_string());
    }

    // If nothing, return None
    None
}
