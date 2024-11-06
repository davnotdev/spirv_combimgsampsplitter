pub struct TypePointerIn<'a> {
    pub spv: &'a [u32],
    pub new_spv: &'a mut [u32],

    pub op_type_image_idxs: &'a [usize],
    pub op_type_pointer_idxs: &'a [usize],
    pub op_type_sampled_image_idxs: &'a [usize],
}

pub struct TypePointerOut {
    pub tp_res_id: u32,
    pub underlying_image_id: u32,
    pub is_array: bool,
}

pub fn type_pointer(tp_in: TypePointerIn) -> Vec<TypePointerOut> {
    let mut tp_res = vec![];

    let TypePointerIn {
        spv,
        new_spv,
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
            let is_array = op_type_image_idxs.iter().any(|&ti_spv_idx| {
                underlying_image_id == spv[ti_spv_idx + 1] && spv[ti_spv_idx + 5] != 0
            });

            Some((tp_spv_idx, underlying_image_id, is_array))
        })
        .for_each(|(tp_spv_idx, underlying_image_id, is_array)| {
            // - Change combined image sampler type to underlying image type
            new_spv[tp_spv_idx + 3] = underlying_image_id;

            // - Save the OpTypePointer res id for later
            tp_res.push(TypePointerOut {
                tp_res_id: spv[tp_spv_idx + 1],
                underlying_image_id,
                is_array,
            });
        });

    tp_res
}
