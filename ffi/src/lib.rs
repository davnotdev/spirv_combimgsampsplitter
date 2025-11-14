#![allow(clippy::missing_safety_doc)]

use core::{ptr, slice};
use spirv_combimgsampsplitter::{combimgsampsplitter, dreftexturesplitter};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn combimgsampsplitter_alloc(
    in_spv: *const u32,
    in_count: u32,
    out_spv: *mut *const u32,
    out_count: *mut u32,
) {
    let in_spv = unsafe { slice::from_raw_parts(in_spv, in_count as usize) };
    match combimgsampsplitter(in_spv) {
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
pub unsafe extern "C" fn combimgsampsplitter_free(out_spv: *mut u32) {
    unsafe { drop(Box::from_raw(out_spv)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dreftexturesplitter_alloc(
    in_spv: *const u32,
    in_count: u32,
    out_spv: *mut *const u32,
    out_count: *mut u32,
) {
    let in_spv = unsafe { slice::from_raw_parts(in_spv, in_count as usize) };
    match dreftexturesplitter(in_spv) {
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
pub unsafe extern "C" fn dreftexturesplitter_free(out_spv: *mut u32) {
    unsafe { drop(Box::from_raw(out_spv)) }
}
