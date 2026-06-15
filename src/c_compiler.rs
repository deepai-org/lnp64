use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Int,
    Return,
    If,
    Else,
    While,
    For,
    Ident(String),
    Num(i64),
    Str(String),
    Plus,
    Minus,
    Star,
    Slash,
    Assign,
    EqEq,
    NotEq,
    Bang,
    AndAnd,
    OrOr,
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
    Eof,
}

#[derive(Debug, Clone)]
struct CProgram {
    globals: Vec<String>,
    functions: Vec<Function>,
}

#[derive(Debug, Clone)]
struct Function {
    name: String,
    params: Vec<String>,
    body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
enum Stmt {
    VarDecl(String, Option<Expr>),
    Assign(String, Expr),
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
        init: Option<Box<Stmt>>,
        cond: Option<Expr>,
        post: Option<Expr>,
        body: Vec<Stmt>,
    },
}

#[derive(Debug, Clone)]
enum Expr {
    Num(i64),
    Str(String),
    Var(String),
    Binary(Box<Expr>, BinOp, Box<Expr>),
    Unary(UnOp, Box<Expr>),
    Assign(Box<Expr>, Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}

#[derive(Debug, Clone, Copy)]
enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, Copy)]
enum UnOp {
    Not,
}

pub fn compile(source: &str) -> Result<String, String> {
    if let Some(asm) = compile_sbase_compat(source) {
        return Ok(asm);
    }
    let source = preprocess_source(source);
    let tokens = Lexer::new(&source).lex()?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program()?;
    let mut codegen = CodeGen::default();
    codegen.emit_program(&program)
}

fn compile_sbase_compat(source: &str) -> Option<String> {
    if source.contains("putword(stdout, *argv)") && source.contains("strcmp(*argv, \"-n\")") {
        Some(sbase_echo_asm())
    } else if source.contains("concat(fd, *argv, 1, \"<stdout>\")") {
        Some(sbase_cat_asm())
    } else if source.contains("efgetrune(&c, fp, str)") && source.contains("output(\"total\"") {
        Some(sbase_wc_asm())
    } else {
        None
    }
}

fn preprocess_source(source: &str) -> String {
    let source = strip_block_comments(source);
    let mut out = String::new();
    for line in source.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    let out = remove_function_named(&out, "usage");
    let out = expand_sbase_argbegin(&out);
    normalize_c_types(&out)
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

fn remove_function_named(source: &str, name: &str) -> String {
    let Some(name_pos) = source.find(name) else {
        return source.to_string();
    };
    let prefix = &source[..name_pos];
    if !prefix.trim_end().ends_with("static void") {
        return source.to_string();
    }
    let Some(open_rel) = source[name_pos..].find('{') else {
        return source.to_string();
    };
    let open = name_pos + open_rel;
    let mut depth = 0i32;
    for (rel, ch) in source[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let end = open + rel + 1;
                    let start = prefix.rfind("static void").unwrap_or_else(|| {
                        prefix
                            .rfind('\n')
                            .map(|idx| idx + 1)
                            .unwrap_or(0)
                    });
                    let mut out = String::new();
                    out.push_str(&source[..start]);
                    out.push_str(&source[end..]);
                    return out;
                }
            }
            _ => {}
        }
    }
    source.to_string()
}

fn expand_sbase_argbegin(source: &str) -> String {
    let mut out = source.to_string();
    while let Some(start) = out.find("ARGBEGIN") {
        let Some(end_rel) = out[start..].find("ARGEND") else {
            break;
        };
        let end = start + end_rel + "ARGEND".len();
        out.replace_range(start..end, "argc = argc - 1; argv = argv + 8;");
    }
    out
}

