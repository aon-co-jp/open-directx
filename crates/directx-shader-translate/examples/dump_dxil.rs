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
                            // TYPE_BLOCK_ID_NEW(17) と FUNCTION_BLOCK(12) は
                            // 今回の翻訳作業で実際に必要な中身なので、レコード単位
                            // (code + fields値そのもの)まで掘り下げてダンプする。
                            if sub.id == 17 || sub.id == 12 || sub.id == 11 {
                                for (idx, deeper) in sub.elements.iter().enumerate() {
                                    if let Some(rec) = deeper.as_record() {
                                        println!(
                                            "      [{}] record code={} fields={:?}",
                                            idx,
                                            rec.id,
                                            rec.fields()
                                        );
                                    } else if let Some(deeper_block) = deeper.as_block() {
                                        println!(
                                            "      [{}] sub-sub-block id={} elements={}",
                                            idx,
                                            deeper_block.id,
                                            deeper_block.elements.len()
                                        );
                                        if deeper_block.id == 11 {
                                            for (jdx, deepest) in deeper_block.elements.iter().enumerate() {
                                                if let Some(rec) = deepest.as_record() {
                                                    println!(
                                                        "        [{}] const record code={} fields={:?}",
                                                        jdx,
                                                        rec.id,
                                                        rec.fields()
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else if let Some(rec) = inner.as_record() {
                            println!("    record id={} fields={:?}", rec.id, rec.fields());
                        }
                        if let Some(sub) = inner.as_block() {
                            if sub.id == 14 {
                                let mut sub_clone = sub.clone();
                                for (idx, deeper) in sub_clone.elements.iter_mut().enumerate() {
                                    if let Some(rec) = deeper.as_record_mut() {
                                        let payload = rec.take_payload();
                                        println!(
                                            "      [{}] VST record code={} fields={:?} payload={:?}",
                                            idx,
                                            rec.id,
                                            rec.fields(),
                                            payload
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("llvm-bitcode: FAILED to parse: {:?}", e);
        }
    }

    let module = directx_shader_translate::dxil::parse_dxil_container(&bytes).unwrap();
    let _ = module;
    let bc2 = Bitcode::new(&dxil_chunk.bitcode).unwrap();
    let module_block = bc2.elements.iter().find_map(|el| el.as_block()).unwrap();
    let type_block = module_block.elements.iter().filter_map(|el| el.as_block()).find(|b| b.id == 17).unwrap();
    let types = directx_shader_translate::dxil::resolve_type_table(type_block);
    for (idx, ty) in types.iter().enumerate() {
        println!("type[{idx}] = {ty:?}");
    }
}
