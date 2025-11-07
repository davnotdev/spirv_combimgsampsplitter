use std::collections::{HashMap, HashSet};

mod splitcombined;
mod spv;
mod util;

use spv::*;
use util::*;

pub use splitcombined::*;

#[derive(Debug, Clone)]
struct InstructionInsert {
    previous_spv_idx: usize,
    instruction: Vec<u32>,
}

#[derive(Debug, Clone)]
struct WordInsert {
    idx: usize,
    word: u32,
    head_idx: usize,
}

/// Helper to convert a `&[u8]` into a `Vec<u32>`.
pub fn u8_slice_to_u32_vec(vec: &[u8]) -> Vec<u32> {
    assert_eq!(
        vec.len() % 4,
        0,
        "Input slice length must be a multiple of 4."
    );

    vec.chunks_exact(4)
        .map(|chunk| {
            (chunk[0] as u32)
                | ((chunk[1] as u32) << 8)
                | ((chunk[2] as u32) << 16)
                | ((chunk[3] as u32) << 24)
        })
        .collect::<Vec<_>>()
}

/// Helper to convert a `&[u32]` into a `Vec<u8>`.
pub fn u32_slice_to_u8_vec(vec: &[u32]) -> Vec<u8> {
    vec.iter()
        .flat_map(|&num| {
            vec![
                (num & 0xFF) as u8,
                ((num >> 8) & 0xFF) as u8,
                ((num >> 16) & 0xFF) as u8,
                ((num >> 24) & 0xFF) as u8,
            ]
        })
        .collect::<Vec<u8>>()
}
