# 14 — Performance Engineering & Real-Time Optimization Plan

> **Specification & Technical Architecture Document**  
> **Domain:** Engine Core, Render Graph, Decode Pipeline, Audio Subsystem, MCP Server  
> **Target Crates:** `slate-core`, `slate-media`, `slate-render`, `slate-audio`, `slate-mcp`  
> **Status:** APPROVED FOR IMPLEMENTATION (Every target ships in v1)  

---

## Executive Summary

Non-linear video editors operate under ruthless real-time constraints. At 1080p30, the system has exactly **33.33 milliseconds** to decode multiple overlapping video streams, upload pixel buffers to the GPU, calculate geometric affine transforms, apply parametric WGSL shaders, composite layers via Porter-Duff alpha blending, rasterize vector text, and present the frame to the display swapchain—all while maintaining an uninterrupted, zero-jitter 48 kHz audio output callback and serving asynchronous Model Context Protocol (MCP) tool requests from external AI agents.

This document establishes the authoritative performance engineering specification for **Slate**. It establishes strict, non-negotiable performance targets for v1, maps out the complete multi-threaded concurrency hierarchy, breaks down the microsecond-level frame pipeline budget, details lock-free and zero-allocation memory pools, specifies platform-native zero-copy GPU memory bridges, outlines multi-threaded hardware decode and export pipelines, and specifies deterministic validation suites to guarantee bitwise and perceptual rendering parity.

```
   ┌────────────────────────────────────────────────────────────────────────┐
   │                          Slate Concurrency                            │
   │                                                                        │
   │   UI Thread          Render Thread        Audio Thread      IO Pool   │
   │  ┌──────────┐        ┌──────────┐         ┌───────────┐   ┌──────────┐ │
   │  │  egui    │        │   wgpu   │         │   cpal    │   │  tokio   │ │
   │  │  60 FPS  │◄───────┤Compositor│         │ Real-time │   │  Async   │ │
   │  └────┬─────┘        └────▲─────┘         └─────▲─────┘   └────┬─────┘ │
   │       │                   │                     │              │       │
   │       │ broadcast         │ bounded channel     │ ringbuf      │ IPC   │
   │       ▼                   │                     │              ▼       │
   │  ┌──────────┐        ┌────┴─────┐         ┌─────┴─────┐   ┌──────────┐ │
   │  │  Engine  │───────►│  Decode  │         │   Audio   │   │   MCP    │ │
   │  │  State   │        │ Workers  │         │  Renderer │   │  Server  │ │
   │  └──────────┘        └──────────┘         └───────────┘   └──────────┘ │
   └────────────────────────────────────────────────────────────────────────┘
```

---

## 1. Performance Targets & Reference Baselines

All performance benchmarks and limits are strictly measured and validated against two reference hardware baselines representing standard consumer laptops and mid-tier desktop workstations. **Every target listed below is a hard release blocker for v1.**

### 1.1 Reference Hardware Baselines

| Specification | Mac Reference Machine | PC Reference Machine |
| :--- | :--- | :--- |
| **Processor (CPU)** | Apple M2 (8-core: 4 performance + 4 efficiency) | Intel Core i7-12700 (12 cores / 20 threads) |
| **Graphics (GPU)** | Apple M2 Unified GPU (10-core, 100 GB/s bandwidth) | NVIDIA GeForce RTX 3060 (12 GB GDDR6, PCIe 4.0) |
| **System Memory (RAM)** | 16 GB Unified Memory | 32 GB DDR4-3200 MHz |
| **Storage (I/O)** | 512 GB Apple NVMe SSD (~3,000 MB/s sequential read) | 1 TB PCIe 4.0 NVMe SSD (~5,000 MB/s sequential read) |
| **Operating System** | macOS 14 Sonoma / macOS 15 Sequoia | Windows 11 Pro (23H2) / Ubuntu 24.04 LTS |
| **Display Refresh** | 60 Hz ProMotion Retina Display | 60 Hz / 144 Hz sRGB Monitor |

### 1.2 Performance Metric Verification Matrix

| Area | Target Metric | Measurement Methodology | Failure Threshold |
| :--- | :--- | :--- | :--- |
| **Playback 1080p30** | 4 video tracks + crossfade, real-time, **0 dropped frames** | Hardware dropped frame counter over 60s continuous playback loop | > 0 dropped frames |
| **Playback 4K** | Real-time continuous playback using 720p/1080p proxies | Proxy decode pipeline frame interval timestamping | Frame delta > 33.33ms |
| **Scrub Latency (Cached)** | **< 100 ms** cursor movement to displayed frame | High-resolution timestamp logging from input event to swapchain present | p95 > 100 ms |
| **Scrub Latency (Cold)** | **< 250 ms** across non-cached GOP boundaries | Input dispatch to keyframe seek, forward decode, and presentation | p95 > 250 ms |
| **Edit Latency** | Any single op **< 16 ms** on a 500-clip timeline | Microbenchmark measuring `op.apply(&mut timeline)` wall-clock | Op time > 16 ms |
| **Export 1080p H.264** | **≥ 3× real-time** with hardware encode | Total render & mux wall-clock vs timeline duration ($T_{\text{export}} \le T_{\text{timeline}} / 3$) | Speed < 3.0× |
| **Preview/Export Parity** | **PSNR ≥ 45 dB** (SSIM ≥ 0.999) | Perceptual image quality diff against CPU/GPU golden frame suite | PSNR < 45 dB |
| **Cold Start (Empty)** | **< 1.5 s** from process launch to interactive UI | Benchmark wall-clock timing until initial egui frame finishes | Startup > 1.5 s |
| **Project Reopen (1hr)** | **< 3 s** for 1-hour cached timeline (500+ assets) | Wall-clock time to deserialize bundle, verify media, and draw UI | Reopen > 3.0 s |
| **MCP `timeline_summary`** | **< 2 KB** payload size for a 100-clip project | Byte count of serialized JSON response returned to agent | Payload ≥ 2048 bytes |
| **MCP Non-Render Tool** | **p95 < 200 ms** latency (mutations, queries, slice) | Prometheus / tracing histogram over 1,000 synthetic operations | p95 > 200 ms |
| **MCP `render_frame`** | **p95 < 500 ms** for offscreen readback & encode | High-precision timer around offscreen GPU render and PNG/JPEG encode | p95 > 500 ms |
| **Autosave** | **< 100 ms** for journal append & WAL flush | Disk write & sync benchmark under sustained multi-track editing | Latency > 100 ms |
| **Memory (1080p, 4 tracks)** | **< 2 GB** resident set size (RSS) | OS memory profiler (`mach_task_basic_info` / `procfs`) during playback | RSS ≥ 2.0 GB |
| **Memory (4K, 4 tracks)** | **< 4 GB** resident set size (RSS) using proxies | OS memory profiler during sustained scrub and playback | RSS ≥ 4.0 GB |

---

## 2. Concurrency Architecture & Threading Model

Slate strictly forbids unstructured thread spawning and thread contention on hot paths. To guarantee predictable real-time scheduling and prevent priority inversion, work is compartmentalized into dedicated threads and asynchronous task pools with deterministic lock-free communication channels.

### 2.1 Thread Topology

