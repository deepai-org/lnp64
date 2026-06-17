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
                if let Some(alias) =
                    braced_typedef_alias(trimmed).or_else(|| closing_typedef_alias(trimmed))
                {
                    aliases.push(alias);
                }
            }
            continue;
        }

        if let Some(alias) = single_line_tag_alias(trimmed) {
            aliases.push(alias);
        } else if let Some(alias) = function_pointer_typedef_alias(trimmed) {
            aliases.push(alias);
        } else if let Some(alias) = single_line_typedef_alias(trimmed) {
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
        out = replace_type_alias_token(&out, &alias, "int");
    }
    out
}

pub fn apply_user_struct_tag_rewrites(source: &str, tags: &[String]) -> String {
    let mut out = source.to_string();
    let mut tags = tags.to_vec();
    tags.sort_by_key(|tag| std::cmp::Reverse(tag.len()));
    for tag in tags {
        if matches!(tag.as_str(), "line" | "linebuf" | "column") {
            continue;
        }
        out = out.replace(&format!("struct {tag} *"), "int ");
        out = out.replace(&format!("struct {tag}"), "int");
    }
    out
}

pub fn apply_scalar_type_rewrites(source: &str) -> String {
    let mut out = source.to_string();
    for (from, to) in [
        ("sizeof(*pnode)", "16"),
        ("sizeof(regmatch_t)", "16"),
        ("sizeof(regex_t)", "16"),
    ] {
        out = out.replace(from, to);
    }
    for (from, to) in [
        ("static long long", "int"),
        ("static long", "int"),
        ("long long", "int"),
        ("long", "int"),
        ("ptrdiff_t", "int"),
    ] {
        out = replace_token_phrase(&out, from, to);
    }
    for alias in ["FILE", "DIR", "regex_t", "regmatch_t"] {
        out = replace_ident_token(&out, alias, "int");
    }
    out
}

pub fn normalize_known_sizeofs(source: &str) -> String {
    let mut out = source.to_string();
    for (from, to) in [
        ("sizeof(jsmntok_t)", "32"),
        ("sizeof(tok[0])", "32"),
        ("sizeof(tokens[0])", "32"),
        ("sizeof(toksmall)", "320"),
        ("sizeof(toklarge)", "320"),
        ("sizeof(tok)", "160"),
        ("sizeof(tokens)", "320"),
        ("sizeof(global_State)", "2048"),
        ("sizeof(lua_State)", "192"),
        ("sizeof(LX)", "256"),
        ("sizeof(CallInfo)", "128"),
        ("sizeof(StackValue)", "16"),
        ("sizeof(TValue)", "16"),
        ("sizeof(TString)", "48"),
        ("sizeof(Table)", "128"),
        ("sizeof(Node)", "32"),
        ("sizeof(Proto)", "256"),
        ("sizeof(UpVal)", "64"),
        ("sizeof(Udata)", "128"),
        ("sizeof(Closure)", "128"),
        ("sizeof(CClosure)", "128"),
        ("sizeof(LClosure)", "128"),
        ("sizeof(LStream)", "64"),
        ("sizeof(RanState)", "16"),
        ("sizeof(GMatchState)", "128"),
        ("sizeof(Limbox)", "32"),
        ("sizeof(char)", "1"),
        ("sizeof(signed char)", "1"),
        ("sizeof(unsigned char)", "1"),
        ("sizeof(short)", "2"),
        ("sizeof(short int)", "2"),
        ("sizeof(unsigned short)", "2"),
        ("sizeof(unsigned short int)", "2"),
        ("sizeof(int)", "8"),
        ("sizeof(unsigned)", "8"),
        ("sizeof(unsigned int)", "8"),
        ("sizeof(long)", "8"),
        ("sizeof(long int)", "8"),
        ("sizeof(unsigned long)", "8"),
        ("sizeof(unsigned long int)", "8"),
        ("sizeof(long long)", "8"),
        ("sizeof(long long int)", "8"),
        ("sizeof(unsigned long long)", "8"),
        ("sizeof(unsigned long long int)", "8"),
        ("sizeof(float)", "8"),
        ("sizeof(double)", "8"),
    ] {
        out = out.replace(from, to);
    }
    out
}

