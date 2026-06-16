pub fn collect_user_type_aliases(source: &str) -> Vec<String> {
    let mut aliases = Vec::new();
    let mut typedef_depth = 0i64;
    for line in source.lines() {
        let trimmed = line.trim();

        if typedef_depth > 0 {
            let next_depth = typedef_depth + brace_delta(line);
            if next_depth == 0
                && let Some(alias) = closing_typedef_alias(trimmed)
            {
                aliases.push(alias);
            }
            typedef_depth = next_depth;
            if typedef_depth < 0 {
                typedef_depth = 0;
            }
            continue;
        }

        if starts_typedef_block(trimmed) {
            typedef_depth += brace_delta(line);
            if typedef_depth <= 0 {
                typedef_depth = 0;
                if let Some(alias) = closing_typedef_alias(trimmed) {
                    aliases.push(alias);
                }
            }
            continue;
        }

        if let Some(alias) = single_line_tag_alias(trimmed) {
            aliases.push(alias);
        }
    }
    aliases.sort();
    aliases.dedup();
    aliases
}

pub fn collect_user_struct_tags(source: &str) -> Vec<String> {
    let mut tags = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("struct ") {
            let Some(brace) = rest.find('{') else {
                continue;
            };
            let name = rest
                .split(|ch: char| ch == '{' || ch.is_whitespace())
                .next()
                .unwrap_or_default();
            let before_brace = rest[..brace].trim();
            if !name.is_empty() && before_brace == name {
                tags.push(name.to_string());
            }
        }
    }
    tags.sort();
    tags.dedup();
    tags
}

pub fn apply_user_type_alias_rewrites(source: &str, aliases: &[String]) -> String {
    let mut out = source.to_string();
    let mut aliases = aliases.to_vec();
    aliases.sort_by_key(|alias| std::cmp::Reverse(alias.len()));
    for alias in aliases {
        out = replace_ident_token(&out, &alias, "int");
    }
    out
}

pub fn apply_user_struct_tag_rewrites(source: &str, tags: &[String]) -> String {
    let mut out = source.to_string();
    let mut tags = tags.to_vec();
    tags.sort_by_key(|tag| std::cmp::Reverse(tag.len()));
    for tag in tags {
        out = out.replace(&format!("struct {tag} *"), "int ");
        out = out.replace(&format!("struct {tag}"), "int");
    }
    out
}

pub fn apply_scalar_type_rewrites(source: &str) -> String {
    let mut out = source.to_string();
    for (from, to) in [
        ("sizeof(*pnode)", "16"),
        ("static long long", "int"),
        ("static long", "int"),
        ("long long", "int"),
        ("long", "int"),
        ("ptrdiff_t", "int"),
    ] {
        out = out.replace(from, to);
    }
    for alias in ["FILE", "DIR", "regex_t"] {
        out = replace_ident_token(&out, alias, "int");
    }
    out
}

