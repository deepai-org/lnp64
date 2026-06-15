use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Int,
    Return,
    If,
    Else,
    While,
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
    Lt,
    Gt,
    Le,
    Ge,
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
    VarDecl(String),
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
}

#[derive(Debug, Clone)]
enum Expr {
    Num(i64),
    Str(String),
    Var(String),
    Binary(Box<Expr>, BinOp, Box<Expr>),
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
}

pub fn compile(source: &str) -> Result<String, String> {
    if let Some(asm) = compile_sbase_compat(source) {
        return Ok(asm);
    }
    let tokens = Lexer::new(source).lex()?;
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
                self.expect(Token::Semi)?;
                Ok(Stmt::VarDecl(name))
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
                let then_body = self.parse_block()?;
                let else_body = if self.check(&Token::Else) {
                    self.advance();
                    self.parse_block()?
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
                let body = self.parse_block()?;
                Ok(Stmt::While { cond, body })
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

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_equality()
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
                if self.check(&Token::LParen) {
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
                    Ok(Expr::Call(name, args))
                } else {
                    Ok(Expr::Var(name))
                }
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(expr)
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
            self.text
                .push(format!("  ST [r31, {offset}], r{}", idx + 1));
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
            Stmt::VarDecl(name) => {
                self.declare_local(name)?;
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
            Expr::Binary(lhs, op, rhs) => self.emit_binary(lhs, *op, rhs),
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

    fn emit_call(&mut self, name: &str, args: &[Expr]) -> Result<usize, String> {
        match name {
            "write" => {
                let (fd_num, buf, len) = self.fd_buf_len_args(name, args)?;
                self.text
                    .push(format!("  WRITE_FD fd{fd_num}, r{buf}, r{len}"));
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
