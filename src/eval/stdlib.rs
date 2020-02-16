use super::*;

fn plus(args: &[Value]) -> Result<Value, Error> {
    let mut sum = 0;

    for arg in args {
        match arg {
            Value::Integer(i) => sum += *i,
            _ => {
                return Err(Error {
                    error_message: "all arguments to '+' must be integers".to_string(),
                })
            }
        }
    }

    Ok(Value::Integer(sum))
}

pub(super) fn build() -> Environment {
    let mut env = Environment::new();

    env.set("+".to_string(), Value::NativeFunction(plus));

    env
}
