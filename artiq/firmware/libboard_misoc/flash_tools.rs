use csr;

pub unsafe fn reload () -> ! {
    csr::icap::iprog_write(1);
    loop {}
}
