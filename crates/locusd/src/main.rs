// SPDX-License-Identifier: AGPL-3.0-only
//! Binary entry point. Everything worth testing lives in the library beside it.

use std::net::SocketAddr;
use tiny_http::Server;

fn main() {
    let addr: SocketAddr = std::env::var("LOCUSD_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8787".to_string())
        .parse()
        .unwrap_or_else(|e| {
            eprintln!("locusd: LOCUSD_ADDR is not a socket address: {e}");
            std::process::exit(2);
        });

    let server = Server::http(addr).unwrap_or_else(|e| {
        eprintln!("locusd: cannot listen on {addr}: {e}");
        std::process::exit(1);
    });
    println!("locusd {} listening on {addr}", env!("CARGO_PKG_VERSION"));
    locusd::serve(&server);
}
