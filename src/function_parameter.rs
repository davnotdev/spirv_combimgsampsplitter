use super::*;

pub struct FunctionParameterIn<'a> {
    pub spv: &'a [u32],
    pub instruction_bound: &'a mut u32,
    pub instruction_inserts: &'a mut Vec<InstructionInsert>,

    pub op_type_pointer_sampler_res_id: u32,
    pub op_function_parameter_idxs: &'a [usize],

    pub tp_res: &'a [TypePointerOut],
}

pub struct FunctionParameterOut {
    pub image_parameter_res_id: u32,
    pub sampler_parameter_res_id: u32,
    pub underlying_image_id: u32,
}

pub fn function_parameter(fp_in: FunctionParameterIn) -> Vec<FunctionParameterOut> {
    let FunctionParameterIn {
        spv,
        instruction_bound,
        instruction_inserts,
        op_type_pointer_sampler_res_id,
        op_function_parameter_idxs,
        tp_res,
    } = fp_in;

    let mut parameter_res_ids = HashMap::new();

    op_function_parameter_idxs
        .iter()
        .filter_map(|&fp_idx| {
            // - Find all OpFunctionParameters that use a combimg OpTypePointer
            tp_res.iter().find_map(
                |&TypePointerOut {
                     tp_res_id,
                     underlying_image_id,
                 }| {
                    (spv[fp_idx + 1] == tp_res_id).then_some((
                        fp_idx,
                        spv[fp_idx + 2],
                        underlying_image_id,
                    ))
                },
            )
        })
        .for_each(|(fp_idx, image_parameter_res_id, underlying_image_id)| {
            // - Append a new sampler OpFunctionParameter
            let sampler_parameter_res_id = *instruction_bound;
            *instruction_bound += 1;
            instruction_inserts.push(InstructionInsert {
                previous_spv_idx: fp_idx,
                instruction: vec![
                    encode_word(3, SPV_INSTRUCTION_OP_FUNCTION_PARAMTER),
                    op_type_pointer_sampler_res_id,
                    sampler_parameter_res_id,
                ],
            });
            parameter_res_ids.insert(
                image_parameter_res_id,
                (sampler_parameter_res_id, underlying_image_id),
            );
        });

    parameter_res_ids
        .into_iter()
        .map(
            |(image_parameter_res_id, (sampler_parameter_res_id, underlying_image_id))| {
                FunctionParameterOut {
                    image_parameter_res_id,
                    sampler_parameter_res_id,
                    underlying_image_id,
                }
            },
        )
        .collect::<Vec<_>>()
}
