// SPDX-License-Identifier: GPL-3.0-or-later
// Expresso expression finder + evaluator. Pure JS with no GI imports, so it
// can be unit-tested with plain `gjs -m test.js` outside the shell.
//
// findAndEvaluate(textBeforeCursor) finds the longest math expression ending
// at the cursor (optionally followed by "=") and returns
// {expr, value, display}, or null when there is nothing worth showing.

const FUNCS = {
    sqrt: Math.sqrt, cbrt: Math.cbrt, abs: Math.abs, exp: Math.exp,
    sin: Math.sin, cos: Math.cos, tan: Math.tan,
    asin: Math.asin, acos: Math.acos, atan: Math.atan,
    sinh: Math.sinh, cosh: Math.cosh, tanh: Math.tanh,
    ln: Math.log, log: Math.log10, log10: Math.log10, log2: Math.log2,
    round: Math.round, floor: Math.floor, ceil: Math.ceil,
};
const CONSTS = {pi: Math.PI, 'π': Math.PI, e: Math.E, tau: 2 * Math.PI, 'τ': 2 * Math.PI};

const SYMBOLS = {
    '+': ['op', '+'], '-': ['op', '-'], '−': ['op', '-'], '–': ['op', '-'],
    '*': ['op', '*'], '×': ['op', '*'], '·': ['op', '*'], '∙': ['op', '*'],
    '/': ['op', '/'], '÷': ['op', '/'],
    '^': ['op', '^'], '%': ['op', '%'],
    '(': ['lp'], '[': ['lp'], ')': ['rp'], ']': ['rp'],
    '!': ['bang'], '√': ['fn', Math.sqrt],
};

