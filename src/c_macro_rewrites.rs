use std::collections::{BTreeMap, BTreeSet, HashSet};

pub fn expand_object_like_macros(source: &str) -> String {
    let source = splice_escaped_newlines(source);
    let mut defines = Defines::default();
    let mut conditionals = Vec::new();
    let mut out = String::new();
    let mut code = String::new();
    for line in source.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix('#') {
            flush_code(&mut out, &mut code, &defines);
            handle_directive(
                rest.trim_start(),
                line,
                &mut defines,
                &mut conditionals,
                &mut out,
            );
            continue;
        }
        if conditionals_active(&conditionals) {
            code.push_str(line);
            code.push('\n');
        }
    }
    flush_code(&mut out, &mut code, &defines);
    out
}

fn splice_escaped_newlines(source: &str) -> String {
    let mut out = String::new();
    let mut chars = source.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' && chars.peek() == Some(&'\n') {
            chars.next();
        } else {
            out.push(ch);
        }
    }
    out
}

fn flush_code(out: &mut String, code: &mut String, defines: &Defines) {
    if code.is_empty() {
        return;
    }
    out.push_str(&expand_line(code, defines));
    code.clear();
}

struct Defines {
    names: BTreeSet<String>,
    objects: BTreeMap<String, String>,
    functions: BTreeMap<String, FunctionMacro>,
}

impl Default for Defines {
    fn default() -> Self {
        let mut defines = Self {
            names: BTreeSet::new(),
            objects: BTreeMap::new(),
            functions: BTreeMap::new(),
        };
        for (name, value) in [
            ("CHAR_BIT", "8"),
            ("SCHAR_MAX", "127"),
            ("SCHAR_MIN", "-128"),
            ("UCHAR_MAX", "255"),
            ("SHRT_MAX", "32767"),
            ("SHRT_MIN", "-32768"),
            ("USHRT_MAX", "65535"),
            ("INT_MAX", "2147483647"),
            ("INT_MIN", "-2147483648"),
            ("UINT_MAX", "4294967295"),
            ("LONG_MAX", "9223372036854775807"),
            ("LONG_MIN", "(-9223372036854775807 - 1)"),
            ("ULONG_MAX", "9223372036854775807"),
            ("LLONG_MAX", "9223372036854775807"),
            ("LLONG_MIN", "(-9223372036854775807 - 1)"),
            ("ULLONG_MAX", "9223372036854775807"),
            ("FLT_MANT_DIG", "24"),
            ("FLT_DIG", "6"),
            ("FLT_MAX_10_EXP", "38"),
            ("DBL_MANT_DIG", "53"),
            ("DBL_DIG", "15"),
            ("DBL_MAX_10_EXP", "308"),
            ("LDBL_MANT_DIG", "64"),
            ("LDBL_DIG", "18"),
            ("LDBL_MAX_10_EXP", "4932"),
        ] {
            defines.names.insert(name.to_string());
            defines.objects.insert(name.to_string(), value.to_string());
        }
        defines
    }
}

#[derive(Debug, Clone)]
struct FunctionMacro {
    params: Vec<String>,
    variadic: bool,
    body: String,
}

#[derive(Debug, Clone)]
struct Conditional {
    parent_active: bool,
    active: bool,
    branch_taken: bool,
}

fn handle_directive(
    directive: &str,
    original_line: &str,
    defines: &mut Defines,
    conditionals: &mut Vec<Conditional>,
    out: &mut String,
) {
    if let Some(rest) = directive.strip_prefix("ifdef") {
        push_conditional(conditionals, defines.names.contains(rest.trim()));
        return;
    }
    if let Some(rest) = directive.strip_prefix("ifndef") {
        push_conditional(conditionals, !defines.names.contains(rest.trim()));
        return;
    }
    if let Some(rest) = directive.strip_prefix("if") {
        push_conditional(conditionals, eval_if_expression(rest, defines) != 0);
        return;
    }
    if let Some(rest) = directive.strip_prefix("elif") {
        let parent_active = conditionals
            .last()
            .map(|frame| frame.parent_active)
            .unwrap_or(true);
        let should_eval = conditionals
            .last()
            .is_some_and(|frame| frame.parent_active && !frame.branch_taken);
        let cond = should_eval && eval_if_expression(rest, defines) != 0;
        if let Some(frame) = conditionals.last_mut() {
            frame.active = parent_active && cond;
            frame.branch_taken |= cond;
        }
        return;
    }
    if directive.starts_with("else") {
        if let Some(frame) = conditionals.last_mut() {
            frame.active = frame.parent_active && !frame.branch_taken;
            frame.branch_taken = true;
        }
        return;
    }
    if directive.starts_with("endif") {
        conditionals.pop();
        return;
    }

    if !conditionals_active(conditionals) {
        return;
    }
    if let Some(rest) = directive.strip_prefix("define") {
        define_macro(rest, defines);
        out.push_str(original_line);
        out.push('\n');
    } else if let Some(rest) = directive.strip_prefix("undef") {
        let name = rest.trim();
        defines.names.remove(name);
        defines.objects.remove(name);
    } else {
        out.push_str(original_line);
        out.push('\n');
    }
}

fn push_conditional(conditionals: &mut Vec<Conditional>, cond: bool) {
    let parent_active = conditionals_active(conditionals);
    conditionals.push(Conditional {
        parent_active,
        active: parent_active && cond,
        branch_taken: cond,
    });
}

fn conditionals_active(conditionals: &[Conditional]) -> bool {
    conditionals
        .last()
        .map(|frame| frame.active)
        .unwrap_or(true)
}

