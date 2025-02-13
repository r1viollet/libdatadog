// Unless explicitly stated otherwise all files in this repository are licensed under the Apache License Version 2.0.
// This product includes software developed at Datadog (https://www.datadoghq.com/). Copyright 2021-Present Datadog, Inc.

#[cfg(not(windows))]
use criterion::{criterion_group, criterion_main, Criterion};
#[cfg(not(windows))]
use datadog_ipc::example_interface::{
    ExampleInterfaceRequest, ExampleInterfaceResponse, ExampleServer, ExampleTransport,
};
#[cfg(not(windows))]
use std::{
    os::unix::net::UnixStream as StdUnixStream,
    thread::{self},
};
#[cfg(not(windows))]
use tokio::{net::UnixStream, runtime};

#[cfg(not(windows))]
fn criterion_benchmark(c: &mut Criterion) {
    let (sock_a, sock_b) = StdUnixStream::pair().unwrap();

    let worker = thread::spawn(move || {
        let rt = runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let _g = rt.enter();
        sock_a.set_nonblocking(true).unwrap();
        let socket = UnixStream::from_std(sock_a).unwrap();
        let server = ExampleServer::default();

        rt.block_on(server.accept_connection(socket));
    });

    let mut transport = ExampleTransport::from(sock_b);
    transport.set_nonblocking(false).unwrap();

    c.bench_function("write only interface", |b| {
        b.iter(|| transport.send(ExampleInterfaceRequest::Notify {}).unwrap())
    });

    c.bench_function("two way interface", |b| {
        b.iter(|| transport.call(ExampleInterfaceRequest::ReqCnt {}).unwrap())
    });

    let requests_received = match transport.call(ExampleInterfaceRequest::ReqCnt {}).unwrap() {
        ExampleInterfaceResponse::ReqCnt(cnt) => cnt,
        _ => panic!("shouldn't happen"),
    };

    println!("Total requests handled: {requests_received}");

    drop(transport);
    worker.join().unwrap();
}

#[cfg(not(windows))]
criterion_group!(benches, criterion_benchmark);

#[cfg(not(windows))]
criterion_main!(benches);

#[cfg(windows)]
fn main() {
    println!("IPC benches not implemented for Windows")
}
