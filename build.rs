use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo::rerun-if-changed=proto/");

    let out_dir = env::var("OUT_DIR").unwrap();
    eprintln!("Protobuf files will be compiled to: {}", out_dir);
    tonic_build::configure()
        .emit_rerun_if_changed(false)// turn this off as new 'cargo::' notation is used above
        .file_descriptor_set_path(PathBuf::from(out_dir).join("game_descriptor.bin"))
        .compile(&["proto/game.proto", "proto/rpc.proto"], &["proto/"])?;
    Ok(())
}
