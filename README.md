# CISO-RS

High-performance CISO (PSP) compression, decompression and validation, written in Rust.

This project focuses on:

- Blazing fast performance (3s to compress 1.5GiB on a mid-tier desktop)
- Format correctness before everything else
- High throughput even on large images
- Minimal allocations and I/O overhead

It provides both:

- a CLI tool (`ciso`)
- a library crate for direct integration

## Features

- Fast CISO compression with block-level parallelism
- High-throughput decompression optimized for sequential access
- Strict CISO structure validation
- Optional full zlib integrity checking
- No per-block heap allocations in hot paths
- Designed for emulator-grade workloads

## Usage

```
Usage:
ciso <input.iso> [output.cso] [--level 1..9 | --fast | --optimal | --best]
ciso <input.cso> [output.iso]
ciso <input.cso> --check [--full]

Rules:
.iso → compress
.cso → decompress

Defaults:
compress level = 6 (--optimal)
```

## Library

The core logic is available as a Rust library:

- `compress_ciso`
- `decompress_ciso`
- `check_ciso`

The library exposes the same guarantees as the CLI and is suitable for:

- emulators
- tooling
- batch processing
- integration into larger pipelines

## Validation modes

- Fast path: assumes structurally valid input (maximum performance)
- Check mode: validates index monotonicity, bounds and layout
- Full check: additionally validates every compressed block via zlib

Use `--check --full` when correctness matters more than speed.

## Non-goals

- Supporting malformed, non-standard CISO variants, or V2 (yet)
- Hiding format invariants behind abstractions
- Sacrificing performance for defensive checks in hot paths

If you need strict validation, use `check_ciso`.
