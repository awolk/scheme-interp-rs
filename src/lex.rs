use std::iter::Peekable;
use std::str::Chars;

#[derive(PartialEq, Debug)]
pub enum Token {
    Lparen,
    Rparen,
    Integer(i64),
    Symbol(String),
}

#[derive(Debug)]
pub struct Error {
    line_number: u64,
    column_number: u64,
    error_message: &'static str,
}

impl ToString for Error {
    fn to_string(&self) -> String {
        format!(
            "Syntax error at line {}, col {}: {}",
            self.line_number, self.column_number, self.error_message
        )
    }
}

const INVALID_INTEGER_ERROR: &str = "unable to parse integer value";

struct Lexer<'a> {
    iter: Peekable<Chars<'a>>,
    line_number: u64,
    column_number: u64,
}

impl<'a> Lexer<'a> {
    fn new(source: &str) -> Lexer {
        Lexer {
            iter: source.chars().peekable(),
            line_number: 0,
            column_number: 0,
        }
    }

    fn next_chr(&mut self) -> Option<char> {
        let next = self.iter.next();

        match next {
            Some('\n') => {
                self.column_number = 0;
                self.line_number += 1;
            }
            Some(_) => {
                self.column_number += 1;
            }
            None => {}
        }

        next
    }

    // next returns the next token from the source
    // if there are no more tokens it returns Ok(None)
    // it returns Err if there is a syntax error
    fn next(&mut self) -> Result<Option<Token>, Error> {
        self.dump_whitespace();

        let next_chr = match self.iter.peek() {
            Some(chr) => *chr,
            None => return Ok(None),
        };

        if next_chr == '(' {
            self.next_chr();
            Ok(Some(Token::Lparen))
        } else if next_chr == ')' {
            self.next_chr();
            Ok(Some(Token::Rparen))
        } else if next_chr.is_numeric() {
            self.get_integer().map(Some)
        } else {
            Ok(Some(self.get_symbol()))
        }
    }

    fn dump_whitespace(&mut self) {
        while let Some(chr) = self.iter.peek() {
            if chr.is_whitespace() {
                self.next_chr();
            } else {
                return;
            }
        }
    }

    fn at_delimiter(&mut self) -> bool {
        match self.iter.peek() {
            None => true,
            Some(&chr) => chr.is_whitespace() || chr == '(' || chr == ')',
        }
    }

    fn get_integer(&mut self) -> Result<Token, Error> {
        let mut val = 0;

        loop {
            if self.at_delimiter() {
                return Ok(Token::Integer(val));
            }

            let next_digit = self.next_chr().unwrap().to_digit(10).ok_or(Error {
                line_number: self.line_number,
                column_number: self.column_number - 1,
                error_message: INVALID_INTEGER_ERROR,
            })?;

            val = val * 10 + (next_digit as i64);
        }
    }

    fn get_symbol(&mut self) -> Token {
        let mut val = String::new();

        loop {
            if self.at_delimiter() {
                return Token::Symbol(val);
            }

            val.push(self.next_chr().unwrap());
        }
    }
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, Error> {
    let mut lexer = Lexer::new(source);
    let mut res = Vec::new();

    while let Some(token) = lexer.next()? {
        res.push(token)
    }

    Ok(res)
}

#[cfg(test)]
mod test {
    use super::*;
    use Token::*;

    #[test]
    fn generates_correct_symbols() {
        let source = "(+ 1 21)";
        let tokens = tokenize(source).unwrap();
        assert_eq!(
            tokens,
            vec![
                Lparen,
                Symbol("+".to_string()),
                Integer(1),
                Integer(21),
                Rparen
            ]
        );
    }

    #[test]
    fn generates_error() {
        let source = "hello\n12abc";
        let res = tokenize(source);
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert_eq!(err.line_number, 1);
        assert_eq!(err.column_number, 2);
    }
}
