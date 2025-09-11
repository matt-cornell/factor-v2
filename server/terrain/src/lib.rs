use std::num::NonZero;
use std::ptr::NonNull;

pub mod biome;
pub mod climate;
pub mod noise;

#[derive(Debug, Clone, PartialEq)]
pub struct SphereSurface<T> {
    res: NonZero<usize>,
    inner: NonNull<T>,
}
impl<T> SphereSurface<T> {
    /// Create a surface from a resolution and inner pointer
    ///
    /// ## Safety
    /// `inner` must be a valid pointer to `[T; res * res * 2]`.
    pub const unsafe fn from_raw(res: NonZero<usize>, inner: NonNull<T>) -> Self {
        Self { res, inner }
    }
    /// Create a surface from the boxed data.
    ///
    /// The data should have `res * res * 2`, or else it's returned as an `Err`.
    pub fn from_data(res: usize, data: Box<[T]>) -> Result<Self, Box<[T]>> {
        let Some(nzr) = NonZero::new(res) else {
            return Err(data);
        };
        if res != 0
            && res
                .checked_mul(res)
                .and_then(|v| v.checked_shl(1))
                .is_some_and(|v| v == data.len())
        {
            // SAFETY: we just checked the length
            unsafe {
                Ok(Self::from_raw(
                    nzr,
                    NonNull::new_unchecked(Box::into_raw(data) as *mut T),
                ))
            }
        } else {
            Err(data)
        }
    }
    #[expect(
        clippy::len_without_is_empty,
        reason = "A sphere surface is always non-empty"
    )]
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.len_nonzero().get()
    }
    pub const fn len_nonzero(&self) -> NonZero<usize> {
        unsafe {
            let res = self.res.get();
            NonZero::new_unchecked(res.unchecked_mul(res).unchecked_mul(2))
        }
    }
    #[inline(always)]
    pub const fn resolution(&self) -> usize {
        self.res.get()
    }
    #[inline(always)]
    pub const fn resolution_nonzero(&self) -> NonZero<usize> {
        self.res
    }
    #[inline(always)]
    pub const fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.inner.as_ptr(), self.len()) }
    }
    #[inline(always)]
    pub const fn as_slice_mut(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.inner.as_ptr(), self.len()) }
    }
}
impl<T> Drop for SphereSurface<T> {
    fn drop(&mut self) {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                self.inner.as_ptr(),
                self.len(),
            )));
        }
    }
}
