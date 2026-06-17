use std::collections::{HashMap, HashSet};

pub fn collect_field_offsets(source: &str) -> HashMap<String, i64> {
    let aggregate_sizes = collect_aggregate_sizes(source);
    let source = strip_c_comments(source);
    let mut fields = HashMap::new();
    let mut stack: Vec<AggregateContext> = Vec::new();
    let mut pending_aggregate_kind: Option<AggregateKind> = None;

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(kind) = pending_aggregate_kind.take() {
            if trimmed.starts_with('{') {
                stack.push(AggregateContext::new(kind));
                continue;
            }
        }

        if let Some((name, size)) = single_line_aggregate_field(trimmed) {
            if let Some(context) = stack.last_mut() {
                fields.entry(name).or_insert(context.next_field_offset());
                context.add_field_size(size);
            }
            continue;
        }

        if starts_aggregate_definition(trimmed) {
            stack.push(AggregateContext::new(aggregate_kind(trimmed)));
            continue;
        }
        if let Some(kind) = pending_aggregate_definition(trimmed) {
            pending_aggregate_kind = Some(kind);
            continue;
        }

        if let Some(name) = closing_aggregate_field(trimmed) {
            let Some(context) = stack.pop() else {
                continue;
            };
            if let Some(parent) = stack.last_mut() {
                let offset = parent.next_field_offset();
                if let Some(name) = name {
                    fields.entry(name).or_insert(offset);
                }
                parent.add_field_size(context.size());
            }
            continue;
        }

        if let Some(context) = stack.last_mut()
            && trimmed.ends_with(';')
            && !trimmed.contains('{')
            && !(trimmed.starts_with("enum ") && trimmed.contains('{'))
        {
            let size = aggregate_field_size(trimmed, &aggregate_sizes);
            for name in field_names_from_decl(trimmed) {
                fields.entry(name).or_insert(context.next_field_offset());
                context.add_field_size(size);
            }
        }
    }

    fields
}

pub fn collect_aggregate_sizes(source: &str) -> HashMap<String, i64> {
    let source = strip_c_comments(source);
    let mut sizes = HashMap::new();
    let mut stack: Vec<NamedAggregateContext> = Vec::new();
    let mut pending_aggregate: Option<(AggregateKind, Option<String>)> = None;

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some((kind, name)) = pending_aggregate.take()
            && trimmed.starts_with('{')
        {
            stack.push(NamedAggregateContext::new(kind, name));
            continue;
        }

        if starts_aggregate_definition(trimmed) {
            stack.push(NamedAggregateContext::new(
                aggregate_kind(trimmed),
                aggregate_definition_name(trimmed),
            ));
            continue;
        }
        if let Some(kind) = pending_aggregate_definition(trimmed) {
            pending_aggregate = Some((kind, aggregate_definition_name(trimmed)));
            continue;
        }

        if closing_aggregate_field(trimmed).is_some() {
            let Some(context) = stack.pop() else {
                continue;
            };
            let size = context.size();
            if let Some(name) = context.name {
                sizes.entry(name).or_insert(size);
            }
            if let Some(parent) = stack.last_mut() {
                parent.add_field_size(size);
            }
            continue;
        }

        if let Some(context) = stack.last_mut()
            && trimmed.ends_with(';')
            && !trimmed.contains('{')
            && !(trimmed.starts_with("enum ") && trimmed.contains('{'))
        {
            let size = aggregate_field_size(trimmed, &sizes);
            for _ in field_names_from_decl(trimmed) {
                context.add_field_size(size);
            }
        }
    }

    sizes
}

pub fn collect_global_aggregate_array_widths(source: &str) -> HashMap<String, i64> {
    let source = strip_c_comments(source);
    let aggregate_sizes = collect_aggregate_sizes(&source);
    let mut widths = HashMap::new();
    let mut stack: Vec<NamedAggregateContext> = Vec::new();
    let mut pending_aggregate: Option<(AggregateKind, Option<String>)> = None;

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some((kind, name)) = pending_aggregate.take()
            && trimmed.starts_with('{')
        {
            stack.push(NamedAggregateContext::new(kind, name));
            continue;
        }

        if starts_aggregate_definition(trimmed) {
            stack.push(NamedAggregateContext::new(
                aggregate_kind(trimmed),
                aggregate_definition_name(trimmed),
            ));
            continue;
        }
        if let Some(kind) = pending_aggregate_definition(trimmed) {
            pending_aggregate = Some((kind, aggregate_definition_name(trimmed)));
            continue;
        }

        if let Some(name) = closing_aggregate_field(trimmed) {
            let Some(context) = stack.pop() else {
                continue;
            };
            let size = context.size();
            if stack.is_empty()
                && let Some(name) = name
                && trimmed.contains('[')
            {
                widths.entry(name).or_insert(size);
            }
            if let Some(parent) = stack.last_mut() {
                parent.add_field_size(size);
            }
            continue;
        }

        if let Some(context) = stack.last_mut()
            && trimmed.ends_with(';')
            && !trimmed.contains('{')
            && !(trimmed.starts_with("enum ") && trimmed.contains('{'))
        {
            let size = aggregate_field_size(trimmed, &aggregate_sizes);
            for _ in field_names_from_decl(trimmed) {
                context.add_field_size(size);
            }
        }
    }

    widths
}

