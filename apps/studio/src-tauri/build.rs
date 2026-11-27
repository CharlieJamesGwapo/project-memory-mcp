fn main() {
    tauri_build::build();
    if !tauri_build::is_dev() {
        println!("cargo:rerun-if-changed=../dist");
        assert!(
            std::path::Path::new("../dist/index.html").is_file(),
            "Build the frontend before packaging Studio"
        );
    }
}
