fn main() {
    println!(
        "cargo:rustc-env=WIT_DIR={}",
        std::env::current_dir().unwrap().to_str().unwrap()
    );
}
