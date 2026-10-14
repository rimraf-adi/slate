# Implementation Plan: `slate-media` Media Pipeline

- **Target Crate:** `crates/slate-media`
- **Specification Version:** 1.0.0
- **Status:** Draft / Specification
- **Dependencies:** `slate-core` (traits, `RationalTime`, `MediaId`, `PixelFormat`), `ffmpeg-next = "7.1"`, `wgpu = "0.20"`, `crossbeam-channel = "0.5"`, `dashmap = "6.0"`, `parking_lot = "0.12"`

---

## 1. Overview

`slate-media` provides the high-performance media demuxing, decoding, encoding, hardware acceleration, and metadata probing foundation for the Slate non-linear video editor. Built on top of FFmpeg via the safe `ffmpeg-next` Rust bindings and platform-native multimedia APIs, `slate-media` fulfills the `MediaDecoder` trait defined in `slate-core` while enforcing thread-safe resource isolation, zero-copy GPU memory pathways, frame-accurate seeking, and background utility pipelines (proxies, waveforms, thumbnails).

### 1.1 Architecture & Core Responsibilities

```
+-----------------------------------------------------------------------------------------+
|                                    SLATE APPLICATION                                   |
|   (Timeline Engine, Video Compositor [wgpu], Audio Mixer, Playhead Scrub Controller)    |
+-----------------------------------------------------------------------------------------+
                                 |                                 ^
           Frame Requests        |                                 | Decoded Frames / Audio
           & Seek Commands       v                                 |
+-----------------------------------------------------------------------------------------+
|                                      SLATE-MEDIA                                       |
|                                                                                         |
|  +------------------------+      +---------------------+      +----------------------+  |
|  |     DecoderPool        | ---> |  Worker Thread 0    | ---> |  HW Context (Metal/  |  |
|  | (N Worker Dispatcher)  |      |  (AVFormatContext)  |      |  Vulkan/DXGI)        |  |
|  +------------------------+      +---------------------+      +----------------------+  |
|              |                             |                               |            |
|              | Request Channel             v Packet / Frame Queue          v            |
|              |                   +---------------------+      +----------------------+  |
|              |                   |  Worker Thread N    | ---> | Software Fallback    |  |
|              |                   |  (AVCodecContext)   |      | (swscale/swresample) |  |
|              |                   +---------------------+      +----------------------+  |
|              v                                                     |                    |
|  +------------------------+                                        v                    |
|  |       FrameCache       | <======================================================+    |
|  |  (Weighted Memory LRU) |                                                             |
|  +------------------------+                                                             |
|                                                                                         |
|  +-----------------------------------------------------------------------------------+  |
|  | Background Pipelines:                                                             |  |
|  | - ProxyManager: 720p H.264 offline transcoder with progress channels              |  |
|  | - WaveformGenerator: Multi-resolution peak/RMS binary cache generation            |  |
|  | - ThumbnailGenerator: Fast I-frame contact sheet & filmstrip extractor            |  |
|  | - VfrConformer: FFmpeg filter-graph CFR conformer                                  |  |
|  | - ExportEncoder: Muxing, HW/SW encoding, container serialization                  |  |
|  +-----------------------------------------------------------------------------------+  |
+-----------------------------------------------------------------------------------------+
```

### 1.2 Design Philosophy

1. **Strict Context Isolation:** FFmpeg's `AVFormatContext` and `AVCodecContext` structures are fundamentally non-thread-safe. `slate-media` isolates each context to a single thread worker within a worker pool, completely eliminating locking on the demuxer/codec FFI boundaries.
2. **Zero-Copy First:** Frame data remains on hardware surfaces (`IOSurface` on macOS, `DMA-BUF` on Linux, `DXGI Shared Surfaces` on Windows) whenever hardware acceleration is active, importing directly into `wgpu::Texture` descriptors without round-tripping through system RAM.
3. **Graceful Degradation:** Hardware acceleration failure at any point (driver initialization, codec profile unsupported, mid-stream packet decode error) triggers an automatic, transparent fallback to optimized CPU decoding (`dav1d`, `ffh264`, `swscale`).
4. **Frame-Accurate Determinism:** Audio-video synchronization and NLE timeline precision demand exact timestamp alignment. Seeking employs GOP-aware backward keyframe seeking followed by forward discard decoding to land on the exact user-demanded PTS.

---

## 2. Supported Formats

### 2.1 Decode Support Matrix

| Media / Codec | FourCC / FFmpeg ID | Container Formats | Hardware Acceleration Path | Fallback Decoder | Notes |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **H.264 / AVC** | `avc1`, `h264` | MP4, MOV, MKV, M4V, TS | VideoToolbox, NVDEC, VAAPI, D3D11VA | `ffh264` (multi-threaded CPU) | Full 8-bit & 10-bit High Profile support |
| **H.265 / HEVC** | `hvc1`, `hev1` | MP4, MOV, MKV, TS | VideoToolbox, NVDEC, VAAPI, D3D11VA | `ffhevc` (multi-threaded CPU) | 8-bit, 10-bit Main/Main10 profiles |
| **Apple ProRes** | `apch`, `apcn`, `apcs`, `apco`, `ap4h` | MOV, MKV | VideoToolbox (Apple Silicon/T2 HW) | `prores` (software) | Proxy, LT, 422, 422 HQ, 4444, 4444 XQ |
| **VP9** | `vp09` | WebM, MKV | VAAPI, NVDEC, D3D11VA | `libvpx-vp9` | Profile 0 (8-bit) and Profile 2 (10-bit) |
| **AV1** | `av01` | MP4, WebM, MKV | NVDEC (RTX 30+), VAAPI (Intel Gen12+), VideoToolbox (M3+) | `libdav1d` | High-efficiency next-gen open codec |
| **Linear PCM / WAV**| `pcm_s16le`, `pcm_s24le`, `pcm_f32le` | WAV, AIFF, MOV | N/A (CPU) | Built-in FFmpeg PCM decoders | Lossless broadcast standard |
| **MP3** | `mp3` | MP3 | N/A (CPU) | `mp3float` | Standard consumer audio |
| **AAC** | `mp4a.40.2` | MP4, M4A, MOV, MKV | VideoToolbox (macOS audio decode) | `aac` | Low-complexity (LC) and High-efficiency (HE) |
| **PNG** | `png` | PNG | N/A | `png` / `image-rs` | Alpha channel preservation (RGBA) |
| **JPEG** | `jpeg`, `mjpeg` | JPG, JPEG | N/A | `mjpeg` / `image-rs` | Photographic stills and MJPEG video |

### 2.2 Encode Support Matrix

| Codec | Container | Hardware Acceleration | Bitrate Modes | Target Use-Case |
| :--- | :--- | :--- | :--- | :--- |
| **H.264 / AVC** | MP4 (`.mp4`), MOV (`.mov`) | `h264_videotoolbox`, `h264_nvenc`, `h264_vaapi` | CRF, CBR, VBR (2-pass optional) | Web distribution, YouTube, Vimeo, Discord |
| **H.265 / HEVC** | MP4 (`.mp4`), MOV (`.mov`) | `hevc_videotoolbox`, `hevc_nvenc`, `hevc_vaapi` | CRF, CBR, CQP | 4K/8K master export, HDR10 delivery |
| **Apple ProRes** | MOV (`.mov`) | `prores_videotoolbox` (macOS) / `prores_ks` (CPU) | Constant Quality (Profile-dependent) | Archival, grading intermediate, master |
| **AAC** | MP4 (`.mp4`), M4A (`.m4a`) | Native OS audio encoder (AudioToolbox / MediaFoundation) | CBR (96k–320k), VBR | General delivery stereo audio |
| **Linear PCM** | WAV (`.wav`), MOV (`.mov`) | N/A | Lossless (16-bit, 24-bit, 32-bit float) | High-fidelity master audio output |

