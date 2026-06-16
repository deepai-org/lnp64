pub fn normalize_queue_macros(source: &str) -> String {
    let mut out = String::new();
    for line in source.lines() {
        let trimmed = line.trim();
        let indent_len = line.len() - line.trim_start().len();
        let indent = &line[..indent_len];

        if let Some(args) = macro_args(trimmed, "SLIST_HEAD") {
            if let Some(name) = args.first() {
                let var = trailing_ident_after_macro(trimmed).unwrap_or_else(|| name.to_string());
                out.push_str(indent);
                out.push_str("int ");
                out.push_str(&var);
                out.push_str(" = 0;\n");
                continue;
            }
        }

        if trimmed.starts_with("SLIST_ENTRY(") {
            out.push_str(indent);
            out.push_str("int entry;\n");
            continue;
        }

        if let Some(args) = macro_args(trimmed, "SLIST_INIT") {
            if let Some(head) = args.first() {
                out.push_str(indent);
                out.push_str(strip_addr(head));
                out.push_str(" = 0;\n");
                continue;
            }
        }

        if let Some(args) = macro_args(trimmed, "SLIST_INSERT_HEAD") {
            if args.len() >= 3 {
                let head = strip_addr(&args[0]);
                let elm = args[1].trim();
                let field = args[2].trim();
                out.push_str(indent);
                out.push_str(elm);
                out.push_str("->");
                out.push_str(field);
                out.push_str(".sle_next = ");
                out.push_str(head);
                out.push_str(";\n");
                out.push_str(indent);
                out.push_str(head);
                out.push_str(" = ");
                out.push_str(elm);
                out.push_str(";\n");
                continue;
            }
        }

        if let Some(args) = macro_args(trimmed, "SLIST_FOREACH") {
            if args.len() >= 3 {
                let var = args[0].trim();
                let head = strip_addr(&args[1]);
                let field = args[2].trim();
                let suffix = trimmed
                    .find(')')
                    .map(|idx| trimmed[idx + 1..].trim())
                    .unwrap_or("");
                out.push_str(indent);
                out.push_str("for (");
                out.push_str(var);
                out.push_str(" = ");
                out.push_str(head);
                out.push_str("; ");
                out.push_str(var);
                out.push_str("; ");
                out.push_str(var);
                out.push_str(" = ");
                out.push_str(var);
                out.push_str("->");
                out.push_str(field);
                out.push_str(".sle_next) ");
                out.push_str(suffix);
                out.push('\n');
                continue;
            }
        }

        out.push_str(line);
        out.push('\n');
    }
    out
}

fn macro_args(line: &str, name: &str) -> Option<Vec<String>> {
    let start = line.find(name)?;
    let open = start + name.len();
    if line[open..].chars().next()? != '(' {
        return None;
    }
    let close = matching_paren(line, open)?;
    Some(split_args(&line[open + 1..close]))
}

fn matching_paren(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0i64;
    for (rel, ch) in text[open..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + rel);
                }
            }
            _ => {}
        }
    }
    None
}

fn split_args(text: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut depth = 0i64;
    let mut start = 0usize;
    for (idx, ch) in text.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                args.push(text[start..idx].trim().to_string());
                start = idx + 1;
            }
            _ => {}
        }
    }
    args.push(text[start..].trim().to_string());
    args
}

fn trailing_ident_after_macro(line: &str) -> Option<String> {
    let close = line.rfind(')')?;
    let rest = line[close + 1..].trim().trim_end_matches(';').trim();
    if rest.is_empty() {
        return None;
    }
    Some(rest.to_string())
}

fn strip_addr(text: &str) -> &str {
    text.trim().strip_prefix('&').unwrap_or(text.trim()).trim()
}
