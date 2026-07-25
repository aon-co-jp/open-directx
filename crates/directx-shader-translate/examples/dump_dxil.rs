//! 調査用の一時ツール。実dxc.exe出力のDXILチャンクを`dxbc`クレートで
//! コンテナレベルまでパースし、その後`llvm-bitcode`クレートで
//! LLVM bitstreamのブロック/レコード木がどこまで読めるか実際に試す。
//! `cargo run -p directx-shader-translate --example dump_dxil -- <path.dxil>`

use dxbc::{scan_dxbc, ChunkData};
use llvm_bitcode::Bitcode;

fn main() {
    let path = std::env::args().nth(1).expect("usage: dump_dxil <path.dxil>");
    let bytes = std::fs::read(&path).expect("read dxil file");
    let containers = scan_dxbc(&bytes);
    let container = containers.into_iter().next().expect("no DXBC container found");
    println!("chunks: {}", container.chunks.len());
    for chunk in &container.chunks {
        println!("  fourcc={:?}", chunk.fourcc_str());
    }
    let dxil_chunk = container
        .chunks
        .iter()
        .find_map(|c| match c.parse() {
            ChunkData::Dxil(d) => Some(d),
            _ => None,
        })
        .expect("no DXIL chunk found");
    println!(
        "DXIL: shader_kind={} SM{}.{} dxil_version={} bitcode_bytes={}",
        dxil_chunk.shader_kind,
        dxil_chunk.major_version,
        dxil_chunk.minor_version,
        dxil_chunk.dxil_version,
        dxil_chunk.bitcode.len()
    );
    println!(
        "bitcode first 8 bytes (should be LLVM wrapper magic 'BC\\xC0\\xDE'): {:02x?}",
        &dxil_chunk.bitcode[..8.min(dxil_chunk.bitcode.len())]
    );

    match Bitcode::new(&dxil_chunk.bitcode) {
        Ok(bc) => {
            println!("llvm-bitcode: parsed top-level bitstream successfully");
            println!("  top-level elements: {}", bc.elements.len());
            for el in &bc.elements {
                if let Some(block) = el.as_block() {
                    println!("  block id={} elements={}", block.id, block.elements.len());
                    for inner in &block.elements {
                        if let Some(sub) = inner.as_block() {
                            println!("    sub-block id={} elements={}", sub.id, sub.elements.len());
                        } else if let Some(rec) = inner.as_record() {
                            println!("    record id={} fields={}", rec.id, rec.fields().len());
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("llvm-bitcode: FAILED to parse: {:?}", e);
        }
    }
}
