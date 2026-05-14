/// Minimal SCIM 2.0 filter parser (RFC 7644 §3.4.2.2).
///
/// Supports: eq, ne, sw, co, pr, and, or.
/// Complex paths (sub-attribute filters like `emails[type eq "work"]`) are
/// rejected with UnsupportedOperation.
use crate::error::ScimError;

#[derive(Debug, Clone, PartialEq)]
pub enum FilterExpr {
    Eq(String, String),
    Ne(String, String),
    Sw(String, String),
    Co(String, String),
    Pr(String),
    And(Box<FilterExpr>, Box<FilterExpr>),
    Or(Box<FilterExpr>, Box<FilterExpr>),
}

pub fn parse(input: &str) -> Result<FilterExpr, ScimError> {
    let tokens = tokenize(input)?;
    let (expr, rest) = parse_or(&tokens)?;
    if !rest.is_empty() {
        return Err(ScimError::InvalidFilter(format!(
            "unexpected tokens: {rest:?}"
        )));
    }
    Ok(expr)
}

// ── Tokeniser ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Word(String),
    QuotedString(String),
    LParen,
    RParen,
}

fn tokenize(input: &str) -> Result<Vec<Token>, ScimError> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' | '\t' => {
                chars.next();
            }
            '(' => {
                chars.next();
                tokens.push(Token::LParen);
            }
            ')' => {
                chars.next();
                tokens.push(Token::RParen);
            }
            '"' => {
                chars.next();
                let mut s = String::new();
                for c in chars.by_ref() {
                    if c == '"' {
                        break;
                    }
                    s.push(c);
                }
                tokens.push(Token::QuotedString(s));
            }
            '[' => {
                return Err(ScimError::UnsupportedOperation(
                    "sub-attribute filters are not supported".into(),
                ));
            }
            _ => {
                let mut word = String::new();
                while let Some(&c) = chars.peek() {
                    if c == ' ' || c == '\t' || c == '(' || c == ')' {
                        break;
                    }
                    word.push(c);
                    chars.next();
                }
                tokens.push(Token::Word(word));
            }
        }
    }

    Ok(tokens)
}

// ── Recursive-descent parser ──────────────────────────────────────────────────

type ParseResult<'a> = Result<(FilterExpr, &'a [Token]), ScimError>;

fn parse_or<'a>(tokens: &'a [Token]) -> ParseResult<'a> {
    let (mut left, mut rest) = parse_and(tokens)?;
    while matches!(rest.first(), Some(Token::Word(w)) if w.eq_ignore_ascii_case("or")) {
        let (right, after) = parse_and(&rest[1..])?;
        left = FilterExpr::Or(Box::new(left), Box::new(right));
        rest = after;
    }
    Ok((left, rest))
}

fn parse_and<'a>(tokens: &'a [Token]) -> ParseResult<'a> {
    let (mut left, mut rest) = parse_primary(tokens)?;
    while matches!(rest.first(), Some(Token::Word(w)) if w.eq_ignore_ascii_case("and")) {
        let (right, after) = parse_primary(&rest[1..])?;
        left = FilterExpr::And(Box::new(left), Box::new(right));
        rest = after;
    }
    Ok((left, rest))
}

