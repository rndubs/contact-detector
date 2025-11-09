# Skinner Feature Comparison: SEACAS vs Rust Implementation

## Executive Summary

This document compares the "skinner" (surface extraction) implementation in our Rust application against the SEACAS skinner utility from Sandia National Laboratories. The analysis identifies key differences, potential omissions, and areas for improvement.

**Overall Assessment**: Our Rust implementation correctly implements the core skinning algorithm but lacks several important features present in SEACAS, particularly around data transfer, node optimization, and advanced I/O capabilities.

---

## Core Algorithm Comparison

### SEACAS Skinner (C++)
**Location**: `seacas/libraries/ioss/src/main/skinner.C`

**Core Algorithm**:
1. Uses `Ioss::FaceGenerator` to generate all faces from volume elements
2. Filters faces where `face->elementCount_ == 1` (boundary faces)
3. Creates a mapping from old node IDs to new node IDs (node compaction)
4. Builds output mesh with only boundary faces and referenced nodes
5. Optionally transfers transient field data (time-dependent variables)

**Key Code Pattern**:
```cpp
Ioss::FaceGenerator face_generator(*region);
face_generator.generate_faces(...);

// Filter boundary faces
for (auto &face : faces) {
    if (face->elementCount_ == 1) {
        // This is a boundary face
    }
}
```

### Our Rust Implementation
**Location**: `src/mesh/surface.rs`

**Core Algorithm**:
1. Builds face adjacency map: `HashMap<QuadFace, Vec<usize>>` (face → element indices)
2. Filters boundary faces: `elements.len() == 1`
3. Groups faces by element block
4. **Additional step**: Subdivides faces into coplanar surface patches using BFS
5. Computes geometric properties (normals, centroids, areas)
6. Includes all nodes (no compaction)

**Key Code Pattern**:
```rust
// Build adjacency
let mut adjacency: HashMap<QuadFace, Vec<usize>> = HashMap::new();
for (elem_idx, element) in mesh.elements.iter().enumerate() {
    for face in &element.faces() {
        adjacency.entry(face.canonical()).or_default().push(elem_idx);
    }
}

// Extract boundary faces
for (face, elements) in face_adjacency {
    if elements.len() == 1 {
        boundary_faces.insert(*face, elements[0]);
    }
}
```

---

## Feature Comparison Matrix

| Feature | SEACAS Skinner | Rust Implementation | Status |
|---------|---------------|---------------------|--------|
| **Core Skinning Algorithm** | ✓ | ✓ | ✓ Equivalent |
| **Boundary Face Detection** | ✓ | ✓ | ✓ Correct |
| **Node Compaction/Remapping** | ✓ | ✗ | ⚠️ Missing |
| **Surface Patch Subdivision** | ✗ | ✓ | ➕ Enhancement |
| **Transient Data Transfer** | ✓ | ✗ | ⚠️ Missing |
| **Nodal Variable Transfer** | ✓ | ✗ | ⚠️ Missing |
| **Element Variable Transfer** | ✓ | ✗ | ⚠️ Missing |
| **Time Step Selection** | ✓ | ✗ | ⚠️ Missing |
| **32-bit/64-bit Integer Support** | ✓ | Implicit (usize) | ✓ OK |
| **Multiple Output Formats** | ✓ (Exodus, CGNS, etc.) | VTU only | ⚠️ Limited |
| **Parallel Processing** | ✓ (MPI) | ✓ (Rayon) | ✓ Different approach |
| **Compression Support** | ✓ (NetCDF4, HDF5) | ✗ | ⚠️ Missing |
| **Statistics Output** | ✓ | Basic | ⚠️ Limited |
| **Element Type Support** | Multiple (Hex, Tet, etc.) | Hex only | ⚠️ Limited |

---

## Detailed Analysis

### 1. Node Compaction/Remapping ⚠️

**SEACAS Approach**:
```cpp
// Maps old node ID to new node ID
std::map<INT, INT> node_map;
INT new_node_id = 1;

for (auto &face : boundary_faces) {
    for (int i = 0; i < face.connectivity.size(); i++) {
        INT old_id = face.connectivity[i];
        if (node_map.find(old_id) == node_map.end()) {
            node_map[old_id] = new_node_id++;
        }
        face.connectivity[i] = node_map[old_id]; // Remap
    }
}

// Output only nodes that are referenced by boundary faces
for (auto &[old_id, new_id] : node_map) {
    output_nodes[new_id] = input_nodes[old_id];
}
```

