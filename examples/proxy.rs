use hsproxy::{log::log_init, upgrade_to_proxied_tcp_stream};
use log::info;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

async fn copy<R: AsyncReadExt + Unpin, W: AsyncWriteExt + Unpin>(
    mut r: R,
    mut w: W,
) -> Result<(), anyhow::Error> {
    let mut buf = bytes::BytesMut::with_capacity(2048);
    loop {
        buf.clear();
        let n = r.read_buf(&mut buf).await?;
        if n == 0 {
            return Ok(());
        }
        w.write_all(&buf[..n]).await?;
    }
}

async fn handle_conn(conn: TcpStream) -> Result<(), anyhow::Error> {
    let proxy_stream = upgrade_to_proxied_tcp_stream(conn).await?;
    let remote_conn = TcpStream::connect(proxy_stream.connect_addr).await?;
    let (remote_rh, mut remote_wh) = remote_conn.into_split();
    let (rh, mut wh) = proxy_stream.stream.into_split();
    if proxy_stream.is_connect {
        info!("=== returning ok");
        let resp = b"HTTP/1.1 200 OK\r\n\r\n";
        wh.write_all(&resp[..]).await?;
    } else {
        info!("=== buf len: {}", &proxy_stream.buf.len());
        remote_wh.write_all(&proxy_stream.buf).await?;
    }
    tokio::spawn(copy(remote_rh, wh));
    tokio::spawn(copy(rh, remote_wh));

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    std::env::set_var("RUST_LOG", "debug");
    log_init();
    info!("http&socks proxy starting");
    let addr: SocketAddr = "127.0.0.1:8989".parse().unwrap();
    let listener = TcpListener::bind(addr).await?;
    while let Ok((conn, peer)) = listener.accept().await {
        info!("handling {:?}", peer);
        let fut = async move {
            handle_conn(conn)
                .await
                .map_err(|e| info!("handling error: {:?}", e))
        };
        tokio::spawn(fut);
    }
    Ok(())
}