---

## 3. Architecture & Core Types

### 3.1 Crate File Structure

```
crates/slate-media/
├── Cargo.toml
├── src/
│   ├── lib.rs                  # Module declarations, FFmpeg global initialization
│   ├── error.rs                # MediaError, Result<T, MediaError>
│   ├── probe.rs                # MediaProbe, StreamInfo, VFR detection
│   ├── frame.rs                # VideoFrame, AudioBuffer, PixelFormat, Colorimetry
│   ├── ffi_safety.rs           # RAII wrappers, buffer pools, safe FFI helpers
│   ├── decoder/
│   │   ├── mod.rs              # MediaDecoder trait implementation & DecoderHandle
│   │   ├── pool.rs             # DecoderWorkerPool, task dispatching
│   │   ├── worker.rs           # DecoderWorker thread loop, packet parsing
│   │   ├── seek.rs             # GOP indexing and forward-discard seek engine
│   │   └── prefetch.rs         # Directional playback prefetcher
│   ├── cache/
│   │   ├── mod.rs              # FrameCache trait and public interface
│   │   ├── lru.rs              # Byte-budgeted LRU cache implementation
│   │   └── key.rs              # CacheKey structure
│   ├── hw/
│   │   ├── mod.rs              # Hardware abstraction and detection engine
│   │   ├── videotoolbox.rs     # macOS VideoToolbox + IOSurface + Metal interop
│   │   ├── vaapi.rs            # Linux VAAPI + DMA-BUF + Vulkan interop
│   │   ├── d3d11va.rs          # Windows D3D11VA + DXGI shared handles + DX12 interop
│   │   └── fallback.rs         # CPU swscale & swresample pipelines
│   ├── proxy/
│   │   ├── mod.rs              # ProxyManager, status tracker
│   │   └── generator.rs        # Background proxy transcode job runner
│   ├── vfr/
│   │   ├── mod.rs              # VFR detection and jitter statistics
│   │   └── conformer.rs        # FFmpeg filter-graph CFR conformer
│   ├── audio/
│   │   ├── mod.rs              # Audio stream decoder and resampler
│   │   └── waveform.rs         # Multi-zoom binary peak & RMS waveform generator
│   ├── thumbnail.rs            # I-frame extractor & contact sheet generator
│   └── encoder/
│       ├── mod.rs              # Encoder engine & export session
│       ├── presets.rs          # Export presets (YouTube, ProRes, etc.)
│       ├── muxer.rs            # AVFormat muxer and container writer
│       ├── video_encoder.rs    # HW/SW video encoder wrapper
│       └── audio_encoder.rs    # Audio encoder and resampler wrapper
└── tests/
    ├── probe_tests.rs
    ├── decode_tests.rs
    ├── seek_tests.rs
    ├── encode_tests.rs
    └── fixtures/
```

### 3.2 Key Data Structures

Here are the concrete type definitions implemented in `crates/slate-media/src/frame.rs` and `crates/slate-media/src/probe.rs`:

```rust
// crates/slate-media/src/frame.rs

use std::sync::Arc;
use slate_core::time::RationalTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelFormat {
    Rgba8,
    Bgra8,
    Rgb24,
    Nv12,
    Yuv420p,
    Yuv422p10Le,
    Yuv444p10Le,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
    Bt709,
    Bt2020,
    Bt601,
    Unspecified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorRange {
    Full,
    Limited,
}

/// Zero-copy hardware surface handles for GPU interop
#[derive(Debug, Clone)]
pub enum HwTextureHandle {
    #[cfg(target_os = "macos")]
    IOSurface {
        surface_ref: usize, // IOSurfaceRef as raw pointer address
        width: u32,
        height: u32,
        pixel_format: u32,  // OSType (e.g. '420v')
    },
    #[cfg(target_os = "linux")]
    DmaBuf {
        fd: std::os::unix::io::RawFd,
        stride: u32,
        modifier: u64,
        width: u32,
        height: u32,
        fourcc: u32,
    },
    #[cfg(target_os = "windows")]
    DxgiShared {
        shared_handle: usize, // HANDLE to shared DXGI texture
        width: u32,
        height: u32,
        dxgi_format: u32,
    },
}

/// Fully decoded video frame delivered to the compositor or cache
#[derive(Clone)]
pub struct VideoFrame {
    /// Host buffer memory (empty or omitted when hw_texture is present and zero-copy is retained)
    pub data: Arc<Vec<u8>>,
    pub width: u32,
    pub height: u32,
    pub strides: [usize; 4],
    pub pixel_format: PixelFormat,
    pub color_space: ColorSpace,
    pub color_range: ColorRange,
    pub timestamp: RationalTime,
    pub duration: RationalTime,
    /// Direct GPU handle when hardware decode was used without readback
    pub hw_texture: Option<HwTextureHandle>,
}

impl VideoFrame {
    /// Calculates the memory footprint in bytes for cache budget tracking
    pub fn memory_size_bytes(&self) -> usize {
        if self.hw_texture.is_some() {
            // Estimated GPU VRAM footprint
            (self.width * self.height * 4) as usize
        } else {
            self.data.len()
        }
    }
}

/// Decoded multichannel audio buffer
#[derive(Debug, Clone)]
pub struct AudioBuffer {
    /// Interleaved 32-bit floating point PCM audio samples
    pub samples: Arc<Vec<f32>>,
    pub channels: u16,
    pub sample_rate: u32,
    pub timestamp: RationalTime,
    pub sample_count: usize,
}

impl AudioBuffer {
    pub fn duration(&self) -> RationalTime {
        RationalTime::from_frames(self.sample_count as i64, self.sample_rate)
    }
}
```

```rust
// crates/slate-media/src/probe.rs

use std::path::{Path, PathBuf};
use slate_core::time::RationalTime;
use crate::error::MediaError;

#[derive(Debug, Clone)]
pub struct VideoStreamInfo {
    pub stream_index: usize,
    pub codec_name: String,
    pub codec_fourcc: String,
    pub width: u32,
    pub height: u32,
    pub frame_rate_num: u32,
    pub frame_rate_den: u32,
    pub nominal_fps: f64,
    pub bit_rate: Option<u64>,
    pub pixel_format: String,
    pub color_space: String,
    pub color_range: String,
    pub bit_depth: u8,
    pub total_frames: Option<i64>,
    pub is_vfr: bool,
    pub avg_packet_duration_jitter: f64,
}

#[derive(Debug, Clone)]
pub struct AudioStreamInfo {
    pub stream_index: usize,
    pub codec_name: String,
    pub channels: u16,
    pub channel_layout: String,
    pub sample_rate: u32,
    pub bit_rate: Option<u64>,
    pub sample_format: String,
}

#[derive(Debug, Clone)]
pub struct MediaProbeResult {
    pub file_path: PathBuf,
    pub duration: RationalTime,
    pub video_streams: Vec<VideoStreamInfo>,
    pub audio_streams: Vec<AudioStreamInfo>,
    pub container_format: String,
    pub file_size: u64,
    pub is_vfr: bool,
    pub creation_time: Option<String>,
}

pub fn probe_media(path: &Path) -> Result<MediaProbeResult, MediaError> {
    // Uses ffmpeg::format::input to inspect headers, packet timestamps, and stream properties
    // Details in Section 6 and 11
    todo!()
}
```

### 3.3 Decoder Pool Architecture

The `DecoderPool` orchestrates concurrent decoding across multiple clips on the timeline and satisfies real-time scrubbing requests.

