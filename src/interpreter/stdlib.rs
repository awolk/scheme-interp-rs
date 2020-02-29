use super::{env::*, value::*, Continuation, Error};
use std::collections::HashMap;
use std::rc::Rc;

fn plus(args: &[Rc<Value>], cont: Continuation) {
    let mut sum = 0;

    for arg in args {
        match arg.as_ref() {
            Value::Integer(i) => sum += *i,
            _ => {
                cont(Err(Error {
                    message: "all arguments to '+' must be integers".to_string(),
                }));
                return;
            }
        }
    }

    cont(Ok(Value::Integer(sum).rc()))
}

pub fn build() -> Rc<MutEnvironment> {
    let mut bindings = HashMap::new();

    bindings.insert("+".to_string(), Value::NativeFunction(plus).rc());

    MutEnvironment::new_with_bindings(bindings)
}
