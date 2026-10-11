// SPDX-License-Identifier: GPL-3.0-or-later
//! Expresso expression finder + evaluator.
//!
//! A line-for-line port of `linux/extension/expresso@bulrok/calc.js`. Both are
//! held to the same behaviour by `spec/cases.json`; change the spec first,
//! then both implementations.
//!
//! [`find_and_evaluate`] finds the longest math expression ending at the caret
//! (optionally followed by `=`) and returns it with its value and display
//! string, or `None` when there is nothing worth showing.

use std::f64::consts::{E, PI, TAU};

const MAX_LOOKBACK: usize = 300;

#[derive(Debug, Clone, PartialEq)]
pub struct Evaluation {
    pub expr: String,
    pub value: f64,
    pub display: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Percent,
    Mod,
}

#[derive(Debug, Clone, Copy)]
enum Kind {
    Num(f64),
    Const(f64),
    Fn(fn(f64) -> f64),
    Op(Op),
    LParen,
    RParen,
    Bang,
    Word,
}

#[derive(Debug, Clone, Copy)]
struct Tok {
    kind: Kind,
    start: usize, // char index into the input
}

fn is_value_start(t: Option<&Tok>) -> bool {
    matches!(
        t.map(|t| t.kind),
        Some(Kind::Num(_) | Kind::Const(_) | Kind::Fn(_) | Kind::LParen)
    )
}

fn is_value_end(t: Option<&Tok>) -> bool {
    matches!(
        t.map(|t| t.kind),
        Some(Kind::Num(_) | Kind::Const(_) | Kind::RParen | Kind::Bang)
    )
}

// JS Math.round: halves round towards +infinity.
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

fn lookup_fn(name: &str) -> Option<fn(f64) -> f64> {
    Some(match name {
        "sqrt" => f64::sqrt,
        "cbrt" => f64::cbrt,
        "abs" => f64::abs,
        "exp" => f64::exp,
        "sin" => f64::sin,
        "cos" => f64::cos,
        "tan" => f64::tan,
        "asin" => f64::asin,
        "acos" => f64::acos,
        "atan" => f64::atan,
        "sinh" => f64::sinh,
        "cosh" => f64::cosh,
        "tanh" => f64::tanh,
        "ln" => f64::ln,
        "log" | "log10" => f64::log10,
        "log2" => f64::log2,
        "round" => js_round,
        "floor" => f64::floor,
        "ceil" => f64::ceil,
        _ => return None,
    })
}

fn lookup_const(name: &str) -> Option<f64> {
    match name {
        "pi" | "π" => Some(PI),
        "e" => Some(E),
        "tau" | "τ" => Some(TAU),
        _ => None,
    }
}

fn symbol(c: char) -> Option<Kind> {
    Some(match c {
        '+' => Kind::Op(Op::Add),
        '-' | '−' | '–' => Kind::Op(Op::Sub),
        '*' | '×' | '·' | '∙' => Kind::Op(Op::Mul),
        '/' | '÷' => Kind::Op(Op::Div),
        '^' => Kind::Op(Op::Pow),
        '%' => Kind::Op(Op::Percent),
        '(' | '[' => Kind::LParen,
        ')' | ']' => Kind::RParen,
        '!' => Kind::Bang,
        '√' => Kind::Fn(f64::sqrt),
        _ => return None,
    })
}

// ---- number scanning (mirrors the three regexes in calc.js) ----------------

fn digit_run(s: &[char], i: usize) -> usize {
    s[i.min(s.len())..]
        .iter()
        .take_while(|c| c.is_ascii_digit())
        .count()
}

/// `[eE][+-]?\d+` at `i`, all or nothing.
fn exponent(s: &[char], i: usize) -> usize {
    if !matches!(s.get(i), Some('e' | 'E')) {
        return 0;
    }
    let sign = usize::from(matches!(s.get(i + 1), Some('+' | '-')));
    let d = digit_run(s, i + 1 + sign);
    if d == 0 {
        0
    } else {
        1 + sign + d
    }
}

