// Build script to generate Rust code from protobuf definitions
use std::io::Result;

fn main() -> Result<()> {
    // Configure tonic-build to generate Rust code from .proto files
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/")
        .compile_protos(
            &[
                "inference.proto",
                "worker.proto",
            ],
            &["."],
        )?;

    // Re-run build if any proto file changes
    println!("cargo:rerun-if-changed=inference.proto");
    println!("cargo:rerun-if-changed=worker.proto");

    Ok(())
}
