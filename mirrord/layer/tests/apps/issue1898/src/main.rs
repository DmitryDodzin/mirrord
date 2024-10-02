use std::net::{Shutdown, SocketAddr, TcpStream as SyncTcpStream};

use async_std::net::TcpStream as AsyncTcpStream;
use thread_async::ThreadFutureExt;

fn main() {
    println!("test issue 1898: START");

    let socket_addr: SocketAddr = "1.2.3.4:80".parse().unwrap();
    let second_socket_addr: SocketAddr = "2.3.4.5:80".parse().unwrap();

    let stream = SyncTcpStream::connect(socket_addr).expect("sync tcp stream was not created");

    let async_stream = AsyncTcpStream::connect(second_socket_addr)
        .thread_await()
        .expect("sync tcp stream was not created");

    stream
        .shutdown(Shutdown::Both)
        .expect("unable to shutdown sync tcp stream");

    async_stream
        .shutdown(Shutdown::Both)
        .expect("unable to shutdown sync tcp stream");

    println!("test issue 1898: SUCCESS");
}
