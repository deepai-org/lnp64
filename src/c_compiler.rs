use std::collections::{BTreeMap, HashMap};

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
    let tokens = Lexer::new(source).lex()?;
    let mut parser = Parser::new(tokens);
    let body = parser.parse_program()?;
    let mut codegen = CodeGen::default();
    codegen.emit_program(&body)
}

struct Lexer<'a> {
    chars: Vec<char>,
    pos: usize,
    source: &'a str,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            chars: source.chars().collect(),
            pos: 0,
            source,
        }
    }

    fn lex(mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        while let Some(ch) = self.peek() {
            match ch {
                c if c.is_whitespace() => {
                    self.pos += 1;
                }
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
                other => {
                    return Err(format!(
                        "unexpected character {other:?} at byte-like offset {} in source of {} bytes",
                        self.pos,
                        self.source.len()
                    ));
                }
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

    fn parse_program(&mut self) -> Result<Vec<Stmt>, String> {
        self.expect(Token::Int)?;
        self.expect_ident("main")?;
        self.expect(Token::LParen)?;
        self.expect(Token::RParen)?;
        self.parse_block()
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
            let rhs = self.parse_relational()?;
            expr = Expr::Binary(Box::new(expr), op, Box::new(rhs));
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
            let rhs = self.parse_additive()?;
            expr = Expr::Binary(Box::new(expr), op, Box::new(rhs));
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
            let rhs = self.parse_term()?;
            expr = Expr::Binary(Box::new(expr), op, Box::new(rhs));
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
            let rhs = self.parse_factor()?;
            expr = Expr::Binary(Box::new(expr), op, Box::new(rhs));
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

    fn expect_ident(&mut self, expected: &str) -> Result<(), String> {
        match self.peek() {
            Token::Ident(name) if name == expected => {
                self.advance();
                Ok(())
            }
            other => Err(format!("expected identifier {expected:?}, got {other:?}")),
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
    vars: HashMap<String, String>,
    temp_reg: usize,
    label_id: usize,
    string_id: usize,
}

impl CodeGen {
    fn emit_program(&mut self, body: &[Stmt]) -> Result<String, String> {
        self.text.push(".text".to_string());
        self.text.push("main:".to_string());
        for stmt in body {
            self.emit_stmt(stmt)?;
        }
        self.text.push("  EXIT r0".to_string());

        let mut out = String::new();
        if !self.data.is_empty() || !self.vars.is_empty() {
            out.push_str(".data\n");
            for (label, init) in &self.data {
                out.push_str(label);
                out.push_str(": ");
                out.push_str(init);
                out.push('\n');
            }
            let mut vars = self.vars.values().cloned().collect::<Vec<_>>();
            vars.sort();
            for label in vars {
                out.push_str(&label);
                out.push_str(": .quad 0\n");
            }
        }
        for line in &self.text {
            out.push_str(line);
            out.push('\n');
        }
        Ok(out)
    }

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::VarDecl(name) => {
                self.declare_var(name)?;
            }
            Stmt::Assign(name, expr) => {
                let reg = self.emit_expr(expr)?;
                let label = self.var_label(name)?;
                self.text.push(format!("  ST {label}, r{reg}"));
            }
            Stmt::Return(expr) => {
                let reg = self.emit_expr(expr)?;
                self.text.push(format!("  EXIT r{reg}"));
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
            Expr::Var(name) => {
                let label = self.var_label(name)?;
                let reg = self.alloc_reg()?;
                self.text.push(format!("  LD r{reg}, {label}"));
                Ok(reg)
            }
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
                if args.len() != 3 {
                    return Err("write(fd, buffer, len) expects 3 arguments".to_string());
                }
                let fd_num = match &args[0] {
                    Expr::Num(v) if (0..=255).contains(v) => *v as usize,
                    _ => return Err("write first argument must be a numeric fd".to_string()),
                };
                let buf = self.emit_expr(&args[1])?;
                let len = self.emit_expr(&args[2])?;
                self.text
                    .push(format!("  WRITE_FD fd{fd_num}, r{buf}, r{len}"));
                Ok(0)
            }
            "alloc" => {
                if args.len() != 1 {
                    return Err("alloc(bytes) expects 1 argument".to_string());
                }
                let len = self.emit_expr(&args[0])?;
                let dst = self.alloc_reg()?;
                self.text.push(format!("  ALLOC r{dst}, r{len}"));
                Ok(dst)
            }
            "free" => {
                if args.len() != 1 {
                    return Err("free(ptr) expects 1 argument".to_string());
                }
                let ptr = self.emit_expr(&args[0])?;
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
            "exit" => {
                if args.len() != 1 {
                    return Err("exit(code) expects 1 argument".to_string());
                }
                let code = self.emit_expr(&args[0])?;
                self.text.push(format!("  EXIT r{code}"));
                Ok(0)
            }
            _ => Err(format!("unsupported function call {name:?}")),
        }
    }

    fn declare_var(&mut self, name: &str) -> Result<(), String> {
        if self.vars.contains_key(name) {
            return Err(format!("duplicate variable {name:?}"));
        }
        let label = format!("var_{name}");
        self.vars.insert(name.to_string(), label);
        Ok(())
    }

    fn var_label(&self, name: &str) -> Result<String, String> {
        self.vars
            .get(name)
            .cloned()
            .ok_or_else(|| format!("unknown variable {name:?}"))
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
        let reg = 1 + (self.temp_reg % 27);
        self.temp_reg += 1;
        if reg == 31 {
            return Err("internal register allocator selected locked stack pointer".to_string());
        }
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
}
