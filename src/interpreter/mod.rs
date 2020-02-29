mod env;
pub mod repl;
mod stdlib;
mod value;

use self::value::*;
use crate::interpreter::env::MutEnvironment;
use crate::parse::AST;
use env::{Environment, NestedEnvironment};
use std::borrow::Borrow;
use std::rc::Rc;

#[derive(Debug)]
pub struct Error {
    pub message: String,
}

impl ToString for Error {
    fn to_string(&self) -> String {
        format!("Runtime error: {}", self.message)
    }
}

const UNBOUND_SYMBOL_ERROR: &str = "unbound symbol";
const EVAL_EMPTY_LIST_ERROR: &str = "cannot evaluate empty list";
const EVAL_BAD_LIST_ERROR: &str = "attempt to evaluate malformed list";
const INVALID_FUNCTION_ERROR: &str = "attempt to call a non-function value";
const WRONG_NUMBER_ARGS_ERROR: &str = "wrong number of arguments";
const INVALID_IF_ERROR: &str = "invalid structure for if expression";
const INVALID_LAMBDA_ERROR: &str = "invalid structure for lambda expression";
const INVALID_DEFINE_ERROR: &str = "invalid structure for define expression";

type Continuation = Box<dyn FnOnce(Result<Rc<Value>, Error>)>;

// returns None if malformed list
fn cons_list_to_vector(head: Rc<Value>, tail: Rc<Value>) -> Option<Vec<Rc<Value>>> {
    let mut res = Vec::new();
    res.push(head);

    let mut ptr = &tail;
    while let Value::Cons(hd, tl) = ptr.borrow() {
        res.push(Rc::clone(hd));
        ptr = tl;
    }

    if let Value::Nil = ptr.borrow() {
        Some(res)
    } else {
        // error if last non-list value isn't nil
        None
    }
}

