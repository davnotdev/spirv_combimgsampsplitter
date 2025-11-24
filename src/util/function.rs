use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParameterEntry {
    pub parameter_idx: usize,
    pub function_idx: usize,
    pub parameter_instruction_idx: usize,
}

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

// NOTE: You will see this comment everywhere: Someone can find a better algorithm later.
pub fn get_function_index_of_instruction_index(spv: &[u32], instruction_idx: usize) -> usize {
    let mut spv_idx = 0;
    let mut last_function_idx = 0;
    while spv_idx < instruction_idx {
        let op = spv[spv_idx];
        let word_count = hiword(op);
        let instruction = loword(op);

        if instruction == SPV_INSTRUCTION_OP_FUNCTION {
            last_function_idx = spv_idx
        }
        spv_idx += word_count as usize
    }

    last_function_idx
}

// Trace a function backwards to a OpVariable, return variables and dependent function calls
pub struct TraceFunctionArgumentToVariablesIn<'a> {
    pub spv: &'a [u32],
    pub op_variable_idxs: &'a [usize],
    pub op_function_parameter_idxs: &'a [usize],
    pub op_function_call_idxs: &'a [usize],

    pub entry: ParameterEntry,
    pub traced_function_call_idxs: &'a mut Vec<TracedFunctionCall>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TracedFunctionCall {
    pub function_call_idx: usize,
    pub call_parameter: ParameterEntry,
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
            inputs.traced_function_call_idxs.push(TracedFunctionCall {
                function_call_idx: *idx,
                call_parameter: entry,
            });
            let argument_id = spv[idx + 4 + entry.parameter_instruction_idx];
            if let Some(mut out_variables) =
                trace_function_argument_to_variables_inner(&mut inputs, argument_id)
            {
                variables.append(&mut out_variables);
            }
        }
    }

    variables.dedup();
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
        op_function_parameter_idxs,
        entry: _,
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
