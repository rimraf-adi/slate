# Slate: Rust Video Editor + Built-in MCP Server

> A frame-accurate non-linear video editor written entirely in Rust, with a built-in Model Context Protocol (MCP) server that empowers AI agents to collaboratively edit alongside humans on the same live timeline.

---

## Architecture & System Overview

Slate is built on a headless engine architecture where **`slate-core`** owns the timeline data model, rational time calculations, and every state mutation through an invertible Command pattern (`Op`). Both the desktop GUI (`slate-ui`) and the MCP server (`slate-mcp`) act as thin clients on top of this engine, sharing a unified linear undo/redo history.

```
+-------------------------------------------------------------+
|                        Clients                              |
|   +-------------------+              +------------------+   |
|   | Desktop GUI (UI)  |              |   slate-mcp     |   |
|   |  (egui + wgpu)    |              |  (stdio / HTTP)  |   |
|   +---------+---------+              +--------+---------+   |
|             |                                 |             |
|             |       JSON-RPC (IPC Socket)     |             |
|             +----------------+----------------+             |
+------------------------------|------------------------------+
                               v
+-------------------------------------------------------------+
|                         Engine                              |
|                   +------------------+                      |
|                   |   slate-core    |                      |
|                   | (Timeline / Ops) |                      |
|                   +--------+---------+                      |
|                            |                                |
|        +-----------+-------+-------+-----------+            |
|        v           v               v           v            |
|   slate-media slate-render  slate-audio slate-analysis  |
|  (ffmpeg-next)    (wgpu)        (cpal)    (whisper/VAD/etc) |
+-------------------------------------------------------------+
```

---

## Full v1 Engineering Specifications

All features ship in v1 with zero deferrals. The complete system design, math, data models, WGSL shaders, RPC protocols, and implementation checklists are detailed across 17 comprehensive specification documents in [`docs/`](./docs/):

| # | Specification Document | Focus Area | Line Count |
|---|------------------------|------------|------------|
| 00 | [`00-master-plan.md`](./docs/00-master-plan.md) | Workspace layout, crate boundaries, 4-phase build order, ADRs | 639 |
| 01 | [`01-core-engine.md`](./docs/01-core-engine.md) | `slate-core`: `RationalTime`, OTIO data models, `Op` enum, WAL journal | 2,688 |
| 02 | [`02-media-pipeline.md`](./docs/02-media-pipeline.md) | `slate-media`: FFmpeg decode/encode, HW accel (VideoToolbox/VAAPI/NVDEC), proxies | 1,419 |
| 03 | [`03-render-pipeline.md`](./docs/03-render-pipeline.md) | `slate-render`: Unified wgpu compositor, preview/export parity, WGSL shaders | 1,809 |
| 04 | [`04-audio-pipeline.md`](./docs/04-audio-pipeline.md) | `slate-audio`: Real-time cpal mixer, A/V sync master clock, rubato, ebur128 | 1,648 |
| 05 | [`05-analysis-engine.md`](./docs/05-analysis-engine.md) | `slate-analysis`: Local Whisper STT, Silero VAD, scene cuts, contact sheets | 1,906 |
| 06 | [`06-mcp-server.md`](./docs/06-mcp-server.md) | `slate-mcp`: 26 MCP tools, resources, prompts, stdio + HTTP/SSE, agent guardrails | 2,060 |
| 07 | [`07-ipc-protocol.md`](./docs/07-ipc-protocol.md) | `slate-ipc`: JSON-RPC 2.0 local socket / named pipe, event push subscriptions | 1,913 |
| 08 | [`08-desktop-ui.md`](./docs/08-desktop-ui.md) | `slate-ui`: egui/eframe desktop UI, virtual canvas timeline, Agent Activity Panel | 1,440 |
| 09 | [`09-cli.md`](./docs/09-cli.md) | `slate-cli`: Single `slate` binary, headless batch render, CI/CD pipelines | 1,426 |
| 10 | [`10-project-format.md`](./docs/10-project-format.md) | `.slateproj` folder bundle, CRC32 WAL journal, atomic saves, media relinking | 2,034 |
| 11 | [`11-effects-system.md`](./docs/11-effects-system.md) | WGSL shaders (color, LUT, blur, chroma key), plugin sandbox, Bezier curves | 1,894 |
| 12 | [`12-otio-interop.md`](./docs/12-otio-interop.md) | OpenTimelineIO bidirectional import/export, DaVinci/Premiere interchange | 1,711 |
| 13 | [`13-testing-qa.md`](./docs/13-testing-qa.md) | 5-tier testing pyramid, proptest invariants, golden frames (PSNR ≥ 45dB), 20-task eval | 1,639 |
| 14 | [`14-performance.md`](./docs/14-performance.md) | 1080p30 / 4K budgets, <100ms scrub, <16ms op benchmark, zero-copy GPU staging | 1,302 |
| 15 | [`15-security-privacy.md`](./docs/15-security-privacy.md) | 100% offline privacy, CSPRNG bearer tokens, filesystem jail, 25% edit confirmation | 1,293 |
| 16 | [`16-build-distribution.md`](./docs/16-build-distribution.md) | macOS (.dmg), Windows (.msi), Linux (.deb/AppImage), dynamic LGPL FFmpeg | 2,022 |
| **Total** | | **Comprehensive v1 Blueprint** | **28,843 lines** |

---

## Key Technical Decisions (ADRs)

1. **Exact Rational Time Math (`ADR-001`)**: All timeline time calculations utilize `num_rational::Rational64` ticks over an integer timebase. Eliminates floating-point accumulation errors and drift across non-standard or fractional rates (23.976, 29.97, 59.94).
2. **Deterministic Command Pattern (`ADR-002`)**: Every timeline modification is an instance of `Op` with a mathematical `OpInverse`. All mutations from human mouse drags or autonomous MCP agent calls enter the same transactional history.
3. **Unified Single Render Path (`ADR-003`)**: Preview in `slate-ui` and final file export in `slate-cli` execute the identical `wgpu` pipeline and WGSL shaders. Guarantees golden-frame parity (PSNR $\ge 45\text{ dB}$, SSIM $\ge 0.999$).
4. **Local Socket Engine IPC (`ADR-004`)**: `slate-mcp` connects to running GUI sessions via Unix domain sockets (`/tmp/slate-*.sock`) or Windows Named Pipes (`\\.\pipe\slate-*`), allowing live ghost-cursor tracking and zero-latency timeline updates.
5. **1:1 MCP Tool-to-Op Reflection (`ADR-005`)**: The 26 MCP tools map 1:1 onto engine operations and perception queries. Types derived with `schemars` automatically compile into MCP JSON Schemas to prevent drift.
6. **Self-Contained Directory Bundle (`ADR-006`)**: `.slateproj` bundles package structural JSON metadata alongside transient caches (waveforms, 720p proxies, Whisper transcripts) and an append-only WAL journal (`autosave.journal`) ensuring `kill -9` crash resistance.
7. **Audio as Master Sync Clock (`ADR-007`)**: The `cpal` low-latency real-time callback drives the master presentation clock; the video compositor slaves to audio sample position, dropping or repeating frames only when hardware limits require it.
8. **VFR to CFR Conform on Ingest (`ADR-008`)**: Variable frame rate mobile recordings are detected on probe and conformed to Constant Frame Rate streams to prevent long-form audio/video drift.
