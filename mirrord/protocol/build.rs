fn main() -> std::io::Result<()> {
    prost_build::compile_protos(
        &["proto/error.proto", "proto/net.proto", "proto/tcp.proto"],
        &["proto/"],
    )?;

    Ok(())
}
