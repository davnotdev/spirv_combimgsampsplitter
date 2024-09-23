use std::{env, fs};

const SPV_HEADER_LENGTH: usize = 5;
const SPV_HEADER_MAGIC_NUM_OFFSET: usize = 0;
const SPV_HEADER_INSTRUCTION_BOUND_OFFSET: usize = 3;

const SPV_INSTRUCTION_OP_TYPE_IMAGE: u16 = 25;
const SPV_INSTRUCTION_OP_TYPE_SAMPLER: u16 = 26;
const SPV_INSTRUCTION_OP_TYPE_SAMPLED_IMAGE: u16 = 27;
const SPV_INSTRUCTION_OP_TYPE_POINTER: u16 = 32;
const SPV_INSTRUCTION_OP_VARIABLE: u16 = 59;
const SPV_INSTRUCTION_OP_LOAD: u16 = 61;
const SPV_INSTRUCTION_OP_SAMPLED_IMAGE: u16 = 86;

const SPV_STORAGE_CLASS_UNIFORM_CONSTANT: u32 = 0;

#[derive(Debug, Clone)]
struct InstructionInsert {
    previous_spv_idx: usize,
    instruction: Vec<u32>,
}

fn main() {
    let spv_file = env::args().nth(1).unwrap();
    let out_spv_file = env::args().nth(2).unwrap();
    let spv = fs::read(spv_file)
        .unwrap()
        .chunks_exact(4)
        .map(|chunk| {
            (chunk[0] as u32)
                | ((chunk[1] as u32) << 8)
                | ((chunk[2] as u32) << 16)
                | ((chunk[3] as u32) << 24)
        })
        .collect::<Vec<_>>();

    let mut instruction_bound = spv[SPV_HEADER_INSTRUCTION_BOUND_OFFSET];
    let magic_number = spv[SPV_HEADER_MAGIC_NUM_OFFSET];

    let mut spv_header = spv[0..SPV_HEADER_LENGTH].to_owned();

    assert_eq!(spv_header[0], magic_number);

    let mut inserts = vec![];

    let spv = spv.into_iter().skip(SPV_HEADER_LENGTH).collect::<Vec<_>>();
    let mut new_spv = spv.clone();

    let mut op_type_sampler_idx = None;
    let mut first_op_type_image_idx = None;

    let mut op_type_sampled_image_idxs = vec![];
    let mut op_type_pointer_idxs = vec![];
    let mut op_variables_idxs = vec![];
    let mut op_loads_idxs = vec![];

    // 1. Find locations instructions we need
    let mut spv_idx = 0;
    while spv_idx < spv.len() {
        let op = spv[spv_idx];
        let byte_count = hiword(op);
        let instruction = loword(op);

        match instruction {
            SPV_INSTRUCTION_OP_TYPE_SAMPLER => {
                op_type_sampler_idx = Some(spv_idx);
            }
            SPV_INSTRUCTION_OP_TYPE_IMAGE => {
                first_op_type_image_idx.get_or_insert(spv_idx);
            }
            SPV_INSTRUCTION_OP_TYPE_SAMPLED_IMAGE => op_type_sampled_image_idxs.push(spv_idx),
            SPV_INSTRUCTION_OP_TYPE_POINTER => {
                if spv[spv_idx + 2] == SPV_STORAGE_CLASS_UNIFORM_CONSTANT {
                    op_type_pointer_idxs.push(spv_idx);
                }
            }
            SPV_INSTRUCTION_OP_VARIABLE => op_variables_idxs.push(spv_idx),
            SPV_INSTRUCTION_OP_LOAD => op_loads_idxs.push(spv_idx),
            _ => {}
        }

        spv_idx += byte_count as usize;
    }

    // 2. Insert OpTypeSampler and respective OpTypePointer if neccessary
    let op_type_image_idx = first_op_type_image_idx.unwrap();
    let (op_type_sampler_res_id, op_type_pointer_sampler_res_id) =
        if let Some(idx) = op_type_sampler_idx {
            let mut ret = None;
            let op_type_sampler_res_id = spv[idx + 1];

            let mut spv_idx = 0;
            while spv_idx < spv.len() {
                let op = spv[spv_idx];
                let byte_count = hiword(op);
                let instruction = loword(op);

                if instruction == SPV_INSTRUCTION_OP_TYPE_POINTER
                    && spv[spv_idx + 2] == SPV_STORAGE_CLASS_UNIFORM_CONSTANT
                    && spv[spv_idx + 3] == op_type_sampler_res_id
                {
                    ret = Some((op_type_sampler_res_id, spv[spv_idx + 1]));
                    break;
                }

                spv_idx += byte_count as usize;
            }
            let op_type_pointer_sampler_res_id = instruction_bound;
            instruction_bound += 1;
            inserts.push(InstructionInsert {
                previous_spv_idx: op_type_image_idx,
                instruction: vec![
                    encode_word(4, SPV_INSTRUCTION_OP_TYPE_POINTER),
                    op_type_pointer_sampler_res_id,
                    SPV_STORAGE_CLASS_UNIFORM_CONSTANT,
                    op_type_sampler_res_id,
                ],
            });
            if let Some(ret) = ret {
                ret
            } else {
                (op_type_sampler_res_id, op_type_pointer_sampler_res_id)
            }
        } else {
            let op_type_sampler_res_id = instruction_bound;
            instruction_bound += 1;
            let op_type_pointer_sampler_res_id = instruction_bound;
            instruction_bound += 1;
            inserts.push(InstructionInsert {
                previous_spv_idx: op_type_image_idx,
                instruction: vec![
                    encode_word(2, SPV_INSTRUCTION_OP_TYPE_SAMPLER),
                    op_type_sampler_res_id,
                    encode_word(4, SPV_INSTRUCTION_OP_TYPE_POINTER),
                    op_type_pointer_sampler_res_id,
                    SPV_STORAGE_CLASS_UNIFORM_CONSTANT,
                    op_type_sampler_res_id,
                ],
            });
            (op_type_sampler_res_id, op_type_pointer_sampler_res_id)
        };

    // 3. OpTypePointer
    let mut tp_res_ids = vec![];

    op_type_pointer_idxs
        .iter()
        .filter_map(|&tp_spv_idx| {
            // - Find OpTypePointers that ref OpTypeSampledImage
            op_type_sampled_image_idxs.iter().find_map(|&ts_spv_idx| {
                (spv[tp_spv_idx + 3] == spv[ts_spv_idx + 1])
                    .then_some((tp_spv_idx, spv[ts_spv_idx + 2]))
            })
        })
        .for_each(|(tp_spv_idx, underlying_image_id)| {
            // - Inject OpTypePointer for sampler pair
            let op_type_pointer_res = instruction_bound;
            instruction_bound += 1;
            inserts.push(InstructionInsert {
                previous_spv_idx: tp_spv_idx,
                instruction: vec![
                    encode_word(4, SPV_INSTRUCTION_OP_TYPE_POINTER),
                    op_type_pointer_res,
                    SPV_STORAGE_CLASS_UNIFORM_CONSTANT,
                    underlying_image_id,
                ],
            });

            // - Change combined image sampler type to underlying image type
            new_spv[tp_spv_idx + 3] = underlying_image_id;

            // - Save the OpTypePointer res id for later
            tp_res_ids.push((spv[tp_spv_idx + 1], underlying_image_id));
        });

    // 4. OpVariable
    let mut v_res_ids = vec![];

    op_variables_idxs
        .iter()
        .filter_map(|&v_idx| {
            // - Find all OpVariables that ref our tp_spv_idxs
            tp_res_ids
                .iter()
                .find_map(|&(tp_res_id, underlying_image_id)| {
                    (tp_res_id == spv[v_idx + 1]).then_some((
                        v_idx,
                        spv[v_idx + 2],
                        underlying_image_id,
                    ))
                })
        })
        .for_each(|(v_idx, v_res_id, underlying_image_id)| {
            // - Inject OpVariable for new sampler
            let sampler_op_variable_res_id = instruction_bound;
            instruction_bound += 1;
            inserts.push(InstructionInsert {
                previous_spv_idx: v_idx,
                instruction: vec![
                    encode_word(4, SPV_INSTRUCTION_OP_VARIABLE),
                    op_type_pointer_sampler_res_id,
                    sampler_op_variable_res_id,
                    SPV_STORAGE_CLASS_UNIFORM_CONSTANT,
                ],
            });
            // - Save the OpVariable res id for later
            v_res_ids.push((v_res_id, sampler_op_variable_res_id, underlying_image_id));
        });

    // 5. OpLoad
    op_loads_idxs
        .iter()
        .filter_map(|&l_idx| {
            // - Find all OpLoads that ref our v_res_ids
            v_res_ids
                .iter()
                .find_map(|&(v_res_id, sampler_v_res_id, underlying_image_id)| {
                    eprintln!("{} {}", v_res_id, spv[l_idx + 3]);
                    (v_res_id == spv[l_idx + 3]).then_some((
                        l_idx,
                        sampler_v_res_id,
                        underlying_image_id,
                    ))
                })
        })
        .for_each(|(l_idx, sampler_v_res_id, underlying_image_id)| {
            // - Insert OpLoads and OpSampledImage to replace combimgsamp
            let image_op_load_res_id = instruction_bound;
            instruction_bound += 1;

            let image_original_res_id = spv[l_idx + 2];
            let original_combined_res_id = new_spv[l_idx + 1];

            new_spv[l_idx + 1] = underlying_image_id;
            new_spv[l_idx + 2] = image_op_load_res_id;

            let sampler_op_load_res_id = instruction_bound;
            instruction_bound += 1;
            inserts.push(InstructionInsert {
                previous_spv_idx: l_idx,
                instruction: vec![
                    encode_word(4, SPV_INSTRUCTION_OP_LOAD),
                    op_type_sampler_res_id,
                    sampler_op_load_res_id,
                    sampler_v_res_id,
                    encode_word(5, SPV_INSTRUCTION_OP_SAMPLED_IMAGE),
                    original_combined_res_id,
                    image_original_res_id,
                    image_op_load_res_id,
                    sampler_op_load_res_id,
                ],
            });
        });

    // 6. Insert New Instructions
    inserts.sort_by_key(|instruction| instruction.previous_spv_idx);
    inserts.iter().rev().for_each(|new_instruction| {
        let offset = hiword(spv[new_instruction.previous_spv_idx]);
        for idx in 0..new_instruction.instruction.len() {
            new_spv.insert(
                new_instruction.previous_spv_idx + offset as usize + idx,
                new_instruction.instruction[idx],
            )
        }
    });

    spv_header[SPV_HEADER_INSTRUCTION_BOUND_OFFSET] = instruction_bound;
    let mut out_spv = spv_header;
    out_spv.append(&mut new_spv);

    fs::write(
        out_spv_file,
        out_spv
            .iter()
            .flat_map(|&n| n.to_le_bytes())
            .collect::<Vec<_>>(),
    )
    .unwrap();
}

fn hiword(value: u32) -> u16 {
    ((value >> 16) & 0xFFFF) as u16
}

fn loword(value: u32) -> u16 {
    (value & 0xFFFF) as u16
}

fn encode_word(hiword: u16, loword: u16) -> u32 {
    ((hiword as u32) << 16) | (loword as u32)
}
