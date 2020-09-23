use std::ffi::CString;

/// Executes a cmd. cmd must contain no 0s.
pub fn system<T: Into<Vec<u8>>>(cmd: T) {
    let cmd = CString::new(cmd).expect("Valid cmd CString");

    unsafe {
        libc::system(cmd.as_ptr());
    }
}

pub fn open_program(path: &str) {
    system(format!("\"{}\"", path));
}
