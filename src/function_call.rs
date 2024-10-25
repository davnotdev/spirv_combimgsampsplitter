use super::*;

pub struct FunctionCallIn<'a> {
    pub spv: &'a [u32],
    pub word_inserts: &'a mut Vec<WordInsert>,

    pub op_function_call_idxs: &'a [usize],

    pub v_res: &'a [VariableOut],
    pub parameter_res: &'a [FunctionParameterOut],
}

pub fn function_call(fc_in: FunctionCallIn) {
    let FunctionCallIn {
        spv,
        word_inserts,
        op_function_call_idxs,
        v_res,
        parameter_res,
    } = fc_in;

    op_function_call_idxs.iter().for_each(|&fc_idx| {
        parameter_res
            // - Handle use of nested function calls
            .iter()
            .map(
                |FunctionParameterOut {
                     image_parameter_res_id,
                     sampler_parameter_res_id,
                     ..
                 }| { (image_parameter_res_id, sampler_parameter_res_id) },
            )
            // - Handle use of uniform variables
            .chain(v_res.iter().map(
                |VariableOut {
                     v_res_id: image_id,
                     new_sampler_v_res_id: sampler_id,
                     ..
                 }| (image_id, sampler_id),
            ))
            .for_each(|(&image_id, &sampler_id)| {
                let word_count = hiword(spv[fc_idx]);
                for (i, param) in spv[fc_idx + 4..fc_idx + word_count as usize]
                    .iter()
                    .enumerate()
                {
                    if *param == image_id {
                        word_inserts.push(WordInsert {
                            idx: fc_idx + 4 + i,
                            word: sampler_id,
                            head_idx: fc_idx,
                        })
                    }
                }
            });
    });
}
