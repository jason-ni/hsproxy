extern crate env_logger;
extern crate futures;
extern crate thrussh;
extern crate thrussh_keys;
extern crate tokio;
use anyhow::Context;
use futures::StreamExt;
use hsproxy;
use log::info;
use std::sync::Arc;
use thrussh::client::channel::ChannelExt;
use thrussh::client::tunnel::{handle_connect, upgrade_to_remote_forward_tcpip_listener};
use thrussh::*;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

#[tokio::main]
async fn main() {
    hsproxy::log::log_init();
    let config = thrussh::client::Config::default();
    let config = Arc::new(config);
    let sh = client::tunnel::TunnelClient::new();

    let kp = thrussh_keys::load_secret_key("id_rsa", None).unwrap();
    let mut session = thrussh::client::connect(config, "127.0.0.1:2222", sh)
        .await
        .unwrap();
    let kp_ref = Arc::new(kp);
    let auth_res = session
        .authenticate_publickey("jason", kp_ref)
        .await
        .unwrap();
    println!("=== auth: {}", auth_res);
    let channel = session
        .channel_open_session()
        .await
        .context("open channel")
        .unwrap();
    let mut listener = upgrade_to_remote_forward_tcpip_listener(channel, "127.0.0.1", 8989)
        .await
        .unwrap();
    while let Some(channel) = listener.next().await {
        info!("=== handling channel: {:?}", &channel);
        tokio::spawn(handle_connect(channel, (), |ch, _| {
            let fut = async move {
                let (ch_rh, mut ch_wh) = ch.split()?;
                let hsproxy::ProxyWrapper {
                    stream,
                    buf,
                    is_connect,
                    connect_addr,
                } = hsproxy::upgrade_to_proxied_tcp_stream(ch_rh).await.unwrap();
                let mut remote_conn = TcpStream::connect(connect_addr).await?;
                if is_connect {
                    let resp = b"HTTP/1.1 200 OK\r\n\r\n";
                    ch_wh.write_all(&resp[..]).await?;
                } else {
                    remote_conn.write_all(&buf).await?;
                }

                Ok((remote_conn, stream, ch_wh))
            };
            fut
        }));
    }
    let _ = session
        .disconnect(Disconnect::ByApplication, "", "English")
        .await
        .map_err(|e| {
            println!("=== {:#?}", e);
        });
}