```
Main Thread (UI)
├── egui immediate-mode event loop
├── Platform window & input handling (winit)
└── UI rendering (egui pass + wgpu surface presentation)

Render Thread
├── Compositor state machine (wgpu RenderGraph)
├── GPU command buffer encoding & submission
└── Presentation to swapchain (onscreen) OR staging readback (export/MCP)

Decode Worker Pool (N = physical CPU cores - 2, min 4)
├── Isolated FFmpeg AVFormatContext & AVCodecContext instances
├── LRU frame cache ingestion & eviction
└── Predictive pre-fetch scheduling (forward/reverse)

Audio Thread (OS Real-Time Priority / SCHED_FIFO)
├── cpal audio stream callback
├── Lock-free ring buffer consumer (SPSC)
└── Master timeline clock updates (sample-exact monotonic clock)

Audio Render Thread (High Priority)
├── Multi-track audio decode & rubato sinc resampling
├── Software mixing, volume automation, and EBU R128 loudness metering
└── Lock-free ring buffer producer (SPSC)

Tokio Runtime (Async I/O Multi-Thread Pool)
├── JSON-RPC 2.0 Engine Server (jsonrpsee Unix socket / named pipe)
├── Model Context Protocol Server (rmcp stdio/SSE)
├── Background utility pipelines (proxy generation, waveform peaks, Whisper STT)
├── Write-Ahead Log (WAL) autosave journal timer
└── Asynchronous disk I/O & asset hashing
```

### 2.2 Thread Communication Topologies

```mermaid
flowchart TD
    subgraph UI_Domain ["UI Domain (Main Thread)"]
        UI["egui UI Event Loop"]
    end

    subgraph Core_Domain ["Core & Async Domain"]
        TOKIO["Tokio Async Runtime<br/>(IPC & MCP Server)"]
        WAL["Autosave WAL Journal"]
        BG["Background Jobs<br/>(Proxy / Waveform)"]
    end

    subgraph Decode_Domain ["Media Decode Pool"]
        DEC0["Decode Worker 0"]
        DEC1["Decode Worker 1"]
        DECN["Decode Worker N"]
    end

    subgraph Render_Domain ["Render Domain (Render Thread)"]
        COMP["wgpu Compositor<br/>RenderGraph Engine"]
    end

    subgraph Audio_Domain ["Audio Domain"]
        AMIX["Audio Render Thread<br/>(rubato Mixer)"]
        ACALL["Audio Thread (cpal)<br/>[Real-Time Priority]"]
    end

    %% Communication channels
    TOKIO -->|Operation Dispatched| UI
    UI -->|Broadcast Event| TOKIO
    UI -->|Autosave Trigger| WAL
    TOKIO -->|Spawn Task| BG

    UI -->|Seek / Playhead Command| DEC0
    UI -->|Seek / Playhead Command| DEC1
    UI -->|Seek / Playhead Command| DECN

    DEC0 -->|crossbeam bounded channel| COMP
    DEC1 -->|crossbeam bounded channel| COMP
    DECN -->|crossbeam bounded channel| COMP

    COMP -->|Present Surface| UI
    
    UI -->|Timebase Sync| AMIX
    AMIX -->|SPSC lock-free ringbuf| ACALL
    ACALL -->|Master Clock Feedback| UI
    ACALL -->|Clock Drift Signal| COMP
```

### 2.3 Detailed Inter-Thread Communication Primitives

#### 1. Decode Pool to Render Thread (`crossbeam_channel::bounded`)
- **Queue Structure**: `crossbeam_channel::bounded::<DecodedVideoFrame>(4)` per active video track.
- **Backpressure Behavior**: If the render thread stalls or encounters GPU bottleneck, the decode worker blocks on channel send, automatically throttling demuxing and decoding without unconstrained memory growth.
- **Zero-Allocation**: `DecodedVideoFrame` contains an `Arc<FrameBuffer>` or an imported zero-copy GPU handle (`GpuSurfaceHandle`), allowing frame data to cross thread boundaries via atomic reference increment.

#### 2. Audio Render Thread to Audio Output Callback (`ringbuf` SPSC)
- **Ring Buffer Crate**: `ringbuf::HeapRb::<f32>`.
- **Producer**: Audio Render Thread mixes timeline audio tracks, applies volume envelopes and audio plugins, and pushes interleaved 32-bit float samples into the producer half.
- **Consumer**: The `cpal` audio output callback on the OS real-time thread consumes samples directly into the hardware output buffer.
- **Real-Time Safety Invariant**: The audio callback contains **zero allocations, zero mutex acquisitions, zero system calls, and zero filesystem operations**. If the buffer underflows, it emits silence zeros and increments a hardware underflow diagnostic counter.

#### 3. Engine Core to UI Presentation (`tokio::sync::broadcast`)
- **Broadcast Channel**: `tokio::sync::broadcast::channel::<TimelineEvent>(256)`.
- **Event Notification**: Timeline structural changes (`ClipMoved`, `TrackAdded`, `MarkerUpdated`) are emitted to all subscribed UI controllers and MCP notification listeners.
- **Lag Handling**: Slow UI listeners receive `RecvError::Lagged`, triggering a full viewport dirty-state reconciliation rather than crashing or stalling the engine.

#### 4. MCP Agent to Engine Core (IPC vs Headless In-Process)
- **Attached Mode (Desktop App Running)**: The MCP server communicates with the engine via `slate-ipc` over a local UNIX domain socket (macOS/Linux: `/tmp/slate-<pid>.sock`) or named pipe (Windows: `\\.\pipe\slate-<pid>`) using `jsonrpsee`.
- **Headless Mode (CLI / Batch / Autonomous Agent)**: The MCP server runs within the same process address space, invoking engine operations via direct Rust trait method dispatch (`EngineHandle::dispatch(&op)`), bypassing serialization and IPC round-trips entirely.

### 2.4 OS Thread Priority Elevation & Affinity

To guarantee real-time audio and smooth GPU pacing without hitching from background OS tasks, Slate configures thread scheduling classes at startup:

```rust
// crates/slate-audio/src/priority.rs
use tracing::{info, warn};

pub fn set_audio_thread_realtime() {
    #[cfg(target_os = "macos")]
    unsafe {
        use mach::mach_time::*;
        use mach::thread_policy::*;
        use mach::traps::mach_thread_self;
        
        let thread_port = mach_thread_self();
        let mut timebase_info = mach_timebase_info::default();
        mach_timebase_info(&mut timebase_info);
        
        // 128 samples at 48000 Hz = 2.66ms period
        let period_ns = (128 * 1_000_000_000) / 48_000;
        let computation_ns = (period_ns * 30) / 100; // 30% computation reservation
        let constraint_ns = (period_ns * 70) / 100;  // 70% deadline constraint
        
        let factor = (timebase_info.den as f64 / timebase_info.num as f64) * 1e-9;
        let mut policy = thread_time_constraint_policy {
            period: (period_ns as f64 * factor) as u32,
            computation: (computation_ns as f64 * factor) as u32,
            constraint: (constraint_ns as f64 * factor) as u32,
            preemptible: 1,
        };
        
        let ret = thread_policy_set(
            thread_port,
            THREAD_TIME_CONSTRAINT_POLICY,
            &mut policy as *mut _ as *mut i32,
            THREAD_TIME_CONSTRAINT_POLICY_COUNT,
        );
        if ret != 0 {
            warn!("Failed setting macOS Mach real-time thread constraint policy: {}", ret);
        } else {
            info!("macOS Mach real-time audio thread priority activated");
        }
    }

    #[cfg(target_os = "linux")]
    unsafe {
        let param = libc::sched_param { sched_priority: 90 };
        let ret = libc::pthread_setschedparam(libc::pthread_self(), libc::SCHED_FIFO, &param);
        if ret != 0 {
            warn!("Failed setting Linux SCHED_FIFO on audio thread (requires CAP_SYS_NICE or limits.conf)");
        } else {
            info!("Linux SCHED_FIFO priority 90 established for audio callback");
        }
    }

    #[cfg(target_os = "windows")]
    unsafe {
        use windows::Win32::System::Threading::*;
        let handle = GetCurrentThread();
        let _ = SetThreadPriority(handle, THREAD_PRIORITY_TIME_CRITICAL);
        info!("Windows THREAD_PRIORITY_TIME_CRITICAL set on audio callback");
    }
}
```

