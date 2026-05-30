use crate::closure::ClosureObject;
use crate::function::FunctionObject;
use crate::value::{Object, ObjectType};

pub struct Heap {
    objects: *mut Object,
}
impl Heap {
    pub fn new() -> Self {
        Self { objects: std::ptr::null_mut() }
    }
    pub fn allocate(&mut self, obj_type: ObjectType) -> *mut Object {
        let obj = Box::new(Object {
            obj_type,
            is_marked: false,
            next: self.objects,
        });
        let ptr = Box::into_raw(obj);
        self.objects = ptr;
        ptr
    }
}
impl Drop for Heap {
    fn drop(&mut self) {
        let mut current = self.objects;
        while !current.is_null() {
            unsafe {
                let next = (*current).next;
                drop(Box::from_raw(current));
                current = next;
            }
        }
        self.objects = std::ptr::null_mut();
    }
}
