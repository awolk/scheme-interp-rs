mod allocator;
pub mod repl;
mod stdlib;
mod value;

use self::value::*;
use crate::interpreter::allocator::{Allocator, Environment, Ptr};
use crate::parse::AST;

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
const WRONG_NUMBER_ARGS_ERROR: &str = "wrong number of arguments";
const INVALID_IF_ERROR: &str = "invalid structure for if expression";
const INVALID_LAMBDA_ERROR: &str = "invalid structure for lambda expression";
const INVALID_DEFINE_ERROR: &str = "invalid structure for define expression";
const INVALID_QUOTE_ERROR: &str = "invalid structure for quote expression";

trait StepTrait: FnOnce(&mut Interpreter) {
    fn clone_box(&self) -> Step;
}

impl<T: 'static + FnOnce(&mut Interpreter) + Clone> StepTrait for T {
    fn clone_box(&self) -> Step {
        Box::new(self.clone())
    }
}

type Step = Box<dyn StepTrait>;

pub struct Interpreter {
    alloc: Allocator,
    next_steps: Vec<Step>,
    results: Vec<Ptr<Value>>,
    saved_results: Vec<Vec<Ptr<Value>>>,
    error: Option<Error>,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            alloc: Allocator::new(),
            next_steps: Vec::new(),
            results: Vec::new(),
            saved_results: Vec::new(),
            error: None,
        }
    }

    fn clear_run_state(&mut self) {
        self.next_steps = Vec::new();
        self.results = Vec::new();
        self.saved_results = Vec::new();
        self.error = None;
    }

    // returns None if malformed list
    fn cons_list_to_vector(&self, head: Ptr<Value>, tail: Ptr<Value>) -> Option<Vec<Ptr<Value>>> {
        let mut res = Vec::new();
        res.push(head);

        let mut ptr = tail;
        while let Value::Cons(hd, tl) = self.alloc.get_val(ptr) {
            res.push(*hd);
            ptr = *tl;
        }

        if let Value::Nil = self.alloc.get_val(ptr) {
            Some(res)
        } else {
            // error if last non-list value isn't nil
            None
        }
    }

    fn handle_func_call(&mut self, nodes: Vec<Ptr<Value>>, env: Ptr<Environment>) {
        self.saved_results
            .push(std::mem::replace(&mut self.results, Vec::new()));

        self.next_steps.push(Box::new(move |interp| {
            let mut vals =
                std::mem::replace(&mut interp.results, interp.saved_results.pop().unwrap())
                    .into_iter();
            let func = vals.next().unwrap();
            let func_val = interp.alloc.get_val(func);
            match func_val {
                Value::Function(Function { args, env, body }) => {
                    if args.len() != vals.len() {
                        interp.error = Some(Error {
                            message: format!(
                                "{}: expected {}, received {}",
                                WRONG_NUMBER_ARGS_ERROR,
                                args.len(),
                                vals.len()
                            ),
                        });
                        return;
                    }

                    let body = *body;
                    let new_bindings = args.iter().map(String::clone).zip(vals).collect();
                    let bound_env = Environment::new_child_with_bindings(*env, new_bindings);
                    let bound_env_ptr = interp.alloc.new_env(bound_env);

                    interp.eval_node(body, bound_env_ptr)
                }
                Value::NativeFunction(f) => {
                    // test
                    f(interp, env, vals.as_slice())
                }
                Value::Continuation(c) => {
                    if vals.len() != 1 {
                        interp.error = Some(Error {
                            message: "continuation must be called with 1 argument".to_string(),
                        });
                        return;
                    }

                    interp.next_steps = clone_steps(&c.next_steps);
                    interp.results = c.results.clone();
                    interp.saved_results = c.saved_results.clone();
                    interp.results.push(vals.next().unwrap());
                }
                _ => {
                    interp.error = Some(Error {
                        message: format!(
                            "attempt to call a non-function value: {}",
                            func_val.to_string(&interp.alloc)
                        ),
                    });
                }
            };
        }));

        for node in nodes.into_iter().rev() {
            self.next_steps.push(Box::new(move |interp| {
                interp.eval_node(node, env);
            }))
        }
    }

    fn eval_node(&mut self, node: Ptr<Value>, env: Ptr<Environment>) {
        match self.alloc.get_val(node) {
            Value::Integer(_) => self.results.push(node),
            Value::Bool(_) => self.results.push(node),
            Value::NativeFunction(_) => self.results.push(node),
            Value::Function(_) => self.results.push(node),
            Value::Continuation(_) => self.results.push(node),

            Value::Symbol(s) => match self.alloc.get_bound_ptr(env, s) {
                None => {
                    self.error = Some(Error {
                        message: format!("{}: {}", UNBOUND_SYMBOL_ERROR, s),
                    })
                }
                Some(p) => self.results.push(p),
            },

            Value::Nil => {
                self.error = Some(Error {
                    message: EVAL_EMPTY_LIST_ERROR.to_string(),
                })
            }

            Value::Cons(hd, tl) => {
                let nodes = match self.cons_list_to_vector(*hd, *tl) {
                    Some(nodes) => nodes,
                    None => {
                        self.error = Some(Error {
                            message: EVAL_BAD_LIST_ERROR.to_string(),
                        });
                        return;
                    }
                };

                if nodes.is_empty() {
                    self.error = Some(Error {
                        message: EVAL_EMPTY_LIST_ERROR.to_string(),
                    });
                    return;
                }

                // handle special forms
                if let Value::Symbol(first_sym) = self.alloc.get_val(nodes[0]) {
                    match first_sym.as_str() {
                        "if" => {
                            if nodes.len() != 4 {
                                self.error = Some(Error {
                                    message: INVALID_IF_ERROR.to_string(),
                                });
                                return;
                            }

                            let else_clause = nodes[3];
                            let then_clause = nodes[2];

                            self.next_steps.push(Box::new(move |interp| {
                                let res = interp.results.pop().unwrap();

                                if let Value::Bool(false) = interp.alloc.get_val(res) {
                                    interp.eval_node(else_clause, env)
                                } else {
                                    interp.eval_node(then_clause, env)
                                }
                            }));
                            self.eval_node(nodes[1], env);
                            return;
                        }
                        "lambda" => {
                            if nodes.len() != 3 {
                                self.error = Some(Error {
                                    message: INVALID_LAMBDA_ERROR.to_string(),
                                });
                                return;
                            }

                            let mut args_names: Vec<String>;
                            if let Value::Cons(al_hd, al_tl) = self.alloc.get_val(nodes[1]) {
                                let arg_list = match self.cons_list_to_vector(*al_hd, *al_tl) {
                                    Some(al) => al,
                                    None => {
                                        self.error = Some(Error {
                                            message: INVALID_LAMBDA_ERROR.to_string(),
                                        });
                                        return;
                                    }
                                };

                                args_names = Vec::with_capacity(arg_list.len());
                                for arg in arg_list {
                                    if let Value::Symbol(arg) = self.alloc.get_val(arg) {
                                        args_names.push(arg.clone());
                                    } else {
                                        self.error = Some(Error {
                                            message: INVALID_LAMBDA_ERROR.to_string(),
                                        });
                                        return;
                                    }
                                }
                            } else {
                                self.error = Some(Error {
                                    message: INVALID_LAMBDA_ERROR.to_string(),
                                });
                                return;
                            }

                            self.results.push(
                                Value::Function(Function {
                                    args: args_names,
                                    env,
                                    body: nodes[2],
                                })
                                .gc(&mut self.alloc),
                            );

                            return;
                        }
                        "quote" => {
                            if nodes.len() != 2 {
                                self.error = Some(Error {
                                    message: INVALID_QUOTE_ERROR.to_string(),
                                });
                                return;
                            }

                            self.results.push(nodes[1]);
                            return;
                        }
                        "define" => {
                            if nodes.len() != 3 {
                                self.error = Some(Error {
                                    message: INVALID_DEFINE_ERROR.to_string(),
                                });
                                return;
                            }

                            let mut iter = nodes.into_iter();
                            iter.next(); // drop define

                            let name = match self.alloc.get_val(iter.next().unwrap()) {
                                Value::Symbol(s) => s.clone(),
                                _ => {
                                    self.error = Some(Error {
                                        message: INVALID_DEFINE_ERROR.to_string(),
                                    });
                                    return;
                                }
                            };

                            self.next_steps.push(Box::new(move |interp| {
                                assert_eq!(interp.results.len(), 1);
                                interp.alloc.set_bound_value(
                                    env,
                                    name,
                                    interp.results.pop().unwrap(),
                                );
                                interp.results.push(Value::Nil.gc(&mut interp.alloc));
                            }));
                            self.eval_node(iter.next().unwrap(), env);
                            return;
                        }
                        _ => {}
                    }
                }

                self.handle_func_call(nodes, env)
            }
        }
    }

    fn eval_ast(&mut self, node: AST, env: Ptr<Environment>) {
        let node_as_val = Value::from_ast(node, &mut self.alloc);
        self.eval_node(node_as_val, env);
    }

    fn run(&mut self) -> Result<Ptr<Value>, Error> {
        if self.error.is_some() {
            return Err(std::mem::replace(&mut self.error, None).unwrap());
        }

        while let Some(step) = self.next_steps.pop() {
            step(self);
            if self.error.is_some() {
                let err = std::mem::replace(&mut self.error, None).unwrap();
                self.clear_run_state();
                return Err(err);
            }
        }

        assert_eq!(self.results.len(), 1);
        Ok(self.results.pop().unwrap())
    }
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     use std::cell::Cell;
//
//     #[test]
//     fn runs_simple_example() {
//         let node = AST::List(vec![
//             AST::Symbol("+".to_string()),
//             AST::Integer(1),
//             AST::Integer(2),
//         ]);
//
//         let env = stdlib::build();
//         let cont_called = Rc::new(Cell::new(false));
//         let ccb = cont_called.clone();
//
//         eval_node(
//             Value::from(node).rc(),
//             env,
//             Box::new(move |res| {
//                 let res = res.unwrap();
//                 match res.borrow() {
//                     Value::Integer(3) => {}
//                     _ => panic!("expected 3"),
//                 };
//                 ccb.set(true);
//             }),
//         );
//
//         assert!(cont_called.get());
//     }
//
//     #[test]
//     fn handles_if() {
//         // (if #t (if #f 1 2) 3) -> 2
//         let node = AST::List(vec![
//             AST::Symbol("if".to_string()),
//             AST::Bool(true),
//             AST::List(vec![
//                 AST::Symbol("if".to_string()),
//                 AST::Bool(false),
//                 AST::Integer(1),
//                 AST::Integer(2),
//             ]),
//             AST::Integer(3),
//         ]);
//
//         let env = stdlib::build();
//         let cont_called = Rc::new(Cell::new(false));
//         let ccb = cont_called.clone();
//
//         eval_node(
//             Value::from(node).rc(),
//             env,
//             Box::new(move |res| {
//                 let res = res.unwrap();
//                 match res.borrow() {
//                     Value::Integer(2) => {}
//                     _ => panic!("expected 2"),
//                 };
//                 ccb.set(true);
//             }),
//         );
//
//         assert!(cont_called.get());
//     }
//
//     #[test]
//     fn handles_lambda() {
//         // ((lambda (x) (+ x 1)) 2) -> 3
//         let node = AST::List(vec![
//             AST::List(vec![
//                 AST::Symbol("lambda".to_string()),
//                 AST::List(vec![AST::Symbol("x".to_string())]),
//                 AST::List(vec![
//                     AST::Symbol("+".to_string()),
//                     AST::Symbol("x".to_string()),
//                     AST::Integer(1),
//                 ]),
//             ]),
//             AST::Integer(2),
//         ]);
//
//         let env = stdlib::build();
//         let cont_called = Rc::new(Cell::new(false));
//         let ccb = cont_called.clone();
//         eval_node(
//             Value::from(node).rc(),
//             env,
//             Box::new(move |res| {
//                 let res = res.unwrap();
//                 match res.borrow() {
//                     Value::Integer(3) => {}
//                     _ => panic!("expected 2"),
//                 };
//                 ccb.set(true);
//             }),
//         );
//         assert!(cont_called.get())
//     }
//
//     #[test]
//     fn runs_program() {
//         // (define x 1) x
//         let program = vec![
//             AST::List(vec![
//                 AST::Symbol("define".to_string()),
//                 AST::Symbol("x".to_string()),
//                 AST::Integer(1),
//             ]),
//             AST::Symbol("x".to_string()),
//         ];
//
//         let cont_called = Rc::new(Cell::new(false));
//         let ccb = cont_called.clone();
//
//         eval_nodes_toplevel(
//             program,
//             stdlib::build(),
//             Box::new(move |res| {
//                 let res = res.unwrap();
//                 if let Value::Integer(i) = res.as_ref() {
//                     assert_eq!(*i, 1);
//                 } else {
//                     panic!("expected integer result");
//                 }
//                 ccb.set(true);
//             }),
//         );
//         assert!(cont_called.get());
//     }
// }
