// Quick test of our sanitization logic
fn main() {
    // Test cases that should be sanitized
    let test_cases = vec![
        ".NET Rocks!",
        "CON",  // Windows reserved name
        "Some//Bad\\Characters:Here",
        "   Leading and trailing spaces   ",
        "Unicode: café résumé naïve",
        r#"Smart quotes: "Hello" and 'world'"#,
        "Very long name that exceeds typical filename limits and should be truncated at some reasonable point to ensure cross platform compatibility",
        "",  // Empty string
    ];
    
    for test in test_cases {
        println!("Original: '{}'", test);
        
        // Simulate our sanitization logic
        let mut sanitized = String::new();
        for ch in test.trim().chars() {
            match ch {
                // Windows prohibited characters - replace with safe alternatives
                '<' => sanitized.push('('),
                '>' => sanitized.push(')'),
                ':' => sanitized.push('-'),
                '"' => sanitized.push('\''),
                '/' => sanitized.push('-'),
                '\\' => sanitized.push('-'),
                '|' => sanitized.push('-'),
                '?' => {}, // Remove question marks
                '*' => {}, // Remove wildcards
                // Control characters - skip
                c if c.is_control() => {},
                // Keep safe characters
                c if c.is_ascii_alphanumeric() => sanitized.push(ch),
                ' ' | '-' | '_' | '(' | ')' => sanitized.push(ch),
                // Handle periods - don't allow leading
                '.' => {
                    if !sanitized.is_empty() {
                        sanitized.push('.');
                    }
                },
                // Convert accented characters
                'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'ā' => sanitized.push('a'),
                'é' | 'è' | 'ê' | 'ë' | 'ē' => sanitized.push('e'),
                'í' | 'ì' | 'î' | 'ï' | 'ī' => sanitized.push('i'),
                'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ō' => sanitized.push('o'),
                'ú' | 'ù' | 'û' | 'ü' | 'ū' => sanitized.push('u'),
                'ñ' => sanitized.push('n'),
                'ç' => sanitized.push('c'),
                '&' => sanitized.push_str("and"),
                '!' => {}, // Remove exclamation marks
                _ => sanitized.push(' '),
            }
        }
        
        // Clean up spaces
        let cleaned = sanitized
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .replace("--", "-")
            .replace("__", "_");
        
        let mut final_name = cleaned.trim().to_string();
        
        // Don't allow names that end with period or space
        while final_name.ends_with('.') || final_name.ends_with(' ') {
            final_name.pop();
        }
        
        // Don't allow names that start with period
        while final_name.starts_with('.') {
            final_name = final_name.chars().skip(1).collect();
        }
        
        // Check for Windows reserved names
        let upper_name = final_name.to_uppercase();
        if ["CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"].contains(&upper_name.as_str()) {
            final_name = format!("_{}", final_name);
        }
        
        // Handle empty case
        if final_name.trim().is_empty() {
            final_name = "Podcast".to_string();
        }
        
        println!("Sanitized: '{}'\n", final_name);
    }
}