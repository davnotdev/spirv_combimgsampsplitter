use super::*;

//  DREF OPS:
//
//  OpImageSampleDrefImplicitLod
//  OpImageSampleDrefExplicitLod
//  OpImageSampleProjDrefImplicitLod
//  OpImageSampleProjDrefExplicitLod
//  OpImageDrefGather
//  OpImageSparseSampleDrefImplicitLod
//  OpImageSparseSampleDrefExplicitLod
//  OpImageSparseDrefGather
//
//  - OpImageSampleProjDrefImplicitLod %result %sampled ...
//      - backtrace OpSampledImage %type %sampled %loaded_image
//          - backtrace OpLoad %type %loaded_image %image
//              - return %image

// ```
// bump OpDecorate %new_image
// duplicate OpVariable %image for %new_image
// replace OpLoad's %image with %new_image
// ```

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
    let mut op_dref_idxs = vec![];
    let mut op_sampled_image_idxs = vec![];
    let mut op_load_idxs = vec![];

    let mut spv_idx = 0;
    while spv_idx < spv.len() {
        let op = spv[spv_idx];
        let word_count = hiword(op);
        let instruction = loword(op);

        match instruction {
            SPV_INSTRUCTION_OP_TYPE_SAMPLED_IMAGE => op_sampled_image_idxs.push(spv_idx),
            SPV_INSTRUCTION_OP_LOAD => op_load_idxs.push(spv_idx),
            SPV_INSTRUCTION_OP_IMAGE_SAMPLE_DREF_IMPLICIT_LOD
            | SPV_INSTRUCTION_OP_IMAGE_SAMPLE_DREF_EXPLICIT_LOD
            | SPV_INSTRUCTION_OP_IMAGE_SAMPLE_PROJ_DREF_IMPLICIT_LOD
            | SPV_INSTRUCTION_OP_IMAGE_SAMPLE_PROJ_DREF_EXPLICIT_LOD
            | SPV_INSTRUCTION_OP_IMAGE_DREF_GATHER
            | SPV_INSTRUCTION_OP_IMAGE_SPARSE_SAMPLE_DREF_IMPLICIT_LOD
            | SPV_INSTRUCTION_OP_IMAGE_SPARSE_SAMPLE_DREF_EXPLICIT_LOD
            | SPV_INSTRUCTION_OP_IMAGE_SPARSE_DREF_GATHER => op_dref_idxs.push(spv_idx),

            _ => {}
        }

        spv_idx += word_count as usize;
    }

    // Remove Instructions that have been Whited Out.
    prune_noops(&mut new_spv);

    // Write New Header and New Code
    Ok(fuse_final(spv_header, new_spv, instruction_bound))
}
