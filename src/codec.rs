use core::marker::PhantomData;

pub struct Codec<M> {
    _phantom: PhantomData<M>,
}

impl<M> Codec<M> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<M> Default for Codec<M> {
    fn default() -> Self {
        Self::new()
    }
}
