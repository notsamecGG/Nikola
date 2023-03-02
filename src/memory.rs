use std::{
    rc::Rc,
    cell::RefCell,
    ops::{Deref, DerefMut}, borrow::{BorrowMut, Borrow},
};

#[derive(Debug)]
pub struct Rcc<T: Copy> {
    data: Rc<RefCell<T>>,
}

impl<T: Copy> Rcc<T> {
    pub fn new(data: T) -> Self {
        Rcc {
            data: Rc::new(RefCell::new(data)),
        }
    }

    pub fn from_other(other: &Rcc<T>) -> Self {
        Rcc {
            data: other.data.clone()
        }
    }
}

impl<T: Copy> Clone for Rcc<T> {
    fn clone(&self) -> Self {
        Rcc::from_other(self)
    }
}

impl<T: Copy> Deref for Rcc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            self.data.as_ptr().as_ref()
        }.unwrap() 
    }
}

impl<T: Copy> DerefMut for Rcc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            (*self.data).as_ptr().as_mut()
        }.unwrap()
    }
}

impl<T: Copy> Borrow<T> for Rcc<T> {
    fn borrow(&self) -> &T {
        unsafe {
            self.data.as_ptr().as_ref()
        }.unwrap()
    }
}

impl<T: Copy> BorrowMut<T> for Rcc<T> {
    fn borrow_mut(&mut self) -> &mut T {
        unsafe {
            (*self.data).as_ptr().as_mut()
        }.unwrap()
    }
}
