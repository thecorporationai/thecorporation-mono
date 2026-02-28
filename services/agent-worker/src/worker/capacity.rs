//! Auto-detect max concurrency from host resources.

/// Detect max concurrent containers based on CPU and memory.
/// Reserves 20% of memory for the worker process itself.
pub fn detect() -> usize {
    let cpus = num_cpus();
    let memory_mb = total_memory_mb();

    // Default container: 512MB memory, 0.5 CPU
    let by_cpu = (cpus as f64 / 0.5).floor() as usize;
    let by_memory = ((memory_mb as f64 * 0.8) / 512.0).floor() as usize;

    let result = by_cpu.min(by_memory).min(32).max(1);
    tracing::info!(cpus, memory_mb, by_cpu, by_memory, result, "auto-detected concurrency");
    result
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

fn total_memory_mb() -> u64 {
    // Read from /proc/meminfo on Linux
    if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(kb) = parts.get(1).and_then(|s| s.parse::<u64>().ok()) {
                    return kb / 1024;
                }
            }
        }
    }
    // Fallback: assume 4GB
    4096
}