```
                     +---------------------------------------+
                     |             DecoderPool               |
                     |  - worker_count = max(1, num_cpus/2)  |
                     |  - Round-robin / Media affinity queue |
                     +---------------------------------------+
                                         |
                +------------------------+------------------------+
                |                                                 |
         DecodeRequest                                     DecodeRequest
                |                                                 |
                v                                                 v
  +---------------------------+                     +---------------------------+
  |    DecoderWorker (Thread) |                     |    DecoderWorker (Thread) |
  |                           |                     |                           |
  | - Open AVFormatContext    |                     | - Open AVFormatContext    |
  | - Open AVCodecContext     |                     | - Open AVCodecContext     |
  | - GOP Seek Engine         |                     | - GOP Seek Engine         |
  | - HW Accelerator Context  |                     | - HW Accelerator Context  |
  +---------------------------+                     +---------------------------+
                |                                                 |
                v Frame Ready                                     v Frame Ready
  +-----------------------------------------------------------------------------+
  |                           FrameCache (DashMap + LRU)                        |
  +-----------------------------------------------------------------------------+
```

#### Pool Characteristics:
- **Worker Allocation:** By default, `worker_count = std::cmp::max(1, num_cpus::get() / 2)`. This leaves remaining CPU cores dedicated to UI rendering, audio synthesis, and the wgpu compositing engine.
- **Request Channels:** Crossbeam bounded channels provide backpressure. If the UI scrubs faster than frames can decode, stale requests are cancelled via atomic cancellation tokens.
- **Media Affinity:** Requests for the same `media_id` are preferentially routed to the worker already maintaining that file's `AVFormatContext` open, eliminating repeated file open and header re-parsing overhead.

```rust
// crates/slate-media/src/decoder/pool.rs

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crossbeam_channel::{bounded, Receiver, Sender};
use slate_core::time::RationalTime;
use slate_core::id::MediaId;
use crate::error::MediaError;
use crate::frame::VideoFrame;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DecodePriority {
    Scrubbing = 0, // Highest priority: user dragging the playhead
    Playback = 1,  // High priority: real-time sequential playback
    Prefetch = 2,  // Medium: filling read-ahead cache window
    Thumbnail = 3, // Low: background UI thumbnails
}

pub struct DecodeRequest {
    pub media_id: MediaId,
    pub target_time: RationalTime,
    pub priority: DecodePriority,
    pub cancel_token: Arc<AtomicBool>,
    pub response: Sender<Result<VideoFrame, MediaError>>,
}

pub struct DecoderPool {
    workers: Vec<WorkerHandle>,
    request_tx: Sender<DecodeRequest>,
}

struct WorkerHandle {
    thread: std::thread::JoinHandle<()>,
}

impl DecoderPool {
    pub fn new(num_workers: usize) -> Self {
        let (tx, rx) = bounded::<DecodeRequest>(128);
        let mut workers = Vec::with_capacity(num_workers);

        for worker_id in 0..num_workers {
            let worker_rx = rx.clone();
            let thread = std::thread::Builder::new()
                .name(format!("slate-decode-worker-{}", worker_id))
                .spawn(move || {
                    let mut worker = crate::decoder::worker::DecoderWorker::new(worker_id, worker_rx);
                    worker.run();
                })
                .expect("Failed to spawn decoder worker thread");

            workers.push(WorkerHandle { thread });
        }

        Self {
            workers,
            request_tx: tx,
        }
    }

    pub fn request_frame(
        &self,
        media_id: MediaId,
        target_time: RationalTime,
        priority: DecodePriority,
    ) -> (Receiver<Result<VideoFrame, MediaError>>, Arc<AtomicBool>) {
        let cancel_token = Arc::new(AtomicBool::new(false));
        let (res_tx, res_rx) = bounded(1);

        let req = DecodeRequest {
            media_id,
            target_time,
            priority,
            cancel_token: cancel_token.clone(),
            response: res_tx,
        };

        // If queue is full, evict or prioritize
        let _ = self.request_tx.send(req);
        (res_rx, cancel_token)
    }
}
```

### 3.4 GOP-Aware Seeking Strategy

Video compressed with modern long-GOP codecs (H.264, HEVC, AV1) relies on I-frames (Keyframes), P-frames (Predictive), and B-frames (Bi-directional). Fast scrubbing requires a deterministic two-phase seeking algorithm:

```
Timeline Target PTS: 00:01:23;14 (Frame 2002)
---------------------------------------------------------------------------------
[I-Frame 1980] ---> [B 1981] ---> [P 1982] ---> ... ---> [P 2002] ---> [B 2003]
     ^                                                      ^
     |                                                      |
1. Seek backwards to I-Frame 1980       2. Decode & discard forward to PTS 2002
```

1. **Step 1: Locate Nearest Preceding Keyframe:**
   - FFmpeg's `avformat_seek_file` is called with flag `AVSEEK_FLAG_BACKWARD`.
   - The seek target is timestamped at or before `target_time`.
   - Decoder buffers are flushed using `avcodec_flush_buffers(codec_ctx)`.
2. **Step 2: Forward Packet Decode & Discard:**
   - Worker reads packets sequentially from the demuxer.
   - Packets are passed to `avcodec_send_packet`.
   - Frames are received via `avcodec_receive_frame`.
   - Any frame with `PTS < target_time - half_frame_duration` is immediately returned to the buffer pool without pixel conversion (no `swscale` or texture copy).
   - Once the frame matching `target_time` is reached, it is color-converted / hardware-transferred and returned to the caller.
3. **Scrubbing Cache Optimization:**
   - During continuous reverse scrubbing, sequential backward seeking induces heavy GOP decode thrashing. To eliminate this, when decoding forward from an I-frame, all decoded frames within that GOP are held in a small local worker ring-buffer (capacity = average GOP size, ~30–60 frames). Subsequent reverse scrubbing requests within that same GOP are satisfied in $O(1)$ time without re-seeking.

### 3.5 Pre-Fetch Strategy for Sequential Playback

When the playhead is in `Playback` mode:
- The playback engine predicts the next $N$ frames needed based on timeline playback speed (`1x`, `2x`, or `-1x`).
- At $1x$ forward playback (24 fps), the pre-fetch window targets 24 frames ahead (~1 second of media buffer).
- Requests are dispatched with priority `DecodePriority::Prefetch`.
- If the playhead stops or the user abruptly seeks, all pending `Prefetch` tasks have their `cancel_token` set to `true`, instantly clearing worker pipelines for high-priority `Scrubbing` requests.

---

## 4. Hardware Acceleration

Zero-copy hardware acceleration eliminates the memory bus bottleneck of transferring uncompressed 4K 60fps video frames (approx. 1 GB/sec of bandwidth) between GPU and CPU.

### 4.1 macOS: VideoToolbox Integration

macOS uses Apple's VideoToolbox framework, producing underlying `CVPixelBufferRef` instances backed by `IOSurfaceRef` objects.

```
+------------------------------------+
|         FFmpeg VideoToolbox        |
|  hwaccel: AV_HWDEVICE_TYPE_VIDEOTOOLBOX
+------------------------------------+
                  |
                  v Decodes into
+------------------------------------+
|          CVPixelBufferRef          |
|      Format: kCVPixelFormatType_   |
|            420YpCbCr8BiPlanarVideoRange
+------------------------------------+
                  |
                  v CVPixelBufferGetIOSurface()
+------------------------------------+
|            IOSurfaceRef            |
+------------------------------------+
                  |
                  v wgpu::Device::create_texture_from_hal
+------------------------------------+
|        wgpu Metal Texture          |
|  (Zero-copy bind to render pipeline)|
+------------------------------------+
```

#### Implementation Steps:
1. **Enable HW Accelerator in FFmpeg:**
   Configure `av_hwdevice_ctx_create` with `AV_HWDEVICE_TYPE_VIDEOTOOLBOX`.
   Set `get_format` callback to select `AV_PIX_FMT_VIDEOTOOLBOX`.
2. **Extract IOSurface:**
   When a frame is received, `frame->data[3]` contains the raw `CVPixelBufferRef`.
   Call `CVPixelBufferGetIOSurface(pixel_buffer)`.
