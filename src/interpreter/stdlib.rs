use super::{env::*, value::*, Continuation, Error};
use std::collections::HashMap;
use std::rc::Rc;

fn plus(args: &[Rc<Value>], cont: Continuation) {
    let mut sum = 0;

    for arg in args {
        match arg.as_ref() {
            Value::Integer(i) => sum += *i,
            _ => {
                return cont(Err(Error {
                    message: "all arguments to '+' must be integers".to_string(),
                }));
            }
        }
    }

    cont(Ok(Value::Integer(sum).rc()))
}

fn cons(args: &[Rc<Value>], cont: Continuation) {
    if args.len() != 2 {
        return cont(Err(Error {
            message: "cons takes 2 arguments".to_string(),
        }));
    }

    cont(Ok(
        Value::Cons(Rc::clone(&args[0]), Rc::clone(&args[1])).rc()
    ))
}

pub fn build() -> Rc<MutEnvironment> {
    let mut bindings = HashMap::new();

    bindings.insert("+".to_string(), Value::NativeFunction(plus).rc());
    bindings.insert("cons".to_string(), Value::NativeFunction(cons).rc());
    bindings.insert("nil".to_string(), Value::Nil.rc());

    MutEnvironment::new_with_bindings(bindings)
}
