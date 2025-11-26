use super::*;

pub struct CorrectDecorateIn<'a> {
    pub new_spv: &'a mut [u32],
    pub descriptor_sets_to_correct: HashSet<u32>,
}

// Correct descriptor sets whose binding index has been invalidated.
// This should be called after instructions have been inserted.
pub fn correct_decorate(cd_in: CorrectDecorateIn) {
    let CorrectDecorateIn {
        new_spv,
        descriptor_sets_to_correct,
    } = cd_in;
    let mut candidates = HashMap::new();

    let mut d_idx = 0;
    while d_idx < new_spv.len() {
        let op = new_spv[d_idx];
        let word_count = hiword(op);
        let instruction = loword(op);
        if instruction == SPV_INSTRUCTION_OP_DECORATE {
            match new_spv[d_idx + 2] {
                SPV_DECORATION_DESCRIPTOR_SET => {
                    candidates
                        .entry(new_spv[d_idx + 1])
                        .or_insert((None, None))
                        .0 = Some(new_spv[d_idx + 3])
                }
                SPV_DECORATION_BINDING => {
                    candidates
                        .entry(new_spv[d_idx + 1])
                        .or_insert((None, None))
                        .1 = Some((d_idx, new_spv[d_idx + 3]))
                }
                _ => {}
            }
        }

        d_idx += word_count as usize;
    }

    for descriptor_set in descriptor_sets_to_correct {
        let mut bindings = candidates
            .iter()
            .filter_map(|(_, &(maybe_descriptor_set, maybe_binding))| {
                let this_descriptor_set = maybe_descriptor_set.unwrap();
                let (binding_idx, this_binding) = maybe_binding.unwrap();
                (this_descriptor_set == descriptor_set).then_some((binding_idx, this_binding))
            })
            .collect::<Vec<_>>();
        bindings.sort_by_cached_key(|&(_, binding)| binding);

        // We can assume that our new ~~samplers~~ variables will have a greater instruction ID than the original
        // ~~combined image samplers~~ variables.
        let mut prev_binding = -1;
        let mut increment = 0;
        for (d_idx, binding) in bindings {
            if binding as i32 == prev_binding {
                increment += 1;
            }
            new_spv[d_idx + 3] = binding + increment;
            prev_binding = binding as i32;
        }
    }
}
