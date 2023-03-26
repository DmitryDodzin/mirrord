fn main() -> std::io::Result<()> {
    prost_build::compile_protos(
        &[
            "proto/error.proto",
            "proto/std_types.proto",
            "proto/tcp.proto",
        ],
        &["proto/"],
    )?;

    Ok(())
}
