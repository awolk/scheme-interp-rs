use super::*;

fn plus(args: &[Rc<Value>]) -> Result<Rc<Value>, Error> {
    let mut sum = 0;

    for arg in args {
        match arg.as_ref() {
            Value::Integer(i) => sum += *i,
            _ => {
                return Err(Error {
                    error_message: "all arguments to '+' must be integers".to_string(),
                })
            }
        }
    }

    Ok(Value::Integer(sum).rc())
}

pub(super) fn build() -> Rc<Environment> {
    let mut bindings = HashMap::new();

    bindings.insert("+".to_string(), Value::NativeFunction(plus).rc());

    Environment::new_with_bindings(bindings)
}
