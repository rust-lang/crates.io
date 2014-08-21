use std::any::{Any, AnyMutRefExt, AnyRefExt};
use std::intrinsics::TypeId;
use std::collections::HashMap;

pub struct TypeMap {
    data: HashMap<TypeId, Box<Any>>
}

impl TypeMap {
    pub fn new() -> TypeMap {
        TypeMap { data: HashMap::new() }
    }

    pub fn find<T: 'static>(&self) -> Option<&T> {
        self.data.find(&TypeId::of::<T>()).and_then(|a| a.downcast_ref())
    }

    pub fn find_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.data.find_mut(&TypeId::of::<T>()).and_then(|a| a.downcast_mut())
    }

    pub fn insert<T: 'static>(&mut self, val: T) -> bool {
        self.data.insert(TypeId::of::<T>(), box val as Box<Any>)
    }

    pub fn remove<T: 'static>(&mut self) -> bool {
        self.data.remove(&TypeId::of::<T>())
    }

    pub fn contains<T: 'static>(&mut self) -> bool {
        self.data.contains_key(&TypeId::of::<T>())
    }
}

