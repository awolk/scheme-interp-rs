use super::env::MutEnvironment;
use std::io::{stdin, stdout, Write};
use std::rc::Rc;

pub fn repl() {
    fn repl_rec(env: Rc<MutEnvironment>) {
        let mut stdout = stdout();
        if stdout.write_all(b"> ").is_err() {
            return;
        }
        if stdout.flush().is_err() {
            return;
        }

        let mut line = String::new();
        match stdin().read_line(&mut line) {
            Err(_) => return,
            Ok(0) => return,
            _ => {}
        }

        let tokens = match crate::lex::tokenize(&line) {
            Ok(tokens) => tokens,
            Err(err) => {
                eprintln!("{}", err.to_string());
                return repl_rec(env);
            }
        };

        let nodes = match crate::parse::parse(&tokens) {
            Ok(nodes) => nodes,
            Err(err) => {
                eprintln!("{}", err.to_string());
                return repl_rec(env);
            }
        };

        super::eval_nodes_toplevel(
            nodes,
            Rc::<MutEnvironment>::clone(&env),
            Box::new(move |res| {
                match res {
                    Err(err) => eprintln!("Error: {}", err.to_string()),
                    Ok(val) => println!("{}", val.to_string()),
                }
                repl_rec(env);
            }),
        )
    }

    let env = super::stdlib::build();
    repl_rec(env);
}
