//! Process management syscalls
use crate::{
    mm::{MapPermission, PageTable, VirtAddr, translated_byte_buffer}, 
    task::{change_program_brk, current_user_token, exit_current_and_run_next, get_syscall_record, mmap, munmap, suspend_current_and_run_next},
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let token = current_user_token();
    let mut bufs = translated_byte_buffer(token, ts as *const u8, core::mem::size_of::<TimeVal>());
    let tv = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
    };
    let tv_bytes = unsafe {
        core::slice::from_raw_parts(&tv as *const _ as *const u8, core::mem::size_of::<TimeVal>())
    };
    let mut written = 0;
    for b in bufs.iter_mut() {
        let n = b.len();
        b.copy_from_slice(&tv_bytes[written..written+n]);
        written += n;
    }
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    let token = current_user_token();
    let page_table = PageTable::from_token(token);
    match trace_request {
        0 => {
            let va = VirtAddr::from(id);
            let pte_result = page_table.translate(va.floor());
            match pte_result {
                // do not find page
                None => {
                    -1
                },
                Some(pte) => {
                    if pte.user() && pte.readable() {
                        let offset = va.page_offset();
                        let bytes = pte.ppn().get_bytes_array();
                        bytes[offset] as isize
                    } else {
                        // the page is not readable
                        -1
                    }
                },
            }
        }
        1 => {
            let va = VirtAddr::from(id);
            if let Some(pte) = page_table.translate(va.floor()) {
                if pte.user() && pte.writable() {
                    let offset = va.page_offset();
                    let bytes = pte.ppn().get_bytes_array();
                    bytes[offset] = data as u8;
                    0
                } else {
                    -1
                }
            } else {
                -1
            }
        }
        2 => get_syscall_record(id),
        _ => -1,
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    trace!("kernel: sys_mmap");
    let mut map_perm = MapPermission::U;
    if (prot & 0x7) == 0 {
        return -1;
    }
    if (prot & (!0x7)) != 0 {
        return -1;
    }
    if (prot & 0x1) != 0 {
        map_perm |= MapPermission::R;
    }
    if (prot & 0x2) != 0 {
        map_perm |= MapPermission::W;
    }
    if (prot & 0x4) != 0 {
        map_perm |= MapPermission::X;
    }
    mmap(VirtAddr::from(start), VirtAddr::from(start+len), map_perm)
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap");
    munmap(VirtAddr::from(start), VirtAddr::from(start+len))
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
