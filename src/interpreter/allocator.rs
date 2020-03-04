use crate::interpreter::value::Value;
use std::collections::HashMap;
use std::marker::PhantomData;

pub(super) struct Ptr<T> {
    index: usize,
    phantom: PhantomData<T>,
}

impl<T> Ptr<T> {
    fn new(index: usize) -> Self {
        Self {
            index,
            phantom: PhantomData,
        }
    }
}

impl<T> Clone for Ptr<T> {
    fn clone(&self) -> Self {
        Ptr {
            index: self.index,
            phantom: PhantomData,
        }
    }
}

impl<T> Copy for Ptr<T> {}

struct GcNode<T> {
    item: T,
    marked: bool,
    free: bool,
}

impl<T> GcNode<T> {
    fn new(item: T) -> Self {
        Self {
            item,
            marked: false,
            free: false,
        }
    }
}

struct ItemAllocator<T> {
    values: Vec<GcNode<T>>,
    free: Vec<usize>,
}

impl<T> ItemAllocator<T> {
    fn new() -> Self {
        Self {
            values: Vec::new(),
            free: Vec::new(),
        }
    }

    fn alloc(&mut self, item: T) -> Ptr<T> {
        match self.free.pop() {
            None => {
                let index = self.values.len();
                self.values.push(GcNode::new(item));
                Ptr::new(index)
            }
            Some(index) => {
                self.values[index] = GcNode::new(item);
                Ptr::new(index)
            }
        }
    }

    fn get(&self, ptr: Ptr<T>) -> &T {
        &self.values[ptr.index].item
    }

    fn get_mut(&mut self, ptr: Ptr<T>) -> &mut T {
        &mut self.values[ptr.index].item
    }

    fn sweep(&mut self) {
        for (i, node) in self.values.iter_mut().enumerate() {
            if !node.free && !node.marked {
                self.free.push(i);
                node.free = true;
            }
        }
    }

    // returns previous mark value
    fn mark(&mut self, ptr: Ptr<T>) -> bool {
        std::mem::replace(&mut self.values[ptr.index].marked, true)
    }
}

pub(super) struct Environment {
    parent: Option<Ptr<Environment>>,
    bindings: HashMap<String, Ptr<Value>>,
}

impl Environment {
    pub(super) fn new_child_with_bindings(
        parent: Ptr<Environment>,
        bindings: HashMap<String, Ptr<Value>>,
    ) -> Self {
        Self {
            parent: Some(parent),
            bindings,
        }
    }

    pub(super) fn new_with_bindings(bindings: HashMap<String, Ptr<Value>>) -> Self {
        Self {
            parent: None,
            bindings,
        }
    }

    pub(super) fn gc(self, alloc: &mut Allocator) -> Ptr<Self> {
        alloc.new_env(self)
    }
}

pub(super) struct Allocator {
    values: ItemAllocator<Value>,
    environments: ItemAllocator<Environment>,
}

impl Allocator {
    pub(super) fn new() -> Self {
        Self {
            values: ItemAllocator::new(),
            environments: ItemAllocator::new(),
        }
    }

    pub(super) fn new_val(&mut self, val: Value) -> Ptr<Value> {
        self.values.alloc(val)
    }

    pub(super) fn new_env(&mut self, env: Environment) -> Ptr<Environment> {
        self.environments.alloc(env)
    }

    pub(super) fn get_val(&self, ptr: Ptr<Value>) -> &Value {
        self.values.get(ptr)
    }

    pub(super) fn get_bound_ptr(&self, env: Ptr<Environment>, name: &str) -> Option<Ptr<Value>> {
        let mut env_ptr = env;
        loop {
            let env = self.environments.get(env_ptr);
            if let Some(&ptr) = env.bindings.get(name) {
                return Some(ptr);
            }
            env_ptr = env.parent?;
        }
    }

    pub(super) fn set_bound_value(
        &mut self,
        env: Ptr<Environment>,
        name: String,
        value: Ptr<Value>,
    ) {
        let env = self.environments.get_mut(env);
        env.bindings.insert(name, value);
    }

    fn mark_env(&mut self, env: Ptr<Environment>) {
        if self.environments.mark(env) {
            return; // return if already marked
        }
        let env = self.environments.get(env);
        let values = env.bindings.values().copied().collect::<Vec<_>>();
        let parent = env.parent;

        for value in values {
            self.mark_val(value);
        }
        if let Some(parent) = parent {
            self.mark_env(parent)
        }
    }

    fn mark_val(&mut self, val: Ptr<Value>) {
        if self.values.mark(val) {
            return; // return if already marked
        }
        match self.values.get(val) {
            Value::Cons(a, b) => {
                let (a, b) = (*a, *b);
                self.mark_val(a);
                self.mark_val(b);
            }
            Value::Function(f) => {
                let env = f.env;
                self.mark_env(env)
            }
            Value::Continuation(c) => {
                let mut all_vals = Vec::new();
                all_vals.extend_from_slice(&c.results);
                for sr in &c.saved_results {
                    all_vals.extend_from_slice(sr)
                }

                for val in all_vals {
                    self.mark_val(val)
                }
            }
            _ => {}
        }
    }

    pub(super) fn gc(&mut self, leaf: Ptr<Environment>) {
        self.mark_env(leaf);
        self.values.sweep();
        self.environments.sweep();
    }

    pub(super) fn profile(&self) -> GCInfo {
        GCInfo {
            values_heap_size: self.values.values.len(),
            values_heap_free: self.values.free.len(),
            environments_heap_size: self.environments.values.len(),
            environments_heap_free: self.environments.free.len(),
        }
    }
}

pub(super) struct GCInfo {
    pub(super) values_heap_size: usize,
    pub(super) values_heap_free: usize,
    pub(super) environments_heap_size: usize,
    pub(super) environments_heap_free: usize,
}
