fn main() {
    cxx_build::bridge("src/lib.rs")
        .file("src/jetstream.cc")
        .flag_if_supported("-std=c++14")
        .compile("jsp_cpp");

    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/jetstream.cc");
    println!("cargo:rerun-if-changed=include/jetstream.h");
}
