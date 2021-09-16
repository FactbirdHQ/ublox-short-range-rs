//! This module is required in order to satisfy the requirements of defmt, while running tests.
//! Note that this will cause all log `defmt::` log statements to be thrown away.
use core::ptr::NonNull;

#[defmt::global_logger]
struct Logger;
impl defmt::Write for Logger {
    fn write(&mut self, _bytes: &[u8]) {}
}

unsafe impl defmt::Logger for Logger {
    fn acquire() -> Option<NonNull<dyn defmt::Write>> {
        Some(NonNull::from(&Logger as &dyn defmt::Write))
    }

    unsafe fn release(_: NonNull<dyn defmt::Write>) {}
}

defmt::timestamp!("");

#[export_name = "_defmt_panic"]
fn panic() -> ! {
    panic!()
}