fn define_macro(rest: &str, defines: &mut Defines) {
    let rest = rest.trim_start();
    let Some((name, after_name)) = split_macro_name(rest) else {
        return;
    };
    defines.names.insert(name.to_string());
    if after_name.starts_with('(') {
        if let Some(function_macro) = parse_function_macro(name, after_name) {
            defines.functions.insert(name.to_string(), function_macro);
        } else {
            defines.functions.remove(name);
        }
        defines.objects.remove(name);
        return;
    }
    let replacement = strip_trailing_line_comment(after_name.trim()).trim();
    if replacement.is_empty() {
        defines.objects.insert(name.to_string(), String::new());
        defines.functions.remove(name);
        return;
    }
    if !is_expandable_replacement(replacement) {
        defines.objects.remove(name);
        return;
    }
    defines
        .objects
        .insert(name.to_string(), replacement.to_string());
    defines.functions.remove(name);
}

fn strip_trailing_line_comment(text: &str) -> &str {
    let mut in_string = false;
    let mut in_char = false;
    let mut escaped = false;
    let mut prev_slash = false;
    for (idx, ch) in text.char_indices() {
        if in_string || in_char {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if in_string && ch == '"' {
                in_string = false;
            } else if in_char && ch == '\'' {
                in_char = false;
            }
            prev_slash = false;
            continue;
        }
        if prev_slash {
            if ch == '/' {
                return text[..idx - 1].trim_end();
            }
            prev_slash = false;
        }
        match ch {
            '"' => in_string = true,
            '\'' => in_char = true,
            '/' => prev_slash = true,
            _ => {}
        }
    }
    text
}

fn parse_function_macro(name: &str, text: &str) -> Option<FunctionMacro> {
    let close = text.find(')')?;
    let mut params = text[1..close]
        .split(',')
        .map(str::trim)
        .filter(|param| !param.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let variadic = params.last().is_some_and(|param| param == "...");
    if variadic {
        *params.last_mut()? = "__VA_ARGS__".to_string();
    }
    if params.iter().any(|param| !is_identifier(param)) {
        return None;
    }
    let body = text[close + 1..].trim();
    if !is_expandable_function_replacement(name, &params, body) {
        return None;
    }
    Some(FunctionMacro {
        params,
        variadic,
        body: body.to_string(),
    })
}

fn split_macro_name(text: &str) -> Option<(&str, &str)> {
    let mut end = 0usize;
    for (idx, ch) in text.char_indices() {
        if idx == 0 && !(ch == '_' || ch.is_ascii_alphabetic()) {
            return None;
        }
        if !(ch == '_' || ch.is_ascii_alphanumeric()) {
            break;
        }
        end = idx + ch.len_utf8();
    }
    if end == 0 {
        None
    } else {
        Some((&text[..end], &text[end..]))
    }
}

fn is_expandable_replacement(text: &str) -> bool {
    is_expandable_replacement_with_hash(text, false)
}

fn is_expandable_function_replacement(name: &str, params: &[String], text: &str) -> bool {
    if is_queue_macro_name(name)
        || text.contains("struct ")
        || text.contains("do ")
        || (text.contains("sizeof")
            && name != "l_numbits"
            && !has_only_scalar_or_param_sizeofs(text, params))
    {
        return false;
    }
    is_expandable_replacement_with_hash(text, true)
}

fn has_only_scalar_or_param_sizeofs(text: &str, params: &[String]) -> bool {
    let mut rest = text;
    while let Some(pos) = rest.find("sizeof") {
        let after = rest[pos + "sizeof".len()..].trim_start();
        let (ty, next_rest) = if let Some(after) = after.strip_prefix('(') {
            let Some(close) = matching_sizeof_close(after) else {
                return false;
            };
            (after[..close].trim(), &after[close + 1..])
        } else {
            let end = after
                .char_indices()
                .find_map(|(idx, ch)| {
                    (!matches!(ch, '_' | '*' | '[' | ']' | '0') && !ch.is_ascii_alphanumeric())
                        .then_some(idx)
                })
                .unwrap_or(after.len());
            (after[..end].trim(), &after[end..])
        };
        if !matches!(
            ty,
            "char"
                | "signed char"
                | "unsigned char"
                | "short"
                | "short int"
                | "unsigned short"
                | "unsigned short int"
                | "int"
                | "unsigned"
                | "unsigned int"
                | "long"
                | "long int"
                | "unsigned long"
                | "unsigned long int"
                | "long long"
                | "long long int"
                | "unsigned long long"
                | "unsigned long long int"
                | "float"
                | "double"
        ) && !params.iter().any(|param| param == ty)
            && !sizeof_deref_param(ty, params)
            && !sizeof_index_param(ty, params)
            && !sizeof_named_type(ty)
        {
            return false;
        }
        rest = next_rest;
    }
    true
}

fn matching_sizeof_close(text: &str) -> Option<usize> {
    let mut depth = 0i64;
    for (idx, ch) in text.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' if depth == 0 => return Some(idx),
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
}

fn sizeof_deref_param(text: &str, params: &[String]) -> bool {
    let text = text.trim();
    let Some(rest) = text.strip_prefix('*') else {
        return false;
    };
    let rest = rest.trim();
    let inner = rest
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
        .unwrap_or(rest)
        .trim();
    params.iter().any(|param| param == inner)
}

fn sizeof_index_param(text: &str, params: &[String]) -> bool {
    let text = text.trim();
    let Some((base, index)) = text.rsplit_once('[') else {
        return false;
    };
    if index.trim() != "0]" {
        return false;
    }
    let base = base
        .trim()
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
        .unwrap_or_else(|| base.trim())
        .trim();
    params.iter().any(|param| param == base)
}

fn sizeof_named_type(text: &str) -> bool {
    let mut saw_ident = false;
    for part in text.split_whitespace() {
        let name = part.trim_matches('*');
        if name.is_empty() {
            continue;
        }
        let mut chars = name.chars();
        let Some(first) = chars.next() else {
            continue;
        };
        if !(first == '_' || first.is_ascii_alphabetic()) {
            return false;
        }
        if !chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric()) {
            return false;
        }
        saw_ident = true;
    }
    saw_ident
}