3. **Zero-Copy Metal Interop:**
   Wrap the `IOSurfaceRef` into an `MTLTexture` using Metal API:
   ```objc
   MTLTextureDescriptor* desc = [MTLTextureDescriptor 
       texture2DDescriptorWithPixelFormat:MTLPixelFormatR8Unorm 
       width:width height:height mipmapped:NO];
   id<MTLTexture> tex = [metalDevice newTextureWithDescriptor:desc 
                                                    iosurface:surface 
                                                        plane:0];
   ```
   Import into `wgpu` via `wgpu::Texture::from_hal::<metal::Device>(metal_texture)`.
4. **Fallback:** If `wgpu` fails to initialize the Metal texture, or if CPU image operations are requested, lock the `CVPixelBuffer` base address using `CVPixelBufferLockBaseAddress` and execute an optimized SIMD memory copy into a standard `VideoFrame::data` host vector.

### 4.2 Linux: VAAPI Integration

Linux systems (Intel, AMD) utilize the Video Acceleration API (VAAPI) over Direct Rendering Manager (DRM).

1. **Device Detection:** Enumerate `/dev/dri/renderD*` nodes (default `/dev/dri/renderD128`). Open via DRM and instantiate `AV_HWDEVICE_TYPE_VAAPI`.
2. **Surface Export to DMA-BUF:**
   Query the decoded `VASurfaceID` from `AVFrame`. Call `vaExportSurfaceHandle` to extract a Linux file descriptor (`RawFd`) representing the DRM DMA-BUF backing memory, along with DRM format modifiers and byte stride.
3. **Vulkan Zero-Copy Interop:**
   Use the Vulkan extension `VK_KHR_external_memory_fd` and `VK_EXT_image_drm_format_modifier`. Construct a `VkImage` wrapping the imported DMA-BUF file descriptor. Import the resulting `VkImage` into `wgpu` via `wgpu::Texture::from_hal::<vulkan::Device>`.
4. **Fallback:** Call `av_hwframe_transfer_data` to copy the VAAPI hardware surface into a software `AV_PIX_FMT_NV12` host frame.

### 4.3 Windows: D3D11VA & NVDEC Integration

Windows systems utilize Direct3D 11 Video Acceleration (D3D11VA) and NVIDIA NVDEC.

1. **D3D11VA Setup:** Initialize `AV_HWDEVICE_TYPE_D3D11VA`. Match the DirectX adapter LUID with the physical GPU adapter chosen by `wgpu`.
2. **DXGI Shared Surfaces:**
   Decoded video frames reside in an `ID3D11Texture2D`. Query `IDXGIResource` from the texture pointer and call `IDXGIResource::GetSharedHandle(&shared_handle)`.
3. **Direct3D 12 Interop with wgpu:**
   In `wgpu`'s DX12 backend, access `ID3D12Device::OpenSharedHandle` to import the D3D11 surface directly into a DX12 resource without copying across system RAM.
4. **Fallback:** Use `av_hwframe_transfer_data` to download the texture from DirectX VRAM into a system memory buffer (`AV_PIX_FMT_NV12` or `AV_PIX_FMT_RGBA`).

### 4.4 Hardware Fallback Architecture & Feature Flags

`Cargo.toml` features permit granular compilation targets:

```toml
[features]
default = ["hw-videotoolbox", "hw-vaapi", "hw-d3d11va"]

hw-videotoolbox = []
hw-vaapi = []
hw-nvdec = []
hw-d3d11va = []
```

```rust
// crates/slate-media/src/hw/mod.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HwAccelType {
    None,
    VideoToolbox,
    Vaapi,
    D3d11va,
    Nvdec,
}

pub struct HwDeviceManager;

impl HwDeviceManager {
    /// Detects platform hardware capabilities at runtime
    pub fn probe_supported_accels() -> Vec<HwAccelType> {
        let mut supported = Vec::new();

        #[cfg(all(target_os = "macos", feature = "hw-videotoolbox"))]
        {
            if Self::test_videotoolbox_init() {
                supported.push(HwAccelType::VideoToolbox);
            }
        }

        #[cfg(all(target_os = "linux", feature = "hw-vaapi"))]
        {
            if Self::test_vaapi_init() {
                supported.push(HwAccelType::Vaapi);
            }
        }

        #[cfg(all(target_os = "windows", feature = "hw-d3d11va"))]
        {
            if Self::test_d3d11va_init() {
                supported.push(HwAccelType::D3d11va);
            }
        }

        supported.push(HwAccelType::None); // Software CPU always supported
        supported
    }

    /// Automatically downgraded if hardware decoding fails mid-stream
    pub fn select_best_accel_for_codec(codec: &str) -> HwAccelType {
        let available = Self::probe_supported_accels();
        // Match codec against known HW limits (e.g., ProRes only on VideoToolbox Apple Silicon)
        available.first().copied().unwrap_or(HwAccelType::None)
    }
}
```

> [!IMPORTANT]
> If a hardware decoder returns an unrecoverable packet error (such as corrupt bitstream or GPU hang), the worker automatically logs a warning, flags the media clip as `hw_unsupported`, tears down the hardware context, and opens a software decoder instance (`ffh264`, `ffhevc`, or `dav1d`) without crashing the application.

---

## 5. Proxy Generation

High-resolution media (4K/8K H.264/HEVC/ProRes) strains scrubbing responsiveness. The proxy pipeline generates lightweight 720p H.264 mezzanine files in the background.

### 5.1 Proxy Specifications
- **Target Container:** QuickTime MP4 (`.mp4`)
- **Video Codec:** H.264 (Baseline/Main Profile, Level 3.1)
- **Target Resolution:** Scaled to fit within $1280 \times 720$ while preserving source aspect ratio.
- **Bitrate:** Constant Rate Factor (CRF) 23, preset `veryfast`.
- **GOP Size:** Short fixed GOP of 12 frames (0.5 sec at 24fps) to enable near-instantaneous scrubbing.
- **Audio:** Passthrough (copy) or AAC stereo at 128 kbps.

### 5.2 Storage & Directory Layout

Proxies are stored in the project's internal cache folder:

```
<project_root>/
└── .slate/
    └── proxies/
        ├── a9f83c1e2b4d_720p.mp4
        └── a9f83c1e2b4d.proxy.json
```

The metadata sidecar (`a9f83c1e2b4d.proxy.json`) preserves linkage to the source asset:

```json
{
  "source_path": "/Volumes/Storage/Footage/A001_C001_0612_4K.mov",
  "source_media_id": "a9f83c1e-2b4d-48a1-9b93-524f0c8a14b9",
  "source_hash_blake3": "4e72a8c081297d26859666bb4",
  "proxy_file": "a9f83c1e2b4d_720p.mp4",
  "width": 1280,
  "height": 720,
  "frame_rate": [24000, 1001],
  "status": "Ready",
  "created_at_utc": "2026-10-14T10:15:00Z"
}
```

### 5.3 Proxy Engine API

