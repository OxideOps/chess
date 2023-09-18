fn main() {
    if cfg!(feature = "server") {
        println!("Building client and converting to WASM...");
    }
}