---

## 3. Frame Pipeline & Microsecond Timing Budget

At 1080p30 (30 frames per second), each frame has a strict wall-clock window of **33.33 milliseconds**. For a 4-track timeline with cross-dissolve transitions, color grading, and titles, Slate allocates and enforces a precise microsecond timing budget.

### 3.1 1080p30 Timing Budget Breakdown

```
Budget: 33.33ms (33,333 µs)
Actual Target: ~11.50ms (11,500 µs)
Headroom: 21.83ms (65.5% idle/safety margin for 60fps & 4K proxy scaling)

[0.0ms] ─── Decode (~5.0ms) ────────► [5.0ms]
[5.0ms] ─── Upload (~1.0ms) ────────► [6.0ms]
[6.0ms] ─── Transform (~0.5ms) ─────► [6.5ms]
[6.5ms] ─── Effects (~2.0ms) ───────► [8.5ms]
[8.5ms] ─── Composite (~1.0ms) ─────► [9.5ms]
[9.5ms] ─── Text/Titles (~0.5ms) ───► [10.0ms]
[10.0ms] ── Present (~0.5ms) ───────► [10.5ms]
[10.5ms] ── Overhead (~1.0ms) ──────► [11.5ms] (Frame Complete)
```

| Pipeline Stage | Sub-Operations & GPU Passes | Microsecond Budget | Percent of Frame |
| :--- | :--- | :--- | :--- |
| **1. Decode** | GOP cache lookup; multi-threaded FFmpeg packet parse; hardware frame decode across 4 tracks | **5,000 µs** | 15.0% |
| **2. Upload** | Zero-copy IOSurface/DMA-BUF descriptor map OR CPU staging buffer DMA transfer (`write_texture`) | **1,000 µs** | 3.0% |
| **3. Transform** | Vertex shader affine transform matrix calculation (crop, anchor, rotation, scale, position) | **500 µs** | 1.5% |
| **4. Effects** | WGSL fragment passes: Rec.709 color grade, exposure/contrast, two-pass Gaussian blur | **2,000 µs** | 6.0% |
| **5. Composite** | Multi-layer Porter-Duff source-over blending with cross-dissolve transition interpolation | **1,000 µs** | 3.0% |
| **6. Text/Titles** | Vector font glyph rasterization via `glyphon` / `cosmic-text` and subtitle overlay blending | **500 µs** | 1.5% |
| **7. Present** | `wgpu::SurfaceTexture::present()` or offscreen readback staging copy submission | **500 µs** | 1.5% |
| **8. Overhead** | Event queue dispatch, tracing instrumentation, lock-free channel sync, timeline clock latch | **1,000 µs** | 3.0% |
| **Total Wall-Clock** | **Complete end-to-end frame composition** | **11,500 µs** | **34.5%** |

### 3.2 1080p60 & 4K Proxy Budgets

- **1080p60 Budget (16.67 ms)**: The 11.50 ms engine frame time fits comfortably within the 16.67 ms window with **5.17 ms (31%) safety headroom**.
- **4K Editing Strategy**: Direct 4K playback of 4 concurrent H.264/HEVC long-GOP streams would exceed the 5 ms decode budget. Slate enforces an automatic **Proxy Pipeline**:
  - Background proxy generator creates ProRes 422 Proxy or 720p H.264 intra-frame lightweight copies.
  - Proxy playback decode duration: **~2.8 ms** for 4 tracks.
  - Compositor renders at proxy viewport scale (1280x720) or full project resolution (3840x2160) using proxy textures upscaled via high-performance bicubic filtering shaders.

### 3.3 Temporal Frame Pipelining (Double/Triple Pipelining)

To prevent the CPU from idling while the GPU renders, Slate implements a 3-stage temporal execution pipeline:

```mermaid
sequenceDiagram
    participant D as Decode Workers
    participant R as Render Thread (CPU)
    participant G as GPU (wgpu Queue)
    participant P as Display Surface

    Note over D,P: Frame Time Interval N (0ms - 16.6ms)
    D->>D: Decode Frame N+1 (Tracks 1-4)
    R->>G: Encode & Submit Passes for Frame N
    G->>G: Execute Shaders & Composite Frame N
    G->>P: Swapchain Present Frame N-1

    Note over D,P: Frame Time Interval N+1 (16.6ms - 33.3ms)
    D->>D: Decode Frame N+2 (Tracks 1-4)
    R->>G: Encode & Submit Passes for Frame N+1
    G->>G: Execute Shaders & Composite Frame N+1
    G->>P: Swapchain Present Frame N
```

### 3.4 Clock Synchronization & Drift Compensation

Playback pacing is strictly anchored to the **Audio Monotonic Clock**:
1. The hardware audio callback updates an atomic sample counter (`AtomicU64`) on every buffer consumption:
   $$\text{MasterTime} = \frac{\text{ConsumedSamples}}{\text{SampleRate}}$$
2. Before the Render Thread prepares a frame, it reads $\text{MasterTime}$.
3. If video rendering leads the audio clock by $> 1.0$ frame, the render thread yields.
4. If video rendering lags behind audio by $> 1.5$ frames due to transient system load, the engine skips GPU composition of the intermediate frame and signals the decode pool to decode directly to the current PTS, guaranteeing audio never stutters.

---

## 4. Memory Management & Allocation Architecture

Slate enforces strict zero-allocation steady-state rendering. Frame buffers, GPU textures, audio blocks, and intermediate compute surfaces are pre-allocated at startup or pooled into typed reusable rings.

### 4.1 Frame Cache (`slate-media`)

The frame cache stores uncompressed decoded video frames in host memory or pinned GPU memory to deliver sub-100ms scrub responses without hitting FFmpeg demuxers.

```rust
// crates/slate-media/src/cache.rs
use std::sync::Arc;
use std::hash::Hash;
use lru::LruCache;
use parking_lot::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResolutionTier {
    Full,     // Native media resolution
    Half,     // 50% scale
    Quarter,  // 25% scale
    Proxy,    // 720p intra-frame proxy
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameCacheKey {
    pub asset_id: Uuid,
    pub frame_index: u64,
    pub tier: ResolutionTier,
}

pub struct CachedFrame {
    pub key: FrameCacheKey,
    pub byte_size: usize,
    pub width: u32,
    pub height: u32,
    pub pts: slate_core::time::RationalTime,
    pub data: Arc<[u8]>,
}

pub struct FrameCache {
    inner: Mutex<LruCache<FrameCacheKey, CachedFrame>>,
    current_bytes: Mutex<usize>,
    max_bytes: usize,
}

impl FrameCache {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            inner: Mutex::new(LruCache::unbounded()),
            current_bytes: Mutex::new(0),
            max_bytes, // Default: 512 MB (536,870,912 bytes)
        }
    }

    pub fn get(&self, key: &FrameCacheKey) -> Option<CachedFrame> {
        let mut cache = self.inner.lock();
        cache.get(key).cloned()
    }

    pub fn insert(&self, frame: CachedFrame) {
        let mut cache = self.inner.lock();
        let mut bytes = self.current_bytes.lock();

        let frame_size = frame.byte_size;
        
        // Evict LRU frames until within budget
        while *bytes + frame_size > self.max_bytes && !cache.is_empty() {
            if let Some((_, evicted)) = cache.pop_lru() {
                *bytes = bytes.saturating_sub(evicted.byte_size);
            }
        }

        *bytes += frame_size;
        if let Some(old) = cache.put(frame.key, frame) {
            *bytes = bytes.saturating_sub(old.byte_size);
        }
    }

    pub fn invalidate_asset(&self, asset_id: Uuid) {
        let mut cache = self.inner.lock();
        let mut bytes = self.current_bytes.lock();
        
        // Collect keys to evict
        let keys_to_remove: Vec<FrameCacheKey> = cache
            .iter()
            .filter(|(k, _)| k.asset_id == asset_id)
            .map(|(k, _)| *k)
            .collect();

        for key in keys_to_remove {
            if let Some(evicted) = cache.pop(&key) {
                *bytes = bytes.saturating_sub(evicted.byte_size);
            }
        }
    }
}
```

