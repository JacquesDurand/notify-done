#![no_std]
#![no_main]
#![allow(static_mut_refs)]
#![feature(asm_experimental_arch)]

use aya_ebpf::{
    helpers::{bpf_get_current_pid_tgid, bpf_get_current_uid_gid, bpf_ktime_get_ns},
    macros::{map, tracepoint},
    maps::RingBuf,
    programs::TracePointContext,
    EbpfContext,
};
use notify_done_common::{EventType, ProcessExecEvent, ProcessExitEvent, RING_BUF_SIZE};

/// Ring buffer for sending events to userspace
#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(RING_BUF_SIZE, 0);

/// Minimum UID to track (system users are below 1000)
const MIN_UID: u32 = 1000;

/// Tracepoint for sched:sched_process_exec
#[tracepoint(category = "sched", name = "sched_process_exec")]
pub fn sched_process_exec(ctx: TracePointContext) -> u32 {
    match try_sched_process_exec(&ctx) {
        Ok(()) => 0,
        Err(_) => 0,
    }
}

fn try_sched_process_exec(ctx: &TracePointContext) -> Result<(), i64> {
    let uid_gid = bpf_get_current_uid_gid();
    let uid = uid_gid as u32;

    if uid < MIN_UID {
        return Ok(());
    }

    let pid_tgid = bpf_get_current_pid_tgid();
    let tgid = (pid_tgid >> 32) as u32;
    let pid = pid_tgid as u32;
    let timestamp = unsafe { bpf_ktime_get_ns() };

    if let Some(mut entry) = EVENTS.reserve::<ProcessExecEvent>(0) {
        let base = entry.as_mut_ptr() as *mut u8;
        unsafe {
            // ProcessExecEvent layout (offsets calculated from #[repr(C)]):
            // event_type: u8 @ 0
            // _pad: [u8; 3] @ 1
            // pid: u32 @ 4
            // tgid: u32 @ 8
            // ppid: u32 @ 12
            // uid: u32 @ 16
            // timestamp_ns: u64 @ 24 (aligned)
            // comm: [u8; 16] @ 32
            // filename: [u8; 256] @ 48

            // Write scalar fields using byte offsets - no method calls
            *base = EventType::Exec as u8;
            *base.wrapping_add(1) = 0;
            *base.wrapping_add(2) = 0;
            *base.wrapping_add(3) = 0;
            *(base.wrapping_add(4) as *mut u32) = pid;
            *(base.wrapping_add(8) as *mut u32) = tgid;
            *(base.wrapping_add(12) as *mut u32) = 0; // ppid
            *(base.wrapping_add(16) as *mut u32) = uid;
            *(base.wrapping_add(24) as *mut u64) = timestamp;

            // Zero comm (16 bytes at offset 32) as 2 u64s
            *(base.wrapping_add(32) as *mut u64) = 0;
            *(base.wrapping_add(40) as *mut u64) = 0;

            // Copy command name if available
            if let Ok(comm) = ctx.command() {
                let comm_base = base.wrapping_add(32);
                let src = comm.as_ptr();
                *comm_base = *src;
                *comm_base.wrapping_add(1) = *src.wrapping_add(1);
                *comm_base.wrapping_add(2) = *src.wrapping_add(2);
                *comm_base.wrapping_add(3) = *src.wrapping_add(3);
                *comm_base.wrapping_add(4) = *src.wrapping_add(4);
                *comm_base.wrapping_add(5) = *src.wrapping_add(5);
                *comm_base.wrapping_add(6) = *src.wrapping_add(6);
                *comm_base.wrapping_add(7) = *src.wrapping_add(7);
                *comm_base.wrapping_add(8) = *src.wrapping_add(8);
                *comm_base.wrapping_add(9) = *src.wrapping_add(9);
                *comm_base.wrapping_add(10) = *src.wrapping_add(10);
                *comm_base.wrapping_add(11) = *src.wrapping_add(11);
                *comm_base.wrapping_add(12) = *src.wrapping_add(12);
                *comm_base.wrapping_add(13) = *src.wrapping_add(13);
                *comm_base.wrapping_add(14) = *src.wrapping_add(14);
                *comm_base.wrapping_add(15) = *src.wrapping_add(15);
            }

            // Zero filename (256 bytes at offset 48) as 32 u64s
            let fn_base = base.wrapping_add(48) as *mut u64;
            *fn_base = 0;
            *fn_base.wrapping_add(1) = 0;
            *fn_base.wrapping_add(2) = 0;
            *fn_base.wrapping_add(3) = 0;
            *fn_base.wrapping_add(4) = 0;
            *fn_base.wrapping_add(5) = 0;
            *fn_base.wrapping_add(6) = 0;
            *fn_base.wrapping_add(7) = 0;
            *fn_base.wrapping_add(8) = 0;
            *fn_base.wrapping_add(9) = 0;
            *fn_base.wrapping_add(10) = 0;
            *fn_base.wrapping_add(11) = 0;
            *fn_base.wrapping_add(12) = 0;
            *fn_base.wrapping_add(13) = 0;
            *fn_base.wrapping_add(14) = 0;
            *fn_base.wrapping_add(15) = 0;
            *fn_base.wrapping_add(16) = 0;
            *fn_base.wrapping_add(17) = 0;
            *fn_base.wrapping_add(18) = 0;
            *fn_base.wrapping_add(19) = 0;
            *fn_base.wrapping_add(20) = 0;
            *fn_base.wrapping_add(21) = 0;
            *fn_base.wrapping_add(22) = 0;
            *fn_base.wrapping_add(23) = 0;
            *fn_base.wrapping_add(24) = 0;
            *fn_base.wrapping_add(25) = 0;
            *fn_base.wrapping_add(26) = 0;
            *fn_base.wrapping_add(27) = 0;
            *fn_base.wrapping_add(28) = 0;
            *fn_base.wrapping_add(29) = 0;
            *fn_base.wrapping_add(30) = 0;
            *fn_base.wrapping_add(31) = 0;
        }
        entry.submit(0);
    }

    Ok(())
}