/// `^\d{1,3}(?:,\d{3})+(?![\d,])(?:\.\d+)?` — grouping commas.
fn num_grouped(s: &[char], i: usize) -> usize {
    let lead = digit_run(s, i);
    if !(1..=3).contains(&lead) {
        return 0;
    }
    let mut j = i + lead;
    let mut groups = 0;
    while s.get(j) == Some(&',') && digit_run(s, j + 1) >= 3 {
        j += 4;
        groups += 1;
    }
    if groups == 0 || matches!(s.get(j), Some(c) if c.is_ascii_digit() || *c == ',') {
        return 0;
    }
    if s.get(j) == Some(&'.') {
        let f = digit_run(s, j + 1);
        if f > 0 {
            j += 1 + f;
        }
    }
    j - i
}

/// `^\d+,\d+(?:[eE][+-]?\d+)?` — decimal comma.
fn num_deccomma(s: &[char], i: usize) -> usize {
    let a = digit_run(s, i);
    if a == 0 || s.get(i + a) != Some(&',') {
        return 0;
    }
    let b = digit_run(s, i + a + 1);
    if b == 0 {
        return 0;
    }
    let j = i + a + 1 + b;
    j + exponent(s, j) - i
}

/// `^(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?` — plain.
fn num_plain(s: &[char], i: usize) -> usize {
    let a = digit_run(s, i);
    let mut j;
    if a > 0 {
        j = i + a;
        if s.get(j) == Some(&'.') {
            j += 1 + digit_run(s, j + 1);
        }
    } else if s.get(i) == Some(&'.') && digit_run(s, i + 1) > 0 {
        j = i + 1 + digit_run(s, i + 1);
    } else {
        return 0;
    }
    j + exponent(s, j) - i
}

fn parse_num(s: &str) -> f64 {
    s.parse::<f64>().unwrap_or(f64::NAN)
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Token kinds: numbers, operators, parens, `!`, functions, constants, and
/// `Word` for anything unknown (an expression never spans one).
fn tokenize(s: &[char]) -> (Vec<Tok>, bool) {
    let mut toks: Vec<Tok> = Vec::new();
    let mut used_comma = false;
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        let collect = |n: usize, f: fn(char) -> Option<char>| -> String {
            s[i..i + n].iter().filter_map(|&c| f(c)).collect()
        };
        let (kind, len) = if let n @ 1.. = num_grouped(s, i) {
            (
                Kind::Num(parse_num(&collect(n, |c| (c != ',').then_some(c)))),
                n,
            )
        } else if let n @ 1.. = num_deccomma(s, i) {
            used_comma = true;
            (
                Kind::Num(parse_num(&collect(n, |c| {
                    Some(if c == ',' { '.' } else { c })
                }))),
                n,
            )
        } else if let n @ 1.. = num_plain(s, i) {
            (Kind::Num(parse_num(&collect(n, Some))), n)
        } else if (c == 'x' || c == 'X')
            && is_value_end(toks.last())
            && !s.get(i + 1).is_some_and(|&n| is_ident_start(n))
        {
            (Kind::Op(Op::Mul), 1) // 3x4, 3 x 4
        } else if is_ident_start(c) || c == 'π' || c == 'τ' {
            let n = if is_ident_start(c) {
                1 + s[i + 1..].iter().take_while(|&&c| is_ident_char(c)).count()
            } else {
                1
            };
            let lw = collect(n, Some).to_lowercase();
            let kind = if lw == "mod" {
                Kind::Op(Op::Mod)
            } else if let Some(f) = lookup_fn(&lw) {
                Kind::Fn(f)
            } else if let Some(v) = lookup_const(&lw) {
                Kind::Const(v)
            } else {
                Kind::Word
            };
            (kind, n)
        } else if c == '*' && s.get(i + 1) == Some(&'*') {
            (Kind::Op(Op::Pow), 2)
        } else {
            (symbol(c).unwrap_or(Kind::Word), 1)
        };
        toks.push(Tok { kind, start: i });
        i += len;
    }

    // "a % b" is modulo; "15%" followed by nothing / an operator is percent.
    for j in 0..toks.len() {
        if matches!(toks[j].kind, Kind::Op(Op::Percent)) && is_value_start(toks.get(j + 1)) {
            toks[j].kind = Kind::Op(Op::Mod);
        }
    }
    (toks, used_comma)
}