// 1,234,567(.89)  -> grouping commas
// 1,5             -> decimal comma (pt-BR)
// 1.5  .5  1e-3   -> plain
const NUM_GROUPED = /^\d{1,3}(?:,\d{3})+(?![\d,])(?:\.\d+)?/;
const NUM_DECCOMMA = /^\d+,\d+(?:[eE][+-]?\d+)?/;
const NUM_PLAIN = /^(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?/;
const IDENT = /^(?:[a-zA-Z_]\w*|π|τ)/;

const MAX_LOOKBACK = 300;

function isValueStart(t) {
    return t && (t.k === 'num' || t.k === 'const' || t.k === 'fn' || t.k === 'lp');
}

function isValueEnd(t) {
    return t && (t.k === 'num' || t.k === 'const' || t.k === 'rp' || t.k === 'bang');
}

// Token kinds: num, op, lp, rp, bang, fn, const, and word (anything we don't
// understand; an expression never spans one).
function tokenize(s) {
    const toks = [];
    let usedComma = false;
    let i = 0;
    while (i < s.length) {
        const c = s[i];
        if (/\s/.test(c)) {
            i++;
            continue;
        }
        const rest = s.slice(i);
        let m;
        if ((m = NUM_GROUPED.exec(rest))) {
            toks.push({k: 'num', v: parseFloat(m[0].replace(/,/g, '')), start: i});
        } else if ((m = NUM_DECCOMMA.exec(rest))) {
            usedComma = true;
            toks.push({k: 'num', v: parseFloat(m[0].replace(',', '.')), start: i});
        } else if ((m = NUM_PLAIN.exec(rest))) {
            toks.push({k: 'num', v: parseFloat(m[0]), start: i});
        } else if ((c === 'x' || c === 'X') && isValueEnd(toks.at(-1)) && !/^.[a-zA-Z_]/.test(rest)) {
            m = [c];
            toks.push({k: 'op', v: '*', start: i});                // 3x4, 3 x 4
        } else if ((m = IDENT.exec(rest))) {
            const lw = m[0].toLowerCase();
            if (lw === 'mod')
                toks.push({k: 'op', v: 'mod', start: i});
            else if (lw in FUNCS)
                toks.push({k: 'fn', v: FUNCS[lw], start: i});
            else if (lw in CONSTS)
                toks.push({k: 'const', v: CONSTS[lw], start: i});
            else
                toks.push({k: 'word', start: i});
        } else if (c === '*' && s[i + 1] === '*') {
            m = ['**'];
            toks.push({k: 'op', v: '^', start: i});
        } else if (SYMBOLS[c]) {
            m = [c];
            const [k, v] = SYMBOLS[c];
            toks.push({k, v, start: i});
        } else {
            m = [c];
            toks.push({k: 'word', start: i});
        }
        i += m[0].length;
    }

    // "a % b" is modulo; "15%" followed by nothing / an operator is percent.
    for (let j = 0; j < toks.length; j++) {
        if (toks[j].k === 'op' && toks[j].v === '%' && isValueStart(toks[j + 1]))
            toks[j].v = 'mod';
    }
    return {toks, usedComma};
}

// Grammar (recursive descent):
//   expr    := term (('+'|'-') term)*        a ± b%  means  a * (1 ± b/100)
//   term    := unary (('*'|'/'|mod) unary | <implicit> unary)*
//   unary   := ('-'|'+') unary | power
//   power   := postfix ('^' unary)?            right-assoc, so -2^2 = -4
//   postfix := atom ('!' | '%')*
//   atom    := num | const | fn atom | fn '(' expr ')' | '(' expr ')'
// Unclosed parentheses at the end are accepted, so "(1+2" works while typing.
class Parser {
    constructor(toks) {
        this.t = toks;
        this.i = 0;
        this.ops = 0;          // operations seen; 0 means "just a number"
    }

    peek(k, v) {
        const t = this.t[this.i];
        return Boolean(t) && t.k === k && (v === undefined || t.v === v);
    }

    parse() {
        const {v} = this.expr();
        if (this.i !== this.t.length)
            throw new Error('trailing input');
        return v;
    }

    closeParen() {
        if (this.i === this.t.length)
            return;
        if (!this.peek('rp'))
            throw new Error('expected )');
        this.i++;
    }

    // Each level returns {v, pct}; pct marks a value that ended in a bare %,
    // which only matters directly to the right of + or -.
    expr() {
        let {v} = this.term();
        while (this.peek('op', '+') || this.peek('op', '-')) {
            const op = this.t[this.i++].v;
            const r = this.term();
            this.ops++;
            const rv = r.pct ? v * r.v : r.v;
            v = op === '+' ? v + rv : v - rv;
        }
        return {v, pct: false};
    }

    term() {
        let r = this.unary();
        for (;;) {
            let op;
            if (this.peek('op', '*') || this.peek('op', '/') || this.peek('op', 'mod'))
                op = this.t[this.i++].v;
            else if (isValueStart(this.t[this.i]) && !this.peek('num'))
                op = '*';          // implicit: 2pi, 3(1+1); never "1.2 .3"
            else
                return r;
            const b = this.unary().v;
            this.ops++;
            const a = r.v;
            r = {v: op === '*' ? a * b : op === '/' ? a / b : ((a % b) + b) % b, pct: false};
        }
    }

    unary() {
        if (this.peek('op', '-')) {
            this.i++;
            const r = this.unary();
            return {v: -r.v, pct: r.pct};
        }
        if (this.peek('op', '+')) {
            this.i++;
            return this.unary();
        }
        return this.power();
    }

    power() {
        const base = this.postfix();
        if (!this.peek('op', '^'))
            return base;
        this.i++;
        this.ops++;
        return {v: base.v ** this.unary().v, pct: false};
    }

    postfix() {
        let v = this.atom();
        let pct = false;
        for (;;) {
            if (this.peek('bang')) {
                v = factorial(v);
                pct = false;
            } else if (this.peek('op', '%')) {
                v /= 100;
                pct = true;
            } else {
                return {v, pct};
            }
            this.i++;
            this.ops++;
        }
    }

    atom() {
        const t = this.t[this.i];
        if (!t)
            throw new Error('unexpected end');
        this.i++;
        switch (t.k) {
        case 'num':
        case 'const':
            return t.v;
        case 'fn': {
            this.ops++;
            if (this.peek('lp')) {
                this.i++;
                const {v} = this.expr();
                this.closeParen();
                return t.v(v);
            }
            return t.v(this.postfix().v);
        }
        case 'lp': {
            const {v} = this.expr();
            this.closeParen();
            return v;
        }
        default:
            throw new Error('unexpected token');
        }
    }
}

function factorial(n) {
    if (n < 0 || n > 170 || !Number.isInteger(n))
        return NaN;
    let r = 1;
    for (let i = 2; i <= n; i++)
        r *= i;
    return r;
}

export function formatNumber(v, decimalComma = false) {
    if (Object.is(v, -0))
        v = 0;
    const a = Math.abs(v);
    if (a !== 0 && (a >= 1e15 || a < 1e-9))
        return v.toExponential(9).replace(/\.?0+e/, 'e').replace('e+', 'e');

    // 12 significant digits hides float noise (0.1+0.2 -> 0.3).
    let s = String(parseFloat(v.toPrecision(12)));
    if (s.includes('e'))
        s = parseFloat(v.toPrecision(12)).toFixed(12).replace(/\.?0+$/, '');
    let [ip, fp] = s.split('.');
    const sign = ip.startsWith('-') ? '-' : '';
    if (sign)
        ip = ip.slice(1);
    if (ip.length > 4)
        ip = ip.replace(/\B(?=(\d{3})+(?!\d))/g, decimalComma ? '.' : ',');
    return sign + ip + (fp ? (decimalComma ? ',' : '.') + fp : '');
}

// Arithmetic-shaped text that is almost always something else.
const FALSE_POSITIVES = [
    /^\d{4}-\d{1,2}-\d{1,2}$/,                 // 2026-10-06
    /^\d{1,2}[/.-]\d{1,2}[/.-]\d{2,4}$/,       // 06/10/2026
    /^\(?\d{2,3}\)?[\s-]?\d{4,5}-\d{4}$/,      // (11) 98765-4321
];

export function findAndEvaluate(before) {
    if (!before)
        return null;
    const nl = Math.max(before.lastIndexOf('\n'), before.lastIndexOf('\r'));
    let line = before.slice(nl + 1);
    if (line.length > MAX_LOOKBACK)
        line = line.slice(-MAX_LOOKBACK);

    let body = line.trimEnd();
    if (body.endsWith('='))
        body = body.slice(0, -1).trimEnd();
    if (!body)
        return null;

    const {toks, usedComma} = tokenize(body);

    // The expression can't extend left past the last unknown word.
    let lo = 0;
    for (let i = toks.length - 1; i >= 0; i--) {
        if (toks[i].k === 'word') {
            lo = i + 1;
            break;
        }
    }
    if (lo >= toks.length)
        return null;

    // Longest valid suffix wins. Starting mid-way is only allowed at a token
    // that can begin a value; a sign is only a valid start at the far left,
    // otherwise "5-3" would also be tried as "-3".
    for (let i = lo; i < toks.length; i++) {
        const t0 = toks[i];
        const isSign = t0.k === 'op' && (t0.v === '-' || t0.v === '+');
        if (!isValueStart(t0) && !(isSign && i === lo))
            continue;
        let p, v;
        try {
            p = new Parser(toks.slice(i));
            v = p.parse();
        } catch {
            continue;
        }
        if (p.ops === 0 || !Number.isFinite(v))
            return null;
        const expr = body.slice(t0.start);
        if (FALSE_POSITIVES.some(re => re.test(expr)))
            return null;
        return {expr, value: v, display: formatNumber(v, usedComma)};
    }
    return null;
}
