//! The "eXtra Small Box"! A purely stack-allocated box intended for holding trait objects. Heavily
//! drawn from the `smallbox` crate.

use std::{
    any::Any,
    cmp::Ordering,
    fmt,
    hash::{self, Hash},
    marker::PhantomData,
    mem::{self, ManuallyDrop},
    ops::{Deref, DerefMut},
    ptr::{self, Pointee},
};

#[macro_export]
macro_rules! xsbox {
    ( $e: expr ) => {{
        let val = $e;
        let ptr = &val as *const _;
        #[allow(unsafe_code)]
        unsafe {
            $crate::xsbox::XsBox::new_unchecked(val, ptr)
        }
    }};
}

pub type CopyBox<T, Space> = XsBox<T, Copyable<T, Space>>;

pub trait Storable<T: ?Sized, U: ?Sized>: Storage<T> {
    #[allow(clippy::missing_safety_doc)]
    unsafe fn new_copy(val: &U, ptr: *const T) -> Self;

    type Downcast: Storage<U>;
    #[allow(clippy::missing_safety_doc)]
    unsafe fn downcast_unchecked(self) -> Self::Downcast
    where
        U: Any + Sized;
}

pub trait Storage<T: ?Sized> {
    fn as_ptr(&self) -> *const T;
    fn as_mut_ptr(&mut self) -> *mut T;
}

pub struct NonCopyable<T: ?Sized, Space> {
    metadata: <T as Pointee>::Metadata,
    space: ManuallyDrop<Space>,
    _phantom: PhantomData<T>,
}

impl<T: ?Sized, Space, U: ?Sized> Storable<T, U> for NonCopyable<T, Space> {
    unsafe fn new_copy(val: &U, ptr: *const T) -> Self {
        let size = mem::size_of_val::<U>(val);
        let align = mem::align_of_val::<U>(val);
        let metadata = ptr::metadata(ptr);

        let mut space = mem::MaybeUninit::<Space>::uninit();

        let ptr_copy: *mut u8 = if size == 0 {
            align as *mut u8
        } else if size > mem::size_of::<Space>() || align > mem::align_of::<Space>() {
            panic!("value does not fit inside provided space/with required alignment.");
        } else {
            space.as_mut_ptr() as *mut u8
        };

        ptr::copy_nonoverlapping(val as *const _ as *const u8, ptr_copy, size);

        Self {
            metadata,
            space: ManuallyDrop::new(space.assume_init()),
            _phantom: PhantomData,
        }
    }

    type Downcast = NonCopyable<U, Space>;
    unsafe fn downcast_unchecked(self) -> Self::Downcast
    where
        U: Any + Sized,
    {
        let size = mem::size_of::<U>();
        let mut space = mem::MaybeUninit::<Space>::uninit();

        ptr::copy_nonoverlapping(
            &*self.space as *const _ as *const u8,
            space.as_mut_ptr() as *mut u8,
            size,
        );

        let metadata = ptr::metadata(space.as_ptr() as *const U);

        mem::forget(self);

        NonCopyable {
            metadata,
            space: ManuallyDrop::new(space.assume_init()),
            _phantom: Default::default(),
        }
    }
}

impl<T: ?Sized, Space> Storage<T> for NonCopyable<T, Space> {
    fn as_ptr(&self) -> *const T {
        ptr::from_raw_parts(&self.space as *const _ as *const (), self.metadata)
    }

    fn as_mut_ptr(&mut self) -> *mut T {
        ptr::from_raw_parts_mut(&mut self.space as *mut _ as *mut (), self.metadata)
    }
}

impl<T: ?Sized, Space> Drop for NonCopyable<T, Space> {
    fn drop(&mut self) {
        unsafe {
            ptr::drop_in_place(self.as_mut_ptr());
        }
    }
}

pub struct Copyable<T: ?Sized, Space: Copy> {
    metadata: <T as Pointee>::Metadata,
    space: ManuallyDrop<Space>,
    _phantom: PhantomData<T>,
}

impl<T: ?Sized, Space: Copy> Clone for Copyable<T, Space> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized, Space: Copy> Copy for Copyable<T, Space> {}

impl<T: ?Sized, Space: Copy, U: Copy> Storable<T, U> for Copyable<T, Space> {
    unsafe fn new_copy(val: &U, ptr: *const T) -> Self {
        let size = mem::size_of_val::<U>(val);
        let align = mem::align_of_val::<U>(val);
        let metadata = ptr::metadata(ptr);

        let mut space = mem::MaybeUninit::<Space>::uninit();

        let ptr_copy: *mut u8 = if size == 0 {
            align as *mut u8
        } else if size > mem::size_of::<Space>() || align > mem::align_of::<Space>() {
            panic!("value does not fit inside provided space/with required alignment.");
        } else {
            space.as_mut_ptr() as *mut u8
        };

        ptr::copy_nonoverlapping(val as *const _ as *const u8, ptr_copy, size);

        Self {
            metadata,
            space: ManuallyDrop::new(space.assume_init()),
            _phantom: PhantomData,
        }
    }

    type Downcast = Copyable<U, Space>;
    unsafe fn downcast_unchecked(self) -> Self::Downcast
    where
        U: Any + Sized,
    {
        let size = mem::size_of::<U>();
        let mut space = mem::MaybeUninit::<Space>::uninit();

        ptr::copy_nonoverlapping(
            &*self.space as *const _ as *const u8,
            space.as_mut_ptr() as *mut u8,
            size,
        );

        let metadata = ptr::metadata(space.as_ptr() as *const U);

        // No need to `mem::forget` as we're working with `Copy` types here.

        Copyable {
            metadata,
            space: ManuallyDrop::new(space.assume_init()),
            _phantom: Default::default(),
        }
    }
}

