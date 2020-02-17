mod eval;
mod lex;
mod parse;

fn main() {
    let source = "(+ 1 2)";
    let tokens = match lex::tokenize(source) {
        Ok(tokens) => tokens,
        Err(err) => {
            eprintln!("{}", err.to_string());
            return;
        }
    };
    let nodes = match parse::parse(&tokens) {
        Ok(nodes) => nodes,
        Err(err) => {
            eprintln!("{}", err.to_string());
            return;
        }
    };
    eval::eval_program(
        nodes,
        Box::new(|res| match res {
            Err(err) => eprintln!("{}", err.to_string()),
            Ok(val) => println!("=> {}", val.to_string()),
        }),
    );
}
