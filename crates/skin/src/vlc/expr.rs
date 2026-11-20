//! VLC's boolean expressions (`visible`, a checkbox's `state`; M9):
//! variables joined by `not`, `and`, `or` and parentheses. VLC splits
//! tokens only at spaces and `)`, and binds `or` tighter than `and`
//! (`a and b or c` is `a and (b or c)`); `not` binds tightest. An
//! expression with a token VLC does not know is void: a control with such
//! a `visible` always shows.
//!
//! An expression is read and computed by functions that call themselves
//! for each `not`, parenthesis and operator, so its length is bounded: a
//! theme's own expressions by `limits::EXPRESSION_TOKENS` (the theme does
//! not load beyond it), and [`Expr::parse`] takes no more than
//! [`PARSE_TOKENS`].

use crate::limits;

/// The most tokens [`Expr::parse`] takes: two of a theme's expressions
/// joined by RVP (a checkbox's `visible` and its `state`), with the
/// parentheses, `and` and `not` RVP puts around them.
pub const PARSE_TOKENS: usize = 2 * limits::EXPRESSION_TOKENS + 8;

/// The number of tokens in an expression's text.
#[must_use]
pub fn token_count(text: &str) -> usize {
    tokens(text).len()
}

/// A parsed expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Var(String),
    Not(Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
}

/// The tokens: split at spaces, `(` and `)` kept as tokens of their own
/// only where VLC sees them (a `(` glued to a word stays in it).
fn tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for word in text.split_whitespace() {
        let mut rest = word;
        while let Some(stripped) = rest.strip_prefix('(') {
            out.push("(".to_owned());
            rest = stripped;
        }
        // A `)` always ends a token: `vlc.hasVout(false)` is the unknown
        // `vlc.hasVout(false` and a `)`.
        let mut closing = 0;
        while let Some(stripped) = rest.strip_suffix(')') {
            closing += 1;
            rest = stripped;
        }
        if !rest.is_empty() {
            out.push(rest.to_owned());
        }
        out.extend(std::iter::repeat_n(")".to_owned(), closing));
    }
    out
}

struct Parser {
    tokens: Vec<String>,
    at: usize,
}

impl Parser {
    fn peek(&self) -> Option<&str> {
        self.tokens.get(self.at).map(String::as_str)
    }

    fn next(&mut self) -> Option<String> {
        let token = self.tokens.get(self.at).cloned();
        self.at += 1;
        token
    }

    // and: the loosest.
    fn and(&mut self) -> Option<Expr> {
        let mut left = self.or()?;
        while self.peek() == Some("and") {
            self.next();
            let right = self.or()?;
            left = Expr::And(Box::new(left), Box::new(right));
        }
        Some(left)
    }

    fn or(&mut self) -> Option<Expr> {
        let mut left = self.not()?;
        while self.peek() == Some("or") {
            self.next();
            let right = self.not()?;
            left = Expr::Or(Box::new(left), Box::new(right));
        }
        Some(left)
    }

    fn not(&mut self) -> Option<Expr> {
        if self.peek() == Some("not") {
            self.next();
            return Some(Expr::Not(Box::new(self.not()?)));
        }
        match self.next()?.as_str() {
            "(" => {
                let inner = self.and()?;
                (self.next().as_deref() == Some(")")).then_some(inner)
            }
            ")" | "and" | "or" => None,
            word => Some(Expr::Var(word.to_owned())),
        }
    }
}

