// Copyright 2019 TiKV Project Authors. Licensed under Apache-2.0.

#[macro_use]
extern crate log;

#[path = "../log_util.rs"]
mod log_util;

use std::io::Read;
use std::sync::Arc;
use std::{io, thread};

use futures::channel::oneshot;
use futures::executor::block_on;
use futures::prelude::*;
use grpcio::{ChannelBuilder, Environment, ResourceQuota, RpcContext, ServerBuilder, UnarySink};

use grpcio_proto::example::echo::{EchoResponse, EchoRequest};
use grpcio_proto::example::echo_grpc::{create_echo, Echo};

#[derive(Clone)]
struct EchoService;

impl Echo for EchoService {
    fn unary_echo(&mut self, ctx: RpcContext<'_>, req: EchoRequest, sink: UnarySink<EchoResponse>) {
        let msg = format!("Hello {}", req.get_message());
        println!("server got: {:?}", msg);
        let mut resp = EchoResponse::default();
        resp.set_message(msg);
        let f = sink
            .success(resp)
            .map_err(move |e| error!("failed to reply {:?}: {:?}", req, e))
            .map(|_| ());
        ctx.spawn(f)
    }
}

fn main() {
    let _guard = log_util::init_log(None);
    let env = Arc::new(Environment::new(1));
    let service = create_echo(EchoService);

    let quota = ResourceQuota::new(Some("ServerQuota")).resize_memory(1024 * 1024);
    let ch_builder = ChannelBuilder::new(env.clone()).set_resource_quota(quota);

    let mut server = ServerBuilder::new(env)
        .register_service(service)
        .bind("0.0.0.0", 51051)
        .channel_args(ch_builder.build_args())
        .build()
        .unwrap();
    server.start();
    for (host, port) in server.bind_addrs() {
        info!("listening on {}:{}", host, port);
    }
    let (tx, rx) = oneshot::channel();
    thread::spawn(move || {
        info!("Press 1ENTER to exit...");
        let _ = io::stdin().read(&mut [0]).unwrap();
        tx.send(())
    });
    let _ = block_on(rx);
    let _ = block_on(server.shutdown());
}