#### Directional Adaptive Pre-fetching
The `PrefetchEngine` monitors user scrub velocity and playback rates:
- **1x Normal Playback**: Decodes ahead **12 frames** into the future.
- **2x Fast-Forward**: Decodes ahead **24 frames**, skipping intermediate B-frames if bandwidth constrains.
- **-1x Reverse Playback**: Prefetches backwards by seeking to the prior GOP boundary and populating the cache in forward bursts.
- **High-Velocity Scrubbing**: Drops prefetch depth to **2 frames**, prioritizing immediate presentation of keyframes.

### 4.2 GPU Texture Pool (`slate-render`)

Allocating `wgpu::Texture` objects per frame triggers Vulkan/Metal heap allocations and driver stalls. `slate-render` recycles textures using a descriptor-hashed pool:

```rust
// crates/slate-render/src/texture_pool.rs
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::Mutex;
use wgpu::{Device, Extent3d, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PooledTextureKey {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub usage: TextureUsages,
}

pub struct PooledTexture {
    pub texture: Texture,
    pub key: PooledTextureKey,
    last_used_frame: u64,
}

pub struct TexturePool {
    device: Arc<Device>,
    available: Mutex<HashMap<PooledTextureKey, Vec<PooledTexture>>>,
    active_frame: Mutex<u64>,
}

impl TexturePool {
    pub fn new(device: Arc<Device>) -> Self {
        Self {
            device,
            available: Mutex::new(HashMap::new()),
            active_frame: Mutex::new(0),
        }
    }

    pub fn acquire(
        &self,
        width: u32,
        height: u32,
        format: TextureFormat,
        usage: TextureUsages,
    ) -> Texture {
        let key = PooledTextureKey { width, height, format, usage };
        let mut available = self.available.lock();

        if let Some(vec) = available.get_mut(&key) {
            if let Some(pooled) = vec.pop() {
                return pooled.texture;
            }
        }

        // Cache miss: Allocate on GPU device
        self.device.create_texture(&TextureDescriptor {
            label: Some("PooledTexture"),
            size: Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    }

    pub fn release(
        &self,
        texture: Texture,
        width: u32,
        height: u32,
        format: TextureFormat,
        usage: TextureUsages,
    ) {
        let key = PooledTextureKey { width, height, format, usage };
        let frame = *self.active_frame.lock();
        let mut available = self.available.lock();

        available.entry(key.clone()).or_default().push(PooledTexture {
            texture,
            key,
            last_used_frame: frame,
        });
    }

    pub fn advance_frame(&self) {
        let mut frame = self.active_frame.lock();
        *frame += 1;
        
        // Evict textures unused for more than 120 frames (4 seconds at 30fps)
        if *frame % 60 == 0 {
            let mut available = self.available.lock();
            for pool in available.values_mut() {
                pool.retain(|item| *frame - item.last_used_frame < 120);
            }
        }
    }
}
```

### 4.3 Zero-Allocation Audio Buffering (`slate-audio`)

```rust
// crates/slate-audio/src/ring_buffer.rs
use ringbuf::{
    traits::{Consumer, Producer, Split},
    HeapRb,
};

pub struct AudioSubsystemRing {
    producer: ringbuf::Producer<f32, std::sync::Arc<HeapRb<f32>>>,
    consumer: ringbuf::Consumer<f32, std::sync::Arc<HeapRb<f32>>>,
}

impl AudioSubsystemRing {
    pub fn new(capacity_samples: usize) -> (Self, AudioCallbackConsumer) {
        // e.g., 48,000 samples/sec * 0.25 sec = 12,000 samples buffer
        let rb = HeapRb::<f32>::new(capacity_samples);
        let (producer, consumer) = rb.split();

        (
            Self { producer, consumer },
            AudioCallbackConsumer { consumer }
        )
    }
}

pub struct AudioCallbackConsumer {
    consumer: ringbuf::Consumer<f32, std::sync::Arc<HeapRb<f32>>>,
}

impl AudioCallbackConsumer {
    /// Strictly lock-free, zero-allocation callback invoked by cpal
    #[inline(always)]
    pub fn fill_output_buffer(&mut self, output: &mut [f32]) {
        let read = self.consumer.pop_slice(output);
        if read < output.len() {
            // Buffer underflow: Zero out remaining audio samples
            output[read..].fill(0.0);
        }
    }
}
```

### 4.4 System Memory Profiling & OOM Prevention

Slate constantly monitors OS-level resident memory (RSS). If RSS approaches hardware thresholds, the system triggers graceful degraded cache shedding rather than panicking or crashing.

```rust
// crates/slate-core/src/memory_guard.rs
use tracing::warn;

pub enum MemoryPressure {
    Normal,
    Warning(usize),  // > 75% budget
    Critical(usize), // > 90% budget
}

pub fn get_resident_memory_bytes() -> usize {
    #[cfg(target_os = "macos")]
    unsafe {
        use mach::kern_return::KERN_SUCCESS;
        use mach::message::mach_msg_type_number_t;
        use mach::task::task_info;
        use mach::task_info::{mach_task_basic_info, MACH_TASK_BASIC_INFO, MACH_TASK_BASIC_INFO_COUNT};
        use mach::traps::mach_task_self;

        let mut info = mach_task_basic_info::default();
        let mut count = MACH_TASK_BASIC_INFO_COUNT as mach_msg_type_number_t;
        let kret = task_info(
            mach_task_self(),
            MACH_TASK_BASIC_INFO,
            &mut info as *mut _ as *mut i32,
            &mut count,
        );
        if kret == KERN_SUCCESS {
            info.resident_size as usize
        } else {
            0
        }
    }

    #[cfg(target_os = "linux")]
    {
        use std::fs::File;
        use std::io::Read;
        let mut statm = String::new();
        if let Ok(mut f) = File::open("/proc/self/statm") {
            let _ = f.read_to_string(&mut statm);
            let parts: Vec<&str> = statm.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(pages) = parts[1].parse::<usize>() {
                    return pages * 4096;
                }
            }
        }
        0
    }

    #[cfg(target_os = "windows")]
    unsafe {
        use windows::Win32::System::ProcessStatus::*;
        use windows::Win32::System::Threading::GetCurrentProcess;
        let mut counters = PROCESS_MEMORY_COUNTERS::default();
        let handle = GetCurrentProcess();
        if GetProcessMemoryInfo(handle, &mut counters, std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32).as_bool() {
            counters.WorkingSetSize
        } else {
            0
        }
    }
}
```

---

