use super::*;

pub struct ImageOp<'a> {
    pub spv: &'a [u32],
    pub new_spv: &'a mut [u32],
    pub instruction_bound: &'a mut u32,
    pub instruction_inserts: &'a mut Vec<InstructionInsert>,

    pub op_type_int_id: u32,
    pub op_type_float_id: u32,
    pub op_constant_2_id: u32,
    pub op_type_sampler_res_id: u32,
    pub op_type_pointer_sampler_res_id: u32,
    pub op_loads_idxs: &'a [usize],
    pub op_image_op_idxs: &'a [usize],

    pub v_res: &'a [VariableOut],
    pub parameter_res: &'a [FunctionParameterOut],
}

pub fn image_op(io_in: ImageOp) {
    let ImageOp {
        spv,
        new_spv,
        instruction_bound,
        instruction_inserts,
        op_type_int_id,
        op_type_float_id,
        op_constant_2_id,
        op_type_sampler_res_id,
        op_type_pointer_sampler_res_id,
        op_loads_idxs,
        op_image_op_idxs,
        v_res,
        parameter_res,
    } = io_in;

    op_loads_idxs
        .iter()
        .filter_map(|&l_idx| {
            // - Find all OpLoads that ref our v_res_ids
            v_res.iter().find_map(
                |&VariableOut {
                     v_res_id,
                     new_sampler_v_res_id,
                     underlying_image_id,
                     is_array,
                 }| {
                    (is_array && v_res_id == spv[l_idx + 3]).then_some((
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
                     is_array,
                 }| {
                    (*is_array && *image_parameter_res_id == spv[l_idx + 3]).then_some((
                        l_idx,
                        *sampler_parameter_res_id,
                        *underlying_image_id,
                    ))
                },
            )
        }))
        .filter_map(
            |(
                l_idx,
                new_sampler_v_res_id,
                underlying_image_id,
            )| {
                // - Find OpImage...s that ref our OpLoad res ids
                op_image_op_idxs.iter().find_map(|&io_idx| {
                    (spv[io_idx + 3] == spv[l_idx + 2]).then_some((
                        io_idx,
                        l_idx,
                        new_sampler_v_res_id,
                        underlying_image_id,
                    ))
                })
            },
        )
        .for_each(
            |(
                io_idx,
                l_idx,
                new_sampler_v_res_id,
                underlying_image_id,
            )| {
                // - Insert the following pseudocode
                //
                // ```
                // let z = tex_coord.z;
                // let sam = samplers[z];
                // let comb = sampledImage(texture_array, sam);
                // ```
                // ac => access chain
                // c => converted
                // l => load
                //

                let l_tex = *instruction_bound;
                *instruction_bound += 1;
                let l_z = *instruction_bound;
                *instruction_bound += 1;
                let c_z = *instruction_bound;
                *instruction_bound += 1;
                let ac_sam = *instruction_bound;
                *instruction_bound += 1;
                let l_sam = *instruction_bound;
                *instruction_bound += 1;
                let comb = *instruction_bound;
                *instruction_bound += 1;

                let io_word_count = hiword(spv[io_idx]);
                let l_word_count = hiword(spv[l_idx]);

                // - Remove original OpImage... and OpLoad of original comb
                new_spv[l_idx] = encode_word(l_word_count, SPV_INSTRUCTION_OP_NOP);
                new_spv[io_idx] = encode_word(io_word_count, SPV_INSTRUCTION_OP_NOP);

                // - Extract info from OpLoad of original comb
                let tex_id = spv[l_idx + 3];
                let comb_type_pointer_id = spv[l_idx + 1];

                let tex_coord_id = spv[io_idx + 4];

                // - New Instructions
                let mut instruction = vec![
                    // %l_tex = OpLoad %. %tex_id
                    encode_word(4, SPV_INSTRUCTION_OP_LOAD),
                    underlying_image_id,
                    l_tex,
                    tex_id,
                    // ---
                    // %l_z = OpVectorExtractDynamic %. %tex_coord %2
                    encode_word(5, SPV_INSTRUCTION_OP_VECTOR_EXTRACT_DYNAMIC),
                    op_type_float_id,
                    l_z,
                    tex_coord_id,
                    op_constant_2_id,
                    // %c_z = OpConvertFToI %. %l_z
                    encode_word(4, SPV_INSTRUCTION_OP_CONVERT_F_TO_I),
                    op_type_int_id,
                    c_z,
                    l_z,
                    // %ac_sam = OpAccessChain %. %tex_coord %c_z
                    encode_word(5, SPV_INSTRUCTION_OP_ACCESS_CHAIN),
                    op_type_pointer_sampler_res_id,
                    ac_sam,
                    new_sampler_v_res_id,
                    c_z,
                    // %l_sam = OpLoad %. %ac_sam
                    encode_word(4, SPV_INSTRUCTION_OP_LOAD),
                    op_type_sampler_res_id,
                    l_sam,
                    ac_sam,
                    // %comb = OpSampledImage %. %l_tex %l_sam
                    encode_word(5, SPV_INSTRUCTION_OP_SAMPLED_IMAGE),
                    comb_type_pointer_id,
                    comb,
                    l_tex,
                    l_sam,
                ];

                // - Patch the original OpImage... instruction too
                for i in 0..io_word_count {
                    instruction.push(spv[io_idx + i as usize]);
                }
                let instruction_len = instruction.len();
                instruction[instruction_len - io_word_count as usize + 3] = comb;

                instruction_inserts.push(InstructionInsert {
                    previous_spv_idx: io_idx,
                    instruction,
                })
            },
        );
}
