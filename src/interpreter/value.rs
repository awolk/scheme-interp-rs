use super::env::Environment;
use super::Continuation;
use crate::parse::AST;
use std::rc::Rc;

pub struct Function {
    pub(super) args: Vec<String>,
    pub(super) env: Rc<dyn Environment>,
    pub(super) body: Rc<Value>,
}

pub enum Value {
    Integer(i64),
    Bool(bool),
    Function(Function),
    NativeFunction(fn(&[Rc<Value>], Continuation)),
    Symbol(String),
    Nil,
    Cons(Rc<Value>, Rc<Value>),
}

impl Value {
    pub fn rc(self) -> Rc<Self> {
        Rc::new(self)
    }
}

impl From<AST> for Value {
    fn from(node: AST) -> Self {
        match node {
            AST::Symbol(s) => Value::Symbol(s),
            AST::Integer(i) => Value::Integer(i),
            AST::Bool(b) => Value::Bool(b),
            AST::List(l) => {
                let mut res = Value::Nil;
                let mut iter = l.into_iter();
                while let Some(entry) = iter.next_back() {
                    res = Value::Cons(Value::from(entry).rc(), res.rc());
                }
                res
            }
        }
    }
}

impl ToString for Value {
    fn to_string(&self) -> String {
        match self {
            Value::Integer(i) => i.to_string(),
            Value::Bool(b) => (if *b { "#t" } else { "#f" }).to_string(),
            Value::Function(_f) => "<lisp function>".to_string(),
            Value::NativeFunction(_f) => "<native function>".to_string(),
            Value::Symbol(s) => s.clone(),
            Value::Nil => "()".to_string(),
            Value::Cons(a, b) => format!("({} . {})", a.to_string(), b.to_string()),
        }
    }
}
