use super::*;

pub struct TypeFunctionIn<'a> {
    pub spv: &'a [u32],
    pub word_inserts: &'a mut Vec<WordInsert>,

    pub op_type_pointer_sampler_res_id: u32,
    pub op_type_function_idxs: &'a [usize],

    pub tp_res: &'a [TypePointerOut],
}

pub fn type_function(tf_in: TypeFunctionIn) {
    let TypeFunctionIn {
        spv,
        word_inserts,
        op_type_pointer_sampler_res_id,
        op_type_function_idxs,
        tp_res,
    } = tf_in;

    op_type_function_idxs.iter().for_each(|&tf_idx| {
        // - Append a sampler OpTypePointer to OpTypeFunction instruction when an combimg OpTypePointer is found.
        tp_res.iter().for_each(|&TypePointerOut { tp_res_id, .. }| {
            let word_count = hiword(spv[tf_idx]);
            for (i, ty) in spv[tf_idx + 3..tf_idx + word_count as usize]
                .iter()
                .enumerate()
            {
                if *ty == tp_res_id {
                    word_inserts.push(WordInsert {
                        idx: tf_idx + 3 + i,
                        word: op_type_pointer_sampler_res_id,
                        head_idx: tf_idx,
                    })
                }
            }
        })
    });
}
