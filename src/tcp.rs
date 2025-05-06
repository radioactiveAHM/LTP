pub struct TcpBiGeneric<'a, W> {
    pub io: std::pin::Pin<&'a mut W>,
    pub signal: tokio::sync::mpsc::Sender<()>,
}
impl<W> tokio::io::AsyncWrite for TcpBiGeneric<'_, W>
where
    W: tokio::io::AsyncWrite + Unpin + Send,
{
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let _ = self.signal.try_send(());
        self.io.as_mut().poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        self.io.as_mut().poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        self.io.as_mut().poll_shutdown(cx)
    }
}

impl<W> tokio::io::AsyncRead for TcpBiGeneric<'_, W>
where
    W: tokio::io::AsyncRead + Unpin + Send,
{
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let _ = self.signal.try_send(());
        self.io.as_mut().poll_read(cx, buf)
    }
}