/// Tracepoint for sched:sched_process_exit
#[tracepoint(category = "sched", name = "sched_process_exit")]
pub fn sched_process_exit(ctx: TracePointContext) -> u32 {
    match try_sched_process_exit(&ctx) {
        Ok(()) => 0,
        Err(_) => 0,
    }
}

fn try_sched_process_exit(ctx: &TracePointContext) -> Result<(), i64> {
    let uid_gid = bpf_get_current_uid_gid();
    let uid = uid_gid as u32;

    if uid < MIN_UID {
        return Ok(());
    }

    let pid_tgid = bpf_get_current_pid_tgid();
    let tgid = (pid_tgid >> 32) as u32;
    let pid = pid_tgid as u32;
    let timestamp = unsafe { bpf_ktime_get_ns() };

    if let Some(mut entry) = EVENTS.reserve::<ProcessExitEvent>(0) {
        let base = entry.as_mut_ptr() as *mut u8;
        unsafe {
            // ProcessExitEvent layout (offsets calculated from #[repr(C)]):
            // event_type: u8 @ 0
            // _pad: [u8; 3] @ 1
            // pid: u32 @ 4
            // tgid: u32 @ 8
            // uid: u32 @ 12
            // exit_code: i32 @ 16
            // timestamp_ns: u64 @ 24 (aligned)
            // comm: [u8; 16] @ 32

            // Write scalar fields using byte offsets - no method calls
            *base = EventType::Exit as u8;
            *base.wrapping_add(1) = 0;
            *base.wrapping_add(2) = 0;
            *base.wrapping_add(3) = 0;
            *(base.wrapping_add(4) as *mut u32) = pid;
            *(base.wrapping_add(8) as *mut u32) = tgid;
            *(base.wrapping_add(12) as *mut u32) = uid;
            *(base.wrapping_add(16) as *mut i32) = 0; // exit_code
            *(base.wrapping_add(24) as *mut u64) = timestamp;

            // Zero comm (16 bytes at offset 32) as 2 u64s
            *(base.wrapping_add(32) as *mut u64) = 0;
            *(base.wrapping_add(40) as *mut u64) = 0;

            // Copy command name if available
            if let Ok(comm) = ctx.command() {
                let comm_base = base.wrapping_add(32);
                let src = comm.as_ptr();
                *comm_base = *src;
                *comm_base.wrapping_add(1) = *src.wrapping_add(1);
                *comm_base.wrapping_add(2) = *src.wrapping_add(2);
                *comm_base.wrapping_add(3) = *src.wrapping_add(3);
                *comm_base.wrapping_add(4) = *src.wrapping_add(4);
                *comm_base.wrapping_add(5) = *src.wrapping_add(5);
                *comm_base.wrapping_add(6) = *src.wrapping_add(6);
                *comm_base.wrapping_add(7) = *src.wrapping_add(7);
                *comm_base.wrapping_add(8) = *src.wrapping_add(8);
                *comm_base.wrapping_add(9) = *src.wrapping_add(9);
                *comm_base.wrapping_add(10) = *src.wrapping_add(10);
                *comm_base.wrapping_add(11) = *src.wrapping_add(11);
                *comm_base.wrapping_add(12) = *src.wrapping_add(12);
                *comm_base.wrapping_add(13) = *src.wrapping_add(13);
                *comm_base.wrapping_add(14) = *src.wrapping_add(14);
                *comm_base.wrapping_add(15) = *src.wrapping_add(15);
            }
        }
        entry.submit(0);
    }

    Ok(())
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe {
        core::arch::asm!(
            "r0 = 0",
            "exit",
            options(noreturn)
        )
    }
}
