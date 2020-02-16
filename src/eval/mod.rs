mod stdlib;

use crate::parse::AST;
use std::collections::HashMap;

#[derive(Clone)]
pub struct Function {
    args: Vec<String>,
    env: Environment,
    body: AST,
}

#[derive(Clone)]
pub enum Value {
    Integer(i64),
    Bool(bool),
    Function(Function),
    NativeFunction(fn(&[Value]) -> Result<Value, Error>),
}

#[derive(Clone)]
struct Environment {
    parent: Option<Box<Environment>>,
    data: HashMap<String, Value>,
}

impl Environment {
    fn new() -> Self {
        Environment {
            parent: None,
            data: HashMap::new(),
        }
    }

    fn new_child(&self) -> Self {
        Environment {
            parent: Some(Box::new(self.clone())),
            data: HashMap::new(),
        }
    }

    fn get(&self, key: &str) -> Option<Value> {
        self.data
            .get(key)
            .cloned()
            .or_else(|| self.parent.as_ref().and_then(|parent| parent.get(key)))
    }

    fn set(&mut self, key: String, value: Value) {
        self.data.insert(key, value);
    }
}

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

type Continuation = Box<dyn FnOnce(Result<Value, Error>)>;

fn eval_node(node: AST, env: &Environment, cont: Continuation) {
    match node {
        AST::Integer(i) => cont(Ok(Value::Integer(i))),

        AST::Bool(b) => cont(Ok(Value::Bool(b))),

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
                mut vals: Vec<Value>,
                mut nodes: std::vec::IntoIter<AST>,
                env: &Environment,
                cont: Continuation,
            ) {
                match nodes.next() {
                    None => {
                        match &vals[0] {
                            Value::Function(Function { args, env, body }) => unimplemented!(),
                            Value::NativeFunction(f) => match f(&vals[1..]) {
                                Ok(val) => cont(Ok(val)),
                                Err(err) => cont(Err(err)),
                            },
                            _ => cont(Err(Error {
                                error_message: INVALID_FUNCTION_ERROR.to_string(),
                            })),
                        };
                    }
                    Some(node) => {
                        let env_clone = env.clone();
                        eval_node(
                            node,
                            env,
                            Box::new(move |arg| match arg {
                                Ok(val) => {
                                    vals.push(val);
                                    eval_nodes(vals, nodes, &env_clone, cont);
                                }
                                Err(err) => cont(Err(err)),
                            }),
                        )
                    }
                }
            }

            let vals = Vec::with_capacity(nodes.len());
            eval_nodes(vals, nodes.into_iter(), env, cont);
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

        let mut env = stdlib::build();

        eval_node(
            node,
            &env,
            Box::new(|res| {
                assert!(res.is_ok());

                if let Ok(Value::Integer(i)) = res {
                    assert_eq!(i, 3);
                } else {
                    panic!("result should have type Integer")
                }
            }),
        );
    }
}
