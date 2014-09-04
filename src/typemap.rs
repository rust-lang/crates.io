use std::any::{Any, AnyMutRefExt, AnyRefExt};
use std::boxed::BoxAny;
use std::intrinsics::TypeId;
use std::collections::HashMap;

pub struct TypeMap {
    data: HashMap<TypeId, Box<Any + 'static>>
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

    pub fn pop<T: 'static>(&mut self) -> Option<T> {
        let data = match self.data.pop(&TypeId::of::<T>()) {
            Some(data) => data,
            None => return None,
        };
        Some(*data.downcast::<T>().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::TypeMap;

    #[test]
    fn smoke() {
        let mut m = TypeMap::new();
        assert!(m.insert(1i));
        assert_eq!(*m.find::<int>().unwrap(), 1);
        assert_eq!(*m.find_mut::<int>().unwrap(), 1);
        assert!(!m.insert(2i));
        assert!(m.remove::<int>());
        assert!(!m.contains::<int>());
        assert!(m.insert(4i));
        assert_eq!(m.pop::<int>().unwrap(), 4);
    }
}