## 5. GPU Optimization & Acceleration

### 5.1 Pipeline Caching (`wgpu::PipelineCache`)

Shader compilation via Naga into Metal Shading Language (MSL), SPIR-V, or HLSL at runtime causes visual frame hitching on initial clip evaluation. Slate eliminates this through persistent pipeline caching and boot-time pre-compilation.

```rust
// crates/slate-render/src/pipeline_cache.rs
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};
use wgpu::{Device, PipelineCache, PipelineCacheDescriptor};

pub struct DiskPipelineCache {
    cache_path: PathBuf,
    pipeline_cache: Arc<PipelineCache>,
}

impl DiskPipelineCache {
    pub fn new(device: &Device, cache_dir: PathBuf) -> Self {
        let cache_path = cache_dir.join("wgpu_pipelines.bin");
        let initial_data = if cache_path.exists() {
            std::fs::read(&cache_path).ok()
        } else {
            None
        };

        let pipeline_cache = unsafe {
            device.create_pipeline_cache(&PipelineCacheDescriptor {
                label: Some("SlateGlobalPipelineCache"),
                data: initial_data.as_deref(),
                fallback: true,
            })
        };

        Self {
            cache_path,
            pipeline_cache: Arc::new(pipeline_cache),
        }
    }

    pub fn raw(&self) -> &PipelineCache {
        &self.pipeline_cache
    }

    pub fn persist_to_disk(&self) {
        if let Some(bytes) = self.pipeline_cache.get_data() {
            if let Err(e) = std::fs::write(&self.cache_path, bytes) {
                warn!("Failed persisting wgpu pipeline cache to disk: {}", e);
            } else {
                info!("Persisted {} bytes of wgpu pipeline cache", self.cache_path.display());
            }
        }
    }
}
```

### 5.2 Pre-Compilation Suite at Startup
During the < 1.5s cold start sequence, Slate loads and pre-compiles all WGSL pipelines in parallel on a background thread:
1. `blit.wgsl` (Affine transformations, bilinear filtering)
2. `composite.wgsl` (Porter-Duff alpha compositing, BlendModes)
3. `color_grade.wgsl` (Rec.709 lift/gamma/gain, saturation, temperature/tint)
4. `lut3d.wgsl` (Tetrahedral 3D LUT cube interpolation)
5. `transition_dissolve.wgsl` (Crossfade, dip-to-color)
6. `blur.wgsl` (Separable Gaussian horizontal and vertical kernel passes)
7. `chroma_key.wgsl` (Green/blue spill suppression & matte erode)

### 5.3 Batch Rendering & Minimized Pass Submissions

Slate avoids breaking render passes across clips on the same timeline track:
- **Geometry Instancing**: A single quad instance buffer encodes translation, scaling, crop rectangles, and opacity for all non-overlapping clips visible on the current frame.
- **Texture 2D Arrays**: Video frames matching dimensions and formats are bound into a `texture_2d_array`, eliminating costly descriptor set swaps during composition.
- **Single Pass Composition**: Layers are blended directly into the swapchain target in a single multi-draw pass whenever color grading passes can be folded into the composite fragment shader.

### 5.4 Zero-Copy Frame Upload Architecture

CPU-to-GPU memory transfer via `queue.write_texture` incurs memory bus traffic (~1 ms for 1080p, ~4 ms for 4K). Slate implements platform-native zero-copy bridges:

```
Platform Zero-Copy Pathways:
├── macOS:   AVFoundation / VideoToolbox CVPixelBufferRef
│            └── IOSurfaceRef ──► Metal id<MTLTexture> ──► wgpu::hal::metal
├── Linux:   VAAPI / V4L2 DMA-BUF File Descriptor
│            └── VK_KHR_external_memory_fd ──► Vulkan VkImage ──► wgpu::hal::vulkan
└── Windows: NVDEC / D3D11VA Shared Texture Handle
             └── IDXGIResource::GetSharedHandle ──► ID3D12Resource ──► wgpu::hal::dx12
```

```rust
// crates/slate-render/src/zero_copy.rs
#[cfg(target_os = "macos")]
pub mod macos {
    use std::ffi::c_void;
    use wgpu::hal::metal as hal_metal;

    pub unsafe fn import_cvpixelbuffer_to_metal(
        device: &wgpu::Device,
        pixel_buffer: *mut c_void, // CVPixelBufferRef
    ) -> Result<wgpu::Texture, String> {
        // 1. Extract IOSurfaceRef from CVPixelBuffer
        // 2. Wrap via [MTLDevice newTextureWithDescriptor:iosurface:plane:]
        // 3. Import raw Metal texture handle into wgpu using device.as_hal()
        // Eliminates CPU copy; frame is available on GPU in 0.05ms
        todo!("Native Metal IOSurface zero-copy bridge")
    }
}
```
*Fallback*: When platform hardware bridges are unavailable, frames pass through pinned memory staging buffers via `wgpu::Queue::write_texture` with row byte pitch aligned to `wgpu::COPY_BYTES_PER_ROW_ALIGNMENT` (256 bytes).

---

## 6. Video Decode Optimization

### 6.1 GOP-Aware Seeking Strategy

Long-GOP formats (H.264, HEVC) only contain complete picture information on Intra-frames (I-frames), with intermediate P/B frames depending on prior frames. Scrubbing backwards or jumping randomly requires GOP navigation.

```mermaid
flowchart LR
    A["Target PTS = Frame 47"] --> B{"Is Frame in Cache?"}
    B -- Yes --> C["Instant Return (< 1ms)"]
    B -- No --> D["Lookup GOP Table in MediaProbe"]
    D --> E["Locate Preceding I-Frame (Frame 30)"]
    E --> F["Seek FFmpeg AVFormatContext to Frame 30"]
    F --> G["Decode Frames 30..46 (Discard Pixels)"]
    G --> H["Decode Frame 47 (Emit to Renderer)"]
    H --> I["Insert Frames 30..47 into LRU Cache"]
```

1. **GOP Index Table**: Generated during file import / media probe. Maps PTS timestamps to file byte offsets of preceding keyframes.
2. **Fast Forward Decode**: Intermediate frames (30 through 46) have pixel format conversion (`swscale`) disabled; FFmpeg decodes directly into reusable YUV packet scratch buffers without transferring to RGB.
3. **Seek Debounce & Cancellation**: When the user drags the playhead rapidly across the timeline, pending decode requests in the decode worker queue are tagged with a monotonically increasing `scrub_generation: u64`. Stale decode jobs matching older generations are aborted immediately at the demuxer boundary.

### 6.2 Hardware-Accelerated Decode Integrations

| Platform | Hardware Subsystem | FFmpeg HW Acceleration Device Type | Pixel Format Output |
| :--- | :--- | :--- | :--- |
| **macOS** | Apple Silicon VideoToolbox / Intel VDA | `AV_HWDEVICE_TYPE_VIDEOTOOLBOX` | `AV_PIX_FMT_VIDEOTOOLBOX` / `NV12` |
| **NVIDIA (PC/Linux)** | NVIDIA NVDEC (via CUDA) | `AV_HWDEVICE_TYPE_CUDA` | `AV_PIX_FMT_CUDA` / `NV12` |
| **Linux (Intel/AMD)** | VAAPI (Video Acceleration API) | `AV_HWDEVICE_TYPE_VAAPI` | `AV_PIX_FMT_VAAPI` |
| **Windows (Intel/AMD/NV)**| Direct3D 11 Video Acceleration | `AV_HWDEVICE_TYPE_D3D11VA` | `AV_PIX_FMT_D3D11` |

