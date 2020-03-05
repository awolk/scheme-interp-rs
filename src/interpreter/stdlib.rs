use super::{value::*, Error};
use crate::interpreter::allocator::{Allocator, Environment, Ptr};
use crate::interpreter::Interpreter;
use std::collections::HashMap;

fn plus(interp: &mut Interpreter, _env: Ptr<Environment>, args: &[Ptr<Value>]) {
    let mut sum = 0;

    for arg in args {
        match interp.alloc.get_val(*arg) {
            Value::Integer(i) => sum += *i,
            _ => {
                interp.error = Some(Error {
                    message: "all arguments to '+' must be integers".to_string(),
                });
                return;
            }
        }
    }

    interp
        .results
        .push(Value::Integer(sum).gc(&mut interp.alloc));
}

fn times(interp: &mut Interpreter, _env: Ptr<Environment>, args: &[Ptr<Value>]) {
    let mut prod = 1;

    for arg in args {
        match interp.alloc.get_val(*arg) {
            Value::Integer(i) => prod *= *i,
            _ => {
                interp.error = Some(Error {
                    message: "all arguments to '*' must be integers".to_string(),
                });
                return;
            }
        }
    }

    interp
        .results
        .push(Value::Integer(prod).gc(&mut interp.alloc));
}

fn ieq(interp: &mut Interpreter, _env: Ptr<Environment>, args: &[Ptr<Value>]) {
    if args.len() != 2 {
        interp.error = Some(Error {
            message: "i= takes 2 arguments".to_string(),
        });
        return;
    }

    let i1 = match interp.alloc.get_val(args[0]) {
        Value::Integer(i) => i,
        _ => {
            interp.error = Some(Error {
                message: "the first argument to 'i=' must be an integer".to_string(),
            });
            return;
        }
    };

    let i2 = match interp.alloc.get_val(args[1]) {
        Value::Integer(i) => i,
        _ => {
            interp.error = Some(Error {
                message: "the second argument to 'i=' must be an integer".to_string(),
            });
            return;
        }
    };

    interp
        .results
        .push(Value::Bool(i1 == i2).gc(&mut interp.alloc));
}

fn minus(interp: &mut Interpreter, _env: Ptr<Environment>, args: &[Ptr<Value>]) {
    if args.len() != 2 {
        interp.error = Some(Error {
            message: "'-' takes 2 arguments".to_string(),
        });
        return;
    }

    let i1 = match interp.alloc.get_val(args[0]) {
        Value::Integer(i) => i,
        _ => {
            interp.error = Some(Error {
                message: "the first argument to '-' must be an integer".to_string(),
            });
            return;
        }
    };

    let i2 = match interp.alloc.get_val(args[1]) {
        Value::Integer(i) => i,
        _ => {
            interp.error = Some(Error {
                message: "the second argument to '-' must be an integer".to_string(),
            });
            return;
        }
    };

    interp
        .results
        .push(Value::Integer(i1 - i2).gc(&mut interp.alloc));
}

fn cons(interp: &mut Interpreter, _env: Ptr<Environment>, args: &[Ptr<Value>]) {
    if args.len() != 2 {
        interp.error = Some(Error {
            message: "cons takes 2 arguments".to_string(),
        });
        return;
    }

    interp
        .results
        .push(Value::Cons(args[0], args[1]).gc(&mut interp.alloc))
}

fn call_with_cc(interp: &mut Interpreter, env: Ptr<Environment>, args: &[Ptr<Value>]) {
    if args.len() != 1 {
        interp.error = Some(Error {
            message: "call/cc takes 1 argument".to_string(),
        });
        return;
    }

    let next_steps = clone_steps(&interp.next_steps);
    let cont_val = Continuation {
        // TODO: can we eliminate the amount of copied data for a continuation
        next_steps,
        results: interp.results.clone(),
        saved_results: interp.saved_results.clone(),
    };
    let cont = Value::Continuation(cont_val).gc(&mut interp.alloc);
    let func_call = vec![args[0], cont];
    interp.handle_func_call(func_call, env)
}

fn last(interp: &mut Interpreter, _env: Ptr<Environment>, args: &[Ptr<Value>]) {
    if args.is_empty() {
        interp.error = Some(Error {
            message: "last requires at least 1 argument".to_string(),
        });
        return;
    }

    interp.results.push(args[args.len() - 1]);
}

fn gc_profile(interp: &mut Interpreter, _env: Ptr<Environment>, _args: &[Ptr<Value>]) {
    let info = interp.alloc.profile();
    println!(
        "values: size: {}, allocated: {}",
        info.values_heap_size,
        info.values_heap_size - info.values_heap_free
    );
    println!(
        "environments: size: {}, allocated: {}",
        info.environments_heap_size,
        info.environments_heap_size - info.environments_heap_free
    );
    interp.results.push(Value::Nil.gc(&mut interp.alloc));
}

fn gc_run(interp: &mut Interpreter, env: Ptr<Environment>, _args: &[Ptr<Value>]) {
    interp.alloc.gc(env);
    interp.results.push(Value::Nil.gc(&mut interp.alloc));
}

pub(super) fn build(alloc: &mut Allocator) -> Ptr<Environment> {
    let mut bindings = HashMap::new();

    bindings.insert("+".to_string(), Value::NativeFunction(plus).gc(alloc));
    bindings.insert("*".to_string(), Value::NativeFunction(times).gc(alloc));
    bindings.insert("i=".to_string(), Value::NativeFunction(ieq).gc(alloc));
    bindings.insert("-".to_string(), Value::NativeFunction(minus).gc(alloc));
    bindings.insert("cons".to_string(), Value::NativeFunction(cons).gc(alloc));
    bindings.insert(
        "call/cc".to_string(),
        Value::NativeFunction(call_with_cc).gc(alloc),
    );
    bindings.insert("last".to_string(), Value::NativeFunction(last).gc(alloc));
    bindings.insert(
        "gc-profile".to_string(),
        Value::NativeFunction(gc_profile).gc(alloc),
    );
    bindings.insert(
        "gc-run".to_string(),
        Value::NativeFunction(gc_run).gc(alloc),
    );
    bindings.insert("nil".to_string(), Value::Nil.gc(alloc));

    Environment::new_with_bindings(bindings).gc(alloc)
}
