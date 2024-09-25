fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_file = "./proto/media.proto";

    tonic_build::configure().compile(&[proto_file], &["proto"])?;

    Ok(())
}
