use core::future::Future;

pub trait AsyncWrite {
    type Error;

    fn write_all<'a>(
        &'a mut self,
        buf: &'a [u8],
    ) -> impl Future<Output = Result<(), Self::Error>> + 'a;
}