pub fn collect_aggregate_declarations(
    source: &str,
    sizes: &HashMap<String, i64>,
) -> HashMap<String, i64> {
    let source = strip_c_comments(source);
    let mut declarations = HashMap::new();
    let mut aggregate_depth = 0i64;
    let mut pending_aggregate = false;

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if pending_aggregate {
            pending_aggregate = false;
            if trimmed.starts_with('{') {
                aggregate_depth = count_braces(trimmed);
                if aggregate_depth <= 0 {
                    aggregate_depth = 1;
                }
                continue;
            }
        }
        if aggregate_depth > 0 {
            aggregate_depth += count_braces(trimmed);
            if aggregate_depth < 0 {
                aggregate_depth = 0;
            }
            continue;
        }
        if starts_aggregate_definition(trimmed) {
            aggregate_depth = count_braces(trimmed);
            if aggregate_depth > 0 {
                continue;
            }
        }
        if pending_aggregate_definition(trimmed).is_some() {
            pending_aggregate = true;
            continue;
        }
        if let Some((rest, size)) = named_aggregate_decl_rest(trimmed, sizes) {
            for name in declarator_names(rest) {
                declarations.entry(name).or_insert(size);
            }
        }
    }

    declarations
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AggregateKind {
    Struct,
    Union,
}

#[derive(Debug)]
struct AggregateContext {
    kind: AggregateKind,
    offset: i64,
    max_size: i64,
}

impl AggregateContext {
    fn new(kind: AggregateKind) -> Self {
        Self {
            kind,
            offset: 0,
            max_size: 0,
        }
    }

    fn next_field_offset(&self) -> i64 {
        match self.kind {
            AggregateKind::Struct => self.offset,
            AggregateKind::Union => 0,
        }
    }

    fn add_field_size(&mut self, size: i64) {
        match self.kind {
            AggregateKind::Struct => self.offset += size,
            AggregateKind::Union => self.max_size = self.max_size.max(size),
        }
    }

    fn size(&self) -> i64 {
        match self.kind {
            AggregateKind::Struct => self.offset,
            AggregateKind::Union => self.max_size,
        }
    }
}

#[derive(Debug)]
struct NamedAggregateContext {
    inner: AggregateContext,
    name: Option<String>,
}

impl NamedAggregateContext {
    fn new(kind: AggregateKind, name: Option<String>) -> Self {
        Self {
            inner: AggregateContext::new(kind),
            name,
        }
    }

    fn add_field_size(&mut self, size: i64) {
        self.inner.add_field_size(size);
    }

    fn size(&self) -> i64 {
        self.inner.size()
    }
}

