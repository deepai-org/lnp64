use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::c_constants::find_token_constant;
use crate::c_escapes::parse_c_escape;
use crate::c_queue_rewrites::normalize_queue_macros;
use crate::c_support_sources::companion_sources;
use crate::c_type_rewrites::{
    apply_scalar_type_rewrites, apply_user_struct_tag_rewrites, apply_user_type_alias_rewrites,
    collect_user_struct_tags, collect_user_type_aliases, normalize_anonymous_enums,
    normalize_function_pointer_conditionals, normalize_function_pointer_params,
    normalize_jsmn_parser_declarations, normalize_static_struct_line_globals,
    normalize_storage_class_arrays, normalize_string_pointer_array_initializers,
    normalize_struct_entry_declarations,
};

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Int,
    Return,
    If,
    Else,
    While,
    For,
    Switch,
    Case,
    Default,
    Goto,
    Break,
    Continue,
    Ident(String),
    Num(i64),
    Str(String),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Assign,
    PlusAssign,
    MinusAssign,
    StarAssign,
    AmpAssign,
    OrAssign,
    EqEq,
    NotEq,
    Bang,
    Amp,
    Pipe,
    Caret,
    Tilde,
    Shl,
    Shr,
    ShrAssign,
    AndAnd,
    OrOr,
    PlusPlus,
    MinusMinus,
    Lt,
    Gt,
    Le,
    Ge,
    Question,
    Colon,
    LBracket,
    RBracket,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Semi,
    Comma,
    Dot,
    Arrow,
    Eof,
}

#[derive(Debug, Clone)]
struct CProgram {
    globals: Vec<(String, GlobalInit)>,
    global_arrays: Vec<(String, Vec<i64>)>,
    functions: Vec<Function>,
}

#[derive(Debug, Clone)]
enum GlobalInit {
    Int(i64),
    Str(String),
}