```rust
// crates/slate-media/src/proxy/mod.rs

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crossbeam_channel::{Receiver, Sender};
use slate_core::id::MediaId;
use crate::error::MediaError;

#[derive(Debug, Clone)]
pub struct ProxyProgress {
    pub media_id: MediaId,
    pub progress_percent: f32, // 0.0 to 100.0
    pub current_frame: u64,
    pub total_frames: u64,
    pub encoding_fps: f32,
    pub eta_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyStatus {
    NotGenerated,
    Queued,
    Generating(u8), // progress percent
    Ready(PathBuf),
    Failed(String),
}

pub struct ProxyManager {
    cache_dir: PathBuf,
    status_map: dashmap::DashMap<MediaId, ProxyStatus>,
}

impl ProxyManager {
    pub fn new(cache_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&cache_dir).ok();
        Self {
            cache_dir,
            status_map: dashmap::DashMap::new(),
        }
    }

    pub fn start_proxy_generation(
        &self,
        media_id: MediaId,
        source_path: &Path,
        progress_tx: Sender<ProxyProgress>,
        cancel_token: Arc<AtomicBool>,
    ) -> Result<(), MediaError> {
        let output_path = self.cache_dir.join(format!("{}_720p.mp4", media_id));
        self.status_map.insert(media_id, ProxyStatus::Generating(0));

        // Spawns background transcoding job using FFmpeg transcoding pipeline
        crate::proxy::generator::spawn_transcode_job(
            media_id,
            source_path.to_path_buf(),
            output_path,
            progress_tx,
            cancel_token,
        );

        Ok(())
    }

    pub fn get_status(&self, media_id: &MediaId) -> ProxyStatus {
        self.status_map
            .get(media_id)
            .map(|r| r.value().clone())
            .unwrap_or(ProxyStatus::NotGenerated)
    }
}
```

### 5.4 Playback Proxy Toggle

The timeline playback controller holds a global setting: `PlaybackResolution::Original` or `PlaybackResolution::Proxy`. When set to `Proxy`, the `DecoderPool` transparently redirects requests for `media_id` to the generated proxy file path if its status is `ProxyStatus::Ready`. Because PTS timestamps and frame indices match identically, timeline playback requires zero re-calculation of edit cuts or transition points.

---

## 6. Variable Frame Rate (VFR) to Constant Frame Rate (CFR) Conforming

Footage originating from smartphones (iPhone/Android), webcams, and screen capture tools (OBS, Zoom) often features Variable Frame Rate (VFR). VFR causes progressive audio/video desynchronization, audio popping, and stuttering on non-linear editing timelines.

### 6.1 VFR Detection Algorithm

During `probe_media`, the probe engine analyzes packet presentation timestamps (PTS) over the first 300 video frames:

$$\Delta \text{PTS}_i = \text{PTS}_i - \text{PTS}_{i-1}$$

The jitter metric $J$ is calculated as the standard deviation of $\Delta \text{PTS}$ divided by the mean interval:

$$J = \frac{\sigma(\Delta \text{PTS})}{\mu(\Delta \text{PTS})}$$

- If $J \le 0.01$ (less than 1% variance): Marked as **CFR**.
- If $J > 0.01$: Marked as **VFR** (`is_vfr = true`).

### 6.2 CFR Conforming Architecture

When VFR is detected, the user is offered automatic conforming, or the editor can conform on-the-fly during proxy generation.

```
+---------------------------------------------------------------+
|                       Source VFR Stream                       |
+---------------------------------------------------------------+
                               |
                               v
+---------------------------------------------------------------+
|               FFmpeg Filter Graph: Conforming                 |
|                                                               |
|  [video_in] ---> fps=fps=30:round=near ---> [video_out]       |
|                                                               |
|  [audio_in] ---> aresample=async=1000 ---> [audio_out]        |
+---------------------------------------------------------------+
                               |
                               v
+---------------------------------------------------------------+
|               Output Constant Frame Rate Stream               |
|      (Sample-accurate audio sync, perfectly fixed PTS deltas)  |
+---------------------------------------------------------------+
```

1. **Target Frame Rate Selection:** The engine selects the nearest standard CFR: $23.976, 24.0, 25.0, 29.97, 30.0, 50.0, 59.94, 60.0$.
2. **Video Filter Graph:**
   - Filter `fps=fps={target_fps}:round=near`: Duplicates or drops video frames as mathematically required to ensure strictly equidistant frame boundaries.
   - Filter `setpts=N/(FPS*TB)`: Resets PTS to monotonic, evenly spaced ticks.
3. **Audio Filter Graph:**
   - Filter `aresample=async=1000:min_hard_comp=0.100000:first_pts=0`: Dynamically stretches or squeezes audio samples slightly (sub-perceptual) to compensate for drifting master clocks, preventing lip-sync drift over multi-hour clips.

---

## 7. Frame Cache Architecture

High scrubbing fluidity requires sub-5-millisecond frame delivery. The `FrameCache` provides a multi-threaded, memory-budgeted LRU cache.

```
                               Cache Query: (MediaId, StreamIdx, FrameNum, Resolution)
                                                        |
                                                        v
                                        +-------------------------------+
                                        |    DashMap Lookup (Key)       |
                                        +-------------------------------+
                                          /                           \
                                 Hit     /                             \ Miss
                                        v                               v
                        +-------------------------------+       +-------------------------------+
                        | Return Arc<CachedFrame>       |       | Dispatch to DecoderPool       |
                        | Update LRU List Order         |       | Insert into Cache on Decode   |
                        +-------------------------------+       | Enforce Memory Budget Limit   |
                                                                +-------------------------------+
```

### 7.1 Cache Specifications
- **Default Budget:** $512\text{ MB}$ (configurable up to $16\text{ GB}$ in user preferences).
- **Key Representation:** `CacheKey { media_id: MediaId, stream_index: usize, frame_number: i64, resolution: FrameResolution }`.
- **Thread Safety:** Lock-free reads via `DashMap` for frame lookups; eviction tracking synchronized via `parking_lot::Mutex<LruTracker>`.
- **Cost Metric:** Weighted by raw byte footprint (`VideoFrame::memory_size_bytes()`).

```rust
// crates/slate-media/src/cache/mod.rs

use std::sync::Arc;
use dashmap::DashMap;
use parking_lot::Mutex;
use slate_core::id::MediaId;
use crate::frame::VideoFrame;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameResolution {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub media_id: MediaId,
    pub stream_index: usize,
    pub frame_number: i64,
    pub resolution: FrameResolution,
}

pub struct CachedFrame {
    pub frame: VideoFrame,
    pub byte_size: usize,
}

pub struct FrameCache {
    max_memory_bytes: usize,
    current_memory_bytes: std::sync::atomic::AtomicUsize,
    map: DashMap<CacheKey, Arc<CachedFrame>>,
    lru_order: Mutex<lru::LruCache<CacheKey, usize>>, // Key -> size
}

impl FrameCache {
    pub fn new(max_memory_bytes: usize) -> Self {
        Self {
            max_memory_bytes,
            current_memory_bytes: std::sync::atomic::AtomicUsize::new(0),
            map: DashMap::new(),
            lru_order: Mutex::new(lru::LruCache::unbounded()),
        }
    }

    pub fn get(&self, key: &CacheKey) -> Option<VideoFrame> {
        if let Some(entry) = self.map.get(key) {
            let mut lru = self.lru_order.lock();
            lru.get(key); // Touch entry to update recency
            Some(entry.value().frame.clone())
        } else {
            None
        }
    }

    pub fn insert(&self, key: CacheKey, frame: VideoFrame) {
        let size = frame.memory_size_bytes();
        let cached = Arc::new(CachedFrame {
            frame,
            byte_size: size,
        });

        // Insert into concurrent map
        self.map.insert(key.clone(), cached);
        self.current_memory_bytes.fetch_add(size, std::sync::atomic::Ordering::Relaxed);

        // Update LRU and evict if over budget
        let mut lru = self.lru_order.lock();
        lru.put(key, size);

        while self.current_memory_bytes.load(std::sync::atomic::Ordering::Relaxed) > self.max_memory_bytes {
            if let Some((evicted_key, evicted_size)) = lru.pop_lru() {
                self.map.remove(&evicted_key);
                self.current_memory_bytes.fetch_sub(evicted_size, std::sync::atomic::Ordering::Relaxed);
            } else {
                break;
            }
        }
    }

    pub fn invalidate_media(&self, media_id: &MediaId) {
        self.map.retain(|key, val| {
            if key.media_id == *media_id {
                self.current_memory_bytes.fetch_sub(val.byte_size, std::sync::atomic::Ordering::Relaxed);
                false
            } else {
                true
            }
        });
        let mut lru = self.lru_order.lock();
        lru.clear(); // Rebuilds automatically on subsequent reads
    }
}
```

