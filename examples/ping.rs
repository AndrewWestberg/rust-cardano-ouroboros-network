use std::env;
use std::time::{Duration, Instant};

use futures::{
    executor::block_on,
    future::join_all,
};
use futures::io::Error;
use log::{info, error};

/**
© 2020 PERLUR Group

SPDX-License-Identifier: GPL-3.0-only OR LGPL-3.0-only

*/

use cardano_ouroboros_network::mux;
use cardano_ouroboros_network::mux::tcp::Channel;

mod common;

fn main() {
    let cfg = common::init();

    block_on(async {
        let mut args = env::args();

        args.next();
        join_all(args.map(|arg| async {
            let host = arg;
            let port = cfg.port;
            let start = Instant::now();
            match mux::tcp::connect(&host, port, cfg.magic).await {
                Ok((channel, connect_duration)) => {
                    let total_duration = start.elapsed();
                    info!("Ping {}:{} success! : connect_duration: {}, total_duration: {}", &host, port, connect_duration.as_millis(), total_duration.as_millis());
                }
                Err(error) => {
                    error!("Ping {}:{} failed! : {:?}", &host, port, error);
                }
            }
        })).await;
    });
}