// ---- parser (recursive descent, same grammar as calc.js) --------------------
//   expr    := term (('+'|'-') term)*        a ± b%  means  a * (1 ± b/100)
//   term    := unary (('*'|'/'|mod) unary | <implicit> unary)*
//   unary   := ('-'|'+') unary | power
//   power   := postfix ('^' unary)?            right-assoc, so -2^2 = -4
//   postfix := atom ('!' | '%')*
//   atom    := num | const | fn atom | fn '(' expr ')' | '(' expr ')'

struct ParseError;
type PResult<T> = Result<T, ParseError>;

/// A value plus whether it ended in a bare `%` (matters only right of + / -).
#[derive(Clone, Copy)]
struct Val {
    v: f64,
    pct: bool,
}

struct Parser<'a> {
    t: &'a [Tok],
    i: usize,
    ops: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<Kind> {
        self.t.get(self.i).map(|t| t.kind)
    }

    fn peek_op(&self, op: Op) -> bool {
        matches!(self.peek(), Some(Kind::Op(o)) if o == op)
    }

    fn parse(&mut self) -> PResult<f64> {
        let v = self.expr()?.v;
        if self.i != self.t.len() {
            return Err(ParseError);
        }
        Ok(v)
    }

    fn close_paren(&mut self) -> PResult<()> {
        if self.i == self.t.len() {
            return Ok(()); // unclosed at the caret is fine while typing
        }
        match self.peek() {
            Some(Kind::RParen) => {
                self.i += 1;
                Ok(())
            }
            _ => Err(ParseError),
        }
    }

    fn expr(&mut self) -> PResult<Val> {
        let mut v = self.term()?.v;
        while self.peek_op(Op::Add) || self.peek_op(Op::Sub) {
            let add = self.peek_op(Op::Add);
            self.i += 1;
            let r = self.term()?;
            self.ops += 1;
            let rv = if r.pct { v * r.v } else { r.v };
            v = if add { v + rv } else { v - rv };
        }
        Ok(Val { v, pct: false })
    }

    fn term(&mut self) -> PResult<Val> {
        let mut r = self.unary()?;
        loop {
            let op = match self.peek() {
                Some(Kind::Op(o @ (Op::Mul | Op::Div | Op::Mod))) => {
                    self.i += 1;
                    o
                }
                // implicit: 2pi, 3(1+1); never "1.2 .3"
                Some(Kind::Const(_) | Kind::Fn(_) | Kind::LParen) => Op::Mul,
                _ => return Ok(r),
            };
            let b = self.unary()?.v;
            self.ops += 1;
            let a = r.v;
            let v = match op {
                Op::Mul => a * b,
                Op::Div => a / b,
                _ => ((a % b) + b) % b,
            };
            r = Val { v, pct: false };
        }
    }

    fn unary(&mut self) -> PResult<Val> {
        if self.peek_op(Op::Sub) {
            self.i += 1;
            let r = self.unary()?;
            return Ok(Val {
                v: -r.v,
                pct: r.pct,
            });
        }
        if self.peek_op(Op::Add) {
            self.i += 1;
            return self.unary();
        }
        self.power()
    }

    fn power(&mut self) -> PResult<Val> {
        let base = self.postfix()?;
        if !self.peek_op(Op::Pow) {
            return Ok(base);
        }
        self.i += 1;
        self.ops += 1;
        let e = self.unary()?.v;
        Ok(Val {
            v: base.v.powf(e),
            pct: false,
        })
    }

    fn postfix(&mut self) -> PResult<Val> {
        let mut v = self.atom()?;
        let mut pct = false;
        loop {
            match self.peek() {
                Some(Kind::Bang) => {
                    v = factorial(v);
                    pct = false;
                }
                Some(Kind::Op(Op::Percent)) => {
                    v /= 100.0;
                    pct = true;
                }
                _ => return Ok(Val { v, pct }),
            }
            self.i += 1;
            self.ops += 1;
        }
    }

    fn atom(&mut self) -> PResult<f64> {
        let t = self.peek().ok_or(ParseError)?;
        self.i += 1;
        match t {
            Kind::Num(v) | Kind::Const(v) => Ok(v),
            Kind::Fn(f) => {
                self.ops += 1;
                if matches!(self.peek(), Some(Kind::LParen)) {
                    self.i += 1;
                    let v = self.expr()?.v;
                    self.close_paren()?;
                    Ok(f(v))
                } else {
                    Ok(f(self.postfix()?.v))
                }
            }
            Kind::LParen => {
                let v = self.expr()?.v;
                self.close_paren()?;
                Ok(v)
            }
            _ => Err(ParseError),
        }
    }
}

