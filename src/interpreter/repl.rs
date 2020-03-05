use crate::interpreter::Interpreter;
use rustyline::Editor;

pub fn repl() {
    let mut interp = Interpreter::new();
    let env = super::stdlib::build(&mut interp.alloc);
    let mut editor = Editor::<()>::new();

    loop {
        let line = match editor.readline("> ") {
            Ok(line) => line,
            Err(_) => return,
        };
        editor.add_history_entry(&line);

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
