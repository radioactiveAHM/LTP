use tokio::io::{AsyncRead, AsyncWriteExt, ReadBuf};

#[inline(always)]
pub async fn copy<R, W>(
    mut r: R,
    w: &mut W,
    buf: &mut ReadBuf<'_>,
    fill_buf: bool,
) -> tokio::io::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWriteExt + Unpin,
{
    let mut pinned: std::pin::Pin<&mut R> = std::pin::Pin::new(&mut r);
    if fill_buf {
        fill(&mut pinned, buf).await?;
    } else {
        read(&mut pinned, buf).await?;
    }
    let _ = w.write(buf.filled()).await?;
    buf.clear();
    Ok(())
}

#[inline(always)]
pub async fn read<R>(
    pinned: &mut std::pin::Pin<&mut R>,
    buf: &mut ReadBuf<'_>,
) -> tokio::io::Result<()>
where
    R: AsyncRead + Unpin,
{
    std::future::poll_fn(|cx| match pinned.as_mut().poll_read(cx, buf) {
        std::task::Poll::Pending => std::task::Poll::Pending,
        std::task::Poll::Ready(Ok(_)) => {
            if buf.filled().is_empty() {
                std::task::Poll::Ready(Err(tokio::io::Error::other("Pipe read EOF")))
            } else {
                std::task::Poll::Ready(Ok(()))
            }
        }
        std::task::Poll::Ready(Err(e)) => std::task::Poll::Ready(Err(e)),
    })
    .await
}

#[inline(always)]
pub async fn fill<R>(
    pinned: &mut std::pin::Pin<&mut R>,
    buf: &mut ReadBuf<'_>,
) -> tokio::io::Result<()>
where
    R: AsyncRead + Unpin,
{
    loop {
        tokio::task::yield_now().await;
        if std::future::poll_fn(|cx| match pinned.as_mut().poll_read(cx, buf) {
            std::task::Poll::Pending => {
                if buf.filled().is_empty() {
                    std::task::Poll::Pending
                } else {
                    // nothing to read anymore
                    std::task::Poll::Ready(Ok(true))
                }
            }
            std::task::Poll::Ready(Ok(_)) => {
                if buf.filled().is_empty() {
                    std::task::Poll::Ready(Err(tokio::io::Error::other("EOF")))
                } else if buf.remaining() == 0 {
                    // buf full
                    std::task::Poll::Ready(Ok(true))
                } else {
                    // continue reading
                    std::task::Poll::Ready(Ok(false))
                }
            }
            std::task::Poll::Ready(Err(e)) => std::task::Poll::Ready(Err(e)),
        })
        .await?
        {
            break;
        }
    }
    Ok(())
}
