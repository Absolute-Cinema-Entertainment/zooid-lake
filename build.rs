use std::env::var_os;

use embed_manifest::{
    embed_manifest,
    manifest::{MaxVersionTested, SupportedOS::Windows10},
    new_manifest,
};

fn main() {
    if var_os("CARGO_CFG_WINDOWS").is_some() {
        embed_manifest(
            new_manifest("Zooid Lake")
                .supported_os(Windows10..=Windows10)
                .max_version_tested(MaxVersionTested::Windows11Version22H2), // .heap_type(HeapType::SegmentHeap),
        )
        .unwrap();
    }

    println!("cargo:rerun-if-changed=build.rs");
}
