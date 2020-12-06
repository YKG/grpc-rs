// Copyright 2019 TiKV Project Authors. Licensed under Apache-2.0.

#[macro_use]
extern crate log;

#[path = "../log_util.rs"]
mod log_util;

use std::sync::Arc;

use grpcio::{ChannelBuilder, EnvBuilder};
use grpcio_proto::example::echo::EchoRequest;
use grpcio_proto::example::echo_grpc::EchoClient;

fn main() {
    let _guard = log_util::init_log(None);
    let env = Arc::new(EnvBuilder::new().build());
    let ch = ChannelBuilder::new(env).connect("ipv4:127.0.0.1:51051");
    let client = EchoClient::new(ch);

    let mut req = EchoRequest::default();
    req.set_message("world".to_owned());
    let reply = client.unary_echo(&req).expect("rpc");
    info!("Greeter received: {}", reply.get_message());
}
