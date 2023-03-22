mod tcp_generated;
mod tcp_generated2;

pub mod tcp {
    pub use super::tcp_generated::mirrord_protocol::*;
}
pub mod tcp2 {
    pub use super::tcp_generated2::mirrord_protocol::*;
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use flatbuffers::FlatBufferBuilder;

    use super::*;

    #[test]
    fn simple_message() {
        let mut builder = FlatBufferBuilder::new();

        let mut new_connection_builder = tcp::NewTcpConnectionBuilder::new(&mut builder);

        new_connection_builder.add_connection_id(1);
        new_connection_builder.add_destination_port(80);
        new_connection_builder.add_source_port(1337);
        new_connection_builder.add_remote_address(&tcp::IpAddr(Ipv4Addr::UNSPECIFIED.octets()));
        new_connection_builder.add_local_address(&tcp::IpAddr(Ipv4Addr::LOCALHOST.octets()));

        let new_connection = new_connection_builder.finish();

        let mut message_builder = tcp::DaemonTcpBuilder::new(&mut builder);

        message_builder.add_inner(new_connection.as_union_value());
        message_builder.add_inner_type(tcp::DaemonTcpMessage::NewTcpConnection);

        let tcp_message = message_builder.finish();

        tcp::finish_daemon_tcp_buffer(&mut builder, tcp_message);

        let (buffer, start) = builder.collapse();

        println!("{:#?}", tcp2::root_as_daemon_tcp(&buffer[start..]));

        let mut builder = FlatBufferBuilder::new();

        let mut new_connection_builder = tcp2::NewTcpConnectionBuilder::new(&mut builder);

        new_connection_builder.add_connection_id(1);
        new_connection_builder.add_source_port(1337);
        new_connection_builder.add_remote_address(&tcp2::IpAddr(Ipv4Addr::UNSPECIFIED.octets()));
        new_connection_builder.add_local_address(&tcp2::IpAddr(Ipv4Addr::LOCALHOST.octets()));

        let new_connection = new_connection_builder.finish();

        let mut message_builder = tcp2::DaemonTcpBuilder::new(&mut builder);

        message_builder.add_inner(new_connection.as_union_value());
        message_builder.add_inner_type(tcp2::DaemonTcpMessage::NewTcpConnection);

        let tcp_message = message_builder.finish();

        tcp2::finish_daemon_tcp_buffer(&mut builder, tcp_message);

        let (buffer, start) = builder.collapse();

        println!("{:#?}", tcp::root_as_daemon_tcp(&buffer[start..]));
    }
}
