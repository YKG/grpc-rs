// Copyright 2019 TiKV Project Authors. Licensed under Apache-2.0.

#[macro_use]
extern crate log;

#[path = "../log_util.rs"]
mod log_util;

use std::io::Read;
use std::sync::{Arc};
use std::{io, thread};

use futures::channel::oneshot;
use futures::executor::block_on;
use futures::prelude::*;
use grpcio::{ChannelBuilder, Environment, ResourceQuota, RpcContext, ServerBuilder, UnarySink, ServerStreamingSink, WriteFlags, ClientStreamingSink, RequestStream, DuplexSink};

use grpcio_proto::example::echo::{EchoResponse, EchoRequest};
use grpcio_proto::example::echo_grpc::{create_echo, Echo};
use futures_timer::Delay;

#[derive(Clone)]
struct EchoService;

fn gen_resp(req : &EchoRequest) -> EchoResponse {
    let mut resp1 = EchoResponse::default();
    resp1.set_message(req.get_message().to_string());
    resp1
}

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

    fn server_streaming_echo(&mut self, ctx: RpcContext<'_>, req: EchoRequest, mut sink: ServerStreamingSink<EchoResponse>) {
        let msg = format!("Hello {}", req.get_message());
        println!("server got: {:?}", msg);
        // let mut resp1 = EchoResponse::default();
        // resp1.set_message(msg);

        let f = async move {
                for i in 0..2 {
                    // println!("server got: {:?}", "msg");
                    let mut resp1 = EchoResponse::default();
                    resp1.set_message(msg.clone() + &*i.to_string());
                    sink.send((resp1, WriteFlags::default())).await?;
                }
                sink.close().await?;
                Ok(())
            }
            .map_err(|e: grpcio::Error| error!("failed to handle listfeatures request: {:?}", e))
            .map(|_| ());
        ctx.spawn(f)
    }

    fn client_streaming_echo(&mut self, ctx: RpcContext<'_>, mut reqs: RequestStream<EchoRequest>, sink: ClientStreamingSink<EchoResponse>) {
        let f = async move {
            while let Some(req) = reqs.try_next().await? {
                let msg = format!("Hello {}", req.get_message());
                println!("server got: {:?}", msg);
            }
            let mut resp1 = EchoResponse::default();
            resp1.set_message("server done".to_string());
            sink.success(resp1).await?;
            Ok(())
        }
        .map_err(|e: grpcio::Error| error!("failed to handle listfeatures request: {:?}", e))
        .map(|_| ());
        ctx.spawn(f)
    }

    // fn bidirectional_streaming_echo(&mut self, ctx: RpcContext<'_>, mut reqs: RequestStream<EchoRequest>, mut sink: DuplexSink<EchoResponse>) {
    //     let f = async move {
    //         while let Some(req) = reqs.try_next().await? {
    //             sink.send(( gen_resp(&req), WriteFlags::default() )).await?;
    //         }
    //         sink.close().await?;
    //         Ok(())
    //     }
    //         .map_err(|e: grpcio::Error| error!("failed to handle listfeatures request: {:?}", e))
    //         .map(|_| ());
    //     ctx.spawn(f)
    // }

    fn bidirectional_streaming_echo(&mut self, ctx: RpcContext<'_>, mut reqs: RequestStream<EchoRequest>, mut sink: DuplexSink<EchoResponse>) {
        // let buf = Arc::new(Mutex::new(Vec::new()));
        // let buf2 = buf.clone();
        let (tx, rx) = futures::channel::oneshot::channel();
        let (mut tx2, mut rx2) = futures::channel::mpsc::channel(100);
        let f = async move {
            while let Some(req) = reqs.try_next().await? {
                println!("recv: {:?}", req);
                // buf.lock().unwrap().push(req);
                tx2.send(req).await.unwrap();
            }
            tx.send(true).unwrap();
            Ok(())
        }
            .map_err(|e: grpcio::Error| error!("failed to handle listfeatures request: {:?}", e))
            .map(|_| ());
        ctx.spawn(f);

        let mut rx = rx.fuse();
        let rr  = async move {
            let mut timeout = Delay::new(std::time::Duration::from_millis(5000)).fuse();
            // let mut interval = time::interval(Duration::from_millis(50));
            let mut v2 = Vec::new();

            sink.enhance_batch(true);
            let mut eof = false;
            loop {
                futures::select! {
                    _ = rx => {
                        eof = true;
                    }
                    _ = timeout => {
                        // println!("operation timed out--------------");
                        // println!("timeo {:?} {:?}", v2, std::time::SystemTime::now());
                    }
                    r = rx2.next() => match r {
                        None => {
                            // println!(">>>>>>>>> batch {:?} GOT NONE", std::time::SystemTime::now());
                            eof = true;
                        },
                        Some(req) => {
                            // println!(">>>>>>>>> batch {:?} {:?} {:?}", std::time::SystemTime::now(), v2, req);
                            v2.push(req);
                            if v2.len() < 1 {
                                continue
                            }
                        }
                    }
                }

                if eof {
                    sink.close().await.expect("err");
                    break
                }

                let length = v2.len();
                let mut stream1 = stream::iter(v2.into_iter().map(|req|{(gen_resp(&req), WriteFlags::default())}).map(Ok));
                println!("sendall >>>>>>>>>>> -------------- {}", length);
                sink.send_all(&mut stream1).await.unwrap();
                timeout = Delay::new(std::time::Duration::from_millis(50)).fuse();
                v2 = Vec::new();
            }
        }
            .map(|_| ());
        ctx.spawn(rr);
    }
}

fn main() {
    let _guard = log_util::init_log(None);
    let env = Arc::new(Environment::new(1));
    let service = create_echo(EchoService);

    let quota = ResourceQuota::new(Some("ServerQuota")).resize_memory(300 * 1024 * 1024);
    let ch_builder = ChannelBuilder::new(env.clone()).set_resource_quota(quota);

    let mut server = ServerBuilder::new(env)
        .register_service(service)
        .bind("0.0.0.0", 50052)
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
