mod env;
mod stdlib;

use crate::eval::env::MutEnvironment;
use crate::parse::AST;
use env::{Environment, NestedEnvironment};
use std::collections::HashMap;
use std::rc::Rc;

pub struct Function {
    args: Vec<String>,
    env: Rc<dyn Environment>,
    body: AST,
}

pub enum Value {
    Integer(i64),
    Bool(bool),
    Function(Function),
    NativeFunction(fn(&[Rc<Value>], Continuation)),
    Unit,
}

impl Value {
    fn rc(self) -> Rc<Self> {
        Rc::new(self)
    }
}

impl ToString for Value {
    fn to_string(&self) -> String {
        match self {
            Value::Integer(i) => i.to_string(),
            Value::Bool(b) => (if *b { "#t" } else { "#f" }).to_string(),
            Value::Function(_f) => "<lisp function>".to_string(),
            Value::NativeFunction(_f) => "<native function>".to_string(),
            Value::Unit => "<unit>".to_string(),
        }
    }
}

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
const INVALID_FUNCTION_ERROR: &str = "attempt to call a non-function value";
const WRONG_NUMBER_ARGS_ERROR: &str = "wrong number of arguments";
const INVALID_IF_ERROR: &str = "invalid structure for if expression";
const INVALID_LAMBDA_ERROR: &str = "invalid structure for lambda expression";
const INVALID_DEFINE_ERROR: &str = "invalid structure for define expression";

type Continuation<'a> = Box<dyn FnOnce(Result<Rc<Value>, Error>) + 'a>;

fn eval_node<'a>(node: &'a AST, env: Rc<dyn Environment>, cont: Continuation<'a>) {
    match node {
        AST::Integer(i) => cont(Ok(Value::Integer(*i).rc())),

        AST::Bool(b) => cont(Ok(Value::Bool(*b).rc())),

        AST::Symbol(s) => match env.get(&s) {
            None => cont(Err(Error {
                message: format!("{}: {}", UNBOUND_SYMBOL_ERROR, s),
            })),
            Some(val) => cont(Ok(val)),
        },

        AST::List(nodes) => {
            if nodes.is_empty() {
                cont(Err(Error {
                    message: EVAL_EMPTY_LIST_ERROR.to_string(),
                }));
                return;
            }

            // handle special forms
            if let AST::Symbol(first_sym) = &nodes[0] {
                match first_sym.as_str() {
                    "if" => {
                        if nodes.len() != 4 {
                            cont(Err(Error {
                                message: INVALID_IF_ERROR.to_string(),
                            }));
                            return;
                        }

                        eval_node(
                            &nodes[1],
                            Rc::clone(&env),
                            Box::new(move |res| {
                                if res.is_err() {
                                    cont(res);
                                    return;
                                }

                                let res = res.unwrap();
                                if let Value::Bool(true) = res.as_ref() {
                                    eval_node(&nodes[2], env, cont)
                                } else {
                                    eval_node(&nodes[3], env, cont)
                                }
                            }),
                        );
                        return;
                    }
                    "lambda" => {
                        if nodes.len() != 3 {
                            cont(Err(Error {
                                message: INVALID_LAMBDA_ERROR.to_string(),
                            }));
                            return;
                        }

                        let mut args_names: Vec<String>;
                        if let AST::List(arg_list) = &nodes[1] {
                            args_names = Vec::with_capacity(arg_list.len());
                            for arg in arg_list {
                                if let AST::Symbol(arg) = arg {
                                    args_names.push(arg.clone());
                                } else {
                                    cont(Err(Error {
                                        message: INVALID_LAMBDA_ERROR.to_string(),
                                    }));
                                    return;
                                }
                            }
                        } else {
                            cont(Err(Error {
                                message: INVALID_LAMBDA_ERROR.to_string(),
                            }));
                            return;
                        }

                        cont(Ok(Value::Function(Function {
                            args: args_names,
                            env: Rc::clone(&env),
                            // TODO: can this clone be removed?
                            body: nodes[2].clone(),
                        })
                        .rc()));
                        return;
                    }
                    _ => {}
                }
            }

            // evaluate function
            fn eval_nodes(
                mut vals: Vec<Rc<Value>>,
                mut nodes: std::slice::Iter<AST>,
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

                                eval_node(body, bound_env, cont)
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
            eval_nodes(vals, nodes.iter(), env, cont);
        }
    }
}