### 6.3 Multi-Threaded Codec Configuration

When instantiating FFmpeg `AVCodecContext` instances in `slate-media`:
- `thread_count`: Configured to `0` (FFmpeg auto-selects optimal thread count based on logical cores, capped at 8).
- `thread_type`: `FF_THREAD_FRAME | FF_THREAD_SLICE` enabled for multi-slice concurrent decoding.
- **Multi-Instance Isolation**: Each video track in the timeline is assigned an independent decoder instance on a separate worker thread. A 4-track timeline decodes 4 tracks concurrently across 4 CPU cores rather than multiplexing a single context.

---

## 7. Export Pipeline Optimization

Offline export demands maximum hardware throughput without being throttled by display V-Sync or UI rendering.

### 7.1 Asynchronous Parallel Export Architecture

```mermaid
flowchart LR
    subgraph Decode_Stage ["Parallel Decode Stage"]
        T1["Track 1 Worker"]
        T2["Track 2 Worker"]
        T3["Track 3 Worker"]
        T4["Track 4 Worker"]
    end

    subgraph GPU_Stage ["GPU Render Stage"]
        COMP["wgpu Headless Compositor<br/>Offscreen Pass"]
    end

    subgraph Readback_Stage ["Pipelined Readback Stage"]
        RB0["Readback Buffer A (Frame N)"]
        RB1["Readback Buffer B (Frame N+1)"]
    end

    subgraph Encode_Stage ["Hardware Encode Stage"]
        HWENC["Hardware Video Encoder<br/>(VideoToolbox / NVENC)"]
    end

    subgraph Audio_Stage ["Audio Rendering Stage"]
        AMIX["Parallel Audio Mixdown"]
    end

    subgraph Mux_Stage ["Multiplexer Stage"]
        MUX["FFmpeg MP4/MOV Muxer"]
    end

    T1 --> COMP
    T2 --> COMP
    T3 --> COMP
    T4 --> COMP

    COMP --> RB0
    COMP --> RB1

    RB0 --> HWENC
    RB1 --> HWENC

    HWENC --> MUX
    AMIX --> MUX
```

### 7.2 Double-Buffered GPU Readback Loop

To prevent CPU stalling while reading pixels back from GPU VRAM:
1. Two staging buffers (`wgpu::Buffer` with usage `COPY_DST | MAP_READ`) are toggled frame-by-frame.
2. While Frame $N$ is being read and mapped into host memory for encoding, Frame $N+1$ is being copied into the secondary staging buffer by the GPU.

```rust
// crates/slate-render/src/export_readback.rs
use std::sync::Arc;
use wgpu::{Buffer, BufferDescriptor, BufferUsages, CommandEncoder, Device, Extent3d, ImageCopyBuffer, ImageCopyTexture, Origin3d, Texture, TextureAspect};

pub struct DoubleBufferedReadback {
    device: Arc<Device>,
    buffers: [Buffer; 2],
    active_idx: usize,
    width: u32,
    height: u32,
}

impl DoubleBufferedReadback {
    pub fn new(device: Arc<Device>, width: u32, height: u32) -> Self {
        let unpadded_bytes_per_row = width * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = ((unpadded_bytes_per_row + align - 1) / align) * align;
        let buffer_size = (padded_bytes_per_row * height) as u64;

        let desc = BufferDescriptor {
            label: Some("ExportStagingBuffer"),
            size: buffer_size,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        };

        let b0 = device.create_buffer(&desc);
        let b1 = device.create_buffer(&desc);

        Self {
            device,
            buffers: [b0, b1],
            active_idx: 0,
            width,
            height,
        }
    }

    pub fn copy_frame_to_staging(&mut self, encoder: &mut CommandEncoder, source: &Texture) {
        let unpadded = self.width * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let bytes_per_row = ((unpadded + align - 1) / align) * align;

        encoder.copy_texture_to_buffer(
            ImageCopyTexture {
                texture: source,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            ImageCopyBuffer {
                buffer: &self.buffers[self.active_idx],
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        self.active_idx = 1 - self.active_idx;
    }
}
```

### 7.3 Hardware-Accelerated Export Encoders

- **macOS**: `h264_videotoolbox` / `hevc_videotoolbox` leveraging Apple Silicon dedicated Media Engine encoders (reaches **≥ 5× real-time** on M2).
- **Windows / Linux (NVIDIA)**: `h264_nvenc` / `hevc_nvenc` using NVENC Dual Encoders (reaches **≥ 4× real-time** on RTX 3060).
- **Linux (AMD/Intel)**: `h264_vaapi` hardware encoding.
- **Fallback**: Multithreaded `libx264` with preset `fast` and CRF 18.

---

## 8. Startup & Project Loading Optimization

### 8.1 Startup Phase Budget (< 1.5s Total)

```
[0.00s] Process Launch & Dynamic Linking (50ms)
[0.05s] Tokio Multi-Thread Runtime Boot (15ms)
[0.065s] wgpu Adapter & Device Initialization (180ms)
[0.245s] Shader Pre-Compilation & Pipeline Cache Load (200ms)
[0.445s] Audio Subsystem Initialization (cpal) (120ms)
[0.565s] egui / eframe Window Creation & Font Texture Rasterization (180ms)
[0.745s] IPC & MCP Socket Listener Binding (20ms)
[0.765s] Initial UI Frame Composition & Swapchain Presentation (100ms)
[0.865s] Ready for User Interaction (Target < 1.500s)
```

### 8.2 Startup Strategies
1. **Lazy Initialization of FFmpeg Codecs**: During project open, only file headers are probed for container metadata (duration, streams, dimensions). Decoders are instantiated lazily only when clips become visible in the active timeline viewport.
2. **Parallel Cache Hydration**: Waveform peak data (`.slate/cache/waveforms/*.peak`) and filmstrip thumbnails are read into memory using Rayon parallel iterators.
3. **Deferred UI Drawing**: Offscreen inspector tabs, hidden tracks, and non-visible asset panels are not evaluated or laid out until scrolled into view.
4. **Shader Cache Persistence**: Shaders are cached in binary format under `~/.cache/slate/pipelines.bin`. On subsequent launches, shader compilation time drops from ~200ms to **< 15ms**.

---

## 9. MCP Response Optimization

The Model Context Protocol (MCP) server enables AI agents to query and mutate the project. Agent workflows can fail or stall if tool responses consume excessive context tokens or exceed latency timeouts.

### 9.1 Compact `timeline_summary` Architecture (< 2 KB for 100 Clips)

A standard serialized JSON timeline containing 100 clips easily exceeds **45 KB**, overflowing LLM context windows and incurring heavy JSON parsing overhead. Slate maintains a pre-computed, compact summary representation that invalidates only on timeline mutations.

