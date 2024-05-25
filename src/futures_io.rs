use crate::{decode::async_read::AsyncRead, encode::async_write::AsyncWrite};
use core::future::Future;
use futures::io::{
    AsyncRead as FuturesAsyncRead, AsyncReadExt, AsyncWrite as FuturesAsyncWrite, AsyncWriteExt,
    Error as IoError,
};

pub struct Compat<T>(T);

impl<T> Compat<T> {
    pub fn new(inner: T) -> Self {
        Self(inner)
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> AsRef<T> for Compat<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> AsMut<T> for Compat<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<R> AsyncRead for Compat<R>
where
    R: FuturesAsyncRead + Unpin,
{
    type Error = IoError;

    fn read<'a>(
        &'a mut self,
        buf: &'a mut [u8],
    ) -> impl Future<Output = Result<usize, Self::Error>> {
        self.0.read(buf)
    }
}

impl<W> AsyncWrite for Compat<W>
where
    W: FuturesAsyncWrite + Unpin,
{
    type Error = IoError;

    fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> impl Future<Output = Result<(), Self::Error>> {
        self.0.write_all(buf)
    }
}
