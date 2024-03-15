use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = env::var("OUT_DIR").unwrap();
    eprintln!("Protobuf files will be compiled to: {}", out_dir);
    tonic_build::configure()
        .file_descriptor_set_path(PathBuf::from(out_dir).join("game_descriptor.bin"))
        .compile(&["proto/game.proto"], &["proto/"])?;
    Ok(())
}
