pub fn promote_static_local_scalars(source: &str) -> String {
    let mut globals = Vec::new();
    let mut body = String::new();
    let mut pending_fn: Option<String> = None;
    let mut current_fn: Option<String> = None;
    let mut function_depth = 0i32;
    let mut renames: Vec<(String, String)> = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();
        if current_fn.is_none() {
            if let Some(name) = function_name_before_body(trimmed) {
                pending_fn = Some(name);
            }
        }

        let mut emit_line = line.to_string();
        if let Some(fn_name) = current_fn.as_deref()
            && let Some((name, init)) = parse_static_scalar_decl(trimmed)
        {
            let global = format!("__c_static_{fn_name}_{name}");
            globals.push(format!("int {global} = {init};"));
            renames.push((name, global));
            emit_line.clear();
        } else if current_fn.is_some() {
            for (from, to) in &renames {
                emit_line = replace_ident(&emit_line, from, to);
            }
        }

        body.push_str(&emit_line);
        body.push('\n');

        let (opens, closes) = count_code_braces(line);
        if current_fn.is_none() && opens > 0 {
            if let Some(name) = pending_fn.take() {
                current_fn = Some(name);
                function_depth = opens - closes;
            }
        } else if current_fn.is_some() {
            function_depth += opens - closes;
            if function_depth <= 0 {
                current_fn = None;
                renames.clear();
            }
        }
    }

    if globals.is_empty() {
        body
    } else {
        let mut out = globals.join("\n");
        out.push('\n');
        out.push_str(&body);
        out
    }
}

fn function_name_before_body(trimmed: &str) -> Option<String> {
    if trimmed.ends_with(';') || !trimmed.ends_with(')') {
        return None;
    }
    let before_paren = trimmed.rsplit_once('(')?.0.trim();
    let name = before_paren
        .rsplit(|ch: char| ch.is_whitespace() || ch == '*')
        .next()?;
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn parse_static_scalar_decl(trimmed: &str) -> Option<(String, String)> {
    if !trimmed.starts_with("static ") || !trimmed.ends_with(';') {
        return None;
    }
    if trimmed.contains('(') || trimmed.contains('[') || trimmed.contains('*') {
        return None;
    }
    let body = trimmed
        .trim_start_matches("static ")
        .trim_end_matches(';')
        .trim();
    let (left, init) = body.split_once('=')?;
    let name = left.split_whitespace().last()?.trim();
    if name.is_empty() {
        return None;
    }
    Some((name.to_string(), init.trim().to_string()))
}

fn replace_ident(line: &str, from: &str, to: &str) -> String {
    let mut out = String::new();
    let mut ident = String::new();
    for ch in line.chars() {
        if ch == '_' || ch.is_ascii_alphanumeric() {
            ident.push(ch);
            continue;
        }
        flush_ident(&mut out, &mut ident, from, to);
        out.push(ch);
    }
    flush_ident(&mut out, &mut ident, from, to);
    out
}

fn count_code_braces(line: &str) -> (i32, i32) {
    let mut opens = 0;
    let mut closes = 0;
    let mut in_string = false;
    let mut in_char = false;
    let mut escaped = false;
    for ch in line.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && (in_string || in_char) {
            escaped = true;
            continue;
        }
        if ch == '"' && !in_char {
            in_string = !in_string;
            continue;
        }
        if ch == '\'' && !in_string {
            in_char = !in_char;
            continue;
        }
        if in_string || in_char {
            continue;
        }
        match ch {
            '{' => opens += 1,
            '}' => closes += 1,
            _ => {}
        }
    }
    (opens, closes)
}

fn flush_ident(out: &mut String, ident: &mut String, from: &str, to: &str) {
    if ident.is_empty() {
        return;
    }
    if ident == from {
        out.push_str(to);
    } else {
        out.push_str(ident);
    }
    ident.clear();
}
