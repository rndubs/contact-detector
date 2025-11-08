# Hexahedral Mesh Contact Detector - Implementation Plan

## Task List

- [x] **Phase 1: Foundation & Infrastructure** - Set up project, data structures, and Exodus II reading (see [Phase 1](#phase-1-foundation--infrastructure-week-1-2))
- [x] **Phase 2: Surface Extraction** - Implement "skinning" algorithm to extract outer surface (see [Phase 2](#phase-2-surface-extraction-week-2-3))
- [ ] **Phase 3: Spatial Indexing & Contact Detection** - Implement contact pair detection with octree acceleration (see [Phase 3](#phase-3-spatial-indexing--contact-detection-week-3-5))
- [ ] **Phase 4: Metric Computation** - Compute surface-level and element-level metrics (see [Phase 4](#phase-4-metric-computation-week-5-6))
- [ ] **Phase 5: CLI Polishing & Documentation** - Production-ready CLI with good UX (see [Phase 5](#phase-5-cli-polishing--documentation-week-6-7))
- [ ] **Phase 6 (Optional): Performance Optimization** - Ensure ≤30s for 1M elements (see [Phase 6](#phase-6-optional-performance-optimization))
- [ ] **Phase 7 (Future): CAD Import** - Import meshless CAD geometry from STEP/IGES (see [Phase 7](#phase-7-future-cad-import-stepiges))

## Executive Summary

This document outlines the implementation plan for a high-performance Rust application that processes massive hexahedral mesh files (up to 1-2M elements) for surface extraction, contact pair detection, and metric computation with a target processing time of ≤30 seconds for 1M elements.

## Requirements Summary

Based on clarifying discussions, the following requirements are confirmed:

- **Mesh Size**: Up to 1-2 million hexahedral elements
- **Performance Target**: Process 1M elements in ≤30 seconds
- **Parallelization**: Optional (not required initially)
- **Architecture**: CLI application only (not a library)
- **Visualization**: Export data for separate VS Code extension using VTK.js
- **Input Format**: Standard Exodus II (.exo) files only
- **Output Format**: VTK/VTU files with embedded metadata (format optimized for application needs)
- **Contact Pairs**: Handle multiple contact pair definitions in a single run
- **Metadata**: Preserve Exodus II metadata (part/material names, nodesets, sidesets)
- **Tolerances**: Configurable initial gap distance (e.g., 0.001-0.005 inches)
- **Platforms**: Linux primary; macOS/Windows optional
- **Use Case**: Internal use only
- **Test Data**: Downloaded from msh2exo-examples repository (see `test-data/` directory)

## Research Findings - Rust Ecosystem

### Available Libraries

#### 1. **netcdf** (v0.11.x) - Exodus II Reading
- **Repo**: https://github.com/georust/netcdf
- **Purpose**: Read Exodus II files (which are built on NetCDF4/HDF5)
- **Status**: Actively maintained, updated 2024
- **Note**: No native Rust Exodus II parser exists; we'll use netcdf and manually parse the Exodus II data model
- **Alternative**: FFI bindings to SEACAS C library (not recommended due to complexity)

#### 2. **vtkio** (latest) - VTK/VTU Writing
- **Repo**: https://github.com/elrnv/vtkio
- **Purpose**: Parse and write VTK/VTU files (XML and Legacy formats)
- **Features**:
  - Full support for unstructured grids (VTU files)
  - Compression support (via feature flag)
  - Field data and metadata support
- **Status**: Actively maintained (updated July 2024)
- **Perfect for our use case**: Can write surface meshes with custom metadata

#### 3. **spatialtree** (latest) - Spatial Indexing
- **Repo**: https://github.com/alexpyattaev/spatialtree
- **Purpose**: Generic octree/quadtree for realtime applications
- **Features**:
  - Slab arena allocation (low fragmentation)
  - Fast queries for proximity detection
- **Status**: Updated February 2025
- **Use case**: Accelerate contact pair detection via spatial partitioning

#### 4. **nalgebra** (latest) - Linear Algebra
- **Repo**: https://github.com/dimforge/nalgebra
- **Purpose**: Vector/matrix operations, normals, distances
- **Features**: Industry standard for Rust scientific computing
- **Use case**: All geometric computations (normals, distances, projections)

#### 5. **rayon** (latest) - Parallelization
- **Repo**: https://github.com/rayon-rs/rayon
- **Purpose**: Data parallelism
- **Use case**: Optional parallel processing for independent operations
- **Note**: May not be needed initially for 1M elements in 30s target

#### 6. **clap** (v4.x) - CLI Framework
- **Repo**: https://github.com/clap-rs/clap
- **Purpose**: Command-line argument parsing
- **Features**: Derive macros for easy CLI definition
- **Use case**: Main CLI interface

#### 7. Supporting Libraries
- **serde** + **serde_json**: Configuration file parsing
- **thiserror** / **anyhow**: Error handling
- **indicatif**: Progress bars for CLI
- **log** + **env_logger**: Logging

### Algorithms from Research

#### Surface Extraction
- **Approach**: Face enumeration from hex elements
- **Method**: Each hex has 6 quad faces; extract faces that are not shared with another element
- **Implementation**: Build face-to-element map, keep faces with only one adjacent element

#### Contact Pair Detection
- **Algorithm**: Master-Slave Common Normal Concept
- **Method**:
  1. For each node/element on slave surface, find closest point on master surface
  2. Verify that master surface normal passes through slave point
  3. Distance threshold and interpenetration checks
- **Acceleration**: Octree spatial indexing for O(log n) proximity queries instead of O(n²) brute force

#### Distance Metrics
- **Element-to-Element**: Project element centroid onto opposite surface along surface normal
- **Surface-to-Surface**: Average element-level metrics weighted by area
- **Miss Distance**: Track elements without pairs within tolerance

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     CLI Interface (clap)                     │
│  - File paths, tolerances, output options                   │
└─────────────────┬───────────────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────────────┐
│                  Exodus II Reader (netcdf)                   │
│  - Parse mesh geometry (nodes, elements)                     │
│  - Extract metadata (part names, nodesets, sidesets)         │
└─────────────────┬───────────────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────────────┐
│                    Mesh Data Structure                       │
│  - Nodes: Vec<Point3<f64>>                                   │
│  - Elements: Vec<HexElement>                                 │
│  - Parts/Blocks: HashMap<PartId, ElementSet>                 │
│  - Nodesets/Sidesets: Preserved from Exodus                  │
└─────────────────┬───────────────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────────────┐
│              Surface Extraction ("Skinning")                 │
│  - Build face-to-element adjacency map                       │
│  - Extract boundary faces (single adjacent element)          │
│  - Group faces by part                                       │
│  - Output: SurfaceMesh per part                              │
└─────────────────┬───────────────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────────────┐
│            Surface Contact Pair Detection                    │
│  1. Build octree for each surface (spatialtree)              │
│  2. For each surface A element:                              │
│     a. Compute element centroid and normal                   │
│     b. Query octree of surface B for candidates              │
│     c. Find closest point on B along normal                  │
│     d. Apply distance/interpenetration criteria              │
│  3. Store ContactPair with metadata                          │
└─────────────────┬───────────────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────────────┐
│                   Metric Computation                         │
│  - Element-level: distance, normal alignment, gap/overlap    │
│  - Surface-level: average distance, std dev, min/max         │
│  - Miss distance: elements without pairs                     │
│  - Area-weighted statistics                                  │
└─────────────────┬───────────────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────────────┐
│                 VTU Writer (vtkio)                           │
│  - Export skinned mesh per part                              │
│  - Attach contact pair IDs as cell data                      │
│  - Attach metrics as field data:                             │
│    * Distance, normal angle, gap/overlap flag                │
│    * Pair ID, miss distance                                  │
│  - Preserve part names, nodesets, sidesets as metadata       │
└─────────────────────────────────────────────────────────────┘
```

## Implementation Phases

### Phase 1: Foundation & Infrastructure (Week 1-2)
**Goal**: Set up project, data structures, and Exodus II reading

#### Tasks:
1. **Project Setup**
   - Initialize Cargo workspace
   - Add dependencies: `netcdf`, `nalgebra`, `clap`, `serde`, `thiserror`, `indicatif`, `log`
   - Set up basic CLI with `clap` (derive API)
   - Configure logging and progress indicators

2. **Core Data Structures**
   ```rust
   // Core mesh types
   struct Point3 { x: f64, y: f64, z: f64 }
   struct HexElement { node_ids: [usize; 8] }
   struct QuadFace { node_ids: [usize; 4] }

   // Mesh container
   struct Mesh {
       nodes: Vec<Point3>,
       elements: Vec<HexElement>,
       element_blocks: HashMap<String, Vec<usize>>, // part_name -> element indices
       node_sets: HashMap<String, Vec<usize>>,
       side_sets: HashMap<String, Vec<(usize, u8)>>, // (elem_id, face_id)
   }

   // Surface representation
   struct SurfaceMesh {
       part_name: String,
       faces: Vec<QuadFace>,
       face_normals: Vec<Vector3>,
       face_centroids: Vec<Point3>,
       face_areas: Vec<f64>,
   }
   ```

3. **Exodus II Reader**
   - Research Exodus II data model (nodes, element blocks, nodesets, sidesets)
   - Implement `ExodusReader` using `netcdf` crate
   - Parse coordinate arrays (coordx, coordy, coordz)
   - Parse element connectivity by block
   - Parse metadata (block names, nodeset names, sideset names)
   - Write unit tests with small example Exodus files

4. **Basic Geometric Operations**
   - Hex element face enumeration (6 faces per hex, canonical ordering)
   - Quad face normal computation (cross product of diagonals)
   - Quad face centroid and area calculation
   - Unit tests for geometric primitives

**Deliverable**: CLI tool that reads Exodus II file and prints mesh statistics

---

### Phase 2: Surface Extraction (Week 2-3)
**Goal**: Implement "skinning" algorithm to extract outer surface

#### Tasks:
1. **Face Adjacency Builder**
   - Create hash map: `Face -> Vec<ElementId>`
   - Hash function for quad faces (canonical node ordering)
   - Iterate all hex elements, add 6 faces each to map

2. **Boundary Face Extraction**
   - Filter faces with exactly 1 adjacent element
   - Group by element block (part)
   - Build `SurfaceMesh` per part

3. **Surface Properties**
   - Compute face normals (outward pointing)
   - Compute face centroids and areas
   - Validate surface is closed (for debugging)

4. **VTU Writer (vtkio)**
   - Convert `SurfaceMesh` to `vtkio::model::UnstructuredGrid`
   - Write VTU file with surface mesh
   - Add part name as field data
   - Test with ParaView/VTK.js

**Deliverable**: CLI command to extract and export surface mesh as VTU

---

### Phase 3: Spatial Indexing & Contact Detection (Week 3-5)
**Goal**: Implement contact pair detection with octree acceleration

#### Tasks:
1. **Octree Integration**
   - Integrate `spatialtree` crate
   - Build octree per surface (key: face centroid)
   - Test query performance (should be O(log n))

2. **Contact Pair Detection Algorithm**
   ```rust
   struct ContactPair {
       surface_a_face_id: usize,
       surface_b_face_id: usize,
       distance: f64,           // signed: + for gap, - for overlap
       normal_angle: f64,       // angle between normals (degrees)
       contact_point: Point3,   // point on surface B
   }

   struct ContactCriteria {
       max_gap_distance: f64,        // e.g., 0.005 inches
       max_penetration: f64,         // e.g., 0.001 inches
       max_normal_angle: f64,        // e.g., 45 degrees
       search_radius_multiplier: f64, // octree query radius
   }
   ```

3. **Master-Slave Contact Search**
   - For each face on surface A:
     - Get centroid and normal
     - Query octree of surface B with search radius
     - For each candidate face on B:
       - Project A's centroid onto B's plane
       - Check if projection is within B's quad bounds
       - Compute signed distance (+ gap, - overlap)
       - Compute angle between normals
       - Apply criteria filters
     - Store best match as `ContactPair`

4. **Bidirectional Search**
   - Run A→B and B→A searches
   - Optionally enforce symmetry (pair if both directions agree)

5. **Miss Distance Tracking**
   - Track faces on A without pair on B (and vice versa)
   - Store as special metadata

**Deliverable**: CLI command to detect contact pairs and print statistics

---

### Phase 4: Metric Computation (Week 5-6)
**Goal**: Compute surface-level and element-level metrics

#### Tasks:
1. **Element-Level Metrics**
   - Distance (gap/overlap)
   - Normal angle
   - Contact point coordinates
   - Paired element ID (or -1 for miss)

2. **Surface-Level Metrics**
   ```rust
   struct SurfaceMetrics {
       total_area: f64,
       paired_area: f64,
       unpaired_area: f64,
       avg_distance: f64,
       std_dev_distance: f64,
       min_distance: f64,
       max_distance: f64,
       avg_normal_angle: f64,
       num_pairs: usize,
       num_misses: usize,
   }
   ```
   - Area-weighted averages for distance
   - Standard deviation
   - Min/max statistics
   - Histograms (optional)

3. **Output to VTU with Metadata**
   - Write surface mesh VTU
   - Add cell data (per face):
     - `pair_id`: i32 (or -1 for miss)
     - `distance`: f64
     - `normal_angle`: f64
     - `is_paired`: bool
   - Add field data (per surface):
     - All surface-level metrics
     - Criteria used for detection
   - Preserve Exodus metadata (part names, nodesets)

**Deliverable**: Complete VTU output with all metadata for visualization

---

### Phase 5: CLI Polishing & Documentation (Week 6-7)
**Goal**: Production-ready CLI with good UX

#### Tasks:
1. **CLI Commands**
   ```bash
   # Extract surface mesh
   contact-detector skin <input.exo> -o <output.vtu>

   # Detect contact pairs
   contact-detector contact <input.exo> \
       --part-a "Block1" \
       --part-b "Block2" \
       --max-gap 0.005 \
       --max-penetration 0.001 \
       --max-angle 45 \
       -o results.vtu

   # Full pipeline
   contact-detector analyze <input.exo> \
       --pairs "Block1:Block2,Block3:Block4" \
       --config config.json \
       -o output_dir/
   ```

2. **Configuration File Support**
   - JSON/YAML config for batch processing
   - Multiple contact pair definitions
   - Per-pair criteria

3. **Progress Indicators**
   - Progress bars for long operations (indicatif)
   - Estimated time remaining
   - Memory usage stats

4. **Error Handling**
   - Clear error messages for bad inputs
   - Validation of mesh topology
   - Graceful handling of edge cases

5. **Documentation**
   - README with examples
   - API documentation (rustdoc)
   - Example Exodus files for testing
   - VTK.js visualization example

**Deliverable**: Production CLI tool with documentation

---

### Phase 6 (Optional): Performance Optimization
**Goal**: Ensure ≤30s for 1M elements

#### Tasks:
1. **Profiling**
   - Use `cargo flamegraph` or `perf`
   - Identify bottlenecks

2. **Optimization Strategies**
   - Parallel octree construction (rayon)
   - Parallel contact search per surface
   - SIMD for distance calculations (if needed)
   - Memory pooling for large allocations

3. **Benchmarking**
   - Create benchmark suite with varying mesh sizes
   - Target: 1M elements < 30s on commodity hardware

**Deliverable**: Optimized tool meeting performance requirements

---

### Phase 7 (Future): CAD Import (STEP/IGES)
**Goal**: Import meshless CAD geometry

#### Research Needed:
- **opencascade-rs**: Rust bindings for OpenCASCADE (STEP/IGES support)
- **truck**: Pure Rust CAD kernel (early stage)
- Meshing CAD surfaces to quads (challenging)

**Note**: This is significantly more complex and may require external tools like Gmsh or CUBIT for meshing, then import the mesh.

---

## File Structure

```
contact-detector/
├── Cargo.toml
├── README.md
├── IMPLEMENTATION_PLAN.md
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library interface
│   ├── cli/
│   │   ├── mod.rs           # CLI command definitions
│   │   ├── skin.rs          # Surface extraction command
│   │   ├── contact.rs       # Contact detection command
│   │   └── analyze.rs       # Full pipeline command
│   ├── io/
│   │   ├── mod.rs
│   │   ├── exodus.rs        # Exodus II reader
│   │   └── vtu.rs           # VTU writer
│   ├── mesh/
│   │   ├── mod.rs
│   │   ├── types.rs         # Core data structures
│   │   ├── geometry.rs      # Geometric operations
│   │   └── surface.rs       # Surface extraction
│   ├── contact/
│   │   ├── mod.rs
│   │   ├── detection.rs     # Contact pair algorithm
│   │   ├── octree.rs        # Spatial indexing
│   │   └── metrics.rs       # Metric computation
│   └── error.rs             # Error types
├── tests/
│   ├── integration_tests.rs
│   └── data/
│       └── sample.exo       # Small test mesh
└── examples/
    ├── basic_usage.rs
    └── config.json
```

## Risk Mitigation

### Risk 1: No Native Exodus II Parser
**Mitigation**: Use `netcdf` crate and manually implement Exodus II data model. The format is well-documented (Sandia SEACAS docs).

### Risk 2: Performance on 1M+ Elements
**Mitigation**: Use octree (O(log n) queries) instead of brute force (O(n²)). Profile early and optimize hot paths. Use Rayon if needed.

### Risk 3: Complex Contact Geometry
**Mitigation**: Start with simple master-slave common normal. Add refinements (e.g., mortar methods) only if needed.

### Risk 4: Metadata Preservation in VTU
**Mitigation**: `vtkio` supports field data and cell data. Test early with VTK.js to ensure metadata is accessible.

## Success Criteria

1. ✅ Read Exodus II files with 1M+ hex elements
2. ✅ Extract surface mesh in <10 seconds
3. ✅ Detect contact pairs with configurable criteria
4. ✅ Compute accurate distance and angle metrics
5. ✅ Export VTU with metadata viewable in VTK.js
6. ✅ Total processing time <30s for 1M elements
7. ✅ Preserve Exodus II metadata (parts, nodesets, sidesets)
8. ✅ Clean CLI interface with progress indicators

## Timeline Estimate

- **Phase 1**: 1.5 weeks
- **Phase 2**: 1 week
- **Phase 3**: 2 weeks (most complex)
- **Phase 4**: 1 week
- **Phase 5**: 1 week
- **Phase 6**: 1 week (if needed)

**Total**: 6-8 weeks for production-ready tool (without CAD import)

## Next Steps

1. ✅ ~~Review plan and confirm approach~~ - **CONFIRMED**
2. ✅ ~~Obtain test Exodus II file~~ - **Downloaded to `test-data/hexcyl.exo`**
3. Set up initial Cargo project structure
4. Begin Phase 1 implementation
5. Iterate with feedback on intermediate deliverables

---

## Decisions Made

All clarifying questions have been answered:

1. ✅ **Test Files**: Downloaded hexahedral mesh example from msh2exo-examples (264KB, HEX8 elements)
2. ✅ **Multiple Contact Pairs**: Yes, handle all contact pair definitions in a single run
3. ✅ **VTK.js Metadata**: Format will be optimized for this application; VS Code extension will adapt
4. ✅ **Library vs CLI**: CLI application only (no library crate needed)
5. ✅ **Future Formats**: No additional mesh formats required (Exodus II only)

**Status**: Ready to begin Phase 1 implementation.