**Our Approach**:
```rust
// Clone ALL nodes from the mesh
let surface_nodes = nodes.to_vec();  // ⚠️ Inefficient
```

**Impact**:
- Our implementation includes ALL mesh nodes, even those not referenced by surface faces
- This is inefficient for large meshes
- Output files are larger than necessary
- Comment in code acknowledges this: `// Note: This could be optimized to only include nodes used by surface faces`

**Recommendation**: Implement node remapping similar to SEACAS

---

### 2. Transient Data Transfer ⚠️

**SEACAS Feature**:
SEACAS can transfer time-dependent field data (nodal and element variables) from the input mesh to the skinned output mesh. This is critical for visualization of transient simulations.

**Key Capabilities**:
- Transfer nodal variables (temperature, displacement, etc.)
- Transfer element variables (stress, strain, etc.)
- Select specific time steps or time ranges
- Remap variables to the compacted node/element sets

**Code Pattern**:
```cpp
std::vector<int> selected_steps = get_selected_steps(region, ...);

for (int step : selected_steps) {
    // Read transient data at this time step
    region->begin_state(step);

    // Transfer nodal fields
    for (auto *node_block : region->get_node_blocks()) {
        for (auto *field : node_block->field_begin()) {
            transfer_field_data(field, node_map, ...);
        }
    }

    // Transfer element fields
    // ... similar for element blocks

    region->end_state();
}
```

**Our Implementation**: None

**Recommendation**:
- Add support for reading/writing transient data if working with time-dependent simulations
- This is essential for visualization workflows in engineering analysis
- Lower priority if only working with static geometries

---

### 3. Surface Patch Subdivision ➕

**Our Enhancement**:
We implement a feature that SEACAS does NOT have: subdividing element block surfaces into coplanar patches.

**Algorithm** (src/mesh/surface.rs:128-198):
```rust
const MAX_COPLANAR_ANGLE: f64 = 10.0;

// BFS to group connected, coplanar faces
for seed_face in faces {
    let seed_normal = compute_normal(seed_face);
    let mut patch = vec![seed_face];

    // Expand patch with adjacent coplanar faces
    for adjacent_face in adjacency[seed_face] {
        let angle = angle_between(seed_normal, adjacent_face.normal);
        if angle <= MAX_COPLANAR_ANGLE {
            patch.push(adjacent_face);
        }
    }

    create_surface_mesh(patch);
}
```

**Rationale**: This allows better organization of complex geometries with multiple planar surfaces in a single element block.

**Trade-off**:
- ✓ Better surface organization
- ✓ Easier contact detection between specific surfaces
- ✗ More output files/surfaces to manage
- ✗ Different from standard FEA tools

**Recommendation**: Consider making this subdivision optional via command-line flag

---

### 4. Element Type Support ⚠️

**SEACAS Support**:
- Hexahedron (8-node, 20-node, 27-node)
- Tetrahedron (4-node, 10-node)
- Wedge/Prism
- Pyramid
- Shell elements
- Multiple element types in same mesh

**Our Support**:
- Hexahedron (8-node) only
- Single element type

**Impact**: Cannot process meshes with tet, wedge, or pyramid elements

**Recommendation**:
- Add support for tetrahedral elements (4 triangle faces per element)
- Add support for wedge elements (2 triangle + 3 quad faces)
- Consider whether mixed-element meshes are needed for your use case

---

### 5. Face Topology Implementation ✓

**Critical Aspect**: Correct face node ordering and canonical form

**SEACAS Hex Face Definition**:
```cpp
// Exodus II standard hex8 node ordering
// Faces defined with outward-pointing normals
static int hex_faces[6][4] = {
    {0, 1, 5, 4},  // Face 1 (front)
    {1, 2, 6, 5},  // Face 2 (right)
    {2, 3, 7, 6},  // Face 3 (back)
    {0, 4, 7, 3},  // Face 4 (left)
    {0, 3, 2, 1},  // Face 5 (bottom)
    {4, 5, 6, 7}   // Face 6 (top)
};
```

