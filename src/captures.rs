pub trait Captures<U> {}

impl<T: ?Sized, U> Captures<U> for T {}
