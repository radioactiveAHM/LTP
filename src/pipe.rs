use tokio::io::{AsyncRead, AsyncWriteExt, ReadBuf};

#[inline(always)]
pub async fn copy<R, W>(
    r: &mut std::pin::Pin<&mut R>,
    w: &mut std::pin::Pin<&mut W>,
    buf: &mut ReadBuf<'_>,
    fill_buf: bool,
) -> tokio::io::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWriteExt + Unpin,
{
    if fill_buf {
        let res = Fill(r, buf).await;
        if !buf.filled().is_empty() {
            Write(w, buf.filled()).await?;
        }
        res?;
    } else {
        Read(r, buf).await?;
        Write(w, buf.filled()).await?;
    }
    buf.clear();
    Ok(())
}

struct Read<'a, 'b, 'c, R>(&'a mut std::pin::Pin<&'b mut R>, &'a mut ReadBuf<'c>);
impl<'a, 'b, 'c, R> Future for Read<'a, 'b, 'c, R>
where
    R: AsyncRead + Unpin,
{
    type Output = tokio::io::Result<()>;
    #[inline(always)]
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = &mut *self;
        std::task::ready!(this.0.as_mut().poll_read(cx, this.1)).map(|_| {
            if this.1.filled().is_empty() {
                std::task::Poll::Ready(Err(tokio::io::Error::other("Pipe read EOF")))
            } else {
                std::task::Poll::Ready(Ok(()))
            }
        })?
    }
}

struct Fill<'a, 'b, 'c, R>(&'a mut std::pin::Pin<&'b mut R>, &'a mut ReadBuf<'c>);
impl<'a, 'b, 'c, R> Future for Fill<'a, 'b, 'c, R>
where
    R: AsyncRead + Unpin,
{
    type Output = tokio::io::Result<()>;
    #[inline(always)]
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = &mut *self;
        let mut filled = 0;
        loop {
            match this.0.as_mut().poll_read(cx, this.1) {
                std::task::Poll::Pending => {
                    if filled == 0 {
                        return std::task::Poll::Pending;
                    } else {
                        return std::task::Poll::Ready(Ok(()));
                    }
                }
                std::task::Poll::Ready(Ok(_)) => {
                    let fill = this.1.filled().len();
                    if fill == 0 || filled == fill {
                        return std::task::Poll::Ready(Err(tokio::io::Error::other(
                            "Pipe read EOF",
                        )));
                    } else if this.1.remaining() == 0 {
                        return std::task::Poll::Ready(Ok(()));
                    }
                    filled = fill;
                }
                std::task::Poll::Ready(Err(e)) => return std::task::Poll::Ready(Err(e)),
            };
        }
    }
}

pub struct Write<'a, 'b, W>(pub &'a mut std::pin::Pin<&'b mut W>, pub &'a [u8]);
impl<'a, 'b, W> Future for Write<'a, 'b, W>
where
    W: AsyncWriteExt + Unpin,
{
    type Output = tokio::io::Result<usize>;
    #[inline(always)]
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = &mut *self;
        this.0.as_mut().poll_write(cx, this.1)
    }
}
