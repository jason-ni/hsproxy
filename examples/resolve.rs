use dns_lookup::lookup_host;
use hsproxy::log::log_init;
use log::trace;

fn main() {
    log_init();
    let host = "localhost";
    let ips = lookup_host(host).unwrap();
    trace!("localhost ips: {:#?}", ips);
    let ips = lookup_host("baidu.com").unwrap();
    trace!("baidu.com ips: {:#?}", ips);
}
