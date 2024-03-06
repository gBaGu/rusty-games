fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .out_dir("proto")
        .compile(
            &["proto/game.proto"],
            &["proto/"],
        )?;
    Ok(())
}