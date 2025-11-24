use super::{combimgsampsplitter, drefsplitter, u8_slice_to_u32_vec, u32_slice_to_u8_vec};

use naga::{back, front, valid};
use spirv_tools::val::{self, Validator};

const SPV_VALIDATE: u8 = 0b0000001;
const NAGA_VALIDATE: u8 = 0b0000010;
const NAGA_CONVERT: u8 = 0b0000100;
const DO_ALL: u8 = 0xff;

#[macro_export]
macro_rules! test_with_spv_and_fn {
    ($NAME:ident, $FLAGS: expr, $SPV:expr, $FN:expr) => {
        #[test]
        fn $NAME() {
            let spv = include_bytes!($SPV);
            let spv = u8_slice_to_u32_vec(spv);
            let out_spv = $FN(&spv).unwrap();
            try_spv_to_wgsl(&out_spv, $FLAGS);
        }
    };
}

fn try_spv_to_wgsl(spv: &[u32], flags: u8) {
    let spv_u8 = u32_slice_to_u8_vec(spv);
    if flags & SPV_VALIDATE != 0 {
        let validator = val::create(None);
        validator.validate(spv, None).unwrap();
    }

    if flags & NAGA_VALIDATE != 0 {
        let module = front::spv::parse_u8_slice(&spv_u8, &front::spv::Options::default()).unwrap();

        let mut caps = valid::Capabilities::default();
        caps.set(
            valid::Capabilities::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
            true,
        );
        caps.set(valid::Capabilities::SAMPLER_NON_UNIFORM_INDEXING, true);

        let mut info = valid::Validator::new(valid::ValidationFlags::all(), caps);
        let info = info.validate(&module).unwrap();

        if flags & NAGA_CONVERT != 0 {
            back::wgsl::write_string(&module, &info, back::wgsl::WriterFlags::all()).unwrap();
        }
    }
}

test_with_spv_and_fn!(
    splitcombined_test,
    DO_ALL,
    "./test/splitcombined/test.spv",
    combimgsampsplitter
);
test_with_spv_and_fn!(
    splitcombined_test_arrayed,
    DO_ALL,
    "./test/splitcombined/test_arrayed.spv",
    combimgsampsplitter
);
test_with_spv_and_fn!(
    splitcombined_test_nested,
    DO_ALL,
    "./test/splitcombined/test_nested.spv",
    combimgsampsplitter
);
test_with_spv_and_fn!(
    splitcombined_test_mixed,
    DO_ALL,
    "./test/splitcombined/test_mixed.spv",
    combimgsampsplitter
);

test_with_spv_and_fn!(
    splitdref_test_wrong_type_image,
    SPV_VALIDATE,
    "./test/splitdref/test_wrong_type_image.spv",
    drefsplitter
);
test_with_spv_and_fn!(
    splitdref_test_image,
    SPV_VALIDATE,
    "./test/splitdref/test_image.spv",
    drefsplitter
);
test_with_spv_and_fn!(
    splitdref_test_nested_image,
    SPV_VALIDATE,
    "./test/splitdref/test_nested_image.spv",
    drefsplitter
);
test_with_spv_and_fn!(
    splitdref_test_nested2_image,
    SPV_VALIDATE,
    "./test/splitdref/test_nested2_image.spv",
    drefsplitter
);
test_with_spv_and_fn!(
    splitdref_test_sampler,
    SPV_VALIDATE,
    "./test/splitdref/test_sampler.spv",
    drefsplitter
);
test_with_spv_and_fn!(
    splitdref_test_nested_sampler,
    SPV_VALIDATE,
    "./test/splitdref/test_nested_sampler.spv",
    drefsplitter
);
test_with_spv_and_fn!(
    splitdref_test_nested2_sampler,
    SPV_VALIDATE,
    "./test/splitdref/test_nested2_sampler.spv",
    drefsplitter
);
test_with_spv_and_fn!(
    splitdref_test_hidden_dref,
    SPV_VALIDATE,
    "./test/splitdref/test_hidden_dref.spv",
    drefsplitter
);
test_with_spv_and_fn!(
    splitdref_test_hidden2_dref,
    SPV_VALIDATE,
    "./test/splitdref/test_hidden2_dref.spv",
    drefsplitter
);
test_with_spv_and_fn!(
    splitdref_test_hidden3_dref,
    SPV_VALIDATE,
    "./test/splitdref/test_hidden3_dref.spv",
    drefsplitter
);
