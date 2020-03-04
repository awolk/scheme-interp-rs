use super::allocator::{Allocator, Environment, Ptr};
use crate::interpreter::{Interpreter, Step};
use crate::parse::AST;

pub struct Function {
    pub(super) args: Vec<String>,
    pub(super) env: Ptr<Environment>,
    pub(super) body: Ptr<Value>,
}

pub struct Continuation {
    pub(super) next_steps: Vec<Step>,
    pub(super) results: Vec<Ptr<Value>>,
    pub(super) saved_results: Vec<Vec<Ptr<Value>>>,
}

pub(super) enum Value {
    Integer(i64),
    Bool(bool),
    Function(Function),
    NativeFunction(fn(&mut Interpreter, Ptr<Environment>, &[Ptr<Value>])),
    Symbol(String),
    Nil,
    Cons(Ptr<Value>, Ptr<Value>),
    Continuation(Continuation),
}

impl Value {
    pub(super) fn gc(self, alloc: &mut Allocator) -> Ptr<Self> {
        alloc.new_val(self)
    }

    pub(super) fn from_ast(node: AST, alloc: &mut Allocator) -> Ptr<Self> {
        match node {
            AST::Symbol(s) => Value::Symbol(s).gc(alloc),
            AST::Integer(i) => Value::Integer(i).gc(alloc),
            AST::Bool(b) => Value::Bool(b).gc(alloc),
            AST::List(l) => {
                let mut res = Value::Nil.gc(alloc);
                let mut iter = l.into_iter();
                while let Some(entry) = iter.next_back() {
                    res = Value::Cons(Value::from_ast(entry, alloc), res).gc(alloc);
                }
                res
            }
        }
    }

    pub(super) fn to_string(&self, alloc: &Allocator) -> String {
        match self {
            Value::Integer(i) => i.to_string(),
            Value::Bool(b) => (if *b { "#t" } else { "#f" }).to_string(),
            Value::Function(_f) => "<lisp function>".to_string(),
            Value::NativeFunction(_f) => "<native function>".to_string(),
            Value::Symbol(s) => s.clone(),
            Value::Nil => "()".to_string(),
            Value::Cons(a, b) => format!(
                "({} . {})",
                alloc.get_val(*a).to_string(alloc),
                alloc.get_val(*b).to_string(alloc)
            ),
            Value::Continuation(_c) => "<continuation>".to_string(),
        }
    }
}

pub(super) fn clone_steps(cc: &Vec<Step>) -> Vec<Step> {
    cc.iter().map(|step| step.clone_box()).collect::<Vec<_>>()
}