fn is_queue_macro_name(name: &str) -> bool {
    name.starts_with("TAILQ_") || name.starts_with("SLIST_") || name.starts_with("LIST_")
}

fn is_expandable_replacement_with_hash(text: &str, allow_hash: bool) -> bool {
    let trimmed = text.trim();
    if trimmed.contains("##") && !allow_hash {
        return false;
    }
    let mut chars = trimmed.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '"' {
            let mut escaped = false;
            let mut closed = false;
            for inner in chars.by_ref() {
                if escaped {
                    escaped = false;
                } else if inner == '\\' {
                    escaped = true;
                } else if inner == '"' {
                    closed = true;
                    break;
                }
            }
            if !closed {
                return false;
            }
            continue;
        }
        if ch == '\'' {
            let mut escaped = false;
            let mut closed = false;
            for inner in chars.by_ref() {
                if escaped {
                    escaped = false;
                } else if inner == '\\' {
                    escaped = true;
                } else if inner == '\'' {
                    closed = true;
                    break;
                }
            }
            if !closed {
                return false;
            }
            continue;
        }
        if !(ch.is_ascii_alphanumeric()
            || matches!(
                ch,
                '_' | ' '
                    | '\t'
                    | '"'
                    | '\''
                    | '('
                    | ')'
                    | '['
                    | ']'
                    | '{'
                    | '}'
                    | '+'
                    | '-'
                    | '*'
                    | '/'
                    | '%'
                    | '<'
                    | '>'
                    | '='
                    | '!'
                    | '&'
                    | '|'
                    | '^'
                    | '~'
                    | '?'
                    | ','
                    | ':'
                    | '.'
                    | ';'
            ))
            && !(allow_hash && ch == '#')
        {
            return false;
        }
    }
    true
}

fn expand_line(line: &str, defines: &Defines) -> String {
    let mut current = line.to_string();
    for _ in 0..64 {
        let (expanded, changed) = expand_line_once(&current, defines);
        current = expanded;
        if !changed {
            break;
        }
    }
    current
}

fn expand_line_once(line: &str, defines: &Defines) -> (String, bool) {
    let mut out = String::new();
    let mut changed = false;
    let mut skipped = HashSet::new();
    let mut pos = 0usize;
    while pos < line.len() {
        let Some(ch) = line[pos..].chars().next() else {
            break;
        };
        if ch == '"' {
            pos = copy_quoted_at(line, pos, '"', &mut out);
            continue;
        }
        if ch == '\'' {
            pos = copy_quoted_at(line, pos, '\'', &mut out);
            continue;
        }
        if ch == '_' || ch.is_ascii_alphabetic() {
            let end = read_ident_end(line, pos);
            let ident = &line[pos..end];
            if let Some(function_macro) = defines.functions.get(ident)
                && let Some((args, call_end)) = parse_macro_call_args(line, end)
            {
                out.push_str(&expand_function_macro(function_macro, &args, defines));
                changed = true;
                pos = call_end;
                continue;
            }
            if let Some(replacement) = defines.objects.get(ident) {
                if !skipped.contains(ident) && !ident_appears_in_replacement(ident, replacement) {
                    out.push_str(replacement);
                    changed = true;
                } else {
                    skipped.insert(ident.to_string());
                    out.push_str(ident);
                }
            } else {
                out.push_str(ident);
            }
            pos = end;
            continue;
        }
        out.push(ch);
        pos += ch.len_utf8();
    }
    (out, changed)
}

fn copy_quoted_at(line: &str, start: usize, quote: char, out: &mut String) -> usize {
    out.push(quote);
    let mut pos = start + quote.len_utf8();
    let mut escaped = false;
    while pos < line.len() {
        let Some(ch) = line[pos..].chars().next() else {
            break;
        };
        out.push(ch);
        pos += ch.len_utf8();
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == quote {
            return pos;
        }
    }
    pos
}

fn read_ident_end(text: &str, start: usize) -> usize {
    let mut end = start;
    for (rel, ch) in text[start..].char_indices() {
        if rel == 0 {
            end = start + ch.len_utf8();
            continue;
        }
        if ch == '_' || ch.is_ascii_alphanumeric() {
            end = start + rel + ch.len_utf8();
        } else {
            break;
        }
    }
    end
}

fn parse_macro_call_args(line: &str, after_name: usize) -> Option<(Vec<String>, usize)> {
    let open = skip_ws(line, after_name);
    if !line[open..].starts_with('(') {
        return None;
    }
    let mut args = Vec::new();
    let mut depth = 0i64;
    let mut arg_start = open + 1;
    let mut pos = open;
    let mut in_string = false;
    let mut in_char = false;
    let mut escaped = false;
    while pos < line.len() {
        let ch = line[pos..].chars().next()?;
        if in_string || in_char {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if in_string && ch == '"' {
                in_string = false;
            } else if in_char && ch == '\'' {
                in_char = false;
            }
            pos += ch.len_utf8();
            continue;
        }
        match ch {
            '"' => in_string = true,
            '\'' => in_char = true,
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    if !(args.is_empty() && line[arg_start..pos].trim().is_empty()) {
                        args.push(line[arg_start..pos].trim().to_string());
                    }
                    return Some((args, pos + 1));
                }
            }
            ',' if depth == 1 => {
                args.push(line[arg_start..pos].trim().to_string());
                arg_start = pos + 1;
            }
            _ => {}
        }
        pos += ch.len_utf8();
    }
    None
}

