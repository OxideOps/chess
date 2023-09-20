use common::args::*;
use dioxus_fullstack::prelude::*;

pub fn main() {
    dioxus_logger::init(Args::parse().log_level).expect("Failed to initialize dioxus logger");
    client::launcher::launch();
}
