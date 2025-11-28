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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MixState {
    Mixed,
    PotentiallyMixed,
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
pub fn drefsplitter(
    in_spv: &[u32],
    corrections: &mut Option<CorrectionMap>,
) -> Result<Vec<u32>, ()> {
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
        return Ok(in_spv.to_vec());
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

    for (id, load_idx, ty) in object_ids.iter().copied() {
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
    let patch_variable_idxs = op_variable_idxs
        .iter()
        .filter_map(|idx: &usize| {
            let result_id = spv[*idx + 2];
            mixed_object_ids
                .iter()
                .find(|id| id.inner() == result_id)
                .map(|id| id.next(*idx))
        })
        .collect::<Vec<_>>();

    // 7. Find OpFunctionParameter of ~~the mismatched~~ ALL images operations
    // Later, we can keep the ones that trace to mismatched global variables
    let patch_function_parameter_idxs = op_function_parameter_idxs
        .iter()
        .filter_map(|idx: &usize| {
            let result_id = spv[*idx + 2];
            mixed_object_ids
                .iter()
                .find_map(|id| {
                    (id.inner() == result_id).then_some((id.next(*idx), MixState::Mixed))
                })
                .or(object_ids.iter().find_map(|(id, _, _)| {
                    (id.inner() == result_id).then_some((id.next(*idx), MixState::PotentiallyMixed))
                }))
        })
        .collect::<Vec<_>>();

    // 8. Find the OpVariable that eventually reaches OpFunctionCall of our OpFunctions
    // Because functions may be deeply nested, we'll have to account for other OpFunctionCalls
    let function_patch_variables_with_calls = patch_function_parameter_idxs
        .iter()
        .map(|&(idx, mix_state)| {
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
                mix_state,
            )
        })
        .collect::<Vec<_>>();

    // Filter out PotentiallyMixed parameters that don't relate to any Mixed function parameters or
    // mixed variables
    // TODO: This cannot handle mixing between different contexts, see `test_hidden3_dref.frag`
    let function_patch_variables_with_calls = function_patch_variables_with_calls
        .iter()
        .cloned()
        .filter_map(|(variables, calls, mix_state)| match mix_state {
            MixState::Mixed => Some((variables, calls)),
            MixState::PotentiallyMixed => (function_patch_variables_with_calls.iter().any(
                |(mixed_variables, _, mix_state)| {
                    *mix_state == MixState::Mixed
                        && mixed_variables
                            .iter()
                            .any(|va| variables.iter().any(|vb| va == vb))
                },
            ) || patch_variable_idxs
                .iter()
                .any(|idx| variables.iter().any(|va| va == idx)))
            .then_some((variables, calls)),
        })
        .collect::<Vec<_>>();

    let mut patch_variable_idxs = patch_variable_idxs
        .iter()
        .copied()
        .map(|idx| (idx, LoadType::Variable))
        .collect::<Vec<_>>();

    let mut function_patch_variable_set = HashSet::new();
    for (variables, _) in function_patch_variables_with_calls.iter() {
        for variable in variables {
            if !function_patch_variable_set.contains(variable) {
                patch_variable_idxs.push((*variable, LoadType::FunctionArgument));
                function_patch_variable_set.insert(*variable);
            }
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

    // 10. Find OpTypeImage that resulted in OpTypePointer
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
                            instruction_bound - 1
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

    // 11. New OpVariable with a new_id, patch old OpLoads, and new depth=1 OpTypeImage.
    // Map new function arguments to the correct instructions.
    // NOTE: GENERALLY, with glslc, each OpImage* will get its own OpLoad, so we don't need to
    // check that its result isn't used for both regular and dref operations!
    let mut affected_variables = Vec::new();

    // There may be a shared OpTypeFunction but not shared OpFunctionParameter
    let mut patched_function_types = HashMap::new();
    let mut patched_function_parameters = HashSet::new();

    // We may patch ourselves a new OpTypeFunction multiple times.
    // Maps function type id and function index to our new type.
    let mut defered_new_function_types: HashMap<(u32, usize), InstructionInsert> = HashMap::new();

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
            correction_type: match complement_ty {
                OperationVariant::Regular => CorrectionType::SplitDrefComparison,
                OperationVariant::Dref => CorrectionType::SplitDrefRegular,
            },
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
                let mut function_id_and_index_to_new_parameter_id = HashMap::new();

                // Patch function types, definition parameter, and final loads
                for (variables, calls) in function_patch_variables_with_calls.iter() {
                    if variables.contains(&variable_idx_typed.next(variable_idx)) {
                        for &call in calls.iter().rev() {
                            let function_id = spv[call.call_parameter.function_idx + 2];
                            let type_function_id = spv[call.call_parameter.function_idx + 4];
                            if !patched_function_parameters.contains(&(
                                call.call_parameter.parameter_instruction_idx,
                                spv[call.call_parameter.function_idx + 2],
                            )) {
                                let Some(type_function_idx) =
                                    op_type_function_idxs.iter().find(|&&idx| {
                                        let result_id = spv[idx + 1];
                                        type_function_id == result_id
                                    })
                                else {
                                    panic!(
                                        "OpTypeFunction does not exist for function {}, type {}",
                                        function_id, type_function_id
                                    );
                                };

                                // To allow multiple patching we can either patch by taking
                                // an instruction directly from the code, or by patching a
                                // function we have already began
                                // We save patching the function's type for later in case of
                                // duplicate OpTypeFunction
                                {
                                    let (new_type_function_id, type_instruction_type_info) =
                                        if let Some(new_function_type) = defered_new_function_types
                                            .get_mut(&(
                                                type_function_id,
                                                call.call_parameter.function_idx,
                                            ))
                                        {
                                            new_function_type.instruction.insert(
                                                3 + call.call_parameter.parameter_instruction_idx
                                                    + 1
                                                    + 1,
                                                complement_tp_id,
                                            );
                                            new_function_type.instruction[0] = encode_word(
                                                new_function_type.instruction.len() as u16,
                                                SPV_INSTRUCTION_OP_TYPE_FUNCTION,
                                            );
                                            (
                                                new_function_type.instruction[1],
                                                new_function_type.instruction[2..].to_vec(),
                                            )
                                        } else {
                                            let new_function_type_id = instruction_bound;
                                            instruction_bound += 1;

                                            let word_count = hiword(spv[*type_function_idx]);
                                            let mut type_function = Vec::new();
                                            type_function.extend_from_slice(
                                                &spv[*type_function_idx
                                                    ..*type_function_idx + word_count as usize],
                                            );
                                            type_function[0] = encode_word(
                                                word_count + 1,
                                                SPV_INSTRUCTION_OP_TYPE_FUNCTION,
                                            );
                                            type_function[1] = new_function_type_id;
                                            type_function.insert(
                                                3 + call.call_parameter.parameter_instruction_idx
                                                    + 1,
                                                complement_tp_id,
                                            );

                                            let type_instruction_type_info =
                                                type_function[2..].to_vec();

                                            defered_new_function_types.insert(
                                                (
                                                    type_function_id,
                                                    call.call_parameter.function_idx,
                                                ),
                                                InstructionInsert {
                                                    previous_spv_idx: *type_function_idx,
                                                    instruction: type_function,
                                                },
                                            );
                                            (new_function_type_id, type_instruction_type_info)
                                        };
                                    let entry = patched_function_types
                                        .entry(type_instruction_type_info)
                                        .or_insert((new_type_function_id, vec![]));
                                    entry
                                        .1
                                        .push((type_function_id, call.call_parameter.function_idx));
                                }

                                // Patch function parameter
                                let new_parameter_id = instruction_bound;
                                instruction_bound += 1;
                                instruction_inserts.push(InstructionInsert {
                                    previous_spv_idx: call.call_parameter.parameter_idx,
                                    instruction: vec![
                                        encode_word(3, SPV_INSTRUCTION_OP_FUNCTION_PARAMTER),
                                        complement_tp_id,
                                        new_parameter_id,
                                    ],
                                });

                                // Use our new parameters to patch dependent OpLoads
                                for load_idx in op_load_idxs.iter() {
                                    let result_id = spv[load_idx + 2];
                                    let ptr_id = spv[load_idx + 3];
                                    let parameter_result_id =
                                        spv[call.call_parameter.parameter_idx + 2];

                                    // TODO: OPT Someone else can come by and rearrange these silly data
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
                                            new_spv[load_idx + 3] = new_parameter_id;
                                        }
                                    }
                                }

                                let function_id = spv[call.function_call_idx + 3];
                                function_id_and_index_to_new_parameter_id.insert(
                                    (function_id, call.call_parameter.parameter_instruction_idx),
                                    new_parameter_id,
                                );
                                patched_function_parameters.insert((
                                    call.call_parameter.parameter_instruction_idx,
                                    function_id,
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

    // 12. Remove duplicate function types and patch them into OpFunction
    for (_, (new_type_function_id, functions)) in patched_function_types.into_iter() {
        for (idx, &(type_function_id, function_idx)) in functions.iter().enumerate() {
            if idx != 0 {
                defered_new_function_types.remove(&(type_function_id, function_idx));
            }
            new_spv[function_idx + 4] = new_type_function_id;
        }
    }

    // We now insert our new function types
    for (_, new_instruction) in defered_new_function_types {
        instruction_inserts.push(new_instruction)
    }

    // 13. Insert new OpDecorate
    let DecorateOut {
        descriptor_sets_to_correct,
    } = util::decorate(DecorateIn {
        spv: &spv,
        instruction_inserts: &mut instruction_inserts,
        first_op_deocrate_idx,
        op_decorate_idxs: &op_decorate_idxs,
        affected_variables: &affected_variables,
        corrections,
    });

    // 14. Insert New Instructions
    insert_new_instructions(&spv, &mut new_spv, &word_inserts, &instruction_inserts);

    // 15. Correct OpDecorate Bindings
    util::correct_decorate(CorrectDecorateIn {
        new_spv: &mut new_spv,
        descriptor_sets_to_correct,
    });

    // 16. Remove Instructions that have been Whited Out.
    prune_noops(&mut new_spv);

    // 17. Write New Header and New Code
    Ok(fuse_final(spv_header, new_spv, instruction_bound))
}
