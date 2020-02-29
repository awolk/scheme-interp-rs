mod interpreter;
mod lex;
mod parse;

fn main() {
    interpreter::repl::repl();
}
