mod lex;

fn main() {
    let source = "(+ 1 2)";
    let tokens = match lex::tokenize(source) {
        Ok(tokens) => tokens,
        Err(err) => {
            eprintln!("{}", err.to_string());
            return;
        }
    };
    dbg!(tokens);
}
