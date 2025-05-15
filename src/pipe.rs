use tokio::io::{AsyncRead, AsyncWriteExt, ReadBuf};

pub async fn stack_copy<R, W>(r: R, w: &mut W, size: usize) -> tokio::io::Result<()> where R: AsyncRead + Unpin, W: AsyncWriteExt + Unpin {
    match size {
        64 => {
            let mut buf = [0; 1024*64];
            copy(r, w, &mut buf).await
        },
        32 => {
            let mut buf = [0; 1024*32];
            copy(r, w, &mut buf).await
        },
        16 => {
            let mut buf = [0; 1024*16];
            copy(r, w, &mut buf).await
        },
        8 => {
            let mut buf = [0; 1024*8];
            copy(r, w, &mut buf).await
        },
        4 => {
            let mut buf = [0; 1024*4];
            copy(r, w, &mut buf).await
        },
        _ => {
            let mut buf = vec![0u8; size*1024];
            copy(r, w, &mut buf).await
        }
    }
}

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