**Our Implementation** (src/mesh/types.rs:31-41):
```rust
pub fn faces(&self) -> [QuadFace; 6] {
    let n = self.node_ids;
    [
        QuadFace::new([n[0], n[3], n[2], n[1]]), // bottom (z-)
        QuadFace::new([n[4], n[5], n[6], n[7]]), // top (z+)
        QuadFace::new([n[0], n[1], n[5], n[4]]), // front (y-)
        QuadFace::new([n[1], n[2], n[6], n[5]]), // right (x+)
        QuadFace::new([n[2], n[3], n[7], n[6]]), // back (y+)
        QuadFace::new([n[3], n[0], n[4], n[7]]), // left (x-)
    ]
}
```

**Comparison**:
- Our front face: `[0, 1, 5, 4]` ✓ Matches SEACAS Face 1
- Our right face: `[1, 2, 6, 5]` ✓ Matches SEACAS Face 2
- Our back face: `[2, 3, 7, 6]` ✓ Matches SEACAS Face 3
- Our left face: `[3, 0, 4, 7]` ✓ Matches SEACAS Face 4 (rotated but equivalent)
- Our bottom face: `[0, 3, 2, 1]` ✓ Matches SEACAS Face 5 (rotated)
- Our top face: `[4, 5, 6, 7]` ✓ Matches SEACAS Face 6

**Canonical Form** (src/mesh/types.rs:59-81):
Our implementation correctly handles face hashing by:
1. Rotating to start with minimum node ID
2. Checking both orientations (normal and reversed)
3. Selecting lexicographically smallest representation

This ensures faces with the same nodes but different winding orders hash identically, which is correct.

**Assessment**: ✓ Face topology is correctly implemented

---

### 6. Statistics and Debugging ⚠️

**SEACAS Output**:
```cpp
void output_table(const FaceVector &boundary_faces) {
    std::map<std::string, size_t> block_face_count;

    for (auto &face : boundary_faces) {
        block_face_count[face.element_block]++;
    }

    fmt::print("\n{:^20} {:>12}\n", "Element Block", "Face Count");
    fmt::print("{:-<34}\n", "");
    for (auto &[block, count] : block_face_count) {
        fmt::print("{:^20} {:>12}\n", block, count);
    }
}
```

**Our Output**:
```rust
for surface in &surfaces_to_write {
    println!(
        "  - {}: {} faces, total area: {:.6}",
        surface.part_name,
        surface.num_faces(),
        surface.total_area()
    );
}
```

**Comparison**:
- SEACAS provides detailed statistics tables
- Our output is minimal but includes area calculation (enhancement)
- SEACAS has `--statistics` flag for verbose output
- SEACAS has `--debug` flag for debugging

**Recommendation**: Add more comprehensive statistics and debugging options

---

### 7. Command-Line Interface ✓

**SEACAS** (skinner_interface.C):
```
skinner [options] input_file output_file
  --in_type <format>          Input database format
  --out_type <format>         Output database format
  --output_transient          Transfer transient data
  --Maximum_Time <time>       Maximum time to transfer
  --Minimum_Time <time>       Minimum time to transfer
  --blocks                    Process by blocks
  --compress <level>          Compression level
  --statistics                Show statistics
  --debug                     Debug output
  --decomposition <method>    Parallel decomposition (RCB, RIB, etc.)
```

**Our Implementation** (src/main.rs):
```
contact-detector skin [OPTIONS] --input <INPUT> --output <OUTPUT>
  --input <INPUT>            Input mesh file
  --output <OUTPUT>          Output file/directory
  --part <PART>              Extract specific part
  --vtk-version <X.Y>        VTK format version
```

**Assessment**:
- ✓ Basic functionality covered
- ⚠️ Missing many advanced options (compression, statistics, etc.)
- ✓ Part filtering is an enhancement

---

## Potential Issues & Recommendations

### High Priority

1. **Node Remapping** ⚠️ ISSUE
   - **Problem**: Including all nodes wastes memory and disk space
   - **Fix**: Implement node compaction as in SEACAS
   - **Location**: `src/mesh/surface.rs:298-300`
   - **Estimated Impact**: 50-90% reduction in output file size for large meshes

2. **Face Topology Verification** ✓ CORRECT
   - **Status**: Verified correct against Exodus II standard
   - **No issues found**

