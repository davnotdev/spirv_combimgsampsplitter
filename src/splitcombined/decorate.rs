use super::*;

pub struct DecorateIn<'a> {
    pub spv: &'a [u32],
    pub instruction_inserts: &'a mut Vec<InstructionInsert>,

    pub first_op_deocrate_idx: Option<usize>,
    pub op_decorate_idxs: &'a [usize],

    pub v_res: &'a [VariableOut],
}

pub struct DecorateOut {
    pub descriptor_sets_to_correct: HashSet<u32>,
}

pub fn decorate(d_in: DecorateIn) -> DecorateOut {
    let DecorateIn {
        spv,
        instruction_inserts,
        first_op_deocrate_idx,
        op_decorate_idxs,
        v_res,
    } = d_in;

    let mut sampler_id_to_decorations = HashMap::new();
    let mut descriptor_sets_to_correct = HashSet::new();

    // - Find the current binding and descriptor set pair for each combimgsamp
    op_decorate_idxs.iter().for_each(|&d_idx| {
        v_res.iter().for_each(
            |&VariableOut {
                 v_res_id,
                 new_sampler_v_res_id,
                 ..
             }| {
                if v_res_id == spv[d_idx + 1] {
                    if spv[d_idx + 2] == SPV_DECORATION_BINDING {
                        sampler_id_to_decorations
                            .entry(new_sampler_v_res_id)
                            .or_insert((None, None))
                            .0 = Some((d_idx, spv[d_idx + 3]));
                    } else if spv[d_idx + 2] == SPV_DECORATION_DESCRIPTOR_SET {
                        sampler_id_to_decorations
                            .entry(new_sampler_v_res_id)
                            .or_insert((None, None))
                            .1 = Some((d_idx, spv[d_idx + 3]));
                        descriptor_sets_to_correct.insert(spv[d_idx + 3]);
                    }
                }
            },
        );
    });

    let mut sampler_id_to_decorations = sampler_id_to_decorations.into_iter().collect::<Vec<_>>();
    sampler_id_to_decorations.sort_by_key(|(_, (maybe_binding, _))| {
        let (_, binding) = maybe_binding.unwrap();
        binding
    });
    let sampler_id_to_decorations = sampler_id_to_decorations
        .into_iter()
        .map(|(sampler_id, (maybe_binding, maybe_descriptor_set))| {
            let (binding_idx, binding) = maybe_binding.unwrap();
            let (descriptor_set_idx, descriptor_set) = maybe_descriptor_set.unwrap();

            (
                sampler_id,
                ((binding_idx, binding), (descriptor_set_idx, descriptor_set)),
            )
        })
        .collect::<HashMap<_, _>>();

    // - Insert new descriptor set and binding for new sampler
    sampler_id_to_decorations.iter().for_each(
        |(sampler_v_res_id, ((_binding_idx, binding), (_descriptor_set_idx, descriptor_set)))| {
            instruction_inserts.push(InstructionInsert {
                // NOTE: If bindings are not ordered reasonably in spv, the original
                // implementation may fail.
                // Example:
                //      %u_other = (0, 1)
                //      %u_combined = (0, 0)
                //      %inserted_sampler = (0, 0)
                // becomes
                //      %u_other = (0, 1)
                //      %u_combined = (0, 0)
                //      %inserted_sampler = (0, 2)
                // previous_spv_idx: descriptor_set_idx.max(binding_idx),
                previous_spv_idx: first_op_deocrate_idx.unwrap(),
                instruction: vec![
                    encode_word(4, SPV_INSTRUCTION_OP_DECORATE),
                    *sampler_v_res_id,
                    SPV_DECORATION_DESCRIPTOR_SET,
                    *descriptor_set,
                    encode_word(4, SPV_INSTRUCTION_OP_DECORATE),
                    *sampler_v_res_id,
                    SPV_DECORATION_BINDING,
                    binding + 1,
                ],
            })
        },
    );

    DecorateOut {
        descriptor_sets_to_correct,
    }
}
