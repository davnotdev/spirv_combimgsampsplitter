use std::collections::{HashMap, HashSet};

mod decorate;
mod function_call;
mod function_parameter;
mod load;
mod type_function;
mod type_pointer;
mod variable;

#[cfg(test)]
mod test;

use decorate::*;
use function_call::*;
use function_parameter::*;
use load::*;
use type_function::*;
use type_pointer::*;
use variable::*;

const SPV_HEADER_LENGTH: usize = 5;
const SPV_HEADER_MAGIC: u32 = 0x07230203;
const SPV_HEADER_MAGIC_NUM_OFFSET: usize = 0;
const SPV_HEADER_INSTRUCTION_BOUND_OFFSET: usize = 3;

const SPV_INSTRUCTION_OP_NOP: u16 = 1;
const SPV_INSTRUCTION_OP_TYPE_VOID: u16 = 19;
const SPV_INSTRUCTION_OP_TYPE_IMAGE: u16 = 25;
const SPV_INSTRUCTION_OP_TYPE_SAMPLER: u16 = 26;
const SPV_INSTRUCTION_OP_TYPE_SAMPLED_IMAGE: u16 = 27;
const SPV_INSTRUCTION_OP_TYPE_POINTER: u16 = 32;
const SPV_INSTRUCTION_OP_TYPE_FUNCTION: u16 = 33;
const SPV_INSTRUCTION_OP_FUNCTION_PARAMTER: u16 = 55;
const SPV_INSTRUCTION_OP_FUNCTION_CALL: u16 = 57;
const SPV_INSTRUCTION_OP_VARIABLE: u16 = 59;
const SPV_INSTRUCTION_OP_LOAD: u16 = 61;
const SPV_INSTRUCTION_OP_DECORATE: u16 = 71;
const SPV_INSTRUCTION_OP_SAMPLED_IMAGE: u16 = 86;

const SPV_STORAGE_CLASS_UNIFORM_CONSTANT: u32 = 0;
const SPV_DECORATION_BINDING: u32 = 33;
const SPV_DECORATION_DESCRIPTOR_SET: u32 = 34;

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

