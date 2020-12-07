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
use std::time::{Instant};

async fn get_stream(client : &EchoClient) -> Result<()> {
    let mut req = EchoRequest::default();
    req.set_message("server stream ".to_owned());
    info!("client send the requet >>>>>");
    let start = Instant::now();
    let mut list_features = client.server_streaming_echo(&req)?;
    while let Some(feature) = list_features.try_next().await? {
        let msg = feature.get_message();
        info!(
            "Found feature {} ",
            msg
        );
    }
    info!("client recv done       <<<<< {:?} server_stream\n", start.elapsed());
    Ok(())
}


async fn client_stream(client : &EchoClient) -> Result<()> {
    let mut req = EchoRequest::default();
    req.set_message("client stream ".to_owned());
    info!("client send the requet >>>>>");
    let start = Instant::now();
    let (mut sink, receiver) = client.client_streaming_echo()?;

    for _i in 1..3 {
        sink.send((req.to_owned(), WriteFlags::default() )).await?;
    }
    sink.close().await?;
    let result = receiver.await?;
    info!("result from server: {:?}", result);
    info!("client recv done       <<<<< {:?} client_stream\n", start.elapsed());
    Ok(())
}

async fn bi_stream(client : &EchoClient) -> Result<()> {

    info!("client send the requet >>>>>");
    let start = Instant::now();
    let (mut sink, mut receiver) = client.bidirectional_streaming_echo()?;

    let send = async move {
        let mut req = EchoRequest::default();

        let mut _i: i32 = 0;
        while _i < 3 {
            _i += 1;
            req.set_message("client bi_stream ".to_owned() + &*_i.to_string());
            sink.send((req.to_owned(), WriteFlags::default())).await?;
        }

        sink.close().await?;
        Ok(()) as Result<_>
    };

    let recv = async {
        while let Some(feature) = receiver.try_next().await? {
            let msg = feature.get_message();
            info!(
                "bi_stream recv: {} ",
                msg
            );
        }
        Ok(()) as Result<_>
    };
    let (sr, rr) = futures::join!(send, recv);
    sr.and(rr)?;
    info!("client recv done       <<<<< {:?} bi_stream\n", start.elapsed());
    Ok(())
}

async fn async_main() -> Result<()> {
    let _guard = log_util::init_log(None);
    let env = Arc::new(EnvBuilder::new().build());
    let ch = ChannelBuilder::new(env).connect("ipv4:127.0.0.1:50051");
    let client = EchoClient::new(ch);

    // get_stream(&client).await?;
    // client_stream(&client).await?;
    bi_stream(&client).await?;

    Ok(())
}

fn main() {
    futures::executor::block_on(async_main()).unwrap();
}
