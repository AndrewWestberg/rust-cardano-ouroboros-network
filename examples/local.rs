use futures::executor::block_on;
/**
© 2020 PERLUR Group

SPDX-License-Identifier: GPL-3.0-only OR LGPL-3.0-only

*/
use log::info;

use cardano_ouroboros_network::mux;

mod common;

fn main() {
    let _cfg = common::init();

    block_on(async {
        let channel = mux::unix::connect("/home/westbam/haskell/local/db/socket").await.unwrap();
        channel.handshake(764824073).await.unwrap();
        info!("Ping unix socket success");
    });
}