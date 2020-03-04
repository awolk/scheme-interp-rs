use crate::interpreter::Interpreter;
use std::io::{stdin, stdout, BufRead, Write};

pub fn repl() {
    let mut interp = Interpreter::new();
    let env = super::stdlib::build(&mut interp.alloc);
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let stdin = stdin();
    let mut stdin = stdin.lock();

    loop {
        if stdout.write_all(b"> ").is_err() {
            return;
        }
        if stdout.flush().is_err() {
            return;
        }

        let mut line = String::new();
        match stdin.read_line(&mut line) {
            Err(_) => return,
            Ok(0) => return,
            _ => {}
        }

        let tokens = match crate::lex::tokenize(&line) {
            Ok(tokens) => tokens,
            Err(err) => {
                eprintln!("{}", err.to_string());
                continue;
            }
        };

        let nodes = match crate::parse::parse(&tokens) {
            Ok(nodes) => nodes,
            Err(err) => {
                eprintln!("{}", err.to_string());
                continue;
            }
        };

        for node in nodes {
            interp.eval_ast(node, env);
            match interp.run() {
                Err(err) => eprintln!("Error: {}", err.to_string()),
                Ok(val) => println!("{}", interp.alloc.get_val(val).to_string(&interp.alloc)),
            }
        }
    }
}