fn factorial(n: f64) -> f64 {
    if !(0.0..=170.0).contains(&n) || n.fract() != 0.0 {
        return f64::NAN;
    }
    (2..=n as u32).fold(1.0, |r, i| r * f64::from(i))
}

// ---- formatting -------------------------------------------------------------

/// Round a finite, nonzero value to `sig` significant digits, halves away
/// from zero like JS `toPrecision` (Rust's formatter rounds half-to-even).
fn round_sig(v: f64, sig: usize) -> f64 {
    // Shortest round-trip digits, e.g. "1.234567890125e12".
    let s = format!("{:e}", v.abs());
    let (mant, exp) = s.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    let digits: Vec<u8> = mant
        .bytes()
        .filter(u8::is_ascii_digit)
        .map(|b| b - b'0')
        .collect();
    if digits.len() <= sig {
        return v;
    }
    let mut kept: Vec<u8> = digits[..sig].to_vec();
    if digits[sig] >= 5 {
        let mut k = sig;
        loop {
            if k == 0 {
                kept.insert(0, 1);
                kept.pop();
                return v.signum() * rebuild(&kept, exp + 1);
            }
            k -= 1;
            if kept[k] == 9 {
                kept[k] = 0;
            } else {
                kept[k] += 1;
                break;
            }
        }
    }
    v.signum() * rebuild(&kept, exp)
}

fn rebuild(digits: &[u8], exp: i32) -> f64 {
    let d: String = digits.iter().map(|d| char::from(b'0' + d)).collect();
    format!("{}.{}e{}", &d[..1], &d[1..], exp).parse().unwrap()
}

fn trim_fraction(s: &str) -> &str {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.')
    } else {
        s
    }
}

pub fn format_number(v: f64, decimal_comma: bool) -> String {
    let v = if v == 0.0 { 0.0 } else { v }; // -0 -> 0
    let a = v.abs();
    if a != 0.0 && !(1e-9..1e15).contains(&a) {
        // toExponential(9) with trailing zeros and "e+" stripped.
        let s = format!("{v:.9e}");
        let (mant, exp) = s.split_once('e').unwrap();
        return format!("{}e{}", trim_fraction(mant), exp);
    }

    // 12 significant digits hides float noise (0.1+0.2 -> 0.3).
    let r = if a == 0.0 { 0.0 } else { round_sig(v, 12) };
    let s = if r != 0.0 && r.abs() < 1e-6 {
        // JS prints these in exponent form; calc.js then uses toFixed(12).
        trim_fraction(&format!("{r:.12}")).to_string()
    } else {
        r.to_string()
    };

    let (sign, rest) = s.strip_prefix('-').map_or(("", s.as_str()), |r| ("-", r));
    let (ip, fp) = rest
        .split_once('.')
        .map_or((rest, None), |(i, f)| (i, Some(f)));
    let ip = if ip.len() > 4 {
        let sep = if decimal_comma { '.' } else { ',' };
        let mut out = String::new();
        for (k, ch) in ip.chars().enumerate() {
            if k > 0 && (ip.len() - k) % 3 == 0 {
                out.push(sep);
            }
            out.push(ch);
        }
        out
    } else {
        ip.to_string()
    };
    match fp {
        Some(f) => format!("{sign}{ip}{}{f}", if decimal_comma { ',' } else { '.' }),
        None => format!("{sign}{ip}"),
    }
}

