use super::*;

pub struct LoadIn<'a> {
    pub spv: &'a [u32],
    pub new_spv: &'a mut [u32],
    pub instruction_bound: &'a mut u32,
    pub instruction_inserts: &'a mut Vec<InstructionInsert>,

    pub op_type_sampler_res_id: u32,
    pub op_loads_idxs: &'a [usize],

    pub v_res: &'a [VariableOut],
    pub parameter_res: &'a [FunctionParameterOut],
}

pub fn load(l_in: LoadIn) {
    let LoadIn {
        spv,
        new_spv,
        instruction_bound,
        instruction_inserts,
        op_type_sampler_res_id,
        op_loads_idxs,
        v_res,
        parameter_res,
    } = l_in;

    op_loads_idxs
        .iter()
        .filter_map(|&l_idx| {
            // - Find all OpLoads that ref our v_res_ids
            v_res.iter().find_map(
                |&VariableOut {
                     v_res_id,
                     new_sampler_v_res_id,
                     underlying_image_id,
                 }| {
                    (v_res_id == spv[l_idx + 3]).then_some((
                        l_idx,
                        new_sampler_v_res_id,
                        underlying_image_id,
                    ))
                },
            )
        })
        .chain(op_loads_idxs.iter().filter_map(|&l_idx| {
            // - Find all OpLoads that ref our parameter_res_ids
            parameter_res.iter().find_map(
                |FunctionParameterOut {
                     image_parameter_res_id,
                     sampler_parameter_res_id,
                     underlying_image_id,
                 }| {
                    (*image_parameter_res_id == spv[l_idx + 3]).then_some((
                        l_idx,
                        *sampler_parameter_res_id,
                        *underlying_image_id,
                    ))
                },
            )
        }))
        .for_each(|(l_idx, sampler_v_res_id, underlying_image_id)| {
            // - Insert OpLoads and OpSampledImage to replace combimgsamp
            let image_op_load_res_id = *instruction_bound;
            *instruction_bound += 1;

            let image_original_res_id = spv[l_idx + 2];
            let original_combined_res_id = new_spv[l_idx + 1];

            new_spv[l_idx + 1] = underlying_image_id;
            new_spv[l_idx + 2] = image_op_load_res_id;

            let sampler_op_load_res_id = *instruction_bound;
            *instruction_bound += 1;
            instruction_inserts.push(InstructionInsert {
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
}