impl<T: ?Sized, Space: Copy> Storage<T> for Copyable<T, Space> {
    fn as_ptr(&self) -> *const T {
        ptr::from_raw_parts(&self.space as *const _ as *const (), self.metadata)
    }

    fn as_mut_ptr(&mut self) -> *mut T {
        ptr::from_raw_parts_mut(&mut self.space as *mut _ as *mut (), self.metadata)
    }
}

impl<T: ?Sized, Space: Storage<T> + Clone> Clone for XsBox<T, Space> {
    fn clone(&self) -> Self {
        Self {
            space: self.space.clone(),
            _phantom: Default::default(),
        }
    }
}

impl<T: ?Sized, Space: Storage<T> + Copy> Copy for XsBox<T, Space> {}

pub struct XsBox<T: ?Sized, Space: Storage<T>> {
    space: Space,
    _phantom: PhantomData<T>,
}

impl<T: ?Sized, Space: Storage<T>> XsBox<T, Space> {
    #[inline(always)]
    pub fn new(val: T) -> Self
    where
        T: Sized,
        Space: Storable<T, T>,
    {
        xsbox!(val)
    }
}

impl<T: ?Sized, Space: Storage<T>> XsBox<T, Space> {
    #[doc(hidden)]
    #[inline]
    pub unsafe fn new_unchecked<U>(val: U, ptr: *const T) -> Self
    where
        U: Sized,
        Space: Storable<T, U>,
    {
        let space = <Space as Storable<T, U>>::new_copy(&val, ptr);
        mem::forget(val);
        Self {
            space,
            _phantom: Default::default(),
        }
    }

    unsafe fn downcast_unchecked<U: Any>(self) -> XsBox<U, <Space as Storable<T, U>>::Downcast>
    where
        Space: Storable<T, U>,
    {
        let space = self.space.downcast_unchecked();

        XsBox {
            space,
            _phantom: Default::default(),
        }
    }

    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self.space.as_ptr()
    }

    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.space.as_mut_ptr()
    }

    #[inline]
    pub fn into_inner(self) -> T
    where
        T: Sized,
    {
        let ret_val: T = unsafe { self.as_ptr().read() };
        mem::forget(self);
        ret_val
    }
}

impl<Space: Storage<dyn Any>> XsBox<dyn Any, Space> {
    #[inline]
    pub fn downcast<T: Any>(self) -> Result<XsBox<T, Space::Downcast>, Self>
    where
        Space: Storable<dyn Any, T>,
    {
        if self.is::<T>() {
            unsafe { Ok(self.downcast_unchecked()) }
        } else {
            Err(self)
        }
    }
}

impl<Space: Storage<dyn Any + Send>> XsBox<dyn Any + Send, Space> {
    #[inline]
    pub fn downcast<T: Any>(self) -> Result<XsBox<T, Space::Downcast>, Self>
    where
        Space: Storable<dyn Any + Send, T>,
    {
        if self.is::<T>() {
            unsafe { Ok(self.downcast_unchecked()) }
        } else {
            Err(self)
        }
    }
}

impl<T: ?Sized, Space: Storage<T>> Deref for XsBox<T, Space> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.as_ptr() }
    }
}

impl<T: ?Sized, Space: Storage<T>> DerefMut for XsBox<T, Space> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.as_mut_ptr() }
    }
}

impl<T: ?Sized + fmt::Display, Space: Storage<T>> fmt::Display for XsBox<T, Space> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: ?Sized + fmt::Debug, Space: Storage<T>> fmt::Debug for XsBox<T, Space> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized, Space: Storage<T>> fmt::Pointer for XsBox<T, Space> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ptr: *const T = &**self;
        fmt::Pointer::fmt(&ptr, f)
    }
}

impl<T: ?Sized + PartialEq, Space: Storage<T>> PartialEq for XsBox<T, Space> {
    fn eq(&self, other: &XsBox<T, Space>) -> bool {
        PartialEq::eq(&**self, &**other)
    }

    #[allow(clippy::partialeq_ne_impl)]
    fn ne(&self, other: &XsBox<T, Space>) -> bool {
        PartialEq::ne(&**self, &**other)
    }
}

impl<T: ?Sized + PartialOrd, Space: Storage<T>> PartialOrd for XsBox<T, Space> {
    fn partial_cmp(&self, other: &XsBox<T, Space>) -> Option<Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }

    fn lt(&self, other: &XsBox<T, Space>) -> bool {
        PartialOrd::lt(&**self, &**other)
    }

    fn le(&self, other: &XsBox<T, Space>) -> bool {
        PartialOrd::le(&**self, &**other)
    }

    fn ge(&self, other: &XsBox<T, Space>) -> bool {
        PartialOrd::ge(&**self, &**other)
    }

    fn gt(&self, other: &XsBox<T, Space>) -> bool {
        PartialOrd::gt(&**self, &**other)
    }
}

impl<T: ?Sized + Ord, Space: Storage<T>> Ord for XsBox<T, Space> {
    fn cmp(&self, other: &XsBox<T, Space>) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}

impl<T: ?Sized + Eq, Space: Storage<T>> Eq for XsBox<T, Space> {}

impl<T: ?Sized + Hash, Space: Storage<T>> Hash for XsBox<T, Space> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

unsafe impl<T: ?Sized + Send, Space: Storage<T>> Send for XsBox<T, Space> {}
unsafe impl<T: ?Sized + Sync, Space: Storage<T>> Sync for XsBox<T, Space> {}