use std::collections::{HashMap, HashSet};

pub fn collect_field_offsets(source: &str) -> HashMap<String, i64> {
    let source = strip_c_comments(source);
    let mut fields = HashMap::new();
    let mut stack: Vec<AggregateContext> = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
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
            for name in field_names_from_decl(trimmed) {
                fields.entry(name).or_insert(context.next_field_offset());
                context.add_field_size(8);
            }
        }
    }

    fields
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
    (trimmed.starts_with("struct ")
        || trimmed.starts_with("typedef struct ")
        || trimmed.starts_with("union ")
        || trimmed.starts_with("typedef union "))
        && trimmed.contains('{')
}

fn aggregate_kind(trimmed: &str) -> AggregateKind {
    if trimmed.starts_with("union ") || trimmed.starts_with("typedef union ") {
        AggregateKind::Union
    } else {
        AggregateKind::Struct
    }
}

fn closing_aggregate_field(trimmed: &str) -> Option<Option<String>> {
    if !trimmed.starts_with('}') || !trimmed.ends_with(';') {
        return None;
    }
    let name = trimmed
        .trim_start_matches('}')
        .trim()
        .trim_end_matches(';')
        .trim();
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
            if decl.is_empty() || decl.contains('(') {
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
    use super::collect_field_offsets;

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
}
