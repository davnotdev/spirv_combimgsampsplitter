use super::*;

pub struct VariableIn<'a> {
    pub spv: &'a [u32],
    pub instruction_bound: &'a mut u32,
    pub instruction_inserts: &'a mut Vec<InstructionInsert>,

    pub op_type_pointer_sampler_res_id: u32,
    pub op_type_pointer_arrayed_sampler_res_id: Option<u32>,
    pub op_variables_idxs: &'a [usize],

    pub tp_res: &'a [TypePointerOut],
}

pub struct VariableOut {
    pub v_res_id: u32,
    pub new_sampler_v_res_id: u32,
    pub underlying_image_id: u32,
    pub is_array: bool,
}

pub fn variable(v_in: VariableIn) -> Vec<VariableOut> {
    let mut v_res = vec![];

    let VariableIn {
        spv,
        instruction_bound,
        instruction_inserts,
        op_type_pointer_sampler_res_id,
        op_type_pointer_arrayed_sampler_res_id,
        op_variables_idxs,
        tp_res,
    } = v_in;

    op_variables_idxs
        .iter()
        .filter_map(|&v_idx| {
            // - Find all OpVariables that ref our tp_spv_idxs
            tp_res.iter().find_map(
                |&TypePointerOut {
                     tp_res_id,
                     underlying_image_id,
                     is_array,
                 }| {
                    (tp_res_id == spv[v_idx + 1]).then_some((
                        v_idx,
                        spv[v_idx + 2],
                        underlying_image_id,
                        is_array,
                    ))
                },
            )
        })
        .for_each(
            |(v_idx, v_res_id, underlying_image_id, is_array)| {
                // - Inject OpVariable for new sampler
                let new_sampler_v_res_id = *instruction_bound;
                *instruction_bound += 1;
                instruction_inserts.push(InstructionInsert {
                    previous_spv_idx: v_idx,
                    instruction: vec![
                        encode_word(4, SPV_INSTRUCTION_OP_VARIABLE),
                        if is_array {
                            op_type_pointer_arrayed_sampler_res_id.unwrap()
                        } else {
                            op_type_pointer_sampler_res_id
                        },
                        new_sampler_v_res_id,
                        SPV_STORAGE_CLASS_UNIFORM_CONSTANT,
                    ],
                });
                // - Save the OpVariable res id for later
                v_res.push(VariableOut {
                    v_res_id,
                    new_sampler_v_res_id,
                    underlying_image_id,
                    is_array,
                });
            },
        );

    v_res
}
