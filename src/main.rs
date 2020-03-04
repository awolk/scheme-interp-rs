use crate::interpreter::repl::repl;

mod interpreter;
mod lex;
mod parse;

fn main() {
    repl();
}
