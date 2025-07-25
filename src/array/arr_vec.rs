use core::{marker::PhantomData, mem::MaybeUninit};

use crate::utils;

use super::{ArrApi, ArrVec, Array, arr_utils::check_len};

#[repr(transparent)]
pub struct ArrVecDrop<A: Array<Item = T>, T = <A as Array>::Item>(ArrVecRepr<A>, PhantomData<T>);
impl<A: Array<Item = T>, T> Drop for ArrVecDrop<A, T> {
    fn drop(&mut self) {
        unsafe {
            let vec = &mut *(&raw mut *self).cast::<ArrVec<A>>();
            core::ptr::drop_in_place(vec.as_mut_slice())
        }
    }
}

// TODO: Consider using repr(C) to make grow/shrink be a simple transmute
pub struct ArrVecRepr<A: Array> {
    len: usize,
    arr: ArrApi<MaybeUninit<A>>,
}

macro_rules! repr {
    ($self:expr) => {
        $self.0.0
    };
}

impl<A: Array<Item = T>, T> ArrVec<A> {
    const unsafe fn from_repr(repr: ArrVecRepr<A>) -> Self {
        Self(ArrVecDrop(repr, PhantomData), PhantomData)
    }
    const fn into_repr(self) -> ArrVecRepr<A> {
        unsafe { utils::transmute(self) }
    }
    pub const fn new() -> Self {
        unsafe {
            Self::from_repr(ArrVecRepr {
                arr: ArrApi::new(MaybeUninit::uninit()),
                len: 0,
            })
        }
    }
    pub const fn full(full: A) -> Self {
        unsafe {
            Self::from_repr(ArrVecRepr {
                arr: ArrApi::new(MaybeUninit::new(full)),
                len: check_len::<A>(),
            })
        }
    }
    pub const fn as_slice(&self) -> &[T] {
        let &ArrVecRepr { ref arr, len } = &repr!(self);
        unsafe { core::slice::from_raw_parts(arr.as_slice().split_at(len).0.as_ptr().cast(), len) }
    }
    pub const fn as_mut_slice(&mut self) -> &mut [T] {
        let &mut ArrVecRepr { ref mut arr, len } = &mut repr!(self);
        unsafe {
            core::slice::from_raw_parts_mut(
                arr.as_mut_slice().split_at_mut(len).0.as_mut_ptr().cast(),
                len,
            )
        }
    }
    pub const fn len(&self) -> usize {
        repr!(self).len
    }
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub const fn capacity(&self) -> usize {
        check_len::<A>()
    }
    pub const fn is_full(&self) -> bool {
        self.len() == check_len::<A>()
    }
    #[track_caller]
    pub const fn into_full(self) -> A {
        if !self.is_full() {
            panic!("Call to `into_full` on non-full `ArrVec`");
        }
        unsafe { self.into_repr().arr.into_inner().assume_init() }
    }
    #[track_caller]
    pub const fn push_alt(&mut self, item: T) {
        if self.is_full() {
            panic!("Call to `push_alt` on full `ArrVec`");
        }
        let ArrVecRepr { arr, len } = &mut repr!(self);
        arr.as_mut_slice()[*len].write(item);
        *len += 1;
    }
    pub const fn push(&mut self, item: T) -> Result<(), T> {
        match self.is_full() {
            true => Err(item),
            false => Ok(self.push_alt(item)),
        }
    }
    pub const fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        let ArrVecRepr { arr, len } = &mut repr!(self);
        *len -= 1;
        Some(unsafe { arr.as_slice()[*len].assume_init_read() })
    }
}