fn eval_node(node: Rc<Value>, env: Rc<dyn Environment>, cont: Continuation) {
    match node.borrow() {
        Value::Integer(_) => cont(Ok(node)),
        Value::Bool(_) => cont(Ok(node)),
        Value::NativeFunction(_) => cont(Ok(node)),
        Value::Function(_) => cont(Ok(node)),

        Value::Symbol(s) => match env.get(s) {
            None => cont(Err(Error {
                message: format!("{}: {}", UNBOUND_SYMBOL_ERROR, s),
            })),
            Some(val) => cont(Ok(val)),
        },

        Value::Nil => cont(Err(Error {
            message: EVAL_EMPTY_LIST_ERROR.to_string(),
        })),

        Value::Cons(hd, tl) => {
            let nodes = match cons_list_to_vector(Rc::clone(hd), Rc::clone(tl)) {
                Some(nodes) => nodes,
                None => {
                    return cont(Err(Error {
                        message: EVAL_BAD_LIST_ERROR.to_string(),
                    }))
                }
            };

            if nodes.is_empty() {
                return cont(Err(Error {
                    message: EVAL_EMPTY_LIST_ERROR.to_string(),
                }));
            }

            // handle special forms
            if let Value::Symbol(first_sym) = &nodes[0].borrow() {
                match first_sym.as_str() {
                    "if" => {
                        if nodes.len() != 4 {
                            return cont(Err(Error {
                                message: INVALID_IF_ERROR.to_string(),
                            }));
                        }

                        return eval_node(
                            Rc::clone(&nodes[1]),
                            Rc::clone(&env),
                            Box::new(move |res| {
                                if res.is_err() {
                                    cont(res);
                                    return;
                                }

                                let res = res.unwrap();
                                if let Value::Bool(true) = res.as_ref() {
                                    eval_node(Rc::clone(&nodes[2]), env, cont)
                                } else {
                                    eval_node(Rc::clone(&nodes[3]), env, cont)
                                }
                            }),
                        );
                    }
                    "lambda" => {
                        if nodes.len() != 3 {
                            return cont(Err(Error {
                                message: INVALID_LAMBDA_ERROR.to_string(),
                            }));
                        }

                        let mut args_names: Vec<String>;
                        if let Value::Cons(al_hd, al_tl) = &nodes[1].borrow() {
                            let arg_list =
                                match cons_list_to_vector(Rc::clone(al_hd), Rc::clone(al_tl)) {
                                    Some(al) => al,
                                    None => {
                                        return cont(Err(Error {
                                            message: INVALID_LAMBDA_ERROR.to_string(),
                                        }))
                                    }
                                };

                            args_names = Vec::with_capacity(arg_list.len());
                            for arg in arg_list {
                                if let Value::Symbol(arg) = arg.borrow() {
                                    args_names.push(arg.clone());
                                } else {
                                    return cont(Err(Error {
                                        message: INVALID_LAMBDA_ERROR.to_string(),
                                    }));
                                }
                            }
                        } else {
                            return cont(Err(Error {
                                message: INVALID_LAMBDA_ERROR.to_string(),
                            }));
                        }

                        return cont(Ok(Value::Function(Function {
                            args: args_names,
                            env: Rc::clone(&env),
                            body: Rc::clone(&nodes[2]),
                        })
                        .rc()));
                    }
                    _ => {}
                }
            }

            // evaluate function
            fn eval_nodes(
                mut vals: Vec<Rc<Value>>,
                mut nodes: std::vec::IntoIter<Rc<Value>>,
                env: Rc<dyn Environment>,
                cont: Continuation,
            ) {
                match nodes.next() {
                    None => {
                        let mut vals = vals.into_iter();
                        let func = vals.next().unwrap();

                        match func.as_ref() {
                            Value::Function(Function { args, env, body }) => {
                                if args.len() != vals.len() {
                                    cont(Err(Error {
                                        message: format!(
                                            "{}: expected {}, received {}",
                                            WRONG_NUMBER_ARGS_ERROR,
                                            args.len(),
                                            vals.len() - 1
                                        ),
                                    }));
                                    return;
                                }

                                let new_bindings =
                                    args.iter().map(String::clone).zip(vals).collect();
                                let bound_env =
                                    NestedEnvironment::new_child_with_bindings(env, new_bindings);

                                eval_node(Rc::clone(body), bound_env, cont)
                            }
                            Value::NativeFunction(f) => f(vals.as_slice(), cont),
                            _ => cont(Err(Error {
                                message: INVALID_FUNCTION_ERROR.to_string(),
                            })),
                        };
                    }
                    Some(node) => eval_node(
                        node,
                        Rc::clone(&env),
                        Box::new(|arg| match arg {
                            Ok(val) => {
                                vals.push(val);
                                eval_nodes(vals, nodes, env, cont);
                            }
                            Err(err) => cont(Err(err)),
                        }),
                    ),
                }
            }

            let vals = Vec::with_capacity(nodes.len());
            eval_nodes(vals, nodes.into_iter(), env, cont);
        }
    }
}

fn eval_toplevel(node: AST, env: Rc<MutEnvironment>, cont: Continuation) {
    // special handler for top-level node
    // recognizes define syntax
    if let AST::List(nodes) = &node {
        if let Some(AST::Symbol(first_sym)) = nodes.first() {
            if first_sym == "define" {
                if nodes.len() != 3 {
                    return cont(Err(Error {
                        message: INVALID_DEFINE_ERROR.to_string(),
                    }));
                }

                // move nodes if we have a valid define form
                let nodes = match node {
                    AST::List(nodes) => nodes,
                    _ => unreachable!(),
                };

                let mut iter = nodes.into_iter();
                iter.next(); // drop define

                let name = match iter.next().unwrap() {
                    AST::Symbol(s) => s,
                    _ => {
                        return cont(Err(Error {
                            message: INVALID_DEFINE_ERROR.to_string(),
                        }))
                    }
                };

                return eval_node(
                    Value::from(iter.next().unwrap()).rc(),
                    Rc::<MutEnvironment>::clone(&env),
                    Box::new(move |res| match res {
                        Err(err) => cont(Err(err)),
                        Ok(val) => {
                            MutEnvironment::set(&env, name, val);
                            cont(Ok(Value::Nil.rc()))
                        }
                    }),
                );
            }
        }
    }

    eval_node(Value::from(node).rc(), env, cont);
}