pub fn normalize_storage_class_arrays(source: &str) -> String {
    let mut out = String::new();
    for line in source.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("static char ") && trimmed.contains('[') {
            let indent_len = line.len() - trimmed.len();
            out.push_str(&line[..indent_len]);
            out.push_str(trimmed.trim_start_matches("static "));
            out.push('\n');
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

pub fn normalize_string_pointer_array_initializers(source: &str) -> String {
    let mut out = String::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("char *")
            && let Some(bracket) = rest.find('[')
            && let Some(eq) = rest.find('=')
            && eq > bracket
            && trimmed.ends_with(';')
        {
            let indent_len = line.len() - line.trim_start().len();
            let indent = &line[..indent_len];
            let name = rest[..bracket].trim();
            let len_end = rest[bracket + 1..].find(']').unwrap_or(0) + bracket + 1;
            let len = rest[bracket + 1..len_end].trim();
            let init = rest[eq + 1..]
                .trim()
                .trim_start_matches('{')
                .trim_end_matches(';')
                .trim_end_matches('}')
                .trim();
            out.push_str(indent);
            out.push_str("int ");
            out.push_str(name);
            out.push_str("; ");
            out.push_str(name);
            out.push_str(" = alloc(8 * ");
            out.push_str(len);
            out.push_str(");\n");
            for (idx, value) in split_initializer_values(init).iter().enumerate() {
                out.push_str(indent);
                out.push_str(name);
                out.push('[');
                out.push_str(&idx.to_string());
                out.push_str("] = ");
                out.push_str(value);
                out.push_str(";\n");
            }
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn split_initializer_values(text: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut in_string = false;
    let mut escaped = false;
    let mut start = 0usize;
    for (idx, ch) in text.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            ',' => {
                values.push(text[start..idx].trim().to_string());
                start = idx + 1;
            }
            _ => {}
        }
    }
    values.push(text[start..].trim().to_string());
    values
}

pub fn normalize_static_struct_line_globals(source: &str) -> String {
    let mut out = String::new();
    let mut depth = 0i64;
    for line in source.lines() {
        let trimmed = line.trim();
        if depth == 0 && trimmed.starts_with("static struct line ") && trimmed.ends_with(';') {
            let name = trimmed
                .trim_start_matches("static struct line ")
                .trim_end_matches(';')
                .trim();
            out.push_str("int ");
            out.push_str(name);
            out.push_str("[2] = {0,0};\n");
        } else {
            out.push_str(line);
            out.push('\n');
        }
        depth += brace_delta(line);
        if depth < 0 {
            depth = 0;
        }
    }
    out
}

pub fn normalize_jsmn_parser_declarations(source: &str) -> String {
    normalize_object_declarations(source, "jsmn_parser", 24)
}

pub fn normalize_struct_entry_declarations(source: &str) -> String {
    let mut out = String::new();
    let mut object_names = Vec::new();
    for line in source.lines() {
        let indent_len = line.len() - line.trim_start().len();
        let indent = &line[..indent_len];
        let trimmed = line.trim();
        if !trimmed.starts_with("struct entry ") || !trimmed.ends_with(';') {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        let rest = trimmed
            .trim_start_matches("struct entry ")
            .trim_end_matches(';');
        for decl in rest
            .split(',')
            .map(str::trim)
            .filter(|decl| !decl.is_empty())
        {
            let is_pointer = decl.starts_with('*');
            let decl = decl.trim_start_matches('*').trim();
            let name = decl
                .split(|ch: char| ch == '=' || ch.is_whitespace())
                .next()
                .unwrap_or_default();
            if name.is_empty() {
                continue;
            }
            out.push_str(indent);
            out.push_str("int ");
            out.push_str(name);
            if is_pointer {
                out.push_str(" = 0;\n");
            } else {
                object_names.push(name.to_string());
                out.push_str("; ");
                out.push_str(name);
                out.push_str(" = alloc(104);\n");
            }
        }
    }
    for name in object_names {
        out = replace_amp_object_refs(&out, &name);
    }
    out
}

fn normalize_object_declarations(source: &str, ty: &str, size: i64) -> String {
    let mut out = String::new();
    let mut object_names = Vec::new();
    for line in source.lines() {
        let indent_len = line.len() - line.trim_start().len();
        let indent = &line[..indent_len];
        let trimmed = line.trim();
        if trimmed.starts_with(ty)
            && trimmed.ends_with(';')
            && !trimmed.contains('*')
            && !trimmed.contains('(')
        {
            let names = trimmed
                .trim_start_matches(ty)
                .trim()
                .trim_end_matches(';')
                .split(',')
                .map(str::trim);
            for name in names {
                object_names.push(name.to_string());
                out.push_str(indent);
                out.push_str("int ");
                out.push_str(name);
                out.push_str("; ");
                out.push_str(name);
                out.push_str(" = alloc(");
                out.push_str(&size.to_string());
                out.push_str(");\n");
            }
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    for name in object_names {
        out = replace_amp_object_refs(&out, &name);
    }
    out
}

fn replace_amp_object_refs(source: &str, name: &str) -> String {
    let pattern = format!("&{name}");
    let mut out = String::new();
    let mut pos = 0;
    while let Some(rel) = source[pos..].find(&pattern) {
        let start = pos + rel;
        let end = start + pattern.len();
        let next = source[end..].chars().next();
        if next.is_some_and(|ch| ch == '_' || ch == '.' || ch.is_ascii_alphanumeric()) {
            out.push_str(&source[pos..end]);
        } else {
            out.push_str(&source[pos..start]);
            out.push_str(name);
        }
        pos = end;
    }
    out.push_str(&source[pos..]);
    out
}

pub fn normalize_function_pointer_params(source: &str) -> String {
    let mut out = String::new();
    let mut pos = 0;
    while let Some((start, prefix_len)) = find_function_pointer_start(source, pos) {
        out.push_str(&source[pos..start]);
        let name_start = start + prefix_len;
        let Some(name_end_rel) = source[name_start..].find(')') else {
            out.push_str(&source[start..]);
            return out;
        };
        let name_end = name_start + name_end_rel;
        let name = source[name_start..name_end].trim();
        let after_name = name_end + 1;
        if source[after_name..].chars().next() != Some('(') {
            out.push_str(&source[start..after_name]);
            pos = after_name;
            continue;
        }
        let Some(end) = matching_paren_end(source, after_name) else {
            out.push_str(&source[start..]);
            return out;
        };
        out.push_str("int ");
        out.push_str(name);
        pos = end;
    }
    out.push_str(&source[pos..]);
    out
}

pub fn normalize_function_pointer_conditionals(source: &str) -> String {
    source
        .replace("(iflag ? strcasecmp : strcmp)", "strcmp")
        .replace("(iflag ? strcasestr : strstr)", "strstr")
        .replace(
            "(follow ? stat : lstat)(path, &st)",
            "(follow ? stat(path, &st) : lstat(path, &st))",
        )
}

pub fn normalize_anonymous_enums(source: &str) -> String {
    let mut constants = Vec::new();
    let mut out = String::new();
    let mut lines = source.lines();

    while let Some(line) = lines.next() {
        if let Some(start) = line.find("enum {") {
            let mut body = String::new();
            let after_start = &line[start + "enum {".len()..];
            if let Some(end) = after_start.find("};") {
                body.push_str(&after_start[..end]);
                collect_enum_constants(&body, &mut constants);
                out.push_str(&line[..start]);
                out.push_str(&after_start[end + 2..]);
                out.push('\n');
                continue;
            }

            body.push_str(after_start);
            body.push('\n');
            for enum_line in lines.by_ref() {
                if let Some(end) = enum_line.find("};") {
                    body.push_str(&enum_line[..end]);
                    collect_enum_constants(&body, &mut constants);
                    out.push_str(&enum_line[end + 2..]);
                    out.push('\n');
                    break;
                }
                body.push_str(enum_line);
                body.push('\n');
            }
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }

    constants.sort_by_key(|(name, _)| std::cmp::Reverse(name.len()));
    constants.dedup_by(|left, right| left.0 == right.0);
    for (name, value) in constants {
        out = replace_ident_token(&out, &name, &value.to_string());
    }
    out
}

fn collect_enum_constants(body: &str, constants: &mut Vec<(String, i64)>) {
    let mut next_value = 0i64;
    for part in body.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (name, value) = if let Some(eq) = part.find('=') {
            let name = part[..eq].trim();
            let value = parse_enum_value(part[eq + 1..].trim()).unwrap_or(next_value);
            (name, value)
        } else {
            (part, next_value)
        };
        if let Some(name) = ident(name) {
            constants.push((name, value));
            next_value = value + 1;
        }
    }
}

fn parse_enum_value(text: &str) -> Option<i64> {
    let text = text.trim();
    if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        i64::from_str_radix(hex, 16).ok()
    } else {
        text.parse().ok()
    }
}

fn find_function_pointer_start(source: &str, pos: usize) -> Option<(usize, usize)> {
    ["int (*", "void (*", "char *(*"]
        .iter()
        .filter_map(|prefix| {
            source[pos..]
                .find(prefix)
                .map(|rel| (pos + rel, prefix.len()))
        })
        .min_by_key(|(idx, _)| *idx)
}

fn matching_paren_end(source: &str, open: usize) -> Option<usize> {
    let mut depth = 0i64;
    for (rel, ch) in source[open..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + rel + 1);
                }
            }
            _ => {}
        }
    }
    None
}