```rust
// crates/slate-mcp/src/summary.rs
use serde::{Deserialize, Serialize};
use slate_core::model::Timeline;
use slate_core::time::RationalTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactClipSummary {
    pub id: String,         // First 8 characters of UUID (e.g. "a1b2c3d4")
    pub name: String,       // Truncated to max 16 chars
    pub track: u8,          // Track index
    pub start: String,      // SMPTE timecode "00:01:23:14"
    pub duration: String,   // SMPTE duration "00:00:04:00"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactTimelineSummary {
    pub project_name: String,
    pub duration: String,
    pub video_tracks: usize,
    pub audio_tracks: usize,
    pub total_clips: usize,
    pub clips: Vec<CompactClipSummary>,
}

pub struct TimelineSummaryCache {
    cached_json: parking_lot::RwLock<Option<String>>,
}

impl TimelineSummaryCache {
    pub fn new() -> Self {
        Self {
            cached_json: parking_lot::RwLock::new(None),
        }
    }

    pub fn invalidate(&self) {
        *self.cached_json.write() = None;
    }

    pub fn get_or_build(&self, timeline: &Timeline) -> String {
        {
            if let Some(ref json) = *self.cached_json.read() {
                return json.clone();
            }
        }

        // Build ultra-compact summary
        let mut clips = Vec::with_capacity(128);
        for (track_idx, track) in timeline.tracks().iter().enumerate() {
            for clip in track.clips() {
                clips.push(CompactClipSummary {
                    id: clip.id.to_string()[..8].to_string(),
                    name: clip.name.chars().take(16).collect(),
                    track: track_idx as u8,
                    start: clip.place_range.start.to_timecode(30, false),
                    duration: clip.place_range.duration.to_timecode(30, false),
                });
            }
        }

        let summary = CompactTimelineSummary {
            project_name: timeline.name.clone(),
            duration: timeline.duration().to_timecode(30, false),
            video_tracks: timeline.video_tracks().len(),
            audio_tracks: timeline.audio_tracks().len(),
            total_clips: clips.len(),
            clips,
        };

        let compact_json = serde_json::to_string(&summary).unwrap();
        *self.cached_json.write() = Some(compact_json.clone());
        compact_json
    }
}
```

#### Size Comparison: 100-Clip Project Representation
- Full `slate-core` Raw JSON Timeline: **~48,200 bytes** (~12,050 tokens)
- Verbose MCP Timeline Listing: **~18,400 bytes** (~4,600 tokens)
- **Slate Compact `timeline_summary`**: **1,840 bytes** (~460 tokens) — **> 96% token reduction**.

### 9.2 Pagination & Range Queries for AI Agents

For projects containing hundreds or thousands of clips, the MCP server provides cursor-based window queries:
- `mcp_get_timeline_range(start_time: "00:02:00:00", duration: "00:01:00:00", tracks: [0, 1])`
- `mcp_get_history(cursor: 40, limit: 20)`
This ensures LLM context usage remains deterministic regardless of project scale.

### 9.3 In-Process Direct-Call Dispatch (Headless Mode)

When Slate CLI executes autonomous MCP workflows, the `slate-mcp` crate interacts directly with `slate-core` via in-process Rust trait method calls without spawning an IPC daemon or encoding JSON-RPC payloads over sockets, achieving **p95 tool latency < 5 ms**.

---

## 10. Profiling, Tracing & Real-Time Monitoring

### 10.1 Structured Tracing (`tracing` Crate)

Slate instruments all hot paths using the `tracing` framework with microsecond precision:

```rust
// crates/slate-render/src/compositor.rs
use tracing::{info_span, instrument};
use slate_core::model::FrameRequest;

pub struct RenderedFrame {
    pub width: u32,
    pub height: u32,
    pub pts: slate_core::time::RationalTime,
}

pub struct GpuCompositor {
    // ...
}

impl GpuCompositor {
    #[instrument(skip(self, request), fields(pts = %request.pts, track_count = request.layers.len()))]
    pub fn composite_frame(&self, request: &FrameRequest) -> Result<RenderedFrame, Box<dyn std::error::Error>> {
        let _span = info_span!("frame_pipeline", frame = request.frame_index).entered();

        let decoded_layers = {
            let _decode_span = info_span!("decode_gather").entered();
            self.gather_decoded_textures(request)?
        };

        let uploaded_textures = {
            let _upload_span = info_span!("texture_upload").entered();
            self.upload_or_bind_surfaces(&decoded_layers)?
        };

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("FrameCompositeEncoder"),
        });

        {
            let _composite_span = info_span!("gpu_composite_pass").entered();
            self.record_compositing_pass(&mut encoder, &uploaded_textures)?;
        }

        {
            let _present_span = info_span!("queue_submit").entered();
            self.queue.submit(Some(encoder.finish()));
        }

        Ok(RenderedFrame {
            width: request.target_width,
            height: request.target_height,
            pts: request.pts,
        })
    }
}
```

### 10.2 Real-Time Runtime Telemetry HUD

The desktop UI exposes an unobtrusive developer telemetry overlay (toggled via `Cmd+Option+P` / `Ctrl+Alt+P`):
- **Live Dropped Frame Counter**: Aggregates playback hitching events.
- **Decode Queue Depths**: Live gauge of `crossbeam` channel capacities across all video tracks.
- **Texture Pool Utilization**: Count of active vs pooled textures and total allocated GPU VRAM.
- **Audio Ring Buffer Margin**: Underflow / overflow proximity indicator in milliseconds.
- **MCP Tool Latency Histogram**: Rolling p50, p95, and p99 execution latencies.

### 10.3 Hot-Path Microbenchmark Suite (`criterion`)

Microbenchmarks live in `benches/` across all crates to catch performance regressions in CI before merging:

```rust
// benches/timeline_ops.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use slate_core::model::{Timeline, Clip, TrackKind};
use slate_core::op::{ClipOps, Op};
use slate_core::time::{RationalTime, TimeRange};
use uuid::Uuid;

fn benchmark_500_clip_timeline_mutation(c: &mut Criterion) {
    let mut timeline = Timeline::new("BenchProject", RationalTime::new(30, 1));
    let track_id = timeline.add_track(TrackKind::Video).unwrap();

    // Populate timeline with 500 clips
    for i in 0..500 {
        let clip = Clip::new(
            Uuid::new_v4(),
            format!("Clip_{}", i),
            TimeRange::new(RationalTime::new(i * 30, 30), RationalTime::new(30, 30)),
            TimeRange::new(RationalTime::new(0, 30), RationalTime::new(30, 30)),
        );
        timeline.insert_clip(track_id, clip).unwrap();
    }

    let clip_to_move = timeline.tracks()[0].clips()[250].id;

    c.bench_function("move_clip_500_items", |b| {
        b.iter(|| {
            let op = Op::MoveClip {
                clip_id: black_box(clip_to_move),
                new_start: black_box(RationalTime::new(7600, 30)),
            };
            timeline.apply(op).unwrap();
        });
    });
}

criterion_group!(benches, benchmark_500_clip_timeline_mutation);
criterion_main!(benches);
```

---

## 11. Deterministic Rendering & Golden Frame Suite

### 11.1 WYSIWYG Preview / Export Parity

A common failure mode in video editors is preview rendering deviating from exported master files (e.g., color banding, differing alpha blending equations, or font kerning drift).

Slate enforces **Mathematical & Visual Parity**:
1. **Single Shader Graph**: The exact same WGSL shader code executed for timeline preview is executed for offline master exports.
2. **Deterministic Color Space**: Compositing occurs strictly in linear Rec.709 or linear ACEScg, with sRGB display transfer functions applied only at presentation.
3. **Deterministic Rounding**: Keyframe parameter interpolation utilizes IEEE 754-2008 compliant double-precision floats (`f64`) before conversion to WGSL uniform `f32` vectors.

### 11.2 Golden Frame Perceptual Test Suite

The test suite in `tests/golden_frames/` renders complex synthetic test projects across both the CPU reference software renderer and the `wgpu` GPU compositor, calculating Peak Signal-to-Noise Ratio (PSNR) and Structural Similarity Index (SSIM):

$$\text{MSE} = \frac{1}{3 W H} \sum_{c=1}^3 \sum_{x=1}^W \sum_{y=1}^H \left( I_{\text{preview}}(x, y, c) - I_{\text{export}}(x, y, c) \right)^2$$