fn eval_toplevel<'a>(node: &'a AST, env: Rc<MutEnvironment>, cont: Continuation<'a>) {
    // special handler for top-level node
    // recognizes define syntax
    if let AST::List(nodes) = node {
        if let Some(AST::Symbol(first_sym)) = nodes.first() {
            if first_sym == "define" {
                if nodes.len() != 3 {
                    return cont(Err(Error {
                        message: INVALID_DEFINE_ERROR.to_string(),
                    }));
                }

                let name: String;
                if let AST::Symbol(s) = &nodes[1] {
                    name = s.clone();
                } else {
                    return cont(Err(Error {
                        message: INVALID_DEFINE_ERROR.to_string(),
                    }));
                }

                return eval_node(
                    &nodes[2],
                    Rc::<MutEnvironment>::clone(&env),
                    Box::new(|res| match res {
                        Err(err) => cont(Err(err)),
                        Ok(val) => {
                            MutEnvironment::set(&env, name, val);
                            cont(Ok(Value::Unit.rc()))
                        }
                    }),
                );
            }
        }
    }

    eval_node(node, env, cont);
}

pub fn eval_program(nodes: Vec<AST>, cont: Continuation) {
    fn eval_program_rec(
        mut nodes: std::vec::IntoIter<AST>,
        env: Rc<MutEnvironment>,
        cont: Continuation,
    ) {
        let node: AST;
        if let Some(next) = nodes.next() {
            node = next;
        } else {
            return cont(Ok(Value::Unit.rc()));
        }

        eval_toplevel(
            &node,
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

    let env = stdlib::build();
    eval_program_rec(nodes.into_iter(), env, cont);
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::eval::env::NestedEnvironment;

    #[test]
    fn runs_simple_example() {
        let node = AST::List(vec![
            AST::Symbol("+".to_string()),
            AST::Integer(1),
            AST::Integer(2),
        ]);

        let env = stdlib::build();

        let mut cont_called = Box::new(false);

        eval_node(
            &node,
            env,
            Box::new(|res| {
                let res = res.unwrap();
                if let Value::Integer(i) = res.as_ref() {
                    assert_eq!(*i, 3);
                } else {
                    panic!("expected integer result");
                }
                *cont_called = true;
            }),
        );

        assert!(*cont_called);
    }

    #[test]
    fn executes_function() {
        fn gen_fun(args: &[Rc<Value>], cont: Continuation) {
            cont(Ok(Value::Function(Function {
                args: vec![],
                env: NestedEnvironment::new_with_bindings(HashMap::new()),
                body: AST::Bool(true),
            })
            .rc()));
        }
        let mut bindings = HashMap::new();
        bindings.insert("gen-fun".to_string(), Value::NativeFunction(gen_fun).rc());
        let env = NestedEnvironment::new_with_bindings(bindings);

        let mut cont_called = Box::new(false);

        // ((gen-fun))
        let node = AST::List(vec![AST::List(vec![AST::Symbol("gen-fun".to_string())])]);
        eval_node(
            &node,
            env,
            Box::new(|res| {
                let res = res.unwrap();
                if let Value::Bool(b) = res.as_ref() {
                    assert_eq!(*b, true);
                } else {
                    panic!("expected boolean result");
                }
                *cont_called = true;
            }),
        );

        assert!(*cont_called)
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

        let env = NestedEnvironment::new_with_bindings(HashMap::new());
        let mut cont_called = Box::new(false);

        eval_node(
            &node,
            env,
            Box::new(|res| {
                let res = res.unwrap();
                if let Value::Integer(i) = res.as_ref() {
                    assert_eq!(*i, 2);
                } else {
                    panic!("expected integer result");
                }
                *cont_called = true;
            }),
        );

        assert!(*cont_called);
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
        let mut cont_called = Box::new(false);
        eval_node(
            &node,
            env,
            Box::new(|res| {
                let res = res.unwrap();
                if let Value::Integer(i) = res.as_ref() {
                    assert_eq!(*i, 3);
                } else {
                    panic!("expected integer result");
                }
                *cont_called = true;
            }),
        );
        assert!(*cont_called)
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

        let mut cont_called = Box::new(false);
        eval_program(
            program,
            Box::new(|res| {
                let res = res.unwrap();
                if let Value::Integer(i) = res.as_ref() {
                    assert_eq!(*i, 1);
                } else {
                    panic!("expected integer result");
                }
                *cont_called = true;
            }),
        );
        assert!(*cont_called);
    }
}
