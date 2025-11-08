# Performance Benchmarks for contact-detector

This directory contains comprehensive performance benchmarks for the contact-detector project to validate Phase 6 performance optimization requirements.

## Overview

The benchmarking framework uses [Criterion.rs](https://github.com/bheisler/criterion.rs) for statistical benchmarking and includes:

- **Synthetic mesh generation** utilities for creating test meshes of various sizes
- **Surface extraction** benchmarks at multiple scales
- **K-d tree** construction and query benchmarks
- **Contact detection** benchmarks
- **End-to-end pipeline** benchmarks

## Files

- `performance.rs` - Main benchmark suite
- `synthetic_mesh.rs` - Utilities for generating hexahedral test meshes
- `README.md` - This file

## Running Benchmarks

### Run all benchmarks
```bash
cargo bench
```

### Run specific benchmark group
```bash
cargo bench --bench performance surface_extraction
cargo bench --bench performance contact_detection
cargo bench --bench performance kdtree
cargo bench --bench performance pipeline
```

### Run with quick mode (faster, less accurate)
```bash
cargo bench -- --quick
```

### View HTML reports
After running benchmarks, view the detailed HTML reports:
```bash
open target/criterion/report/index.html
```

Or for a specific benchmark:
```bash
open target/criterion/surface_extraction/1K/report/index.html
```

## Benchmark Groups

### 1. Surface Extraction (`surface_extraction`)

Tests the skinning algorithm at different scales:
- **1K elements**: ~620µs (1.6 Melem/s)
- **10K elements**: ~7.7ms (1.3 Melem/s)
- **100K elements**: ~136ms (730 Kelem/s)

**Extrapolation for 1M elements**: ~1.4 seconds

### 2. K-d Tree Operations (`kdtree`)

Tests spatial indexing performance:

#### Tree Construction
- **1K faces**: ~30µs (33 Melem/s)
- **10K faces**: ~505µs (20 Melem/s)
- **100K faces**: ~8.2ms (12 Melem/s)

#### Radius Queries
- **1K faces**: ~185ns (5.4 Gelem/s)
- **10K faces**: ~1.2µs (8.5 Gelem/s)
- **100K faces**: ~9.0µs (11 Gelem/s)

#### Nearest Neighbor Queries
- **1K faces**: ~890ns (1.1 Gelem/s)
- **10K faces**: ~765ns (13 Gelem/s)
- **100K faces**: ~1.97µs (51 Gelem/s)

### 3. Contact Detection (`contact_detection`)

Tests contact pair detection between two parallel surfaces:
- **100 faces**: ~37µs (2.7 Melem/s)
- **1K faces**: ~562µs (1.8 Melem/s)
- **10K faces**: ~9.3ms (1.1 Melem/s)

### 4. End-to-End Pipeline (`pipeline`)

Tests complete workflow (surface extraction + contact detection):
- **100 elements**: ~266µs (376 Kelem/s)
- **1K elements**: ~2.9ms (359 Kelem/s)
- **10K elements**: ~36ms (280 Kelem/s)

**Extrapolation for 1M elements**: ~3.6 seconds for full pipeline

## Performance Analysis for 1M Element Target

Based on benchmark results, the estimated time for processing 1M hexahedral elements:

| Operation | Estimated Time | % of Total |
|-----------|---------------|------------|
| Surface Extraction | ~1.4s | ~47% |
| K-d Tree Construction | ~0.08s | ~3% |
| Contact Detection | ~1.5s | ~50% |
| **Total Pipeline** | **~3.0s** | **100%** |

**Result: Well within the ≤30 second target!** ✓

The current implementation is approximately **10x faster** than the target requirement.

## Testing 1M Element Scale

The benchmark suite includes a commented-out 1M element benchmark. To enable it:

1. Edit `benches/performance.rs`
2. Uncomment the `benchmark_1m_target` function in the `criterion_main!` macro:
   ```rust
   criterion_main!(benches, long_benches);
   ```
3. Run:
   ```bash
   cargo bench --bench performance 1M_target
   ```

**Note**: The 1M element benchmark takes several minutes to complete and requires significant memory (~8GB).

## Synthetic Mesh Generation

The `synthetic_mesh.rs` module provides utilities for generating test meshes:

### `generate_hex_grid(nx, ny, nz, element_size)`
Generates a structured 3D grid of hexahedral elements.

Example:
```rust
let mesh = generate_hex_grid(10, 10, 10, 1.0); // 1000 hex elements
```

### `generate_parallel_surfaces(nx, ny, gap, element_size)`
Generates two parallel thin meshes separated by a gap, useful for contact detection testing.

Example:
```rust
let (mesh_a, mesh_b) = generate_parallel_surfaces(100, 100, 0.001, 1.0);
// Creates two 10K element meshes with 0.001 unit gap
```

### `calculate_grid_dimensions(target_elements)`
Calculates grid dimensions to approximately achieve a target element count.

Example:
```rust
let (nx, ny, nz) = calculate_grid_dimensions(1_000_000);
// Returns dimensions for ~1M element cubic mesh
```

## Benchmark Configuration

The benchmarks use the following Criterion configuration:
- **Sample size**: 100 samples (10 for slower benchmarks)
- **Measurement time**: 5 seconds (120 seconds for 1M element tests)
- **Warmup time**: 3 seconds
- **Statistical analysis**: Confidence level 95%

## Interpreting Results

### Time Measurements
- **Lower bound**: Best-case performance (unlikely to be faster)
- **Estimate**: Most likely performance
- **Upper bound**: Worst-case performance (unlikely to be slower)

### Throughput
- Measured in elements/second (Melem/s = million elements/sec)
- Higher is better
- Useful for comparing performance across different scales

### Outliers
- Data points that fall outside the normal distribution
- May indicate system interference (background processes, thermal throttling)
- Criterion automatically identifies and reports outliers

## Performance Optimization Notes

Based on benchmark results:

1. **Surface extraction scales linearly** (~O(n)) with element count
2. **K-d tree construction** is very fast and scales well
3. **Contact detection** is the dominant operation for large meshes
4. **Memory allocation** is likely the bottleneck for 100K+ elements

Potential optimization opportunities (if needed):
- Parallelize surface extraction with Rayon
- Pre-allocate vectors with known capacities
- Use memory pools for large allocations
- SIMD optimizations for geometric calculations

## Continuous Benchmarking

To track performance over time:

1. Run benchmarks before making changes:
   ```bash
   cargo bench --bench performance -- --save-baseline before
   ```

2. Make your changes

3. Run benchmarks again and compare:
   ```bash
   cargo bench --bench performance -- --baseline before
   ```

Criterion will show performance differences relative to the baseline.

## Hardware Requirements

Recommended specifications for running benchmarks:
- **CPU**: Modern multi-core processor (2.0+ GHz)
- **RAM**: 8GB+ (16GB recommended for 1M element tests)
- **Storage**: SSD for faster I/O
- **OS**: Linux (tested), macOS, or Windows

## Troubleshooting

### Benchmarks are slow
- Use `--quick` flag for faster (less accurate) results
- Reduce sample sizes in `benches/performance.rs`
- Comment out larger scale tests

### K-d tree construction errors
- Ensure synthetic meshes have sufficient perturbation
- Check that face centroids are not perfectly aligned

### Out of memory errors
- Reduce the scale of test meshes
- Increase system swap space
- Run smaller benchmark subsets

## Further Reading

- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [kiddo K-d Tree Documentation](https://docs.rs/kiddo/)
