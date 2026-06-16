pub fn normalize_queue_macros(source: &str) -> String {
    let mut out = String::new();
    for line in source.lines() {
        let trimmed = line.trim();
        let indent_len = line.len() - line.trim_start().len();
        let indent = &line[..indent_len];

        if let Some(args) = macro_args(trimmed, "SLIST_HEAD") {
            if let Some(name) = args.first() {
                let var =
                    trailing_decl_name_after_macro(trimmed).unwrap_or_else(|| name.to_string());
                out.push_str(indent);
                out.push_str("int ");
                out.push_str(&var);
                out.push_str(" = 0;\n");
                continue;
            }
        }

        if let Some(args) = macro_args(trimmed, "TAILQ_HEAD") {
            if let Some(name) = args.first() {
                let var =
                    trailing_decl_name_after_macro(trimmed).unwrap_or_else(|| name.to_string());
                out.push_str(indent);
                out.push_str("int ");
                out.push_str(&var);
                out.push_str(" = 0;\n");
                out.push_str(indent);
                out.push_str("int ");
                out.push_str(&var);
                out.push_str("_tail = 0;\n");
                continue;
            }
        }

        if trimmed.starts_with("SLIST_ENTRY(") || trimmed.starts_with("TAILQ_ENTRY(") {
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

        if let Some(args) = macro_args(trimmed, "TAILQ_INSERT_TAIL") {
            if args.len() >= 3 {
                let head = strip_addr(&args[0]);
                let elm = args[1].trim();
                let field = args[2].trim();
                let tail = tail_name(head);
                out.push_str(indent);
                out.push_str(elm);
                out.push_str("->");
                out.push_str(field);
                out.push_str(".tqe_next = 0;\n");
                out.push_str(indent);
                out.push_str("if (");
                out.push_str(&tail);
                out.push_str(") ");
                out.push_str(&tail);
                out.push_str("->");
                out.push_str(field);
                out.push_str(".tqe_next = ");
                out.push_str(elm);
                out.push_str(";\n");
                out.push_str(indent);
                out.push_str("if (!");
                out.push_str(&tail);
                out.push_str(") ");
                out.push_str(head);
                out.push_str(" = ");
                out.push_str(elm);
                out.push_str(";\n");
                out.push_str(indent);
                out.push_str(&tail);
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

        if let Some(args) = macro_args(trimmed, "TAILQ_FOREACH") {
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
                out.push_str(".tqe_next) ");
                out.push_str(suffix);
                out.push('\n');
                continue;
            }
        }

        out.push_str(&replace_tailq_exprs(line));
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

fn trailing_decl_name_after_macro(line: &str) -> Option<String> {
    let close = line.rfind(')')?;
    let rest = line[close + 1..].trim().trim_end_matches(';').trim();
    if rest.is_empty() {
        return None;
    }
    let before_eq = rest.split('=').next().unwrap_or(rest).trim();
    before_eq
        .split_whitespace()
        .last()
        .filter(|name| !name.is_empty())
        .map(|name| name.trim_start_matches('*').to_string())
}

fn strip_addr(text: &str) -> &str {
    text.trim().strip_prefix('&').unwrap_or(text.trim()).trim()
}

fn tail_name(head: &str) -> String {
    format!("{head}_tail")
}

fn replace_tailq_exprs(line: &str) -> String {
    let mut out = line.to_string();
    out = replace_macro_expr(&out, "TAILQ_EMPTY", |args| {
        args.first()
            .map(|head| format!("({} == 0)", strip_addr(head)))
            .unwrap_or_else(|| "0".to_string())
    });
    out = replace_macro_expr(&out, "TAILQ_FIRST", |args| {
        args.first()
            .map(|head| strip_addr(head).to_string())
            .unwrap_or_else(|| "0".to_string())
    });
    replace_macro_expr(&out, "TAILQ_LAST", |args| {
        args.first()
            .map(|head| tail_name(strip_addr(head)))
            .unwrap_or_else(|| "0".to_string())
    })
}

fn replace_macro_expr<F>(source: &str, name: &str, mut replacement: F) -> String
where
    F: FnMut(Vec<String>) -> String,
{
    let mut out = String::new();
    let mut pos = 0;
    while let Some(start) = find_macro_call(source, name, pos) {
        out.push_str(&source[pos..start]);
        let open = start + name.len();
        let Some(close) = matching_paren(source, open) else {
            out.push_str(&source[start..]);
            return out;
        };
        out.push_str(&replacement(split_args(&source[open + 1..close])));
        pos = close + 1;
    }
    out.push_str(&source[pos..]);
    out
}

fn find_macro_call(source: &str, name: &str, pos: usize) -> Option<usize> {
    let mut search = pos;
    while let Some(rel) = source[search..].find(name) {
        let start = search + rel;
        let end = start + name.len();
        let before = source[..start].chars().next_back();
        let after = source[end..].chars().next();
        if !before.is_some_and(is_ident_char) && after == Some('(') {
            return Some(start);
        }
        search = end;
    }
    None
}

fn is_ident_char(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}