pub fn eval_nodes_toplevel(nodes: Vec<AST>, env: Rc<MutEnvironment>, cont: Continuation) {
    fn eval_program_rec(
        mut nodes: std::vec::IntoIter<AST>,
        env: Rc<MutEnvironment>,
        cont: Continuation,
    ) {
        let node: AST;
        if let Some(next) = nodes.next() {
            node = next;
        } else {
            return cont(Ok(Value::Nil.rc()));
        }

        eval_toplevel(
            node,
            Rc::<MutEnvironment>::clone(&env),
            Box::new(|res| match res {
                Err(err) => cont(Err(err)),
                Ok(val) => {
                    if nodes.len() == 0 {
                        // return last value
                        cont(Ok(val))
                    } else {
                        eval_program_rec(nodes, env, cont)
                    }
                }
            }),
        );
    }

    eval_program_rec(nodes.into_iter(), env, cont);
}

#[cfg(test)]
mod test {
    use super::*;
    use std::cell::{Cell, RefCell};

    #[test]
    fn runs_simple_example() {
        let node = AST::List(vec![
            AST::Symbol("+".to_string()),
            AST::Integer(1),
            AST::Integer(2),
        ]);

        let env = stdlib::build();
        let cont_called = Rc::new(Cell::new(false));
        let ccb = cont_called.clone();

        eval_node(
            Value::from(node).rc(),
            env,
            Box::new(move |res| {
                let res = res.unwrap();
                match res.borrow() {
                    Value::Integer(3) => {}
                    _ => panic!("expected 3"),
                };
                ccb.set(true);
            }),
        );

        assert!(cont_called.get());
    }

    #[test]
    fn handles_if() {
        // (if #t (if #f 1 2) 3) -> 2
        let node = AST::List(vec![
            AST::Symbol("if".to_string()),
            AST::Bool(true),
            AST::List(vec![
                AST::Symbol("if".to_string()),
                AST::Bool(false),
                AST::Integer(1),
                AST::Integer(2),
            ]),
            AST::Integer(3),
        ]);

        let env = stdlib::build();
        let cont_called = Rc::new(Cell::new(false));
        let ccb = cont_called.clone();

        eval_node(
            Value::from(node).rc(),
            env,
            Box::new(move |res| {
                let res = res.unwrap();
                match res.borrow() {
                    Value::Integer(2) => {}
                    _ => panic!("expected 2"),
                };
                ccb.set(true);
            }),
        );

        assert!(cont_called.get());
    }

    #[test]
    fn handles_lambda() {
        // ((lambda (x) (+ x 1)) 2) -> 3
        let node = AST::List(vec![
            AST::List(vec![
                AST::Symbol("lambda".to_string()),
                AST::List(vec![AST::Symbol("x".to_string())]),
                AST::List(vec![
                    AST::Symbol("+".to_string()),
                    AST::Symbol("x".to_string()),
                    AST::Integer(1),
                ]),
            ]),
            AST::Integer(2),
        ]);

        let env = stdlib::build();
        let cont_called = Rc::new(Cell::new(false));
        let ccb = cont_called.clone();
        eval_node(
            Value::from(node).rc(),
            env,
            Box::new(move |res| {
                let res = res.unwrap();
                match res.borrow() {
                    Value::Integer(3) => {}
                    _ => panic!("expected 2"),
                };
                ccb.set(true);
            }),
        );
        assert!(cont_called.get())
    }

    #[test]
    fn runs_program() {
        // (define x 1) x
        let program = vec![
            AST::List(vec![
                AST::Symbol("define".to_string()),
                AST::Symbol("x".to_string()),
                AST::Integer(1),
            ]),
            AST::Symbol("x".to_string()),
        ];

        let cont_called = Rc::new(Cell::new(false));
        let ccb = cont_called.clone();

        eval_nodes_toplevel(
            program,
            stdlib::build(),
            Box::new(move |res| {
                let res = res.unwrap();
                if let Value::Integer(i) = res.as_ref() {
                    assert_eq!(*i, 1);
                } else {
                    panic!("expected integer result");
                }
                ccb.set(true);
            }),
        );
        assert!(cont_called.get());
    }
}
