use crate::lex::{AnnotatedToken, Token};

#[derive(PartialEq, Debug, Clone)]
pub enum AST {
    List(Vec<AST>),
    Integer(i64),
    Symbol(String),
}

#[derive(Debug)]
pub struct Error {
    line: u64,
    column: u64,
    message: &'static str,
}

const UNMATCHED_RPAREN_ERROR: &str = "unmatched ')'";
const MISSING_RPAREN_ERROR: &str = "missing matching ')'";

fn parse_node(tokens: &[AnnotatedToken]) -> Result<(AST, &[AnnotatedToken]), Error> {
    let (first, rest) = tokens.split_first().unwrap();

    match &first.token {
        Token::Integer(i) => Ok((AST::Integer(*i), rest)),
        Token::Symbol(s) => Ok((AST::Symbol(s.clone()), rest)),
        Token::Rparen => Err(Error {
            line: first.line,
            column: first.column,
            message: UNMATCHED_RPAREN_ERROR,
        }),
        Token::Lparen => {
            let mut remaining_toks = rest;
            let mut items = Vec::new();

            loop {
                if remaining_toks.is_empty() {
                    return Err(Error {
                        line: first.line,
                        column: first.column,
                        message: MISSING_RPAREN_ERROR,
                    });
                }

                if remaining_toks[0].token == Token::Rparen {
                    remaining_toks = &remaining_toks[1..];
                    break;
                }

                let (item, rest) = parse_node(remaining_toks)?;
                items.push(item);
                remaining_toks = rest;
            }

            Ok((AST::List(items), remaining_toks))
        }
    }
}

impl ToString for Error {
    fn to_string(&self) -> String {
        format!(
            "Syntax error at line {}, col {}: {}",
            self.line, self.column, self.message
        )
    }
}

pub fn parse(tokens: &[AnnotatedToken]) -> Result<Vec<AST>, Error> {
    let mut res = Vec::new();
    let mut tokens = tokens;

    while !tokens.is_empty() {
        let (ast, remaining) = parse_node(tokens)?;
        res.push(ast);
        tokens = remaining
    }

    Ok(res)
}

#[cfg(test)]
mod test {
    use super::*;

    fn annotate_tokens(tokens: Vec<Token>) -> Vec<AnnotatedToken> {
        tokens
            .into_iter()
            .map(|token| AnnotatedToken {
                token,
                line: 0,
                column: 0,
            })
            .collect()
    }

    #[test]
    fn generates_correct_ast() {
        use Token::*;
        // (+ ( 1 2) 3)
        let tokens = annotate_tokens(vec![
            Lparen,
            Symbol("+".to_string()),
            Lparen,
            Symbol("+".to_string()),
            Integer(1),
            Integer(2),
            Rparen,
            Integer(3),
            Rparen,
        ]);
        let ast = parse(&tokens).unwrap();
        assert_eq!(
            ast,
            vec![AST::List(vec![
                AST::Symbol("+".to_string()),
                AST::List(vec![
                    AST::Symbol("+".to_string()),
                    AST::Integer(1),
                    AST::Integer(2),
                ]),
                AST::Integer(3)
            ])]
        );
    }

    #[test]
    fn handles_unmatched_lparen() {
        let tokens = annotate_tokens(vec![Token::Lparen]);
        let res = parse(&tokens);
        assert!(res.is_err())
    }

    #[test]
    fn handles_extra_rparen() {
        let tokens = annotate_tokens(vec![Token::Rparen]);
        let res = parse(&tokens);
        assert!(res.is_err())
    }
}