---

## 8. Audio Waveform Generation

Timeline tracks require instant visual rendering of audio peaks across multiple zoom levels (from whole-clip overview to individual sample accuracy).

### 8.1 Waveform Processing Pipeline

```
+---------------------------------------------------------------+
|                       Audio Stream Data                       |
|                 (Decoded at 48 kHz / 32-bit Float)            |
+---------------------------------------------------------------+
                               |
                               v
+---------------------------------------------------------------+
|            Waveform Bin Accumulator (Level 0: 256 samples)    |
|   - Compute Peak Min: min(sample[i])                          |
|   - Compute Peak Max: max(sample[i])                          |
|   - Compute RMS:      sqrt(sum(sample[i]^2) / N)              |
+---------------------------------------------------------------+
                               |
             +-----------------+-----------------+
             |                                   |
             v                                   v
+-------------------------+         +-------------------------+
| Mipmap Level 1 (1024)   |         | Mipmap Level 2 (4096)   |
| (Aggregated from L0)    |         | (Aggregated from L1)    |
+-------------------------+         +-------------------------+
             |
             v
+---------------------------------------------------------------+
|             Binary Serialization: `.slate-wave`              |
+---------------------------------------------------------------+
```

### 8.2 Binary Cache File Format (`.slate-wave`)

The binary format uses little-endian byte ordering for direct memory-mapped access (`mmap`):

```
+-------------------------------------------------------------------------+
| Header (32 bytes):                                                      |
|   [0..4]   Magic Bytes: "SLTW" (0x53, 0x50, 0x4C, 0x57)                |
|   [4..6]   Version: 1 (u16)                                             |
|   [6..8]   Channels: (u16, e.g. 2)                                      |
|   [8..12]  Sample Rate: (u32, e.g. 48000)                              |
|   [12..20] Total PCM Frames: (u64)                                      |
|   [20..24] Mipmap Count: (u32, default 4)                               |
|   [24..32] Reserved (8 bytes zero-filled)                               |
+-------------------------------------------------------------------------+
| Mipmap Index Table (Mipmap Count * 16 bytes):                           |
|   - Offset from start of file (u64)                                     |
|   - Bin sample count (u32: 256, 1024, 4096, 16384)                     |
|   - Bin record count (u32)                                              |
+-------------------------------------------------------------------------+
| Data Section:                                                           |
|   Repeated records of WaveformBin:                                      |
|     struct WaveformBin {                                                |
|         min_sample: f32,                                                |
|         max_sample: f32,                                                |
|         rms: f32,                                                       |
|     }                                                                   |
|   (Grouped per channel per bin)                                         |
+-------------------------------------------------------------------------+
```

```rust
// crates/slate-media/src/audio/waveform.rs

use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::error::MediaError;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaveformBin {
    pub min_sample: f32,
    pub max_sample: f32,
    pub rms: f32,
}

pub struct WaveformOverview {
    pub channels: u16,
    pub sample_rate: u32,
    pub total_samples: u64,
    pub levels: Vec<WaveformLevel>,
}

pub struct WaveformLevel {
    pub samples_per_bin: u32,
    pub bins_per_channel: Vec<Vec<WaveformBin>>,
}

pub fn generate_waveform_cache(
    audio_path: &Path,
    output_cache_path: &Path,
) -> Result<WaveformOverview, MediaError> {
    // 1. Open audio decoder stream
    // 2. Compute Level 0 (256 samples/bin)
    // 3. Hierarchically aggregate into Levels 1, 2, 3
    // 4. Write binary file with SLTW header
    todo!()
}
```

---

## 9. Thumbnail Generation

The thumbnail engine supplies visual previews for project media bins and timeline clip representations.

### 9.1 I-Frame Extraction Strategy
- Rather than decoding every single frame sequentially up to the target timestamp, the thumbnail generator queries FFmpeg packet flags for `AV_PKT_FLAG_KEY`.
- Only keyframe packets are decoded, allowing thumbnail generation at over 500 frames per second on typical SSD media.

### 9.2 Contact Sheet Generation
- Evenly samples $N$ points along the clip duration:

$$T_k = \frac{k}{N} \times \text{Duration}, \quad k \in [0, N-1]$$

- Decodes the nearest frame and renders into a standardized $160 \times 90$ pixel JPEG.
- Stitched into a single composite contact sheet image (`filmstrip.jpg`) for timeline scrubbing preview with zero individual file I/O overhead.

```rust
// crates/slate-media/src/thumbnail.rs

use std::path::{Path, PathBuf};
use slate_core::time::RationalTime;
use crate::error::MediaError;

pub struct ThumbnailGenerator;

impl ThumbnailGenerator {
    /// Extracts a single representative frame at target timestamp
    pub fn extract_frame(
        media_path: &Path,
        time: RationalTime,
        target_width: u32,
    ) -> Result<image::RgbaImage, MediaError> {
        todo!()
    }

    /// Generates an N-frame contact sheet filmstrip stored on disk
    pub fn generate_filmstrip(
        media_path: &Path,
        output_image: &Path,
        frame_count: usize,
        thumb_height: u32,
    ) -> Result<(), MediaError> {
        todo!()
    }
}
```

---

## 10. Video & Audio Encoder

The `Encoder` struct orchestrates the export pipeline, accepting rendered frames from the compositor (`wgpu`) and interleaved audio buffers from the audio mixing bus, encoding them with hardware acceleration where available, and muxing them into the final destination container.

```
+------------------------------------+      +------------------------------------+
|       Video Compositor (wgpu)      |      |          Audio Mixer Bus           |
|         Rendered VideoFrame        |      |            AudioBuffer             |
+------------------------------------+      +------------------------------------+
                  |                                           |
                  v send_video_frame()                        v send_audio_buffer()
+--------------------------------------------------------------------------------+
|                                    Encoder                                     |
|                                                                                |
|  +--------------------------------+       +---------------------------------+  |
|  | Video Encoder (HW / SW)        |       | Audio Encoder (AAC / PCM)       |  |
|  | - VideoToolbox / NVENC / VAAPI |       | - 48kHz, 24-bit / 320k AAC      |  |
|  | - libx264 / libx265 / prores   |       |                                 |  |
|  +--------------------------------+       +---------------------------------+  |
|                 |                                          |                   |
|                 v Encoded AVPacket                         v Encoded AVPacket  |
|  +--------------------------------------------------------------------------+  |
|  |                           AVFormat Muxer                                 |  |
|  |  - Interleaves audio and video packets monotonically                     |  |
|  |  - Writes container header, moov atom, and finishes output file          |  |
|  +--------------------------------------------------------------------------+  |
+--------------------------------------------------------------------------------+
                                         |
                                         v
                         Destination File (.mp4 / .mov)
```

### 10.1 Export Presets & Configuration

