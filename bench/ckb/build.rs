//! Prepare the fixed guest for block-at-a-time multilinear evaluation in CKB-VM.

use miniz_oxide::deflate::compress_to_vec;
use std::{collections::HashMap, env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let guest = aggregation::unified_guest();
    let table = guest.stacked_bytecode();
    let vars = table.len().ilog2();
    let mut data = Vec::new();
    let mut unique = HashMap::new();
    let mut blocks = Vec::new();
    let mut max_block = 0;
    for block in guest.bytecode().__blocks() {
        let range = unique.entry(block).or_insert_with(|| {
            let mut bytes = Vec::new();
            for (word, positions) in block {
                bytes.extend(word.to_bytes());
                bytes.extend(u16::try_from(positions.len()).unwrap().to_le_bytes());
                let mut previous = 0;
                for &pos in positions {
                    bytes.extend((pos - previous).to_le_bytes());
                    previous = pos;
                }
            }
            max_block = max_block.max(bytes.len());
            let start = data.len();
            if !bytes.is_empty() {
                data.extend(compress_to_vec(&bytes, 9));
            }
            (start, data.len())
        });
        blocks.push(*range);
    }
    let iv = guest.bytecode().fiat_shamir_iv();
    let first = table[0];
    fs::write(out.join("bytecode.bin"), data).unwrap();
    fs::write(out.join("guest.rs"), format!(
        "const IV: [u8;32] = {iv:?};\nconst VARS: usize = {vars};\nconst FIRST: u64 = {first};\nconst MAX_BLOCK: usize = {max_block};\nconst BLOCKS: &[(usize,usize)] = &{blocks:?};"
    )).unwrap();
}
