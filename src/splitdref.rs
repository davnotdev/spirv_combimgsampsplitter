use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OperationVariant {
    Regular,
    Dref,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoadType {
    Variable,
    FunctionArgument,
}

trait IsIndexOrId {}
impl IsIndexOrId for u32 {}
impl IsIndexOrId for usize {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum PatchObjectType<T: IsIndexOrId> {
    Sampler(T),
    Image(T),
}

impl<T> PatchObjectType<T>
where
    T: IsIndexOrId,
{
    fn next<N: IsIndexOrId>(self, next_id: N) -> PatchObjectType<N> {
        match self {
            PatchObjectType::Sampler(_) => PatchObjectType::Sampler(next_id),
            PatchObjectType::Image(_) => PatchObjectType::Image(next_id),
        }
    }

    fn inner(self) -> T {
        match self {
            PatchObjectType::Sampler(v) => v,
            PatchObjectType::Image(v) => v,
        }
    }
}

/// Perform the operation on a `Vec<u32>`.
/// Use [u8_slice_to_u32_vec] to convert a `&[u8]` into a `Vec<u32>`
pub fn drefsplitter(in_spv: &[u32]) -> Result<Vec<u32>, ()> {
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
    let mut op_dref_operation_idxs = vec![];
    let mut op_sampled_operation_idxs = vec![];
    let mut op_sampled_image_idxs = vec![];
    let mut op_load_idxs = vec![];
    let mut op_variable_idxs = vec![];
    let mut op_decorate_idxs = vec![];
    let mut op_type_image_idxs = vec![];
    let mut op_type_pointer_idxs = vec![];
    let mut op_type_function_idxs = vec![];
    let mut op_function_idxs = vec![];
    let mut op_function_call_idxs = vec![];
    let mut op_function_parameter_idxs = vec![];

    let mut first_op_type_sampler_id = None;
    let mut first_op_type_pointer_sampler_id = None;

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
            | SPV_INSTRUCTION_OP_IMAGE_SPARSE_GATHER => op_sampled_operation_idxs.push(spv_idx),
            SPV_INSTRUCTION_OP_VARIABLE => op_variable_idxs.push(spv_idx),
            SPV_INSTRUCTION_OP_DECORATE => op_decorate_idxs.push(spv_idx),
            SPV_INSTRUCTION_OP_TYPE_IMAGE => op_type_image_idxs.push(spv_idx),
            SPV_INSTRUCTION_OP_TYPE_SAMPLER => {
                first_op_type_sampler_id.get_or_insert(spv[spv_idx + 1]);
            }
            SPV_INSTRUCTION_OP_TYPE_POINTER => {
                if first_op_type_sampler_id == Some(spv[spv_idx + 3])
                    && spv[spv_idx + 2] == SPV_STORAGE_CLASS_UNIFORM_CONSTANT
                {
                    first_op_type_pointer_sampler_id = Some(spv[spv_idx + 1]);
                }
                op_type_pointer_idxs.push(spv_idx)
            }
            SPV_INSTRUCTION_OP_TYPE_FUNCTION => op_type_function_idxs.push(spv_idx),
            SPV_INSTRUCTION_OP_FUNCTION => op_function_idxs.push(spv_idx),
            SPV_INSTRUCTION_OP_FUNCTION_CALL => op_function_call_idxs.push(spv_idx),
            SPV_INSTRUCTION_OP_FUNCTION_PARAMTER => op_function_parameter_idxs.push(spv_idx),
            _ => {}
        }

        spv_idx += word_count as usize;
    }

    let first_op_deocrate_idx = op_decorate_idxs.first().copied();

    // If there is no OpTypeSampler, either this is invalid, or we do not need to do any patching at all.
    let (Some(first_op_type_sampler_id), Some(first_op_type_pointer_sampler_id)) =
        (first_op_type_sampler_id, first_op_type_pointer_sampler_id)
    else {
        return Ok(spv);
    };

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
    let loaded_variable_ids = op_sampled_image_idxs
        .iter()
        .filter_map(|idx| {
            let sampled_result_id = spv[idx + 2];
            let loaded_image_id = spv[idx + 3];
            let loaded_sampler_id = spv[idx + 4];
            loaded_sampled_image_ids
                .iter()
                .find_map(|(id, ty)| (*id == sampled_result_id).then_some(ty))
                .map(|ty| {
                    [
                        (PatchObjectType::Image(loaded_image_id), ty),
                        (PatchObjectType::Sampler(loaded_sampler_id), ty),
                    ]
                })
        })
        .flatten()
        .collect::<Vec<_>>();

    // 4. Backtrack to find the OpLoad that resulted in our loaded images
    let object_ids = op_load_idxs
        .iter()
        .filter_map(|idx| {
            let loaded_result_id = spv[idx + 2];
            let original_image_or_sampler = spv[idx + 3];
            loaded_variable_ids
                .iter()
                .find_map(|(id, ty)| (id.inner() == loaded_result_id).then_some((id, ty)))
                .map(|(id, ty)| (id.next(original_image_or_sampler), idx, ty))
        })
        .collect::<Vec<_>>();

    // 5. Find the images that mismatch operations
    let mut mixed_object_ids = HashMap::new();
    let mut patch_object_id_to_loads = HashMap::new();

    for (id, load_idx, ty) in object_ids {
        let entry = mixed_object_ids.entry(id).or_insert((false, false));

        match ty {
            OperationVariant::Regular => entry.0 = true,
            OperationVariant::Dref => {
                entry.1 = true;
            }
        }
        patch_object_id_to_loads
            .entry(id)
            .or_insert(vec![])
            .push((load_idx, ty));
    }

    let mixed_object_ids = mixed_object_ids
        .into_iter()
        .filter_map(|(id, (uses_regular, uses_dref))| (uses_regular && uses_dref).then_some(id))
        .collect::<Vec<_>>();

    // 6. Find the OpVariable of the mismatched images
    let filter_map_mixed_object_ids_for_access = |idx: &usize| {
        let result_id = spv[*idx + 2];
        mixed_object_ids
            .iter()
            .find(|id| id.inner() == result_id)
            .map(|id| id.next(*idx))
    };
    let patch_variable_idxs = op_variable_idxs
        .iter()
        .filter_map(filter_map_mixed_object_ids_for_access)
        .collect::<Vec<_>>();

    // 7. Find OpFunctionParameter of the mismatched images
    let patch_function_parameter_idxs = op_function_parameter_idxs
        .iter()
        .filter_map(filter_map_mixed_object_ids_for_access)
        .collect::<Vec<_>>();

    // 8. Find the OpVariable that eventually reaches OpFunctionCall of our OpFunctions
    // Because functions may be deeply nested, we'll have to account for other OpFunctionCalls
    let function_patch_variables_with_calls = patch_function_parameter_idxs
        .iter()
        .map(|&idx| {
            let mut traced_function_calls = vec![];
            let entry = get_function_from_parameter(&spv, idx.inner());
            let variables =
                trace_function_argument_to_variables(TraceFunctionArgumentToVariablesIn {
                    spv: &spv,
                    op_variable_idxs: &op_variable_idxs,
                    op_function_parameter_idxs: &op_function_parameter_idxs,
                    op_function_call_idxs: &op_function_call_idxs,
                    entry,
                    traced_function_call_idxs: &mut traced_function_calls,
                });
            (
                variables
                    .into_iter()
                    .map(|v| idx.next(v))
                    .collect::<Vec<_>>(),
                traced_function_calls,
            )
        })
        .collect::<Vec<_>>();

    let mut patch_variable_idxs = patch_variable_idxs
        .iter()
        .copied()
        .map(|idx| (idx, LoadType::Variable))
        .collect::<Vec<_>>();

    for (variables, _) in function_patch_variables_with_calls.iter() {
        for variable in variables {
            patch_variable_idxs.push((*variable, LoadType::FunctionArgument));
        }
    }

    // 9. Find OpTypePointer that resulted in OpVariable
    let patch_variable_idxs = patch_variable_idxs.into_iter().map(|(variable_idx, lty)| {
        let type_pointer_id = spv[variable_idx.inner() + 1];
        let maybe_tp_idx = op_type_pointer_idxs.iter().find(|&tp_idx| {
            let tp_id = spv[tp_idx + 1];
            type_pointer_id == tp_id
        });
        (variable_idx, lty, maybe_tp_idx.copied())
    });

    // 9. Find OpTypeImage that resulted in OpTypePointer
    //    We also want to create an complement OpTypeImage (depth=!depth) (without duplicates) and
    //    a respective OpTypePointer ~~and OpTypeSampledImage pair~~ (also no duplicates).
    let mut existing_type_pointers_from_type_image = HashMap::new();
    let mut existing_type_images_from_complement_instruction = HashMap::new();

    let patch_variable_idxs = patch_variable_idxs
        .map(|(variable_idx, lty, tp_idx)| {
            match variable_idx {
                v @ PatchObjectType::Sampler(variable_idx) => {
                    (
                        v.next(variable_idx),
                        lty,
                        first_op_type_sampler_id,
                        first_op_type_pointer_sampler_id,
                        first_op_type_sampler_id,
                        // From the perspective of a SPIRV sampler variable, this doesn't matter
                        OperationVariant::Dref,
                    )
                }
                v @ PatchObjectType::Image(variable_idx) => {
                    let variable_result_id = spv[variable_idx];
                    let image_type_id = if let Some(tp_idx) = tp_idx {
                        // type_image_id
                        spv[tp_idx + 3]
                    } else if let Some(load_idxs) =
                        patch_object_id_to_loads.get(&PatchObjectType::Image(variable_result_id))
                        && let Some(&(load_idx, _)) = load_idxs.first()
                    {
                        // We don't have a type pointer, let's find the OpTypeImage via our original OpLoad!
                        // load_type_result_id
                        spv[load_idx + 1]
                    } else {
                        unreachable!(
                            "Our OpVariable image id should always point back to a OpLoad id"
                        );
                    };

                    // Grab the existing type image
                    let (ti_idx, ti_id) = op_type_image_idxs
                        .iter()
                        .find_map(|&ti_idx| {
                            let result_id = spv[ti_idx + 1];
                            (result_id == image_type_id).then_some((ti_idx, result_id))
                        })
                        .unwrap();

                    // Try to find an type image with the complement properties or (re-)create one
                    let ti_word_count = hiword(spv[ti_idx]) as usize;
                    let mut ti_complement = spv[ti_idx + 2..ti_idx + ti_word_count].to_vec();
                    let complement_ty = match ti_complement[2] {
                        0 | 2 => {
                            ti_complement[2] = 1;
                            OperationVariant::Dref
                        }
                        1 => {
                            ti_complement[2] = 0;
                            OperationVariant::Regular
                        }
                        _ => panic!("depth field on valid spv can only be 0, 1, or 2"),
                    };

                    let mut new_instructions = vec![];

                    let complement_ti_id = existing_type_images_from_complement_instruction
                        .get(&ti_complement)
                        .copied()
                        .or(op_type_image_idxs.iter().find_map(|&idx| {
                            let word_count = hiword(spv[idx]) as usize;
                            let result_id = spv[idx + 1];
                            // To have a consistent instruction ordering, we white-out the existing OpTypeImage
                            if ti_complement == spv[idx + 2..idx + word_count] {
                                for it in new_spv.iter_mut().skip(idx).take(word_count) {
                                    *it = encode_word(1, SPV_INSTRUCTION_OP_NOP);
                                }
                                Some(result_id)
                            } else {
                                None
                            }
                        }));
                    let complement_ti_id = {
                        let new_type_image_id = complement_ti_id.unwrap_or_else(|| {
                            instruction_bound += 1;
                            let new_type_image_id = instruction_bound - 1;
                            existing_type_images_from_complement_instruction
                                .insert(ti_complement.clone(), new_type_image_id);
                            new_type_image_id
                        });
                        if !existing_type_images_from_complement_instruction
                            .contains_key(&ti_complement)
                        {
                            let mut new_instruction = vec![
                                encode_word(
                                    (ti_complement.len() + 2) as u16,
                                    SPV_INSTRUCTION_OP_TYPE_IMAGE,
                                ),
                                new_type_image_id,
                            ];
                            existing_type_images_from_complement_instruction
                                .insert(ti_complement.clone(), new_type_image_id);
                            new_instruction.append(&mut ti_complement);
                            drop(ti_complement);
                            new_instructions.append(&mut new_instruction);
                        }
                        new_type_image_id
                    };

                    // Try to find a type id for complement type image or create one
                    let complement_tp_id = existing_type_pointers_from_type_image
                        .get(&complement_ti_id)
                        .copied()
                        .or(op_type_pointer_idxs.iter().find_map(|&idx| {
                            let result_id = spv[idx + 1];
                            let type_id = spv[idx + 3];
                            if type_id == complement_ti_id {
                                existing_type_pointers_from_type_image
                                    .insert(complement_ti_id, result_id);
                                Some(result_id)
                            } else {
                                None
                            }
                        }))
                        .unwrap_or_else(|| {
                            let new_type_pointer_id = instruction_bound;
                            instruction_bound += 1;
                            let mut new_instruction = vec![
                                encode_word(4, SPV_INSTRUCTION_OP_TYPE_POINTER),
                                new_type_pointer_id,
                                SPV_STORAGE_CLASS_UNIFORM_CONSTANT,
                                complement_ti_id,
                            ];
                            new_instructions.append(&mut new_instruction);
                            existing_type_pointers_from_type_image
                                .insert(complement_ti_id, new_type_pointer_id);
                            new_type_pointer_id
                        });

                    instruction_inserts.push(InstructionInsert {
                        previous_spv_idx: ti_idx,
                        instruction: new_instructions,
                    });

                    (
                        v.next(variable_idx),
                        lty,
                        ti_id,
                        complement_tp_id,
                        complement_ti_id,
                        complement_ty,
                    )
                }
            }
        })
        .collect::<Vec<_>>();

    // 10. New OpVariable with a new_id, patch old OpLoads, and new depth=1 OpTypeImage.
    // Map new function arguments to the correct instructions.
    // NOTE: GENERALLY, with glslc, each OpImage* will get its own OpLoad, so we don't need to
    // check that its result isn't used for both regular and dref operations!
    let mut affected_variables = Vec::new();

    // There may be a shared OpTypeFunction but not shared OpFunctionParameter
    let mut patched_function_types = HashSet::new();
    let mut patched_function_parameters = HashSet::new();

    for (
        variable_idx_typed,
        lty,
        original_ti_id,
        complement_tp_id,
        complement_ti_id,
        complement_ty,
    ) in patch_variable_idxs
    {
        let variable_idx = variable_idx_typed.inner();
        // OpVariable
        let word_count = hiword(spv[variable_idx]);
        let new_variable_id = instruction_bound;
        instruction_bound += 1;
        let mut new_variable = Vec::new();
        new_variable.extend_from_slice(&spv[variable_idx..variable_idx + word_count as usize]);
        new_variable[1] = complement_tp_id;
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
        match lty {
            LoadType::Variable => {
                let old_variable_id = spv[variable_idx + 2];
                if let Some(op_load_idxs) =
                    patch_object_id_to_loads.get(&variable_idx_typed.next(old_variable_id))
                {
                    for &(op_load_idx, ty) in op_load_idxs {
                        if **ty == complement_ty {
                            new_spv[op_load_idx + 1] = complement_ti_id;
                            new_spv[op_load_idx + 3] = new_variable_id;
                        } else {
                            new_spv[op_load_idx + 1] = original_ti_id;
                            new_spv[op_load_idx + 3] = old_variable_id;
                        };
                    }
                }
            }
            LoadType::FunctionArgument => {
                let old_variable_id = spv[variable_idx + 2];
                let mut function_id_and_index_to_new_parameter_id = HashMap::new();

                // Patch function types, definition parameter, and final loads
                for (variables, calls) in function_patch_variables_with_calls.iter() {
                    if variables.contains(&variable_idx_typed.next(variable_idx)) {
                        for &call in calls.iter().rev() {
                            if !patched_function_parameters.contains(&(
                                call.call_parameter.parameter_instruction_idx,
                                spv[call.call_parameter.function_idx + 2],
                            )) {
                                let duplicative_function_type = patched_function_types.contains(&(
                                    call.call_parameter.parameter_instruction_idx,
                                    spv[call.call_parameter.function_idx + 4],
                                ));
                                // Patch function type signature and parameters
                                let new_parameter_id = instruction_bound;
                                instruction_bound += 1;
                                patch_function_type(PatchFunctionTypeIn {
                                    spv: &spv,
                                    instruction_inserts: &mut instruction_inserts,
                                    word_inserts: &mut word_inserts,
                                    op_type_function_idxs: &op_type_function_idxs,
                                    patch_function_type: !duplicative_function_type,
                                    entry: &call.call_parameter,
                                    new_type_id: complement_tp_id,
                                    new_parameter_id,
                                });

                                // Use our new parameters to patch dependent OpLoads
                                for load_idx in op_load_idxs.iter() {
                                    let result_id = spv[load_idx + 2];
                                    let ptr_id = spv[load_idx + 3];
                                    let parameter_result_id =
                                        spv[call.call_parameter.parameter_idx + 2];

                                    // OPT: Someone else can come by and rearrange these silly data
                                    // structures later.
                                    if ptr_id == parameter_result_id {
                                        let ty = loaded_variable_ids
                                            .iter()
                                            .find_map(|&(id, ty)| {
                                                (id.inner() == result_id).then_some(ty)
                                            })
                                            .unwrap();
                                        if *ty == complement_ty {
                                            new_spv[load_idx + 1] = complement_ti_id;
                                            new_spv[load_idx + 3] = new_variable_id;
                                        } else {
                                            new_spv[load_idx + 1] = original_ti_id;
                                            new_spv[load_idx + 3] = old_variable_id;
                                        };
                                    }
                                }

                                let function_id = spv[call.function_call_idx + 3];
                                function_id_and_index_to_new_parameter_id.insert(
                                    (function_id, call.call_parameter.parameter_instruction_idx),
                                    new_parameter_id,
                                );
                                patched_function_parameters.insert((
                                    call.call_parameter.parameter_instruction_idx,
                                    spv[call.call_parameter.function_idx + 2],
                                ));
                                patched_function_types.insert((
                                    call.call_parameter.parameter_instruction_idx,
                                    spv[call.call_parameter.function_idx + 4],
                                ));
                            }
                        }
                    }
                }

                // Patch function calls that call other functions
                for (variables, calls) in function_patch_variables_with_calls.iter() {
                    if variables.contains(&variable_idx_typed.next(variable_idx)) {
                        for &call in calls.iter().rev() {
                            let function_idx = get_function_index_of_instruction_index(
                                &spv,
                                call.function_call_idx,
                            );
                            let function_id = spv[function_idx + 2];
                            if let Some(parameter_word) = function_id_and_index_to_new_parameter_id
                                .get(&(function_id, call.call_parameter.parameter_instruction_idx))
                            {
                                word_inserts.push(WordInsert {
                                    idx: call.function_call_idx
                                        + 4
                                        + call.call_parameter.parameter_instruction_idx,
                                    word: *parameter_word,
                                    head_idx: call.function_call_idx,
                                });
                            } else {
                                word_inserts.push(WordInsert {
                                    idx: call.function_call_idx
                                        + 4
                                        + call.call_parameter.parameter_instruction_idx,
                                    word: new_variable_id,
                                    head_idx: call.function_call_idx,
                                });
                            }
                        }
                    }
                }
            }
        }

        // OpSampledImage
        // NOTE: We did not patch in a new OpSampledImage and OpTypeSampledImage.
        // Thankfully, it seems that `spirv-val`, `naga`, nor `tint` seem to care.
    }

    // 11. Insert new OpDecorate
    let DecorateOut {
        descriptor_sets_to_correct,
    } = util::decorate(DecorateIn {
        spv: &spv,
        instruction_inserts: &mut instruction_inserts,
        first_op_deocrate_idx,
        op_decorate_idxs: &op_decorate_idxs,
        affected_variables: &affected_variables,
    });

    // 12. Insert New Instructions
    insert_new_instructions(&spv, &mut new_spv, &word_inserts, &instruction_inserts);

    // 13. Correct OpDecorate Bindings
    util::correct_decorate(CorrectDecorateIn {
        new_spv: &mut new_spv,
        descriptor_sets_to_correct,
    });

    // 14. Remove Instructions that have been Whited Out.
    prune_noops(&mut new_spv);

    // 15. Write New Header and New Code
    Ok(fuse_final(spv_header, new_spv, instruction_bound))
}
