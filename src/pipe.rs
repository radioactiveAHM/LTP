use tokio::io::{AsyncRead, AsyncWriteExt, ReadBuf};

#[inline(always)]
pub async fn stack_copy<R, W>(r: R, w: &mut W, size: usize) -> tokio::io::Result<()> where R: AsyncRead + Unpin, W: AsyncWriteExt + Unpin {
    match size {
        1024 => {
            on_stack_1024(r, w).await
        },
        512 => {
            on_stack_512(r, w).await
        },
        256 => {
            on_stack_256(r, w).await
        },
        128 => {
            on_stack_128(r, w).await
        },
        64 => {
            on_stack_64(r, w).await
        },
        32 => {
            on_stack_32(r, w).await
        },
        16 => {
            on_stack_16(r, w).await
        },
        8 => {
            on_stack_8(r, w).await
        },
        4 => {
            on_stack_4(r, w).await
        },
        _ => {
            let mut buf = vec![0; 1024*size];
            copy(r, w, &mut buf).await
        }
    }
}

#[inline(never)]
async fn on_stack_1024<R, W>(r: R, w: &mut W) -> tokio::io::Result<()> where R: AsyncRead + Unpin, W: AsyncWriteExt + Unpin {
    let mut buf = vec![0; 1024*1024];
    copy(r, w, &mut buf).await
}

#[inline(never)]
async fn on_stack_512<R, W>(r: R, w: &mut W) -> tokio::io::Result<()> where R: AsyncRead + Unpin, W: AsyncWriteExt + Unpin {
    let mut buf = vec![0; 1024*512];
    copy(r, w, &mut buf).await
}

#[inline(never)]
async fn on_stack_256<R, W>(r: R, w: &mut W) -> tokio::io::Result<()> where R: AsyncRead + Unpin, W: AsyncWriteExt + Unpin {
    let mut buf = vec![0; 1024*256];
    copy(r, w, &mut buf).await
}

#[inline(never)]
async fn on_stack_128<R, W>(r: R, w: &mut W) -> tokio::io::Result<()> where R: AsyncRead + Unpin, W: AsyncWriteExt + Unpin {
    let mut buf = vec![0; 1024*128];
    copy(r, w, &mut buf).await
}

#[inline(never)]
async fn on_stack_64<R, W>(r: R, w: &mut W) -> tokio::io::Result<()> where R: AsyncRead + Unpin, W: AsyncWriteExt + Unpin {
    let mut buf = vec![0; 1024*64];
    copy(r, w, &mut buf).await
}

#[inline(never)]
async fn on_stack_32<R, W>(r: R, w: &mut W) -> tokio::io::Result<()> where R: AsyncRead + Unpin, W: AsyncWriteExt + Unpin {
    let mut buf = vec![0; 1024*32];
    copy(r, w, &mut buf).await
}

#[inline(never)]
async fn on_stack_16<R, W>(r: R, w: &mut W) -> tokio::io::Result<()> where R: AsyncRead + Unpin, W: AsyncWriteExt + Unpin {
    let mut buf = vec![0; 1024*16];
    copy(r, w, &mut buf).await
}

#[inline(never)]
async fn on_stack_8<R, W>(r: R, w: &mut W) -> tokio::io::Result<()> where R: AsyncRead + Unpin, W: AsyncWriteExt + Unpin {
    let mut buf = vec![0; 1024*8];
    copy(r, w, &mut buf).await
}

#[inline(never)]
async fn on_stack_4<R, W>(r: R, w: &mut W) -> tokio::io::Result<()> where R: AsyncRead + Unpin, W: AsyncWriteExt + Unpin {
    let mut buf = vec![0; 1024*4];
    copy(r, w, &mut buf).await
}

#[inline(always)]
async fn copy<R, W>(mut r: R, w: &mut W, buf: &mut [u8]) -> tokio::io::Result<()> where R: AsyncRead + Unpin, W: AsyncWriteExt + Unpin {
    let mut pinned = std::pin::Pin::new(&mut r);
    let mut wrapper = ReadBuf::new(buf);
    loop {
        std::future::poll_fn(|cx| {
            match pinned.as_mut().poll_read(cx, &mut wrapper) {
                std::task::Poll::Pending => std::task::Poll::Pending,
                std::task::Poll::Ready(Ok(_)) => {
                    if wrapper.filled().is_empty() {
                        std::task::Poll::Pending
                    } else {
                        std::task::Poll::Ready(Ok(()))
                    }
                },
                std::task::Poll::Ready(Err(e)) => std::task::Poll::Ready(Err(e))
            }
        }).await?;
        w.write(wrapper.filled()).await?;
        wrapper.clear();
    }
}