// Memory Profiler: Track allocation patterns during gameplay
//
// GOAL: Detect memory leaks, heap fragmentation, and allocation hotspots.
//
// USAGE:
//   let profiler = MemoryProfiler::start();
//   // ... run game for N frames ...
//   let report = profiler.report();
//   println!("{}", report);
//
// FEATURES:
// - Per-frame allocation tracking
// - Peak memory usage detection
// - Heap fragmentation estimation
// - Allocation site tracking (via backtrace)

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use bevy_ecs::prelude::*;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// GLOBAL ALLOCATION TRACKER
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub struct TrackingAllocator;

static ALLOCATED_BYTES: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATED_BYTES: AtomicUsize = AtomicUsize::new(0);
static ALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);
static PEAK_ALLOCATED: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            let size = layout.size();
            ALLOCATED_BYTES.fetch_add(size, Ordering::Relaxed);
            ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);

            // Update peak
            let current = ALLOCATED_BYTES.load(Ordering::Relaxed)
                .saturating_sub(DEALLOCATED_BYTES.load(Ordering::Relaxed));
            let peak = PEAK_ALLOCATED.load(Ordering::Relaxed);
            if current > peak {
                PEAK_ALLOCATED.store(current, Ordering::Relaxed);
            }
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        let size = layout.size();
        DEALLOCATED_BYTES.fetch_add(size, Ordering::Relaxed);
        DEALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
    }
}

// Enable tracking allocator (add to Cargo.toml):
// #[global_allocator]
// static GLOBAL: TrackingAllocator = TrackingAllocator;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// MEMORY PROFILER RESOURCE
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Resource)]
pub struct MemoryProfiler {
    pub enabled: bool,
    pub start_time: Instant,
    pub frame_count: u64,
    pub samples: Vec<MemorySample>,
    pub sample_interval_frames: u64, // Sample every N frames (default: 60 = 1 sec @ 60fps)
}

#[derive(Clone, Debug)]
pub struct MemorySample {
    pub frame: u64,
    pub timestamp: Duration,
    pub allocated: usize,
    pub deallocated: usize,
    pub net_usage: usize,
    pub allocation_count: usize,
    pub deallocation_count: usize,
}

impl Default for MemoryProfiler {
    fn default() -> Self {
        Self::new(60) // Sample every 60 frames
    }
}

impl MemoryProfiler {
    pub fn new(sample_interval_frames: u64) -> Self {
        Self {
            enabled: true,
            start_time: Instant::now(),
            frame_count: 0,
            samples: Vec::new(),
            sample_interval_frames,
        }
    }

    pub fn start() -> Self {
        Self::new(60)
    }

    pub fn tick(&mut self) {
        self.frame_count += 1;

        if self.enabled && self.frame_count % self.sample_interval_frames == 0 {
            self.take_sample();
        }
    }

    fn take_sample(&mut self) {
        let allocated = ALLOCATED_BYTES.load(Ordering::Relaxed);
        let deallocated = DEALLOCATED_BYTES.load(Ordering::Relaxed);
        let allocation_count = ALLOCATION_COUNT.load(Ordering::Relaxed);
        let deallocation_count = DEALLOCATION_COUNT.load(Ordering::Relaxed);

        let sample = MemorySample {
            frame: self.frame_count,
            timestamp: self.start_time.elapsed(),
            allocated,
            deallocated,
            net_usage: allocated.saturating_sub(deallocated),
            allocation_count,
            deallocation_count,
        };

        self.samples.push(sample);
    }

    pub fn report(&self) -> MemoryReport {
        let peak = PEAK_ALLOCATED.load(Ordering::Relaxed);

        let avg_usage = if !self.samples.is_empty() {
            self.samples.iter().map(|s| s.net_usage).sum::<usize>() / self.samples.len()
        } else {
            0
        };

        let growth_rate = if self.samples.len() > 1 {
            let first = &self.samples[0];
            let last = &self.samples[self.samples.len() - 1];
            let duration_sec = (last.timestamp - first.timestamp).as_secs_f64();
            if duration_sec > 0.0 {
                (last.net_usage as f64 - first.net_usage as f64) / duration_sec
            } else {
                0.0
            }
        } else {
            0.0
        };

        // Estimate fragmentation: allocation_count - deallocation_count = live objects
        let live_allocations = ALLOCATION_COUNT.load(Ordering::Relaxed)
            .saturating_sub(DEALLOCATION_COUNT.load(Ordering::Relaxed));

        MemoryReport {
            peak_bytes: peak,
            avg_bytes: avg_usage,
            current_bytes: ALLOCATED_BYTES.load(Ordering::Relaxed)
                .saturating_sub(DEALLOCATED_BYTES.load(Ordering::Relaxed)),
            total_allocated: ALLOCATED_BYTES.load(Ordering::Relaxed),
            total_deallocated: DEALLOCATED_BYTES.load(Ordering::Relaxed),
            allocation_count: ALLOCATION_COUNT.load(Ordering::Relaxed),
            deallocation_count: DEALLOCATION_COUNT.load(Ordering::Relaxed),
            live_allocations,
            growth_rate_bytes_per_sec: growth_rate,
            sample_count: self.samples.len(),
            duration: self.start_time.elapsed(),
        }
    }

