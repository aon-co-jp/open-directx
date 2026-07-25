//! 一時的な調査用ツール。実fxc.exe出力のSHEX命令列を人間に見える形でダンプする
//! (デコーダを一般化する前に「実際に何が出てくるか」を確認するため)。
//! `cargo run -p directx-shader-translate --example dump_shex -- <path.dxbc>`

use dxbc::{scan_dxbc, ChunkData};

fn main() {
    let path = std::env::args().nth(1).expect("usage: dump_shex <path.dxbc>");
    let bytes = std::fs::read(&path).expect("read dxbc file");
    let containers = scan_dxbc(&bytes);
    let container = containers.into_iter().next().expect("no DXBC container found");
    for chunk in &container.chunks {
        if let ChunkData::Shader(program) = chunk.parse() {
            for (i, ins) in program.instructions.iter().enumerate() {
                println!("{i}: opcode={:?} kind={:?}", ins.opcode, ins.kind);
            }
        }
    }
}
