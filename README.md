# Contact Detector

High-performance hexahedral mesh contact pair detection and surface extraction tool for finite element analysis.

## Features

- **Surface Extraction**: Extract outer surface "skin" from hexahedral meshes
- **Contact Detection**: Automatically identify surface contact pairs based on configurable criteria
- **Metric Computation**: Calculate distances, angles, and other metrics for contact pairs
- **Fast Processing**: Process 1M+ element meshes in under 30 seconds
- **Exodus II Support**: Read standard Exodus II mesh files
- **VTK/VTU Export**: Export results with metadata for visualization

## Installation

```bash
cargo build --release
```

## Usage

```bash
# Extract surface mesh
contact-detector skin input.exo -o surface.vtu

# Detect contact pairs
contact-detector contact input.exo \
    --part-a "Block1" \
    --part-b "Block2" \
    --max-gap 0.005 \
    -o results.vtu

# Full analysis pipeline
contact-detector analyze input.exo \
    --pairs "Block1:Block2" \
    -o output_dir/
```

## Development Status

Phase 1: Foundation & Infrastructure (In Progress)

See [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) for detailed roadmap.

## License

MIT OR Apache-2.0
