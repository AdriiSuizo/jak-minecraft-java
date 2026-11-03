pub(crate) fn particle_subset(text: &str, groups: &[&str], parts: &[u32], lightnings: &[&str]) -> String {
    // Its top-level forms, comments and strings skipped over.
    let mut forms: Vec<&str> = Vec::new();
    let bytes = text.as_bytes();
    let (mut depth, mut start, mut i) = (0usize, 0usize, 0usize);
    while i < bytes.len() {
        match bytes[i] {
            b';' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'"' => {
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    i += if bytes[i] == b'\\' { 2 } else { 1 };
                }
            }
            b'(' => {
                if depth == 0 {
                    start = i;
                }
                depth += 1;
            }
            b')' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    forms.push(&text[start..=i]);
                }
            }
            _ => {}
        }
        i += 1;
    }
    let head = |form: &str| -> (String, String) {
        let mut words = form.trim_start_matches('(').split_whitespace();
        (words.next().unwrap_or("").to_owned(), words.next().unwrap_or("").to_owned())
    };
    let launched = |form: &str| -> Vec<u32> {
        let words: Vec<&str> = form.split(|c: char| c.is_whitespace() || c == '(' || c == ')').filter(|w| !w.is_empty()).collect();
        words
            .windows(2)
            .filter(|w| matches!(w[0], "sp-item" | ":next-launcher" | ":binding"))
            .filter_map(|w| w[1].parse().ok())
            .collect()
    };
    let mut wanted: std::collections::BTreeSet<u32> = parts.iter().copied().collect();
    let mut kept: Vec<&str> = Vec::new();
    for form in &forms {
        let (kind, name) = head(form);
        if kind == "defpartgroup" && groups.contains(&name.as_str()) {
            wanted.extend(launched(form));
            kept.push(form);
        }
    }
    // Parts launching parts, until none is new.
    loop {
        let before = wanted.len();
        for form in &forms {
            let (kind, name) = head(form);
            if kind == "defpart" && name.parse().is_ok_and(|id: u32| wanted.contains(&id)) {
                wanted.extend(launched(form));
            }
        }
        if wanted.len() == before {
            break;
        }
    }
    for form in &forms {
        let (kind, name) = head(form);
        if kind == "defpart" && name.parse().is_ok_and(|id: u32| wanted.contains(&id)) {
            kept.push(form);
        }
    }
    // The lightning specs asked for, by their `:name`.
    for form in &forms {
        if form.contains("'lightning-spec")
            && lightnings.iter().any(|name| form.contains(&format!(":name \"{name}\"")))
        {
            kept.push(form);
        }
    }
    kept.join("\n\n")
}