fn skip_ws(text: &str, start: usize) -> usize {
    let mut pos = start;
    while pos < text.len() {
        let Some(ch) = text[pos..].chars().next() else {
            break;
        };
        if !ch.is_ascii_whitespace() {
            break;
        }
        pos += ch.len_utf8();
    }
    pos
}

fn expand_function_macro(
    function_macro: &FunctionMacro,
    args: &[String],
    defines: &Defines,
) -> String {
    if (!function_macro.variadic && args.len() != function_macro.params.len())
        || (function_macro.variadic && args.len() < function_macro.params.len() - 1)
    {
        return String::new();
    }
    let raw_args = if function_macro.variadic {
        let fixed = function_macro.params.len() - 1;
        let mut normalized = args[..fixed].to_vec();
        normalized.push(args[fixed..].join(", "));
        normalized
    } else {
        args.to_vec()
    };
    let expanded_args = raw_args
        .iter()
        .map(|arg| expand_line(arg, defines))
        .collect::<Vec<_>>();
    substitute_function_params(
        &function_macro.body,
        &function_macro.params,
        &raw_args,
        &expanded_args,
    )
}

fn substitute_function_params(
    source: &str,
    params: &[String],
    raw_args: &[String],
    expanded_args: &[String],
) -> String {
    let source = expand_token_pastes(source, params, raw_args);
    let mut out = String::new();
    let mut pos = 0usize;
    while pos < source.len() {
        let Some(ch) = source[pos..].chars().next() else {
            break;
        };
        if ch == '"' {
            pos = copy_quoted_at(&source, pos, '"', &mut out);
            continue;
        }
        if ch == '\'' {
            pos = copy_quoted_at(&source, pos, '\'', &mut out);
            continue;
        }
        if ch == '#' {
            let after_hash = skip_ws(&source, pos + 1);
            if let Some((idx, end)) = param_at(&source, after_hash, params) {
                out.push_str(&quote_c_string(raw_args[idx].trim()));
                pos = end;
                continue;
            }
        }
        if ch == '_' || ch.is_ascii_alphabetic() {
            let end = read_ident_end(&source, pos);
            if let Some(idx) = params.iter().position(|param| param == &source[pos..end]) {
                out.push_str(&expanded_args[idx]);
            } else {
                out.push_str(&source[pos..end]);
            }
            pos = end;
            continue;
        }
        out.push(ch);
        pos += ch.len_utf8();
    }
    out
}

fn expand_token_pastes(source: &str, params: &[String], raw_args: &[String]) -> String {
    let mut out = String::new();
    let mut pos = 0usize;
    while pos < source.len() {
        let Some(ch) = source[pos..].chars().next() else {
            break;
        };
        if ch == '"' {
            pos = copy_quoted_at(source, pos, '"', &mut out);
            continue;
        }
        if ch == '\'' {
            pos = copy_quoted_at(source, pos, '\'', &mut out);
            continue;
        }
        if source[pos..].starts_with("##") {
            trim_trailing_ws(&mut out);
            pos = skip_ws(source, pos + 2);
            let (token, end) = read_paste_operand(source, pos, params, raw_args);
            out.push_str(&token);
            pos = end;
            continue;
        }
        if ch == '_' || ch.is_ascii_alphabetic() {
            let end = read_ident_end(source, pos);
            let ident = &source[pos..end];
            let after_ident = skip_ws(source, end);
            if source[after_ident..].starts_with("##") {
                if let Some(idx) = params.iter().position(|param| param == ident) {
                    out.push_str(raw_args[idx].trim());
                } else {
                    out.push_str(ident);
                }
                pos = after_ident;
                continue;
            }
            out.push_str(ident);
            pos = end;
            continue;
        }
        out.push(ch);
        pos += ch.len_utf8();
    }
    out
}

fn read_paste_operand(
    source: &str,
    pos: usize,
    params: &[String],
    raw_args: &[String],
) -> (String, usize) {
    let Some(ch) = source[pos..].chars().next() else {
        return (String::new(), pos);
    };
    if ch == '_' || ch.is_ascii_alphabetic() {
        let end = read_ident_end(source, pos);
        let ident = &source[pos..end];
        if let Some(idx) = params.iter().position(|param| param == ident) {
            (raw_args[idx].trim().to_string(), end)
        } else {
            (ident.to_string(), end)
        }
    } else {
        (ch.to_string(), pos + ch.len_utf8())
    }
}

fn trim_trailing_ws(text: &mut String) {
    while text.chars().next_back().is_some_and(char::is_whitespace) {
        text.pop();
    }
}

fn param_at(source: &str, start: usize, params: &[String]) -> Option<(usize, usize)> {
    params.iter().enumerate().find_map(|(idx, param)| {
        let end = start + param.len();
        (source[start..].starts_with(param)
            && source[end..]
                .chars()
                .next()
                .is_none_or(|next| !(next == '_' || next.is_ascii_alphanumeric())))
        .then_some((idx, end))
    })
}

