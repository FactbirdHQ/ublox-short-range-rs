//! This module is required in order to satisfy the requirements of defmt, while running tests.
//! Note that this will cause all log `defmt::` log statements to be thrown away.

#[defmt::global_logger]
struct Logger;

unsafe impl defmt::Logger for Logger {
    fn acquire() {}

    unsafe fn flush() {}

    unsafe fn release() {}

    unsafe fn write(_bytes: &[u8]) {}
}

defmt::timestamp!("");

#[export_name = "_defmt_panic"]
fn panic() -> ! {
    panic!()
}
