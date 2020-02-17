use super::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub trait Environment {
    fn get(&self, key: &str) -> Option<Rc<Value>>;
}

pub struct NestedEnvironment {
    parent: Option<Rc<dyn Environment>>,
    bindings: HashMap<String, Rc<Value>>,
}

impl NestedEnvironment {
    pub fn new_child_with_bindings(
        parent: &Rc<dyn Environment>,
        bindings: HashMap<String, Rc<Value>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            parent: Some(Rc::clone(parent)),
            bindings,
        })
    }
}

impl Environment for NestedEnvironment {
    fn get(&self, key: &str) -> Option<Rc<Value>> {
        self.bindings
            .get(key)
            .cloned()
            .or_else(|| self.parent.as_ref().and_then(|parent| parent.get(key)))
    }
}

pub struct MutEnvironment {
    bindings: RefCell<HashMap<String, Rc<Value>>>,
}

impl MutEnvironment {
    pub fn new_with_bindings(bindings: HashMap<String, Rc<Value>>) -> Rc<Self> {
        Rc::new(Self {
            bindings: RefCell::new(bindings),
        })
    }

    pub fn set(env: &Rc<MutEnvironment>, key: String, value: Rc<Value>) {
        env.bindings.borrow_mut().insert(key, value);
    }
}

impl Environment for MutEnvironment {
    fn get(&self, key: &str) -> Option<Rc<Value>> {
        self.bindings.borrow().get(key).cloned()
    }
}