fn quote_c_string(text: &str) -> String {
    let mut out = String::from("\"");
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn ident_appears_in_replacement(ident: &str, replacement: &str) -> bool {
    let mut chars = replacement.char_indices().peekable();
    while let Some((idx, ch)) = chars.next() {
        if ch == '"' || ch == '\'' {
            let quote = ch;
            let mut escaped = false;
            for (_, inner) in chars.by_ref() {
                if escaped {
                    escaped = false;
                } else if inner == '\\' {
                    escaped = true;
                } else if inner == quote {
                    break;
                }
            }
            continue;
        }
        if ch == '_' || ch.is_ascii_alphabetic() {
            let mut end = idx + ch.len_utf8();
            while let Some((next_idx, next)) = chars.peek().copied() {
                if next == '_' || next.is_ascii_alphanumeric() {
                    chars.next();
                    end = next_idx + next.len_utf8();
                } else {
                    break;
                }
            }
            if &replacement[idx..end] == ident {
                return true;
            }
        }
    }
    false
}

fn eval_if_expression(expr: &str, defines: &Defines) -> i64 {
    IfParser::new(expr, defines, 0).parse()
}

#[derive(Debug, Clone, PartialEq)]
enum IfToken {
    Ident(String),
    Num(i64),
    LParen,
    RParen,
    Bang,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Shl,
    Shr,
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Ne,
    Amp,
    Caret,
    Pipe,
    AndAnd,
    OrOr,
    Eof,
}

struct IfParser<'a> {
    tokens: Vec<IfToken>,
    pos: usize,
    defines: &'a Defines,
    depth: usize,
}

impl<'a> IfParser<'a> {
    fn new(expr: &str, defines: &'a Defines, depth: usize) -> Self {
        Self {
            tokens: lex_if_tokens(expr),
            pos: 0,
            defines,
            depth,
        }
    }

    fn parse(&mut self) -> i64 {
        self.parse_or()
    }

    fn parse_or(&mut self) -> i64 {
        let mut value = self.parse_and();
        while self.take(&IfToken::OrOr) {
            let rhs = self.parse_and();
            value = i64::from(value != 0 || rhs != 0);
        }
        value
    }

    fn parse_and(&mut self) -> i64 {
        let mut value = self.parse_bit_or();
        while self.take(&IfToken::AndAnd) {
            let rhs = self.parse_bit_or();
            value = i64::from(value != 0 && rhs != 0);
        }
        value
    }

    fn parse_bit_or(&mut self) -> i64 {
        let mut value = self.parse_bit_xor();
        while self.take(&IfToken::Pipe) {
            value |= self.parse_bit_xor();
        }
        value
    }

    fn parse_bit_xor(&mut self) -> i64 {
        let mut value = self.parse_bit_and();
        while self.take(&IfToken::Caret) {
            value ^= self.parse_bit_and();
        }
        value
    }

    fn parse_bit_and(&mut self) -> i64 {
        let mut value = self.parse_equality();
        while self.take(&IfToken::Amp) {
            value &= self.parse_equality();
        }
        value
    }

    fn parse_equality(&mut self) -> i64 {
        let mut value = self.parse_relational();
        loop {
            if self.take(&IfToken::Eq) {
                value = i64::from(value == self.parse_relational());
            } else if self.take(&IfToken::Ne) {
                value = i64::from(value != self.parse_relational());
            } else {
                return value;
            }
        }
    }

    fn parse_relational(&mut self) -> i64 {
        let mut value = self.parse_shift();
        loop {
            if self.take(&IfToken::Lt) {
                value = i64::from(value < self.parse_shift());
            } else if self.take(&IfToken::Gt) {
                value = i64::from(value > self.parse_shift());
            } else if self.take(&IfToken::Le) {
                value = i64::from(value <= self.parse_shift());
            } else if self.take(&IfToken::Ge) {
                value = i64::from(value >= self.parse_shift());
            } else {
                return value;
            }
        }
    }

    fn parse_shift(&mut self) -> i64 {
        let mut value = self.parse_add();
        loop {
            if self.take(&IfToken::Shl) {
                value <<= self.parse_add().clamp(0, 63);
            } else if self.take(&IfToken::Shr) {
                value >>= self.parse_add().clamp(0, 63);
            } else {
                return value;
            }
        }
    }

    fn parse_add(&mut self) -> i64 {
        let mut value = self.parse_mul();
        loop {
            if self.take(&IfToken::Plus) {
                value = value.saturating_add(self.parse_mul());
            } else if self.take(&IfToken::Minus) {
                value = value.saturating_sub(self.parse_mul());
            } else {
                return value;
            }
        }
    }

    fn parse_mul(&mut self) -> i64 {
        let mut value = self.parse_unary();
        loop {
            if self.take(&IfToken::Star) {
                value = value.saturating_mul(self.parse_unary());
            } else if self.take(&IfToken::Slash) {
                let rhs = self.parse_unary();
                value = if rhs == 0 { 0 } else { value / rhs };
            } else if self.take(&IfToken::Percent) {
                let rhs = self.parse_unary();
                value = if rhs == 0 { 0 } else { value % rhs };
            } else {
                return value;
            }
        }
    }

    fn parse_unary(&mut self) -> i64 {
        if self.take(&IfToken::Bang) {
            i64::from(self.parse_unary() == 0)
        } else if self.take(&IfToken::Minus) {
            -self.parse_unary()
        } else if self.take(&IfToken::Plus) {
            self.parse_unary()
        } else if self.peek_defined() {
            self.pos += 1;
            self.parse_defined_operand()
        } else {
            self.parse_primary()
        }
    }

