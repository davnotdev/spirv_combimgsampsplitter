use super::*;

#[derive(Debug, Clone, Copy)]
enum OperationVariant {
    Regular,
    Dref,
}

/// Perform the operation on a `Vec<u32>`.
/// Use [u8_slice_to_u32_vec] to convert a `&[u8]` into a `Vec<u32>`
pub fn dreftexturesplitter(in_spv: &[u32]) -> Result<Vec<u32>, ()> {
    let spv = in_spv.to_owned();

    let mut instruction_bound = spv[SPV_HEADER_INSTRUCTION_BOUND_OFFSET];
    let magic_number = spv[SPV_HEADER_MAGIC_NUM_OFFSET];

    let spv_header = spv[0..SPV_HEADER_LENGTH].to_owned();

    assert_eq!(magic_number, SPV_HEADER_MAGIC);

    let mut instruction_inserts: Vec<InstructionInsert> = vec![];
    let mut word_inserts: Vec<WordInsert> = vec![];

    let spv = spv.into_iter().skip(SPV_HEADER_LENGTH).collect::<Vec<_>>();
    let mut new_spv = spv.clone();

    // 1. Find locations instructions we need
    let mut first_op_deocrate_idx = None;

    let mut op_dref_operation_idxs = vec![];
    let mut op_sampled_operation_idxs = vec![];
    let mut op_sampled_image_idxs = vec![];
    let mut op_load_idxs = vec![];
    let mut op_variable_idxs = vec![];
    let mut op_decorate_idxs = vec![];

    let mut spv_idx = 0;
    while spv_idx < spv.len() {
        let op = spv[spv_idx];
        let word_count = hiword(op);
        let instruction = loword(op);

        match instruction {
            SPV_INSTRUCTION_OP_SAMPLED_IMAGE => op_sampled_image_idxs.push(spv_idx),
            SPV_INSTRUCTION_OP_LOAD => op_load_idxs.push(spv_idx),
            SPV_INSTRUCTION_OP_IMAGE_SAMPLE_DREF_IMPLICIT_LOD
            | SPV_INSTRUCTION_OP_IMAGE_SAMPLE_DREF_EXPLICIT_LOD
            | SPV_INSTRUCTION_OP_IMAGE_SAMPLE_PROJ_DREF_IMPLICIT_LOD
            | SPV_INSTRUCTION_OP_IMAGE_SAMPLE_PROJ_DREF_EXPLICIT_LOD
            | SPV_INSTRUCTION_OP_IMAGE_DREF_GATHER
            | SPV_INSTRUCTION_OP_IMAGE_SPARSE_SAMPLE_DREF_IMPLICIT_LOD
            | SPV_INSTRUCTION_OP_IMAGE_SPARSE_SAMPLE_DREF_EXPLICIT_LOD
            | SPV_INSTRUCTION_OP_IMAGE_SPARSE_DREF_GATHER => op_dref_operation_idxs.push(spv_idx),
            SPV_INSTRUCTION_OP_IMAGE_SAMPLE_IMPLICIT_LOD
            | SPV_INSTRUCTION_OP_IMAGE_SAMPLE_EXPLICIT_LOD
            | SPV_INSTRUCTION_OP_IMAGE_SAMPLE_PROJ_IMPLICIT_LOD
            | SPV_INSTRUCTION_OP_IMAGE_SAMPLE_PROJ_EXPLICIT_LOD
            | SPV_INSTRUCTION_OP_IMAGE_GATHER
            | SPV_INSTRUCTION_OP_IMAGE_SPARSE_SAMPLE_IMPLICIT_LOD
            | SPV_INSTRUCTION_OP_IMAGE_SPARSE_SAMPLE_EXPLICIT_LOD
            | SPV_INSTRUCTION_OP_IMAGE_SPARSE_GATHER => {
                op_sampled_operation_idxs.push(spv_idx);
            }
            SPV_INSTRUCTION_OP_VARIABLE => op_variable_idxs.push(spv_idx),
            SPV_INSTRUCTION_OP_DECORATE => {
                op_decorate_idxs.push(spv_idx);
                first_op_deocrate_idx.get_or_insert(spv_idx);
            }
            _ => {}
        }

        spv_idx += word_count as usize;
    }

    // 2. Collect all the loaded sampled images of both operation types
    // Conveniently, the offset for this value is always +3 for all of these operations
    let loaded_sampled_image_ids = op_sampled_operation_idxs
        .iter()
        .map(|idx| (spv[idx + 3], OperationVariant::Regular))
        .chain(
            op_dref_operation_idxs
                .iter()
                .map(|idx| (spv[idx + 3], OperationVariant::Dref)),
        )
        .collect::<Vec<_>>();

    // 3. Backtrace to find the OpSampledImage that resulted in our loaded sampled images
    let loaded_image_ids = op_sampled_image_idxs
        .iter()
        .filter_map(|idx| {
            let sampled_result_id = spv[idx + 2];
            let loaded_image_id = spv[idx + 3];
            loaded_sampled_image_ids
                .iter()
                .find_map(|(id, ty)| (*id == sampled_result_id).then_some(ty))
                .map(|ty| (loaded_image_id, ty))
        })
        .collect::<Vec<_>>();

    // 4. Backtrack to find the OpLoad that resulted in our loaded images
    let image_ids = op_load_idxs
        .iter()
        .filter_map(|idx| {
            let loaded_result_id = spv[idx + 2];
            let original_image = spv[idx + 3];
            loaded_image_ids
                .iter()
                .find_map(|(id, ty)| (*id == loaded_result_id).then_some(ty))
                .map(|ty| (original_image, idx, ty))
        })
        .collect::<Vec<_>>();

    // 5. Find the images that mix operations
    let mut mixed_image_ids = HashMap::new();
    let mut image_id_to_loads = HashMap::new();

    for (id, load_idx, ty) in image_ids {
        let entry = mixed_image_ids.entry(id).or_insert((false, false));
        image_id_to_loads.entry(id).or_insert(vec![]).push(load_idx);

        match ty {
            OperationVariant::Regular => entry.0 = true,
            OperationVariant::Dref => entry.1 = true,
        }
    }

    let mixed_image_ids = mixed_image_ids
        .into_iter()
        .filter_map(|(id, (uses_regular, uses_dref))| (uses_regular && uses_dref).then_some(id))
        .collect::<Vec<_>>();

    // 5. Duplicate OpVariable with a new_id and patch old OpLoads
    // NOTE: GENERALLY, with glslc, each OpImage* will get its own OpLoad, so we don't need to
    // check that its result isn't used for both regular and dref operations!
    let patch_variable_idxs = op_variable_idxs
        .iter()
        .filter_map(|idx| {
            let result_id = spv[idx + 2];
            mixed_image_ids
                .iter()
                .find(|id| **id == result_id)
                .map(|id| (idx, id))
        })
        .collect::<Vec<_>>();

    let mut affected_variables = Vec::new();

    for (&variable_idx, &old_variable_id) in patch_variable_idxs {
        let word_count = hiword(spv[variable_idx]);

        // OpVariable
        let new_variable_id = instruction_bound;
        instruction_bound += 1;
        let mut new_variable = Vec::new();
        new_variable.extend_from_slice(&spv[variable_idx..variable_idx + word_count as usize]);
        new_variable[2] = new_variable_id;
        instruction_inserts.push(InstructionInsert {
            previous_spv_idx: variable_idx,
            instruction: new_variable,
        });

        affected_variables.push(util::DecorationVariable {
            original_res_id: spv[variable_idx + 2],
            new_res_id: new_variable_id,
        });

        // OpLoad
        if let Some(op_load_idxs) = image_id_to_loads.get(&old_variable_id) {
            for &op_load_idx in op_load_idxs {
                word_inserts.push(WordInsert {
                    idx: op_load_idx + 3,
                    word: new_variable_id,
                    head_idx: *op_load_idx,
                });
            }
        }
    }

    // 6. Insert new OpDecorate
    let DecorateOut {
        descriptor_sets_to_correct,
    } = util::decorate(DecorateIn {
        spv: &spv,
        instruction_inserts: &mut instruction_inserts,
        first_op_deocrate_idx,
        op_decorate_idxs: &op_decorate_idxs,
        affected_variables: &affected_variables,
    });

    // 7. Insert New Instructions
    insert_new_instructions(&spv, &mut new_spv, &word_inserts, &instruction_inserts);

    // 8. Correct OpDecorate Bindings
    util::correct_decorate(CorrectDecorateIn {
        new_spv: &mut new_spv,
        descriptor_sets_to_correct,
    });

    // 9. Remove Instructions that have been Whited Out.
    prune_noops(&mut new_spv);

    // 10. Write New Header and New Code
    Ok(fuse_final(spv_header, new_spv, instruction_bound))
}
