use crate::{decode::async_read::AsyncRead, encode::async_write::AsyncWrite};
use core::future::Future;
use embedded_io_async::{ErrorType, Read, Write};

pub struct Compat<T>(T);

impl<T> Compat<T> {
    pub fn new(inner: T) -> Self {
        Self(inner)
    }

    pub fn get_ref(&self) -> &T {
        &self.0
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.0
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<R> AsyncRead for Compat<R>
where
    R: Read,
{
    type Error = <R as ErrorType>::Error;

    fn read<'a>(
        &'a mut self,
        buf: &'a mut [u8],
    ) -> impl Future<Output = Result<usize, Self::Error>> {
        self.0.read(buf)
    }
}

impl<W> AsyncWrite for Compat<W>
where
    W: Write,
{
    type Error = <W as ErrorType>::Error;

    fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> impl Future<Output = Result<(), Self::Error>> {
        self.0.write_all(buf)
    }
}
