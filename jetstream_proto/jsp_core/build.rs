use std::process::Command;

fn main() {
    // Check if flatc is available
    let flatc_check = Command::new("flatc")
        .arg("--version")
        .output();
    
    if flatc_check.is_err() {
        println!("cargo:warning=flatc compiler not found. Skipping FlatBuffers code generation.");
        println!("cargo:warning=Install flatc to enable FlatBuffers support:");
        println!("cargo:warning=  Windows: choco install flatbuffers");
        println!("cargo:warning=  Linux: apt-get install flatbuffers-compiler");
        println!("cargo:warning=  macOS: brew install flatbuffers");
        // Don't panic, just skip generation
        return;
    }
    
    let schema_path = "../schemas/messages.fbs";
    let out_dir = "src/serialization/generated";
    
    // Create output directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(out_dir) {
        println!("cargo:warning=Failed to create output directory: {}", e);
        return;
    }
    
    // Generate Rust code from FlatBuffers schema
    let status = Command::new("flatc")
        .args(&[
            "--rust",
            "-o", out_dir,
            schema_path
        ])
        .status();
    
    match status {
        Ok(s) if s.success() => {
            println!("cargo:warning=FlatBuffers code generated successfully");
        }
        Ok(s) => {
            println!("cargo:warning=FlatBuffers generation failed with status: {}", s);
        }
        Err(e) => {
            println!("cargo:warning=Failed to execute flatc: {}", e);
        }
    }
    
    println!("cargo:rerun-if-changed={}", schema_path);
    println!("cargo:rerun-if-changed=build.rs");
}
