use anyhow::{Context, Result};
use aya::{
    maps::{MapData, RingBuf},
    programs::TracePoint,
    Ebpf,
};

/// Loads and manages the eBPF programs
pub struct EbpfLoader {
    bpf: Ebpf,
}

impl EbpfLoader {
    /// Load the eBPF programs from embedded bytecode
    pub fn load() -> Result<Self> {
        // Load the eBPF bytecode from the build output
        let bpf_bytes = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../target/ebpf/notify-done-ebpf"
        ))
        .or_else(|_| {
            std::fs::read(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../notify-done-ebpf/target/bpfel-unknown-none/debug/notify-done-ebpf"
            ))
        })
        .or_else(|_| {
            std::fs::read(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../notify-done-ebpf/target/bpfel-unknown-none/release/notify-done-ebpf"
            ))
        })
        .context("Failed to load eBPF bytecode - run `cargo xtask build-ebpf` first")?;

        let bpf = Ebpf::load(&bpf_bytes).context("Failed to load eBPF program")?;

        // Debug: list all programs and maps
        log::info!("Loaded BPF object. Programs:");
        for (name, _prog) in bpf.programs() {
            log::info!("  - program: {}", name);
        }
        log::info!("Maps:");
        for (name, _map) in bpf.maps() {
            log::info!("  - map: {}", name);
        }

        Ok(Self { bpf })
    }

    /// Attach the tracepoints
    pub fn attach(&mut self) -> Result<()> {
        // Attach sched_process_exec tracepoint
        let exec_prog: &mut TracePoint = self
            .bpf
            .program_mut("sched_process_exec")
            .context("Failed to find sched_process_exec program")?
            .try_into()?;
        exec_prog.load()?;
        exec_prog
            .attach("sched", "sched_process_exec")
            .context("Failed to attach sched_process_exec")?;
        log::info!("Attached sched_process_exec tracepoint");

        // Attach sched_process_exit tracepoint
        let exit_prog: &mut TracePoint = self
            .bpf
            .program_mut("sched_process_exit")
            .context("Failed to find sched_process_exit program")?
            .try_into()?;
        exit_prog.load()?;
        exit_prog
            .attach("sched", "sched_process_exit")
            .context("Failed to attach sched_process_exit")?;
        log::info!("Attached sched_process_exit tracepoint");

        Ok(())
    }

    /// Get the events ring buffer for reading
    pub fn events_ring_buf(&mut self) -> Result<RingBuf<MapData>> {
        let ring_buf = self
            .bpf
            .take_map("EVENTS")
            .context("Failed to find EVENTS ring buffer")?
            .try_into()?;
        Ok(ring_buf)
    }
}