    fn parse_primary(&mut self) -> i64 {
        match self.next() {
            IfToken::Num(value) => value,
            IfToken::Ident(name) => self.expand_ident(&name),
            IfToken::LParen => {
                let value = self.parse_or();
                self.take(&IfToken::RParen);
                value
            }
            _ => 0,
        }
    }

    fn parse_defined_operand(&mut self) -> i64 {
        if self.take(&IfToken::LParen) {
            let defined = match self.next() {
                IfToken::Ident(name) => self.defines.names.contains(&name),
                _ => false,
            };
            self.take(&IfToken::RParen);
            i64::from(defined)
        } else {
            let defined = match self.next() {
                IfToken::Ident(name) => self.defines.names.contains(&name),
                _ => false,
            };
            i64::from(defined)
        }
    }

    fn expand_ident(&self, name: &str) -> i64 {
        if self.depth >= 16 {
            return 0;
        }
        let Some(replacement) = self.defines.objects.get(name) else {
            return 0;
        };
        IfParser::new(replacement, self.defines, self.depth + 1).parse()
    }

    fn peek_defined(&self) -> bool {
        matches!(self.tokens.get(self.pos), Some(IfToken::Ident(name)) if name == "defined")
    }

    fn take(&mut self, expected: &IfToken) -> bool {
        if self.tokens.get(self.pos) == Some(expected) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn next(&mut self) -> IfToken {
        let token = self.tokens.get(self.pos).cloned().unwrap_or(IfToken::Eof);
        self.pos += 1;
        token
    }
}

fn lex_if_tokens(expr: &str) -> Vec<IfToken> {
    let mut tokens = Vec::new();
    let mut chars = expr.char_indices().peekable();
    while let Some((idx, ch)) = chars.next() {
        if ch.is_ascii_whitespace() {
            continue;
        }
        if ch == '_' || ch.is_ascii_alphabetic() {
            let mut end = idx + ch.len_utf8();
            while let Some((next_idx, next)) = chars.peek().copied() {
                if next == '_' || next.is_ascii_alphanumeric() {
                    chars.next();
                    end = next_idx + next.len_utf8();
                } else {
                    break;
                }
            }
            tokens.push(IfToken::Ident(expr[idx..end].to_string()));
            continue;
        }
        if ch.is_ascii_digit() {
            let mut end = idx + ch.len_utf8();
            while let Some((next_idx, next)) = chars.peek().copied() {
                if next.is_ascii_hexdigit() || matches!(next, 'x' | 'X' | 'u' | 'U' | 'l' | 'L') {
                    chars.next();
                    end = next_idx + next.len_utf8();
                } else {
                    break;
                }
            }
            tokens.push(IfToken::Num(parse_if_number(&expr[idx..end])));
            continue;
        }
        let next = chars.peek().map(|(_, next)| *next);
        let token = match (ch, next) {
            ('&', Some('&')) => {
                chars.next();
                IfToken::AndAnd
            }
            ('|', Some('|')) => {
                chars.next();
                IfToken::OrOr
            }
            ('=', Some('=')) => {
                chars.next();
                IfToken::Eq
            }
            ('!', Some('=')) => {
                chars.next();
                IfToken::Ne
            }
            ('<', Some('=')) => {
                chars.next();
                IfToken::Le
            }
            ('>', Some('=')) => {
                chars.next();
                IfToken::Ge
            }
            ('<', Some('<')) => {
                chars.next();
                IfToken::Shl
            }
            ('>', Some('>')) => {
                chars.next();
                IfToken::Shr
            }
            ('(', _) => IfToken::LParen,
            (')', _) => IfToken::RParen,
            ('!', _) => IfToken::Bang,
            ('+', _) => IfToken::Plus,
            ('-', _) => IfToken::Minus,
            ('*', _) => IfToken::Star,
            ('/', _) => IfToken::Slash,
            ('%', _) => IfToken::Percent,
            ('<', _) => IfToken::Lt,
            ('>', _) => IfToken::Gt,
            ('&', _) => IfToken::Amp,
            ('^', _) => IfToken::Caret,
            ('|', _) => IfToken::Pipe,
            _ => continue,
        };
        tokens.push(token);
    }
    tokens.push(IfToken::Eof);
    tokens
}

fn parse_if_number(text: &str) -> i64 {
    let text = text.trim_end_matches(|ch: char| matches!(ch, 'u' | 'U' | 'l' | 'L'));
    if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        i64::from_str_radix(hex, 16).unwrap_or(0)
    } else {
        text.parse().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::expand_object_like_macros;

    #[test]
    fn expands_simple_object_macros_in_code_only() {
        let source = r#"
#define NAME "lua"
#define OTHER NAME "!"
#define call(x) x
static const char *progname = NAME;
static const char *full = OTHER;
static const char *literal = "NAME";
int value = call(NAME);
"#;
        let out = expand_object_like_macros(source);
        assert!(out.contains("static const char *progname = \"lua\";"));
        assert!(out.contains("static const char *full = \"lua\" \"!\";"));
        assert!(out.contains("static const char *literal = \"NAME\";"));
        assert!(out.contains("int value = \"lua\";"));
        assert!(out.contains("#define NAME \"lua\""));
    }

    #[test]
    fn expands_empty_object_macros() {
        let source = r#"
#define API
API int exported(void);
"#;
        let out = expand_object_like_macros(source);
        assert!(out.contains(" int exported(void);"), "{out}");
        assert!(out.contains("#define API"), "{out}");
    }

    #[test]
    fn strips_line_comments_from_object_macro_replacements() {
        let source = r#"
#define LEN 16 // bytes
#define URL "http://example"
int data[LEN];
char *url = URL;
"#;
        let out = expand_object_like_macros(source);
        assert!(out.contains("int data[16];"), "{out}");
        assert!(out.contains("char *url = \"http://example\";"), "{out}");
    }

    #[test]
    fn keeps_only_active_simple_preprocessor_branches() {
        let source = r#"
#define HAVE_FEATURE 1
#if defined(HAVE_FEATURE) && HAVE_FEATURE == 1
int active = NAME;
#else
int inactive = MISSING;
#endif
#if defined(MISSING)
int also_inactive = 1;
#elif HAVE_FEATURE
#define NAME 42
int from_elif = NAME;
#endif
"#;
        let out = expand_object_like_macros(source);
        assert!(out.contains("int active = NAME;"));
        assert!(!out.contains("int inactive"));
        assert!(!out.contains("int also_inactive"));
        assert!(out.contains("int from_elif = 42;"));
    }

    #[test]
    fn expands_no_argument_function_macros() {
        let source = r#"
#define enabled() 1
int value = enabled();
"#;
        let out = expand_object_like_macros(source);
        assert!(out.contains("int value = 1;"));
    }

    #[test]
    fn leaves_structural_function_macros_for_later_rewrites() {
        let source = r#"
#define TAILQ_FIRST(head) ((head)->tqh_first)
int value = TAILQ_FIRST(&head);
"#;
        let out = expand_object_like_macros(source);
        assert!(out.contains("int value = TAILQ_FIRST(&head);"));
    }

    #[test]
    fn leaves_generic_sizeof_function_macros_for_later_rewrites() {
        let source = r#"
#define LEN(x) (sizeof (x) / sizeof *(x))
int value = LEN(items);
"#;
        let out = expand_object_like_macros(source);
        assert!(out.contains("int value = LEN(items);"));
    }

    #[test]
    fn expands_function_macros_with_only_scalar_sizeofs() {
        let source = r#"
#define write(s,l) fwrite((s), sizeof(char), (l), stdout)
int main() { write("x", 1); }
"#;
        let out = expand_object_like_macros(source);
        assert!(
            out.contains(r#"int main() { fwrite(("x"), sizeof(char), (1), stdout); }"#),
            "{out}"
        );
    }

    #[test]
    fn expands_ternary_assignment_function_macros() {
        let source = r#"
#define fastget(t,k,res,f, tag) (tag = (!ttistable(t) ? NOTABLE : f(hvalue(t), k, res)))
int main() { fastget(upval, key, s2v(ra), getshort, tag); }
"#;
        let out = expand_object_like_macros(source);
        assert!(
            out.contains(
                "int main() { (tag = (!ttistable(upval) ? NOTABLE : getshort(hvalue(upval), key, s2v(ra)))); }"
            ),
            "{out}"
        );
    }

    #[test]
    fn substitutes_function_macro_params_simultaneously() {
        let source = r#"
#define s2v(o) (&(o)->val)
#define fastset(t,k,val,hres,f) (hres = (!ttistable(t) ? 2 : f(hvalue(t), k, val)))
int main() { fastset(t, s2v(a), s2v(b), hres, luaH_pset); }
"#;
        let out = expand_object_like_macros(source);
        assert!(
            out.contains(
                "int main() { (hres = (!ttistable(t) ? 2 : luaH_pset(hvalue(t), (&(a)->val), (&(b)->val)))); }"
            ),
            "{out}"
        );
        assert!(!out.contains("->(&"), "{out}");
    }

    #[test]
    fn expands_lua_numbits_sizeof_macro() {
        let source = r#"
#define l_numbits(t) (sizeof(t) * CHAR_BIT)
int value = l_numbits(int);
"#;
        let out = expand_object_like_macros(source);
        assert!(out.contains("int value = (sizeof(int) * 8);"), "{out}");
    }

    #[test]
    fn expands_sizeof_macro_parameters() {
        let source = r#"
#define grow(v,t,n) ((v) = alloc(sizeof(t) * (n)))
int main() { int p; grow(p, int, 4); }
"#;
        let out = expand_object_like_macros(source);
        assert!(
            out.contains("int main() { int p; ((p) = alloc(sizeof(int) * (4))); }"),
            "{out}"
        );
    }

    #[test]
    fn expands_unparenthesized_sizeof_macro_parameters() {
        let source = r#"
#define copy(s) memcpy(buf, s, sizeof s)
int main() { copy("abc"); }
"#;
        let out = expand_object_like_macros(source);
        assert!(
            out.contains("memcpy(buf, \"abc\", sizeof \"abc\")"),
            "{out}"
        );
    }

    #[test]
    fn expands_sizeof_dereferenced_macro_parameters() {
        let source = r#"
#define freearray(p,n) freebytes((p), (n) * sizeof(*(p)))
int main() { freearray(items, count); }
"#;
        let out = expand_object_like_macros(source);
        assert!(
            out.contains("int main() { freebytes((items), (count) * sizeof(*(items))); }"),
            "{out}"
        );
    }

    #[test]
    fn expands_sizeof_indexed_macro_parameters() {
        let source = r#"
#define dumpVector(v,n) dumpBlock(v, (n) * sizeof((v)[0]))
int main() { dumpVector(items, count); }
"#;
        let out = expand_object_like_macros(source);
        assert!(
            out.contains("int main() { dumpBlock(items, (count) * sizeof((items)[0])); }"),
            "{out}"
        );
    }

    #[test]
    fn expands_sizeof_named_types_in_function_macros() {
        let source = r#"
#define closureSize(n) (offsetof(CClosure, upvalue) + sizeof(TValue) * (n) + sizeof(UpVal *))
int main() { int x = closureSize(count); }
"#;
        let out = expand_object_like_macros(source);
        assert!(
            out.contains(
                "int main() { int x = (offsetof(CClosure, upvalue) + sizeof(TValue) * (count) + sizeof(UpVal *)); }"
            ),
            "{out}"
        );
    }

    #[test]
    fn expands_token_paste_function_macros() {
        let source = r#"
#define attr(n) (DBL_##n)
#define mathop(op) op##f
int a = attr(MANT_DIG);
int b = mathop(floor)(x);
"#;
        let out = expand_object_like_macros(source);
        assert!(out.contains("int a = (53);"), "{out}");
        assert!(out.contains("int b = floorf(x);"), "{out}");
    }

    #[test]
    fn expands_variadic_function_macros() {
        let source = r#"
#define TEST(c, ...) ((c) ? 1 : (log(#c " failed: " __VA_ARGS__), 0))
int a = TEST(x == 1, "value %d\n", x);
int b = TEST(y == 2);
"#;
        let out = expand_object_like_macros(source);
        assert!(
            out.contains(
                r#"int a = ((x == 1) ? 1 : (log("x == 1" " failed: " "value %d\n", x), 0));"#
            ),
            "{out}"
        );
        assert!(
            out.contains(r#"int b = ((y == 2) ? 1 : (log("y == 2" " failed: " ), 0));"#),
            "{out}"
        );
    }

    #[test]
    fn expands_deep_object_macro_chains() {
        let source = r#"
#define A0 1
#define A1 (A0 << 1)
#define A2 (A1 << 1)
#define A3 (A2 << 1)
#define A4 (A3 << 1)
#define A5 (A4 << 1)
#define A6 (A5 << 1)
#define A7 (A6 << 1)
#define A8 (A7 << 1)
#define A9 (A8 << 1)
int x = A9;
"#;
        let out = expand_object_like_macros(source);
        let code_line = out
            .lines()
            .find(|line| line.trim_start().starts_with("int x ="))
            .unwrap_or_default();
        assert!(!code_line.contains("A"), "{out}");
        assert!(code_line.contains("int x ="), "{out}");
    }

    #[test]
    fn expands_standard_integer_limit_defaults() {
        let source = "int x = USHRT_MAX + UCHAR_MAX + SHRT_MAX;\n";
        let out = expand_object_like_macros(source);
        assert!(out.contains("int x = 65535 + 255 + 32767;"), "{out}");
    }

    #[test]
    fn expands_multiline_function_macro_calls() {
        let source = r#"
#define check(a,b,c) ((a), (b), (c))
int value = check(1,
                  2,
                  3);
"#;
        let out = expand_object_like_macros(source);
        assert!(out.contains("int value = ((1), (2), (3));"), "{out}");
    }

    #[test]
    fn expands_nested_statement_function_macros() {
        let source = r#"
#define wrap(cond,pre,pos) if (cond) { pre; value = value + 1; pos; } else { value = 0; }
#define preserve(cond,p) wrap(cond, int saved = p, p = saved)
int main() {
  int value = 0;
  int ptr = 4;
  preserve(ptr, ptr);
  return value;
}
"#;
        let out = expand_object_like_macros(source);
        assert!(
            out.contains("if (ptr) { int saved = ptr; value = value + 1; ptr = saved; }"),
            "{out}"
        );
        assert!(!out.contains("preserve(ptr, ptr);"), "{out}");
    }

    #[test]
    fn predefines_standard_integer_limits_for_conditionals() {
        let source = r#"
#if defined(LLONG_MAX)
#define LEN "ll"
#else
#define LEN ""
#endif
#define FMT "%" LEN "d"
char *fmt = FMT;
"#;
        let out = expand_object_like_macros(source);
        assert!(out.contains("char *fmt = \"%\" \"ll\" \"d\";"), "{out}");
    }

    #[test]
    fn expands_case_label_function_macros() {
        let source = r#"
#define vmcase(l) case l:
switch (op) {
  vmcase(OP_MOVE) {
    break;
  }
}
"#;
        let out = expand_object_like_macros(source);
        assert!(out.contains("case OP_MOVE:"), "{out}");
        assert!(!out.contains("  vmcase(OP_MOVE)"), "{out}");
    }

    #[test]
    fn expands_question_mark_inside_string_macros() {
        let source = r#"
#define MARK "?"
char *value = "/" MARK "!";
"#;
        let out = expand_object_like_macros(source);
        assert!(out.contains("char *value = \"/\" \"?\" \"!\";"), "{out}");
    }

    #[test]
    fn expands_hash_inside_string_macros() {
        let source = r##"
#define FLAGS "-+#0 "
char *value = FLAGS "123";
"##;
        let out = expand_object_like_macros(source);
        assert!(out.contains("char *value = \"-+#0 \" \"123\";"), "{out}");
    }

    #[test]
    fn expands_escaped_string_function_macros() {
        let source = r#"
#define write(s,l) fwrite((s), (l), stdout)
#define writeline() (write("\n", 1), fflush(stdout))
int main() { writeline(); }
"#;
        let out = expand_object_like_macros(source);
        assert!(
            out.contains("int main() { (fwrite((\"\\n\"), (1), stdout), fflush(stdout)); }"),
            "{out}"
        );
    }
}
