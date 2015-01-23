use std::any::{Any, TypeId};
use std::boxed::BoxAny;
use std::collections::HashMap;

pub struct TypeMap {
    data: HashMap<TypeId, Box<Any + 'static>>
}

impl TypeMap {
    pub fn new() -> TypeMap {
        TypeMap { data: HashMap::new() }
    }

    pub fn find<T: 'static>(&self) -> Option<&T> {
        self.data.get(&TypeId::of::<T>()).and_then(|a| a.downcast_ref())
    }

    pub fn find_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.data.get_mut(&TypeId::of::<T>()).and_then(|a| a.downcast_mut())
    }

    pub fn insert<T: 'static>(&mut self, val: T) -> bool {
        self.data.insert(TypeId::of::<T>(), Box::new(val) as Box<Any>).is_none()
    }

    pub fn remove<T: 'static>(&mut self) -> bool {
        self.data.remove(&TypeId::of::<T>()).is_some()
    }

    pub fn contains<T: 'static>(&self) -> bool {
        self.data.contains_key(&TypeId::of::<T>())
    }

    pub fn pop<T: 'static>(&mut self) -> Option<T> {
        let data = match self.data.remove(&TypeId::of::<T>()) {
            Some(data) => data,
            None => return None,
        };
        Some(*data.downcast::<T>().ok().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::TypeMap;

    #[test]
    fn smoke() {
        let mut m = TypeMap::new();
        assert!(m.insert(1));
        assert_eq!(*m.find::<i32>().unwrap(), 1);
        assert_eq!(*m.find_mut::<i32>().unwrap(), 1);
        assert!(!m.insert(2));
        assert!(m.remove::<i32>());
        assert!(!m.contains::<i32>());
        assert!(m.insert(4));
        assert_eq!(m.pop::<i32>().unwrap(), 4);
    }
}

