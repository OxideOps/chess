use common::args::*;

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

pub fn main() {
    dioxus_logger::init(Args::parse().log_level).expect("Failed to initialize dioxus logger");
    client::launch();
}