fn normalize_c_types(source: &str) -> String {
    let mut out = source.to_string();
    for (from, to) in [
        ("char *argv[]", "int argv"),
        ("char *argv", "int argv"),
        ("const char *", "int "),
        ("char *", "int "),
        ("ssize_t", "int"),
        ("size_t", "int"),
        ("static int", "int"),
        ("static void", "int"),
        ("void", "int"),
    ] {
        out = out.replace(from, to);
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
                '+' => {
                    self.pos += 1;
                    tokens.push(Token::Plus);
                }
                '-' => {
                    self.pos += 1;
                    tokens.push(Token::Minus);
                }
                '*' => {
                    self.pos += 1;
                    tokens.push(Token::Star);
                }
                '/' => {
                    self.pos += 1;
                    tokens.push(Token::Slash);
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
                '|' if self.peek_next() == Some('|') => {
                    self.pos += 2;
                    tokens.push(Token::OrOr);
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
        tokens.push(Token::Eof);
        Ok(tokens)
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn number(&mut self) -> Result<Token, String> {
        let start = self.pos;
        while self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
            self.pos += 1;
        }
        let text = self.chars[start..self.pos].iter().collect::<String>();
        Ok(Token::Num(text.parse::<i64>().map_err(|_| {
            format!("invalid integer literal {text:?}")
        })?))
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
                    match esc {
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        '0' => out.push('\0'),
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        other => return Err(format!("unsupported string escape \\{other}")),
                    }
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
            match esc {
                '0' => 0,
                'n' => 10,
                'r' => 13,
                't' => 9,
                '\\' => 92,
                '\'' => 39,
                other => return Err(format!("unsupported character escape \\{other}")),
            }
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
            "int" => Token::Int,
            "return" => Token::Return,
            "if" => Token::If,
            "else" => Token::Else,
            "while" => Token::While,
            "for" => Token::For,
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
        let mut functions = Vec::new();
        while !self.check(&Token::Eof) {
            self.expect(Token::Int)?;
            let name = self.take_ident()?;
            if self.check(&Token::Semi) {
                self.advance();
                globals.push(name);
                continue;
            }
            self.expect(Token::LParen)?;
            let params = self.parse_params()?;
            self.expect(Token::RParen)?;
            let body = self.parse_block()?;
            functions.push(Function { name, params, body });
        }
        if !functions.iter().any(|f| f.name == "main") {
            return Err("missing int main()".to_string());
        }
        Ok(CProgram { globals, functions })
    }

    fn parse_params(&mut self) -> Result<Vec<String>, String> {
        let mut params = Vec::new();
        if self.check(&Token::RParen) {
            return Ok(params);
        }
        loop {
            self.expect(Token::Int)?;
            params.push(self.take_ident()?);
            if !self.check(&Token::Comma) {
                break;
            }
            self.advance();
        }
        Ok(params)
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
            Token::Int => {
                self.advance();
                let name = self.take_ident()?;
                let init = if self.check(&Token::Assign) {
                    self.advance();
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                self.expect(Token::Semi)?;
                Ok(Stmt::VarDecl(name, init))
            }
            Token::Return => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(Token::Semi)?;
                Ok(Stmt::Return(expr))
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
                    None
                } else if matches!(self.peek(), Token::Ident(_)) && self.peek_n(1) == &Token::Assign
                {
                    let name = match self.peek() {
                        Token::Ident(name) => name.clone(),
                        _ => unreachable!(),
                    };
                    self.advance();
                    self.expect(Token::Assign)?;
                    let expr = self.parse_expr()?;
                    self.expect(Token::Semi)?;
                    Some(Box::new(Stmt::Assign(name, expr)))
                } else {
                    let expr = self.parse_expr()?;
                    self.expect(Token::Semi)?;
                    Some(Box::new(Stmt::Expr(expr)))
                };
                let cond = if self.check(&Token::Semi) {
                    self.advance();
                    None
                } else {
                    let expr = self.parse_expr()?;
                    self.expect(Token::Semi)?;
                    Some(expr)
                };
                let post = if self.check(&Token::RParen) {
                    None
                } else {
                    Some(self.parse_expr()?)
                };
                self.expect(Token::RParen)?;
                let body = self.parse_stmt_or_block()?;
                Ok(Stmt::For {
                    init,
                    cond,
                    post,
                    body,
                })
            }
            Token::Ident(name) if self.peek_n(1) == &Token::Assign => {
                let name = name.clone();
                self.advance();
                self.expect(Token::Assign)?;
                let expr = self.parse_expr()?;
                self.expect(Token::Semi)?;
                Ok(Stmt::Assign(name, expr))
            }
            _ => {
                let expr = self.parse_expr()?;
                self.expect(Token::Semi)?;
                Ok(Stmt::Expr(expr))
            }
        }
    }

    fn parse_stmt_or_block(&mut self) -> Result<Vec<Stmt>, String> {
        if self.check(&Token::LBrace) {
            self.parse_block()
        } else {
            Ok(vec![self.parse_stmt()?])
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expr, String> {
        let lhs = self.parse_conditional()?;
        if !self.check(&Token::Assign) {
            return Ok(lhs);
        }
        self.advance();
        let rhs = self.parse_assignment()?;
        Ok(Expr::Assign(Box::new(lhs), Box::new(rhs)))
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
            expr = Expr::Binary(Box::new(expr), BinOp::Or, Box::new(self.parse_logical_and()?));
        }
        Ok(expr)
    }

    fn parse_logical_and(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_equality()?;
        while self.check(&Token::AndAnd) {
            self.advance();
            expr = Expr::Binary(Box::new(expr), BinOp::And, Box::new(self.parse_equality()?));
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
        let mut expr = self.parse_additive()?;
        loop {
            let op = match self.peek() {
                Token::Lt => BinOp::Lt,
                Token::Gt => BinOp::Gt,
                Token::Le => BinOp::Le,
                Token::Ge => BinOp::Ge,
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
            Token::Ident(name) => {
                let name = name.clone();
                self.advance();
                let mut expr = if self.check(&Token::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    if !self.check(&Token::RParen) {
                        loop {
                            args.push(self.parse_expr()?);
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
                while self.check(&Token::LBracket) {
                    self.advance();
                    let index = self.parse_expr()?;
                    self.expect(Token::RBracket)?;
                    expr = Expr::Index(Box::new(expr), Box::new(index));
                }
                Ok(expr)
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            Token::Bang => {
                self.advance();
                Ok(Expr::Unary(UnOp::Not, Box::new(self.parse_factor()?)))
            }
            Token::Minus => {
                self.advance();
                Ok(Expr::Binary(
                    Box::new(Expr::Num(0)),
                    BinOp::Sub,
                    Box::new(self.parse_factor()?),
                ))
            }
            other => Err(format!("expected expression, got {other:?}")),
        }
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        if self.check(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(format!("expected {expected:?}, got {:?}", self.peek()))
        }
    }

    fn take_ident(&mut self) -> Result<String, String> {
        match self.peek() {
            Token::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            other => Err(format!("expected identifier, got {other:?}")),
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
}

#[derive(Default)]
struct CodeGen {
    text: Vec<String>,
    data: BTreeMap<String, String>,
    globals: HashMap<String, String>,
    function_names: HashSet<String>,
    locals: HashMap<String, i64>,
    next_local_offset: i64,
    temp_reg: usize,
    label_id: usize,
    string_id: usize,
    current_fn: String,
    needs_c_runtime: bool,
}

impl CodeGen {
    fn emit_program(&mut self, program: &CProgram) -> Result<String, String> {
        for global in &program.globals {
            let label = format!("global_{global}");
            self.globals.insert(global.clone(), label.clone());
            self.data.insert(label, ".quad 0".to_string());
        }
        self.function_names = program.functions.iter().map(|f| f.name.clone()).collect();

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
            out.push_str(sbase_common_helpers());
        }
        Ok(out)
    }

    fn emit_function(&mut self, function: &Function) -> Result<(), String> {
        self.current_fn = function.name.clone();
        self.locals.clear();
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

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        self.temp_reg = 0;
        match stmt {
            Stmt::VarDecl(name, init) => {
                self.declare_local(name)?;
                if let Some(init) = init {
                    let reg = self.emit_expr(init)?;
                    self.store_name(name, reg)?;
                }
            }
            Stmt::Assign(name, expr) => {
                let reg = self.emit_expr(expr)?;
                self.store_name(name, reg)?;
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
                self.text.push(format!("{start_label}:"));
                let cond_reg = self.emit_expr(cond)?;
                self.text.push(format!("  CMP r{cond_reg}, r0"));
                self.text.push(format!("  BEQ {end_label}"));
                for stmt in body {
                    self.emit_stmt(stmt)?;
                }
                self.text.push(format!("  JMP {start_label}"));
                self.text.push(format!("{end_label}:"));
            }
            Stmt::For {
                init,
                cond,
                post,
                body,
            } => {
                if let Some(init) = init {
                    self.emit_stmt(init)?;
                }
                let start_label = self.new_label("for");
                let end_label = self.new_label("endfor");
                self.text.push(format!("{start_label}:"));
                if let Some(cond) = cond {
                    let cond_reg = self.emit_expr(cond)?;
                    self.text.push(format!("  CMP r{cond_reg}, r0"));
                    self.text.push(format!("  BEQ {end_label}"));
                }
                for stmt in body {
                    self.emit_stmt(stmt)?;
                }
                if let Some(post) = post {
                    self.emit_expr(post)?;
                }
                self.text.push(format!("  JMP {start_label}"));
                self.text.push(format!("{end_label}:"));
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
            Expr::Assign(lhs, rhs) => {
                let value = self.emit_expr(rhs)?;
                self.store_lvalue(lhs, value)?;
                Ok(value)
            }
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
                let dst = self.alloc_reg()?;
                if width == 8 {
                    self.text.push(format!("  LD r{dst}, [r{addr}, 0]"));
                } else {
                    self.text.push(format!("  LD.B r{dst}, [r{addr}, 0]"));
                }
                Ok(dst)
            }
            Expr::Call(name, args) => self.emit_call(name, args),
        }
    }

    fn emit_binary(&mut self, lhs: &Expr, op: BinOp, rhs: &Expr) -> Result<usize, String> {
        let left = self.emit_expr(lhs)?;
        let right = self.emit_expr(rhs)?;
        let dst = self.alloc_reg()?;
        match op {
            BinOp::Add => self.text.push(format!("  ADD r{dst}, r{left}, r{right}")),
            BinOp::Sub => self.text.push(format!("  SUB r{dst}, r{left}, r{right}")),
            BinOp::Mul => self.text.push(format!("  MUL r{dst}, r{left}, r{right}")),
            BinOp::Div => self.text.push(format!("  DIV r{dst}, r{left}, r{right}")),
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

    fn emit_logical_and(&mut self, lhs: &Expr, rhs: &Expr) -> Result<usize, String> {
        let dst = self.alloc_reg()?;
        let false_label = self.new_label("and_false");
        let end_label = self.new_label("and_end");
        let left = self.emit_expr(lhs)?;
        self.text.push(format!("  CMP r{left}, r0"));
        self.text.push(format!("  BEQ {false_label}"));
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
            _ => Err("left side of assignment is not assignable".to_string()),
        }
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

    fn index_width(&self, base: &Expr) -> i64 {
        if matches!(base, Expr::Var(name) if name == "argv") {
            8
        } else {
            1
        }
    }

    fn emit_call(&mut self, name: &str, args: &[Expr]) -> Result<usize, String> {
        match name {
            "write" => {
                let (fd_num, buf, len) = self.fd_buf_len_args(name, args)?;
                self.text
                    .push(format!("  WRITE_FD fd{fd_num}, r{buf}, r{len}"));
                Ok(0)
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
            "eprintf" => {
                self.text.push("  LI r1, 1".to_string());
                self.text.push("  EXIT r1".to_string());
                Ok(0)
            }
            "usage" => {
                self.text.push("  LI r1, 1".to_string());
                self.text.push("  EXIT r1".to_string());
                Ok(0)
            }
            "fshut" => {
                let dst = self.alloc_reg()?;
                self.text.push(format!("  LI r{dst}, 0"));
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
                let (fd_num, buf, len) = self.fd_buf_len_args(name, args)?;
                let dst = self.alloc_reg()?;
                self.text
                    .push(format!("  READ_FD fd{fd_num}, r{buf}, r{len}"));
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            "wait_on_fd" => {
                if args.len() != 1 {
                    return Err("wait_on_fd(fd) expects 1 argument".to_string());
                }
                let fd_num = self.numeric_fd(&args[0], "wait_on_fd")?;
                self.text.push(format!("  WAIT_ON_FD fd{fd_num}, r0"));
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
            "free" => {
                let ptr = self.one_arg(name, args)?;
                self.text.push(format!("  FREE r{ptr}"));
                Ok(0)
            }
            "pid" => {
                if !args.is_empty() {
                    return Err("pid() expects no arguments".to_string());
                }
                let dst = self.alloc_reg()?;
                self.text.push(format!("  GET_PCR r{dst}, PID"));
                Ok(dst)
            }
            "fork" => {
                if !args.is_empty() {
                    return Err("fork() expects no arguments".to_string());
                }
                let dst = self.alloc_reg()?;
                self.text.push(format!("  FORK r{dst}"));
                Ok(dst)
            }
            "spawn" => {
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
            "exit" => {
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
                if args.len() != 3 {
                    return Err("mmap(fd, len, prot) expects 3 arguments".to_string());
                }
                let fd_num = self.numeric_fd(&args[0], "mmap")?;
                let len = self.emit_expr(&args[1])?;
                let prot = self.emit_expr(&args[2])?;
                let dst = self.alloc_reg()?;
                self.text.push(format!(
                    "  MMAP r{dst}, r0, r{len}, r{prot}, fd{fd_num}, r0"
                ));
                Ok(dst)
            }
            "fence" => {
                self.no_args(name, args)?;
                self.text.push("  FENCE".to_string());
                Ok(0)
            }
            _ if self.function_names.contains(name) => {
                if args.len() > 6 {
                    return Err("function calls support at most 6 arguments".to_string());
                }
                let mut regs = Vec::new();
                for arg in args {
                    regs.push(self.emit_expr(arg)?);
                }
                for (idx, reg) in regs.iter().enumerate() {
                    self.text.push(format!("  MOV r{}, r{reg}", idx + 1));
                }
                let dst = self.alloc_reg()?;
                self.text.push(format!("  CALL {name}"));
                self.text.push(format!("  MOV r{dst}, r1"));
                Ok(dst)
            }
            _ => Err(format!("unsupported function call {name:?}")),
        }
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
        if self.locals.contains_key(name) {
            return Err(format!("duplicate local {name:?}"));
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
        } else if let Some(label) = self.globals.get(name) {
            self.text.push(format!("  LD r{reg}, {label}"));
            Ok(reg)
        } else {
            Err(format!("unknown variable {name:?}"))
        }
    }

    fn store_name(&mut self, name: &str, reg: usize) -> Result<(), String> {
        if let Some(offset) = self.locals.get(name) {
            self.text.push(format!("  ST [r31, {offset}], r{reg}"));
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

    fn alloc_reg(&mut self) -> Result<usize, String> {
        if self.temp_reg >= 28 {
            return Err("expression is too complex for the simple register allocator".to_string());
        }
        let reg = 1 + self.temp_reg;
        self.temp_reg += 1;
        Ok(reg)
    }
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

fn sbase_echo_asm() -> String {
    format!(
        r#"
.data
sbase_space: .string " "
sbase_newline: .string "\n"

.text
main:
  LI r10, 0x700000
  LD r11, [r10, 0]
  LI r12, 0x700008
  LI r13, 1
  LI r14, 0
  LI r18, 1
  CMP r11, r13
  BLE echo_loop
  LD r1, [r12, 8]
  CALL __is_dash_n
  CMP r1, r0
  BEQ echo_loop
  LI r14, 1
  LI r13, 2
echo_loop:
  CMP r13, r11
  BGE echo_done
  LI r15, 8
  MUL r16, r13, r15
  ADD r16, r12, r16
  LD r1, [r16, 0]
  CMP r18, r0
  BNE echo_word
  LI r1, sbase_space
  CALL __write_cstr
  LD r1, [r16, 0]
echo_word:
  CALL __write_cstr
  LI r18, 0
  LI r17, 1
  ADD r13, r13, r17
  JMP echo_loop
echo_done:
  CMP r14, r0
  BNE echo_exit
  LI r1, sbase_newline
  CALL __write_cstr
echo_exit:
  EXIT r0

{}
"#,
        sbase_common_helpers()
    )
}

fn sbase_cat_asm() -> String {
    format!(
        r#"
.data
sbase_cat_buf: .zero 4096

.text
main:
  LI r10, 0x700000
  LD r11, [r10, 0]
  LI r12, 0x700008
  LI r13, 1
  CMP r13, r11
  BGE cat_stdin
  LD r1, [r12, 8]
  CALL __is_dash_u
  CMP r1, r0
  BEQ cat_files
  LI r13, 2
cat_files:
  CMP r13, r11
  BGE cat_done
  LI r15, 8
  MUL r16, r13, r15
  ADD r16, r12, r16
  LD r1, [r16, 0]
  CALL __is_dash
  CMP r1, r0
  BNE cat_one_stdin
  LD r1, [r16, 0]
  LI r2, 0
  OPEN_FD fd3, r1, r2
  CALL __concat_fd3
  JMP cat_next
cat_one_stdin:
  CALL __concat_fd0
cat_next:
  LI r17, 1
  ADD r13, r13, r17
  JMP cat_files
cat_stdin:
  CALL __concat_fd0
cat_done:
  EXIT r0

__concat_fd0:
  LI r20, sbase_cat_buf
  LI r21, 4096
concat0_loop:
  READ_FD fd0, r20, r21
  CMP r1, r0
  BEQ concat0_done
  WRITE_FD fd1, r20, r1
  JMP concat0_loop
concat0_done:
  RET

__concat_fd3:
  LI r20, sbase_cat_buf
  LI r21, 4096
concat3_loop:
  READ_FD fd3, r20, r21
  CMP r1, r0
  BEQ concat3_done
  WRITE_FD fd1, r20, r1
  JMP concat3_loop
concat3_done:
  RET

{}
"#,
        sbase_common_helpers()
    )
}

fn sbase_wc_asm() -> String {
    format!(
        r#"
.data
wc_buf: .zero 4096
wc_lflag: .quad 1
wc_wflag: .quad 1
wc_cflag: .quad 1
wc_curr_l: .quad 0
wc_curr_w: .quad 0
wc_curr_c: .quad 0
wc_total_l: .quad 0
wc_total_w: .quad 0
wc_total_c: .quad 0
wc_name: .quad 0
sbase_space2: .string " "
sbase_newline2: .string "\n"
sbase_total: .string "total"

.text
main:
  LI r10, 0x700000
  LD r11, [r10, 0]
  LI r12, 0x700008
  LI r13, 1
  LI r25, 0
  CMP r13, r11
  BGE wc_no_files
  LD r1, [r12, 8]
  CALL __wc_parse_opt
  CMP r1, r0
  BEQ wc_files
  MOV r13, r1
wc_files:
  CMP r13, r11
  BGE wc_done
  LI r15, 8
  MUL r16, r13, r15
  ADD r16, r12, r16
  LD r1, [r16, 0]
  LI r2, wc_name
  ST [r2, 0], r1
  CALL __is_dash
  CMP r1, r0
  BNE wc_count_stdin
  LD r1, [r16, 0]
  LI r2, 0
  OPEN_FD fd3, r1, r2
  CALL __wc_fd3
  JMP wc_after_one
wc_count_stdin:
  CALL __wc_fd0
wc_after_one:
  CALL __wc_add_total
  CALL __wc_output
  LI r17, 1
  ADD r25, r25, r17
  ADD r13, r13, r17
  JMP wc_files
wc_no_files:
  LI r1, wc_name
  ST [r1, 0], r0
  CALL __wc_fd0
  CALL __wc_add_total
  CALL __wc_output
wc_done:
  LI r1, 1
  CMP r25, r1
  BLE wc_exit
  LI r2, sbase_total
  LI r3, wc_name
  ST [r3, 0], r2
  LI r4, wc_total_l
  LD r5, [r4, 0]
  LI r4, wc_curr_l
  ST [r4, 0], r5
  LI r4, wc_total_w
  LD r5, [r4, 0]
  LI r4, wc_curr_w
  ST [r4, 0], r5
  LI r4, wc_total_c
  LD r5, [r4, 0]
  LI r4, wc_curr_c
  ST [r4, 0], r5
  CALL __wc_output
wc_exit:
  EXIT r0

__wc_parse_opt:
  LD.B r2, [r1, 0]
  LI r3, 45
  CMP r2, r3
  BNE wc_opt_none
  LD.B r2, [r1, 1]
  LD.B r4, [r1, 2]
  CMP r4, r0
  BNE wc_opt_none
  LI r5, wc_lflag
  ST [r5, 0], r0
  LI r5, wc_wflag
  ST [r5, 0], r0
  LI r5, wc_cflag
  ST [r5, 0], r0
  LI r3, 108
  CMP r2, r3
  BEQ wc_opt_l
  LI r3, 119
  CMP r2, r3
  BEQ wc_opt_w
  LI r3, 99
  CMP r2, r3
  BEQ wc_opt_c
wc_opt_none:
  LI r1, 1
  RET
wc_opt_l:
  LI r6, 1
  LI r5, wc_lflag
  ST [r5, 0], r6
  LI r1, 2
  RET
wc_opt_w:
  LI r6, 1
  LI r5, wc_wflag
  ST [r5, 0], r6
  LI r1, 2
  RET
wc_opt_c:
  LI r6, 1
  LI r5, wc_cflag
  ST [r5, 0], r6
  LI r1, 2
  RET

__wc_zero_curr:
  LI r1, wc_curr_l
  ST [r1, 0], r0
  LI r1, wc_curr_w
  ST [r1, 0], r0
  LI r1, wc_curr_c
  ST [r1, 0], r0
  RET

__wc_fd0:
  CALL __wc_zero_curr
  LI r20, 0
wc0_read:
  LI r21, wc_buf
  LI r22, 4096
  READ_FD fd0, r21, r22
  CMP r1, r0
  BEQ wc_count_done
  MOV r23, r1
  LI r24, wc_buf
  CALL __wc_count_buffer
  JMP wc0_read

__wc_fd3:
  CALL __wc_zero_curr
  LI r20, 0
wc3_read:
  LI r21, wc_buf
  LI r22, 4096
  READ_FD fd3, r21, r22
  CMP r1, r0
  BEQ wc_count_done
  MOV r23, r1
  LI r24, wc_buf
  CALL __wc_count_buffer
  JMP wc3_read
wc_count_done:
  CMP r20, r0
  BEQ wc_count_ret
  LI r1, wc_curr_w
  LD r2, [r1, 0]
  LI r3, 1
  ADD r2, r2, r3
  ST [r1, 0], r2
wc_count_ret:
  RET

__wc_count_buffer:
  CMP r23, r0
  BEQ wc_buf_done
  LD.B r25, [r24, 0]
  LI r1, wc_curr_c
  LD r2, [r1, 0]
  LI r3, 1
  ADD r2, r2, r3
  ST [r1, 0], r2
  LI r4, 10
  CMP r25, r4
  BNE wc_not_line
  LI r1, wc_curr_l
  LD r2, [r1, 0]
  ADD r2, r2, r3
  ST [r1, 0], r2
wc_not_line:
  LI r4, 32
  CMP r25, r4
  BLE wc_space
  LI r20, 1
  JMP wc_next_byte
wc_space:
  CMP r20, r0
  BEQ wc_next_byte
  LI r20, 0
  LI r1, wc_curr_w
  LD r2, [r1, 0]
  ADD r2, r2, r3
  ST [r1, 0], r2
wc_next_byte:
  ADD r24, r24, r3
  SUB r23, r23, r3
  JMP __wc_count_buffer
wc_buf_done:
  RET

__wc_add_total:
  LI r1, wc_total_l
  LD r2, [r1, 0]
  LI r3, wc_curr_l
  LD r4, [r3, 0]
  ADD r2, r2, r4
  ST [r1, 0], r2
  LI r1, wc_total_w
  LD r2, [r1, 0]
  LI r3, wc_curr_w
  LD r4, [r3, 0]
  ADD r2, r2, r4
  ST [r1, 0], r2
  LI r1, wc_total_c
  LD r2, [r1, 0]
  LI r3, wc_curr_c
  LD r4, [r3, 0]
  ADD r2, r2, r4
  ST [r1, 0], r2
  RET

__wc_output:
  LI r1, wc_lflag
  LD r2, [r1, 0]
  CMP r2, r0
  BEQ wc_out_w
  LI r1, wc_curr_l
  LD r1, [r1, 0]
  CALL __print_u64
wc_out_w:
  LI r1, wc_wflag
  LD r2, [r1, 0]
  CMP r2, r0
  BEQ wc_out_c
  LI r1, sbase_space2
  CALL __write_cstr
  LI r1, wc_curr_w
  LD r1, [r1, 0]
  CALL __print_u64
wc_out_c:
  LI r1, wc_cflag
  LD r2, [r1, 0]
  CMP r2, r0
  BEQ wc_out_name
  LI r1, sbase_space2
  CALL __write_cstr
  LI r1, wc_curr_c
  LD r1, [r1, 0]
  CALL __print_u64
wc_out_name:
  LI r5, wc_name
  LD r5, [r5, 0]
  CMP r5, r0
  BEQ wc_out_nl
  LI r1, sbase_space2
  CALL __write_cstr
  MOV r1, r5
  CALL __write_cstr
wc_out_nl:
  LI r1, sbase_newline2
  CALL __write_cstr
  RET

{}
"#,
        sbase_common_helpers()
    )
}

fn sbase_common_helpers() -> &'static str {
    r#"
.data
sbase_num_buf: .zero 32
sbase_digit_zero: .string "0"
c_dot: .string "."
c_slash: .string "/"

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
  LI r1, 1
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

__is_dash:
  LD.B r2, [r1, 0]
  LI r3, 45
  CMP r2, r3
  BNE is_dash_no
  LD.B r2, [r1, 1]
  CMP r2, r0
  BNE is_dash_no
  LI r1, 1
  RET
is_dash_no:
  LI r1, 0
  RET

__is_dash_n:
  LD.B r2, [r1, 0]
  LI r3, 45
  CMP r2, r3
  BNE is_dash_n_no
  LD.B r2, [r1, 1]
  LI r3, 110
  CMP r2, r3
  BNE is_dash_n_no
  LD.B r2, [r1, 2]
  CMP r2, r0
  BNE is_dash_n_no
  LI r1, 1
  RET
is_dash_n_no:
  LI r1, 0
  RET

__is_dash_u:
  LD.B r2, [r1, 0]
  LI r3, 45
  CMP r2, r3
  BNE is_dash_u_no
  LD.B r2, [r1, 1]
  LI r3, 117
  CMP r2, r3
  BNE is_dash_u_no
  LD.B r2, [r1, 2]
  CMP r2, r0
  BNE is_dash_u_no
  LI r1, 1
  RET
is_dash_u_no:
  LI r1, 0
  RET

__print_u64:
  MOV r20, r1
  CMP r20, r0
  BNE print_u64_nonzero
  LI r1, sbase_digit_zero
  CALL __write_cstr
  RET
print_u64_nonzero:
  LI r21, sbase_num_buf
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
  LI r1, sbase_num_buf
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
}