```rust
// crates/slate-media/src/encoder/presets.rs

#[derive(Debug, Clone)]
pub enum ExportContainer {
    Mp4,
    Mov,
    Wav,
}

#[derive(Debug, Clone)]
pub enum VideoExportCodec {
    H264 { crf: u8, preset: String },
    H265 { crf: u8, preset: String },
    ProRes { profile: ProResProfile },
}

#[derive(Debug, Clone)]
pub enum ProResProfile {
    Proxy,
    Lt,
    Standard422,
    Hq422,
    Profile4444,
}

#[derive(Debug, Clone)]
pub enum AudioExportCodec {
    Aac { bitrate_kbps: u32 },
    Pcm16,
    Pcm24,
    Pcm32Float,
}

#[derive(Debug, Clone)]
pub struct ExportPreset {
    pub name: String,
    pub container: ExportContainer,
    pub video_codec: Option<VideoExportCodec>,
    pub audio_codec: AudioExportCodec,
    pub width: u32,
    pub height: u32,
    pub fps_num: u32,
    pub fps_den: u32,
}

impl ExportPreset {
    pub fn youtube_4k() -> Self {
        Self {
            name: "YouTube 4K (H.264)".to_string(),
            container: ExportContainer::Mp4,
            video_codec: Some(VideoExportCodec::H264 { crf: 18, preset: "slow".to_string() }),
            audio_codec: AudioExportCodec::Aac { bitrate_kbps: 320 },
            width: 3840,
            height: 2160,
            fps_num: 60,
            fps_den: 1,
        }
    }

    pub fn prores_master_1080p() -> Self {
        Self {
            name: "ProRes 422 HQ 1080p Master".to_string(),
            container: ExportContainer::Mov,
            video_codec: Some(VideoExportCodec::ProRes { profile: ProResProfile::Hq422 }),
            audio_codec: AudioExportCodec::Pcm24,
            width: 1920,
            height: 1080,
            fps_num: 24,
            fps_den: 1,
        }
    }
}
```

### 10.2 Encoder Engine Implementation

```rust
// crates/slate-media/src/encoder/mod.rs

use std::path::Path;
use crossbeam_channel::{Receiver, Sender};
use slate_core::time::RationalTime;
use crate::error::MediaError;
use crate::frame::{AudioBuffer, VideoFrame};
use crate::encoder::presets::ExportPreset;

#[derive(Debug, Clone)]
pub struct ExportProgress {
    pub percent: f32,
    pub current_video_frame: u64,
    pub total_video_frames: u64,
    pub elapsed_seconds: f64,
    pub estimated_remaining_seconds: f64,
    pub fps: f32,
}

pub struct Encoder {
    preset: ExportPreset,
    output_path: std::path::PathBuf,
    video_frames_encoded: u64,
    audio_samples_encoded: u64,
    // ffmpeg internal contexts are encapsulated within worker state
}

impl Encoder {
    pub fn start(
        output_path: &Path,
        preset: ExportPreset,
        total_frames: u64,
    ) -> Result<(Self, Receiver<ExportProgress>), MediaError> {
        // Initializes AVFormatContext for container muxing
        // Initializes AVCodecContext for video and audio
        // Spawns monitoring thread to send ExportProgress
        todo!()
    }

    /// Pushes a rendered video frame into the encoder pipeline
    pub fn send_video_frame(&mut self, frame: VideoFrame) -> Result<(), MediaError> {
        todo!()
    }

    /// Pushes an audio buffer into the resampler & encoder pipeline
    pub fn send_audio_buffer(&mut self, audio: AudioBuffer) -> Result<(), MediaError> {
        todo!()
    }

    /// Flushes encoder internal queues, writes container trailer, and closes file atomically
    pub fn finish(self) -> Result<(), MediaError> {
        todo!()
    }
}
```

---

## 11. FFmpeg Binding Safety & Memory Management

FFmpeg is a massive C library. Interfacing with it from safe Rust requires disciplined architectural boundaries.

### 11.1 Global Initialization
FFmpeg must only be initialized once per application process lifetime:

```rust
// crates/slate-media/src/lib.rs

use std::sync::Once;

static FFMPEG_INIT: Once = Once::new();

pub fn init_ffmpeg() {
    FFMPEG_INIT.call_once(|| {
        ffmpeg::init().expect("Failed to initialize FFmpeg subsystem");
        // Suppress noisy FFmpeg internal logging or redirect to tracing crate
        ffmpeg::util::log::set_level(ffmpeg::util::log::level::Level::Warning);
    });
}
```

### 11.2 Thread Safety Boundaries
- `AVFormatContext` and `AVCodecContext` structures are **NOT** thread-safe and must never be shared across thread boundaries without external mutual exclusion.
- In `slate-media`, contexts are strictly thread-affine: a `DecoderWorker` creates, manipulates, and destroys its own contexts. No pointers are shared across channels. Only thread-safe, owned Rust structures (`VideoFrame`, `AudioBuffer`) cross channel boundaries.

### 11.3 Buffer Pooling to Prevent Heap Fragmentation
Continuously allocating and deallocating $3840 \times 2160 \times 4\text{ bytes} \approx 33\text{ MB}$ per frame at 60 fps causes extreme memory fragmentation and triggers garbage collection stalls in allocator metadata.

```
                    +------------------------------------+
                    |         BufferPool (Lockfree)      |
                    |  - Preallocated Vec<u8> buffers    |
                    |  - Capacity: 32 frames             |
                    +------------------------------------+
                        /                            ^
            Acquire    /                              \ Recycled on Arc drop
                      v                                \
          +------------------------+        +------------------------+
          | Decoded VideoFrame     | -----> | Arc::drop(data)        |
          +------------------------+        +------------------------+
```

An RAII buffer pool recycles frame vectors when their reference count drops to zero.

### 11.4 LGPL Dynamic Linking Compliance
To strictly adhere to FFmpeg's LGPL v2.1/v3 license:
1. `slate-media` builds against dynamically linked shared libraries (`libavcodec.dylib`/`.so`/`.dll`, `libavformat`, etc.).
2. No proprietary closed-source code is statically compiled into the FFmpeg binary objects.
3. Users and downstream packaging systems can substitute their own compatible dynamic builds of FFmpeg without recompiling the Slate executable.

---

## 12. Testing & Quality Assurance Plan

### 12.1 Test Suite Breakdown

| Test Category | Target Module | Test Description | Verification Criterion |
| :--- | :--- | :--- | :--- |
| **Probe Tests** | `src/probe.rs` | Probe test fixtures across H.264, HEVC, ProRes, AV1, MP3, WAV | Stream count, dimensions, frame rate, duration match within 1 microsecond |
| **Seek Accuracy** | `src/decoder/seek.rs` | Seek to 50 pseudo-random frames in long-GOP 4K H.264 clip | Decoded frame PTS strictly matches expected requested PTS without drift |
| **GOP Discard** | `src/decoder/seek.rs` | Verify non-target frames before target PTS do not invoke pixel conversion | CPU profile confirms zero `swscale` calls for discarded B/P frames |
| **Hardware Probe** | `src/hw/mod.rs` | Detect available HW accelerators on target machine | Gracefully identifies VideoToolbox/VAAPI/D3D11VA or safely returns `None` |
| **Roundtrip Encode** | `src/encoder/` | Generate synthetic SMPTE color bars, encode to MP4, decode back | PSNR $> 42\text{ dB}$, SSIM $> 0.98$ against raw reference frames |
| **VFR Detection** | `src/vfr/mod.rs` | Analyze synthetic VFR clip generated with varying frame delays | Accurately identifies `is_vfr = true` and calculates jitter within 2% margin |
| **Audio Waveform** | `src/audio/waveform.rs`| Generate `.slate-wave` from 1 kHz reference sine wave | Peak values equal $1.0$, RMS equals $\frac{1}{\sqrt{2}} \approx 0.707$ across all bins |
| **Memory Leak** | `src/cache/lru.rs` | Run 10,000 seek iterations in a tight loop | Heap allocation remains strictly within specified cache budget ($512\text{ MB}$) |

### 12.2 Synthetic Test Clip Generation
Test fixtures are programmatically generated via an offline shell script or Rust build test-helper using the FFmpeg CLI:

