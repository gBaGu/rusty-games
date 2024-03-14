use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = env::var("OUT_DIR").unwrap();
    eprintln!("Protobuf files will be compiled to: {}", out_dir);
    tonic_build::configure()
        // .file_descriptor_set_path() //TODO
        .compile(&["proto/game.proto"], &["proto/"])?;
    Ok(())
}