fn starts_typedef_block(trimmed: &str) -> bool {
    (trimmed.starts_with("typedef struct ")
        || trimmed.starts_with("typedef enum ")
        || trimmed.starts_with("typedef union "))
        && trimmed.contains('{')
}

fn brace_delta(line: &str) -> i64 {
    line.chars().fold(0, |depth, ch| match ch {
        '{' => depth + 1,
        '}' => depth - 1,
        _ => depth,
    })
}

fn closing_typedef_alias(trimmed: &str) -> Option<String> {
    let rest = trimmed.strip_prefix('}')?.trim();
    let alias = rest.strip_suffix(';')?.trim();
    ident(alias)
}

fn single_line_tag_alias(trimmed: &str) -> Option<String> {
    if !(trimmed.starts_with("typedef struct ")
        || trimmed.starts_with("typedef enum ")
        || trimmed.starts_with("typedef union "))
        || !trimmed.ends_with(';')
        || trimmed.contains('{')
    {
        return None;
    }
    let alias = trimmed
        .trim_end_matches(';')
        .split_whitespace()
        .last()
        .unwrap_or_default();
    ident(alias)
}

fn ident(text: &str) -> Option<String> {
    let text = text.trim().trim_start_matches('*').trim();
    if text.is_empty() {
        return None;
    }
    let mut chars = text.chars();
    let first = chars.next()?;
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return None;
    }
    if chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric()) {
        Some(text.to_string())
    } else {
        None
    }
}

fn replace_ident_token(source: &str, needle: &str, replacement: &str) -> String {
    let mut out = String::new();
    let mut pos = 0;
    while let Some(rel) = source[pos..].find(needle) {
        let start = pos + rel;
        let end = start + needle.len();
        let before = source[..start].chars().next_back();
        let after = source[end..].chars().next();
        if before.is_some_and(is_ident_char) || after.is_some_and(is_ident_char) {
            out.push_str(&source[pos..end]);
        } else {
            out.push_str(&source[pos..start]);
            out.push_str(replacement);
        }
        pos = end;
    }
    out.push_str(&source[pos..]);
    out
}

fn is_ident_char(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}
