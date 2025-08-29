use tokio::io::{AsyncRead, AsyncWriteExt, ReadBuf};

#[inline(always)]
pub async fn copy<R, W>(
    mut r: R,
    w: &mut W,
    buff_size: usize,
    fill_buf: bool,
) -> tokio::io::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWriteExt + Unpin,
{
    let mut buf = vec![0; 1024 * buff_size];
    let mut pinned: std::pin::Pin<&mut R> = std::pin::Pin::new(&mut r);
    let mut wrapper: ReadBuf<'_> = ReadBuf::new(&mut buf);
    loop {
        tokio::task::yield_now().await;
        if fill_buf {
            fill(&mut pinned, &mut wrapper).await?;
        } else {
            read(&mut pinned, &mut wrapper).await?;
        }
        let _ = w.write(wrapper.filled()).await?;
        let _ = w.flush().await;
        wrapper.clear();
    }
}

#[inline(always)]
pub async fn read<R>(
    pinned: &mut std::pin::Pin<&mut R>,
    wrapper: &mut ReadBuf<'_>,
) -> tokio::io::Result<()>
where
    R: AsyncRead + Unpin,
{
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
}

#[inline(always)]
pub async fn fill<R>(
    pinned: &mut std::pin::Pin<&mut R>,
    wrapper: &mut ReadBuf<'_>,
) -> tokio::io::Result<()>
where
    R: AsyncRead + Unpin,
{
    loop {
        tokio::task::yield_now().await;
        if std::future::poll_fn(|cx| match pinned.as_mut().poll_read(cx, wrapper) {
            std::task::Poll::Pending => {
                if wrapper.filled().is_empty() {
                    std::task::Poll::Pending
                } else {
                    // nothing to read anymore
                    std::task::Poll::Ready(Ok(true))
                }
            }
            std::task::Poll::Ready(Ok(_)) => {
                if wrapper.filled().is_empty() {
                    std::task::Poll::Ready(Err(tokio::io::Error::other("EOF")))
                } else if wrapper.remaining() == 0 {
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
