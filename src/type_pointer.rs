use super::*;

pub struct TypePointerIn<'a> {
    pub spv: &'a [u32],
    pub new_spv: &'a mut [u32],
    pub instruction_bound: &'a mut u32,
    pub instruction_inserts: &'a mut Vec<InstructionInsert>,

    pub op_type_image_idxs: &'a [usize],
    pub op_type_pointer_idxs: &'a [usize],
    pub op_type_sampled_image_idxs: &'a [usize],
}

pub struct TypePointerOut {
    pub tp_res_id: u32,
    pub underlying_image_id: u32,
    pub is_array: bool,
    pub type_pointer_underlying_image_id: Option<u32>,
}

pub fn type_pointer(tp_in: TypePointerIn) -> Vec<TypePointerOut> {
    let mut tp_res = vec![];

    let TypePointerIn {
        spv,
        new_spv,
        instruction_bound,
        instruction_inserts,
        op_type_image_idxs,
        op_type_pointer_idxs,
        op_type_sampled_image_idxs,
    } = tp_in;

    op_type_pointer_idxs
        .iter()
        .filter_map(|&tp_spv_idx| {
            // - Find OpTypePointers that ref OpTypeSampledImage
            let (tp_spv_idx, underlying_image_id) =
                op_type_sampled_image_idxs.iter().find_map(|&ts_spv_idx| {
                    (spv[tp_spv_idx + 3] == spv[ts_spv_idx + 1])
                        .then_some((tp_spv_idx, spv[ts_spv_idx + 2]))
                })?;

            // - Find our underlying_image_id to check if it is arrayed
            let array_underlying_image_idx = op_type_image_idxs.iter().find_map(|&ti_spv_idx| {
                (underlying_image_id == spv[ti_spv_idx + 1] && spv[ti_spv_idx + 5] != 0)
                    .then_some(ti_spv_idx)
            });

            Some((tp_spv_idx, underlying_image_id, array_underlying_image_idx))
        })
        .for_each(
            |(tp_spv_idx, underlying_image_id, array_underlying_image_idx)| {
                let mut tex_pointer_target = underlying_image_id;

                // - We may need this later if we have an arrayed image.
                let type_pointer_underlying_image_id =
                    if let Some(ti_spv_idx) = array_underlying_image_idx {
                        let op_type_runtime_array_res = *instruction_bound;
                        *instruction_bound += 1;
                        let op_type_pointer_underlying_image_id = *instruction_bound;
                        *instruction_bound += 1;
                        instruction_inserts.push(InstructionInsert {
                            previous_spv_idx: tp_spv_idx - hiword(spv[tp_spv_idx]) as usize + 1,
                            instruction: vec![
                                encode_word(3, SPV_INSTRUCTION_OP_TYPE_RUNTIME_ARRAY),
                                op_type_runtime_array_res,
                                underlying_image_id,
                                encode_word(4, SPV_INSTRUCTION_OP_TYPE_POINTER),
                                op_type_pointer_underlying_image_id,
                                SPV_STORAGE_CLASS_UNIFORM_CONSTANT,
                                underlying_image_id,
                            ],
                        });
                        tex_pointer_target = op_type_runtime_array_res;

                        // - Change type from texture array to texture because we will wrap this in an
                        // array anyway
                        new_spv[ti_spv_idx + 5] = 0;

                        Some(op_type_pointer_underlying_image_id)
                    } else {
                        None
                    };

                // - Change combined image sampler type to underlying image type
                new_spv[tp_spv_idx + 3] = tex_pointer_target;

                // - Save the OpTypePointer res id for later
                tp_res.push(TypePointerOut {
                    tp_res_id: spv[tp_spv_idx + 1],
                    underlying_image_id,
                    is_array: array_underlying_image_idx.is_some(),
                    type_pointer_underlying_image_id,
                });
            },
        );

    tp_res
}
