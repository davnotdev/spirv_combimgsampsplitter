use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParameterEntry {
    pub parameter_idx: usize,
    pub function_idx: usize,
    pub parameter_instruction_idx: usize,
}

// Given a vec of paramater indices, find the (parameter, function, parameter index)
pub fn get_function_from_parameter(spv: &[u32], function_parameter_idx: usize) -> ParameterEntry {
    let mut spv_idx = function_parameter_idx;
    let mut param_idx = 0;
    let mut bumped_function = false;
    loop {
        let op = spv[spv_idx];
        let word_count = hiword(op);
        let instruction = loword(op);
        match instruction {
            SPV_INSTRUCTION_OP_FUNCTION_PARAMTER => {
                spv_idx -= word_count as usize;
                param_idx += 1;
            }
            SPV_INSTRUCTION_OP_FUNCTION => {
                return ParameterEntry {
                    parameter_idx: function_parameter_idx,
                    function_idx: spv_idx,
                    parameter_instruction_idx: param_idx - 1,
                };
            }
            _ => {
                if bumped_function {
                    panic!(
                        "Expected OpFunction or OpFunctionParameter, got {},{}",
                        word_count, instruction
                    );
                }
                // OpFunction is an offset of 5 rather than 3.
                spv_idx -= 2;
                bumped_function = true;
                continue;
            }
        }
    }
}

// Given a parameter, function, and parameter index, patch OpTypeFunction, OpFunctionParameter
pub struct PatchFunctionTypeIn<'a> {
    spv: &'a [u32],
    instruction_inserts: &'a mut Vec<InstructionInsert>,
    word_inserts: &'a mut Vec<WordInsert>,
    op_type_function_idxs: &'a [usize],

    entry: &'a ParameterEntry,
    new_type_id: u32,
    new_parameter_id: u32,
}

fn patch_function_type(inputs: PatchFunctionTypeIn) {
    let PatchFunctionTypeIn {
        spv,
        instruction_inserts,
        word_inserts,
        op_type_function_idxs,
        entry,
        new_type_id,
        new_parameter_id,
    } = inputs;

    let type_function_id = spv[entry.function_idx + 4];
    if let Some(idx) = op_type_function_idxs.iter().find(|&&idx| {
        let result_id = spv[idx + 1];
        type_function_id == result_id
    }) {
        word_inserts.push(WordInsert {
            idx: idx + 3 + entry.parameter_instruction_idx,
            word: new_type_id,
            head_idx: *idx,
        });
    }

    instruction_inserts.push(InstructionInsert {
        previous_spv_idx: entry.parameter_idx,
        instruction: vec![
            encode_word(3, SPV_INSTRUCTION_OP_FUNCTION_PARAMTER),
            new_type_id,
            new_parameter_id,
        ],
    });
}

// Trace a function backwards to a OpVariable, returns variable index, and list of affected
// (parameter, function, parameter index)
pub struct TraceFunctionArgumentToVariablesIn<'a> {
    pub spv: &'a [u32],
    pub op_variable_idxs: &'a [usize],
    pub op_function_parameter_idxs: &'a [usize],
    pub op_function_call_idxs: &'a [usize],

    pub entry: ParameterEntry,
    pub traced_function_call_idxs: &'a mut Vec<(usize, ParameterEntry)>,
}

pub fn trace_function_argument_to_variables(
    mut inputs: TraceFunctionArgumentToVariablesIn,
) -> Vec<usize> {
    let TraceFunctionArgumentToVariablesIn {
        spv,
        op_variable_idxs: _,
        op_function_parameter_idxs: _,
        op_function_call_idxs,
        entry,
        traced_function_call_idxs: _,
    } = inputs;

    let mut variables = vec![];
    for idx in op_function_call_idxs.iter() {
        let function_id = spv[idx + 3];
        if function_id == spv[entry.function_idx + 2] {
            inputs.traced_function_call_idxs.push((*idx, entry));
            let argument_id = spv[idx + 4 + entry.parameter_instruction_idx];
            if let Some(mut out_variables) =
                trace_function_argument_to_variables_inner(&mut inputs, argument_id)
            {
                variables.append(&mut out_variables);
            }
        }
    }

    variables.dedup();
    inputs.traced_function_call_idxs.dedup();

    variables
}

fn trace_function_argument_to_variables_inner(
    inputs: &mut TraceFunctionArgumentToVariablesIn,
    result_id: u32,
) -> Option<Vec<usize>> {
    let TraceFunctionArgumentToVariablesIn {
        spv,
        op_variable_idxs,
        op_function_call_idxs,
        entry: _,
        op_function_parameter_idxs,
        traced_function_call_idxs,
    } = inputs;

    enum TraceResult {
        Variable(usize),
        FunctionParameter(ParameterEntry),
    }

    match op_variable_idxs
        .iter()
        .find_map(|&idx| (spv[idx + 2] == result_id).then_some(TraceResult::Variable(idx)))
        .or(op_function_parameter_idxs.iter().find_map(|&idx| {
            (spv[idx + 2] == result_id).then_some(TraceResult::FunctionParameter(
                get_function_from_parameter(spv, idx),
            ))
        })) {
        Some(TraceResult::Variable(variable_idx)) => Some(vec![variable_idx]),
        Some(TraceResult::FunctionParameter(entry)) => Some(trace_function_argument_to_variables(
            TraceFunctionArgumentToVariablesIn {
                spv,
                op_variable_idxs,
                op_function_parameter_idxs,
                op_function_call_idxs,
                entry,
                traced_function_call_idxs,
            },
        )),
        _ => None,
    }
}