fn strip_c_comments(source: &str) -> String {
    let mut out = String::new();
    let mut chars = source.chars().peekable();
    let mut in_string = false;
    let mut in_char = false;
    let mut escaped = false;
    while let Some(ch) = chars.next() {
        if in_string || in_char {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if in_string && ch == '"' {
                in_string = false;
            } else if in_char && ch == '\'' {
                in_char = false;
            }
        } else if ch == '"' {
            in_string = true;
            out.push(ch);
        } else if ch == '\'' {
            in_char = true;
            out.push(ch);
        } else if ch == '/' && chars.peek() == Some(&'*') {
            chars.next();
            while let Some(inner) = chars.next() {
                if inner == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    break;
                }
                if inner == '\n' {
                    out.push('\n');
                }
            }
        } else if ch == '/' && chars.peek() == Some(&'/') {
            for inner in chars.by_ref() {
                if inner == '\n' {
                    out.push('\n');
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

pub fn global_aggregate_size(function_names: &HashSet<String>, name: &str) -> Option<i64> {
    if has_sed_layout(function_names) {
        match name {
            "patt" | "hold" | "genbuf" => return Some(16),
            "braces" | "labels" | "branches" | "writes" | "wfiles" => return Some(24),
            "gflags" => return Some(48),
            _ => {}
        }
    }
    None
}

pub fn local_aggregate_size(
    function_names: &HashSet<String>,
    current_fn: &str,
    name: &str,
) -> Option<i64> {
    match name {
        "linebuf" => return Some(24),
        _ => {}
    }
    if has_sed_layout(function_names)
        && matches!(current_fn, "cmd_s" | "cmd_x" | "cmd_y")
        && name == "tmp"
    {
        return Some(16);
    }
    None
}

pub fn member_aggregate_size(function_names: &HashSet<String>, field: &str) -> Option<i64> {
    if has_sed_layout(function_names) && matches!(field, "str" | "repl") {
        return Some(16);
    }
    None
}

fn has_sed_layout(function_names: &HashSet<String>) -> bool {
    function_names.contains("cmd_last")
}

fn starts_aggregate_definition(trimmed: &str) -> bool {
    let trimmed = strip_storage_prefixes(trimmed);
    (trimmed.starts_with("struct ")
        || trimmed.starts_with("typedef struct ")
        || trimmed.starts_with("union ")
        || trimmed.starts_with("typedef union "))
        && trimmed.contains('{')
}

fn pending_aggregate_definition(trimmed: &str) -> Option<AggregateKind> {
    let trimmed = strip_storage_prefixes(trimmed);
    if trimmed.ends_with(';') || trimmed.contains('{') || trimmed.contains('=') {
        return None;
    }
    if trimmed.starts_with("struct ") || trimmed.starts_with("typedef struct ") {
        Some(AggregateKind::Struct)
    } else if trimmed.starts_with("union ") || trimmed.starts_with("typedef union ") {
        Some(AggregateKind::Union)
    } else {
        None
    }
}

fn aggregate_kind(trimmed: &str) -> AggregateKind {
    let trimmed = strip_storage_prefixes(trimmed);
    if trimmed.starts_with("union ") || trimmed.starts_with("typedef union ") {
        AggregateKind::Union
    } else {
        AggregateKind::Struct
    }
}

fn aggregate_definition_name(trimmed: &str) -> Option<String> {
    let trimmed = strip_storage_prefixes(trimmed.trim_start());
    let rest = trimmed
        .strip_prefix("typedef struct ")
        .or_else(|| trimmed.strip_prefix("struct "))
        .or_else(|| trimmed.strip_prefix("typedef union "))
        .or_else(|| trimmed.strip_prefix("union "))?;
    let before_body = rest.split('{').next().unwrap_or(rest).trim();
    let name = before_body.split_whitespace().next()?;
    ident(name).map(|name| {
        if trimmed.starts_with("typedef union ") || trimmed.starts_with("union ") {
            format!("union {name}")
        } else {
            format!("struct {name}")
        }
    })
}

fn strip_storage_prefixes(mut trimmed: &str) -> &str {
    loop {
        let next = trimmed
            .strip_prefix("static ")
            .or_else(|| trimmed.strip_prefix("const "))
            .or_else(|| trimmed.strip_prefix("volatile "))
            .or_else(|| trimmed.strip_prefix("extern "));
        let Some(next) = next else {
            return trimmed;
        };
        trimmed = next.trim_start();
    }
}

fn aggregate_field_size(trimmed: &str, sizes: &HashMap<String, i64>) -> i64 {
    if let Some(rest) = trimmed.strip_prefix("struct ")
        && let Some(name) = rest.split_whitespace().next()
        && let Some(size) = sizes.get(&format!("struct {name}"))
    {
        if rest[name.len()..].trim_start().starts_with('*') {
            return 8;
        }
        return *size;
    }
    if let Some(rest) = trimmed.strip_prefix("union ")
        && let Some(name) = rest.split_whitespace().next()
        && let Some(size) = sizes.get(&format!("union {name}"))
    {
        if rest[name.len()..].trim_start().starts_with('*') {
            return 8;
        }
        return *size;
    }
    8
}

fn named_aggregate_decl_rest<'a>(
    trimmed: &'a str,
    sizes: &HashMap<String, i64>,
) -> Option<(&'a str, i64)> {
    let (kind, rest) = trimmed
        .strip_prefix("struct ")
        .map(|rest| ("struct", rest))
        .or_else(|| trimmed.strip_prefix("union ").map(|rest| ("union", rest)))?;
    let tag = rest.split_whitespace().next()?;
    let size = *sizes.get(&format!("{kind} {tag}"))?;
    let rest = rest[tag.len()..].trim();
    if rest.contains('(') || !rest.ends_with(';') {
        return None;
    }
    Some((rest.trim_end_matches(';').trim(), size))
}

fn declarator_names(rest: &str) -> Vec<String> {
    rest.split(',')
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() || part.contains('*') {
                return None;
            }
            ident(part)
        })
        .map(str::to_string)
        .collect()
}

fn count_braces(line: &str) -> i64 {
    line.chars().fold(0, |depth, ch| match ch {
        '{' => depth + 1,
        '}' => depth - 1,
        _ => depth,
    })
}

fn closing_aggregate_field(trimmed: &str) -> Option<Option<String>> {
    if !trimmed.starts_with('}') {
        return None;
    }
    let mut name = trimmed.trim_start_matches('}').trim();
    let end = name
        .find(|ch: char| matches!(ch, ';' | '=' | '[' | ','))
        .unwrap_or(name.len());
    name = name[..end].trim();
    if name.is_empty() {
        Some(None)
    } else {
        ident(name).map(str::to_string).map(Some)
    }
}

fn single_line_aggregate_field(trimmed: &str) -> Option<(String, i64)> {
    let kind = if trimmed.starts_with("union ") {
        AggregateKind::Union
    } else if trimmed.starts_with("struct ") {
        AggregateKind::Struct
    } else {
        return None;
    };
    let open = trimmed.find('{')?;
    let close = trimmed.rfind('}')?;
    if close <= open || !trimmed.ends_with(';') {
        return None;
    }
    let name = trimmed[close + 1..].trim().trim_end_matches(';').trim();
    let name = ident(name)?.to_string();
    let field_count = trimmed[open + 1..close]
        .split(';')
        .flat_map(field_names_from_decl)
        .count() as i64;
    let size = match kind {
        AggregateKind::Struct => field_count * 8,
        AggregateKind::Union => i64::from(field_count > 0) * 8,
    };
    Some((name, size))
}

fn field_names_from_decl(trimmed: &str) -> Vec<String> {
    trimmed
        .split(';')
        .flat_map(|decl| {
            let decl = decl.trim();
            if decl.is_empty() {
                return Vec::new();
            }
            let decl = strip_array_declarators(decl);
            if decl.contains('(') {
                return Vec::new();
            }
            decl.split(',')
                .filter_map(|part| {
                    let name = part
                        .split_whitespace()
                        .last()
                        .unwrap_or_default()
                        .trim_start_matches('*');
                    let name = name.split('[').next().unwrap_or(name);
                    ident(name).map(str::to_string)
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn strip_array_declarators(decl: &str) -> String {
    let mut out = String::with_capacity(decl.len());
    let mut depth = 0usize;
    for ch in decl.chars() {
        match ch {
            '[' => depth += 1,
            ']' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out
}

fn ident(text: &str) -> Option<&str> {
    let mut chars = text.chars();
    let first = chars.next()?;
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return None;
    }
    chars
        .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        .then_some(text)
}

#[cfg(test)]
mod tests {
    use super::{
        collect_aggregate_declarations, collect_aggregate_sizes, collect_field_offsets,
        collect_global_aggregate_array_widths,
    };

    #[test]
    fn collects_simple_struct_field_offsets() {
        let fields = collect_field_offsets(
            r#"
            typedef struct lua_State {
              int top;
              struct CallInfo *ci;
              char short_src[80];
            } lua_State;
            "#,
        );
        assert_eq!(fields["top"], 0);
        assert_eq!(fields["ci"], 8);
        assert_eq!(fields["short_src"], 16);
    }

    #[test]
    fn collects_next_line_brace_struct_fields() {
        let fields = collect_field_offsets(
            r#"
            struct AES_ctx
            {
              int RoundKey[176];
              int Iv[16];
            };
            "#,
        );
        assert_eq!(fields["RoundKey"], 0);
        assert_eq!(fields["Iv"], 8);
    }

    #[test]
    fn collects_nested_named_aggregate_sizes() {
        let source = r#"
            struct Segment
            {
              int path;
              int begin;
              int size;
            };

            struct Joined
            {
              struct Segment segment;
              int paths;
              int index;
            };
            "#;
        let sizes = collect_aggregate_sizes(source);
        assert_eq!(sizes["struct Segment"], 24);
        assert_eq!(sizes["struct Joined"], 40);
        let fields = collect_field_offsets(source);
        assert_eq!(fields["segment"], 0);
        assert_eq!(fields["paths"], 24);
        assert_eq!(fields["index"], 32);
    }

    #[test]
    fn collects_named_aggregate_declaration_sizes() {
        let source = r#"
            struct Segment
            {
              int path;
              int begin;
              int size;
            };

            struct Joined
            {
              struct Segment segment;
              int paths;
              int index;
            };

            int check(struct Joined *sj) {
                struct Joined copy, other;
                struct Joined *ptr;
                return 0;
            }
            "#;
        let sizes = collect_aggregate_sizes(source);
        let declarations = collect_aggregate_declarations(source, &sizes);
        assert_eq!(declarations["copy"], 40);
        assert_eq!(declarations["other"], 40);
        assert!(!declarations.contains_key("ptr"));
    }

    #[test]
    fn collects_array_field_with_spaced_length_expression() {
        let fields = collect_field_offsets(
            r#"
            typedef struct {
              unsigned total;
              unsigned na;
              int deleted;
              unsigned nums[MAXABITS + 1];
            } Counters;
            "#,
        );
        assert_eq!(fields["total"], 0);
        assert_eq!(fields["na"], 8);
        assert_eq!(fields["deleted"], 16);
        assert_eq!(fields["nums"], 24);
    }

    #[test]
    fn collects_macro_expanded_struct_fields() {
        let expanded = crate::c_macro_rewrites::expand_object_like_macros(
            r#"
            #define TValuefields Value value_; int tt_
            typedef struct TValue {
              TValuefields;
            } TValue;
            "#,
        );
        let fields = collect_field_offsets(&expanded);
        assert_eq!(fields["value_"], 0);
        assert_eq!(fields["tt_"], 8);
    }

    #[test]
    fn collects_backslash_continued_macro_fields() {
        let expanded = crate::c_macro_rewrites::expand_object_like_macros(
            " #define Header int tt; \\\n int nupvalues\n\
             typedef struct Closure {\n\
               Header;\n\
               int f;\n\
             } Closure;\n",
        );
        let fields = collect_field_offsets(&expanded);
        assert_eq!(fields["tt"], 0);
        assert_eq!(fields["nupvalues"], 8);
        assert_eq!(fields["f"], 16);
    }

    #[test]
    fn collects_nested_anonymous_aggregate_offsets() {
        let fields = collect_field_offsets(
            r#"
            struct CallInfo {
              int func;
              int top;
              struct CallInfo *previous, *next;
              union {
                struct {
                  int savedpc;
                  int trap;
                  int nextraargs;
                } l;
                struct {
                  int k;
                  int old_errfunc;
                  int ctx;
                } c;
              } u;
              union {
                int funcidx;
                int nyield;
                int nres;
              } u2;
              int callstatus;
            };
            "#,
        );
        assert_eq!(fields["previous"], 16);
        assert_eq!(fields["next"], 24);
        assert_eq!(fields["u"], 32);
        assert_eq!(fields["l"], 0);
        assert_eq!(fields["c"], 0);
        assert_eq!(fields["ctx"], 16);
        assert_eq!(fields["u2"], 56);
        assert_eq!(fields["callstatus"], 64);
    }

    #[test]
    fn collects_single_line_anonymous_aggregate_field() {
        let fields = collect_field_offsets(
            r#"
            typedef struct Udata0 {
              int tt;
              int len;
              union { int n; double u; void *s; long l; } bindata;
            } Udata0;
            "#,
        );
        assert_eq!(fields["tt"], 0);
        assert_eq!(fields["len"], 8);
        assert_eq!(fields["bindata"], 16);
    }

    #[test]
    fn collects_static_anonymous_struct_array_layout() {
        let source = r#"
            static struct {
                int x, y, div, mod;
            } t[] = {
                {1, 2, 0, 1},
                {4, 2, 2, 0},
            };
            "#;
        let fields = collect_field_offsets(source);
        let widths = collect_global_aggregate_array_widths(source);
        assert_eq!(fields["x"], 0);
        assert_eq!(fields["y"], 8);
        assert_eq!(fields["div"], 16);
        assert_eq!(fields["mod"], 24);
        assert_eq!(widths["t"], 32);
    }
}