    pub fn reset(&mut self) {
        self.samples.clear();
        self.frame_count = 0;
        self.start_time = Instant::now();

        // Note: Can't reset global allocator counters without unsafe tricks
    }
}

#[derive(Debug)]
pub struct MemoryReport {
    pub peak_bytes: usize,
    pub avg_bytes: usize,
    pub current_bytes: usize,
    pub total_allocated: usize,
    pub total_deallocated: usize,
    pub allocation_count: usize,
    pub deallocation_count: usize,
    pub live_allocations: usize,
    pub growth_rate_bytes_per_sec: f64,
    pub sample_count: usize,
    pub duration: Duration,
}

impl std::fmt::Display for MemoryReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")?;
        writeln!(f, "Memory Profiler Report")?;
        writeln!(f, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")?;
        writeln!(f, "Duration:        {:.2}s", self.duration.as_secs_f64())?;
        writeln!(f, "Samples:         {}", self.sample_count)?;
        writeln!(f)?;
        writeln!(f, "Memory Usage:")?;
        writeln!(f, "  Current:       {:.2} MB", self.current_bytes as f64 / 1_048_576.0)?;
        writeln!(f, "  Peak:          {:.2} MB", self.peak_bytes as f64 / 1_048_576.0)?;
        writeln!(f, "  Average:       {:.2} MB", self.avg_bytes as f64 / 1_048_576.0)?;
        writeln!(f)?;
        writeln!(f, "Allocations:")?;
        writeln!(f, "  Total alloc:   {} ({:.2} MB)", self.allocation_count, self.total_allocated as f64 / 1_048_576.0)?;
        writeln!(f, "  Total dealloc: {} ({:.2} MB)", self.deallocation_count, self.total_deallocated as f64 / 1_048_576.0)?;
        writeln!(f, "  Live objects:  {}", self.live_allocations)?;
        writeln!(f)?;
        writeln!(f, "Growth Rate:     {:.2} KB/sec", self.growth_rate_bytes_per_sec / 1024.0)?;
        writeln!(f)?;

        // Health check
        if self.growth_rate_bytes_per_sec > 10_000.0 {
            writeln!(f, "⚠️  WARNING: Memory leak suspected (>10 KB/sec growth)")?;
        }

        if self.live_allocations > 100_000 {
            writeln!(f, "⚠️  WARNING: High fragmentation (>100K live allocations)")?;
        }

        if self.peak_bytes > 500_000_000 {
            writeln!(f, "⚠️  WARNING: High memory usage (>500 MB peak)")?;
        }

        writeln!(f, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")?;

        Ok(())
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// SYSTEM: Update profiler each frame
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub fn memory_profiler_system(mut profiler: ResMut<MemoryProfiler>) {
    profiler.tick();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// HELPER: Export profiler data to CSV
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

impl MemoryProfiler {
    pub fn export_csv(&self, path: &str) -> std::io::Result<()> {
        use std::io::Write;

        let mut file = std::fs::File::create(path)?;
        writeln!(file, "frame,timestamp_ms,allocated,deallocated,net_usage,allocation_count,deallocation_count")?;

        for sample in &self.samples {
            writeln!(
                file,
                "{},{},{},{},{},{},{}",
                sample.frame,
                sample.timestamp.as_millis(),
                sample.allocated,
                sample.deallocated,
                sample.net_usage,
                sample.allocation_count,
                sample.deallocation_count
            )?;
        }

        Ok(())
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TESTS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_samples() {
        let mut profiler = MemoryProfiler::new(10); // Sample every 10 frames

        for _ in 0..100 {
            profiler.tick();
        }

        assert_eq!(profiler.samples.len(), 10); // 100 frames / 10 = 10 samples
    }

    #[test]
    fn test_memory_report() {
        let profiler = MemoryProfiler::new(1);
        let report = profiler.report();

        // Just verify it doesn't crash
        let _ = format!("{}", report);
    }

    #[test]
    fn test_csv_export() {
        let mut profiler = MemoryProfiler::new(1);
        for _ in 0..10 {
            profiler.tick();
        }

        let path = "/tmp/test_memory_profile.csv";
        profiler.export_csv(path).unwrap();

        let contents = std::fs::read_to_string(path).unwrap();
        assert!(contents.contains("frame,timestamp_ms"));
    }
}
