use std::ffi::c_void;

#[allow(dead_code)]
extern "C" {
    pub(crate) fn asort_asc_f32(
        x: *const c_void,
        dst: *mut c_void,
        nrows: i32,
        ncols: i32,
        inplace: bool,
        stream: i64,
    );
    pub(crate) fn asort_asc_f16(
        x: *const c_void,
        dst: *mut c_void,
        nrows: i32,
        ncols: i32,
        inplace: bool,
        stream: i64,
    );
    pub(crate) fn asort_asc_bf16(
        x: *const c_void,
        dst: *mut c_void,
        nrows: i32,
        ncols: i32,
        inplace: bool,
        stream: i64,
    );
    pub(crate) fn asort_asc_f64(
        x: *const c_void,
        dst: *mut c_void,
        nrows: i32,
        ncols: i32,
        inplace: bool,
        stream: i64,
    );
    pub(crate) fn asort_asc_u8(
        x: *const c_void,
        dst: *mut c_void,
        nrows: i32,
        ncols: i32,
        inplace: bool,
        stream: i64,
    );
    pub(crate) fn asort_asc_u32(
        x: *const c_void,
        dst: *mut c_void,
        nrows: i32,
        ncols: i32,
        inplace: bool,
        stream: i64,
    );
    pub(crate) fn asort_asc_i64(
        x: *const c_void,
        dst: *mut c_void,
        nrows: i32,
        ncols: i32,
        inplace: bool,
        stream: i64,
    );
    pub(crate) fn asort_desc_f32(
        x: *const c_void,
        dst: *mut c_void,
        nrows: i32,
        ncols: i32,
        inplace: bool,
        stream: i64,
    );
    pub(crate) fn asort_desc_f16(
        x: *const c_void,
        dst: *mut c_void,
        nrows: i32,
        ncols: i32,
        inplace: bool,
        stream: i64,
    );
    pub(crate) fn asort_desc_bf16(
        x: *const c_void,
        dst: *mut c_void,
        nrows: i32,
        ncols: i32,
        inplace: bool,
        stream: i64,
    );
    pub(crate) fn asort_desc_f64(
        x: *const c_void,
        dst: *mut c_void,
        nrows: i32,
        ncols: i32,
        inplace: bool,
        stream: i64,
    );
    pub(crate) fn asort_desc_u8(
        x: *const c_void,
        dst: *mut c_void,
        nrows: i32,
        ncols: i32,
        inplace: bool,
        stream: i64,
    );
    pub(crate) fn asort_desc_u32(
        x: *const c_void,
        dst: *mut c_void,
        nrows: i32,
        ncols: i32,
        inplace: bool,
        stream: i64,
    );
    pub(crate) fn asort_desc_i64(
        x: *const c_void,
        dst: *mut c_void,
        nrows: i32,
        ncols: i32,
        inplace: bool,
        stream: i64,
    );

    pub(crate) fn topk_f32(
        input: *const c_void,
        values_out: *mut c_void,
        indices_out: *mut c_void,
        nrows: i32,
        ncols: i32,
        k: i32,
        stream: i64,
    );
    pub(crate) fn topk_f16(
        input: *const c_void,
        values_out: *mut c_void,
        indices_out: *mut c_void,
        nrows: i32,
        ncols: i32,
        k: i32,
        stream: i64,
    );
    pub(crate) fn topk_bf16(
        input: *const c_void,
        values_out: *mut c_void,
        indices_out: *mut c_void,
        nrows: i32,
        ncols: i32,
        k: i32,
        stream: i64,
    );

    pub(crate) fn topk_softmax_f32(
        input: *const c_void,
        weights_out: *mut c_void,
        indices_out: *mut c_void,
        nrows: i32,
        ncols: i32,
        k: i32,
        stream: i64,
    );
    pub(crate) fn topk_softmax_f16(
        input: *const c_void,
        weights_out: *mut c_void,
        indices_out: *mut c_void,
        nrows: i32,
        ncols: i32,
        k: i32,
        stream: i64,
    );
    pub(crate) fn topk_softmax_bf16(
        input: *const c_void,
        weights_out: *mut c_void,
        indices_out: *mut c_void,
        nrows: i32,
        ncols: i32,
        k: i32,
        stream: i64,
    );
}
