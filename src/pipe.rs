use tokio::{
    io::{AsyncRead, AsyncWriteExt, ReadBuf},
    time::timeout,
};

#[inline(always)]
pub async fn copy<R, W>(
    mut r: R,
    w: &mut W,
    buff_size: usize,
    timeout_dur: u64,
) -> tokio::io::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWriteExt + Unpin,
{
    let mut buf = vec![0; 1024 * buff_size];
    let mut pinned: std::pin::Pin<&mut R> = std::pin::Pin::new(&mut r);
    let mut wrapper: ReadBuf<'_> = ReadBuf::new(&mut buf);
    loop {
        read(&mut pinned, &mut wrapper, timeout_dur).await?;
        let _ = w.write(wrapper.filled()).await?;
        let _ = w.flush().await;
        wrapper.clear();
    }
}

pub async fn read<R>(
    pinned: &mut std::pin::Pin<&mut R>,
    wrapper: &mut ReadBuf<'_>,
    timeout_dur: u64,
) -> tokio::io::Result<()>
where
    R: AsyncRead + Unpin,
{
    match timeout(std::time::Duration::from_secs(timeout_dur), async {
        std::future::poll_fn(|cx| match pinned.as_mut().poll_read(cx, wrapper) {
            std::task::Poll::Pending => std::task::Poll::Pending,
            std::task::Poll::Ready(Ok(_)) => {
                if wrapper.filled().is_empty() {
                    std::task::Poll::Ready(Err(tokio::io::Error::other("Pipe read EOF")))
                } else {
                    std::task::Poll::Ready(Ok(()))
                }
            }
            std::task::Poll::Ready(Err(e)) => std::task::Poll::Ready(Err(e)),
        })
        .await
    })
    .await
    {
        Ok(v) => v,
        Err(_) => Err(tokio::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "Pipe read timeout",
        )),
    }
}
