use std::env;

const PROTO_FOLDER: &str = "../server/proto/";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo::rerun-if-changed={}", PROTO_FOLDER);

    let out_dir = env::var("OUT_DIR").unwrap();
    eprintln!("Protobuf files will be compiled to: {}", out_dir);
    tonic_build::configure()
        .emit_rerun_if_changed(false) // turn this off as new 'cargo::' notation is used above
        .build_server(false)
        .compile(
            &[
                format!("{}/game.proto", PROTO_FOLDER),
                format!("{}/rpc.proto", PROTO_FOLDER),
            ],
            &[PROTO_FOLDER],
        )?;
    Ok(())
}
