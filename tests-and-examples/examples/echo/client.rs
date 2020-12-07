// Copyright 2019 TiKV Project Authors. Licensed under Apache-2.0.
// #![allow(unused_imports)]
// #![allow(dead_code)]

#[macro_use]
extern crate log;
// #[macro_use]
// extern crate serde_derive;

#[path = "../log_util.rs"]
mod log_util;

use std::sync::Arc;
use futures::prelude::*;

// FIXME ============================
use grpcio::*; // Result<()>
// TODO  ============================

use grpcio::{ChannelBuilder, EnvBuilder};
use grpcio_proto::example::echo::EchoRequest;
use grpcio_proto::example::echo_grpc::EchoClient;

async fn get_stream(client : &EchoClient) -> Result<()> {
    let mut req = EchoRequest::default();
    req.set_message("world2".to_owned());
    let mut list_features = client.server_streaming_echo(&req)?;
    while let Some(feature) = list_features.try_next().await? {
        let msg = feature.get_message();
        info!(
            "Found feature {} at {}",
            msg, "1"
        );
    }
    Ok(())
}

async fn async_main() -> Result<()> {
    let _guard = log_util::init_log(None);
    let env = Arc::new(EnvBuilder::new().build());
    let ch = ChannelBuilder::new(env).connect("ipv4:127.0.0.1:51051");
    let client = EchoClient::new(ch);

    get_stream(&client).await?;

    Ok(())
}

fn main() {
    futures::executor::block_on(async_main()).unwrap();
}
