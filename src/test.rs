use super::{combimgsampsplitter, dreftexturesplitter, u8_slice_to_u32_vec, u32_slice_to_u8_vec};
use naga::{back, front, valid};

#[macro_export]
macro_rules! test_with_spv_and_fn {
    ($NAME:ident, $SPV:expr, $FN:expr) => {
        #[test]
        fn $NAME() {
            let spv = include_bytes!($SPV);
            let spv = u8_slice_to_u32_vec(spv);
            let out_spv = $FN(&spv).unwrap();
            let out_spv = u32_slice_to_u8_vec(&out_spv);
            try_spv_to_wgsl(&out_spv);
        }
    };
}

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

test_with_spv_and_fn!(
    splitcombined_test,
    "./test/splitcombined/test.spv",
    combimgsampsplitter
);
test_with_spv_and_fn!(
    splitcombined_test_arrayed,
    "./test/splitcombined/test_arrayed.spv",
    combimgsampsplitter
);
test_with_spv_and_fn!(
    splitcombined_test_nested,
    "./test/splitcombined/test_nested.spv",
    combimgsampsplitter
);
test_with_spv_and_fn!(
    splitcombined_test_mixed,
    "./test/splitcombined/test_mixed.spv",
    combimgsampsplitter
);

test_with_spv_and_fn!(
    splitdref_test,
    "./test/splitdref/test.spv",
    dreftexturesplitter
);

test_with_spv_and_fn!(
    splitdref_test_wrong_type_image,
    "./test/splitdref/test_wrong_type_image.spv",
    dreftexturesplitter
);
