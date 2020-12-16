use ::log::trace;
use bytes::BytesMut;
use dns_lookup::lookup_host;
use std::net::SocketAddr;
use tokio::io::AsyncReadExt;
pub mod log;

pub struct ProxyWrapper<S> {
    pub stream: S,
    pub buf: BytesMut,
    pub is_connect: bool,
    pub connect_addr: SocketAddr,
}

pub async fn upgrade_to_proxied_tcp_stream<S: AsyncReadExt + Unpin>(
    mut stream: S,
) -> Result<ProxyWrapper<S>, anyhow::Error> {
    let mut buf = BytesMut::with_capacity(1024);

    stream.read_buf(&mut buf).await?;
    trace!("=== buf: {}", pretty_hex::pretty_hex(&buf.as_ref()));
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut req = httparse::Request::new(&mut headers);
    req.parse(&buf)?;
    trace!("=== req.path: {:?}", req.path);
    trace!("=== req.headers: {:?}", req.headers);
    let (host, port, is_connect) = match req.method {
        Some("CONNECT") => match req.path {
            Some(path) => {
                let parts: Vec<_> = path.split(":").collect();
                if parts.len() != 2 {
                    (parts[0].to_owned(), 443u16, true)
                } else {
                    (parts[0].to_owned(), parts[1].parse::<u16>()?, true)
                }
            }
            None => anyhow::bail!("invalid request on parsing CONNECT path"),
        },
        _ => {
            let path = match req.path {
                Some(path) => path,
                None => anyhow::bail!("invalid request"),
            };
            let u = url::Url::parse(path)?;
            match u.host_str() {
                Some(host) => match u.port() {
                    Some(port) => (host.to_owned(), port, false),
                    None => (host.to_owned(), 80u16, false),
                },
                None => anyhow::bail!("invalid request"),
            }
        }
    };
    let addrs = tokio::task::block_in_place(|| lookup_host(&host))?;
    match addrs.first() {
        Some(ip_addr) => {
            let connect_addr = SocketAddr::new(ip_addr.to_owned(), port);
            Ok(ProxyWrapper {
                stream,
                buf,
                is_connect,
                connect_addr,
            })
        }
        None => anyhow::bail!("failed to find ip for {}", host),
    }
}