// ---- false positives: arithmetic-shaped text that is something else ---------

/// One element of a tiny anchored pattern: a char class repeated min..=max.
type Elem = (fn(char) -> bool, usize, usize);

fn digit(c: char) -> bool {
    c.is_ascii_digit()
}

/// Anchored, backtracking match of `pat` against all of `s`.
fn matches_all(s: &[char], pat: &[Elem]) -> bool {
    let Some(&(class, min, max)) = pat.first() else {
        return s.is_empty();
    };
    let avail = s.iter().take(max).take_while(|&&c| class(c)).count();
    (min..=avail).rev().any(|n| matches_all(&s[n..], &pat[1..]))
}

fn is_false_positive(expr: &[char]) -> bool {
    const PATTERNS: &[&[Elem]] = &[
        // 2026-10-06
        &[
            (digit, 4, 4),
            (|c| c == '-', 1, 1),
            (digit, 1, 2),
            (|c| c == '-', 1, 1),
            (digit, 1, 2),
        ],
        // 06/10/2026
        &[
            (digit, 1, 2),
            (|c| matches!(c, '/' | '.' | '-'), 1, 1),
            (digit, 1, 2),
            (|c| matches!(c, '/' | '.' | '-'), 1, 1),
            (digit, 2, 4),
        ],
        // (11) 98765-4321
        &[
            (|c| c == '(', 0, 1),
            (digit, 2, 3),
            (|c| c == ')', 0, 1),
            (|c| c.is_whitespace() || c == '-', 0, 1),
            (digit, 4, 5),
            (|c| c == '-', 1, 1),
            (digit, 4, 4),
        ],
    ];
    PATTERNS.iter().any(|p| matches_all(expr, p))
}

// ---- entry point ----------------------------------------------------------------

pub fn find_and_evaluate(before: &str) -> Option<Evaluation> {
    let line_start = before.rfind(['\n', '\r']).map_or(0, |i| i + 1);
    let mut line: Vec<char> = before[line_start..].chars().collect();
    if line.len() > MAX_LOOKBACK {
        line.drain(..line.len() - MAX_LOOKBACK);
    }

    let mut end = line.len();
    while end > 0 && line[end - 1].is_whitespace() {
        end -= 1;
    }
    if end > 0 && line[end - 1] == '=' {
        end -= 1;
        while end > 0 && line[end - 1].is_whitespace() {
            end -= 1;
        }
    }
    let body = &line[..end];
    if body.is_empty() {
        return None;
    }

    let (toks, used_comma) = tokenize(body);

    // The expression can't extend left past the last unknown word.
    let lo = toks
        .iter()
        .rposition(|t| matches!(t.kind, Kind::Word))
        .map_or(0, |i| i + 1);
    if lo >= toks.len() {
        return None;
    }

    // Longest valid suffix wins. Starting mid-way is only allowed at a token
    // that can begin a value; a sign is only a valid start at the far left,
    // otherwise "5-3" would also be tried as "-3".
    for i in lo..toks.len() {
        let t0 = toks[i];
        let is_sign = matches!(t0.kind, Kind::Op(Op::Add | Op::Sub));
        if !is_value_start(Some(&t0)) && !(is_sign && i == lo) {
            continue;
        }
        let mut p = Parser {
            t: &toks[i..],
            i: 0,
            ops: 0,
        };
        let Ok(v) = p.parse() else { continue };
        if p.ops == 0 || !v.is_finite() {
            return None;
        }
        let expr = &body[t0.start..];
        if is_false_positive(expr) {
            return None;
        }
        return Some(Evaluation {
            expr: expr.iter().collect(),
            value: v,
            display: format_number(v, used_comma),
        });
    }
    None
}
