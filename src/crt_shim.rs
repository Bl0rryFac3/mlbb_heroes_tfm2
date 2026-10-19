use core::arch::global_asm;

#[no_mangle]
pub static _fltused: i32 = 0;

#[no_mangle]
pub static mut _tls_index: u32 = 0;

#[repr(C)]
pub struct ImageTlsDirectory64 {
    pub start_address_of_raw_data: *const u8,
    pub end_address_of_raw_data: *const u8,
    pub address_of_index: *mut u32,
    pub address_of_call_backs: *const *const u8,
    pub size_of_zero_fill: u32,
    pub characteristics: u32,
}

unsafe impl Sync for ImageTlsDirectory64 {}

#[link_section = ".tls"]
#[no_mangle]
pub static mut _tls_start: [u8; 0] = [];

#[link_section = ".tls$ZZZ"]
#[no_mangle]
pub static mut _tls_end: u64 = 0;

#[link_section = ".CRT$XLA"]
#[no_mangle]
pub static __xl_a: usize = 0;

#[link_section = ".CRT$XLZ"]
#[no_mangle]
pub static __xl_z: usize = 0;

#[no_mangle]
pub static _tls_used: ImageTlsDirectory64 = ImageTlsDirectory64 {
    start_address_of_raw_data: core::ptr::addr_of!(_tls_start) as *const u8,
    end_address_of_raw_data: core::ptr::addr_of!(_tls_end) as *const u8,
    address_of_index: core::ptr::addr_of_mut!(_tls_index),
    address_of_call_backs: unsafe { (core::ptr::addr_of!(__xl_a) as *const *const u8).add(1) },
    size_of_zero_fill: 0,
    characteristics: 0x00400000,
};

#[no_mangle]
pub unsafe extern "system" fn _DllMainCRTStartup(_hinst: *mut u8, _reason: u32, _reserved: *mut u8) -> i32 {
    1
}

#[no_mangle]
pub unsafe extern "C" fn atexit(_func: Option<unsafe extern "C" fn()>) -> i32 {
    0
}

#[no_mangle]
pub unsafe extern "C" fn __CxxFrameHandler3() -> i32 {
    0
}

#[no_mangle]
pub unsafe extern "C" fn _CxxThrowException(_p_exception_object: *mut u8, _p_throw_info: *mut u8) -> ! {
    core::hint::unreachable_unchecked()
}

#[no_mangle]
pub unsafe extern "C" fn round(x: f64) -> f64 {
    if x >= 0.0 {
        (x + 0.5) as i64 as f64
    } else {
        (x - 0.5) as i64 as f64
    }
}

global_asm!(
    ".section .rdata,\"dr\"",
    ".globl \"??_7type_info@@6B@\"",
    ".p2align 3",
    "\"??_7type_info@@6B@\":",
    ".quad 0",
    ".text",
    ".globl __chkstk",
    ".p2align 4",
    "__chkstk:",
    "sub rsp, 16",
    "mov [rsp], r10",
    "mov [rsp + 8], r11",
    "xor r11, r11",
    "lea r10, [rsp + 24]",
    "sub r10, rax",
    "cmovb r10, r11",
    "mov r11, gs:[0x10]",
    "cmp r10, r11",
    "jae 2f",
    "and r10w, 0xf000",
    "1:",
    "lea r11, [r11 - 0x1000]",
    "mov byte ptr [r11], 0",
    "cmp r10, r11",
    "jne 1b",
    "2:",
    "mov r10, [rsp]",
    "mov r11, [rsp + 8]",
    "add rsp, 16",
    "ret"
);
