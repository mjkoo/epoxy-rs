#[link(name = "epoxy")] extern {}

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
