//! Process management syscalls
use alloc::sync::Arc;

use crate::{
    loader::get_app_data_by_name,
    mm::{
        translated_byte_buffer, translated_refmut, translated_str, MapPermission, PageTable,
        VirtAddr,
    },
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next,
    },
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!(
        "kernel::pid[{}] sys_waitpid [{}]",
        current_task().unwrap().pid.0,
        pid
    );
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// YOUR JOB: get time with second and microsecond (Completed)
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel:pid[{}] sys_get_time", current_task().unwrap().pid.0);
    let time_us = get_time_us();
    let time_val = TimeVal {
        sec: time_us / 1_000_000,
        usec: time_us % 1_000_000,
    };
    let time_val_bytes = unsafe {
        core::slice::from_raw_parts(
            &time_val as *const _ as *const u8,
            core::mem::size_of::<TimeVal>(),
        )
    };
    let ts_byte_buffers = translated_byte_buffer(
        current_user_token(),
        ts as *const u8,
        core::mem::size_of::<TimeVal>(),
    );
    let mut bytes_written = 0;
    for ts_byte_buf in ts_byte_buffers {
        let ts_byte_buf_len = ts_byte_buf.len();
        ts_byte_buf.copy_from_slice(&time_val_bytes[bytes_written..ts_byte_buf_len]);
        bytes_written += ts_byte_buf_len;
    }
    if bytes_written == core::mem::size_of::<TimeVal>() {
        0
    } else {
        -1
    }
}

/// YOUR JOB: Implement mmap. (Completed)
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_mmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );

    // Check port validity, invalid if low 3 bits are 0 or any other bits are set
    if (port & 0x7) == 0 || (port & !0x7) != 0 {
        return -1;
    }

    // Convert usize to va and check if page-aligned
    let start_va = VirtAddr::from(start);
    if start_va.page_offset() != 0 {
        return -1;
    }
    let end_va = VirtAddr::from(start + len);

    // Convert va to vpn and check for page conflicts
    let page_table = PageTable::from_token(current_user_token());
    let mut vpn_idx = start_va.floor();
    let vpn_end = end_va.ceil();
    while vpn_idx < vpn_end {
        if let Some(pte) = page_table.translate(vpn_idx) {
            if pte.is_valid() {
                return -1;
            }
        }
        vpn_idx.0 += 1;
    }

    // Construct MapPermission
    let mut map_perm: MapPermission = MapPermission::U;
    if port & 1 != 0 {
        map_perm |= MapPermission::R;
    }
    if port & 2 != 0 {
        map_perm |= MapPermission::W;
    }
    if port & 4 != 0 {
        map_perm |= MapPermission::X;
    }

    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    inner
        .memory_set
        .insert_framed_area(start_va, end_va, map_perm);
    drop(inner);
    0
}

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_munmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );

    // Convert usize to va and check if page-aligned
    let start_va: VirtAddr = VirtAddr::from(start);
    if start_va.page_offset() != 0 {
        // Not page-aligned
        return -1;
    }
    let end_va: VirtAddr = VirtAddr::from(start + len);

    // Convert va to vpn and check if pages exist
    let start_vpn = start_va.floor();
    let end_vpn = end_va.ceil();
    let mut idx_vpn = start_vpn;
    let pagetable = PageTable::from_token(current_user_token());
    while idx_vpn < end_vpn {
        if pagetable.translate(idx_vpn).is_none() {
            // vpn does not exist
            return -1;
        }
        idx_vpn.0 += 1;
    }

    // Get current task and free mapareas
    let current_task = current_task().unwrap();
    let mut inner = current_task.inner_exclusive_access();
    let res = inner.memory_set.unmap_area(start_vpn, end_vpn);
    drop(inner);
    res
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        let new_task = task.spawn(data);
        let new_pid = new_task.pid.0;
        add_task(new_task);
        new_pid as isize
    } else {
        -1
    }
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    if prio >= 2 {
        current_task().unwrap().set_priority(prio);
        prio
    } else {
        -1
    }
}
