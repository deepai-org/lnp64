pub fn normalize_do_while_loops(source: &str) -> String {
    let mut out = String::new();
    let mut pos = 0;
    while let Some(start) = find_do_block(source, pos) {
        out.push_str(&source[pos..start]);
        let Some(rewrite) = rewrite_do_block(source, start) else {
            out.push_str(&source[start..start + 2]);
            pos = start + 2;
            continue;
        };
        out.push_str(&rewrite.text);
        pos = rewrite.end;
    }
    out.push_str(&source[pos..]);
    out
}

struct Rewrite {
    text: String,
    end: usize,
}

fn rewrite_do_block(source: &str, start: usize) -> Option<Rewrite> {
    let open_brace = skip_ws(source, start + 2)?;
    if source[open_brace..].chars().next()? != '{' {
        return None;
    }
    let close_brace = matching_delim(source, open_brace, '{', '}')?;
    let while_pos = skip_ws(source, close_brace + 1)?;
    if !source[while_pos..].starts_with("while") {
        return None;
    }
    let open_paren = skip_ws(source, while_pos + "while".len())?;
    if source[open_paren..].chars().next()? != '(' {
        return None;
    }
    let close_paren = matching_delim(source, open_paren, '(', ')')?;
    let semi = skip_ws(source, close_paren + 1)?;
    if source[semi..].chars().next()? != ';' {
        return None;
    }

    let body = &source[open_brace + 1..close_brace];
    let cond = source[open_paren + 1..close_paren].trim();
    Some(Rewrite {
        text: format!("while (1) {{{body}\nif (!({cond})) break;\n}}"),
        end: semi + 1,
    })
}

fn find_do_block(source: &str, pos: usize) -> Option<usize> {
    let mut search = pos;
    while let Some(rel) = source[search..].find("do") {
        let start = search + rel;
        let end = start + 2;
        let before = source[..start].chars().next_back();
        let after = source[end..].chars().next();
        if !before.is_some_and(is_ident_char)
            && after.is_some_and(|ch| ch.is_whitespace() || ch == '{')
        {
            return Some(start);
        }
        search = end;
    }
    None
}

fn skip_ws(source: &str, mut pos: usize) -> Option<usize> {
    while pos < source.len() {
        let ch = source[pos..].chars().next()?;
        if !ch.is_whitespace() {
            return Some(pos);
        }
        pos += ch.len_utf8();
    }
    None
}

fn matching_delim(source: &str, open: usize, left: char, right: char) -> Option<usize> {
    let mut depth = 0i64;
    let mut in_string = false;
    let mut in_char = false;
    let mut escaped = false;
    for (rel, ch) in source[open..].char_indices() {
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
            continue;
        }
        if ch == '"' {
            in_string = true;
            continue;
        }
        if ch == '\'' {
            in_char = true;
            continue;
        }
        if ch == left {
            depth += 1;
        } else if ch == right {
            depth -= 1;
            if depth == 0 {
                return Some(open + rel);
            }
        }
    }
    None
}

fn is_ident_char(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::normalize_do_while_loops;

    #[test]
    fn do_while_rewrite_ignores_braces_inside_char_literals() {
        let source = r#"
int main() {
  do {
    if (token == '}') break;
    token = '{';
  } while (next());
}
"#;
        let out = normalize_do_while_loops(source);
        assert!(out.contains("while (1)"), "{out}");
        assert!(out.contains("if (!(next())) break;"), "{out}");
        assert!(!out.contains("do {"), "{out}");
    }
}
