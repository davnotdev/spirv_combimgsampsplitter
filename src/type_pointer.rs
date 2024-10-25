use super::*;

pub struct TypePointerIn<'a> {
    pub spv: &'a [u32],
    pub new_spv: &'a mut [u32],
    pub instruction_bound: &'a mut u32,
    pub instruction_inserts: &'a mut Vec<InstructionInsert>,

    pub op_type_pointer_idxs: &'a [usize],
    pub op_type_sampled_image_idxs: &'a [usize],
}

pub struct TypePointerOut {
    pub tp_res_id: u32,
    pub underlying_image_id: u32,
}

pub fn type_pointer(tp_in: TypePointerIn) -> Vec<TypePointerOut> {
    let mut tp_res = vec![];

    let TypePointerIn {
        spv,
        new_spv,
        instruction_bound,
        instruction_inserts,
        op_type_pointer_idxs,
        op_type_sampled_image_idxs,
    } = tp_in;

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
            let op_type_pointer_res = *instruction_bound;
            *instruction_bound += 1;
            instruction_inserts.push(InstructionInsert {
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
            tp_res.push(TypePointerOut {
                tp_res_id: spv[tp_spv_idx + 1],
                underlying_image_id,
            });
        });

    tp_res
}