impl Expr {
    /// Parses an expression; `None` for an empty or broken one, and for
    /// one of more than [`PARSE_TOKENS`] tokens.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let tokens = tokens(text);
        if tokens.len() > PARSE_TOKENS {
            return None;
        }
        let mut parser = Parser { tokens, at: 0 };
        let expr = parser.and()?;
        (parser.at == parser.tokens.len()).then_some(expr)
    }

    /// The value, with `var` giving each variable's (`None`: unknown, which
    /// voids the whole expression).
    pub fn eval(&self, var: &mut impl FnMut(&str) -> Option<bool>) -> Option<bool> {
        Some(match self {
            Expr::Var(name) => match name.as_str() {
                "true" => true,
                "false" => false,
                name => var(name)?,
            },
            Expr::Not(inner) => !inner.eval(var)?,
            Expr::And(a, b) => {
                let (a, b) = (a.eval(var)?, b.eval(var)?);
                a && b
            }
            Expr::Or(a, b) => {
                let (a, b) = (a.eval(var)?, b.eval(var)?);
                a || b
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(text: &str, a: bool, b: bool, c: bool) -> Option<bool> {
        Expr::parse(text)?.eval(&mut |name| match name {
            "a" => Some(a),
            "b" => Some(b),
            "c" => Some(c),
            _ => None,
        })
    }

    #[test]
    fn or_binds_tighter_than_and() {
        // a and (b or c)
        assert_eq!(eval("a and b or c", false, false, true), Some(false));
        assert_eq!(eval("a and b or c", true, false, true), Some(true));
        // (a or b) and c
        assert_eq!(eval("a or b and c", true, false, false), Some(false));
        assert_eq!(eval("not a or b", true, false, false), Some(false));
        assert_eq!(eval("not (a or b)", false, false, false), Some(true));
        assert_eq!(eval("(a and b) or c", false, true, true), Some(true));
    }

    #[test]
    fn unknown_tokens_void_the_expression() {
        assert_eq!(eval("a and x", true, true, true), None);
        // VLC keeps `vlc.hasVout(false` as one token.
        assert_eq!(
            tokens("not vlc.hasVout(false)"),
            vec!["not", "vlc.hasVout(false", ")"]
        );
        assert_eq!(eval("vlc.isStopped;vlc.isPaused", true, true, true), None);
        assert_eq!(Expr::parse("a and"), None);
        assert_eq!(eval("true", false, false, false), Some(true));
    }

    #[test]
    fn long_expressions_are_refused() {
        // Each of these is read, computed and dropped by functions that
        // call themselves once per token: 100,000 tokens end the process.
        let texts = [
            "not ".repeat(100_000) + "a",
            "( ".repeat(100_000) + "a" + &" )".repeat(100_000),
            "a".to_owned() + &" and a".repeat(100_000),
            "a".to_owned() + &" or a".repeat(100_000),
        ];
        let parsed = limits::on_small_stack(move || {
            texts
                .iter()
                .map(|text| (token_count(text), Expr::parse(text).is_some()))
                .collect::<Vec<_>>()
        });
        for (tokens, parsed) in parsed {
            assert!(tokens > 100_000);
            assert!(!parsed);
        }
        // The longest expressions that are taken, of each kind, on the
        // same small stack.
        let longest = limits::on_small_stack(|| {
            let texts = [
                "not ".repeat(PARSE_TOKENS - 1) + "true",
                "( ".repeat((PARSE_TOKENS - 1) / 2) + "true" + &" )".repeat((PARSE_TOKENS - 1) / 2),
                "true".to_owned() + &" and true".repeat((PARSE_TOKENS - 1) / 2),
                "true".to_owned() + &" or true".repeat((PARSE_TOKENS - 1) / 2),
            ];
            texts
                .iter()
                .map(|text| {
                    let expr = Expr::parse(text).expect("within the limit");
                    let copy = expr.clone();
                    (token_count(text), copy.eval(&mut |_| None))
                })
                .collect::<Vec<_>>()
        });
        for (tokens, value) in longest {
            assert!(
                (PARSE_TOKENS - 1..=PARSE_TOKENS).contains(&tokens),
                "{tokens}"
            );
            assert!(value.is_some());
        }
        assert!(Expr::parse(&("not ".repeat(PARSE_TOKENS) + "true")).is_none());
    }
}