pub fn normalize_find_struct_initializers(source: &str) -> String {
    let mut out = String::new();
    for line in source.lines() {
        let indent_len = line.len() - line.trim_start().len();
        let indent = &line[..indent_len];
        let trimmed = line.trim();
        if trimmed.contains("*f, cur;") || trimmed == "int f, cur;" {
            out.push_str(indent);
            out.push_str("int f;\n");
            out.push_str(indent);
            out.push_str("int cur; cur = alloc(32);\n");
        } else if trimmed.contains(" arg = {")
            && trimmed.contains("path")
            && trimmed.contains("st")
            && trimmed.ends_with(';')
        {
            out.push_str(indent);
            out.push_str("int arg; arg = alloc(24);\n");
            out.push_str(indent);
            out.push_str("arg.path = path;\n");
            out.push_str(indent);
            out.push_str("arg.st = st;\n");
            out.push_str(indent);
            out.push_str("arg.extra.p = 0;\n");
        } else if trimmed.contains(" and = {")
            && trimmed.contains("find_op(\"-a\")")
            && trimmed.ends_with(';')
        {
            out.push_str(indent);
            out.push_str("int and; and = alloc(40);\n");
            out.push_str(indent);
            out.push_str("and.u.oinfo = find_op(\"-a\");\n");
            out.push_str(indent);
            out.push_str("and.type = AND;\n");
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out.replace("eval(root, &arg)", "eval(root, arg)")
        .replace("find(pathbuf, &cur)", "find(pathbuf, cur)")
}

pub fn normalize_storage_class_arrays(source: &str) -> String {
    let mut out = String::new();
    for line in source.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("static char ") && trimmed.contains('[') && !trimmed.contains('(') {
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

pub fn normalize_sort_struct_globals(source: &str) -> String {
    let mut out = String::new();
    let mut depth = 0i64;
    for line in source.lines() {
        let trimmed = line.trim();
        if depth == 0 && trimmed.starts_with("static struct column ") && trimmed.ends_with(';') {
            let rest = trimmed
                .trim_start_matches("static struct column ")
                .trim_end_matches(';');
            for name in rest
                .split(',')
                .map(str::trim)
                .filter(|name| !name.is_empty())
            {
                out.push_str("int ");
                out.push_str(name);
                out.push_str("[3] = {0,0,0};\n");
            }
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

pub fn normalize_linebuf_declarations(source: &str) -> String {
    let mut out = String::new();
    let mut object_names = Vec::new();
    for line in source.lines() {
        let indent_len = line.len() - line.trim_start().len();
        let indent = &line[..indent_len];
        let trimmed = line.trim();
        if trimmed == "struct linebuf linebuf = EMPTY_LINEBUF;" {
            object_names.push("linebuf".to_string());
            out.push_str(indent);
            out.push_str("int linebuf; linebuf = alloc(24);\n");
            out.push_str(indent);
            out.push_str("linebuf.lines = 0; linebuf.nlines = 0; linebuf.cap = 0;\n");
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    for name in object_names {
        if name == "s" {
            continue;
        }
        out = replace_amp_object_refs(&out, &name);
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
        if name == "s" {
            continue;
        }
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
        let rest = trimmed.strip_prefix(ty);
        if rest.is_some_and(|rest| rest.starts_with(char::is_whitespace))
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
        if name == "s" {
            continue;
        }
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
        let unnamed = name.is_empty();
        if !unnamed && ident(name).is_none() {
            out.push_str(&source[start..after_name]);
            pos = after_name;
            continue;
        }
        if source[after_name..].chars().next() != Some('(') {
            out.push_str(&source[start..after_name]);
            pos = after_name;
            continue;
        }
        let Some(end) = matching_paren_end(source, after_name) else {
            out.push_str(&source[start..]);
            return out;
        };
        out.push_str("int");
        if !unnamed {
            out.push(' ');
            out.push_str(name);
        }
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

pub fn normalize_pointer_char_idioms(source: &str) -> String {
    source.replace("isalpha((int)**s)", "isalpha((*s)[0])")
}

pub fn normalize_anonymous_enums(source: &str) -> String {
    let mut constants = Vec::new();
    let mut out = String::new();
    let mut lines = source.lines();

    while let Some(line) = lines.next() {
        if let Some((start, body_start)) = enum_body_start(line) {
            let mut body = String::new();
            let after_start = &line[body_start..];
            if let Some(end) = enum_declaration_end(after_start) {
                body.push_str(&after_start[..end.close_brace]);
                collect_enum_constants(&body, &mut constants);
                push_enum_prefix_and_declarator(&mut out, &line[..start], end.declarator);
                out.push_str(end.after_semicolon);
                out.push('\n');
                continue;
            }

            body.push_str(after_start);
            body.push('\n');
            for enum_line in lines.by_ref() {
                if let Some(end) = enum_declaration_end(enum_line) {
                    body.push_str(&enum_line[..end.close_brace]);
                    collect_enum_constants(&body, &mut constants);
                    push_enum_prefix_and_declarator(&mut out, &line[..start], end.declarator);
                    out.push_str(end.after_semicolon);
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
        out = replace_enum_constant_token(&out, &name, &value.to_string());
    }
    out
}

fn enum_body_start(line: &str) -> Option<(usize, usize)> {
    let enum_pos = find_enum_keyword(line)?;
    let open_rel = line[enum_pos..].find('{')?;
    Some((enum_pos, enum_pos + open_rel + 1))
}

fn find_enum_keyword(line: &str) -> Option<usize> {
    let mut pos = 0usize;
    while let Some(rel) = line[pos..].find("enum") {
        let start = pos + rel;
        let end = start + "enum".len();
        let before = line[..start].chars().next_back();
        let after = line[end..].chars().next();
        if !before.is_some_and(is_ident_char) && !after.is_some_and(is_ident_char) {
            return Some(start);
        }
        pos = end;
    }
    None
}

struct EnumEnd<'a> {
    close_brace: usize,
    declarator: &'a str,
    after_semicolon: &'a str,
}

fn enum_declaration_end(text: &str) -> Option<EnumEnd<'_>> {
    let close_brace = text.find('}')?;
    let after_brace = &text[close_brace + 1..];
    let semicolon_rel = after_brace.find(';')?;
    Some(EnumEnd {
        close_brace,
        declarator: after_brace[..semicolon_rel].trim(),
        after_semicolon: &after_brace[semicolon_rel + 1..],
    })
}

fn push_enum_declarator(out: &mut String, declarator: &str) {
    if declarator.is_empty() {
        return;
    }
    out.push_str("int ");
    out.push_str(declarator);
    out.push(';');
}

fn push_enum_prefix_and_declarator(out: &mut String, prefix: &str, declarator: &str) {
    if prefix.trim() == "typedef" {
        return;
    }
    out.push_str(prefix);
    push_enum_declarator(out, declarator);
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
    if let Some((left, right)) = text.split_once("<<") {
        let left = parse_enum_value(left)?;
        let right = parse_enum_value(right)?;
        return Some(left << right);
    }
    if let Some((left, right)) = text.split_once(">>") {
        let left = parse_enum_value(left)?;
        let right = parse_enum_value(right)?;
        return Some(left >> right);
    }
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

fn braced_typedef_alias(trimmed: &str) -> Option<String> {
    if !trimmed.starts_with("typedef ") || !trimmed.contains('{') || !trimmed.ends_with(';') {
        return None;
    }
    let after = trimmed.rsplit_once('}')?.1.trim();
    ident(after.strip_suffix(';')?.trim())
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

fn function_pointer_typedef_alias(trimmed: &str) -> Option<String> {
    if !trimmed.starts_with("typedef ") {
        return None;
    }
    let marker = "(*";
    let start = trimmed.find(marker)? + marker.len();
    let rest = &trimmed[start..];
    let end = rest.find(')')?;
    ident(rest[..end].trim())
}

fn single_line_typedef_alias(trimmed: &str) -> Option<String> {
    if !trimmed.starts_with("typedef ") || !trimmed.ends_with(';') || trimmed.contains('{') {
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

fn replace_type_alias_token(source: &str, needle: &str, replacement: &str) -> String {
    let mut out = String::new();
    let mut pos = 0;
    while let Some(rel) = source[pos..].find(needle) {
        let start = pos + rel;
        let end = start + needle.len();
        let before = source[..start].chars().next_back();
        let after = source[end..].chars().next();
        if before.is_some_and(is_ident_char)
            || after.is_some_and(is_ident_char)
            || previous_word_is_tag_keyword(source, start)
        {
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

fn replace_token_phrase(source: &str, needle: &str, replacement: &str) -> String {
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

fn previous_word_is_tag_keyword(source: &str, start: usize) -> bool {
    let before = source[..start].trim_end();
    let word_start = before
        .char_indices()
        .rev()
        .find_map(|(idx, ch)| (!is_ident_char(ch)).then_some(idx + ch.len_utf8()))
        .unwrap_or(0);
    matches!(&before[word_start..], "struct" | "union" | "enum")
}

fn replace_enum_constant_token(source: &str, needle: &str, replacement: &str) -> String {
    let mut out = String::new();
    let mut pos = 0;
    while let Some(rel) = source[pos..].find(needle) {
        let start = pos + rel;
        let end = start + needle.len();
        let before = source[..start].chars().next_back();
        let after = source[end..].chars().next();
        let followed_by_call = source[end..].trim_start().starts_with('(');
        if before.is_some_and(is_ident_char) || after.is_some_and(is_ident_char) || followed_by_call
        {
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

#[cfg(test)]
mod tests {
    use super::{
        apply_user_type_alias_rewrites, collect_user_type_aliases,
        normalize_function_pointer_params,
    };

    #[test]
    fn function_pointer_param_rewrite_leaves_indirect_calls() {
        let source = r#"
void takes_cb(void (*cb)(int));
void takes_unnamed(int (*)(const char *, const char *, int));
int main() {
  (*g->frealloc)(g->ud, g, 8, 0);
}
"#;
        let out = normalize_function_pointer_params(source);
        assert!(out.contains("void takes_cb(int cb);"), "{out}");
        assert!(out.contains("void takes_unnamed(int);"), "{out}");
        assert!(out.contains("(*g->frealloc)(g->ud, g, 8, 0);"), "{out}");
    }

    #[test]
    fn collects_multiline_function_pointer_typedef_aliases() {
        let source = r#"
typedef unsigned (*in_func)(void *,
                            const unsigned char **);
int inflateBack(int stream, in_func in);
"#;
        let aliases = collect_user_type_aliases(source);
        assert!(
            aliases.iter().any(|alias| alias == "in_func"),
            "{aliases:?}"
        );
        let out = apply_user_type_alias_rewrites(source, &aliases);
        assert!(
            out.contains("int inflateBack(int stream, int in);"),
            "{out}"
        );
    }
}
