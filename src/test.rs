use super::{combimgsampsplitter, u32_slice_to_u8_vec, u8_slice_to_u32_vec};
use naga::{back, front, valid};

fn try_spv_to_wgsl(spv: &[u8]) {
    let module = front::spv::parse_u8_slice(spv, &front::spv::Options::default()).unwrap();

    let mut caps = valid::Capabilities::default();
    caps.set(
        valid::Capabilities::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
        true,
    );
    caps.set(valid::Capabilities::SAMPLER_NON_UNIFORM_INDEXING, true);

    let mut info = valid::Validator::new(valid::ValidationFlags::all(), caps);
    let info = info.validate(&module).unwrap();

    back::wgsl::write_string(&module, &info, back::wgsl::WriterFlags::all()).unwrap();
}

#[test]
fn spv_test() {
    let spv = include_bytes!("../test/test.spv");
    let spv = u8_slice_to_u32_vec(spv);
    let out_spv = combimgsampsplitter(&spv).unwrap();
    let out_spv = u32_slice_to_u8_vec(&out_spv);
    try_spv_to_wgsl(&out_spv);
}

#[test]
fn spv_test_arrayed() {
    let spv = include_bytes!("../test/test_arrayed.spv");
    let spv = u8_slice_to_u32_vec(spv);
    let out_spv = combimgsampsplitter(&spv).unwrap();
    let out_spv = u32_slice_to_u8_vec(&out_spv);
    try_spv_to_wgsl(&out_spv);
}

#[test]
fn spv_test_nested() {
    let spv = include_bytes!("../test/test_nested.spv");
    let spv = u8_slice_to_u32_vec(spv);
    let out_spv = combimgsampsplitter(&spv).unwrap();
    let out_spv = u32_slice_to_u8_vec(&out_spv);

    try_spv_to_wgsl(&out_spv);
}

#[test]
fn spv_test_mixed() {
    let spv = include_bytes!("../test/test_mixed.spv");
    let spv = u8_slice_to_u32_vec(spv);
    let out_spv = combimgsampsplitter(&spv).unwrap();
    let out_spv = u32_slice_to_u8_vec(&out_spv);

    try_spv_to_wgsl(&out_spv);
}
