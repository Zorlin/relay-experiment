fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=src/identity.proto"); // Rerun if proto file changes
    prost_build::compile_protos(&["src/identity.proto"], &["src/"])?;
    Ok(())
}