$$\text{PSNR} = 10 \cdot \log_{10} \left( \frac{255^2}{\text{MSE}} \right)$$

- **Pass Criterion**: **PSNR ≥ 45.0 dB** and **SSIM ≥ 0.999** across all golden frames.
- Any GPU vendor floating-point deviation causing PSNR $< 45$ dB triggers immediate CI failure.

### 11.3 10-Iteration Determinism Invariant Test

```rust
// tests/test_determinism.rs
use sha2::{Sha256, Digest};
use slate_render::OffscreenRenderer;
use slate_core::model::FrameRequest;

#[test]
fn test_render_determinism_ten_iterations() {
    let renderer = OffscreenRenderer::new(1920, 1080);
    let request = FrameRequest::synthetic_test_pattern();

    let initial_frame = renderer.render_frame_cpu_sync(&request).unwrap();
    let initial_hash = Sha256::digest(&initial_frame.data);

    for i in 1..10 {
        let current_frame = renderer.render_frame_cpu_sync(&request).unwrap();
        let current_hash = Sha256::digest(&current_frame.data);
        assert_eq!(
            initial_hash, current_hash,
            "Non-deterministic render detected on iteration {}: hash mismatch", i
        );
    }
}
```

---

## 12. Step-by-Step Implementation Checklist

This checklist defines the complete development, benchmarking, and optimization roadmap. Every item must be verified and checked off for v1 release certification.

### Phase 1: Concurrency & Real-Time Threading Infrastructure
- [ ] **Thread Architecture Topology (`slate-core`, `slate-audio`, `slate-render`)**
  - [ ] Implement OS-specific thread priority elevation in `crates/slate-audio/src/priority.rs` (Mach time constraint on macOS, `SCHED_FIFO` on Linux, `THREAD_PRIORITY_TIME_CRITICAL` on Windows).
  - [ ] Build lock-free SPSC audio ring buffer bridge using `ringbuf` in `crates/slate-audio/src/ring_buffer.rs`.
  - [ ] Implement `crossbeam_channel::bounded(4)` multi-track decoded frame dispatch in `crates/slate-media/src/dispatcher.rs`.
  - [ ] Establish `tokio::sync::broadcast` event distribution channel in `crates/slate-core/src/events.rs`.
  - [ ] Verify zero allocations in `cpal` audio output callback via static memory assertions and allocator hook tests.

### Phase 2: Frame Pipeline & Microsecond Budget Enforcement
- [ ] **Frame Pacing & Pipeline Timing (`slate-render`, `slate-media`)**
  - [ ] Implement microsecond timing instrumentation via `tracing` spans around Decode, Upload, Transform, Effects, Composite, and Present.
  - [ ] Build the `DoubleBufferedReadback` pipeline in `crates/slate-render/src/export_readback.rs` for offscreen exports.
  - [ ] Implement monotonic master audio clock synchronizer with dynamic video frame skip/yield drift compensation.
  - [ ] Verify 1080p30 4-track playback completes within the 11.5 ms timing budget with 0 dropped frames on reference hardware.

### Phase 3: Zero-Allocation Memory Pools
- [ ] **Frame & Texture Caching Subsystems (`slate-media`, `slate-render`)**
  - [ ] Implement byte-weighted `FrameCache` with LRU eviction and configurable budget (default 512 MB) in `crates/slate-media/src/cache.rs`.
  - [ ] Implement directional adaptive prefetching (`PrefetchEngine`) supporting 1x forward, 2x fast-forward, -1x reverse, and scrub modes.
  - [ ] Implement descriptor-hashed `TexturePool` in `crates/slate-render/src/texture_pool.rs` with resolution tier recycling.
  - [ ] Implement idle texture trim GC (120-frame retirement).
  - [ ] Implement OS resident set size (RSS) monitor (`mach_task_basic_info`, `/proc/self/statm`, `GetProcessMemoryInfo`) with multi-stage OOM warning thresholds.

### Phase 4: GPU Acceleration & Pipeline Optimization
- [ ] **Shader Compilation & Zero-Copy Uploads (`slate-render`)**
  - [ ] Implement `DiskPipelineCache` wrapping `wgpu::PipelineCache` with disk serialization to `~/.cache/slate/pipelines.bin`.
  - [ ] Implement boot-time pre-compilation routine for all 7 standard WGSL shader modules.
  - [ ] Build macOS `IOSurfaceRef` -> Metal zero-copy texture import bridge.
  - [ ] Build Linux `DMA-BUF` -> Vulkan external memory import bridge (`VK_KHR_external_memory_fd`).
  - [ ] Build Windows `DXGI` shared texture import bridge.
  - [ ] Implement 256-byte aligned fallback staging buffer upload for platforms lacking zero-copy support.

### Phase 5: Fast GOP Decode & Seeking
- [ ] **GOP-Aware Seeking & HW Decode (`slate-media`)**
  - [ ] Build GOP index structure parser during media probing (`crates/slate-media/src/probe.rs`).
  - [ ] Implement nearest keyframe backward seek + forward fast-decode discard loop for playhead scrubbing.
  - [ ] Add `scrub_generation` cancellation token to abort obsolete decode requests in flight.
  - [ ] Enable multi-threaded FFmpeg decoding (`thread_count = 0`, `FF_THREAD_FRAME | FF_THREAD_SLICE`).
  - [ ] Implement hardware decode acceleration bindings: VideoToolbox (macOS), NVDEC (NVIDIA), VAAPI (Linux), D3D11VA (Windows).

### Phase 6: High-Throughput Export Acceleration
- [ ] **Hardware Export Pipeline (`slate-media`, `slate-render`)**
  - [ ] Implement parallel 4-track decode -> GPU offscreen composite -> staging readback pipeline.
  - [ ] Integrate hardware video encoders: `h264_videotoolbox` (macOS), `h264_nvenc` (NVIDIA), `h264_vaapi` (Linux), `h264_mf` (Windows).
  - [ ] Parallelize concurrent audio mixdown and stream interleaving.
  - [ ] Benchmark and verify $\ge 3\times$ real-time export throughput on 1080p H.264 masters.

### Phase 7: MCP Agent Response Optimization
- [ ] **Compact Serialization & Range Queries (`slate-mcp`)**
  - [ ] Implement `CompactTimelineSummary` generator and caching layer in `crates/slate-mcp/src/summary.rs`.
  - [ ] Verify 100-clip summary JSON size is $< 2$ KB.
  - [ ] Implement cursor-based pagination and temporal range filters for timeline queries.
  - [ ] Implement in-process direct trait dispatch for headless CLI / agent workflows.
  - [ ] Verify MCP non-render tool latency p95 is $< 200$ ms; `render_frame` p95 is $< 500$ ms.

### Phase 8: Benchmarks, Telemetry & Parity Certification
- [ ] **Verification & Determinism Suite (`tests/`, `benches/`)**
  - [ ] Implement Criterion microbenchmarks for timeline ops (500 clips $< 16$ ms), rational time arithmetic, and audio mixing.
  - [ ] Create real-time telemetry HUD overlay in `slate-ui` displaying dropped frames, queue depths, VRAM, and MCP latencies.
  - [ ] Build golden frame test harness comparing preview render outputs against master exports.
  - [ ] Certify preview/export parity reaches PSNR $\ge 45$ dB and SSIM $\ge 0.999$.
  - [ ] Implement and verify 10-iteration bitwise determinism test.
  - [ ] Measure and certify cold start interactive time $< 1.5$ s and project reopen time $< 3$ s.