/// Perform the operation on a `Vec<u32>`.
/// Use [u8_slice_to_u32_vec] to convert a `&[u8]` into a `Vec<u32>`
pub fn combimgsampsplitter(in_spv: &[u32]) -> Result<Vec<u32>, ()> {
    let spv = in_spv.to_owned();

    let mut instruction_bound = spv[SPV_HEADER_INSTRUCTION_BOUND_OFFSET];
    let magic_number = spv[SPV_HEADER_MAGIC_NUM_OFFSET];

    let mut spv_header = spv[0..SPV_HEADER_LENGTH].to_owned();

    assert_eq!(magic_number, SPV_HEADER_MAGIC);

    let mut instruction_inserts = vec![];
    let mut word_inserts = vec![];

    let spv = spv.into_iter().skip(SPV_HEADER_LENGTH).collect::<Vec<_>>();
    let mut new_spv = spv.clone();

    let mut op_type_sampler_idx = None;
    let mut first_op_deocrate_idx = None;
    let mut first_op_type_void_idx = None;

    let mut op_type_image_idxs = vec![];
    let mut op_type_sampled_image_idxs = vec![];
    let mut op_type_pointer_idxs = vec![];
    let mut op_variables_idxs = vec![];
    let mut op_loads_idxs = vec![];
    let mut op_decorate_idxs = vec![];
    let mut op_type_function_idxs = vec![];
    let mut op_function_parameter_idxs = vec![];
    let mut op_function_call_idxs = vec![];

    // 1. Find locations instructions we need
    let mut spv_idx = 0;
    while spv_idx < spv.len() {
        let op = spv[spv_idx];
        let word_count = hiword(op);
        let instruction = loword(op);

        match instruction {
            SPV_INSTRUCTION_OP_TYPE_VOID => {
                first_op_type_void_idx = Some(spv_idx);
            }
            SPV_INSTRUCTION_OP_TYPE_SAMPLER => {
                op_type_sampler_idx = Some(spv_idx);
                new_spv[spv_idx] = encode_word(word_count, SPV_INSTRUCTION_OP_NOP);
            }
            SPV_INSTRUCTION_OP_TYPE_IMAGE => {
                op_type_image_idxs.push(spv_idx);
            }
            SPV_INSTRUCTION_OP_TYPE_SAMPLED_IMAGE => op_type_sampled_image_idxs.push(spv_idx),
            SPV_INSTRUCTION_OP_TYPE_POINTER => {
                if spv[spv_idx + 2] == SPV_STORAGE_CLASS_UNIFORM_CONSTANT {
                    op_type_pointer_idxs.push(spv_idx);
                }
            }
            SPV_INSTRUCTION_OP_VARIABLE => op_variables_idxs.push(spv_idx),
            SPV_INSTRUCTION_OP_LOAD => op_loads_idxs.push(spv_idx),
            SPV_INSTRUCTION_OP_DECORATE => {
                op_decorate_idxs.push(spv_idx);
                first_op_deocrate_idx.get_or_insert(spv_idx);
            }
            SPV_INSTRUCTION_OP_TYPE_FUNCTION => op_type_function_idxs.push(spv_idx),
            SPV_INSTRUCTION_OP_FUNCTION_PARAMTER => op_function_parameter_idxs.push(spv_idx),
            SPV_INSTRUCTION_OP_FUNCTION_CALL => op_function_call_idxs.push(spv_idx),

            _ => {}
        }

        spv_idx += word_count as usize;
    }

    // 2. Insert OpTypeSampler and respective OpTypePointer if neccessary

    // - If there has been no OpTypeImage, there will be nothing to do
    if op_type_image_idxs.is_empty() {
        return Ok(in_spv.to_vec());
    };

    let op_type_sampler_res_id = if let Some(idx) = op_type_sampler_idx {
        spv[idx + 1]
    } else {
        let op_type_sampler_res_id = instruction_bound;
        instruction_bound += 1;
        op_type_sampler_res_id
    };

    let op_type_pointer_sampler_res_id = instruction_bound;
    instruction_bound += 1;
    instruction_inserts.push(InstructionInsert {
        // Let's avoid trouble and just insert after OpTypeVoid.
        // previous_spv_idx: op_type_image_idx,
        previous_spv_idx: first_op_type_void_idx.unwrap(),
        instruction: vec![
            encode_word(2, SPV_INSTRUCTION_OP_TYPE_SAMPLER),
            op_type_sampler_res_id,
            encode_word(4, SPV_INSTRUCTION_OP_TYPE_POINTER),
            op_type_pointer_sampler_res_id,
            SPV_STORAGE_CLASS_UNIFORM_CONSTANT,
            op_type_sampler_res_id,
        ],
    });

    // 3. OpTypePointer
    let tp_res = type_pointer(TypePointerIn {
        spv: &spv,
        new_spv: &mut new_spv,

        op_type_pointer_idxs: &op_type_pointer_idxs,
        op_type_sampled_image_idxs: &op_type_sampled_image_idxs,
    });

    // 4. OpVariable
    let v_res = variable(VariableIn {
        spv: &spv,
        instruction_bound: &mut instruction_bound,
        instruction_inserts: &mut instruction_inserts,
        op_type_pointer_sampler_res_id,
        op_variables_idxs: &op_variables_idxs,
        tp_res: &tp_res,
    });

    // 5. OpTypeFunction
    type_function(TypeFunctionIn {
        spv: &spv,
        word_inserts: &mut word_inserts,
        op_type_pointer_sampler_res_id,
        op_type_function_idxs: &op_type_function_idxs,
        tp_res: &tp_res,
    });

    // 6. OpFunctionParameter
    let parameter_res = function_parameter(FunctionParameterIn {
        spv: &spv,
        instruction_bound: &mut instruction_bound,
        instruction_inserts: &mut instruction_inserts,
        op_type_pointer_sampler_res_id,
        op_function_parameter_idxs: &op_function_parameter_idxs,
        tp_res: &tp_res,
    });

    // 7. OpFunctionCall
    function_call(FunctionCallIn {
        spv: &spv,
        word_inserts: &mut word_inserts,
        op_function_call_idxs: &op_function_call_idxs,
        v_res: &v_res,
        parameter_res: &parameter_res,
    });

    // 8. OpLoad
    load(LoadIn {
        spv: &spv,
        new_spv: &mut new_spv,
        instruction_bound: &mut instruction_bound,
        instruction_inserts: &mut instruction_inserts,
        op_type_sampler_res_id,
        op_loads_idxs: &op_loads_idxs,
        v_res: &v_res,
        parameter_res: &parameter_res,
    });

    // 9. OpDecorate
    let DecorateOut {
        descriptor_sets_to_correct,
    } = decorate(DecorateIn {
        spv: &spv,
        instruction_inserts: &mut instruction_inserts,
        first_op_deocrate_idx,
        op_decorate_idxs: &op_decorate_idxs,
        v_res: &v_res,
    });

    // 10. Insert New Instructions
    enum Insert {
        Word(WordInsert),
        Instruction(InstructionInsert),
    }

    let mut inserts = word_inserts
        .into_iter()
        .map(Insert::Word)
        .chain(instruction_inserts.into_iter().map(Insert::Instruction))
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

    // 11. Correct OpDecorate Bindings
    let mut candidates = HashMap::new();

    let mut d_idx = 0;
    while d_idx < new_spv.len() {
        let op = new_spv[d_idx];
        let word_count = hiword(op);
        let instruction = loword(op);
        if instruction == SPV_INSTRUCTION_OP_DECORATE {
            match new_spv[d_idx + 2] {
                SPV_DECORATION_DESCRIPTOR_SET => {
                    candidates
                        .entry(new_spv[d_idx + 1])
                        .or_insert((None, None))
                        .0 = Some(new_spv[d_idx + 3])
                }
                SPV_DECORATION_BINDING => {
                    candidates
                        .entry(new_spv[d_idx + 1])
                        .or_insert((None, None))
                        .1 = Some((d_idx, new_spv[d_idx + 3]))
                }
                _ => {}
            }
        }

        d_idx += word_count as usize;
    }

    for descriptor_set in descriptor_sets_to_correct {
        let mut bindings = candidates
            .iter()
            .filter_map(|(_, &(maybe_descriptor_set, maybe_binding))| {
                let this_descriptor_set = maybe_descriptor_set.unwrap();
                let (binding_idx, this_binding) = maybe_binding.unwrap();
                (this_descriptor_set == descriptor_set).then_some((binding_idx, this_binding))
            })
            .collect::<Vec<_>>();
        bindings.sort_by_cached_key(|&(_, binding)| binding);

        // We can assume that our new samplers will have a greater instruction ID than the original
        // conbined image samplers.
        let mut prev_binding = -1;
        let mut prev_id = -1;
        let mut prev_d_idx = -1;
        let mut increment = 0;
        for (d_idx, binding) in bindings {
            let this_id = new_spv[d_idx + 1];

            if binding as i32 == prev_binding {
                increment += 1;

                if prev_id <= this_id as i32 {
                    new_spv[prev_d_idx as usize + 3] += 1;
                    new_spv[d_idx + 3] -= 1;
                }
            }
            new_spv[d_idx + 3] += increment;
            prev_binding = binding as i32;
            prev_id = this_id as i32;
            prev_d_idx = d_idx as isize;
        }
    }

    // 12. Remove Instructions that have been Whited Out.

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

    // 13. Write New Header and New Code
    spv_header[SPV_HEADER_INSTRUCTION_BOUND_OFFSET] = instruction_bound;
    let mut out_spv = spv_header;
    out_spv.append(&mut new_spv);

    Ok(out_spv)
}

fn hiword(value: u32) -> u16 {
    ((value >> 16) & 0xFFFF) as u16
}

fn loword(value: u32) -> u16 {
    (value & 0xFFFF) as u16
}

const fn encode_word(hiword: u16, loword: u16) -> u32 {
    ((hiword as u32) << 16) | (loword as u32)
}
