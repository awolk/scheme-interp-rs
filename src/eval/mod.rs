mod stdlib;

use crate::parse::AST;
use std::collections::HashMap;
use std::rc::Rc;

pub struct Function {
    args: Vec<String>,
    env: Environment,
    body: AST,
}

pub enum Value {
    Integer(i64),
    Bool(bool),
    Function(Function),
    NativeFunction(fn(&[Rc<Value>]) -> Result<Rc<Value>, Error>),
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

type Continuation = Box<dyn FnOnce(Result<Rc<Value>, Error>)>;

fn eval_node(node: AST, env: Rc<Environment>, cont: Continuation) {
    match node {
        AST::Integer(i) => cont(Ok(Value::Integer(i).rc())),

        AST::Bool(b) => cont(Ok(Value::Bool(b).rc())),

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
                mut nodes: std::vec::IntoIter<AST>,
                env: Rc<Environment>,
                cont: Continuation,
            ) {
                match nodes.next() {
                    None => {
                        match vals[0].as_ref() {
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
                    Some(node) => eval_node(
                        node,
                        Rc::clone(&env),
                        Box::new(move |arg| match arg {
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
            node,
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
}
