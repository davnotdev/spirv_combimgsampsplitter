#![allow(clippy::missing_safety_doc)]

use core::{ffi, ptr, slice};
use spirv_webgpu_transform::{CorrectionMap, combimgsampsplitter, drefsplitter};

type TransformCorrectionMap = *mut ffi::c_void;

pub unsafe fn alloc_or_pass_correction_map(
    map: *mut TransformCorrectionMap,
) -> &'static mut Option<CorrectionMap> {
    unsafe {
        if map.is_null() {
            panic!(
                "Got null correction map pointer, pointer to existing correction map or SPIRV_WEBGPU_TRANFORM_CORRECTION_MAP_NULL"
            )
        }

        if (*map).is_null() {
            let owned = Box::new(None);
            let r = Box::leak(owned);
            let ptr = r as *mut Option<CorrectionMap> as TransformCorrectionMap;
            *map = ptr;
            r
        } else {
            Box::leak(Box::from_raw((*map).cast::<Option<CorrectionMap>>()))
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn spirv_webgpu_transform_combimgsampsplitter_alloc(
    in_spv: *const u32,
    in_count: u32,
    out_spv: *mut *const u32,
    out_count: *mut u32,
    correction_map: *mut TransformCorrectionMap,
) {
    let correction_map = unsafe { alloc_or_pass_correction_map(correction_map) };

    let in_spv = unsafe { slice::from_raw_parts(in_spv, in_count as usize) };
    match combimgsampsplitter(in_spv, correction_map) {
        Ok(spv) => unsafe {
            *out_count = spv.len() as u32;
            let leaked = Box::leak(spv.into_boxed_slice());
            *out_spv = leaked.as_ptr();
        },
        Err(_) => unsafe {
            *out_spv = ptr::null();
            *out_count = 0;
        },
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn spirv_webgpu_transform_combimgsampsplitter_free(out_spv: *mut u32) {
    unsafe { drop(Box::from_raw(out_spv)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn spirv_webgpu_transform_drefsplitter_alloc(
    in_spv: *const u32,
    in_count: u32,
    out_spv: *mut *const u32,
    out_count: *mut u32,
    correction_map: *mut TransformCorrectionMap,
) {
    let correction_map = unsafe { alloc_or_pass_correction_map(correction_map) };

    let in_spv = unsafe { slice::from_raw_parts(in_spv, in_count as usize) };
    match drefsplitter(in_spv, correction_map) {
        Ok(spv) => unsafe {
            *out_count = spv.len() as u32;
            let leaked = Box::leak(spv.into_boxed_slice());
            *out_spv = leaked.as_ptr();
        },
        Err(_) => unsafe {
            *out_spv = ptr::null();
            *out_count = 0;
        },
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn spirv_webgpu_transform_drefsplitter_free(out_spv: *mut u32) {
    unsafe { drop(Box::from_raw(out_spv)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn spirv_webgpu_transform_correction_map_free(
    correction_map: TransformCorrectionMap,
) {
    let _ = unsafe { Box::from_raw(correction_map.cast::<Option<CorrectionMap>>()) };
}