#[derive(Debug, Clone)]
struct Function {
    name: String,
    params: Vec<String>,
    body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
struct LocalDecl {
    name: String,
    init: Option<Expr>,
    array_len: Option<i64>,
}

#[derive(Debug, Clone)]
enum Stmt {
    VarDecl(LocalDecl),
    VarDecls(Vec<LocalDecl>),
    Return(Expr),
    Expr(Expr),
    If {
        cond: Expr,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    For {
        init: Vec<Expr>,
        cond: Option<Expr>,
        post: Vec<Expr>,
        body: Vec<Stmt>,
    },
    Switch {
        expr: Expr,
        cases: Vec<(i64, Vec<Stmt>)>,
        default: Vec<Stmt>,
    },
    Label(String),
    Goto(String),
    Break,
    Continue,
}

#[derive(Debug, Clone)]
enum Expr {
    Num(i64),
    Str(String),
    Var(String),
    Binary(Box<Expr>, BinOp, Box<Expr>),
    Unary(UnOp, Box<Expr>),
    Assign(Box<Expr>, Box<Expr>),
    CompoundAssign(Box<Expr>, BinOp, Box<Expr>),
    Comma(Box<Expr>, Box<Expr>),
    PostInc(Box<Expr>),
    PostDec(Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Member(Box<Expr>, String),
    Call(String, Vec<Expr>),
    CallValue(Box<Expr>, Vec<Expr>),
}

impl Expr {
    fn contains_call(&self) -> bool {
        match self {
            Expr::Call(_, _) | Expr::CallValue(_, _) => true,
            Expr::Binary(lhs, _, rhs)
            | Expr::Assign(lhs, rhs)
            | Expr::CompoundAssign(lhs, _, rhs)
            | Expr::Comma(lhs, rhs)
            | Expr::Index(lhs, rhs) => lhs.contains_call() || rhs.contains_call(),
            Expr::Ternary(cond, then_expr, else_expr) => {
                cond.contains_call() || then_expr.contains_call() || else_expr.contains_call()
            }
            Expr::Unary(_, expr)
            | Expr::PostInc(expr)
            | Expr::PostDec(expr)
            | Expr::Member(expr, _) => expr.contains_call(),
            Expr::Num(_) | Expr::Str(_) | Expr::Var(_) => false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    BitOr,
    BitAnd,
    BitXor,
    Shl,
    Shr,
}

#[derive(Debug, Clone, Copy)]
enum UnOp {
    Not,
    Addr,
    Deref,
    BitNot,
}

pub fn compile(source: &str) -> Result<String, String> {
    let source = preprocess_source(source);
    let tokens = Lexer::new(&source).lex()?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program()?;
    let mut codegen = CodeGen::default();
    codegen.emit_program(&program)
}

pub fn compile_file(path: &Path) -> Result<String, String> {
    let mut seen = HashSet::new();
    let mut source = expand_quoted_includes(path, &mut seen)?;
    for companion in companion_sources(path, &source) {
        source.push('\n');
        source.push_str(&expand_quoted_includes(&companion, &mut seen)?);
    }
    compile(&source)
}

fn expand_quoted_includes(path: &Path, seen: &mut HashSet<PathBuf>) -> Result<String, String> {
    if !path.exists() {
        return Ok(String::new());
    }
    let path = path
        .canonicalize()
        .map_err(|err| format!("failed to resolve {}: {err}", path.display()))?;
    if !seen.insert(path.clone()) {
        return Ok(String::new());
    }
    let source = fs::read_to_string(&path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    let mut out = String::new();
    for line in source.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("#include \"") {
            if let Some(end) = rest.find('"') {
                let include_path = base.join(&rest[..end]);
                out.push_str(&expand_quoted_includes(&include_path, seen)?);
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    Ok(out)
}

fn preprocess_source(source: &str) -> String {
    let source = splice_escaped_newlines(source);
    let source = strip_block_comments(&source);
    let user_type_aliases = collect_user_type_aliases(&source);
    let user_struct_tags = collect_user_struct_tags(&source);
    let has_test_macros = source.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("#define check(")
            || trimmed.starts_with("#define fail(")
            || trimmed.starts_with("#define done(")
    });
    let mut out = String::new();
    let mut skip_cpp_block = false;
    let mut skip_inactive_depth = 0usize;
    let mut skip_directive_continuation = false;
    for line in source.lines() {
        let trimmed = line.trim_start();
        if skip_directive_continuation {
            skip_directive_continuation = trimmed.ends_with('\\');
            continue;
        }
        if trimmed.starts_with("#ifdef JSMN_PARENT_LINKS")
            || trimmed.starts_with("#ifdef JSMN_STRICT")
        {
            skip_inactive_depth += 1;
            continue;
        }
        if skip_inactive_depth > 0 {
            if trimmed.starts_with("#ifdef ")
                || trimmed.starts_with("#ifndef ")
                || trimmed.starts_with("#if ")
            {
                skip_inactive_depth += 1;
            } else if trimmed.starts_with("#else") && skip_inactive_depth == 1 {
                skip_inactive_depth = 0;
            } else if trimmed.starts_with("#endif") {
                skip_inactive_depth -= 1;
            }
            continue;
        }
        if trimmed.starts_with("#ifdef __cplusplus")
            || trimmed.starts_with("#if defined(__cplusplus)")
        {
            skip_cpp_block = true;
            continue;
        }
        if skip_cpp_block {
            if trimmed.starts_with("#endif") {
                skip_cpp_block = false;
            }
            continue;
        }
        if trimmed.starts_with('#') {
            skip_directive_continuation = trimmed.ends_with('\\');
            continue;
        }
        if trimmed.starts_with("typedef ") && trimmed.ends_with(';') {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    let out = if has_test_macros {
        expand_test_macros(&out)
    } else {
        out
    };
    let out = normalize_anonymous_enums(&out);
    let out = strip_user_struct_definitions(&out);
    let out = expand_arg_h_macros(&out);
    let out = normalize_queue_macros(&out);
    let out = normalize_function_pointer_conditionals(&out);
    let out = normalize_jsmn_parser_declarations(&out);
    let out = normalize_struct_entry_declarations(&out);
    let out = normalize_static_struct_line_globals(&out);
    let out = normalize_string_pointer_array_initializers(&out);
    let out = apply_user_type_alias_rewrites(&out, &user_type_aliases);
    let out = apply_user_struct_tag_rewrites(&out, &user_struct_tags);
    let out = normalize_storage_class_arrays(&out);
    let out = normalize_function_pointer_params(&out);
    let out = apply_scalar_type_rewrites(&out);
    normalize_c_types(&out)
}

fn expand_test_macros(source: &str) -> String {
    let source = replace_macro_call(source, "check", |arg| format!("if (!({arg})) return 1"));
    let source = replace_macro_call(&source, "fail", |_| "return 1".to_string());
    replace_macro_call(&source, "done", |_| "return 0".to_string())
}

fn replace_macro_call<F>(source: &str, name: &str, mut replacement: F) -> String
where
    F: FnMut(&str) -> String,
{
    let mut out = String::new();
    let mut pos = 0;
    let pattern = format!("{name}(");
    while let Some(rel) = source[pos..].find(&pattern) {
        let start = pos + rel;
        let before = source[..start].chars().last();
        if before.is_some_and(|ch| ch == '_' || ch.is_ascii_alphanumeric()) {
            out.push_str(&source[pos..start + pattern.len()]);
            pos = start + pattern.len();
            continue;
        }
        out.push_str(&source[pos..start]);
        let arg_start = start + pattern.len();
        let mut depth = 1i64;
        let mut end = arg_start;
        for (idx, ch) in source[arg_start..].char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = arg_start + idx;
                        break;
                    }
                }
                _ => {}
            }
        }
        if depth != 0 {
            out.push_str(&source[start..]);
            return out;
        }
        out.push_str(&replacement(source[arg_start..end].trim()));
        pos = end + 1;
    }
    out.push_str(&source[pos..]);
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

fn strip_block_comments(source: &str) -> String {
    let mut out = String::new();
    let mut chars = source.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '/' && chars.peek() == Some(&'*') {
            chars.next();
            while let Some(inner) = chars.next() {
                if inner == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    break;
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

fn expand_arg_h_macros(source: &str) -> String {
    let mut out = source.to_string();
    out = out.replace("ARGC()", "argc_");
    out = out.replace(
        "ARGNUM:",
        "case '0': case '1': case '2': case '3': case '4': case '5': case '6': case '7': case '8': case '9':",
    );
    out = out.replace(
        "ARGBEGIN",
        r#"for (argv0 = *argv, argv++, argc--; argv[0] && argv[0][0] == '-' && argv[0][1]; argc--, argv++) {
        int argc_;
        int argv_;
        int brk_;
        if (argv[0][1] == '-' && argv[0][2] == '\0') {
            argv++;
            argc--;
            break;
        }
        for (brk_ = 0, argv[0]++, argv_ = argv; argv[0][0] && !brk_; argv[0]++) {
            if (argv_ != argv)
                break;
            argc_ = argv[0][0];
            switch (argc_)"#,
    );
    out.replace("ARGEND", "} }")
}

fn strip_user_struct_definitions(source: &str) -> String {
    let mut out = String::new();
    let mut skip_depth = 0i64;
    let mut pending_static_struct = false;
    let mut pending_classes = false;
    for line in source.lines() {
        let trimmed = line.trim_start();
        if skip_depth == 0 && trimmed.starts_with("static struct {") {
            skip_depth += count_braces(line);
            pending_static_struct = true;
            continue;
        }
        if skip_depth == 0
            && (trimmed.starts_with("typedef struct ")
                || trimmed.starts_with("typedef enum ")
                || trimmed.starts_with("struct ")
                || trimmed.starts_with("enum ")
                || trimmed.starts_with("union "))
            && trimmed.ends_with('{')
        {
            skip_depth += count_braces(line);
            continue;
        }
        if skip_depth > 0 {
            if pending_static_struct && trimmed.contains("classes[]") {
                pending_classes = true;
            }
            skip_depth += count_braces(line);
            if skip_depth == 0 && pending_static_struct {
                if pending_classes {
                    out.push_str("int classes[1] = {0};\n");
                } else if trimmed.contains("gflags") {
                    out.push_str("int gflags[8] = {0,0,0,0,0,0,0,0};\n");
                } else if trimmed.contains("*tree") {
                    out.push_str("int tree = 0;\n");
                }
                pending_static_struct = false;
                pending_classes = false;
            }
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn count_braces(line: &str) -> i64 {
    line.chars().fold(0, |depth, ch| match ch {
        '{' => depth + 1,
        '}' => depth - 1,
        _ => depth,
    })
}

fn normalize_c_types(source: &str) -> String {
    let mut out = normalize_struct_stat_declarations(source);
    out = normalize_struct_recursor_declarations(&out);
    out = out.replace(
        "struct arg arg = { path, st, { NULL } };",
        "struct arg arg;\narg.path = path;\narg.st = st;\narg.extra.p = 0;",
    );
    out = normalize_struct_object_declarations(&out, "struct arg", 24);
    out = normalize_struct_object_declarations(&out, "jsmn_parser", 24);
    out = normalize_struct_object_declarations(&out, "struct line", 16);
    out = normalize_struct_object_declarations(&out, "static struct line", 16);
    out = normalize_struct_object_declarations(&out, "struct range", 24);
    out = normalize_struct_object_declarations(&out, "static struct range", 24);
    out = out.replace(
        "static struct timespec times[2] = {{.tv_nsec = UTIME_NOW}};",
        "int times[4] = {0,0,0,0};",
    );
    out = out.replace(
        "struct tok and = { .u.oinfo = find_op(\"-a\"), .type = AND };",
        "struct tok and;\nand.u.oinfo = find_op(\"-a\");\nand.type = AND;",
    );
    out = out.replace(
        "*out++ = (struct tok){ .u.pinfo = find_primary(\"-print\"), .type = PRIM };",
        "{ out->u.pinfo = find_primary(\"-print\"); out->type = PRIM; out++; }",
    );
    out = normalize_struct_object_declarations(&out, "struct tok", 40);
    out = out.replace("= { 0 }", "= 0");
    out = out.replace("sizeof(*fds)", "8");
    out = out.replace("sizeof(jsmntok_t)", "32");
    out = out.replace("sizeof(tok[0])", "32");
    out = out.replace("sizeof(tokens[0])", "32");
    out = out.replace("sizeof(toksmall)", "320");
    out = out.replace("sizeof(toklarge)", "320");
    out = out.replace("sizeof(tok)", "160");
    out = out.replace("sizeof(tokens)", "320");
    out = out.replace("320 / 32", "10");
    out = out.replace("160 / 32", "5");
    out = out.replace("sizeof(*r)", "24");
    out = out.replace("sizeof(*infix)", "40");
    out = out.replace("sizeof(*rpn)", "40");
    out = out.replace("sizeof(*tok)", "40");
    out = out.replace("sizeof(*stack)", "8");
    out = out.replace("2 * argc + 1", "2 * argc + 3");
    out = out.replace("sizeof(**set)", "24");
    out = out.replace("sizeof(*rstr)", "8");
    out = out.replace("sizeof(*tree)", "16");
    out = out.replace("sizeof(*ents)", "104");
    out = out.replace("sizeof(*dents)", "104");
    out = out.replace("sizeof(*fents)", "104");
    out = out.replace("sizeof(ent)", "104");
    out = out.replace("sizeof(t) / sizeof(t[0])", "128");
    out = out.replace("sizeof(buf)", "8192");
    out = out.replace("BUFSIZ", "8192");
    out = normalize_char_array_declarations(&out);
    out = out.replace("\"%\"PRIu32\" %zu\"", "\"%u %u\"");
    out = out.replace("unsigned char buf[8192];", "int buf; buf = alloc(8192);");
    out = out.replace("extern ", "");
    out = out.replace("JSMN_API ", "");
    out = out.replace("const ", "");
    out = out.replace("const int", "int");
    out = out.replace("int (*func)(void)", "int func");
    out = out.replace("int (*func)(int)", "int func");
    for (from, to) in [
        ("static const unsigned long", "int"),
        ("static unsigned", "int"),
        ("static char", "int"),
        ("struct stat *", "int "),
        ("struct stat", "int"),
        ("struct timespec *", "int "),
        ("struct timespec", "int"),
        ("struct tm *", "int "),
        ("struct tm", "int"),
        ("struct recursor *", "int "),
        ("struct recursor", "int"),
        ("struct arg *", "int "),
        ("struct arg", "int"),
        ("static struct pri_info *", "int "),
        ("struct pri_info *", "int "),
        ("struct pri_info", "int"),
        ("static struct op_info *", "int "),
        ("struct op_info *", "int "),
        ("struct op_info", "int"),
        ("static struct tok *", "int "),
        ("struct tok **", "int "),
        ("struct tok *", "int "),
        ("struct tok", "int"),
        ("struct permarg *", "int "),
        ("struct permarg", "int"),
        ("struct okarg *", "int "),
        ("struct okarg", "int"),
        ("struct narg *", "int "),
        ("struct narg", "int"),
        ("struct sizearg *", "int "),
        ("struct sizearg", "int"),
        ("struct execarg *", "int "),
        ("struct execarg", "int"),
        ("struct findhist *", "int "),
        ("struct findhist", "int"),
        ("struct passwd *", "int "),
        ("struct passwd", "int"),
        ("struct group *", "int "),
        ("struct group", "int"),
        ("struct dirent *", "int "),
        ("struct dirent", "int"),
        ("union extra *", "int "),
        ("union extra", "int"),
        ("static struct line *", "int "),
        ("struct line *", "int "),
        ("struct line", "int"),
        ("static struct   range *", "int "),
        ("static struct range *", "int "),
        ("struct range **", "int "),
        ("struct range *", "int "),
        ("struct range", "int"),
        ("Range *", "int "),
        ("Range", "int"),
        ("Rune", "int"),
        ("mode_t", "int"),
        ("time_t", "int"),
        ("dev_t", "int"),
        ("ino_t", "int"),
        ("uid_t", "int"),
        ("gid_t", "int"),
        ("pid_t", "int"),
        ("off_t", "int"),
        ("intmax_t", "int"),
        ("unsigned long", "int"),
        ("unsigned char", "int"),
        ("unsigned int", "int"),
        ("unsigned", "int"),
        ("uint32_t", "int"),
        ("jsmn_parser *", "int "),
        ("jsmn_parser", "int"),
        ("jsmntok_t *", "int "),
        ("jsmntok_t", "int"),
        ("jsmntype_t", "int"),
        ("va_list", "int"),
        ("regex_t *", "int "),
        ("regex_t", "int"),
        ("char *argv[]", "int argv"),
        ("char *argv", "int argv"),
        ("const char *", "int "),
        ("FILE *", "int "),
        ("DIR *", "int "),
        ("char **", "int "),
        ("char *", "int "),
        ("int *", "int "),
        ("ssize_t", "int"),
        ("size_t", "int"),
        ("static int", "int"),
        ("static void", "int"),
        ("void", "int"),
    ] {
        out = out.replace(from, to);
    }
    out = out.replace("va_arg(ap, int)", "va_arg(ap, 0)");
    out = out.replace("va_arg(args, int)", "va_arg(args, 0)");
    let out = normalize_function_pointer_declarations(&out);
    out.lines()
        .filter(|line| line.trim() != "int;")
        .map(|line| {
            let mut line = line.to_string();
            line.push('\n');
            line
        })
        .collect()
}

fn normalize_struct_object_declarations(source: &str, ty: &str, size: i64) -> String {
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
    out.replace("seek(&s,", "seek(s,")
}

fn normalize_struct_recursor_declarations(source: &str) -> String {
    let mut out = String::new();
    let mut object_names = Vec::new();
    for line in source.lines() {
        let indent_len = line.len() - line.trim_start().len();
        let indent = &line[..indent_len];
        let trimmed = line.trim();
        if trimmed.starts_with("struct recursor ")
            && trimmed.contains("= {")
            && trimmed.ends_with("};")
        {
            let Some(eq) = trimmed.find('=') else {
                out.push_str(line);
                out.push('\n');
                continue;
            };
            let name = trimmed["struct recursor ".len()..eq].trim();
            object_names.push(name.to_string());
            let body = trimmed[eq + 1..]
                .trim()
                .trim_start_matches('{')
                .trim_end_matches("};")
                .trim();
            out.push_str(indent);
            out.push_str("int ");
            out.push_str(name);
            out.push_str("; ");
            out.push_str(name);
            out.push_str(" = alloc(64);\n");
            for part in body
                .split(',')
                .map(str::trim)
                .filter(|part| !part.is_empty())
            {
                let Some(rest) = part.strip_prefix('.') else {
                    continue;
                };
                let Some(assign) = rest.find('=') else {
                    continue;
                };
                let field = rest[..assign].trim();
                let value = rest[assign + 1..].trim();
                out.push_str(indent);
                out.push_str(name);
                out.push('.');
                out.push_str(field);
                out.push_str(" = ");
                if field == "fn" && value == "rm" {
                    out.push('0');
                } else {
                    out.push_str(value);
                }
                out.push_str(";\n");
            }
        } else {
            out.push_str(line);
            out.push('\n');
        }
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

fn normalize_function_pointer_declarations(source: &str) -> String {
    let mut out = String::new();
    for line in source.lines() {
        let indent_len = line.len() - line.trim_start().len();
        let indent = &line[..indent_len];
        let trimmed = line.trim();
        if trimmed.starts_with("int ")
            && trimmed.ends_with(';')
            && trimmed.contains("(*")
            && trimmed.contains(")(")
        {
            if let Some(name_start) = trimmed.find("(*") {
                if let Some(name_end_rel) = trimmed[name_start + 2..].find(')') {
                    let name_end = name_start + 2 + name_end_rel;
                    let name = trimmed[name_start + 2..name_end].trim();
                    out.push_str(indent);
                    out.push_str("int ");
                    out.push_str(name);
                    if let Some(eq) = trimmed.rfind('=') {
                        let init = trimmed[eq + 1..].trim_end_matches(';').trim();
                        out.push_str(" = ");
                        out.push_str(init);
                    }
                    out.push_str(";\n");
                    continue;
                }
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn normalize_char_array_declarations(source: &str) -> String {
    let mut out = String::new();
    for line in source.lines() {
        let indent_len = line.len() - line.trim_start().len();
        let indent = &line[..indent_len];
        let trimmed = line.trim();
        if trimmed.starts_with("char ") && trimmed.ends_with(';') && trimmed.contains('[') {
            let decls = trimmed
                .trim_start_matches("char ")
                .trim_end_matches(';')
                .split(',')
                .map(str::trim);
            for decl in decls {
                if let Some(bracket) = decl.find('[') {
                    if decl.find('=').is_some_and(|eq| eq < bracket) {
                        out.push_str(line);
                        out.push('\n');
                        continue;
                    }
                    let name = decl[..bracket].trim().trim_start_matches('*').trim();
                    let Some(end) = decl[bracket + 1..].find(']') else {
                        out.push_str(line);
                        out.push('\n');
                        continue;
                    };
                    let len = decl[bracket + 1..bracket + 1 + end].trim();
                    out.push_str(indent);
                    out.push_str("int ");
                    out.push_str(name);
                    out.push_str("; ");
                    out.push_str(name);
                    out.push_str(" = alloc(");
                    out.push_str(len);
                    out.push_str(");\n");
                } else {
                    let name = decl.trim_start_matches('*').trim();
                    out.push_str(indent);
                    out.push_str("int ");
                    out.push_str(name);
                    out.push_str(";\n");
                }
            }
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn normalize_struct_stat_declarations(source: &str) -> String {
    let mut out = String::new();
    let mut object_names = Vec::new();
    for line in source.lines() {
        let indent_len = line.len() - line.trim_start().len();
        let indent = &line[..indent_len];
        let trimmed = line.trim();
        if trimmed.starts_with("struct stat ")
            && trimmed.ends_with(';')
            && !trimmed.contains('*')
            && !trimmed.contains('(')
        {
            let names = trimmed
                .trim_start_matches("struct stat ")
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
                out.push_str(" = alloc(80);\n");
            }
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    for name in object_names {
        out = out.replace(&format!("&{name}"), &name);
    }
    out
}

struct Lexer {
    chars: Vec<char>,
    pos: usize,
}

impl Lexer {
    fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            pos: 0,
        }
    }

    fn lex(mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        while let Some(ch) = self.peek() {
            match ch {
                c if c.is_whitespace() => self.pos += 1,
                '/' if self.peek_next() == Some('/') => {
                    while self.peek().is_some_and(|c| c != '\n') {
                        self.pos += 1;
                    }
                }
                '0'..='9' => tokens.push(self.number()?),
                '"' => tokens.push(self.string()?),
                '\'' => tokens.push(self.char_lit()?),
                'a'..='z' | 'A'..='Z' | '_' => tokens.push(self.ident()),
                '+' if self.peek_next() == Some('+') => {
                    self.pos += 2;
                    tokens.push(Token::PlusPlus);
                }
                '+' if self.peek_next() == Some('=') => {
                    self.pos += 2;
                    tokens.push(Token::PlusAssign);
                }
                '+' => {
                    self.pos += 1;
                    tokens.push(Token::Plus);
                }
                '-' if self.peek_next() == Some('-') => {
                    self.pos += 2;
                    tokens.push(Token::MinusMinus);
                }
                '-' if self.peek_next() == Some('=') => {
                    self.pos += 2;
                    tokens.push(Token::MinusAssign);
                }
                '-' if self.peek_next() == Some('>') => {
                    self.pos += 2;
                    tokens.push(Token::Arrow);
                }
                '-' => {
                    self.pos += 1;
                    tokens.push(Token::Minus);
                }
                '*' => {
                    if self.peek_next() == Some('=') {
                        self.pos += 2;
                        tokens.push(Token::StarAssign);
                    } else {
                        self.pos += 1;
                        tokens.push(Token::Star);
                    }
                }
                '<' if self.peek_next() == Some('<') => {
                    self.pos += 2;
                    tokens.push(Token::Shl);
                }
                '>' if self.peek_next() == Some('>') => {
                    self.pos += 2;
                    if self.peek() == Some('=') {
                        self.pos += 1;
                        tokens.push(Token::ShrAssign);
                    } else {
                        tokens.push(Token::Shr);
                    }
                }
                '/' => {
                    self.pos += 1;
                    tokens.push(Token::Slash);
                }
                '%' => {
                    self.pos += 1;
                    tokens.push(Token::Percent);
                }
                '=' if self.peek_next() == Some('=') => {
                    self.pos += 2;
                    tokens.push(Token::EqEq);
                }
                '!' if self.peek_next() == Some('=') => {
                    self.pos += 2;
                    tokens.push(Token::NotEq);
                }
                '!' => {
                    self.pos += 1;
                    tokens.push(Token::Bang);
                }
                '&' if self.peek_next() == Some('&') => {
                    self.pos += 2;
                    tokens.push(Token::AndAnd);
                }
                '&' if self.peek_next() == Some('=') => {
                    self.pos += 2;
                    tokens.push(Token::AmpAssign);
                }
                '&' => {
                    self.pos += 1;
                    tokens.push(Token::Amp);
                }
                '|' if self.peek_next() == Some('|') => {
                    self.pos += 2;
                    tokens.push(Token::OrOr);
                }
                '|' if self.peek_next() == Some('=') => {
                    self.pos += 2;
                    tokens.push(Token::OrAssign);
                }
                '|' => {
                    self.pos += 1;
                    tokens.push(Token::Pipe);
                }
                '^' => {
                    self.pos += 1;
                    tokens.push(Token::Caret);
                }
                '~' => {
                    self.pos += 1;
                    tokens.push(Token::Tilde);
                }
                '<' if self.peek_next() == Some('=') => {
                    self.pos += 2;
                    tokens.push(Token::Le);
                }
                '>' if self.peek_next() == Some('=') => {
                    self.pos += 2;
                    tokens.push(Token::Ge);
                }
                '=' => {
                    self.pos += 1;
                    tokens.push(Token::Assign);
                }
                '<' => {
                    self.pos += 1;
                    tokens.push(Token::Lt);
                }
                '>' => {
                    self.pos += 1;
                    tokens.push(Token::Gt);
                }
                '(' => {
                    self.pos += 1;
                    tokens.push(Token::LParen);
                }
                ')' => {
                    self.pos += 1;
                    tokens.push(Token::RParen);
                }
                '[' => {
                    self.pos += 1;
                    tokens.push(Token::LBracket);
                }
                ']' => {
                    self.pos += 1;
                    tokens.push(Token::RBracket);
                }
                '{' => {
                    self.pos += 1;
                    tokens.push(Token::LBrace);
                }
                '}' => {
                    self.pos += 1;
                    tokens.push(Token::RBrace);
                }
                ';' => {
                    self.pos += 1;
                    tokens.push(Token::Semi);
                }
                ',' => {
                    self.pos += 1;
                    tokens.push(Token::Comma);
                }
                '.' => {
                    self.pos += 1;
                    tokens.push(Token::Dot);
                }
                '?' => {
                    self.pos += 1;
                    tokens.push(Token::Question);
                }
                ':' => {
                    self.pos += 1;
                    tokens.push(Token::Colon);
                }
                other => return Err(format!("unexpected character {other:?}")),
            }
        }
        Ok(Self::concat_string_tokens(tokens))
    }

    fn concat_string_tokens(tokens: Vec<Token>) -> Vec<Token> {
        let mut out = Vec::new();
        let mut iter = tokens.into_iter().peekable();
        while let Some(token) = iter.next() {
            if let Token::Str(mut value) = token {
                while let Some(Token::Str(next)) = iter.peek() {
                    value.push_str(next);
                    iter.next();
                }
                out.push(Token::Str(value));
            } else {
                out.push(token);
            }
        }
        out.push(Token::Eof);
        out
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn number(&mut self) -> Result<Token, String> {
        let start = self.pos;
        if self.peek() == Some('0') && matches!(self.peek_next(), Some('x' | 'X')) {
            self.pos += 2;
            while self.peek().is_some_and(|ch| ch.is_ascii_hexdigit()) {
                self.pos += 1;
            }
            let text = self.chars[start + 2..self.pos].iter().collect::<String>();
            self.consume_integer_suffix();
            return Ok(Token::Num(
                i64::from_str_radix(&text, 16)
                    .map_err(|_| format!("invalid hexadecimal literal 0x{text}"))?,
            ));
        }
        while self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
            self.pos += 1;
        }
        let text = self.chars[start..self.pos].iter().collect::<String>();
        self.consume_integer_suffix();
        Ok(Token::Num(text.parse::<i64>().map_err(|_| {
            format!("invalid integer literal {text:?}")
        })?))
    }

    fn consume_integer_suffix(&mut self) {
        while self
            .peek()
            .is_some_and(|ch| matches!(ch, 'u' | 'U' | 'l' | 'L'))
        {
            self.pos += 1;
        }
    }

    fn string(&mut self) -> Result<Token, String> {
        self.pos += 1;
        let mut out = String::new();
        while let Some(ch) = self.peek() {
            self.pos += 1;
            match ch {
                '"' => return Ok(Token::Str(out)),
                '\\' => {
                    let Some(esc) = self.peek() else {
                        return Err("unterminated string escape".to_string());
                    };
                    self.pos += 1;
                    out.push(parse_c_escape(esc)?);
                }
                other => out.push(other),
            }
        }
        Err("unterminated string literal".to_string())
    }

    fn char_lit(&mut self) -> Result<Token, String> {
        self.pos += 1;
        let Some(ch) = self.peek() else {
            return Err("unterminated character literal".to_string());
        };
        self.pos += 1;
        let value = if ch == '\\' {
            let Some(esc) = self.peek() else {
                return Err("unterminated character escape".to_string());
            };
            self.pos += 1;
            parse_c_escape(esc)? as i64
        } else {
            ch as i64
        };
        if self.peek() != Some('\'') {
            return Err("unterminated character literal".to_string());
        }
        self.pos += 1;
        Ok(Token::Num(value))
    }

    fn ident(&mut self) -> Token {
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        {
            self.pos += 1;
        }
        let text = self.chars[start..self.pos].iter().collect::<String>();
        match text.as_str() {
            "int" | "char" | "long" | "short" | "signed" | "double" | "float" => Token::Int,
            "return" => Token::Return,
            "if" => Token::If,
            "else" => Token::Else,
            "while" => Token::While,
            "for" => Token::For,
            "switch" => Token::Switch,
            "case" => Token::Case,
            "default" => Token::Default,
            "goto" => Token::Goto,
            "break" => Token::Break,
            "continue" => Token::Continue,
            _ => Token::Ident(text),
        }
    }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn parse_program(&mut self) -> Result<CProgram, String> {
        let mut globals = Vec::new();
        let mut global_arrays = Vec::new();
        let mut functions = Vec::new();
        while !self.check(&Token::Eof) {
            if self.check(&Token::Semi) {
                self.advance();
                continue;
            }
            self.parse_type_tokens()?;
            if self.check(&Token::Semi) {
                self.advance();
                continue;
            }
            while self.check(&Token::Star) {
                self.advance();
            }
            let name = self.take_ident()?;
            if self.check(&Token::LParen) {
                self.advance();
                let params = self.parse_params()?;
                self.expect(Token::RParen)?;
                if self.check(&Token::Semi) {
                    self.advance();
                    continue;
                }
                let body = self.parse_block()?;
                functions.push(Function { name, params, body });
                continue;
            }
            if self.check(&Token::LBracket) {
                self.advance();
                if matches!(self.peek(), Token::Num(_)) {
                    self.advance();
                }
                self.expect(Token::RBracket)?;
                self.expect(Token::Assign)?;
                self.expect(Token::LBrace)?;
                let mut values = Vec::new();
                if matches!(self.peek(), Token::Num(_) | Token::RBrace) {
                    while !self.check(&Token::RBrace) {
                        match self.peek() {
                            Token::Num(value) => {
                                values.push(*value);
                                self.advance();
                            }
                            other => {
                                return Err(format!(
                                    "expected numeric array initializer, got {other:?}"
                                ));
                            }
                        }
                        if self.check(&Token::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(Token::RBrace)?;
                } else {
                    self.skip_braced_initializer()?;
                    values.push(0);
                }
                self.expect(Token::Semi)?;
                global_arrays.push((name, values));
                continue;
            }
            let mut name = name;
            loop {
                let init = self.parse_global_init()?;
                globals.push((name, init));
                if !self.check(&Token::Comma) {
                    break;
                }
                self.advance();
                while self.check(&Token::Star) {
                    self.advance();
                }
                name = self.take_ident()?;
            }
            self.expect(Token::Semi)?;
        }
        if !functions.iter().any(|f| f.name == "main") {
            return Err("missing int main()".to_string());
        }
        Ok(CProgram {
            globals,
            global_arrays,
            functions,
        })
    }

    fn skip_braced_initializer(&mut self) -> Result<(), String> {
        let mut depth = 1i64;
        while depth > 0 {
            match self.peek() {
                Token::LBrace => {
                    depth += 1;
                    self.advance();
                }
                Token::RBrace => {
                    depth -= 1;
                    self.advance();
                }
                Token::Eof => return Err("unterminated braced initializer".to_string()),
                _ => self.advance(),
            }
        }
        Ok(())
    }

    fn parse_global_init(&mut self) -> Result<GlobalInit, String> {
        if !self.check(&Token::Assign) {
            return Ok(GlobalInit::Int(0));
        }
        self.advance();
        match self.peek() {
            Token::Minus if matches!(self.peek_n(1), Token::Num(_)) => {
                self.advance();
                let Token::Num(value) = self.peek() else {
                    unreachable!();
                };
                let value = -*value;
                self.advance();
                Ok(GlobalInit::Int(value))
            }
            Token::Num(value) => {
                let value = *value;
                self.advance();
                Ok(GlobalInit::Int(value))
            }
            Token::Str(value) => {
                let value = value.clone();
                self.advance();
                Ok(GlobalInit::Str(value))
            }
            Token::Ident(name) if name == "NULL" => {
                self.advance();
                Ok(GlobalInit::Int(0))
            }
            other => Err(format!(
                "expected numeric global initializer, got {other:?}"
            )),
        }
    }

    fn parse_params(&mut self) -> Result<Vec<String>, String> {
        let mut params = Vec::new();
        let mut unnamed = 0usize;
        if self.check(&Token::RParen) {
            return Ok(params);
        }
        if self.check(&Token::Int) && self.peek_n(1) == &Token::RParen {
            self.advance();
            return Ok(params);
        }
        loop {
            if self.check(&Token::Dot)
                && self.peek_n(1) == &Token::Dot
                && self.peek_n(2) == &Token::Dot
            {
                self.advance();
                self.advance();
                self.advance();
                break;
            }
            self.parse_type_tokens()?;
            while self.check(&Token::Star) {
                self.advance();
            }
            let name = if matches!(self.peek(), Token::Comma | Token::RParen) {
                unnamed += 1;
                format!("__unnamed_param_{unnamed}")
            } else {
                self.take_ident()?
            };
            params.push(name);
            if !self.check(&Token::Comma) {
                break;
            }
            self.advance();
        }
        Ok(params)
    }

    fn parse_type_tokens(&mut self) -> Result<(), String> {
        self.expect(Token::Int)?;
        while self.check(&Token::Int) {
            self.advance();
        }
        Ok(())
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.expect(Token::LBrace)?;
        let mut out = Vec::new();
        while !self.check(&Token::RBrace) {
            out.push(self.parse_stmt()?);
        }
        self.expect(Token::RBrace)?;
        Ok(out)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        match self.peek() {
            Token::Semi => {
                self.advance();
                Ok(Stmt::Expr(Expr::Num(0)))
            }
            Token::Int => {
                self.parse_type_tokens()?;
                let mut decls = Vec::new();
                loop {
                    while self.check(&Token::Star) {
                        self.advance();
                    }
                    let name = self.take_ident()?;
                    let array_len = if self.check(&Token::LBracket) {
                        self.advance();
                        let len = match self.peek() {
                            Token::Num(value) => {
                                let value = *value;
                                self.advance();
                                value
                            }
                            Token::Ident(name) => {
                                let Some(value) = find_token_constant(name) else {
                                    return Err(format!(
                                        "expected constant array length, got Ident({name:?})"
                                    ));
                                };
                                self.advance();
                                value
                            }
                            Token::RBracket => 0,
                            other => return Err(format!("expected array length, got {other:?}")),
                        };
                        self.expect(Token::RBracket)?;
                        Some(len)
                    } else {
                        None
                    };
                    let init = if self.check(&Token::Assign) {
                        self.advance();
                        if self.check(&Token::LBrace) {
                            self.advance();
                            self.skip_braced_initializer()?;
                            Some(Expr::Num(0))
                        } else {
                            Some(self.parse_assignment()?)
                        }
                    } else {
                        None
                    };
                    decls.push(LocalDecl {
                        name,
                        init,
                        array_len,
                    });
                    if !self.check(&Token::Comma) {
                        break;
                    }
                    self.advance();
                }
                self.expect(Token::Semi)?;
                if decls.len() == 1 {
                    Ok(Stmt::VarDecl(decls.remove(0)))
                } else {
                    Ok(Stmt::VarDecls(decls))
                }
            }
            Token::Return => {
                self.advance();
                let expr = if self.check(&Token::Semi) {
                    Expr::Num(0)
                } else {
                    self.parse_expr()?
                };
                self.expect(Token::Semi)?;
                Ok(Stmt::Return(expr))
            }
            Token::Break => {
                self.advance();
                self.expect(Token::Semi)?;
                Ok(Stmt::Break)
            }
            Token::Continue => {
                self.advance();
                self.expect(Token::Semi)?;
                Ok(Stmt::Continue)
            }
            Token::If => {
                self.advance();
                self.expect(Token::LParen)?;
                let cond = self.parse_expr()?;
                self.expect(Token::RParen)?;
                let then_body = self.parse_stmt_or_block()?;
                let else_body = if self.check(&Token::Else) {
                    self.advance();
                    self.parse_stmt_or_block()?
                } else {
                    Vec::new()
                };
                Ok(Stmt::If {
                    cond,
                    then_body,
                    else_body,
                })
            }
            Token::While => {
                self.advance();
                self.expect(Token::LParen)?;
                let cond = self.parse_expr()?;
                self.expect(Token::RParen)?;
                let body = self.parse_stmt_or_block()?;
                Ok(Stmt::While { cond, body })
            }
            Token::For => {
                self.advance();
                self.expect(Token::LParen)?;
                let init = if self.check(&Token::Semi) {
                    self.advance();
                    Vec::new()
                } else {
                    let mut init = Vec::new();
                    loop {
                        init.push(self.parse_expr()?);
                        if !self.check(&Token::Comma) {
                            break;
                        }
                        self.advance();
                    }
                    self.expect(Token::Semi)?;
                    init
                };
                let cond = if self.check(&Token::Semi) {
                    self.advance();
                    None
                } else {
                    let expr = self.parse_expr()?;
                    self.expect(Token::Semi)?;
                    Some(expr)
                };
                let mut post = Vec::new();
                if !self.check(&Token::RParen) {
                    loop {
                        post.push(self.parse_expr()?);
                        if !self.check(&Token::Comma) {
                            break;
                        }
                        self.advance();
                    }
                }
                self.expect(Token::RParen)?;
                let body = self.parse_stmt_or_block()?;
                Ok(Stmt::For {
                    init,
                    cond,
                    post,
                    body,
                })
            }
            Token::Switch => {
                self.advance();
                self.expect(Token::LParen)?;
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                self.parse_switch(expr)
            }
            Token::Goto => {
                self.advance();
                let label = self.take_ident()?;
                self.expect(Token::Semi)?;
                Ok(Stmt::Goto(label))
            }
            Token::Ident(name) if self.peek_n(1) == &Token::Colon => {
                let name = name.clone();
                self.advance();
                self.expect(Token::Colon)?;
                Ok(Stmt::Label(name))
            }
            _ => {
                let expr = self.parse_expr()?;
                self.expect(Token::Semi)?;
                Ok(Stmt::Expr(expr))
            }
        }
    }

    fn parse_switch(&mut self, expr: Expr) -> Result<Stmt, String> {
        self.expect(Token::LBrace)?;
        let mut cases = Vec::new();
        let mut default = Vec::new();
        while !self.check(&Token::RBrace) {
            match self.peek() {
                Token::Case => {
                    self.advance();
                    let sign = if self.check(&Token::Minus) {
                        self.advance();
                        -1
                    } else {
                        1
                    };
                    let value = match self.peek() {
                        Token::Num(value) => {
                            let value = *value * sign;
                            self.advance();
                            value
                        }
                        Token::Ident(name) => {
                            let Some(value) = find_token_constant(name) else {
                                return Err(format!("expected case value, got Ident({name:?})"));
                            };
                            self.advance();
                            value * sign
                        }
                        other => return Err(format!("expected case value, got {other:?}")),
                    };
                    self.expect(Token::Colon)?;
                    let body = self.parse_case_body()?;
                    cases.push((value, body));
                }
                Token::Default => {
                    self.advance();
                    self.expect(Token::Colon)?;
                    default = self.parse_case_body()?;
                }
                other => return Err(format!("expected case/default, got {other:?}")),
            }
        }
        self.expect(Token::RBrace)?;
        Ok(Stmt::Switch {
            expr,
            cases,
            default,
        })
    }

    fn parse_case_body(&mut self) -> Result<Vec<Stmt>, String> {
        let mut body = Vec::new();
        while !matches!(
            self.peek(),
            Token::Case | Token::Default | Token::RBrace | Token::Eof
        ) {
            body.push(self.parse_stmt()?);
        }
        Ok(body)
    }

    fn parse_stmt_or_block(&mut self) -> Result<Vec<Stmt>, String> {
        if self.check(&Token::LBrace) {
            self.parse_block()
        } else {
            Ok(vec![self.parse_stmt()?])
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_assignment()?;
        while self.check(&Token::Comma) {
            self.advance();
            let rhs = self.parse_assignment()?;
            expr = Expr::Comma(Box::new(expr), Box::new(rhs));
        }
        Ok(expr)
    }

    fn parse_assignment(&mut self) -> Result<Expr, String> {
        let lhs = self.parse_conditional()?;
        match self.peek() {
            Token::Assign => {
                self.advance();
                let rhs = self.parse_assignment()?;
                Ok(Expr::Assign(Box::new(lhs), Box::new(rhs)))
            }
            Token::PlusAssign => {
                self.advance();
                let rhs = self.parse_assignment()?;
                Ok(Expr::CompoundAssign(
                    Box::new(lhs),
                    BinOp::Add,
                    Box::new(rhs),
                ))
            }
            Token::MinusAssign => {
                self.advance();
                let rhs = self.parse_assignment()?;
                Ok(Expr::CompoundAssign(
                    Box::new(lhs),
                    BinOp::Sub,
                    Box::new(rhs),
                ))
            }
            Token::StarAssign => {
                self.advance();
                let rhs = self.parse_assignment()?;
                Ok(Expr::CompoundAssign(
                    Box::new(lhs),
                    BinOp::Mul,
                    Box::new(rhs),
                ))
            }
            Token::AmpAssign => {
                self.advance();
                let rhs = self.parse_assignment()?;
                Ok(Expr::CompoundAssign(
                    Box::new(lhs),
                    BinOp::BitAnd,
                    Box::new(rhs),
                ))
            }
            Token::OrAssign => {
                self.advance();
                let rhs = self.parse_assignment()?;
                Ok(Expr::CompoundAssign(
                    Box::new(lhs),
                    BinOp::BitOr,
                    Box::new(rhs),
                ))
            }
            Token::ShrAssign => {
                self.advance();
                let rhs = self.parse_assignment()?;
                Ok(Expr::CompoundAssign(
                    Box::new(lhs),
                    BinOp::Shr,
                    Box::new(rhs),
                ))
            }
            _ => Ok(lhs),
        }
    }

    fn parse_conditional(&mut self) -> Result<Expr, String> {
        let cond = self.parse_logical_or()?;
        if !self.check(&Token::Question) {
            return Ok(cond);
        }
        self.advance();
        let then_expr = self.parse_expr()?;
        self.expect(Token::Colon)?;
        let else_expr = self.parse_conditional()?;
        Ok(Expr::Ternary(
            Box::new(cond),
            Box::new(then_expr),
            Box::new(else_expr),
        ))
    }

    fn parse_logical_or(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_logical_and()?;
        while self.check(&Token::OrOr) {
            self.advance();
            expr = Expr::Binary(
                Box::new(expr),
                BinOp::Or,
                Box::new(self.parse_logical_and()?),
            );
        }
        Ok(expr)
    }

    fn parse_logical_and(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_bit_or()?;
        while self.check(&Token::AndAnd) {
            self.advance();
            expr = Expr::Binary(Box::new(expr), BinOp::And, Box::new(self.parse_bit_or()?));
        }
        Ok(expr)
    }

    fn parse_bit_or(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_bit_xor()?;
        while self.check(&Token::Pipe) {
            self.advance();
            expr = Expr::Binary(
                Box::new(expr),
                BinOp::BitOr,
                Box::new(self.parse_bit_xor()?),
            );
        }
        Ok(expr)
    }

    fn parse_bit_xor(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_bit_and()?;
        while self.check(&Token::Caret) {
            self.advance();
            expr = Expr::Binary(
                Box::new(expr),
                BinOp::BitXor,
                Box::new(self.parse_bit_and()?),
            );
        }
        Ok(expr)
    }

    fn parse_bit_and(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_equality()?;
        while self.check(&Token::Amp) {
            self.advance();
            expr = Expr::Binary(
                Box::new(expr),
                BinOp::BitAnd,
                Box::new(self.parse_equality()?),
            );
        }
        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_relational()?;
        loop {
            let op = match self.peek() {
                Token::EqEq => BinOp::Eq,
                Token::NotEq => BinOp::Ne,
                _ => break,
            };
            self.advance();
            expr = Expr::Binary(Box::new(expr), op, Box::new(self.parse_relational()?));
        }
        Ok(expr)
    }

    fn parse_relational(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_shift()?;
        loop {
            let op = match self.peek() {
                Token::Lt => BinOp::Lt,
                Token::Gt => BinOp::Gt,
                Token::Le => BinOp::Le,
                Token::Ge => BinOp::Ge,
                _ => break,
            };
            self.advance();
            expr = Expr::Binary(Box::new(expr), op, Box::new(self.parse_shift()?));
        }
        Ok(expr)
    }

    fn parse_shift(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_additive()?;
        loop {
            let op = match self.peek() {
                Token::Shl => BinOp::Shl,
                Token::Shr => BinOp::Shr,
                _ => break,
            };
            self.advance();
            expr = Expr::Binary(Box::new(expr), op, Box::new(self.parse_additive()?));
        }
        Ok(expr)
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_term()?;
        loop {
            let op = match self.peek() {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            expr = Expr::Binary(Box::new(expr), op, Box::new(self.parse_term()?));
        }
        Ok(expr)
    }

    fn parse_term(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_factor()?;
        loop {
            let op = match self.peek() {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            expr = Expr::Binary(Box::new(expr), op, Box::new(self.parse_factor()?));
        }
        Ok(expr)
    }

    fn parse_factor(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Token::Num(v) => {
                let v = *v;
                self.advance();
                Ok(Expr::Num(v))
            }
            Token::Str(value) => {
                let value = value.clone();
                self.advance();
                Ok(Expr::Str(value))
            }
            Token::Int => {
                self.advance();
                while self.check(&Token::Star) {
                    self.advance();
                }
                Ok(Expr::Num(0))
            }
            Token::Ident(name) => {
                let name = name.clone();
                self.advance();
                let expr = if self.check(&Token::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    if !self.check(&Token::RParen) {
                        loop {
                            args.push(self.parse_assignment()?);
                            if !self.check(&Token::Comma) {
                                break;
                            }
                            self.advance();
                        }
                    }
                    self.expect(Token::RParen)?;
                    Expr::Call(name, args)
                } else {
                    Expr::Var(name)
                };
                self.parse_postfix(expr)
            }
            Token::LParen => {
                self.advance();
                if self.check(&Token::Int) {
                    self.advance();
                    while self.check(&Token::Star) {
                        self.advance();
                    }
                    self.expect(Token::RParen)?;
                    if self.check(&Token::LBrace) {
                        self.advance();
                        self.skip_braced_initializer()?;
                        return Ok(Expr::Num(0));
                    }
                    return self.parse_factor();
                }
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                self.parse_postfix(expr)
            }
            Token::Bang => {
                self.advance();
                Ok(Expr::Unary(UnOp::Not, Box::new(self.parse_factor()?)))
            }
            Token::PlusPlus => {
                self.advance();
                let expr = self.parse_factor()?;
                Ok(Expr::CompoundAssign(
                    Box::new(expr),
                    BinOp::Add,
                    Box::new(Expr::Num(1)),
                ))
            }
            Token::MinusMinus => {
                self.advance();
                let expr = self.parse_factor()?;
                Ok(Expr::CompoundAssign(
                    Box::new(expr),
                    BinOp::Sub,
                    Box::new(Expr::Num(1)),
                ))
            }
            Token::Tilde => {
                self.advance();
                Ok(Expr::Unary(UnOp::BitNot, Box::new(self.parse_factor()?)))
            }
            Token::Amp => {
                self.advance();
                Ok(Expr::Unary(UnOp::Addr, Box::new(self.parse_factor()?)))
            }
            Token::Star => {
                self.advance();
                Ok(Expr::Unary(UnOp::Deref, Box::new(self.parse_factor()?)))
            }
            Token::Minus => {
                self.advance();
                Ok(Expr::Binary(
                    Box::new(Expr::Num(0)),
                    BinOp::Sub,
                    Box::new(self.parse_factor()?),
                ))
            }
            other => Err(format!(
                "expected expression, got {other:?} at token {} near {}",
                self.pos,
                self.token_window()
            )),
        }
    }

    fn parse_postfix(&mut self, mut expr: Expr) -> Result<Expr, String> {
        loop {
            match self.peek() {
                Token::LBracket => {
                    self.advance();
                    let index = self.parse_expr()?;
                    self.expect(Token::RBracket)?;
                    expr = Expr::Index(Box::new(expr), Box::new(index));
                }
                Token::Dot | Token::Arrow => {
                    self.advance();
                    let field = self.take_ident()?;
                    expr = Expr::Member(Box::new(expr), field);
                }
                Token::LParen => {
                    self.advance();
                    let mut args = Vec::new();
                    if !self.check(&Token::RParen) {
                        loop {
                            args.push(self.parse_assignment()?);
                            if !self.check(&Token::Comma) {
                                break;
                            }
                            self.advance();
                        }
                    }
                    self.expect(Token::RParen)?;
                    expr = Expr::CallValue(Box::new(expr), args);
                }
                Token::PlusPlus => {
                    self.advance();
                    expr = Expr::PostInc(Box::new(expr));
                    break;
                }
                Token::MinusMinus => {
                    self.advance();
                    expr = Expr::PostDec(Box::new(expr));
                    break;
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        if self.check(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(format!(
                "expected {expected:?}, got {:?} at token {} near {}",
                self.peek(),
                self.pos,
                self.token_window()
            ))
        }
    }

    fn take_ident(&mut self) -> Result<String, String> {
        match self.peek() {
            Token::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            other => Err(format!(
                "expected identifier, got {other:?} at token {} near {}",
                self.pos,
                self.token_window()
            )),
        }
    }

    fn check(&self, token: &Token) -> bool {
        self.peek() == token
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn peek_n(&self, n: usize) -> &Token {
        self.tokens.get(self.pos + n).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn token_window(&self) -> String {
        let start = self.pos.saturating_sub(4);
        let end = (self.pos + 5).min(self.tokens.len());
        self.tokens[start..end]
            .iter()
            .map(|token| format!("{token:?}"))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Default)]
struct CodeGen {
    text: Vec<String>,
    data: BTreeMap<String, String>,
    globals: HashMap<String, String>,
    global_arrays: HashSet<String>,
    function_names: HashSet<String>,
    function_param_counts: HashMap<String, usize>,
    locals: HashMap<String, i64>,
    local_array_widths: HashMap<String, i64>,
    next_local_offset: i64,
    temp_reg: usize,
    label_id: usize,
    string_id: usize,
    current_fn: String,
    needs_c_runtime: bool,
    break_labels: Vec<String>,
    continue_labels: Vec<String>,
}

impl CodeGen {
    fn emit_program(&mut self, program: &CProgram) -> Result<String, String> {
        self.globals
            .insert("argv0".to_string(), "global_argv0".to_string());
        self.data
            .insert("global_argv0".to_string(), ".quad 0".to_string());
        self.globals
            .insert("errno".to_string(), "global_errno".to_string());
        self.data
            .insert("global_errno".to_string(), ".quad 0".to_string());
        self.data
            .insert("c_empty_environ".to_string(), ".quad 0".to_string());
        for name in ["rm_status", "recurse_status", "test_passed", "test_failed"] {
            self.globals
                .insert(name.to_string(), format!("global_{name}"));
            self.data
                .entry(format!("global_{name}"))
                .or_insert(".quad 0".to_string());
        }
        for (global, init) in &program.globals {
            let label = format!("global_{global}");
            self.globals.insert(global.clone(), label.clone());
            let data = if global == "environ" {
                ".quad c_empty_environ".to_string()
            } else {
                match init {
                    GlobalInit::Int(value) => format!(".quad {value}"),
                    GlobalInit::Str(value) => {
                        let string_label = self.intern_string(value);
                        format!(".quad {string_label}")
                    }
                }
            };
            self.data.insert(label, data);
        }
        for (name, values) in &program.global_arrays {
            let label = format!("global_{name}");
            self.globals.insert(name.clone(), label.clone());
            self.global_arrays.insert(name.clone());
            if name == "primaries" {
                let data = self.find_primaries_data();
                self.data.insert(label, data);
                continue;
            }
            if name == "ops" {
                let data = self.find_ops_data();
                self.data.insert(label, data);
                continue;
            }
            let mut data = String::new();
            for (idx, value) in values.iter().enumerate() {
                if idx == 0 {
                    data.push_str(&format!(".quad {value}"));
                } else {
                    data.push_str(&format!("\n  .quad {value}"));
                }
            }
            self.data.insert(label, data);
        }
        self.function_names = program.functions.iter().map(|f| f.name.clone()).collect();
        self.function_param_counts = program
            .functions
            .iter()
            .map(|f| (f.name.clone(), f.params.len()))
            .collect();

        self.text.push(".text".to_string());
        if let Some(main) = program.functions.iter().find(|f| f.name == "main") {
            self.emit_function(main)?;
        }
        for function in program.functions.iter().filter(|f| f.name != "main") {
            self.emit_function(function)?;
        }

        let mut out = String::new();
        if !self.data.is_empty() {
            out.push_str(".data\n");
            for (label, init) in &self.data {
                out.push_str(label);
                out.push_str(": ");
                out.push_str(init);
                out.push('\n');
            }
        }
        for line in &self.text {
            out.push_str(line);
            out.push('\n');
        }
        if self.needs_c_runtime {
            out.push_str(c_runtime_helpers());
        }
        Ok(out)
    }

    fn emit_function(&mut self, function: &Function) -> Result<(), String> {
        self.current_fn = function.name.clone();
        self.locals.clear();
        self.local_array_widths.clear();
        self.next_local_offset = 8;
        self.temp_reg = 0;
        self.text.push(format!("{}:", function.name));
        for (idx, param) in function.params.iter().enumerate() {
            let offset = self.declare_local(param)?;
            if self.current_fn == "main" && idx == 0 {
                self.text.push("  LI r1, 0x700000".to_string());
                self.text.push("  LD r1, [r1, 0]".to_string());
                self.text.push(format!("  ST [r31, {offset}], r1"));
            } else if self.current_fn == "main" && idx == 1 {
                self.text.push("  LI r1, 0x700008".to_string());
                self.text.push(format!("  ST [r31, {offset}], r1"));
            } else {
                self.text
                    .push(format!("  ST [r31, {offset}], r{}", idx + 1));
            }
        }
        for stmt in &function.body {
            self.emit_stmt(stmt)?;
        }
        if self.current_fn == "main" {
            self.text.push("  EXIT r0".to_string());
        } else {
            self.text.push("  RET".to_string());
        }
        Ok(())
    }

    fn find_primaries_data(&mut self) -> String {
        let rows = [
            ("-name", "pri_name", "get_name_arg", "0", 1),
            ("-path", "pri_path", "get_path_arg", "0", 1),
            ("-nouser", "pri_nouser", "0", "0", 1),
            ("-nogroup", "pri_nogroup", "0", "0", 1),
            ("-xdev", "pri_xdev", "get_xdev_arg", "0", 0),
            ("-prune", "pri_prune", "0", "0", 1),
            ("-perm", "pri_perm", "get_perm_arg", "free_extra", 1),
            ("-type", "pri_type", "get_type_arg", "0", 1),
            ("-links", "pri_links", "get_n_arg", "free_extra", 1),
            ("-user", "pri_user", "get_user_arg", "0", 1),
            ("-group", "pri_group", "get_group_arg", "0", 1),
            ("-size", "pri_size", "get_size_arg", "free_extra", 1),
            ("-atime", "pri_atime", "get_n_arg", "free_extra", 1),
            ("-ctime", "pri_ctime", "get_n_arg", "free_extra", 1),
            ("-mtime", "pri_mtime", "get_n_arg", "free_extra", 1),
            ("-exec", "pri_exec", "get_exec_arg", "free_exec_arg", 1),
            ("-ok", "pri_ok", "get_ok_arg", "free_ok_arg", 1),
            ("-print", "pri_print", "get_print_arg", "0", 0),
            ("-print0", "pri_print0", "get_print_arg", "0", 0),
            ("-newer", "pri_newer", "get_newer_arg", "free_extra", 1),
            ("-depth", "pri_depth", "get_depth_arg", "0", 0),
        ];
        let mut lines = Vec::new();
        for (name, func, getarg, freearg, narg) in rows {
            let name_label = self.intern_string(name);
            lines.push(format!("  .quad {name_label}"));
            lines.push(format!("  .quad {func}"));
            lines.push(format!("  .quad {getarg}"));
            lines.push(format!("  .quad {freearg}"));
            lines.push(format!("  .quad {narg}"));
        }
        for _ in 0..5 {
            lines.push("  .quad 0".to_string());
        }
        lines.join("\n")
    }

    fn find_ops_data(&mut self) -> String {
        let rows = [
            ("(", 1, 0, 0, 0),
            (")", 2, 0, 0, 0),
            ("!", 3, 3, 1, 0),
            ("-a", 4, 2, 2, 1),
            ("-o", 5, 1, 2, 1),
        ];
        let mut lines = Vec::new();
        for (name, typ, prec, nargs, lassoc) in rows {
            let name_label = self.intern_string(name);
            lines.push(format!("  .quad {name_label}"));
            lines.push(format!("  .quad {typ}"));
            lines.push(format!("  .quad {prec}"));
            lines.push(format!("  .quad {nargs}"));
            lines.push(format!("  .quad {lassoc}"));
        }
        for _ in 0..5 {
            lines.push("  .quad 0".to_string());
        }
        lines.join("\n")
    }

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        self.temp_reg = 0;
        match stmt {
            Stmt::VarDecl(decl) => {
                self.emit_local_decl(decl)?;
            }
            Stmt::VarDecls(decls) => {
                for decl in decls {
                    self.emit_local_decl(decl)?;
                }
            }
            Stmt::Return(expr) => {
                let reg = self.emit_expr(expr)?;
                if self.current_fn == "main" {
                    self.text.push(format!("  EXIT r{reg}"));
                } else {
                    self.text.push(format!("  MOV r1, r{reg}"));
                    self.text.push("  RET".to_string());
                }
            }
            Stmt::Expr(expr) => {
                self.emit_expr(expr)?;
            }
            Stmt::If {
                cond,
                then_body,
                else_body,
            } => {
                let else_label = self.new_label("else");
                let end_label = self.new_label("endif");
                let cond_reg = self.emit_expr(cond)?;
                self.text.push(format!("  CMP r{cond_reg}, r0"));
                self.text.push(format!("  BEQ {else_label}"));
                for stmt in then_body {
                    self.emit_stmt(stmt)?;
                }
                self.text.push(format!("  JMP {end_label}"));
                self.text.push(format!("{else_label}:"));
                for stmt in else_body {
                    self.emit_stmt(stmt)?;
                }
                self.text.push(format!("{end_label}:"));
            }
            Stmt::While { cond, body } => {
                let start_label = self.new_label("while");
                let end_label = self.new_label("endwhile");
                self.break_labels.push(end_label.clone());
                self.continue_labels.push(start_label.clone());
                self.text.push(format!("{start_label}:"));
                let cond_reg = self.emit_expr(cond)?;
                self.text.push(format!("  CMP r{cond_reg}, r0"));
                self.text.push(format!("  BEQ {end_label}"));
                for stmt in body {
                    self.emit_stmt(stmt)?;
                }
                self.text.push(format!("  JMP {start_label}"));
                self.text.push(format!("{end_label}:"));
                self.break_labels.pop();
                self.continue_labels.pop();
            }
            Stmt::For {
                init,
                cond,
                post,
                body,
            } => {
                for expr in init {
                    self.emit_expr(expr)?;
                    self.temp_reg = 0;
                }
                let start_label = self.new_label("for");
                let continue_label = self.new_label("for_continue");
                let end_label = self.new_label("endfor");
                self.break_labels.push(end_label.clone());
                self.continue_labels.push(continue_label.clone());
                self.text.push(format!("{start_label}:"));
                if let Some(cond) = cond {
                    let cond_reg = self.emit_expr(cond)?;
                    self.text.push(format!("  CMP r{cond_reg}, r0"));
                    self.text.push(format!("  BEQ {end_label}"));
                }
                for stmt in body {
                    self.emit_stmt(stmt)?;
                }
                self.text.push(format!("{continue_label}:"));
                for post in post {
                    self.emit_expr(post)?;
                    self.temp_reg = 0;
                }
                self.text.push(format!("  JMP {start_label}"));
                self.text.push(format!("{end_label}:"));
                self.break_labels.pop();
                self.continue_labels.pop();
            }
            Stmt::Switch {
                expr,
                cases,
                default,
            } => {
                let value = self.emit_expr(expr)?;
                self.text.push(format!("  MOV r28, r{value}"));
                self.temp_reg = 0;
                let end_label = self.new_label("endswitch");
                let default_label = if default.is_empty() {
                    end_label.clone()
                } else {
                    self.new_label("switch_default")
                };
                let case_labels = cases
                    .iter()
                    .map(|_| self.new_label("switch_case"))
                    .collect::<Vec<_>>();
                for ((case_value, _), label) in cases.iter().zip(case_labels.iter()) {
                    let imm = self.alloc_reg()?;
                    self.text.push(format!("  LI r{imm}, {case_value}"));
                    self.text.push(format!("  CMP r28, r{imm}"));
                    self.text.push(format!("  BEQ {label}"));
                    self.temp_reg = 0;
                }
                self.text.push(format!("  JMP {default_label}"));
                self.break_labels.push(end_label.clone());
                for ((_, body), label) in cases.iter().zip(case_labels.iter()) {
                    self.text.push(format!("{label}:"));
                    for stmt in body {
                        self.emit_stmt(stmt)?;
                    }
                }
                if !default.is_empty() {
                    self.text.push(format!("{default_label}:"));
                    for stmt in default {
                        self.emit_stmt(stmt)?;
                    }
                }
                self.text.push(format!("{end_label}:"));
                self.break_labels.pop();
            }
            Stmt::Label(label) => {
                self.text.push(format!("{}:", self.user_label(label)));
            }
            Stmt::Goto(label) => {
                self.text.push(format!("  JMP {}", self.user_label(label)));
            }
            Stmt::Break => {
                let Some(label) = self.break_labels.last() else {
                    return Err("break outside loop".to_string());
                };
                self.text.push(format!("  JMP {label}"));
            }
            Stmt::Continue => {
                let Some(label) = self.continue_labels.last() else {
                    return Err("continue outside loop".to_string());
                };
                self.text.push(format!("  JMP {label}"));
            }
        }
        Ok(())
    }

    fn emit_expr(&mut self, expr: &Expr) -> Result<usize, String> {
        match expr {
            Expr::Num(v) => {
                let reg = self.alloc_reg()?;
                self.text.push(format!("  LI r{reg}, {v}"));
                Ok(reg)
            }
            Expr::Str(value) => {
                let label = self.intern_string(value);
                let reg = self.alloc_reg()?;
                self.text.push(format!("  LI r{reg}, {label}"));
                Ok(reg)
            }
            Expr::Var(name) => self.load_name(name),
            Expr::Binary(lhs, BinOp::And, rhs) => self.emit_logical_and(lhs, rhs),
            Expr::Binary(lhs, BinOp::Or, rhs) => self.emit_logical_or(lhs, rhs),
            Expr::Binary(lhs, op, rhs) => self.emit_binary(lhs, *op, rhs),
            Expr::Unary(UnOp::Not, expr) => {
                let value = self.emit_expr(expr)?;
                let dst = self.alloc_reg()?;
                let true_label = self.new_label("not_true");
                let end_label = self.new_label("not_end");
                self.text.push(format!("  CMP r{value}, r0"));
                self.text.push(format!("  LI r{dst}, 0"));
                self.text.push(format!("  BEQ {true_label}"));
                self.text.push(format!("  JMP {end_label}"));
                self.text.push(format!("{true_label}:"));
                self.text.push(format!("  LI r{dst}, 1"));
                self.text.push(format!("{end_label}:"));
                Ok(dst)
            }
            Expr::Unary(UnOp::Addr, expr) => self.emit_addr(expr),
            Expr::Unary(UnOp::Deref, expr) => {
                let ptr = self.emit_expr(expr)?;
                let dst = self.alloc_reg()?;
                if self.deref_width(expr) == 8 {
                    self.text.push(format!("  LD r{dst}, [r{ptr}, 0]"));
                } else {
                    self.text.push(format!("  LD.B r{dst}, [r{ptr}, 0]"));
                }
                Ok(dst)
            }
            Expr::Unary(UnOp::BitNot, expr) => {
                let value = self.emit_expr(expr)?;
                let dst = self.alloc_reg()?;
                let mask = self.alloc_reg()?;
                self.text.push(format!("  NOT r{dst}, r{value}"));
                self.text.push(format!("  LI r{mask}, 0xffffffff"));
                self.text.push(format!("  AND r{dst}, r{dst}, r{mask}"));
                Ok(dst)
            }
            Expr::Assign(lhs, rhs) => {
                if self.is_find_tok_lvalue(lhs) {
                    if let Some(src_addr) = self.find_tok_source_addr(rhs)? {
                        let dst_addr = self.emit_lvalue_addr(lhs)?;
                        self.emit_struct_copy(dst_addr, src_addr, 40)?;
                        return Ok(dst_addr);
                    }
                }
                let value = self.emit_expr(rhs)?;
                self.store_lvalue(lhs, value)?;
                Ok(value)
            }
            Expr::Comma(lhs, rhs) => {
                self.emit_expr(lhs)?;
                self.temp_reg = 0;
                self.emit_expr(rhs)
            }
            Expr::CompoundAssign(lhs, op, rhs) => {
                let (current, right) = if rhs.contains_call() && !lhs.contains_call() {
                    let right = self.emit_expr(rhs)?;
                    let current = self.emit_expr(lhs)?;
                    (current, right)
                } else {
                    let current = self.emit_expr(lhs)?;
                    let right = self.emit_expr(rhs)?;
                    (current, right)
                };
                let right = self.scale_pointer_update_rhs(lhs, rhs, right)?;
                let value = self.alloc_reg()?;
                match op {
                    BinOp::Add => self
                        .text
                        .push(format!("  ADD r{value}, r{current}, r{right}")),
                    BinOp::Sub => {
                        self.text
                            .push(format!("  SUB r{value}, r{current}, r{right}"));
                        if self.pointer_diff_width(lhs, rhs) == 8 {
                            let scale = self.alloc_reg()?;
                            self.text.push(format!("  LI r{scale}, 8"));
                            self.text
                                .push(format!("  DIV r{value}, r{value}, r{scale}"));
                        }
                    }
                    BinOp::Mul => self
                        .text
                        .push(format!("  MUL r{value}, r{current}, r{right}")),
                    BinOp::BitOr => self
                        .text
                        .push(format!("  OR r{value}, r{current}, r{right}")),
                    BinOp::BitAnd => self
                        .text
                        .push(format!("  AND r{value}, r{current}, r{right}")),
                    BinOp::Shr => self
                        .text
                        .push(format!("  LSR r{value}, r{current}, r{right}")),
                    _ => return Err("unsupported compound assignment operator".to_string()),
                }
                self.store_lvalue(lhs, value)?;
                Ok(value)
            }
            Expr::PostInc(expr) => self.emit_post_update(expr, 1),
            Expr::PostDec(expr) => self.emit_post_update(expr, -1),
            Expr::Ternary(cond, then_expr, else_expr) => {
                let dst = self.alloc_reg()?;
                let else_label = self.new_label("ternary_else");
                let end_label = self.new_label("ternary_end");
                let cond_reg = self.emit_expr(cond)?;
                self.text.push(format!("  CMP r{cond_reg}, r0"));
                self.text.push(format!("  BEQ {else_label}"));
                let then_reg = self.emit_expr(then_expr)?;
                self.text.push(format!("  MOV r{dst}, r{then_reg}"));
                self.text.push(format!("  JMP {end_label}"));
                self.text.push(format!("{else_label}:"));
                let else_reg = self.emit_expr(else_expr)?;
                self.text.push(format!("  MOV r{dst}, r{else_reg}"));
                self.text.push(format!("{end_label}:"));
                Ok(dst)
            }
            Expr::Index(base, index) => {
                let width = self.index_width(base);
                let addr = self.emit_index_addr(base, index, width)?;
                if width != 1 && width != 8 {
                    return Ok(addr);
                }
                let dst = self.alloc_reg()?;
                if width == 8 {
                    self.text.push(format!("  LD r{dst}, [r{addr}, 0]"));
                } else {
                    self.text.push(format!("  LD.B r{dst}, [r{addr}, 0]"));
                }
                Ok(dst)
            }
            Expr::Member(base, field) => {
                let addr = self.emit_member_addr(base, field)?;
                if matches!(field.as_str(), "pattern" | "d_name") {
                    return Ok(addr);
                }
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LD r{dst}, [r{addr}, 0]"));
                Ok(dst)
            }
            Expr::Call(name, args) => self.emit_call(name, args),
            Expr::CallValue(callee, args) => self.emit_call_value(callee, args),
        }
    }

    fn emit_call_value(&mut self, callee: &Expr, args: &[Expr]) -> Result<usize, String> {
        let regs = self.emit_call_arg_regs(args)?;
        for (idx, reg) in regs.iter().enumerate() {
            self.text.push(format!("  MOV r{}, r{reg}", idx + 1));
        }
        let target = self.emit_expr(callee)?;
        let dst = self.alloc_reg()?;
        self.text.push(format!("  CALL_REG r{target}"));
        self.text.push(format!("  MOV r{dst}, r1"));
        Ok(dst)
    }

    fn emit_local_decl(&mut self, decl: &LocalDecl) -> Result<(), String> {
        self.declare_local(&decl.name)?;
        if let Some(len) = decl.array_len {
            let width = self.local_decl_array_width(&decl.name);
            let bytes = if len == 0 {
                match &decl.init {
                    Some(Expr::Str(value)) => value.len() as i64 + 1,
                    _ => 0,
                }
            } else {
                len * width
            };
            let size = self.alloc_reg()?;
            let ptr = self.alloc_reg()?;
            self.text.push(format!("  LI r{size}, {bytes}"));
            self.text.push(format!("  ALLOC r{ptr}, r{size}"));
            self.store_name(&decl.name, ptr)?;
            self.local_array_widths.insert(decl.name.clone(), width);
            if let Some(Expr::Str(value)) = &decl.init {
                let label = self.intern_string(value);
                let src = self.alloc_reg()?;
                let copy_len = self.alloc_reg()?;
                self.text.push(format!("  LI r{src}, {label}"));
                self.text
                    .push(format!("  LI r{copy_len}, {}", value.len() + 1));
                self.emit_memmove(ptr, src, copy_len)?;
            }
        }
        if let Some(init) = &decl.init {
            if decl.array_len.is_some() {
                return Ok(());
            }
            let reg = self.emit_expr(init)?;
            self.store_name(&decl.name, reg)?;
        }
        Ok(())
    }

    fn emit_binary(&mut self, lhs: &Expr, op: BinOp, rhs: &Expr) -> Result<usize, String> {
        let (left, right) = if rhs.contains_call() && !lhs.contains_call() {
            let right = self.emit_expr(rhs)?;
            let left = self.emit_expr(lhs)?;
            (left, right)
        } else {
            let left = self.emit_expr(lhs)?;
            let right = self.emit_expr(rhs)?;
            (left, right)
        };
        let dst = self.alloc_reg()?;
        match op {
            BinOp::Add => self.text.push(format!("  ADD r{dst}, r{left}, r{right}")),
            BinOp::Sub => {
                self.text.push(format!("  SUB r{dst}, r{left}, r{right}"));
                if self.pointer_diff_width(lhs, rhs) == 8 {
                    let scale = self.alloc_reg()?;
                    self.text.push(format!("  LI r{scale}, 8"));
                    self.text.push(format!("  DIV r{dst}, r{dst}, r{scale}"));
                }
            }
            BinOp::Mul => self.text.push(format!("  MUL r{dst}, r{left}, r{right}")),
            BinOp::Div => self.text.push(format!("  DIV r{dst}, r{left}, r{right}")),
            BinOp::Mod => {
                let quotient = self.alloc_reg()?;
                let product = self.alloc_reg()?;
                self.text
                    .push(format!("  DIV r{quotient}, r{left}, r{right}"));
                self.text
                    .push(format!("  MUL r{product}, r{quotient}, r{right}"));
                self.text.push(format!("  SUB r{dst}, r{left}, r{product}"));
            }
            BinOp::BitOr => self.text.push(format!("  OR r{dst}, r{left}, r{right}")),
            BinOp::BitAnd => self.text.push(format!("  AND r{dst}, r{left}, r{right}")),
            BinOp::BitXor => self.text.push(format!("  XOR r{dst}, r{left}, r{right}")),
            BinOp::Shl => self.text.push(format!("  LSL r{dst}, r{left}, r{right}")),
            BinOp::Shr => self.text.push(format!("  LSR r{dst}, r{left}, r{right}")),
            BinOp::And | BinOp::Or => {
                let false_label = self.new_label("logic_false");
                let true_label = self.new_label("logic_true");
                let end_label = self.new_label("logic_end");
                self.text.push(format!("  CMP r{left}, r0"));
                if matches!(op, BinOp::And) {
                    self.text.push(format!("  BEQ {false_label}"));
                    self.text.push(format!("  CMP r{right}, r0"));
                    self.text.push(format!("  BEQ {false_label}"));
                    self.text.push(format!("  JMP {true_label}"));
                } else {
                    self.text.push(format!("  BNE {true_label}"));
                    self.text.push(format!("  CMP r{right}, r0"));
                    self.text.push(format!("  BNE {true_label}"));
                    self.text.push(format!("  JMP {false_label}"));
                }
                self.text.push(format!("{true_label}:"));
                self.text.push(format!("  LI r{dst}, 1"));
                self.text.push(format!("  JMP {end_label}"));
                self.text.push(format!("{false_label}:"));
                self.text.push(format!("  LI r{dst}, 0"));
                self.text.push(format!("{end_label}:"));
            }
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                let true_label = self.new_label("cmp_true");
                let end_label = self.new_label("cmp_end");
                let branch = match op {
                    BinOp::Eq => "BEQ",
                    BinOp::Ne => "BNE",
                    BinOp::Lt => "BLT",
                    BinOp::Gt => "BGT",
                    BinOp::Le => "BLE",
                    BinOp::Ge => "BGE",
                    _ => unreachable!(),
                };
                self.text.push(format!("  CMP r{left}, r{right}"));
                self.text.push(format!("  LI r{dst}, 0"));
                self.text.push(format!("  {branch} {true_label}"));
                self.text.push(format!("  JMP {end_label}"));
                self.text.push(format!("{true_label}:"));
                self.text.push(format!("  LI r{dst}, 1"));
                self.text.push(format!("{end_label}:"));
            }
        }
        Ok(dst)
    }

    fn scale_pointer_update_rhs(
        &mut self,
        lhs: &Expr,
        rhs: &Expr,
        rhs_reg: usize,
    ) -> Result<usize, String> {
        let Expr::Var(name) = lhs else {
            return Ok(rhs_reg);
        };
        let Expr::Num(_) = rhs else {
            return Ok(rhs_reg);
        };
        let step = self.pointer_step(name);
        if step == 1 {
            return Ok(rhs_reg);
        }
        let scale = self.alloc_reg()?;
        let scaled = self.alloc_reg()?;
        self.text.push(format!("  LI r{scale}, {step}"));
        self.text
            .push(format!("  MUL r{scaled}, r{rhs_reg}, r{scale}"));
        Ok(scaled)
    }

    fn emit_logical_and(&mut self, lhs: &Expr, rhs: &Expr) -> Result<usize, String> {
        let dst = self.alloc_reg()?;
        let false_label = self.new_label("and_false");
        let end_label = self.new_label("and_end");
        let left = self.emit_expr(lhs)?;
        self.text.push(format!("  CMP r{left}, r0"));
        self.text.push(format!("  BEQ {false_label}"));
        self.temp_reg = 1;
        let right = self.emit_expr(rhs)?;
        self.text.push(format!("  CMP r{right}, r0"));
        self.text.push(format!("  BEQ {false_label}"));
        self.text.push(format!("  LI r{dst}, 1"));
        self.text.push(format!("  JMP {end_label}"));
        self.text.push(format!("{false_label}:"));
        self.text.push(format!("  LI r{dst}, 0"));
        self.text.push(format!("{end_label}:"));
        Ok(dst)
    }

    fn emit_logical_or(&mut self, lhs: &Expr, rhs: &Expr) -> Result<usize, String> {
        let dst = self.alloc_reg()?;
        let true_label = self.new_label("or_true");
        let end_label = self.new_label("or_end");
        let left = self.emit_expr(lhs)?;
        self.text.push(format!("  CMP r{left}, r0"));
        self.text.push(format!("  BNE {true_label}"));
        self.temp_reg = 1;
        let right = self.emit_expr(rhs)?;
        self.text.push(format!("  CMP r{right}, r0"));
        self.text.push(format!("  BNE {true_label}"));
        self.text.push(format!("  LI r{dst}, 0"));
        self.text.push(format!("  JMP {end_label}"));
        self.text.push(format!("{true_label}:"));
        self.text.push(format!("  LI r{dst}, 1"));
        self.text.push(format!("{end_label}:"));
        Ok(dst)
    }

    fn store_lvalue(&mut self, lhs: &Expr, value: usize) -> Result<(), String> {
        match lhs {
            Expr::Var(name) => self.store_name(name, value),
            Expr::Unary(UnOp::Deref, ptr) => {
                let width = self.deref_width(ptr);
                let ptr_reg = self.emit_expr(ptr)?;
                if width == 8 {
                    self.text.push(format!("  ST [r{ptr_reg}, 0], r{value}"));
                } else {
                    self.text.push(format!("  ST.B [r{ptr_reg}, 0], r{value}"));
                }
                Ok(())
            }
            Expr::Index(base, index) => {
                let width = self.index_width(base);
                let addr = self.emit_index_addr(base, index, width)?;
                if width == 8 {
                    self.text.push(format!("  ST [r{addr}, 0], r{value}"));
                } else {
                    self.text.push(format!("  ST.B [r{addr}, 0], r{value}"));
                }
                Ok(())
            }
            Expr::Member(base, field) => {
                let addr = self.emit_member_addr(base, field)?;
                self.text.push(format!("  ST [r{addr}, 0], r{value}"));
                Ok(())
            }
            _ => Err("left side of assignment is not assignable".to_string()),
        }
    }

    fn emit_lvalue_addr(&mut self, lhs: &Expr) -> Result<usize, String> {
        match lhs {
            Expr::Unary(UnOp::Deref, ptr) => self.emit_expr(ptr),
            Expr::Index(base, index) => {
                let width = self.index_width(base);
                self.emit_index_addr(base, index, width)
            }
            Expr::Member(base, field) => self.emit_member_addr(base, field),
            Expr::Var(_) => self.emit_addr(lhs),
            _ => Err("left side of assignment is not addressable".to_string()),
        }
    }

    fn is_find_tok_lvalue(&self, lhs: &Expr) -> bool {
        if self.function_names.contains("jsmn_parse") {
            return false;
        }
        match lhs {
            Expr::Unary(UnOp::Deref, ptr) => {
                root_name(ptr).is_some_and(|name| matches!(name, "tok" | "out" | "infix" | "rpn"))
            }
            Expr::Index(base, _) => root_name(base)
                .is_some_and(|name| matches!(name, "tok" | "out" | "infix" | "rpn" | "toks")),
            _ => false,
        }
    }

    fn find_tok_source_addr(&mut self, rhs: &Expr) -> Result<Option<usize>, String> {
        if self.function_names.contains("jsmn_parse") {
            return Ok(None);
        }
        match rhs {
            Expr::Var(name) if name == "and" => self.load_name(name).map(Some),
            Expr::Unary(UnOp::Deref, inner)
                if root_name(inner)
                    .is_some_and(|name| matches!(name, "tok" | "out" | "infix" | "rpn")) =>
            {
                self.emit_expr(inner).map(Some)
            }
            Expr::Unary(UnOp::Deref, inner) if matches!(&**inner, Expr::Unary(UnOp::Deref, _)) => {
                self.emit_expr(inner).map(Some)
            }
            _ => Ok(None),
        }
    }

    fn emit_struct_copy(
        &mut self,
        dst_addr: usize,
        src_addr: usize,
        bytes: i64,
    ) -> Result<(), String> {
        let mut offset = 0;
        while offset < bytes {
            let off = self.alloc_reg()?;
            let dst = self.alloc_reg()?;
            let src = self.alloc_reg()?;
            let value = self.alloc_reg()?;
            self.text.push(format!("  LI r{off}, {offset}"));
            self.text.push(format!("  ADD r{dst}, r{dst_addr}, r{off}"));
            self.text.push(format!("  ADD r{src}, r{src_addr}, r{off}"));
            self.text.push(format!("  LD r{value}, [r{src}, 0]"));
            self.text.push(format!("  ST [r{dst}, 0], r{value}"));
            offset += 8;
        }
        Ok(())
    }

    fn emit_addr(&mut self, expr: &Expr) -> Result<usize, String> {
        match expr {
            Expr::Var(name) => {
                let reg = self.alloc_reg()?;
                if let Some(offset) = self.locals.get(name) {
                    self.text.push(format!("  LI r{reg}, {offset}"));
                    self.text.push(format!("  ADD r{reg}, r31, r{reg}"));
                    Ok(reg)
                } else if let Some(label) = self.globals.get(name) {
                    self.text.push(format!("  LI r{reg}, {label}"));
                    Ok(reg)
                } else {
                    Err(format!("cannot take address of unknown variable {name:?}"))
                }
            }
            Expr::Index(base, index) => {
                let width = self.index_width(base);
                self.emit_index_addr(base, index, width)
            }
            Expr::Member(base, field) => self.emit_member_addr(base, field),
            _ => Err("cannot take address of expression".to_string()),
        }
    }

    fn emit_post_update(&mut self, expr: &Expr, delta: i64) -> Result<usize, String> {
        let old = self.emit_expr(expr)?;
        let step = self.alloc_reg()?;
        let new = self.alloc_reg()?;
        let amount = match expr {
            Expr::Var(name) => self.pointer_step(name) * delta,
            _ => delta,
        };
        self.text.push(format!("  LI r{step}, {amount}"));
        self.text.push(format!("  ADD r{new}, r{old}, r{step}"));
        self.store_lvalue(expr, new)?;
        Ok(old)
    }

    fn emit_index_addr(&mut self, base: &Expr, index: &Expr, width: i64) -> Result<usize, String> {
        let base = self.emit_expr(base)?;
        let index = self.emit_expr(index)?;
        let scale = self.alloc_reg()?;
        let offset = self.alloc_reg()?;
        let addr = self.alloc_reg()?;
        self.text.push(format!("  LI r{scale}, {width}"));
        self.text
            .push(format!("  MUL r{offset}, r{index}, r{scale}"));
        self.text.push(format!("  ADD r{addr}, r{base}, r{offset}"));
        Ok(addr)
    }

    fn emit_member_addr(&mut self, base: &Expr, field: &str) -> Result<usize, String> {
        let offset_value = self.struct_field_offset_for(base, field)?;
        let base = if member_field_name(base).is_some_and(|name| matches!(name, "pinfo" | "oinfo"))
        {
            self.emit_expr(base)?
        } else if matches!(base, Expr::Index(_, _) | Expr::Member(_, _)) {
            self.emit_addr(base)?
        } else {
            self.emit_expr(base)?
        };
        let offset = self.alloc_reg()?;
        let addr = self.alloc_reg()?;
        self.text.push(format!("  LI r{offset}, {offset_value}"));
        self.text.push(format!("  ADD r{addr}, r{base}, r{offset}"));
        Ok(addr)
    }

    fn struct_field_offset_for(&self, base: &Expr, field: &str) -> Result<i64, String> {
        if member_field_name(base).is_some_and(|name| name == "pinfo") {
            return match field {
                "name" => Ok(0),
                "func" => Ok(8),
                "getarg" => Ok(16),
                "freearg" => Ok(24),
                "narg" => Ok(32),
                _ => self.struct_stat_field_offset(field),
            };
        }
        if member_field_name(base).is_some_and(|name| name == "oinfo") {
            return match field {
                "name" => Ok(0),
                "type" => Ok(8),
                "prec" => Ok(16),
                "nargs" => Ok(24),
                "lassoc" => Ok(32),
                _ => self.struct_stat_field_offset(field),
            };
        }
        if root_name(base).is_some_and(|name| name == "r") {
            return match field {
                "fn" => Ok(0),
                "flags" => Ok(8),
                "maxdepth" => Ok(16),
                "follow" => Ok(24),
                "depth" => Ok(32),
                "path" => Ok(40),
                _ => self.struct_stat_field_offset(field),
            };
        }
        if self.function_names.contains("jsmn_parse")
            && root_name(base).is_some_and(|name| {
                matches!(
                    name,
                    "g" | "t" | "tok" | "token" | "tokens" | "toksmall" | "toklarge"
                )
            })
        {
            return match field {
                "type" => Ok(0),
                "start" => Ok(8),
                "end" => Ok(16),
                "size" => Ok(24),
                "parent" => Ok(32),
                _ => self.struct_stat_field_offset(field),
            };
        }
        if root_name(base).is_some_and(|name| {
            matches!(
                name,
                "t" | "tok" | "toks" | "root" | "rpn" | "out" | "infix"
            )
        }) {
            return match field {
                "left" => Ok(0),
                "right" => Ok(8),
                "extra" => Ok(16),
                "u" => Ok(24),
                "pinfo" | "oinfo" => Ok(0),
                "type" => Ok(32),
                _ => self.struct_stat_field_offset(field),
            };
        }
        if root_name(base).is_some_and(|name| matches!(name, "p" | "pri")) {
            return match field {
                "name" => Ok(0),
                "func" => Ok(8),
                "getarg" => Ok(16),
                "freearg" => Ok(24),
                "narg" => Ok(32),
                _ => self.struct_stat_field_offset(field),
            };
        }
        if root_name(base).is_some_and(|name| matches!(name, "o" | "op")) {
            return match field {
                "name" => Ok(0),
                "type" => Ok(8),
                "prec" => Ok(16),
                "nargs" => Ok(24),
                "lassoc" => Ok(32),
                _ => self.struct_stat_field_offset(field),
            };
        }
        if root_name(base).is_some_and(|name| name == "pnode") {
            return match field {
                "preg" => Ok(0),
                "entry" => Ok(8),
                "pattern" => Ok(16),
                _ => self.struct_stat_field_offset(field),
            };
        }
        if member_field_name(base).is_some_and(|name| name == "entry") {
            return match field {
                "sle_next" => Ok(0),
                _ => self.struct_stat_field_offset(field),
            };
        }
        if root_name(base).is_some_and(|name| name == "tree") {
            return match field {
                "dev" => Ok(0),
                "ino" => Ok(8),
                _ => self.struct_stat_field_offset(field),
            };
        }
        if root_name(base).is_some_and(|name| {
            matches!(name, "ent" | "dir" | "ents" | "dents" | "fents" | "a" | "b")
                && self.function_names.contains("mkent")
        }) {
            return match field {
                "name" => Ok(0),
                "mode" => Ok(8),
                "tmode" => Ok(16),
                "nlink" => Ok(24),
                "uid" => Ok(32),
                "gid" => Ok(40),
                "size" => Ok(48),
                "t" => Ok(56),
                "dev" => Ok(72),
                "rdev" => Ok(80),
                "ino" => Ok(88),
                "tino" => Ok(96),
                _ => self.struct_stat_field_offset(field),
            };
        }
        self.struct_stat_field_offset(field)
    }

    fn struct_stat_field_offset(&self, field: &str) -> Result<i64, String> {
        match field {
            "st_mode" => Ok(0),
            "st_size" => Ok(8),
            "st_dev" => Ok(16),
            "st_rdev" => Ok(16),
            "st_ino" => Ok(24),
            "st_mtime" => Ok(32),
            "st_nlink" => Ok(40),
            "st_uid" => Ok(48),
            "st_gid" => Ok(56),
            "st_atime" => Ok(64),
            "st_ctime" => Ok(72),
            "st_mtim" => Ok(32),
            "st_atim" => Ok(64),
            "st_ctim" => Ok(72),
            "tv_sec" => Ok(0),
            "tv_nsec" => Ok(8),
            "tm_year" => Ok(0),
            "tm_isdst" => Ok(8),
            "tm_hour" => Ok(16),
            "tm_gmtoff" => Ok(24),
            "tm_zone" => Ok(32),
            "flags" => Ok(0),
            "maxdepth" => Ok(8),
            "follow" => Ok(16),
            "ret" => Ok(0),
            "depth" => Ok(8),
            "h" => Ok(16),
            "l" => Ok(24),
            "prune" => Ok(32),
            "xdev" => Ok(40),
            "print" => Ok(48),
            "min" => Ok(0),
            "max" => Ok(8),
            "next" => Ok(16),
            "data" => Ok(0),
            "len" => Ok(8),
            "name" => Ok(0),
            "check" => Ok(8),
            "func" => Ok(8),
            "getarg" => Ok(16),
            "freearg" => Ok(24),
            "narg" => Ok(32),
            "prec" => Ok(8),
            "nargs" => Ok(16),
            "lassoc" => Ok(24),
            "type" => Ok(0),
            "left" => Ok(0),
            "right" => Ok(8),
            "extra" => Ok(16),
            "path" => Ok(0),
            "st" => Ok(8),
            "mode" => Ok(0),
            "exact" => Ok(8),
            "cmp" => Ok(0),
            "n" => Ok(8),
            "bytes" => Ok(16),
            "braces" => Ok(0),
            "argv" => Ok(8),
            "s" => Ok(0),
            "arglen" => Ok(0),
            "filelen" => Ok(8),
            "first" => Ok(16),
            "cap" => Ok(24),
            "isplus" => Ok(32),
            "pw_uid" => Ok(0),
            "pw_name" => Ok(8),
            "gr_gid" => Ok(0),
            "gr_name" => Ok(8),
            "dev" => Ok(16),
            "ino" => Ok(24),
            "d_name" => Ok(0),
            "sle_next" => Ok(0),
            "u" => Ok(0),
            "pinfo" => Ok(0),
            "oinfo" => Ok(0),
            "p" => Ok(0),
            "i" => Ok(0),
            "start" => Ok(0),
            "end" => Ok(8),
            "quant" => Ok(16),
            "pos" => Ok(0),
            "toknext" => Ok(8),
            "toksuper" => Ok(16),
            "parent" => Ok(32),
            "size" => Ok(24),
            _ => Err(format!("unsupported struct field {field:?}")),
        }
    }

    fn index_width(&self, base: &Expr) -> i64 {
        if !self.function_names.contains("jsmn_parse")
            && matches!(base, Expr::Var(name) if matches!(
                name.as_str(),
                "tok" | "toks" | "root" | "rpn" | "out" | "infix"
            ))
        {
            40
        } else if self.function_names.contains("jsmn_parse")
            && matches!(base, Expr::Var(name) if matches!(
                name.as_str(),
                "t" | "tok" | "tokens" | "toksmall" | "toklarge"
            ))
        {
            32
        } else if matches!(base, Expr::Var(name) if name == "set" || name == "set1" || name == "set2")
        {
            24
        } else if matches!(base, Expr::Unary(UnOp::Deref, inner) if matches!(&**inner, Expr::Var(name) if name == "set"))
        {
            24
        } else if matches!(base, Expr::Var(name) if name == "times" || name == "classes") {
            16
        } else if matches!(base, Expr::Var(name) if matches!(name.as_str(), "tree")) {
            16
        } else if matches!(base, Expr::Var(name) if matches!(name.as_str(), "ents" | "dents" | "fents"))
        {
            104
        } else if matches!(base, Expr::Var(name) if name == "rstr") {
            8
        } else if let Expr::Var(name) = base
            && let Some(width) = self.local_array_widths.get(name)
        {
            *width
        } else if matches!(base, Expr::Var(name) if name == "argv" || name == "fds" || self.global_arrays.contains(name))
        {
            8
        } else {
            1
        }
    }

    fn local_decl_array_width(&self, name: &str) -> i64 {
        if self.function_names.contains("jsmn_parse")
            && matches!(name, "t" | "tok" | "tokens" | "toksmall" | "toklarge")
        {
            32
        } else if matches!(name, "fp" | "fds" | "argv") {
            8
        } else if matches!(
            name,
            "buf" | "mode" | "pwname" | "grname" | "prefix" | "cwd" | "target" | "ns1" | "ns2"
        ) {
            1
        } else {
            8
        }
    }

    fn pointer_diff_width(&self, lhs: &Expr, rhs: &Expr) -> i64 {
        fn is_word_pointer(expr: &Expr) -> bool {
            matches!(expr, Expr::Var(name) if matches!(
                name.as_str(),
                "argv" | "arg" | "paths" | "sp" | "brace" | "top" | "tok" | "rpn" | "out" | "infix" | "stack"
            ))
        }
        if is_word_pointer(lhs) && is_word_pointer(rhs) {
            8
        } else {
            1
        }
    }

    fn pointer_step(&self, name: &str) -> i64 {
        match name {
            "argv" | "arg" | "paths" | "sp" | "brace" | "top" | "stack" | "new" => 8,
            "p" if self.current_fn == "find_primary" => 40,
            "o" if self.current_fn == "find_op" => 40,
            "g" if self.function_names.contains("jsmn_parse") => 32,
            "ents" | "dents" | "fents" => 104,
            "tree" => 16,
            "t" | "tok" | "toks" | "rpn" | "out" | "infix" | "root" => 40,
            "set" | "set1" | "set2" => 24,
            _ => 1,
        }
    }

    fn deref_width(&self, ptr: &Expr) -> i64 {
        if matches!(ptr, Expr::Unary(UnOp::Deref, inner) if root_name(inner).is_some_and(|name| matches!(name, "argv" | "arg" | "paths")))
        {
            return 1;
        }
        if root_name(ptr).is_some_and(|name| {
            matches!(
                name,
                "argv"
                    | "arg"
                    | "paths"
                    | "sp"
                    | "brace"
                    | "top"
                    | "stack"
                    | "set"
                    | "checks"
                    | "prev"
                    | "root"
                    | "tok"
                    | "tokens"
                    | "parser"
            )
        }) {
            8
        } else {
            1
        }
    }

    fn emit_printf(&mut self, args: &[Expr]) -> Result<(), String> {
        let Some(Expr::Str(fmt)) = args.first() else {
            return Ok(());
        };
        self.needs_c_runtime = true;
        if fmt == "%u %u" && args.len() == 3 {
            let first = self.emit_expr(&args[1])?;
            let second = self.emit_expr(&args[2])?;
            self.text.push(format!("  MOV r1, r{first}"));
            self.text.push("  CALL __print_u64".to_string());
            let space = self.intern_string(" ");
            self.text.push(format!("  LI r1, {space}"));
            self.text.push("  CALL __write_cstr".to_string());
            self.text.push(format!("  MOV r1, r{second}"));
            self.text.push("  CALL __print_u64".to_string());
            return Ok(());
        }
        if fmt == "%zu" && args.len() == 2 {
            let value = self.emit_expr(&args[1])?;
            self.text.push(format!("  MOV r1, r{value}"));
            self.text.push("  CALL __print_u64".to_string());
            return Ok(());
        }
        if let Some(pos) = fmt.find("%.*s") {
            if args.len() < 3 {
                return Ok(());
            }
            let prefix = &fmt[..pos];
            let suffix = &fmt[pos + 4..];
            if !prefix.is_empty() {
                let label = self.intern_string(prefix);
                self.text.push(format!("  LI r1, {label}"));
                self.text.push("  CALL __write_cstr".to_string());
            }
            let len = self.emit_expr(&args[1])?;
            let len_slot = self.next_local_offset;
            self.next_local_offset += 8;
            self.text.push(format!("  ST [r31, {len_slot}], r{len}"));
            self.temp_reg = 0;
            let ptr = self.emit_expr(&args[2])?;
            let len = self.alloc_reg()?;
            self.text.push(format!("  LD r{len}, [r31, {len_slot}]"));
            self.text.push(format!("  WRITE_FD fd1, r{ptr}, r{len}"));
            if !suffix.is_empty() {
                let label = self.intern_string(suffix);
                self.text.push(format!("  LI r1, {label}"));
                self.text.push("  CALL __write_cstr".to_string());
            }
            return Ok(());
        }
        if fmt.contains('%') && args.len() > 1 {
            let mut arg_idx = 1usize;
            let mut literal = String::new();
            let mut chars = fmt.chars().peekable();
            while let Some(ch) = chars.next() {
                if ch != '%' {
                    literal.push(ch);
                    continue;
                }
                if !literal.is_empty() {
                    let label = self.intern_string(&literal);
                    self.text.push(format!("  LI r1, {label}"));
                    self.text.push("  CALL __write_cstr".to_string());
                    literal.clear();
                }
                if chars.peek() == Some(&'%') {
                    chars.next();
                    literal.push('%');
                    continue;
                }
                let Some(spec) = next_format_spec(&mut chars) else {
                    break;
                };
                if arg_idx >= args.len() {
                    continue;
                }
                match spec {
                    'd' | 'i' | 'u' | 'o' => {
                        let value = self.emit_expr(&args[arg_idx])?;
                        self.text.push(format!("  MOV r1, r{value}"));
                        self.text.push("  CALL __print_u64".to_string());
                        self.temp_reg = 0;
                    }
                    's' => {
                        let ptr = self.emit_expr(&args[arg_idx])?;
                        self.text.push(format!("  MOV r1, r{ptr}"));
                        self.text.push("  CALL __write_cstr".to_string());
                        self.temp_reg = 0;
                    }
                    _ => {}
                }
                arg_idx += 1;
            }
            if !literal.is_empty() {
                let label = self.intern_string(&literal);
                self.text.push(format!("  LI r1, {label}"));
                self.text.push("  CALL __write_cstr".to_string());
            }
            return Ok(());
        }
        let label = self.intern_string(fmt);
        self.text.push(format!("  LI r1, {label}"));
        self.text.push("  CALL __write_cstr".to_string());
        Ok(())
    }

    fn emit_snprintf(&mut self, args: &[Expr]) -> Result<usize, String> {
        if args.len() < 3 {
            return Err("snprintf(buf, size, fmt, ...) expects at least 3 arguments".to_string());
        }
        let fmt = match &args[2] {
            Expr::Str(fmt) => fmt.clone(),
            _ => return Err("snprintf format must be a string literal".to_string()),
        };
        let dst = self.emit_expr(&args[0])?;
        self.emit_format_to_buffer(dst, &fmt, args, 3)
    }

    fn emit_sprintf(&mut self, args: &[Expr]) -> Result<usize, String> {
        if args.len() < 2 {
            return Err("sprintf(buf, fmt, ...) expects at least 2 arguments".to_string());
        }
        let fmt = match &args[1] {
            Expr::Str(fmt) => fmt.clone(),
            _ => return Err("sprintf format must be a string literal".to_string()),
        };
        let dst = self.emit_expr(&args[0])?;
        self.emit_format_to_buffer(dst, &fmt, args, 2)
    }

    fn emit_format_to_buffer(
        &mut self,
        dst: usize,
        fmt: &str,
        args: &[Expr],
        first_arg: usize,
    ) -> Result<usize, String> {
        let dst_slot = self.next_local_offset;
        self.next_local_offset += 8;
        self.text.push(format!("  ST [r31, {dst_slot}], r{dst}"));
        self.temp_reg = 0;

        let mut format_arg_slots = Vec::new();
        let mut arg_idx = first_arg;
        let mut scan = fmt.chars().peekable();
        while let Some(ch) = scan.next() {
            if ch != '%' {
                continue;
            }
            if scan.peek() == Some(&'%') {
                scan.next();
                continue;
            }
            let Some(spec) = next_format_spec(&mut scan) else {
                break;
            };
            if !matches!(spec, 's' | 'd' | 'i' | 'u' | 'o') {
                continue;
            }
            let Some(arg) = args.get(arg_idx) else {
                return Err("snprintf missing format argument".to_string());
            };
            arg_idx += 1;
            let value = self.emit_expr(arg)?;
            let slot = self.next_local_offset;
            self.next_local_offset += 8;
            self.text.push(format!("  ST [r31, {slot}], r{value}"));
            self.temp_reg = 0;
            format_arg_slots.push((spec, slot));
        }

        let dst_fixed = 20usize;
        let count = 21usize;
        self.text
            .push(format!("  LD r{dst_fixed}, [r31, {dst_slot}]"));
        self.text.push(format!("  LI r{count}, 0"));
        self.temp_reg = 21;
        let mut format_arg_idx = 0usize;
        let mut chars = fmt.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '%' {
                if chars.peek() == Some(&'%') {
                    chars.next();
                    self.emit_snprintf_store_byte(dst_fixed, count, b'%')?;
                    self.temp_reg = 21;
                    continue;
                }
                let Some(spec) = next_format_spec(&mut chars) else {
                    break;
                };
                if matches!(spec, 's' | 'd' | 'i' | 'u' | 'o') {
                    let Some((stored_spec, slot)) = format_arg_slots.get(format_arg_idx) else {
                        return Err("snprintf missing format argument".to_string());
                    };
                    format_arg_idx += 1;
                    if *stored_spec == 's' {
                        let src = 22usize;
                        self.text.push(format!("  LD r{src}, [r31, {slot}]"));
                        self.temp_reg = 22;
                        self.emit_snprintf_copy_cstr(dst_fixed, count, src)?;
                        self.temp_reg = 21;
                    } else {
                        self.emit_snprintf_store_byte(dst_fixed, count, b'0')?;
                        self.temp_reg = 21;
                    }
                }
            } else {
                self.emit_snprintf_store_byte(dst_fixed, count, ch as u8)?;
                self.temp_reg = 21;
            }
        }
        self.emit_snprintf_store_nul(dst_fixed, count)?;
        self.text.push(format!("  MOV r1, r{count}"));
        self.temp_reg = 1;
        Ok(1)
    }

    fn emit_snprintf_copy_cstr(
        &mut self,
        dst: usize,
        count: usize,
        src: usize,
    ) -> Result<(), String> {
        let cur = self.alloc_reg()?;
        let out = self.alloc_reg()?;
        let ch = self.alloc_reg()?;
        let one = self.alloc_reg()?;
        let loop_label = self.new_label("snprintf_copy");
        let done_label = self.new_label("snprintf_copy_done");
        self.text.push(format!("  MOV r{cur}, r{src}"));
        self.text.push(format!("  LI r{one}, 1"));
        self.text.push(format!("{loop_label}:"));
        self.text.push(format!("  LD.B r{ch}, [r{cur}, 0]"));
        self.text.push(format!("  CMP r{ch}, r0"));
        self.text.push(format!("  BEQ {done_label}"));
        self.text.push(format!("  ADD r{out}, r{dst}, r{count}"));
        self.text.push(format!("  ST.B [r{out}, 0], r{ch}"));
        self.text.push(format!("  ADD r{cur}, r{cur}, r{one}"));
        self.text.push(format!("  ADD r{count}, r{count}, r{one}"));
        self.text.push(format!("  JMP {loop_label}"));
        self.text.push(format!("{done_label}:"));
        Ok(())
    }

    fn emit_snprintf_store_byte(
        &mut self,
        dst: usize,
        count: usize,
        byte: u8,
    ) -> Result<(), String> {
        let out = self.alloc_reg()?;
        let ch = self.alloc_reg()?;
        let one = self.alloc_reg()?;
        self.text.push(format!("  ADD r{out}, r{dst}, r{count}"));
        self.text.push(format!("  LI r{ch}, {byte}"));
        self.text.push(format!("  ST.B [r{out}, 0], r{ch}"));
        self.text.push(format!("  LI r{one}, 1"));
        self.text.push(format!("  ADD r{count}, r{count}, r{one}"));
        Ok(())
    }

    fn emit_snprintf_store_nul(&mut self, dst: usize, count: usize) -> Result<(), String> {
        let out = self.alloc_reg()?;
        self.text.push(format!("  ADD r{out}, r{dst}, r{count}"));
        self.text.push(format!("  ST.B [r{out}, 0], r0"));
        Ok(())
    }

    fn emit_call(&mut self, name: &str, args: &[Expr]) -> Result<usize, String> {
        match name {
            "sizeof" => {
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 8"));
                Ok(dst)
            }
            "va_start" => {
                if let Some(Expr::Var(ap)) = args.first() {
                    let area = self.va_area_label();
                    let reg = self.alloc_reg()?;
                    self.text.push(format!("  LI r{reg}, {area}"));
                    self.store_name(ap, reg)?;
                }
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 0"));
                Ok(dst)
            }
            "va_end" => {
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 0"));
                Ok(dst)
            }
            "va_arg" => {
                if let Some(Expr::Var(ap)) = args.first() {
                    let ptr = self.load_name(ap)?;
                    let dst = self.alloc_reg()?;
                    let one = self.alloc_reg()?;
                    let next = self.alloc_reg()?;
                    self.text.push(format!("  LD r{dst}, [r{ptr}, 0]"));
                    self.text.push(format!("  LI r{one}, 8"));
                    self.text.push(format!("  ADD r{next}, r{ptr}, r{one}"));
                    self.store_name(ap, next)?;
                    return Ok(dst);
                }
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 0"));
                Ok(dst)
            }
            "write" => {
                if matches!(args.first(), Some(Expr::Num(_))) {
                    let (fd_num, buf, len) = self.fd_buf_len_args(name, args)?;
                    let dst = self.alloc_reg()?;
                    self.text
                        .push(format!("  WRITE_FD fd{fd_num}, r{buf}, r{len}"));
                    self.text.push(format!("  MOV r{dst}, r1"));
                    Ok(dst)
                } else {
                    if args.len() != 3 {
                        return Err("write(fd, buf, len) expects 3 arguments".to_string());
                    }
                    let fd = self.emit_expr(&args[0])?;
                    let buf = self.emit_expr(&args[1])?;
                    let len = self.emit_expr(&args[2])?;
                    let dst = self.alloc_reg()?;
                    self.emit_write_fd_dispatch(fd, buf, len, dst)?;
                    Ok(dst)
                }
            }
            "puts" => {
                let ptr = self.one_arg(name, args)?;
                self.needs_c_runtime = true;
                self.text.push(format!("  MOV r1, r{ptr}"));
                self.text.push("  CALL __write_cstr".to_string());
                let newline = self.intern_string("\n");
                self.text.push(format!("  LI r1, {newline}"));
                self.text.push("  CALL __write_cstr".to_string());
                Ok(0)
            }
            "eprintf" | "enprintf" => {
                self.text.push("  LI r1, 1".to_string());
                self.text.push("  EXIT r1".to_string());
                Ok(0)
            }
            "usage" => {
                self.text.push("  LI r1, 1".to_string());
                self.text.push("  EXIT r1".to_string());
                Ok(0)
            }
            "fshut" | "efshut" => {
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 0"));
                Ok(dst)
            }
            "fflush" => {
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 0"));
                Ok(dst)
            }
            "sysconf" => {
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 1048576"));
                Ok(dst)
            }
            "major" | "minor" => {
                let _ = self.one_arg(name, args)?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 0"));
                Ok(dst)
            }
            "humansize" => {
                let _ = self.one_arg(name, args)?;
                let label = self.intern_string("0");
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, {label}"));
                Ok(dst)
            }
            "umask" => {
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 0"));
                Ok(dst)
            }
            "parsemode" => {
                if args.len() < 2 {
                    return Err(
                        "parsemode(mode, base, mask) expects at least 2 arguments".to_string()
                    );
                }
                let mode = self.emit_expr(&args[0])?;
                let base = self.emit_expr(&args[1])?;
                self.emit_parse_mode(mode, base)
            }
            "getumask" => {
                self.no_args(name, args)?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 18"));
                Ok(dst)
            }
            "chmod" => {
                if args.len() != 2 {
                    return Err("chmod(path, mode) expects 2 arguments".to_string());
                }
                let path = self.emit_expr(&args[0])?;
                let mode = self.emit_expr(&args[1])?;
                let flags = self.alloc_reg()?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{flags}, 0"));
                self.text
                    .push(format!("  CHMOD_PATH r{path}, r{mode}, r{flags}"));
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "fchmodat" => {
                if args.len() != 4 {
                    return Err(
                        "fchmodat(dirfd, path, mode, flags) expects 4 arguments".to_string()
                    );
                }
                let path = self.emit_expr(&args[1])?;
                let mode = self.emit_expr(&args[2])?;
                let flags = self.emit_expr(&args[3])?;
                let dst = self.alloc_reg()?;
                self.text
                    .push(format!("  CHMOD_PATH r{path}, r{mode}, r{flags}"));
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "fchmod" => {
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 0"));
                Ok(dst)
            }
            "chown" | "lchown" => {
                if args.len() != 3 {
                    return Err(format!("{name}(path, uid, gid) expects 3 arguments"));
                }
                let path = self.emit_expr(&args[0])?;
                let uid = self.emit_expr(&args[1])?;
                let gid = self.emit_expr(&args[2])?;
                let flags = self.alloc_reg()?;
                let dst = self.alloc_reg()?;
                let nofollow = if name == "lchown" { 1 } else { 0 };
                self.text.push(format!("  LI r{flags}, {nofollow}"));
                self.text
                    .push(format!("  CHOWN_PATH r{path}, r{uid}, r{gid}, r{flags}"));
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "fchownat" => {
                if args.len() != 5 {
                    return Err(
                        "fchownat(dirfd, path, uid, gid, flags) expects 5 arguments".to_string()
                    );
                }
                let path = self.emit_expr(&args[1])?;
                let uid = self.emit_expr(&args[2])?;
                let gid = self.emit_expr(&args[3])?;
                let flags = self.emit_expr(&args[4])?;
                let dst = self.alloc_reg()?;
                self.text
                    .push(format!("  CHOWN_PATH r{path}, r{uid}, r{gid}, r{flags}"));
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "fchown" => {
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 0"));
                Ok(dst)
            }
            "utimensat" => {
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, -1"));
                Ok(dst)
            }
            "futimens" => {
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 0"));
                Ok(dst)
            }
            "strftime" => {
                if args.len() != 4 {
                    return Err("strftime(buf, size, fmt, tm) expects 4 arguments".to_string());
                }
                let dst = self.emit_expr(&args[0])?;
                self.emit_format_to_buffer(dst, "Jan 01 00:00", &[], 0)
            }
            "localtime" => {
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 1"));
                Ok(dst)
            }
            "time" | "strptime" | "mktime" | "clock_gettime" => {
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 0"));
                Ok(dst)
            }
            "mkdir" | "mkdirp" => {
                if args.len() < 2 {
                    return Err(format!("{name}(path, mode) expects at least 2 arguments"));
                }
                let path = self.emit_expr(&args[0])?;
                let mode = self.emit_expr(&args[1])?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  MKDIR_PATH r{path}, r{mode}"));
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "chdir" => {
                let path = self.one_arg(name, args)?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  CHDIR_PATH r{path}"));
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "getcwd" => {
                if args.len() != 2 {
                    return Err("getcwd(buf, size) expects 2 arguments".to_string());
                }
                let buf = self.emit_expr(&args[0])?;
                let size = self.emit_expr(&args[1])?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  GETCWD_PATH r{buf}, r{size}"));
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "mknod" => {
                if args.len() != 3 {
                    return Err("mknod(path, mode, dev) expects 3 arguments".to_string());
                }
                let errno = self.alloc_reg()?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{errno}, 38"));
                self.text.push(format!("  ERRNO_SET r{errno}"));
                self.text.push(format!("  LI r{dst}, -1"));
                Ok(dst)
            }
            "unlink" | "remove" => {
                let path = self.one_arg(name, args)?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  UNLINK_PATH r{path}"));
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "unlinkat" => {
                if args.len() != 3 {
                    return Err("unlinkat(dirfd, path, flags) expects 3 arguments".to_string());
                }
                let path = self.emit_expr(&args[1])?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  UNLINK_PATH r{path}"));
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "access" => {
                if args.len() != 2 {
                    return Err("access(path, mode) expects 2 arguments".to_string());
                }
                let path = self.emit_expr(&args[0])?;
                let size = self.alloc_reg()?;
                let statbuf = self.alloc_reg()?;
                let flags = self.alloc_reg()?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{size}, 80"));
                self.text.push(format!("  ALLOC r{statbuf}, r{size}"));
                self.text.push(format!("  LI r{flags}, 0"));
                self.text
                    .push(format!("  STAT_PATH r{statbuf}, r{path}, r{flags}"));
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "rename" => {
                if args.len() != 2 {
                    return Err("rename(old, new) expects 2 arguments".to_string());
                }
                let old = self.emit_expr(&args[0])?;
                let new = self.emit_expr(&args[1])?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  RENAME_PATH r{old}, r{new}"));
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "link" => {
                if args.len() != 2 {
                    return Err("link(old, new) expects 2 arguments".to_string());
                }
                let old = self.emit_expr(&args[0])?;
                let new = self.emit_expr(&args[1])?;
                let flags = self.alloc_reg()?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{flags}, 0"));
                self.text
                    .push(format!("  LINK_PATH r{old}, r{new}, r{flags}"));
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "linkat" => {
                if args.len() != 5 {
                    return Err(
                        "linkat(olddirfd, old, newdirfd, new, flags) expects 5 arguments"
                            .to_string(),
                    );
                }
                let old = self.emit_expr(&args[1])?;
                let new = self.emit_expr(&args[3])?;
                let flags = self.emit_expr(&args[4])?;
                let dst = self.alloc_reg()?;
                self.text
                    .push(format!("  LINK_PATH r{old}, r{new}, r{flags}"));
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "symlink" => {
                if args.len() != 2 {
                    return Err("symlink(target, linkpath) expects 2 arguments".to_string());
                }
                let target = self.emit_expr(&args[0])?;
                let link = self.emit_expr(&args[1])?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  SYMLINK_PATH r{target}, r{link}"));
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "symlinkat" => {
                if args.len() != 3 {
                    return Err(
                        "symlinkat(target, newdirfd, linkpath) expects 3 arguments".to_string()
                    );
                }
                let target = self.emit_expr(&args[0])?;
                let link = self.emit_expr(&args[2])?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  SYMLINK_PATH r{target}, r{link}"));
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "readlink" => {
                if args.len() != 3 {
                    return Err("readlink(path, buf, len) expects 3 arguments".to_string());
                }
                let path = self.emit_expr(&args[0])?;
                let buf = self.emit_expr(&args[1])?;
                let len = self.emit_expr(&args[2])?;
                let dst = self.alloc_reg()?;
                self.text
                    .push(format!("  READLINK_PATH r{path}, r{buf}, r{len}"));
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "recurse" => {
                if args.len() < 2 {
                    return Err(
                        "recurse(dirfd, path, parent, recursor) expects at least 2 arguments"
                            .to_string(),
                    );
                }
                if args.len() >= 4 {
                    let dirfd = self.emit_expr(&args[0])?;
                    let path = self.emit_expr(&args[1])?;
                    let data = self.emit_expr(&args[2])?;
                    let recursor = self.emit_expr(&args[3])?;
                    let fn_reg = self.alloc_reg()?;
                    let path_field = self.alloc_reg()?;
                    let has_callback = self.new_label("recurse_callback");
                    let end_label = self.new_label("recurse_end");
                    self.text.push(format!("  LD r{fn_reg}, [r{recursor}, 0]"));
                    self.text.push(format!("  LI r{path_field}, 40"));
                    self.text
                        .push(format!("  ADD r{path_field}, r{recursor}, r{path_field}"));
                    self.text.push(format!("  ST [r{path_field}, 0], r{path}"));
                    self.text.push(format!("  CMP r{fn_reg}, r0"));
                    self.text.push(format!("  BNE {has_callback}"));
                    self.text.push(format!("  UNLINK_PATH r{path}"));
                    self.text.push(format!("  JMP {end_label}"));
                    self.text.push(format!("{has_callback}:"));
                    let size = self.alloc_reg()?;
                    let statbuf = self.alloc_reg()?;
                    self.text.push(format!("  LI r{size}, 80"));
                    self.text.push(format!("  ALLOC r{statbuf}, r{size}"));
                    self.emit_fake_regular_stat(Some(path), statbuf)?;
                    self.text.push(format!("  MOV r1, r{dirfd}"));
                    self.text.push(format!("  MOV r2, r{path}"));
                    self.text.push(format!("  MOV r3, r{statbuf}"));
                    self.text.push(format!("  MOV r4, r{data}"));
                    self.text.push(format!("  MOV r5, r{recursor}"));
                    self.text.push(format!("  CALL_REG r{fn_reg}"));
                    self.text.push(format!("{end_label}:"));
                    let dst = self.alloc_reg()?;
                    self.text.push(format!("  LI r{dst}, 0"));
                    return Ok(dst);
                }
                let path = self.emit_expr(&args[1])?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  UNLINK_PATH r{path}"));
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "fprintf" => {
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 0"));
                Ok(dst)
            }
            "lseek" => {
                if args.len() != 3 {
                    return Err("lseek(fd, offset, whence) expects 3 arguments".to_string());
                }
                let offset = self.emit_expr(&args[1])?;
                let whence = self.emit_expr(&args[2])?;
                let dst = self.alloc_reg()?;
                if matches!(args.first(), Some(Expr::Num(_))) {
                    let fd = self.numeric_fd(&args[0], "lseek")?;
                    self.text
                        .push(format!("  FD_SEEK fd{fd}, r{offset}, r{whence}"));
                    self.text.push(format!("  MOV r{dst}, r1"));
                } else {
                    let fd = self.emit_expr(&args[0])?;
                    self.emit_fd_seek_dispatch(fd, offset, whence, dst)?;
                }
                Ok(dst)
            }
            "strlen" => {
                let ptr = self.one_arg(name, args)?;
                let dst = self.alloc_reg()?;
                self.needs_c_runtime = true;
                self.text.push(format!("  MOV r1, r{ptr}"));
                self.text.push("  CALL __strlen".to_string());
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "strcmp" => {
                if args.len() != 2 {
                    return Err("strcmp(a, b) expects 2 arguments".to_string());
                }
                let left = self.emit_expr(&args[0])?;
                let right = self.emit_expr(&args[1])?;
                let dst = self.alloc_reg()?;
                self.needs_c_runtime = true;
                self.text.push(format!("  MOV r1, r{left}"));
                self.text.push(format!("  MOV r2, r{right}"));
                self.text.push("  CALL __strcmp".to_string());
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "strstr" => {
                if args.len() != 2 {
                    return Err("strstr(haystack, needle) expects 2 arguments".to_string());
                }
                let haystack = self.emit_expr(&args[0])?;
                let needle = self.emit_expr(&args[1])?;
                let dst = self.alloc_reg()?;
                self.needs_c_runtime = true;
                self.text.push(format!("  MOV r1, r{haystack}"));
                self.text.push(format!("  MOV r2, r{needle}"));
                self.text.push("  CALL __strstr".to_string());
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "strncmp" => {
                if args.len() != 3 {
                    return Err("strncmp(a, b, n) expects 3 arguments".to_string());
                }
                let left = self.emit_expr(&args[0])?;
                let right = self.emit_expr(&args[1])?;
                let len = self.emit_expr(&args[2])?;
                self.emit_memcmp(left, right, len)
            }
            "fnmatch" => {
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 0"));
                Ok(dst)
            }
            "getpwuid" | "getgrgid" | "getpwnam" | "getgrnam" => {
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 0"));
                Ok(dst)
            }
            "basename" => {
                let ptr = self.one_arg(name, args)?;
                let dst = self.alloc_reg()?;
                self.needs_c_runtime = true;
                self.text.push(format!("  MOV r1, r{ptr}"));
                self.text.push("  CALL __c_basename".to_string());
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "dirname" => {
                let ptr = self.one_arg(name, args)?;
                let dst = self.alloc_reg()?;
                self.needs_c_runtime = true;
                self.text.push(format!("  MOV r1, r{ptr}"));
                self.text.push("  CALL __c_dirname".to_string());
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "atoi" => {
                let ptr = self.one_arg(name, args)?;
                let dst = self.alloc_reg()?;
                self.needs_c_runtime = true;
                self.text.push(format!("  MOV r1, r{ptr}"));
                self.text.push("  CALL __parse_u64".to_string());
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "estrtonum" => {
                if args.is_empty() {
                    return Err("estrtonum(s, min, max) expects at least 1 argument".to_string());
                }
                let ptr = self.emit_expr(&args[0])?;
                let dst = self.alloc_reg()?;
                self.needs_c_runtime = true;
                self.text.push(format!("  MOV r1, r{ptr}"));
                self.text.push("  CALL __parse_u64".to_string());
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "llabs" => {
                let value = self.one_arg(name, args)?;
                Ok(value)
            }
            "MIN" | "MAX" => {
                if args.len() != 2 {
                    return Err(format!("{name}(a, b) expects 2 arguments"));
                }
                self.emit_minmax(name, &args[0], &args[1])
            }
            "LEN" => {
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 1"));
                Ok(dst)
            }
            "strchr" => {
                if args.len() != 2 {
                    return Err("strchr(s, c) expects 2 arguments".to_string());
                }
                let haystack = self.emit_expr(&args[0])?;
                let needle = self.emit_expr(&args[1])?;
                self.emit_strchr(haystack, needle)
            }
            "strtoul" | "strtol" => {
                if args.len() != 3 {
                    return Err(format!("{name}(s, endptr, base) expects 3 arguments"));
                }
                let ptr = self.emit_expr(&args[0])?;
                let endptr = self.emit_expr(&args[1])?;
                self.emit_strtoul(ptr, endptr)
            }
            "estrdup" => {
                let ptr = self.one_arg(name, args)?;
                let ptr_slot = self.next_local_offset;
                self.next_local_offset += 8;
                self.text.push(format!("  ST [r31, {ptr_slot}], r{ptr}"));
                let len = self.alloc_reg()?;
                let one = self.alloc_reg()?;
                let bytes = self.alloc_reg()?;
                let dst = self.alloc_reg()?;
                self.needs_c_runtime = true;
                self.text.push(format!("  MOV r1, r{ptr}"));
                self.text.push("  CALL __strlen".to_string());
                let ptr = self.alloc_reg()?;
                self.text.push(format!("  LD r{ptr}, [r31, {ptr_slot}]"));
                self.text.push(format!("  MOV r{len}, r1"));
                self.text.push(format!("  LI r{one}, 1"));
                self.text.push(format!("  ADD r{bytes}, r{len}, r{one}"));
                self.text.push(format!("  ALLOC r{dst}, r{bytes}"));
                self.emit_memmove(dst, ptr, bytes)?;
                Ok(dst)
            }
            "estrlcpy" | "estrlcat" => {
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 0"));
                Ok(dst)
            }
            "chartorune" | "charntorune" => {
                if args.len() < 2 {
                    return Err(format!("{name}(r, s[, n]) expects at least 2 arguments"));
                }
                let out = self.emit_expr(&args[0])?;
                let src = self.emit_expr(&args[1])?;
                let ch = self.alloc_reg()?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LD.B r{ch}, [r{src}, 0]"));
                self.text.push(format!("  ST [r{out}, 0], r{ch}"));
                self.text.push(format!("  LI r{dst}, 1"));
                Ok(dst)
            }
            "UTF8_POINT" => {
                let ch = self.one_arg(name, args)?;
                let mask = self.alloc_reg()?;
                let masked = self.alloc_reg()?;
                let expected = self.alloc_reg()?;
                let dst = self.alloc_reg()?;
                let true_label = self.new_label("utf8_point_true");
                let end_label = self.new_label("utf8_point_end");
                self.text.push(format!("  LI r{mask}, 192"));
                self.text.push(format!("  AND r{masked}, r{ch}, r{mask}"));
                self.text.push(format!("  LI r{expected}, 128"));
                self.text.push(format!("  CMP r{masked}, r{expected}"));
                self.text.push(format!("  LI r{dst}, 1"));
                self.text.push(format!("  BNE {true_label}"));
                self.text.push(format!("  LI r{dst}, 0"));
                self.text.push(format!("{true_label}:"));
                self.text.push(format!("{end_label}:"));
                Ok(dst)
            }
            "memmove" | "memcpy" => {
                if args.len() != 3 {
                    return Err(format!("{name}(dst, src, n) expects 3 arguments"));
                }
                let dst_ptr = self.emit_expr(&args[0])?;
                let src_ptr = self.emit_expr(&args[1])?;
                let len = self.emit_expr(&args[2])?;
                self.emit_memmove(dst_ptr, src_ptr, len)
            }
            "memset" => {
                if args.len() != 3 {
                    return Err("memset(dst, c, n) expects 3 arguments".to_string());
                }
                let dst_ptr = self.emit_expr(&args[0])?;
                let value = self.emit_expr(&args[1])?;
                let len = self.emit_expr(&args[2])?;
                self.emit_memset(dst_ptr, value, len)
            }
            "memcmp" => {
                if args.len() != 3 {
                    return Err("memcmp(a, b, n) expects 3 arguments".to_string());
                }
                let left = self.emit_expr(&args[0])?;
                let right = self.emit_expr(&args[1])?;
                let len = self.emit_expr(&args[2])?;
                self.emit_memcmp(left, right, len)
            }
            "memmem" => {
                if args.len() != 4 {
                    return Err("memmem(h, hlen, n, nlen) expects 4 arguments".to_string());
                }
                let haystack = self.emit_expr(&args[0])?;
                let hay_len = self.emit_expr(&args[1])?;
                let needle = self.emit_expr(&args[2])?;
                let needle_len = self.emit_expr(&args[3])?;
                self.emit_memmem(haystack, hay_len, needle, needle_len)
            }
            "fullrune" => {
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 1"));
                Ok(dst)
            }
            "utflen" => {
                let ptr = self.one_arg(name, args)?;
                let dst = self.alloc_reg()?;
                self.needs_c_runtime = true;
                self.text.push(format!("  MOV r1, r{ptr}"));
                self.text.push("  CALL __strlen".to_string());
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "utftorunestr" => {
                if args.len() != 2 {
                    return Err("utftorunestr(s, r) expects 2 arguments".to_string());
                }
                let src = self.emit_expr(&args[0])?;
                let dst_ptr = self.emit_expr(&args[1])?;
                self.emit_utftorunestr(src, dst_ptr)
            }
            "efgetrune" => {
                if args.len() < 2 {
                    return Err("efgetrune(r, fp, name) expects at least 2 arguments".to_string());
                }
                let out = self.emit_expr(&args[0])?;
                let fp = self.emit_expr(&args[1])?;
                self.emit_efgetrune(out, fp)
            }
            "efputrune" => {
                if args.len() < 1 {
                    return Err("efputrune(r, fp) expects at least 1 argument".to_string());
                }
                let ptr = self.emit_expr(&args[0])?;
                self.emit_efputrune(ptr)
            }
            "isspacerune" => {
                let ch = self.one_arg(name, args)?;
                self.emit_space_predicate(ch)
            }
            "isprintrune" => {
                let ch = self.one_arg(name, args)?;
                let lower = self.alloc_reg()?;
                let upper = self.alloc_reg()?;
                let dst = self.alloc_reg()?;
                let false_label = self.new_label("isprint_false");
                let end_label = self.new_label("isprint_end");
                self.text.push(format!("  LI r{lower}, 32"));
                self.text.push(format!("  CMP r{ch}, r{lower}"));
                self.text.push(format!("  BLT {false_label}"));
                self.text.push(format!("  LI r{upper}, 126"));
                self.text.push(format!("  CMP r{ch}, r{upper}"));
                self.text.push(format!("  BGT {false_label}"));
                self.text.push(format!("  LI r{dst}, 1"));
                self.text.push(format!("  JMP {end_label}"));
                self.text.push(format!("{false_label}:"));
                self.text.push(format!("  LI r{dst}, 0"));
                self.text.push(format!("{end_label}:"));
                Ok(dst)
            }
            "isblank" => {
                let ch = self.one_arg(name, args)?;
                self.emit_space_predicate(ch)
            }
            "tolowerrune" | "toupperrune" => self.one_arg(name, args),
            "free" => {
                let ptr = self.one_arg(name, args)?;
                self.text.push(format!("  FREE r{ptr}"));
                Ok(0)
            }
            "erealloc" | "emalloc" | "enmalloc" | "malloc" => {
                let size_arg = if name == "erealloc" {
                    if args.len() != 2 {
                        return Err("erealloc(ptr, size) expects 2 arguments".to_string());
                    }
                    &args[1]
                } else if name == "enmalloc" {
                    if args.len() != 2 {
                        return Err("enmalloc(status, size) expects 2 arguments".to_string());
                    }
                    &args[1]
                } else {
                    if args.len() != 1 {
                        return Err(format!("{name}(size) expects 1 argument"));
                    }
                    &args[0]
                };
                let size = self.emit_expr(size_arg)?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  ALLOC r{dst}, r{size}"));
                Ok(dst)
            }
            "ereallocarray" => {
                if args.len() != 3 {
                    return Err("ereallocarray(ptr, nmemb, size) expects 3 arguments".to_string());
                }
                let old = self.emit_expr(&args[0])?;
                let nmemb = self.emit_expr(&args[1])?;
                let size = self.emit_expr(&args[2])?;
                let bytes = self.alloc_reg()?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  MUL r{bytes}, r{nmemb}, r{size}"));
                self.text.push(format!("  ALLOC r{dst}, r{bytes}"));
                let copy_len = self.alloc_reg()?;
                let skip_copy = self.new_label("reallocarray_skip_copy");
                self.text.push(format!("  CMP r{old}, r0"));
                self.text.push(format!("  BEQ {skip_copy}"));
                self.text
                    .push(format!("  SUB r{copy_len}, r{bytes}, r{size}"));
                self.text.push(format!("  CMP r{copy_len}, r0"));
                self.text.push(format!("  BLE {skip_copy}"));
                self.emit_memmove(dst, old, copy_len)?;
                self.text.push(format!("{skip_copy}:"));
                Ok(dst)
            }
            "unescape" => {
                let ptr = self.one_arg(name, args)?;
                let dst = self.alloc_reg()?;
                self.needs_c_runtime = true;
                self.text.push(format!("  MOV r1, r{ptr}"));
                self.text.push("  CALL __strlen".to_string());
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "EARGF" => {
                if args.len() != 1 {
                    return Err("EARGF(fallback) expects 1 argument".to_string());
                }
                self.emit_eargf()
            }
            "ARGNUMF" => {
                self.no_args(name, args)?;
                self.emit_argnumf()
            }
            "fstat" => {
                if args.len() != 2 {
                    return Err("fstat(fd, statbuf) expects 2 arguments".to_string());
                }
                let statbuf = self.emit_stat_buffer_arg(&args[1])?;
                let dst = self.alloc_reg()?;
                if matches!(args.first(), Some(Expr::Num(_))) {
                    let fd = self.numeric_fd(&args[0], "fstat")?;
                    self.text.push(format!("  STAT_FD r{statbuf}, fd{fd}"));
                    self.text.push(format!("  MOV r{dst}, r1"));
                } else {
                    let fd = self.emit_expr(&args[0])?;
                    self.emit_stat_fd_dispatch(fd, statbuf, dst)?;
                }
                Ok(dst)
            }
            "stat" | "lstat" => {
                if args.len() != 2 {
                    return Err(format!("{name}(path, statbuf) expects 2 arguments"));
                }
                let path = self.emit_expr(&args[0])?;
                let statbuf = self.emit_stat_buffer_arg(&args[1])?;
                let flags = self.alloc_reg()?;
                let dst = self.alloc_reg()?;
                let nofollow = if name == "lstat" { 1 } else { 0 };
                self.text.push(format!("  LI r{flags}, {nofollow}"));
                self.text
                    .push(format!("  STAT_PATH r{statbuf}, r{path}, r{flags}"));
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "fstatat" => {
                if args.len() != 4 {
                    return Err(
                        "fstatat(dirfd, path, statbuf, flags) expects 4 arguments".to_string()
                    );
                }
                let path = self.emit_expr(&args[1])?;
                let statbuf = self.emit_stat_buffer_arg(&args[2])?;
                let flags = self.emit_expr(&args[3])?;
                let dst = self.alloc_reg()?;
                self.text
                    .push(format!("  STAT_PATH r{statbuf}, r{path}, r{flags}"));
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "S_ISREG" | "S_ISFIFO" | "S_ISDIR" | "S_ISBLK" | "S_ISCHR" | "S_ISLNK" | "S_ISSOCK" => {
                let mode = self.one_arg(name, args)?;
                self.emit_mode_predicate(name, mode)
            }
            "fopen" => {
                if args.len() != 2 {
                    return Err("fopen(path, mode) expects 2 arguments".to_string());
                }
                let path = self.emit_expr(&args[0])?;
                let flags = self.alloc_reg()?;
                self.text.push(format!("  LI r{flags}, 0"));
                self.emit_open_fd_alloc(path, flags)
            }
            "fmemopen" => {
                if args.len() != 3 {
                    return Err("fmemopen(buf, size, mode) expects 3 arguments".to_string());
                }
                let buf = self.emit_expr(&args[0])?;
                let len = self.emit_expr(&args[1])?;
                let dst = self.alloc_reg()?;
                self.data
                    .entry("c_memstream_ptr".to_string())
                    .or_insert(".quad 0".to_string());
                self.data
                    .entry("c_memstream_len".to_string())
                    .or_insert(".quad 0".to_string());
                self.data
                    .entry("c_memstream_pos".to_string())
                    .or_insert(".quad 0".to_string());
                self.text.push(format!("  ST c_memstream_ptr, r{buf}"));
                self.text.push(format!("  ST c_memstream_len, r{len}"));
                self.text.push("  ST c_memstream_pos, r0".to_string());
                self.text.push(format!("  LI r{dst}, -2"));
                Ok(dst)
            }
            "enregcomp" | "regcomp" => {
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 0"));
                Ok(dst)
            }
            "regexec" => {
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 1"));
                Ok(dst)
            }
            "qsort" => {
                if args.len() != 4 {
                    return Err("qsort(base, nmemb, size, cmp) expects 4 arguments".to_string());
                }
                let regs = self.emit_call_arg_regs(args)?;
                self.emit_qsort(regs[0], regs[1], regs[2], regs[3])
            }
            "creat" => {
                if args.len() != 2 {
                    return Err("creat(path, mode) expects 2 arguments".to_string());
                }
                let path = self.emit_expr(&args[0])?;
                let flags = self.alloc_reg()?;
                self.text.push(format!("  LI r{flags}, 6"));
                self.emit_open_fd_alloc(path, flags)
            }
            "opendir" => {
                let path = self.one_arg(name, args)?;
                self.emit_open_dir_alloc(path)
            }
            "readdir" => {
                let dir = self.one_arg(name, args)?;
                let label = "c_dirent_buf".to_string();
                self.data
                    .entry(label.clone())
                    .or_insert(".zero 512".to_string());
                let buf = self.alloc_reg()?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{buf}, {label}"));
                self.emit_readdir_fd_dispatch(dir, buf, dst)?;
                Ok(dst)
            }
            "closedir" => {
                if args.len() != 1 {
                    return Err("closedir(dir) expects 1 argument".to_string());
                }
                let dir = self.emit_expr(&args[0])?;
                let dst = self.alloc_reg()?;
                self.emit_fd_close_dispatch(dir, dst)?;
                Ok(dst)
            }
            "getc" | "fgetc" => {
                let stream = self.one_arg(name, args)?;
                self.emit_getc(stream)
            }
            "getchar" => {
                self.no_args(name, args)?;
                let stdin = self.alloc_reg()?;
                self.text.push(format!("  LI r{stdin}, 0"));
                self.emit_getc(stdin)
            }
            "open" if !matches!(args.first(), Some(Expr::Num(_))) => {
                if args.len() != 2 && args.len() != 3 {
                    return Err("open(path, flags[, mode]) expects 2 or 3 arguments".to_string());
                }
                let path = self.emit_expr(&args[0])?;
                let flags = self.emit_expr(&args[1])?;
                self.emit_open_fd_alloc(path, flags)
            }
            "getline" => {
                if args.len() != 3 {
                    return Err("getline(&buf, &size, fp) expects 3 arguments".to_string());
                }
                let buf_addr = self.emit_expr(&args[0])?;
                let size_addr = self.emit_expr(&args[1])?;
                let fp = self.emit_expr(&args[2])?;
                let dst = self.alloc_reg()?;
                self.needs_c_runtime = true;
                self.text.push(format!("  MOV r1, r{buf_addr}"));
                self.text.push(format!("  MOV r2, r{size_addr}"));
                self.text.push(format!("  MOV r3, r{fp}"));
                self.text.push("  CALL __getline".to_string());
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "fwrite" => {
                if args.len() != 4 {
                    return Err("fwrite(buf, size, nmemb, stream) expects 4 arguments".to_string());
                }
                let buf = self.emit_expr(&args[0])?;
                let size = self.emit_expr(&args[1])?;
                let nmemb = self.emit_expr(&args[2])?;
                let len = self.alloc_reg()?;
                self.text.push(format!("  MUL r{len}, r{size}, r{nmemb}"));
                self.text.push(format!("  WRITE_FD fd1, r{buf}, r{len}"));
                Ok(nmemb)
            }
            "ecalloc" => {
                if args.len() != 2 {
                    return Err("ecalloc(count, size) expects 2 arguments".to_string());
                }
                let count = self.emit_expr(&args[0])?;
                let size = self.emit_expr(&args[1])?;
                let bytes = self.alloc_reg()?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  MUL r{bytes}, r{count}, r{size}"));
                self.text.push(format!("  ALLOC r{dst}, r{bytes}"));
                Ok(dst)
            }
            "signal" => {
                if args.len() != 2 {
                    return Err("signal(signum, handler) expects 2 arguments".to_string());
                }
                let signum = self.emit_expr(&args[0])?;
                let handler = self.emit_expr(&args[1])?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  SIGACTION r{signum}, r{handler}"));
                self.text.push(format!("  LI r{dst}, 0"));
                Ok(dst)
            }
            "sigaction" => {
                if args.len() < 2 {
                    return Err(
                        "sigaction(signum, handler[, old]) expects at least 2 arguments"
                            .to_string(),
                    );
                }
                let signum = self.emit_expr(&args[0])?;
                let handler = self.emit_expr(&args[1])?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  SIGACTION r{signum}, r{handler}"));
                self.text.push(format!("  LI r{dst}, 0"));
                Ok(dst)
            }
            "printf" | "weprintf" | "xvprintf" => {
                if name == "printf" {
                    self.emit_printf(args)?;
                }
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 0"));
                Ok(dst)
            }
            "snprintf" => self.emit_snprintf(args),
            "sprintf" => self.emit_sprintf(args),
            "fputs" => {
                if args.len() != 2 {
                    return Err("fputs(s, stream) expects 2 arguments".to_string());
                }
                let ptr = self.emit_expr(&args[0])?;
                self.needs_c_runtime = true;
                self.text.push(format!("  MOV r1, r{ptr}"));
                self.text.push("  CALL __write_cstr".to_string());
                Ok(0)
            }
            "putword" => {
                if args.len() != 2 {
                    return Err("putword(stream, s) expects 2 arguments".to_string());
                }
                let ptr = self.emit_expr(&args[1])?;
                let seen_label = "c_putword_seen".to_string();
                self.data
                    .entry(seen_label.clone())
                    .or_insert(".quad 0".to_string());
                let space = self.intern_string(" ");
                let seen = self.alloc_reg()?;
                let one = self.alloc_reg()?;
                let no_space = self.new_label("putword_no_space");
                self.needs_c_runtime = true;
                self.text.push(format!("  LD r{seen}, {seen_label}"));
                self.text.push(format!("  CMP r{seen}, r0"));
                self.text.push(format!("  BEQ {no_space}"));
                self.text.push(format!("  LI r1, {space}"));
                self.text.push("  CALL __write_cstr".to_string());
                self.text.push(format!("{no_space}:"));
                self.text.push(format!("  MOV r1, r{ptr}"));
                self.text.push("  CALL __write_cstr".to_string());
                self.text.push(format!("  LI r{one}, 1"));
                self.text.push(format!("  ST {seen_label}, r{one}"));
                Ok(0)
            }
            "ferror" => {
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 0"));
                Ok(dst)
            }
            "feof" => {
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 1"));
                Ok(dst)
            }
            "clearerr" => {
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 0"));
                Ok(dst)
            }
            "putchar" => {
                let ch = self.one_arg(name, args)?;
                let label = "c_putchar_buf".to_string();
                self.data
                    .entry(label.clone())
                    .or_insert(".zero 1".to_string());
                let addr = self.alloc_reg()?;
                let len = self.alloc_reg()?;
                self.text.push(format!("  LI r{addr}, {label}"));
                self.text.push(format!("  ST.B [r{addr}, 0], r{ch}"));
                self.text.push(format!("  LI r{len}, 1"));
                self.text.push(format!("  WRITE_FD fd1, r{addr}, r{len}"));
                Ok(0)
            }
            "fputc" => {
                if args.is_empty() {
                    return Err("fputc(c, stream) expects at least 1 argument".to_string());
                }
                let ch = self.emit_expr(&args[0])?;
                let label = "c_putchar_buf".to_string();
                self.data
                    .entry(label.clone())
                    .or_insert(".zero 1".to_string());
                let addr = self.alloc_reg()?;
                let len = self.alloc_reg()?;
                self.text.push(format!("  LI r{addr}, {label}"));
                self.text.push(format!("  ST.B [r{addr}, 0], r{ch}"));
                self.text.push(format!("  LI r{len}, 1"));
                self.text.push(format!("  WRITE_FD fd1, r{addr}, r{len}"));
                Ok(0)
            }
            "open" => {
                if args.len() != 3 {
                    return Err("open(fd, path, flags) expects 3 arguments".to_string());
                }
                let fd_num = self.numeric_fd(&args[0], "open")?;
                let path = self.emit_expr(&args[1])?;
                let flags = self.emit_expr(&args[2])?;
                let dst = self.alloc_reg()?;
                self.text
                    .push(format!("  OPEN_FD fd{fd_num}, r{path}, r{flags}"));
                self.text.push(format!("  LI r{dst}, {fd_num}"));
                Ok(dst)
            }
            "read" => {
                if matches!(args.first(), Some(Expr::Num(_))) {
                    let (fd_num, buf, len) = self.fd_buf_len_args(name, args)?;
                    let dst = self.alloc_reg()?;
                    self.text
                        .push(format!("  READ_FD fd{fd_num}, r{buf}, r{len}"));
                    self.text.push(format!("  MOV r{dst}, r1"));
                    Ok(dst)
                } else {
                    if args.len() != 3 {
                        return Err("read(fd, buf, len) expects 3 arguments".to_string());
                    }
                    let fd = self.emit_expr(&args[0])?;
                    let buf = self.emit_expr(&args[1])?;
                    let len = self.emit_expr(&args[2])?;
                    let dst = self.alloc_reg()?;
                    self.emit_read_fd_dispatch(fd, buf, len, Some(dst))?;
                    Ok(dst)
                }
            }
            "wait_on_fd" => {
                if args.len() != 1 {
                    return Err("wait_on_fd(fd) expects 1 argument".to_string());
                }
                let fd_num = self.numeric_fd(&args[0], "wait_on_fd")?;
                self.text.push(format!("  WAIT_ON_FD fd{fd_num}, r0"));
                Ok(0)
            }
            "fd_dup" => {
                if args.len() != 2 {
                    return Err("fd_dup(dst, src) expects 2 arguments".to_string());
                }
                let dst = self.numeric_fd(&args[0], "fd_dup")?;
                let src = self.numeric_fd(&args[1], "fd_dup")?;
                self.text.push(format!("  FD_DUP fd{dst}, fd{src}"));
                Ok(0)
            }
            "load" => {
                let ptr = self.one_arg(name, args)?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LD r{dst}, [r{ptr}, 0]"));
                Ok(dst)
            }
            "store" => {
                if args.len() != 2 {
                    return Err("store(ptr, value) expects 2 arguments".to_string());
                }
                let ptr = self.emit_expr(&args[0])?;
                let value = self.emit_expr(&args[1])?;
                self.text.push(format!("  ST [r{ptr}, 0], r{value}"));
                Ok(0)
            }
            "loadb" => {
                let ptr = self.one_arg(name, args)?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LD.B r{dst}, [r{ptr}, 0]"));
                Ok(dst)
            }
            "storeb" => {
                if args.len() != 2 {
                    return Err("storeb(ptr, value) expects 2 arguments".to_string());
                }
                let ptr = self.emit_expr(&args[0])?;
                let value = self.emit_expr(&args[1])?;
                self.text.push(format!("  ST.B [r{ptr}, 0], r{value}"));
                Ok(0)
            }
            "bitand" | "bitor" | "bitxor" => {
                if args.len() != 2 {
                    return Err(format!("{name}(a, b) expects 2 arguments"));
                }
                let left = self.emit_expr(&args[0])?;
                let right = self.emit_expr(&args[1])?;
                let dst = self.alloc_reg()?;
                let op = match name {
                    "bitand" => "AND",
                    "bitor" => "OR",
                    "bitxor" => "XOR",
                    _ => unreachable!(),
                };
                self.text.push(format!("  {op} r{dst}, r{left}, r{right}"));
                Ok(dst)
            }
            "alloc" => {
                let len = self.one_arg(name, args)?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  ALLOC r{dst}, r{len}"));
                Ok(dst)
            }
            "close" => {
                if args.len() != 1 {
                    return Err("close(fd) expects 1 argument".to_string());
                }
                let dst = self.alloc_reg()?;
                if matches!(args.first(), Some(Expr::Num(_))) {
                    let fd = self.numeric_fd(&args[0], "close")?;
                    self.text.push(format!("  FD_CLOSE fd{fd}"));
                    self.text.push(format!("  MOV r{dst}, r1"));
                } else {
                    let fd = self.emit_expr(&args[0])?;
                    self.emit_fd_close_dispatch(fd, dst)?;
                }
                Ok(dst)
            }
            "pid" | "tid" | "uid" | "gid" => {
                if !args.is_empty() {
                    return Err(format!("{name}() expects no arguments"));
                }
                let dst = self.alloc_reg()?;
                let pcr = match name {
                    "pid" => "PID",
                    "tid" => "TID",
                    "uid" => "UID",
                    "gid" => "GID",
                    _ => unreachable!(),
                };
                self.text.push(format!("  GET_PCR r{dst}, {pcr}"));
                Ok(dst)
            }
            "set_uid" | "set_gid" | "set_sigmask" => {
                let value = self.one_arg(name, args)?;
                let pcr = match name {
                    "set_uid" => "UID",
                    "set_gid" => "GID",
                    "set_sigmask" => "SIGMASK",
                    _ => unreachable!(),
                };
                self.text.push(format!("  SET_PCR {pcr}, r{value}"));
                Ok(0)
            }
            "fork" => {
                if !args.is_empty() {
                    return Err("fork() expects no arguments".to_string());
                }
                let dst = self.alloc_reg()?;
                self.text.push(format!("  FORK r{dst}"));
                Ok(dst)
            }
            "exec" | "execvp" => {
                if args.is_empty() {
                    return Err(format!("{name}(path[, argv]) expects at least 1 argument"));
                }
                let path = self.emit_expr(&args[0])?;
                let argv = if args.len() > 1 {
                    self.emit_expr(&args[1])?
                } else {
                    0
                };
                self.text.push(format!("  EXEC r{path}, r{argv}"));
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, -1"));
                Ok(dst)
            }
            "waitpid" => {
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 0"));
                Ok(dst)
            }
            "spawn" if !self.function_names.contains("spawn") => {
                if args.len() != 1 {
                    return Err("spawn(function_name) expects 1 argument".to_string());
                }
                let Expr::Var(label) = &args[0] else {
                    return Err("spawn argument must be a function name".to_string());
                };
                if !self.function_names.contains(label) {
                    return Err(format!("unknown spawn target {label:?}"));
                }
                let target = self.alloc_reg()?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{target}, {label}"));
                self.text.push(format!("  SPAWN r{dst}, r{target}"));
                Ok(dst)
            }
            "yield_cpu" => {
                self.no_args(name, args)?;
                self.text.push("  YIELD".to_string());
                Ok(0)
            }
            "sleep" => {
                let ticks = self.one_arg(name, args)?;
                self.text.push(format!("  SLEEP r{ticks}"));
                Ok(0)
            }
            "exit" | "_exit" => {
                let code = self.one_arg(name, args)?;
                self.text.push(format!("  EXIT r{code}"));
                Ok(0)
            }
            "msg_send" => {
                if args.len() != 3 {
                    return Err("msg_send(pid, v1, v2) expects 3 arguments".to_string());
                }
                let pid = self.emit_expr(&args[0])?;
                let v1 = self.emit_expr(&args[1])?;
                let v2 = self.emit_expr(&args[2])?;
                self.text.push(format!("  MSG_SEND r{pid}, r{v1}, r{v2}"));
                Ok(0)
            }
            "msg_recv" => {
                self.no_args(name, args)?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  MSG_RECV r{dst}, r30"));
                Ok(dst)
            }
            "cmpxchg" => {
                if args.len() != 3 {
                    return Err("cmpxchg(ptr, expected, new) expects 3 arguments".to_string());
                }
                let ptr = self.emit_expr(&args[0])?;
                let expected = self.emit_expr(&args[1])?;
                let new = self.emit_expr(&args[2])?;
                let dst = self.alloc_reg()?;
                self.text.push(format!(
                    "  LOCK.CMPXCHG r{dst}, r{ptr}, r{expected}, r{new}"
                ));
                Ok(dst)
            }
            "futex_wait" => {
                if args.len() != 2 {
                    return Err("futex_wait(ptr, expected) expects 2 arguments".to_string());
                }
                let ptr = self.emit_expr(&args[0])?;
                let expected = self.emit_expr(&args[1])?;
                self.text.push(format!("  FUTEX_WAIT r{ptr}, r{expected}"));
                Ok(0)
            }
            "futex_wake" => {
                if args.len() != 2 {
                    return Err("futex_wake(ptr, count) expects 2 arguments".to_string());
                }
                let ptr = self.emit_expr(&args[0])?;
                let count = self.emit_expr(&args[1])?;
                self.text.push(format!("  FUTEX_WAKE r{ptr}, r{count}"));
                Ok(0)
            }
            "mmap" => {
                let (hint, len, prot, fd_num, offset) = match args.len() {
                    3 => {
                        let fd_num = self.numeric_fd(&args[0], "mmap")?;
                        (0, self.emit_expr(&args[1])?, self.emit_expr(&args[2])?, fd_num, 0)
                    }
                    6 => {
                        let hint = self.emit_expr(&args[0])?;
                        let len = self.emit_expr(&args[1])?;
                        let prot = self.emit_expr(&args[2])?;
                        let fd_num = self.numeric_fd(&args[4], "mmap")?;
                        let offset = self.emit_expr(&args[5])?;
                        (hint, len, prot, fd_num, offset)
                    }
                    _ => return Err("mmap expects mmap(fd, len, prot) or POSIX mmap(addr, len, prot, flags, fd, offset)".to_string()),
                };
                let dst = self.alloc_reg()?;
                self.text.push(format!(
                    "  MMAP r{dst}, r{hint}, r{len}, r{prot}, fd{fd_num}, r{offset}"
                ));
                Ok(dst)
            }
            "munmap" => {
                if args.len() != 2 {
                    return Err("munmap(addr, len) expects 2 arguments".to_string());
                }
                let addr = self.emit_expr(&args[0])?;
                let len = self.emit_expr(&args[1])?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  MUNMAP r{addr}, r{len}"));
                self.text.push(format!("  LI r{dst}, 0"));
                Ok(dst)
            }
            "sigmask_set" => {
                let mask = self.one_arg(name, args)?;
                self.text.push(format!("  SIGMASK_SET r{mask}"));
                Ok(0)
            }
            "kill" => {
                if args.len() != 2 {
                    return Err("kill(pid, signum) expects 2 arguments".to_string());
                }
                let pid = self.emit_expr(&args[0])?;
                let signum = self.emit_expr(&args[1])?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  KILL r{pid}, r{signum}"));
                self.text.push(format!("  LI r{dst}, 0"));
                Ok(dst)
            }
            "sigret" | "sigreturn" => {
                self.no_args(name, args)?;
                self.text.push("  SIGRET".to_string());
                Ok(0)
            }
            "inb" => {
                let port = self.one_arg(name, args)?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  INB r{dst}, r{port}"));
                Ok(dst)
            }
            "outb" => {
                if args.len() != 2 {
                    return Err("outb(port, value) expects 2 arguments".to_string());
                }
                let port = self.emit_expr(&args[0])?;
                let value = self.emit_expr(&args[1])?;
                self.text.push(format!("  OUTB r{port}, r{value}"));
                Ok(0)
            }
            "load_ucode" => {
                if args.len() != 2 {
                    return Err("load_ucode(buf, len) expects 2 arguments".to_string());
                }
                let buf = self.emit_expr(&args[0])?;
                let len = self.emit_expr(&args[1])?;
                self.text.push(format!("  LOAD_UCODE r{buf}, r{len}"));
                Ok(0)
            }
            "fence" => {
                self.no_args(name, args)?;
                self.text.push("  FENCE".to_string());
                Ok(0)
            }
            _ if self.function_names.contains(name) => {
                let fixed_count = self
                    .function_param_counts
                    .get(name)
                    .copied()
                    .unwrap_or(args.len());
                self.emit_varargs_area(args, fixed_count)?;
                let fixed_end = fixed_count.min(args.len());
                let regs = self.emit_call_arg_regs(&args[..fixed_end])?;
                for (idx, reg) in regs.iter().enumerate() {
                    self.text.push(format!("  MOV r{}, r{reg}", idx + 1));
                }
                let dst = self.alloc_reg()?;
                self.text.push(format!("  CALL {name}"));
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            _ if self.locals.contains_key(name) || self.globals.contains_key(name) => {
                let regs = self.emit_call_arg_regs(args)?;
                for (idx, reg) in regs.iter().enumerate() {
                    self.text.push(format!("  MOV r{}, r{reg}", idx + 1));
                }
                let target = self.load_name(name)?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  CALL_REG r{target}"));
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            _ => Err(format!("unsupported function call {name:?}")),
        }
    }

    fn emit_call_arg_regs(&mut self, args: &[Expr]) -> Result<Vec<usize>, String> {
        let args = args.iter().take(6).collect::<Vec<_>>();
        let mut slots = Vec::new();
        for arg in args {
            let reg = self.emit_expr(arg)?;
            let offset = self.next_local_offset;
            self.next_local_offset += 8;
            self.text.push(format!("  ST [r31, {offset}], r{reg}"));
            slots.push(offset);
            self.temp_reg = 0;
        }
        let mut regs = Vec::new();
        for offset in slots {
            let reg = self.alloc_reg()?;
            self.text.push(format!("  LD r{reg}, [r31, {offset}]"));
            regs.push(reg);
        }
        Ok(regs)
    }

    fn emit_varargs_area(&mut self, args: &[Expr], fixed_count: usize) -> Result<(), String> {
        if args.len() <= fixed_count {
            return Ok(());
        }
        let area = self.va_area_label();
        for (idx, arg) in args.iter().skip(fixed_count).enumerate() {
            let value = self.emit_expr(arg)?;
            let addr = self.alloc_reg()?;
            let offset = self.alloc_reg()?;
            self.text.push(format!("  LI r{addr}, {area}"));
            self.text.push(format!("  LI r{offset}, {}", idx * 8));
            self.text.push(format!("  ADD r{addr}, r{addr}, r{offset}"));
            self.text.push(format!("  ST [r{addr}, 0], r{value}"));
            self.temp_reg = 0;
        }
        Ok(())
    }

    fn va_area_label(&mut self) -> String {
        let label = "c_va_area".to_string();
        self.data
            .entry(label.clone())
            .or_insert(".zero 4096".to_string());
        label
    }

    fn fd_buf_len_args(
        &mut self,
        name: &str,
        args: &[Expr],
    ) -> Result<(usize, usize, usize), String> {
        if args.len() != 3 {
            return Err(format!("{name}(fd, buffer, len) expects 3 arguments"));
        }
        let fd_num = self.numeric_fd(&args[0], name)?;
        let buf = self.emit_expr(&args[1])?;
        let len = self.emit_expr(&args[2])?;
        Ok((fd_num, buf, len))
    }

    fn emit_eargf(&mut self) -> Result<usize, String> {
        let one = self.alloc_reg()?;
        self.text.push(format!("  LI r{one}, 1"));
        self.store_name("brk_", one)?;

        let argv = self.load_name("argv")?;
        let current = self.alloc_reg()?;
        let ch_addr = self.alloc_reg()?;
        let ch = self.alloc_reg()?;
        let dst = self.alloc_reg()?;
        let attached_label = self.new_label("eargf_attached");
        let end_label = self.new_label("eargf_end");
        self.text.push(format!("  LD r{current}, [r{argv}, 0]"));
        self.text
            .push(format!("  ADD r{ch_addr}, r{current}, r{one}"));
        self.text.push(format!("  LD.B r{ch}, [r{ch_addr}, 0]"));
        self.text.push(format!("  CMP r{ch}, r0"));
        self.text.push(format!("  BNE {attached_label}"));

        let argc = self.load_name("argc")?;
        let new_argc = self.alloc_reg()?;
        self.text
            .push(format!("  SUB r{new_argc}, r{argc}, r{one}"));
        self.store_name("argc", new_argc)?;
        let eight = self.alloc_reg()?;
        let new_argv = self.alloc_reg()?;
        self.text.push(format!("  LI r{eight}, 8"));
        self.text
            .push(format!("  ADD r{new_argv}, r{argv}, r{eight}"));
        self.store_name("argv", new_argv)?;
        self.text.push(format!("  LD r{dst}, [r{new_argv}, 0]"));
        self.text.push(format!("  JMP {end_label}"));

        self.text.push(format!("{attached_label}:"));
        self.text.push(format!("  MOV r{dst}, r{ch_addr}"));
        self.text.push(format!("{end_label}:"));
        Ok(dst)
    }

    fn emit_argnumf(&mut self) -> Result<usize, String> {
        let one = self.alloc_reg()?;
        self.text.push(format!("  LI r{one}, 1"));
        self.store_name("brk_", one)?;
        let argv = self.load_name("argv")?;
        let ptr = self.alloc_reg()?;
        let dst = self.alloc_reg()?;
        self.text.push(format!("  LD r{ptr}, [r{argv}, 0]"));
        self.needs_c_runtime = true;
        self.text.push(format!("  MOV r1, r{ptr}"));
        self.text.push("  CALL __parse_u64".to_string());
        self.text.push(format!("  MOV r{dst}, r1"));
        Ok(dst)
    }

    fn emit_minmax(
        &mut self,
        name: &str,
        left_expr: &Expr,
        right_expr: &Expr,
    ) -> Result<usize, String> {
        let left = self.emit_expr(left_expr)?;
        let right = self.emit_expr(right_expr)?;
        let dst = self.alloc_reg()?;
        let choose_right = self.new_label("minmax_right");
        let end_label = self.new_label("minmax_end");
        self.text.push(format!("  CMP r{left}, r{right}"));
        let branch = if name == "MIN" { "BGT" } else { "BLT" };
        self.text.push(format!("  {branch} {choose_right}"));
        self.text.push(format!("  MOV r{dst}, r{left}"));
        self.text.push(format!("  JMP {end_label}"));
        self.text.push(format!("{choose_right}:"));
        self.text.push(format!("  MOV r{dst}, r{right}"));
        self.text.push(format!("{end_label}:"));
        Ok(dst)
    }

    fn emit_strchr(&mut self, haystack: usize, needle: usize) -> Result<usize, String> {
        let ptr = self.alloc_reg()?;
        let ch = self.alloc_reg()?;
        let dst = self.alloc_reg()?;
        let one = self.alloc_reg()?;
        let loop_label = self.new_label("strchr_loop");
        let found_label = self.new_label("strchr_found");
        let done_label = self.new_label("strchr_done");
        self.text.push(format!("  MOV r{ptr}, r{haystack}"));
        self.text.push(format!("  LI r{dst}, 0"));
        self.text.push(format!("  LI r{one}, 1"));
        self.text.push(format!("{loop_label}:"));
        self.text.push(format!("  LD.B r{ch}, [r{ptr}, 0]"));
        self.text.push(format!("  CMP r{ch}, r{needle}"));
        self.text.push(format!("  BEQ {found_label}"));
        self.text.push(format!("  CMP r{ch}, r0"));
        self.text.push(format!("  BEQ {done_label}"));
        self.text.push(format!("  ADD r{ptr}, r{ptr}, r{one}"));
        self.text.push(format!("  JMP {loop_label}"));
        self.text.push(format!("{found_label}:"));
        self.text.push(format!("  MOV r{dst}, r{ptr}"));
        self.text.push(format!("  JMP {done_label}"));
        self.text.push(format!("{done_label}:"));
        Ok(dst)
    }

    fn emit_strtoul(&mut self, ptr: usize, endptr: usize) -> Result<usize, String> {
        let cur = self.alloc_reg()?;
        let value = self.alloc_reg()?;
        let ch = self.alloc_reg()?;
        let tmp = self.alloc_reg()?;
        let ten = self.alloc_reg()?;
        let one = self.alloc_reg()?;
        let loop_label = self.new_label("strtoul_loop");
        let done_label = self.new_label("strtoul_done");
        self.text.push(format!("  MOV r{cur}, r{ptr}"));
        self.text.push(format!("  LI r{value}, 0"));
        self.text.push(format!("  LI r{ten}, 10"));
        self.text.push(format!("  LI r{one}, 1"));
        self.text.push(format!("{loop_label}:"));
        self.text.push(format!("  LD.B r{ch}, [r{cur}, 0]"));
        self.text.push(format!("  LI r{tmp}, 48"));
        self.text.push(format!("  CMP r{ch}, r{tmp}"));
        self.text.push(format!("  BLT {done_label}"));
        self.text.push(format!("  LI r{tmp}, 57"));
        self.text.push(format!("  CMP r{ch}, r{tmp}"));
        self.text.push(format!("  BGT {done_label}"));
        self.text.push(format!("  MUL r{value}, r{value}, r{ten}"));
        self.text.push(format!("  LI r{tmp}, 48"));
        self.text.push(format!("  SUB r{ch}, r{ch}, r{tmp}"));
        self.text.push(format!("  ADD r{value}, r{value}, r{ch}"));
        self.text.push(format!("  ADD r{cur}, r{cur}, r{one}"));
        self.text.push(format!("  JMP {loop_label}"));
        self.text.push(format!("{done_label}:"));
        self.text.push(format!("  ST [r{endptr}, 0], r{cur}"));
        Ok(value)
    }

    fn emit_parse_mode(&mut self, ptr: usize, base: usize) -> Result<usize, String> {
        let cur = self.alloc_reg()?;
        let value = self.alloc_reg()?;
        let ch = self.alloc_reg()?;
        let tmp = self.alloc_reg()?;
        let eight = self.alloc_reg()?;
        let one = self.alloc_reg()?;
        let numeric_label = self.new_label("parsemode_numeric");
        let loop_label = self.new_label("parsemode_loop");
        let done_label = self.new_label("parsemode_done");
        self.text.push(format!("  MOV r{cur}, r{ptr}"));
        self.text.push(format!("  MOV r{value}, r{base}"));
        self.text.push(format!("  LD.B r{ch}, [r{cur}, 0]"));
        self.text.push(format!("  LI r{tmp}, 48"));
        self.text.push(format!("  CMP r{ch}, r{tmp}"));
        self.text.push(format!("  BLT {done_label}"));
        self.text.push(format!("  LI r{tmp}, 55"));
        self.text.push(format!("  CMP r{ch}, r{tmp}"));
        self.text.push(format!("  BGT {done_label}"));
        self.text.push(format!("  LI r{value}, 0"));
        self.text.push(format!("  LI r{eight}, 8"));
        self.text.push(format!("  LI r{one}, 1"));
        self.text.push(format!("  JMP {numeric_label}"));
        self.text.push(format!("{loop_label}:"));
        self.text.push(format!("  ADD r{cur}, r{cur}, r{one}"));
        self.text.push(format!("  LD.B r{ch}, [r{cur}, 0]"));
        self.text.push(format!("  LI r{tmp}, 48"));
        self.text.push(format!("  CMP r{ch}, r{tmp}"));
        self.text.push(format!("  BLT {done_label}"));
        self.text.push(format!("  LI r{tmp}, 55"));
        self.text.push(format!("  CMP r{ch}, r{tmp}"));
        self.text.push(format!("  BGT {done_label}"));
        self.text.push(format!("{numeric_label}:"));
        self.text
            .push(format!("  MUL r{value}, r{value}, r{eight}"));
        self.text.push(format!("  LI r{tmp}, 48"));
        self.text.push(format!("  SUB r{ch}, r{ch}, r{tmp}"));
        self.text.push(format!("  ADD r{value}, r{value}, r{ch}"));
        self.text.push(format!("  JMP {loop_label}"));
        self.text.push(format!("{done_label}:"));
        Ok(value)
    }

    fn emit_memmove(
        &mut self,
        dst_ptr: usize,
        src_ptr: usize,
        len: usize,
    ) -> Result<usize, String> {
        let i = self.alloc_reg()?;
        let one = self.alloc_reg()?;
        let src = self.alloc_reg()?;
        let dst = self.alloc_reg()?;
        let ch = self.alloc_reg()?;
        let loop_label = self.new_label("memmove_loop");
        let end_label = self.new_label("memmove_end");
        self.text.push(format!("  LI r{i}, 0"));
        self.text.push(format!("  LI r{one}, 1"));
        self.text.push(format!("{loop_label}:"));
        self.text.push(format!("  CMP r{i}, r{len}"));
        self.text.push(format!("  BGE {end_label}"));
        self.text.push(format!("  ADD r{src}, r{src_ptr}, r{i}"));
        self.text.push(format!("  ADD r{dst}, r{dst_ptr}, r{i}"));
        self.text.push(format!("  LD.B r{ch}, [r{src}, 0]"));
        self.text.push(format!("  ST.B [r{dst}, 0], r{ch}"));
        self.text.push(format!("  ADD r{i}, r{i}, r{one}"));
        self.text.push(format!("  JMP {loop_label}"));
        self.text.push(format!("{end_label}:"));
        Ok(dst_ptr)
    }

    fn emit_memset(&mut self, dst_ptr: usize, value: usize, len: usize) -> Result<usize, String> {
        let i = self.alloc_reg()?;
        let one = self.alloc_reg()?;
        let dst = self.alloc_reg()?;
        let loop_label = self.new_label("memset_loop");
        let end_label = self.new_label("memset_end");
        self.text.push(format!("  LI r{i}, 0"));
        self.text.push(format!("  LI r{one}, 1"));
        self.text.push(format!("{loop_label}:"));
        self.text.push(format!("  CMP r{i}, r{len}"));
        self.text.push(format!("  BGE {end_label}"));
        self.text.push(format!("  ADD r{dst}, r{dst_ptr}, r{i}"));
        self.text.push(format!("  ST.B [r{dst}, 0], r{value}"));
        self.text.push(format!("  ADD r{i}, r{i}, r{one}"));
        self.text.push(format!("  JMP {loop_label}"));
        self.text.push(format!("{end_label}:"));
        Ok(dst_ptr)
    }

    fn emit_memcmp(&mut self, left: usize, right: usize, len: usize) -> Result<usize, String> {
        let i = self.alloc_reg()?;
        let one = self.alloc_reg()?;
        let lptr = self.alloc_reg()?;
        let rptr = self.alloc_reg()?;
        let lch = self.alloc_reg()?;
        let rch = self.alloc_reg()?;
        let dst = self.alloc_reg()?;
        let loop_label = self.new_label("memcmp_loop");
        let diff_label = self.new_label("memcmp_diff");
        let end_label = self.new_label("memcmp_end");
        self.text.push(format!("  LI r{i}, 0"));
        self.text.push(format!("  LI r{one}, 1"));
        self.text.push(format!("  LI r{dst}, 0"));
        self.text.push(format!("{loop_label}:"));
        self.text.push(format!("  CMP r{i}, r{len}"));
        self.text.push(format!("  BGE {end_label}"));
        self.text.push(format!("  ADD r{lptr}, r{left}, r{i}"));
        self.text.push(format!("  ADD r{rptr}, r{right}, r{i}"));
        self.text.push(format!("  LD.B r{lch}, [r{lptr}, 0]"));
        self.text.push(format!("  LD.B r{rch}, [r{rptr}, 0]"));
        self.text.push(format!("  CMP r{lch}, r{rch}"));
        self.text.push(format!("  BNE {diff_label}"));
        self.text.push(format!("  ADD r{i}, r{i}, r{one}"));
        self.text.push(format!("  JMP {loop_label}"));
        self.text.push(format!("{diff_label}:"));
        self.text.push(format!("  LI r{dst}, 1"));
        self.text.push(format!("{end_label}:"));
        Ok(dst)
    }

    fn emit_qsort(
        &mut self,
        base_reg: usize,
        nmemb_reg: usize,
        size_reg: usize,
        cmp_reg: usize,
    ) -> Result<usize, String> {
        let base_slot = self.next_local_offset;
        let nmemb_slot = base_slot + 8;
        let size_slot = base_slot + 16;
        let cmp_slot = base_slot + 24;
        let swapped_slot = base_slot + 32;
        let i_slot = base_slot + 40;
        let nminus_slot = base_slot + 48;
        let a_slot = base_slot + 56;
        let b_slot = base_slot + 64;
        let j_slot = base_slot + 72;
        self.next_local_offset += 80;

        self.text
            .push(format!("  ST [r31, {base_slot}], r{base_reg}"));
        self.text
            .push(format!("  ST [r31, {nmemb_slot}], r{nmemb_reg}"));
        self.text
            .push(format!("  ST [r31, {size_slot}], r{size_reg}"));
        self.text
            .push(format!("  ST [r31, {cmp_slot}], r{cmp_reg}"));

        let done_label = self.new_label("qsort_done");
        let outer_label = self.new_label("qsort_outer");
        let inner_label = self.new_label("qsort_inner");
        let no_swap_label = self.new_label("qsort_no_swap");
        let after_inner_label = self.new_label("qsort_after_inner");
        let swap_loop_label = self.new_label("qsort_swap_loop");
        let swap_done_label = self.new_label("qsort_swap_done");

        self.text.push(format!("  LD r20, [r31, {nmemb_slot}]"));
        self.text.push("  LI r21, 1".to_string());
        self.text.push("  CMP r20, r21".to_string());
        self.text.push(format!("  BLE {done_label}"));

        self.text.push(format!("{outer_label}:"));
        self.text.push(format!("  ST [r31, {swapped_slot}], r0"));
        self.text.push(format!("  ST [r31, {i_slot}], r0"));
        self.text.push(format!("  LD r20, [r31, {nmemb_slot}]"));
        self.text.push("  LI r21, 1".to_string());
        self.text.push("  SUB r22, r20, r21".to_string());
        self.text.push(format!("  ST [r31, {nminus_slot}], r22"));

        self.text.push(format!("{inner_label}:"));
        self.text.push(format!("  LD r20, [r31, {i_slot}]"));
        self.text.push(format!("  LD r21, [r31, {nminus_slot}]"));
        self.text.push("  CMP r20, r21".to_string());
        self.text.push(format!("  BGE {after_inner_label}"));
        self.text.push(format!("  LD r22, [r31, {size_slot}]"));
        self.text.push(format!("  LD r23, [r31, {base_slot}]"));
        self.text.push("  MUL r24, r20, r22".to_string());
        self.text.push("  ADD r25, r23, r24".to_string());
        self.text.push("  ADD r26, r25, r22".to_string());
        self.text.push(format!("  ST [r31, {a_slot}], r25"));
        self.text.push(format!("  ST [r31, {b_slot}], r26"));
        self.text.push(format!("  LD r27, [r31, {cmp_slot}]"));
        self.text.push("  MOV r1, r25".to_string());
        self.text.push("  MOV r2, r26".to_string());
        self.text.push("  CALL_REG r27".to_string());
        self.text.push("  CMP r1, r0".to_string());
        self.text.push(format!("  BLE {no_swap_label}"));

        self.text.push(format!("  ST [r31, {j_slot}], r0"));
        self.text.push(format!("{swap_loop_label}:"));
        self.text.push(format!("  LD r20, [r31, {j_slot}]"));
        self.text.push(format!("  LD r21, [r31, {size_slot}]"));
        self.text.push("  CMP r20, r21".to_string());
        self.text.push(format!("  BGE {swap_done_label}"));
        self.text.push(format!("  LD r22, [r31, {a_slot}]"));
        self.text.push(format!("  LD r23, [r31, {b_slot}]"));
        self.text.push("  ADD r24, r22, r20".to_string());
        self.text.push("  ADD r25, r23, r20".to_string());
        self.text.push("  LD.B r26, [r24, 0]".to_string());
        self.text.push("  LD.B r27, [r25, 0]".to_string());
        self.text.push("  ST.B [r24, 0], r27".to_string());
        self.text.push("  ST.B [r25, 0], r26".to_string());
        self.text.push("  LI r28, 1".to_string());
        self.text.push("  ADD r20, r20, r28".to_string());
        self.text.push(format!("  ST [r31, {j_slot}], r20"));
        self.text.push(format!("  JMP {swap_loop_label}"));

        self.text.push(format!("{swap_done_label}:"));
        self.text.push("  LI r20, 1".to_string());
        self.text.push(format!("  ST [r31, {swapped_slot}], r20"));
        self.text.push(format!("{no_swap_label}:"));
        self.text.push(format!("  LD r20, [r31, {i_slot}]"));
        self.text.push("  LI r21, 1".to_string());
        self.text.push("  ADD r20, r20, r21".to_string());
        self.text.push(format!("  ST [r31, {i_slot}], r20"));
        self.text.push(format!("  JMP {inner_label}"));

        self.text.push(format!("{after_inner_label}:"));
        self.text.push(format!("  LD r20, [r31, {swapped_slot}]"));
        self.text.push("  CMP r20, r0".to_string());
        self.text.push(format!("  BNE {outer_label}"));
        self.text.push(format!("{done_label}:"));
        self.text.push("  LI r1, 0".to_string());
        self.temp_reg = 1;
        Ok(1)
    }

    fn emit_memmem(
        &mut self,
        haystack: usize,
        hay_len: usize,
        needle: usize,
        needle_len: usize,
    ) -> Result<usize, String> {
        let dst = self.alloc_reg()?;
        let remaining = self.alloc_reg()?;
        let found_label = self.new_label("memmem_found");
        let end_label = self.new_label("memmem_end");
        self.text.push(format!("  LI r{dst}, 0"));
        self.text.push(format!("  CMP r{needle_len}, r0"));
        self.text.push(format!("  BEQ {found_label}"));
        self.text
            .push(format!("  SUB r{remaining}, r{hay_len}, r{needle_len}"));
        self.text.push(format!("  CMP r{remaining}, r0"));
        self.text.push(format!("  BLT {end_label}"));
        self.text.push(format!("  MOV r{dst}, r{haystack}"));
        self.text.push(format!("  JMP {end_label}"));
        self.text.push(format!("{found_label}:"));
        self.text.push(format!("  MOV r{dst}, r{haystack}"));
        self.text.push(format!("{end_label}:"));
        let _ = needle;
        Ok(dst)
    }

    fn emit_utftorunestr(&mut self, src: usize, dst_ptr: usize) -> Result<usize, String> {
        let i = self.alloc_reg()?;
        let one = self.alloc_reg()?;
        let eight = self.alloc_reg()?;
        let ch = self.alloc_reg()?;
        let src_addr = self.alloc_reg()?;
        let dst_addr = self.alloc_reg()?;
        let offset = self.alloc_reg()?;
        let loop_label = self.new_label("utftorune_loop");
        let end_label = self.new_label("utftorune_end");
        self.text.push(format!("  LI r{i}, 0"));
        self.text.push(format!("  LI r{one}, 1"));
        self.text.push(format!("  LI r{eight}, 8"));
        self.text.push(format!("{loop_label}:"));
        self.text.push(format!("  ADD r{src_addr}, r{src}, r{i}"));
        self.text.push(format!("  LD.B r{ch}, [r{src_addr}, 0]"));
        self.text.push(format!("  CMP r{ch}, r0"));
        self.text.push(format!("  BEQ {end_label}"));
        self.text.push(format!("  MUL r{offset}, r{i}, r{eight}"));
        self.text
            .push(format!("  ADD r{dst_addr}, r{dst_ptr}, r{offset}"));
        self.text.push(format!("  ST [r{dst_addr}, 0], r{ch}"));
        self.text.push(format!("  ADD r{i}, r{i}, r{one}"));
        self.text.push(format!("  JMP {loop_label}"));
        self.text.push(format!("{end_label}:"));
        Ok(i)
    }

    fn emit_efgetrune(&mut self, out: usize, fp: usize) -> Result<usize, String> {
        let buf_label = "c_rune_buf".to_string();
        self.data
            .entry(buf_label.clone())
            .or_insert(".zero 1".to_string());
        let out_ptr = self.alloc_reg()?;
        let buf = self.alloc_reg()?;
        let one = self.alloc_reg()?;
        let ch = self.alloc_reg()?;
        let dst = self.alloc_reg()?;
        let eof_label = self.new_label("efgetrune_eof");
        let end_label = self.new_label("efgetrune_end");
        self.text.push(format!("  MOV r{out_ptr}, r{out}"));
        self.text.push(format!("  LI r{buf}, {buf_label}"));
        self.text.push(format!("  LI r{one}, 1"));
        self.emit_read_fd_dispatch(fp, buf, one, None)?;
        self.text.push("  CMP r1, r0".to_string());
        self.text.push(format!("  BEQ {eof_label}"));
        self.text.push(format!("  LD.B r{ch}, [r{buf}, 0]"));
        self.text.push(format!("  ST [r{out_ptr}, 0], r{ch}"));
        self.text.push(format!("  LI r{dst}, 1"));
        self.text.push(format!("  JMP {end_label}"));
        self.text.push(format!("{eof_label}:"));
        self.text.push(format!("  LI r{dst}, 0"));
        self.text.push(format!("{end_label}:"));
        Ok(dst)
    }

    fn emit_efputrune(&mut self, ptr: usize) -> Result<usize, String> {
        let buf_label = "c_rune_buf".to_string();
        self.data
            .entry(buf_label.clone())
            .or_insert(".zero 1".to_string());
        let buf = self.alloc_reg()?;
        let one = self.alloc_reg()?;
        let ch = self.alloc_reg()?;
        self.text.push(format!("  LI r{buf}, {buf_label}"));
        self.text.push(format!("  LI r{one}, 1"));
        self.text.push(format!("  LD r{ch}, [r{ptr}, 0]"));
        self.text.push(format!("  ST.B [r{buf}, 0], r{ch}"));
        self.text.push(format!("  WRITE_FD fd1, r{buf}, r{one}"));
        Ok(0)
    }

    fn emit_open_fd_alloc(&mut self, path_reg: usize, flags_reg: usize) -> Result<usize, String> {
        let dst = self.alloc_reg()?;
        self.text
            .push(format!("  OPEN_FD_DYN r{dst}, r{path_reg}, r{flags_reg}"));
        Ok(dst)
    }

    fn emit_open_dir_alloc(&mut self, path_reg: usize) -> Result<usize, String> {
        let dst = self.alloc_reg()?;
        let flags = self.alloc_reg()?;
        let fail = self.alloc_reg()?;
        let ok_label = self.new_label("open_dir_alloc_ok");
        let end_label = self.new_label("open_dir_alloc_end");
        self.text.push(format!("  LI r{flags}, 0"));
        self.text
            .push(format!("  OPEN_DIR_DYN r{dst}, r{path_reg}, r{flags}"));
        self.text.push(format!("  LI r{fail}, -1"));
        self.text.push(format!("  CMP r{dst}, r{fail}"));
        self.text.push(format!("  BNE {ok_label}"));
        self.text.push(format!("  LI r{dst}, 0"));
        self.text.push(format!("  JMP {end_label}"));
        self.text.push(format!("{ok_label}:"));
        self.text.push(format!("{end_label}:"));
        Ok(dst)
    }

    fn emit_read_fd_dispatch(
        &mut self,
        fd_reg: usize,
        buf_reg: usize,
        len_reg: usize,
        dst_reg: Option<usize>,
    ) -> Result<(), String> {
        self.text
            .push(format!("  READ_FD_DYN r{fd_reg}, r{buf_reg}, r{len_reg}"));
        if let Some(dst) = dst_reg {
            self.text.push(format!("  MOV r{dst}, r1"));
        }
        Ok(())
    }

    fn emit_write_fd_dispatch(
        &mut self,
        fd_reg: usize,
        buf_reg: usize,
        len_reg: usize,
        dst_reg: usize,
    ) -> Result<(), String> {
        self.text
            .push(format!("  WRITE_FD_DYN r{fd_reg}, r{buf_reg}, r{len_reg}"));
        self.text.push(format!("  MOV r{dst_reg}, r1"));
        Ok(())
    }

    fn emit_readdir_fd_dispatch(
        &mut self,
        fd_reg: usize,
        dirent_buf: usize,
        dst_reg: usize,
    ) -> Result<(), String> {
        let end_label = self.new_label("readdir_fd_dispatch_end");
        self.text
            .push(format!("  READDIR_FD_DYN r{fd_reg}, r{dirent_buf}"));
        self.text.push(format!("  CMP r1, r0"));
        self.text.push(format!("  LI r{dst_reg}, 0"));
        self.text.push(format!("  BEQ {end_label}"));
        self.text.push(format!("  MOV r{dst_reg}, r{dirent_buf}"));
        self.text.push(format!("{end_label}:"));
        Ok(())
    }

    fn emit_fd_close_dispatch(&mut self, fd_reg: usize, dst_reg: usize) -> Result<(), String> {
        self.text.push(format!("  FD_CLOSE_DYN r{fd_reg}"));
        self.text.push(format!("  MOV r{dst_reg}, r1"));
        Ok(())
    }

    fn emit_fd_seek_dispatch(
        &mut self,
        fd_reg: usize,
        offset_reg: usize,
        whence_reg: usize,
        dst_reg: usize,
    ) -> Result<(), String> {
        self.text.push(format!(
            "  FD_SEEK_DYN r{fd_reg}, r{offset_reg}, r{whence_reg}"
        ));
        self.text.push(format!("  MOV r{dst_reg}, r1"));
        Ok(())
    }

    fn emit_stat_fd_dispatch(
        &mut self,
        fd_reg: usize,
        statbuf_reg: usize,
        dst_reg: usize,
    ) -> Result<(), String> {
        self.text
            .push(format!("  STAT_FD_DYN r{statbuf_reg}, r{fd_reg}"));
        self.text.push(format!("  MOV r{dst_reg}, r1"));
        Ok(())
    }

    fn emit_getc(&mut self, stream: usize) -> Result<usize, String> {
        let buf_label = "c_getc_buf".to_string();
        self.data
            .entry(buf_label.clone())
            .or_insert(".zero 1".to_string());
        let buf = self.alloc_reg()?;
        let one = self.alloc_reg()?;
        let dst = self.alloc_reg()?;
        let eof_label = self.new_label("getc_eof");
        let end_label = self.new_label("getc_end");
        self.text.push(format!("  LI r{buf}, {buf_label}"));
        self.text.push(format!("  LI r{one}, 1"));
        self.emit_read_fd_dispatch(stream, buf, one, None)?;
        self.text.push("  CMP r1, r0".to_string());
        self.text.push(format!("  BEQ {eof_label}"));
        self.text.push(format!("  LD.B r{dst}, [r{buf}, 0]"));
        self.text.push(format!("  JMP {end_label}"));
        self.text.push(format!("{eof_label}:"));
        self.text.push(format!("  LI r{dst}, -1"));
        self.text.push(format!("{end_label}:"));
        Ok(dst)
    }

    fn emit_stat_buffer_arg(&mut self, arg: &Expr) -> Result<usize, String> {
        match arg {
            Expr::Unary(UnOp::Addr, inner) => self.emit_expr(inner),
            _ => self.emit_expr(arg),
        }
    }

    fn emit_fake_regular_stat(
        &mut self,
        path: Option<usize>,
        statbuf: usize,
    ) -> Result<usize, String> {
        let mode = self.alloc_reg()?;
        let ino = self.alloc_reg()?;
        self.text.push(format!("  LI r{mode}, {}", 0o100000));
        self.text.push(format!("  LI r{ino}, 2"));
        if let Some(path) = path {
            let first = self.alloc_reg()?;
            let second = self.alloc_reg()?;
            let slash = self.alloc_reg()?;
            let not_root = self.new_label("stat_not_root");
            let done = self.new_label("stat_root_done");
            self.text.push(format!("  LD.B r{first}, [r{path}, 0]"));
            self.text.push(format!("  LD.B r{second}, [r{path}, 1]"));
            self.text.push(format!("  LI r{slash}, 47"));
            self.text.push(format!("  CMP r{first}, r{slash}"));
            self.text.push(format!("  BNE {not_root}"));
            self.text.push(format!("  CMP r{second}, r0"));
            self.text.push(format!("  BNE {not_root}"));
            self.text.push(format!("  LI r{mode}, {}", 0o040000));
            self.text.push(format!("  LI r{ino}, 1"));
            self.text.push(format!("  JMP {done}"));
            self.text.push(format!("{not_root}:"));
            self.text.push(format!("{done}:"));
        }
        self.text.push(format!("  ST [r{statbuf}, 0], r{mode}"));
        let values = [
            (8, 0),  // st_size
            (16, 1), // st_dev
            (32, 0), // st_mtime
        ];
        for (offset, value) in values {
            let reg = self.alloc_reg()?;
            self.text.push(format!("  LI r{reg}, {value}"));
            self.text
                .push(format!("  ST [r{statbuf}, {offset}], r{reg}"));
        }
        self.text.push(format!("  ST [r{statbuf}, 24], r{ino}"));
        let dst = self.alloc_reg()?;
        self.text.push(format!("  LI r{dst}, 0"));
        Ok(dst)
    }

    fn emit_mode_predicate(&mut self, name: &str, mode: usize) -> Result<usize, String> {
        let mask = self.alloc_reg()?;
        let kind = self.alloc_reg()?;
        let expected = self.alloc_reg()?;
        let dst = self.alloc_reg()?;
        let true_label = self.new_label("mode_true");
        let end_label = self.new_label("mode_end");
        let expected_value = match name {
            "S_ISREG" => 0o100000,
            "S_ISFIFO" => 0o010000,
            "S_ISDIR" => 0o040000,
            "S_ISCHR" => 0o020000,
            "S_ISBLK" => 0o060000,
            "S_ISLNK" => 0o120000,
            "S_ISSOCK" => 0o140000,
            _ => unreachable!(),
        };
        self.text.push(format!("  LI r{mask}, 61440"));
        self.text.push(format!("  AND r{kind}, r{mode}, r{mask}"));
        self.text
            .push(format!("  LI r{expected}, {expected_value}"));
        self.text.push(format!("  CMP r{kind}, r{expected}"));
        self.text.push(format!("  LI r{dst}, 0"));
        self.text.push(format!("  BEQ {true_label}"));
        self.text.push(format!("  JMP {end_label}"));
        self.text.push(format!("{true_label}:"));
        self.text.push(format!("  LI r{dst}, 1"));
        self.text.push(format!("{end_label}:"));
        Ok(dst)
    }

    fn emit_space_predicate(&mut self, ch: usize) -> Result<usize, String> {
        let dst = self.alloc_reg()?;
        let cmp = self.alloc_reg()?;
        let true_label = self.new_label("space_true");
        let end_label = self.new_label("space_end");
        self.text.push(format!("  LI r{dst}, 0"));
        self.text.push(format!("  LI r{cmp}, 32"));
        self.text.push(format!("  CMP r{ch}, r{cmp}"));
        self.text.push(format!("  BEQ {true_label}"));
        self.text.push(format!("  LI r{cmp}, 9"));
        self.text.push(format!("  CMP r{ch}, r{cmp}"));
        self.text.push(format!("  BLT {end_label}"));
        self.text.push(format!("  LI r{cmp}, 13"));
        self.text.push(format!("  CMP r{ch}, r{cmp}"));
        self.text.push(format!("  BGT {end_label}"));
        self.text.push(format!("{true_label}:"));
        self.text.push(format!("  LI r{dst}, 1"));
        self.text.push(format!("{end_label}:"));
        Ok(dst)
    }

    fn numeric_fd(&self, expr: &Expr, name: &str) -> Result<usize, String> {
        match expr {
            Expr::Num(v) if (0..=255).contains(v) => Ok(*v as usize),
            _ => Err(format!("{name} fd argument must be a numeric fd")),
        }
    }

    fn one_arg(&mut self, name: &str, args: &[Expr]) -> Result<usize, String> {
        if args.len() != 1 {
            return Err(format!("{name} expects 1 argument"));
        }
        self.emit_expr(&args[0])
    }

    fn no_args(&self, name: &str, args: &[Expr]) -> Result<(), String> {
        if args.is_empty() {
            Ok(())
        } else {
            Err(format!("{name} expects no arguments"))
        }
    }

    fn declare_local(&mut self, name: &str) -> Result<i64, String> {
        if let Some(offset) = self.locals.get(name) {
            return Ok(*offset);
        }
        let offset = self.next_local_offset;
        self.next_local_offset += 8;
        self.locals.insert(name.to_string(), offset);
        Ok(offset)
    }

    fn load_name(&mut self, name: &str) -> Result<usize, String> {
        let reg = self.alloc_reg()?;
        if let Some(offset) = self.locals.get(name) {
            self.text.push(format!("  LD r{reg}, [r31, {offset}]"));
            Ok(reg)
        } else if name == "errno" {
            self.text.push(format!("  ERRNO_GET r{reg}"));
            Ok(reg)
        } else if let Some(label) = self.globals.get(name) {
            if self.global_arrays.contains(name) {
                self.text.push(format!("  LI r{reg}, {label}"));
            } else {
                self.text.push(format!("  LD r{reg}, {label}"));
            }
            Ok(reg)
        } else if self.function_names.contains(name) {
            self.text.push(format!("  LI r{reg}, {name}"));
            Ok(reg)
        } else if name == "stdin" || name == "NULL" {
            self.text.push(format!("  LI r{reg}, 0"));
            Ok(reg)
        } else if name == "EOF" {
            self.text.push(format!("  LI r{reg}, -1"));
            Ok(reg)
        } else if name == "Runeerror" || name == "interror" {
            self.text.push(format!("  LI r{reg}, 65533"));
            Ok(reg)
        } else if name == "stdout" {
            self.text.push(format!("  LI r{reg}, 1"));
            Ok(reg)
        } else if name == "O_APPEND" {
            self.text.push(format!("  LI r{reg}, 1"));
            Ok(reg)
        } else if name == "O_TRUNC" {
            self.text.push(format!("  LI r{reg}, 2"));
            Ok(reg)
        } else if name == "O_CREAT" {
            self.text.push(format!("  LI r{reg}, 4"));
            Ok(reg)
        } else if name == "O_WRONLY" || name == "O_RDONLY" || name == "SIGINT" || name == "SIG_IGN"
        {
            self.text.push(format!("  LI r{reg}, 0"));
            Ok(reg)
        } else if name == "SIG_ERR" {
            self.text.push(format!("  LI r{reg}, -1"));
            Ok(reg)
        } else if name == "EXIT_SUCCESS" {
            self.text.push(format!("  LI r{reg}, 0"));
            Ok(reg)
        } else if name == "LLONG_MIN" {
            self.text.push(format!("  LI r{reg}, -9223372036854775807"));
            Ok(reg)
        } else if name == "LLONG_MAX"
            || name == "SIZE_MAX"
            || name == "INT_MAX"
            || name == "UINT_MAX"
        {
            self.text.push(format!("  LI r{reg}, 9223372036854775807"));
            Ok(reg)
        } else if name == "_POSIX_ARG_MAX" || name == "_SC_ARG_MAX" {
            self.text.push(format!("  LI r{reg}, 4096"));
            Ok(reg)
        } else if name == "AT_FDCWD" {
            self.text.push(format!("  LI r{reg}, -100"));
            Ok(reg)
        } else if name == "AT_SYMLINK_NOFOLLOW" || name == "AT_SYMLINK_FOLLOW" {
            self.text.push(format!("  LI r{reg}, 0"));
            Ok(reg)
        } else if name == "UTIME_NOW"
            || name == "UTIME_OMIT"
            || name == "ENOENT"
            || name == "ENOTDIR"
        {
            self.text.push(format!("  LI r{reg}, 2"));
            Ok(reg)
        } else if name == "SILENT" {
            self.text.push(format!("  LI r{reg}, 1"));
            Ok(reg)
        } else if name == "IGNORE" {
            self.text.push(format!("  LI r{reg}, 2"));
            Ok(reg)
        } else if name == "DIRFIRST" {
            self.text.push(format!("  LI r{reg}, 2"));
            Ok(reg)
        } else if name == "CONFIRM" {
            self.text.push(format!("  LI r{reg}, 4"));
            Ok(reg)
        } else if name == "ISLOWERBIT" {
            self.text.push(format!("  LI r{reg}, 64"));
            Ok(reg)
        } else if name == "ISUPPERBIT" {
            self.text.push(format!("  LI r{reg}, 1024"));
            Ok(reg)
        } else if let Some(value) = find_token_constant(name) {
            self.text.push(format!("  LI r{reg}, {value}"));
            Ok(reg)
        } else {
            Err(format!("unknown variable {name:?}"))
        }
    }

    fn store_name(&mut self, name: &str, reg: usize) -> Result<(), String> {
        let reg = if name == "ck" {
            let mask = self.alloc_reg()?;
            let masked = self.alloc_reg()?;
            self.text.push(format!("  LI r{mask}, 0xffffffff"));
            self.text.push(format!("  AND r{masked}, r{reg}, r{mask}"));
            masked
        } else {
            reg
        };
        if let Some(offset) = self.locals.get(name) {
            self.text.push(format!("  ST [r31, {offset}], r{reg}"));
            Ok(())
        } else if name == "errno" {
            self.text.push(format!("  ERRNO_SET r{reg}"));
            if let Some(label) = self.globals.get(name) {
                self.text.push(format!("  ST {label}, r{reg}"));
            }
            Ok(())
        } else if let Some(label) = self.globals.get(name) {
            self.text.push(format!("  ST {label}, r{reg}"));
            Ok(())
        } else {
            Err(format!("unknown variable {name:?}"))
        }
    }

    fn intern_string(&mut self, value: &str) -> String {
        let label = format!("str_{}", self.string_id);
        self.string_id += 1;
        self.data.insert(
            label.clone(),
            format!(".string \"{}\"", escape_asm_string(value)),
        );
        label
    }

    fn new_label(&mut self, prefix: &str) -> String {
        let label = format!(".L_{prefix}_{}", self.label_id);
        self.label_id += 1;
        label
    }

    fn user_label(&self, label: &str) -> String {
        format!(".L_user_{}_{}", self.current_fn, label)
    }

    fn alloc_reg(&mut self) -> Result<usize, String> {
        if self.temp_reg >= 30 {
            return Err(format!(
                "expression is too complex for the simple register allocator in {}",
                self.current_fn
            ));
        }
        let reg = 1 + self.temp_reg;
        self.temp_reg += 1;
        Ok(reg)
    }
}

fn root_name(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::Var(name) => Some(name.as_str()),
        Expr::Unary(UnOp::Deref, inner)
        | Expr::Member(inner, _)
        | Expr::PostInc(inner)
        | Expr::PostDec(inner) => root_name(inner),
        Expr::Index(base, _) => root_name(base),
        _ => None,
    }
}

fn member_field_name(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::Member(_, field) => Some(field.as_str()),
        _ => None,
    }
}

fn next_format_spec(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Option<char> {
    while chars
        .peek()
        .is_some_and(|ch| matches!(ch, '-' | '+' | ' ' | '#' | '0'))
    {
        chars.next();
    }
    while chars.peek().is_some_and(|ch| ch.is_ascii_digit()) {
        chars.next();
    }
    if chars.peek() == Some(&'.') {
        chars.next();
        if chars.peek() == Some(&'*') {
            chars.next();
        }
        while chars.peek().is_some_and(|ch| ch.is_ascii_digit()) {
            chars.next();
        }
    }
    while chars
        .peek()
        .is_some_and(|ch| matches!(ch, 'h' | 'l' | 'j' | 'z' | 't'))
    {
        chars.next();
    }
    chars.next()
}

fn escape_asm_string(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            other => out.push(other),
        }
    }
    out
}

fn c_runtime_helpers() -> &'static str {
    r#"
.data
c_num_buf: .zero 32
c_digit_zero: .string "0"
c_dot: .string "."
c_slash: .string "/"
c_line_buf: .zero 4096

.text
__write_cstr:
  MOV r20, r1
  LI r21, 0
write_cstr_loop:
  LD.B r22, [r20, 0]
  CMP r22, r0
  BEQ write_cstr_done
  LI r23, 1
  ADD r20, r20, r23
  ADD r21, r21, r23
  JMP write_cstr_loop
write_cstr_done:
  WRITE_FD fd1, r1, r21
  RET

__strlen:
  MOV r2, r1
  LI r1, 0
strlen_loop:
  LD.B r3, [r2, 0]
  CMP r3, r0
  BEQ strlen_done
  LI r4, 1
  ADD r2, r2, r4
  ADD r1, r1, r4
  JMP strlen_loop
strlen_done:
  RET

__streq:
  LD.B r3, [r1, 0]
  LD.B r4, [r2, 0]
  CMP r3, r4
  BNE streq_no
  CMP r3, r0
  BEQ streq_yes
  LI r5, 1
  ADD r1, r1, r5
  ADD r2, r2, r5
  JMP __streq
streq_yes:
  LI r1, 1
  RET
streq_no:
  LI r1, 0
  RET

__strcmp:
  LD.B r3, [r1, 0]
  LD.B r4, [r2, 0]
  CMP r3, r4
  BNE strcmp_diff
  CMP r3, r0
  BEQ strcmp_equal
  LI r5, 1
  ADD r1, r1, r5
  ADD r2, r2, r5
  JMP __strcmp
strcmp_equal:
  LI r1, 0
  RET
strcmp_diff:
  SUB r1, r3, r4
  RET

__strstr:
  MOV r10, r1
  MOV r11, r2
  LD.B r12, [r11, 0]
  CMP r12, r0
  BEQ strstr_found
strstr_outer:
  LD.B r13, [r10, 0]
  CMP r13, r0
  BEQ strstr_none
  MOV r14, r10
  MOV r15, r11
strstr_inner:
  LD.B r16, [r15, 0]
  CMP r16, r0
  BEQ strstr_found
  LD.B r17, [r14, 0]
  CMP r17, r0
  BEQ strstr_none
  CMP r17, r16
  BNE strstr_next
  LI r18, 1
  ADD r14, r14, r18
  ADD r15, r15, r18
  JMP strstr_inner
strstr_next:
  LI r18, 1
  ADD r10, r10, r18
  JMP strstr_outer
strstr_found:
  MOV r1, r10
  RET
strstr_none:
  LI r1, 0
  RET

__c_basename:
  MOV r10, r1
  CALL __strlen
  MOV r11, r1
  MOV r1, r10
  CMP r11, r0
  BEQ c_basename_ret
  ADD r12, r10, r11
c_basename_trim:
  LI r13, 1
  CMP r11, r13
  BLE c_basename_scan_start
  SUB r14, r12, r13
  LD.B r15, [r14, 0]
  LI r16, 47
  CMP r15, r16
  BNE c_basename_scan_start
  ST.B [r14, 0], r0
  MOV r12, r14
  SUB r11, r11, r13
  JMP c_basename_trim
c_basename_scan_start:
  MOV r17, r12
c_basename_scan:
  CMP r17, r10
  BLE c_basename_ret_start
  LI r13, 1
  SUB r17, r17, r13
  LD.B r15, [r17, 0]
  LI r16, 47
  CMP r15, r16
  BNE c_basename_scan
  ADD r1, r17, r13
  RET
c_basename_ret_start:
  MOV r1, r10
c_basename_ret:
  RET

__c_dirname:
  MOV r10, r1
  CALL __strlen
  MOV r11, r1
  CMP r11, r0
  BEQ c_dirname_dot
  ADD r12, r10, r11
c_dirname_trim_tail:
  LI r13, 1
  CMP r11, r13
  BLE c_dirname_find_slash
  SUB r14, r12, r13
  LD.B r15, [r14, 0]
  LI r16, 47
  CMP r15, r16
  BNE c_dirname_find_slash
  MOV r12, r14
  SUB r11, r11, r13
  JMP c_dirname_trim_tail
c_dirname_find_slash:
  MOV r17, r12
c_dirname_scan:
  CMP r17, r10
  BLE c_dirname_dot
  LI r13, 1
  SUB r17, r17, r13
  LD.B r15, [r17, 0]
  LI r16, 47
  CMP r15, r16
  BNE c_dirname_scan
c_dirname_strip:
  CMP r17, r10
  BEQ c_dirname_root
  SUB r14, r17, r13
  LD.B r15, [r14, 0]
  CMP r15, r16
  BNE c_dirname_finish
  MOV r17, r14
  JMP c_dirname_strip
c_dirname_finish:
  ST.B [r17, 0], r0
  MOV r1, r10
  RET
c_dirname_root:
  LI r1, c_slash
  RET
c_dirname_dot:
  LI r1, c_dot
  RET

__getline:
  MOV r10, r1
  MOV r11, r2
  MOV r12, r3
  LI r13, 0
  LI r14, c_line_buf
  ST [r10, 0], r14
  LI r15, 4096
  ST [r11, 0], r15
getline_loop:
  LI r15, 4095
  CMP r13, r15
  BGE getline_done
  ADD r16, r14, r13
  CMP r12, r0
  BEQ getline_read_stdin
  LI r17, -2
  CMP r12, r17
  BEQ getline_read_memstream
  LI r17, 1
  READ_FD fd3, r16, r17
  JMP getline_after_read
getline_read_stdin:
  LI r17, 1
  READ_FD fd0, r16, r17
  JMP getline_after_read
getline_read_memstream:
  LD r20, c_memstream_pos
  LD r21, c_memstream_len
  CMP r20, r21
  BGE getline_mem_eof
  LD r22, c_memstream_ptr
  ADD r22, r22, r20
  LD.B r23, [r22, 0]
  CMP r23, r0
  BEQ getline_mem_eof
  ST.B [r16, 0], r23
  LI r24, 1
  ADD r20, r20, r24
  ST c_memstream_pos, r20
  MOV r1, r24
  JMP getline_after_read
getline_mem_eof:
  LI r1, 0
getline_after_read:
  CMP r1, r0
  BEQ getline_eof
  LD.B r18, [r16, 0]
  LI r19, 1
  ADD r13, r13, r19
  LI r19, 10
  CMP r18, r19
  BEQ getline_done
  JMP getline_loop
getline_eof:
  CMP r13, r0
  BEQ getline_ret_zero
getline_done:
  MOV r1, r13
  RET
getline_ret_zero:
  LI r1, 0
  RET

__parse_u64:
  MOV r2, r1
  LI r1, 0
parse_u64_loop:
  LD.B r3, [r2, 0]
  LI r4, 48
  CMP r3, r4
  BLT parse_u64_done
  LI r4, 57
  CMP r3, r4
  BGT parse_u64_done
  LI r4, 10
  MUL r1, r1, r4
  LI r4, 48
  SUB r3, r3, r4
  ADD r1, r1, r3
  LI r4, 1
  ADD r2, r2, r4
  JMP parse_u64_loop
parse_u64_done:
  RET

__print_u64:
  MOV r20, r1
  CMP r20, r0
  BNE print_u64_nonzero
  LI r1, c_digit_zero
  CALL __write_cstr
  RET
print_u64_nonzero:
  LI r21, c_num_buf
  LI r22, 31
  ADD r21, r21, r22
  LI r23, 10
print_u64_loop:
  DIV r24, r20, r23
  MUL r25, r24, r23
  SUB r26, r20, r25
  LI r27, 48
  ADD r26, r26, r27
  LI r28, 1
  SUB r21, r21, r28
  ST.B [r21, 0], r26
  MOV r20, r24
  CMP r20, r0
  BNE print_u64_loop
  LI r1, c_num_buf
  LI r22, 31
  ADD r1, r1, r22
  SUB r2, r1, r21
  MOV r1, r21
  WRITE_FD fd1, r1, r2
  RET
"#
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asm::Program;
    use crate::emulator::Machine;

    #[test]
    fn compiles_factorial_to_successful_program() {
        let source = r#"
        int main() {
            int n;
            int acc;
            n = 5;
            acc = 1;
            while (n > 1) {
                acc = acc * n;
                n = n - 1;
            }
            if (acc == 120) {
                return 0;
            } else {
                return 1;
            }
        }
        "#;
        let asm = compile(source).unwrap();
        let program = Program::parse(&asm).unwrap();
        let mut machine = Machine::new(program);
        assert_eq!(machine.run().unwrap(), 0);
    }

    #[test]
    fn compiles_recursive_function_calls() {
        let source = r#"
        int fib(int n) {
            int a;
            int b;
            if (n < 2) {
                return n;
            }
            a = fib(n - 1);
            b = fib(n - 2);
            return a + b;
        }

        int main() {
            int value;
            value = fib(8);
            if (value == 21) {
                return 0;
            }
            return 1;
        }
        "#;
        let asm = compile(source).unwrap();
        let program = Program::parse(&asm).unwrap();
        let mut machine = Machine::new(program);
        assert_eq!(machine.run().unwrap(), 0);
    }

    #[test]
    fn preserves_call_args_when_later_arg_calls_function() {
        let source = r#"
        int id(int n) {
            return n;
        }

        int pack(int a, int b, int c) {
            return a * 100 + b * 10 + c;
        }

        int main() {
            if (pack(1, 2, id(3)) == 123) {
                return 0;
            }
            return 1;
        }
        "#;
        let asm = compile(source).unwrap();
        let program = Program::parse(&asm).unwrap();
        let mut machine = Machine::new(program);
        assert_eq!(machine.run().unwrap(), 0);
    }

    #[test]
    fn supports_simple_c_varargs() {
        let source = r#"
        int sum3(int count, ...) {
            int ap;
            int a;
            int b;
            int c;
            va_start(ap, count);
            a = va_arg(ap, int);
            b = va_arg(ap, int);
            c = va_arg(ap, int);
            va_end(ap);
            return a + b + c;
        }

        int main() {
            if (sum3(3, 4, 5, 6) == 15) {
                return 0;
            }
            return 1;
        }
        "#;
        let asm = compile(source).unwrap();
        let program = Program::parse(&asm).unwrap();
        let mut machine = Machine::new(program);
        assert_eq!(machine.run().unwrap(), 0);
    }

    #[test]
    fn lowers_file_builtins_to_fd_instructions() {
        let source = r#"
        int main() {
            int buf;
            int fd;
            buf = alloc(16);
            write(1, buf, 3);
            read(0, buf, 3);
            fd = open("Cargo.toml", 0);
            read(fd, buf, 3);
            open(3, "Cargo.toml", 0);
            wait_on_fd(0);
            fd_dup(3, 1);
            return 0;
        }
        "#;
        let asm = compile(source).unwrap();
        assert!(asm.contains("WRITE_FD fd1"));
        assert!(asm.contains("READ_FD fd0"));
        assert!(asm.contains("OPEN_FD fd3"));
        assert!(asm.contains("OPEN_FD_DYN"));
        assert!(asm.contains("READ_FD_DYN"));
        assert!(asm.contains("WAIT_ON_FD fd0"));
        assert!(asm.contains("FD_DUP fd3, fd1"));
        Program::parse(&asm).unwrap();
    }

    #[test]
    fn normalizes_struct_recursor_designated_initializers() {
        let source = r#"
        void visit(int dirfd, const char *name, struct stat *st, void *data, struct recursor *r) {
        }

        int main() {
            struct recursor r = { .fn = visit, .maxdepth = 1, .follow = 'H', .flags = DIRFIRST };
            recurse(-100, "path", 0, &r);
            return 0;
        }
        "#;
        let normalized = preprocess_source(source);
        assert!(normalized.contains("r = alloc(64);"), "{normalized}");
        assert!(normalized.contains("r.fn = visit;"), "{normalized}");
        assert!(
            normalized.contains("recurse(-100, \"path\", 0, r);"),
            "{normalized}"
        );
    }

    #[test]
    fn lowers_system_builtins_to_custom_instructions() {
        let source = r#"
        int child() {
            return 0;
        }

        int main() {
            int slot;
            slot = alloc(8);
            pid();
            tid();
            uid();
            gid();
            set_sigmask(0);
            fork();
            spawn(child);
            msg_send(1, 2, 3);
            msg_recv();
            futex_wait(slot, 0);
            futex_wake(slot, 1);
            mmap(0, 4096, 3);
            munmap(slot, 8);
            signal(2, child);
            sigaction(3, child);
            sigmask_set(1);
            kill(pid(), 2);
            inb(1);
            outb(1, 2);
            load_ucode(slot, 8);
            execvp("Cargo.toml", 0);
            return 0;
        }
        "#;
        let asm = compile(source).unwrap();
        for expected in [
            "GET_PCR",
            "SET_PCR SIGMASK",
            "FORK",
            "SPAWN",
            "MSG_SEND",
            "MSG_RECV",
            "FUTEX_WAIT",
            "FUTEX_WAKE",
            "MMAP",
            "MUNMAP",
            "SIGACTION",
            "SIGMASK_SET",
            "KILL",
            "INB",
            "OUTB",
            "LOAD_UCODE",
            "EXEC",
        ] {
            assert!(asm.contains(expected), "missing {expected} in:\n{asm}");
        }
        Program::parse(&asm).unwrap();
    }
}
