use profiling::puffin;

use super::{log_write, LogLevel};

pub fn enable_profiling() {
    puffin::set_scopes_on(true);
    let server_addr = format!("127.0.0.1:{}", puffin_http::DEFAULT_PORT);
    match puffin_http::Server::new(&server_addr) {
        Ok(server) => {
            log_write(format!("Run this to view profiling data: `puffin_viewer --url {server_addr}`"), LogLevel::Debug);
            let child = std::process::Command::new("puffin_viewer")
                .arg("--url")
                .arg(&server_addr)
                .spawn();
            if child.is_err() {
                log_write("Failed to run `puffin_viewer`. Run `cargo install puffin_viewer` if you didn't install it", LogLevel::Error);
            }
            std::mem::forget(server);
        },
        Err(err) => {
            log_write(err, LogLevel::Error);
        },
    }
}
