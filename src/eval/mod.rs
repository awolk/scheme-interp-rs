mod stdlib;

use crate::parse::AST;
use std::collections::HashMap;
use std::rc::Rc;

pub struct Function {
    args: Vec<String>,
    env: Rc<Environment>,
    body: AST,
}

pub enum Value {
    Integer(i64),
    Bool(bool),
    Function(Function),
    NativeFunction(fn(&[Rc<Value>], Continuation)),
}

impl Value {
    fn rc(self) -> Rc<Self> {
        Rc::new(self)
    }
}

struct Environment {
    parent: Option<Rc<Environment>>,
    bindings: HashMap<String, Rc<Value>>,
}

impl Environment {
    fn new_with_bindings(bindings: HashMap<String, Rc<Value>>) -> Rc<Self> {
        Rc::new(Environment {
            parent: None,
            bindings,
        })
    }

    fn new_child_with_bindings(
        parent: &Rc<Self>,
        bindings: HashMap<String, Rc<Value>>,
    ) -> Rc<Self> {
        Rc::new(Environment {
            parent: Some(Rc::clone(parent)),
            bindings,
        })
    }

    fn get(&self, key: &str) -> Option<Rc<Value>> {
        self.bindings
            .get(key)
            .cloned()
            .or_else(|| self.parent.as_ref().and_then(|parent| parent.get(key)))
    }
}

#[derive(Debug)]
pub struct Error {
    pub error_message: String,
}

impl ToString for Error {
    fn to_string(&self) -> String {
        format!("Runtime error: {}", self.error_message)
    }
}

const UNBOUND_SYMBOL_ERROR: &str = "unbound symbol";
const EVAL_EMPTY_LIST_ERROR: &str = "cannot evaluate empty list";
const INVALID_FUNCTION_ERROR: &str = "attempt to call a non-function value";
const WRONG_NUMBER_ARGS_ERROR: &str = "wrong number of arguments";

type Continuation<'a> = Box<dyn FnOnce(Result<Rc<Value>, Error>) + 'a>;

fn eval_node<'a>(node: &'a AST, env: Rc<Environment>, cont: Continuation<'a>) {
    match node {
        AST::Integer(i) => cont(Ok(Value::Integer(*i).rc())),

        AST::Bool(b) => cont(Ok(Value::Bool(*b).rc())),

        AST::Symbol(s) => match env.get(&s) {
            None => cont(Err(Error {
                error_message: format!("{}: {}", UNBOUND_SYMBOL_ERROR, s),
            })),
            Some(val) => cont(Ok(val)),
        },

        AST::List(nodes) => {
            if nodes.is_empty() {
                cont(Err(Error {
                    error_message: EVAL_EMPTY_LIST_ERROR.to_string(),
                }));
                return;
            }

            fn eval_nodes(
                mut vals: Vec<Rc<Value>>,
                mut nodes: std::slice::Iter<AST>,
                env: Rc<Environment>,
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
                                        error_message: format!(
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
                                    Environment::new_child_with_bindings(env, new_bindings);

                                eval_node(body, bound_env, cont)
                            }
                            Value::NativeFunction(f) => f(vals.as_slice(), cont),
                            _ => cont(Err(Error {
                                error_message: INVALID_FUNCTION_ERROR.to_string(),
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn runs_simple_example() {
        let node = AST::List(vec![
            AST::Symbol("+".to_string()),
            AST::Integer(1),
            AST::Integer(2),
        ]);

        let env = stdlib::build();

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
            }),
        );
    }

    #[test]
    fn executes_function() {
        fn gen_fun(args: &[Rc<Value>], cont: Continuation) {
            cont(Ok(Value::Function(Function {
                args: vec![],
                env: Environment::new_with_bindings(HashMap::new()),
                body: AST::Bool(true),
            })
            .rc()));
        }
        let mut bindings = HashMap::new();
        bindings.insert("gen-fun".to_string(), Value::NativeFunction(gen_fun).rc());
        let env = Environment::new_with_bindings(bindings);

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
            }),
        )
    }
}
