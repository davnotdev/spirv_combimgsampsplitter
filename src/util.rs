use super::*;

mod correct_decorate;
mod decorate;

pub use correct_decorate::*;
pub use decorate::*;

pub fn hiword(value: u32) -> u16 {
    ((value >> 16) & 0xFFFF) as u16
}

pub fn loword(value: u32) -> u16 {
    (value & 0xFFFF) as u16
}

pub const fn encode_word(hiword: u16, loword: u16) -> u32 {
    ((hiword as u32) << 16) | (loword as u32)
}

pub fn insert_new_instructions(
    spv: &[u32],
    new_spv: &mut Vec<u32>,
    word_inserts: &[WordInsert],
    instruction_inserts: &[InstructionInsert],
) {
    // 10. Insert New Instructions
    enum Insert {
        Word(WordInsert),
        Instruction(InstructionInsert),
    }
    let mut inserts = word_inserts
        .iter()
        .cloned()
        .map(Insert::Word)
        .chain(instruction_inserts.iter().cloned().map(Insert::Instruction))
        .collect::<Vec<_>>();

    inserts.sort_by_key(|instruction| match instruction {
        Insert::Word(insert) => insert.idx,
        Insert::Instruction(insert) => insert.previous_spv_idx,
    });
    inserts.iter().rev().for_each(|insert| match insert {
        Insert::Word(new_word) => {
            new_spv.insert(new_word.idx + 1, new_word.word);
            new_spv[new_word.head_idx] = encode_word(
                hiword(new_spv[new_word.head_idx]) + 1,
                loword(new_spv[new_word.head_idx]),
            );
        }
        Insert::Instruction(new_instruction) => {
            let offset = hiword(spv[new_instruction.previous_spv_idx]);
            for idx in 0..new_instruction.instruction.len() {
                new_spv.insert(
                    new_instruction.previous_spv_idx + offset as usize + idx,
                    new_instruction.instruction[idx],
                )
            }
        }
    });
}

pub fn prune_noops(new_spv: &mut Vec<u32>) {
    let mut i_idx = 0;
    while i_idx < new_spv.len() {
        let op = new_spv[i_idx];
        let word_count = hiword(op);
        let instruction = loword(op);

        if instruction == SPV_INSTRUCTION_OP_NOP {
            for _ in 0..word_count {
                new_spv.remove(i_idx);
            }
        } else {
            i_idx += word_count as usize;
        }
    }
}

pub fn fuse_final(
    mut spv_header: Vec<u32>,
    mut new_spv: Vec<u32>,
    new_instruction_bound: u32,
) -> Vec<u32> {
    spv_header[SPV_HEADER_INSTRUCTION_BOUND_OFFSET] = new_instruction_bound;
    let mut out_spv = spv_header;
    out_spv.append(&mut new_spv);
    out_spv
}
