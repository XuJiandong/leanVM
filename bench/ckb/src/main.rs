#![no_std]
#![no_main]

ckb_std::entry!(main);
const FIXED_HEAP: usize = 64 * 1024;
const HEAP: usize = 2 * 1024 * 1024;
ckb_std::default_alloc!(FIXED_HEAP, HEAP, 64);

use alloc::vec::Vec;
use ckb_std::{
    ckb_constants::Source,
    high_level::{load_cell_data, load_witness},
};
use verifier::{Bytecode, E, K, Proof};

include!(concat!(env!("OUT_DIR"), "/guest.rs"));

/// Each group holds a word, its occurrence count and the deltas between positions within its block.
fn evaluate(low: &[E], high: &[E]) -> E {
    let data = include_bytes!(concat!(env!("OUT_DIR"), "/bytecode.bin"));
    let mut scratch = alloc::vec![0; MAX_BLOCK];
    let mut result = E::default();
    for (&(start, end), &weight) in BLOCKS.iter().zip(high) {
        if start == end {
            continue;
        }
        let len = miniz_oxide::inflate::decompress_slice_iter_to_slice(
            &mut scratch,
            core::iter::once(&data[start..end]),
            false,
            true,
        )
        .expect("embedded bytecode block");
        let mut groups = &scratch[..len];
        let mut block = E::default();
        while !groups.is_empty() {
            let word = u64::from_le_bytes(groups[..8].try_into().unwrap());
            let count = u16::from_le_bytes(groups[8..10].try_into().unwrap()) as usize;
            let (deltas, tail) = groups[10..].split_at(count * 2);
            groups = tail;
            let mut i = 0;
            let mut sum = E::default();
            for delta in deltas.as_chunks().0 {
                i += u16::from_le_bytes(*delta) as usize;
                sum += low[i];
            }
            block += sum * K(word);
        }
        result += weight * block;
    }
    result
}

/// Both public key types order their fields in wire order, followed by the message.
fn take_list(bytes: &mut &[u8]) -> Option<[u8; 32]> {
    let (count, tail) = bytes.split_first_chunk()?;
    let len = usize::try_from(u64::from_le_bytes(*count)).ok()?.checked_mul(64)?;
    let (list, tail) = tail.split_at_checked(len)?;
    *bytes = tail;
    list.as_chunks::<64>()
        .0
        .is_sorted_by(|a, b| a < b)
        .then(|| keccak256::hash(list))
}

fn decode(mut claims: &[u8], aggregate: &[u8], guest: &Bytecode) -> Option<([u8; 32], Proof)> {
    let mut statement = Vec::from(IV);
    statement.extend(take_list(&mut claims)?);
    statement.extend(take_list(&mut claims)?);
    let root = match claims {
        [0] => None,
        [1, rest @ ..] => Some(*rest.as_array()?),
        _ => return None,
    };
    statement.extend(verifier::da::digest(root));
    let (count, rest) = aggregate.split_first_chunk()?;
    if u64::from_le_bytes(*count) != VARS as u64 {
        return None;
    }
    let (bytes, proof) = rest.split_at_checked(VARS * 16)?;
    let point: Vec<E> = bytes
        .as_chunks::<8>()
        .0
        .as_chunks::<2>()
        .0
        .iter()
        .map(|[lo, hi]| E::new(u64::from_le_bytes(*lo), u64::from_le_bytes(*hi)))
        .collect();
    let value = if point.iter().all(|&v| v == E::default()) {
        E::from(K(FIRST))
    } else {
        guest.evaluate(&point).ok()?
    };
    statement.extend_from_slice(bytes);
    statement.extend(value.c0.to_le_bytes());
    statement.extend(value.c1.to_le_bytes());
    Some((keccak256::hash(&statement), Proof::from_bytes(proof)?))
}

fn main() -> i8 {
    if ckb_std::syscalls::load_cell_data(&mut [], 0, 1, Source::GroupInput)
        != Err(ckb_std::error::SysError::IndexOutOfBound)
    {
        return 2;
    }
    let Ok(claims) = load_cell_data(0, Source::GroupInput) else {
        return 2;
    };
    let Ok(proof) = load_witness(0, Source::GroupInput) else {
        return 2;
    };
    let guest = Bytecode::__from_embedded(1 << VARS, IV, evaluate);
    let Some((statement, decoded)) = decode(&claims, &proof, &guest) else {
        return 1;
    };
    drop(claims);
    drop(proof);
    if verifier::verify_execution(&guest, &statement, &decoded).is_ok() {
        0
    } else {
        1
    }
}
