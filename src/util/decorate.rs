use super::*;

pub struct DecorationVariable {
    pub original_res_id: u32,
    pub new_res_id: u32,
    pub correction_type: CorrectionType,
}

pub struct DecorateIn<'a> {
    pub spv: &'a [u32],
    pub instruction_inserts: &'a mut Vec<InstructionInsert>,

    pub first_op_deocrate_idx: Option<usize>,
    pub op_decorate_idxs: &'a [usize],

    pub affected_variables: &'a [DecorationVariable],
    pub corrections: &'a mut Option<CorrectionMap>,
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
        affected_variables,
        corrections,
    } = d_in;

    let mut new_variable_id_to_decorations = HashMap::new();
    let mut descriptor_sets_to_correct = HashSet::new();

    // - If corrections is empty, we will need to build a new one using existing set bindings
    let mut all_descriptor_sets = corrections.is_none().then_some(HashMap::new());

    // - Find the current binding and descriptor set pair for each combimgsamp
    op_decorate_idxs.iter().for_each(|&d_idx| {
        affected_variables.iter().for_each(
            |&DecorationVariable {
                 original_res_id,
                 new_res_id,
                 correction_type,
             }| {
                let target_id = spv[d_idx + 1];
                let decoration_id = spv[d_idx + 2];
                let decoration_value = spv[d_idx + 3];

                if decoration_id == SPV_DECORATION_BINDING {
                    if original_res_id == target_id {
                        new_variable_id_to_decorations
                            .entry((new_res_id, correction_type))
                            .or_insert((None, None))
                            .0 = Some((d_idx, spv[d_idx + 3]));
                    }

                    if let Some(all_descriptor_sets) = all_descriptor_sets.as_mut() {
                        all_descriptor_sets
                            .entry(target_id)
                            .or_insert((None, None))
                            .0 = Some(decoration_value);
                    }
                } else if decoration_id == SPV_DECORATION_DESCRIPTOR_SET {
                    if original_res_id == target_id {
                        new_variable_id_to_decorations
                            .entry((new_res_id, correction_type))
                            .or_insert((None, None))
                            .1 = Some((d_idx, decoration_value));
                        descriptor_sets_to_correct.insert(decoration_value);
                    }

                    if let Some(all_descriptor_sets) = all_descriptor_sets.as_mut() {
                        all_descriptor_sets
                            .entry(target_id)
                            .or_insert((None, None))
                            .1 = Some(decoration_value);
                    }
                }
            },
        );
    });

    // - Sort and unwrap set binding pairs.
    let mut new_variable_id_to_decorations = new_variable_id_to_decorations
        .into_iter()
        .collect::<Vec<_>>();
    new_variable_id_to_decorations.sort_by_key(|(_, (maybe_binding, _))| {
        let (_, binding) = maybe_binding.unwrap();
        binding
    });
    let new_variable_id_to_decorations = new_variable_id_to_decorations
        .into_iter()
        .map(|(new_res_id, (maybe_binding, maybe_descriptor_set))| {
            let (binding_idx, binding) = maybe_binding.unwrap();
            let (descriptor_set_idx, descriptor_set) = maybe_descriptor_set.unwrap();

            (
                new_res_id,
                ((binding_idx, binding), (descriptor_set_idx, descriptor_set)),
            )
        })
        .collect::<HashMap<_, _>>();

    // - If we need to, build a new correction map
    if let Some(all_descriptor_sets) = all_descriptor_sets {
        let mut new_corrections = CorrectionMap::default();
        let mut all_descriptor_sets = all_descriptor_sets.into_iter().collect::<Vec<_>>();
        all_descriptor_sets.sort_by_key(|(_, (maybe_binding, _))| maybe_binding.unwrap());

        let mut existing_sets: HashSet<u32> = HashSet::new();
        for (_, (binding, set)) in all_descriptor_sets {
            let set = set.unwrap();
            let binding = binding.unwrap();

            if !existing_sets.contains(&set) {
                new_corrections.sets.push(CorrectionSet {
                    set,
                    bindings: vec![],
                });
                existing_sets.insert(set);
            }

            new_corrections.sets[set as usize]
                .bindings
                .push(CorrectionBinding {
                    binding,
                    corrections: vec![],
                });
        }

        *corrections = Some(new_corrections);
    }

    let old_corrections = corrections.clone();

    // - Insert new descriptor set and binding for new ~~sampler~~ variable
    new_variable_id_to_decorations.iter().for_each(
        |(
            (new_res_id, correction_type),
            ((_binding_idx, binding), (_descriptor_set_idx, descriptor_set)),
        )| {
            // - Create the decorations for the new variable
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
                    *new_res_id,
                    SPV_DECORATION_DESCRIPTOR_SET,
                    *descriptor_set,
                    encode_word(4, SPV_INSTRUCTION_OP_DECORATE),
                    *new_res_id,
                    SPV_DECORATION_BINDING,
                    binding + 1,
                ],
            });

            // - Stamp our correction map with new variables
            if let Some(bindings) = corrections
                .as_mut()
                .unwrap()
                .sets
                .get_mut(*descriptor_set as usize)
            {
                // NOTE: We do expect this to be sorted by binding
                // by the current init logic, this will be sorted
                let input_bindings = old_corrections
                    .as_ref()
                    .unwrap()
                    .sets
                    .get(*descriptor_set as usize)
                    .unwrap()
                    .bindings
                    .iter()
                    .map(|correction| correction.corrections.len() + 1)
                    .collect::<Vec<_>>();

                let mut my_binding = *binding as isize;
                for (idx, &binding_count) in input_bindings.iter().enumerate() {
                    if my_binding <= 0 {
                        // The leftover `my_binding` corresponds with the case of having to insert
                        // between or after previously inserted variables
                        bindings.bindings[idx]
                            .corrections
                            .insert(my_binding.unsigned_abs(), *correction_type);

                        break;
                    }
                    my_binding -= binding_count as isize;
                }
            }
        },
    );

    DecorateOut {
        descriptor_sets_to_correct,
    }
}
