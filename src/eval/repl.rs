use super::env::MutEnvironment;
use std::io::{BufRead, StdinLock, StdoutLock, Write};
use std::rc::Rc;

pub fn repl() {
    fn repl_rec(env: Rc<MutEnvironment>, mut stdin: StdinLock, mut stdout: StdoutLock) {
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
                return repl_rec(env, stdin, stdout);
            }
        };

        let nodes = match crate::parse::parse(&tokens) {
            Ok(nodes) => nodes,
            Err(err) => {
                eprintln!("{}", err.to_string());
                return repl_rec(env, stdin, stdout);
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
                repl_rec(env, stdin, stdout);
            }),
        )
    }

    let env = super::stdlib::build();
    let stdin = std::io::stdin();
    let stdin = stdin.lock();
    let stdout = std::io::stdout();
    let stdout = stdout.lock();
    repl_rec(env, stdin, stdout);
}
