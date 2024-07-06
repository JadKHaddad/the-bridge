use core::future::Future;

// TODO: make sure AsyncWrite can close the connection so we can call it on sink close
pub trait AsyncWrite {
    type Error;

    fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> impl Future<Output = Result<(), Self::Error>>;
}