```bash
# 1. Standard CFR 1080p 24fps H.264 Test Pattern
ffmpeg -y -f lavfi -i testsrc=duration=10:size=1920x1080:rate=24 \
       -f lavfi -i sine=frequency=1000:duration=10:sample_rate=48000 \
       -c:v libx264 -g 24 -pix_fmt yuv420p -c:a aac \
       tests/fixtures/test_cfr_1080p24.mp4

# 2. Variable Frame Rate (VFR) Test Clip
ffmpeg -y -f lavfi -i testsrc=duration=5:size=1280x720:rate=30 \
       -vf "setpts='if(eq(N,0),0,PTS+1/30*TB*(1+0.5*sin(N)))'" \
       -c:v libx264 tests/fixtures/test_vfr_720p.mp4
```

---

## 13. Step-by-Step Implementation Checklist

This checklist defines the complete development progression for implementing `crates/slate-media`.

### Phase 1: Foundation & Safe FFI Layer
- [ ] **1.1 Crate Scaffold:**
  - [ ] Initialize `crates/slate-media/Cargo.toml` with dependencies (`ffmpeg-next`, `crossbeam-channel`, `dashmap`, `parking_lot`, `thiserror`, `image`, `byteorder`, `tracing`).
  - [ ] Define feature flags: `hw-videotoolbox`, `hw-vaapi`, `hw-nvdec`, `hw-d3d11va`.
- [ ] **1.2 Error Handling (`src/error.rs`):**
  - [ ] Implement `MediaError` enum (`OpenFailed`, `StreamNotFound`, `CodecNotFound`, `DecodeError`, `SeekError`, `EncodeError`, `UnsupportedFormat`, `FfmpegError`).
  - [ ] Implement `From<ffmpeg::Error>` for `MediaError`.
- [ ] **1.3 Initialization & Global State (`src/lib.rs`, `src/ffi_safety.rs`):**
  - [ ] Implement `init_ffmpeg()` with `std::sync::Once`.
  - [ ] Implement RAII buffer allocator pool for reusable frame buffers.

### Phase 2: Metadata Probing & Frame Types
- [ ] **2.1 Frame Representations (`src/frame.rs`):**
  - [ ] Implement `PixelFormat`, `ColorSpace`, and `ColorRange` enums.
  - [ ] Implement `HwTextureHandle` for platform-specific handles.
  - [ ] Implement `VideoFrame` struct with `memory_size_bytes()`.
  - [ ] Implement `AudioBuffer` struct with duration and sample count calculations.
- [ ] **2.2 Media Prober (`src/probe.rs`):**
  - [ ] Implement `probe_media(path: &Path) -> Result<MediaProbeResult, MediaError>`.
  - [ ] Extract duration, video stream dimensions, SAR/DAR, nominal frame rate, bit depth.
  - [ ] Extract audio stream channel count, layout, and sample rate.
  - [ ] Implement VFR jitter analysis over the first 300 packet timestamps.

### Phase 3: Hardware Acceleration Engine
- [ ] **3.1 Hardware Device Manager (`src/hw/mod.rs`):**
  - [ ] Implement `HwDeviceManager::probe_supported_accels()`.
  - [ ] Implement automatic fallback strategy if hardware initialization fails.
- [ ] **3.2 Platform Implementations:**
  - [ ] `src/hw/videotoolbox.rs`: macOS `CVPixelBufferRef` to `IOSurface` extraction and `wgpu` Metal texture binding.
  - [ ] `src/hw/vaapi.rs`: Linux VAAPI DRM node discovery and DMA-BUF export.
  - [ ] `src/hw/d3d11va.rs`: Windows D3D11VA DXGI shared texture handle export.
  - [ ] `src/hw/fallback.rs`: CPU software fallback via `ffmpeg::software::scaling::Context`.

### Phase 4: Decoder Worker Pool & Seeking Engine
- [ ] **4.1 Frame Cache (`src/cache/`):**
  - [ ] Implement `CacheKey` and `CachedFrame`.
  - [ ] Implement `FrameCache` with thread-safe `DashMap` and byte-bounded `lru::LruCache`.
  - [ ] Implement cache invalidation on proxy toggle or media unload.
- [ ] **4.2 Seeking Engine (`src/decoder/seek.rs`):**
  - [ ] Implement GOP-aware backward keyframe seek with `avformat_seek_file`.
  - [ ] Implement forward packet decode loop with fast frame discard for non-target PTS.
  - [ ] Implement intra-GOP local ring-buffer for reverse scrubbing optimization.
- [ ] **4.3 Worker & Pool (`src/decoder/worker.rs`, `src/decoder/pool.rs`):**
  - [ ] Implement `DecoderWorker` run loop processing `DecodeRequest` items.
  - [ ] Implement `DecoderPool` worker allocation based on CPU core count.
  - [ ] Implement atomic cancellation token checking to discard stale scrub requests.
- [ ] **4.4 Playback Prefetch (`src/decoder/prefetch.rs`):**
  - [ ] Implement directional prefetch queue (+24 frames forward during playback).

### Phase 5: Audio & Utility Pipelines
- [ ] **5.1 Audio Decoder & Resampler (`src/audio/mod.rs`):**
  - [ ] Implement audio packet demuxing and conversion to 32-bit float interleaved PCM.
  - [ ] Implement resampler using `ffmpeg::software::resampling::Context`.
- [ ] **5.2 Waveform Generator (`src/audio/waveform.rs`):**
  - [ ] Implement multi-resolution peak/RMS accumulator.
  - [ ] Implement binary serialization and deserialization for `.slate-wave` files.
- [ ] **5.3 Thumbnail Generator (`src/thumbnail.rs`):**
  - [ ] Implement fast I-frame packet seeker.
  - [ ] Implement contact sheet grid stitcher outputting standardized JPEG previews.
- [ ] **5.4 VFR Conformer (`src/vfr/conformer.rs`):**
  - [ ] Construct FFmpeg filter graph with `fps` and `aresample` filters to conform VFR media to standard CFR.
- [ ] **5.5 Proxy Pipeline (`src/proxy/`):**
  - [ ] Implement `ProxyManager` tracking background generation status.
  - [ ] Implement 720p H.264 low-overhead background transcode job.
  - [ ] Write `.proxy.json` metadata sidecar.

### Phase 6: Encoding & Export Pipeline
- [ ] **6.1 Export Presets (`src/encoder/presets.rs`):**
  - [ ] Define standard export profiles (YouTube 4K, 1080p H.264, ProRes 422 Master, Instagram 9:16).
- [ ] **6.2 Video Encoder (`src/encoder/video_encoder.rs`):**
  - [ ] Initialize hardware-accelerated video encoder (`h264_videotoolbox`, `h264_nvenc`, `h264_vaapi`) with CPU fallback (`libx264`).
- [ ] **6.3 Audio Encoder (`src/encoder/audio_encoder.rs`):**
  - [ ] Initialize AAC and PCM encoders.
- [ ] **6.4 Container Muxer (`src/encoder/muxer.rs`):**
  - [ ] Implement packet interleaver and container header/trailer writer.
- [ ] **6.5 Export Session Coordinator (`src/encoder/mod.rs`):**
  - [ ] Implement `Encoder::start`, `send_video_frame`, `send_audio_buffer`, and `finish`.
  - [ ] Implement progress reporting channel sending `ExportProgress`.

### Phase 7: Verification & Benchmarking
- [ ] **7.1 Unit Tests:**
  - [ ] Verify probe accuracy against synthetic test clips.
  - [ ] Verify seek accuracy across H.264 and HEVC GOP boundaries.
  - [ ] Verify waveform generator output on sine wave audio.
- [ ] **7.2 Integration Tests:**
  - [ ] Roundtrip encode and decode test verifying PSNR $> 42\text{ dB}$.
  - [ ] Conforming test verifying VFR audio-video sync preservation.
- [ ] **7.3 Performance Benchmarks:**
  - [ ] Measure decode throughput (FPS) in 4K H.264 scrub mode.
  - [ ] Measure cache hit latency ($< 1\text{ ms}$ on RAM hit).
