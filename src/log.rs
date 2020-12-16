use std::io::Write;

pub fn log_init() {
    let env_filter = std::env::var("RUST_LOG").unwrap_or("debug".to_owned());
    env_logger::Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] {}:{} - {}",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.level(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.args(),
            )
        })
        .parse_filters(&env_filter)
        .init();
}