fn parse_primary<'a>(tokens: &'a [Token]) -> ParseResult<'a> {
    match tokens.first() {
        Some(Token::LParen) => {
            let (expr, rest) = parse_or(&tokens[1..])?;
            match rest.first() {
                Some(Token::RParen) => Ok((expr, &rest[1..])),
                _ => Err(ScimError::InvalidFilter("expected ')'".into())),
            }
        }
        Some(Token::Word(attr)) => {
            let attr = attr.clone();
            let op = match tokens.get(1) {
                Some(Token::Word(op)) => op.to_lowercase(),
                _ => {
                    return Err(ScimError::InvalidFilter(
                        "expected operator after attribute".into(),
                    ))
                }
            };

            if op == "pr" {
                return Ok((FilterExpr::Pr(attr), &tokens[2..]));
            }

            let value = match tokens.get(2) {
                Some(Token::QuotedString(s)) => s.clone(),
                Some(Token::Word(s)) => s.clone(),
                _ => {
                    return Err(ScimError::InvalidFilter(format!(
                        "expected value for operator {op}"
                    )))
                }
            };

            let expr = match op.as_str() {
                "eq" => FilterExpr::Eq(attr, value),
                "ne" => FilterExpr::Ne(attr, value),
                "sw" => FilterExpr::Sw(attr, value),
                "co" => FilterExpr::Co(attr, value),
                other => {
                    return Err(ScimError::UnsupportedOperation(format!(
                        "filter operator '{other}' is not supported"
                    )))
                }
            };

            Ok((expr, &tokens[3..]))
        }
        other => Err(ScimError::InvalidFilter(format!(
            "unexpected token: {other:?}"
        ))),
    }
}

/// Evaluates a filter against a map of attribute → value strings.
/// Used to post-filter results since we don't push all filters to SQL.
pub fn matches_filter(filter: &FilterExpr, attrs: &std::collections::HashMap<&str, &str>) -> bool {
    match filter {
        FilterExpr::Eq(attr, val) => attrs
            .get(attr.as_str())
            .is_some_and(|v| v.eq_ignore_ascii_case(val)),
        FilterExpr::Ne(attr, val) => !attrs
            .get(attr.as_str())
            .is_some_and(|v| v.eq_ignore_ascii_case(val)),
        FilterExpr::Sw(attr, val) => attrs
            .get(attr.as_str())
            .is_some_and(|v| v.to_lowercase().starts_with(&val.to_lowercase())),
        FilterExpr::Co(attr, val) => attrs
            .get(attr.as_str())
            .is_some_and(|v| v.to_lowercase().contains(&val.to_lowercase())),
        FilterExpr::Pr(attr) => attrs.contains_key(attr.as_str()),
        FilterExpr::And(l, r) => matches_filter(l, attrs) && matches_filter(r, attrs),
        FilterExpr::Or(l, r) => matches_filter(l, attrs) || matches_filter(r, attrs),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn attrs(pairs: &[(&'static str, &'static str)]) -> HashMap<&'static str, &'static str> {
        pairs.iter().cloned().collect()
    }

    #[test]
    fn eq_filter() {
        let f = parse(r#"userName eq "john@example.com""#).unwrap();
        assert!(matches_filter(
            &f,
            &attrs(&[("userName", "john@example.com")])
        ));
        assert!(!matches_filter(
            &f,
            &attrs(&[("userName", "jane@example.com")])
        ));
    }

    #[test]
    fn sw_filter() {
        let f = parse(r#"displayName sw "Eng""#).unwrap();
        assert!(matches_filter(
            &f,
            &attrs(&[("displayName", "Engineering")])
        ));
        assert!(!matches_filter(&f, &attrs(&[("displayName", "Sales")])));
    }

    #[test]
    fn pr_filter() {
        let f = parse("externalId pr").unwrap();
        assert!(matches_filter(&f, &attrs(&[("externalId", "abc")])));
        assert!(!matches_filter(&f, &attrs(&[])));
    }

    #[test]
    fn and_filter() {
        let f = parse(r#"active eq "true" and userName sw "a""#).unwrap();
        assert!(matches_filter(
            &f,
            &attrs(&[("active", "true"), ("userName", "alice")])
        ));
        assert!(!matches_filter(
            &f,
            &attrs(&[("active", "false"), ("userName", "alice")])
        ));
    }

    #[test]
    fn grouped_filter() {
        let f = parse(r#"(userName eq "alice" or userName eq "bob")"#).unwrap();
        assert!(matches_filter(&f, &attrs(&[("userName", "alice")])));
        assert!(matches_filter(&f, &attrs(&[("userName", "bob")])));
        assert!(!matches_filter(&f, &attrs(&[("userName", "carol")])));
    }
}