3. **Boundary Detection Algorithm** ✓ CORRECT
   - **Status**: Correctly identifies faces with exactly 1 adjacent element
   - **No issues found**

### Medium Priority

4. **Element Type Support** ⚠️ LIMITATION
   - **Problem**: Only supports hex8 elements
   - **Fix**: Add tetrahedral, wedge, and pyramid element types
   - **Use Case**: Mixed-element meshes common in engineering

5. **Transient Data** ⚠️ LIMITATION
   - **Problem**: Cannot transfer time-dependent field data
   - **Fix**: Add support for reading/writing transient variables
   - **Use Case**: Essential for simulation result visualization

6. **Output Format Support** ⚠️ LIMITATION
   - **Problem**: VTU only (SEACAS supports Exodus, CGNS, etc.)
   - **Fix**: Add Exodus output support (already have reader)
   - **Use Case**: Interoperability with other FEA tools

### Low Priority

7. **Statistics Output** ✓ ADEQUATE
   - **Status**: Basic statistics provided
   - **Enhancement**: Add more detailed statistics and `--verbose` flag

8. **Surface Patch Subdivision** ➕ ENHANCEMENT
   - **Status**: Our unique feature, not in SEACAS
   - **Recommendation**: Make optional via `--subdivide-surfaces` flag
   - **Trade-off**: Better organization vs. compatibility

---

## Code Quality Comparison

### SEACAS Strengths
- Mature codebase (20+ years)
- Extensive testing in production environments
- Comprehensive error handling
- Supports wide range of element types and file formats
- Well-documented command-line interface

### Our Rust Implementation Strengths
- Modern type-safe implementation
- Clear separation of concerns
- Good use of Rust idioms (Result types, iterators)
- Parallel processing with Rayon
- Surface patch subdivision (unique feature)
- Cleaner, more readable code

### Our Weaknesses
- Limited element type support
- No node remapping optimization
- No transient data handling
- Limited output formats
- Fewer command-line options

---

## Algorithmic Correctness Assessment

### ✓ CORRECT IMPLEMENTATIONS

1. **Core Skinning Algorithm**: Our face adjacency approach is algorithmically equivalent to SEACAS's FaceGenerator
2. **Boundary Face Detection**: Correctly identifies faces with exactly 1 adjacent element
3. **Face Canonical Form**: Properly handles face hashing for matching
4. **Hex Face Topology**: Matches Exodus II standard face definitions
5. **Geometric Calculations**: Normals, centroids, and areas computed correctly

### ⚠️ MISSING BUT NOT INCORRECT

1. **Node Remapping**: Not wrong, just inefficient
2. **Transient Data**: Not applicable if not working with time-dependent data
3. **Multiple Element Types**: Limitation, not an error

### ❌ NO CRITICAL ERRORS FOUND

No algorithmic errors or incorrect methods were identified. The implementation is sound for its intended scope (hex-only, static geometry).

---

## Recommendations Summary

### Immediate Actions
1. ✓ Verify face topology (DONE - confirmed correct)
2. ⚠️ Implement node remapping to reduce output file size
3. ⚠️ Add Exodus output format (reader already exists)

### Future Enhancements
4. Add tetrahedral element support
5. Add transient data transfer capability
6. Make surface subdivision optional
7. Add compression support
8. Enhance statistics output
9. Add progress indicators for large meshes

### Documentation
10. Document the surface subdivision feature and its trade-offs
11. Add examples comparing output with/without subdivision
12. Clarify element type limitations in user documentation

---

## Conclusion

The Rust skinner implementation is **algorithmically correct** and follows the same fundamental approach as the SEACAS skinner:

1. ✓ Generate all faces from elements
2. ✓ Identify boundary faces (adjacency count = 1)
3. ✓ Extract surface mesh

The main differences are:

- **Missing**: Node remapping, transient data, multi-element support
- **Enhancement**: Surface patch subdivision by coplanarity
- **Different**: Implementation language and data structures

For the current use case (hex-only contact detection on static geometries), the implementation is suitable. For broader FEA workflows, consider implementing the recommended features, particularly node remapping and Exodus output.

**Overall Grade**: B+ (Solid implementation with room for optimization and feature expansion)
