mod eval;
mod lex;
mod parse;

fn main() {
    eval::repl::repl();
}
