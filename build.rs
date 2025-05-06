fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tell Cargo to re-run this build script if the proto files change
    println!("cargo:rerun-if-changed=src/identity.proto");
    println!("cargo:rerun-if-changed=src/webrtc_signaling_proto.proto");

    // Generate Rust code from the proto files
    prost_build::compile_protos(&["src/identity.proto", "src/webrtc_signaling_proto.proto"], &["src/"])?;
    Ok(())
}
