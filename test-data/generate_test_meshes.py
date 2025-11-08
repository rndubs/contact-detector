# /// script
# dependencies = [
#  "numpy",
#  "netCDF4",
# ]
# ///
"""
Generate test Exodus II mesh files for contact detection testing.
Run this script with uv so it will install the dependencies for you:
    - uv run generate_test_meshes.py
"""

import sys
try:
    from netCDF4 import Dataset
    import numpy as np
except ImportError:
    print("Error: netCDF4 not installed. Run: pip install netCDF4 numpy")
    sys.exit(1)


def create_exodus_file(filename, num_dim, num_nodes, num_elem, num_el_blk):
    """Create a basic Exodus II file structure."""
    nc = Dataset(filename, 'w', format='NETCDF3_CLASSIC')

    # Define dimensions
    nc.createDimension('len_string', 33)
    nc.createDimension('len_line', 81)
    nc.createDimension('four', 4)
    nc.createDimension('len_name', 33)
    nc.createDimension('time_step', None)
    nc.createDimension('num_dim', num_dim)
    nc.createDimension('num_nodes', num_nodes)
    nc.createDimension('num_elem', num_elem)
    nc.createDimension('num_el_blk', num_el_blk)

    # Global attributes
    nc.api_version = 5.14
    nc.version = 5.14
    nc.floating_point_word_size = 8
    nc.file_size = 1
    nc.title = 'Test mesh for contact detection'

    # Create coordinate arrays
    coordx = nc.createVariable('coordx', 'f8', ('num_nodes',))
    coordy = nc.createVariable('coordy', 'f8', ('num_nodes',))
    coordz = nc.createVariable('coordz', 'f8', ('num_nodes',))

    # Create time_whole variable
    time_whole = nc.createVariable('time_whole', 'f8', ('time_step',))

    # Create element block status
    eb_status = nc.createVariable('eb_status', 'i4', ('num_el_blk',))
    eb_prop1 = nc.createVariable('eb_prop1', 'i4', ('num_el_blk',))
    eb_prop1.setncattr('name', 'ID')

    return nc


def generate_single_hex_contact():
    """
    Generate two hexahedral elements sharing one contact surface.
    Each element is a simple cube.
    """
    filename = 'test-data/single_hex_contact.exo'
    print(f"Generating {filename}...")

    num_nodes = 12  # 8 nodes for first hex + 4 new nodes for second hex (share 4)
    num_elem = 2
    num_el_blk = 2

    nc = create_exodus_file(filename, 3, num_nodes, num_elem, num_el_blk)

    # Define nodes for two cubes sharing a face
    # First cube: nodes 1-8
    # Second cube: nodes 5-8 (shared), 9-12 (new)
    coords_x = np.array([0.0, 1.0, 1.0, 0.0,  # bottom of first cube
                         0.0, 1.0, 1.0, 0.0,  # top of first cube (shared bottom of second)
                         0.0, 1.0, 1.0, 0.0]) # top of second cube
    coords_y = np.array([0.0, 0.0, 1.0, 1.0,
                         0.0, 0.0, 1.0, 1.0,
                         0.0, 0.0, 1.0, 1.0])
    coords_z = np.array([0.0, 0.0, 0.0, 0.0,
                         1.0, 1.0, 1.0, 1.0,
                         2.0, 2.0, 2.0, 2.0])

    nc.variables['coordx'][:] = coords_x
    nc.variables['coordy'][:] = coords_y
    nc.variables['coordz'][:] = coords_z

    # Element block 1: first hex
    nc.createDimension('num_el_in_blk1', 1)
    nc.createDimension('num_nod_per_el1', 8)
    connect1 = nc.createVariable('connect1', 'i4', ('num_el_in_blk1', 'num_nod_per_el1'))
    connect1.elem_type = 'HEX8'
    connect1[:] = [[1, 2, 3, 4, 5, 6, 7, 8]]  # 1-based indexing

    # Element block 2: second hex
    nc.createDimension('num_el_in_blk2', 1)
    nc.createDimension('num_nod_per_el2', 8)
    connect2 = nc.createVariable('connect2', 'i4', ('num_el_in_blk2', 'num_nod_per_el2'))
    connect2.elem_type = 'HEX8'
    connect2[:] = [[5, 6, 7, 8, 9, 10, 11, 12]]  # shares nodes 5-8

    nc.variables['eb_status'][:] = [1, 1]
    nc.variables['eb_prop1'][:] = [1, 2]

    nc.close()
    print(f"  Created: {filename}")


def generate_aligned_cubes_10x10x10():
    """
    Generate two 1"x1"x1" cubes, each with 10x10x10 elements, sharing one contact surface.
    Parts are aligned on edges.
    """
    filename = 'test-data/aligned_cubes_10x10x10.exo'
    print(f"Generating {filename}...")

    n = 10  # elements per dimension
    nodes_per_dim = n + 1

    # First cube: nodes from 0 to nodes_per_dim in each dimension
    # Second cube: shares the top face of first cube, extends upward
    num_nodes_per_cube = nodes_per_dim ** 3
    num_shared_nodes = nodes_per_dim ** 2  # one face worth
    num_nodes = 2 * num_nodes_per_cube - num_shared_nodes

    num_elem_per_cube = n ** 3
    num_elem = 2 * num_elem_per_cube
    num_el_blk = 2

    nc = create_exodus_file(filename, 3, num_nodes, num_elem, num_el_blk)

    # Generate coordinates
    coords = np.linspace(0.0, 1.0, nodes_per_dim)

    # First cube nodes
    coords_x = []
    coords_y = []
    coords_z = []

    for k in range(nodes_per_dim):
        for j in range(nodes_per_dim):
            for i in range(nodes_per_dim):
                coords_x.append(coords[i])
                coords_y.append(coords[j])
                coords_z.append(coords[k])

    # Second cube nodes (skip the shared bottom face, k=0)
    for k in range(1, nodes_per_dim):
        for j in range(nodes_per_dim):
            for i in range(nodes_per_dim):
                coords_x.append(coords[i])
                coords_y.append(coords[j])
                coords_z.append(coords[k] + 1.0)  # offset by 1 inch

    nc.variables['coordx'][:] = np.array(coords_x)
    nc.variables['coordy'][:] = np.array(coords_y)
    nc.variables['coordz'][:] = np.array(coords_z)

    # Generate connectivity for first cube
    nc.createDimension('num_el_in_blk1', num_elem_per_cube)
    nc.createDimension('num_nod_per_el1', 8)
    connect1 = nc.createVariable('connect1', 'i4', ('num_el_in_blk1', 'num_nod_per_el1'))
    connect1.elem_type = 'HEX8'

    elem_idx = 0
    for k in range(n):
        for j in range(n):
            for i in range(n):
                # Node indices (0-based, will convert to 1-based)
                n0 = k * nodes_per_dim * nodes_per_dim + j * nodes_per_dim + i
                n1 = n0 + 1
                n2 = n0 + nodes_per_dim + 1
                n3 = n0 + nodes_per_dim
                n4 = n0 + nodes_per_dim * nodes_per_dim
                n5 = n4 + 1
                n6 = n4 + nodes_per_dim + 1
                n7 = n4 + nodes_per_dim

                # Convert to 1-based indexing
                connect1[elem_idx] = [n0+1, n1+1, n2+1, n3+1, n4+1, n5+1, n6+1, n7+1]
                elem_idx += 1

    # Generate connectivity for second cube
    nc.createDimension('num_el_in_blk2', num_elem_per_cube)
    nc.createDimension('num_nod_per_el2', 8)
    connect2 = nc.createVariable('connect2', 'i4', ('num_el_in_blk2', 'num_nod_per_el2'))
    connect2.elem_type = 'HEX8'

    # Offset for second cube nodes
    base_offset = num_nodes_per_cube

    elem_idx = 0
    for k in range(n):
        for j in range(n):
            for i in range(n):
                # For k=0 layer, bottom nodes are from shared face, top nodes from second cube's first layer
                if k == 0:
                    # Bottom nodes are from the top face of first cube (the shared interface)
                    n0 = (nodes_per_dim - 1) * nodes_per_dim * nodes_per_dim + j * nodes_per_dim + i
                    n1 = n0 + 1
                    n2 = n0 + nodes_per_dim + 1
                    n3 = n0 + nodes_per_dim
                    # Top nodes are from second cube's first layer
                    # Second cube's new nodes start at base_offset
                    n4_local = 0 * nodes_per_dim * nodes_per_dim + j * nodes_per_dim + i
                    n4 = base_offset + n4_local
                    n5 = n4 + 1
                    n6 = n4 + nodes_per_dim + 1
                    n7 = n4 + nodes_per_dim
                else:
                    # All nodes from second cube
                    n0_local = (k - 1) * nodes_per_dim * nodes_per_dim + j * nodes_per_dim + i
                    n0 = base_offset + n0_local
                    n1 = n0 + 1
                    n2 = n0 + nodes_per_dim + 1
                    n3 = n0 + nodes_per_dim
                    n4 = n0 + nodes_per_dim * nodes_per_dim
                    n5 = n4 + 1
                    n6 = n4 + nodes_per_dim + 1
                    n7 = n4 + nodes_per_dim

                # Convert to 1-based indexing
                connect2[elem_idx] = [n0+1, n1+1, n2+1, n3+1, n4+1, n5+1, n6+1, n7+1]
                elem_idx += 1

    nc.variables['eb_status'][:] = [1, 1]
    nc.variables['eb_prop1'][:] = [1, 2]

    nc.close()
    print(f"  Created: {filename}")


def generate_rotated_cube_contact():
    """
    Generate two 1"x1"x1" cubes with 10x10x10 elements.
    One cube is rotated 45 degrees about the Z axis.
    They share a contact surface but are misaligned due to rotation.
    """
    filename = 'test-data/rotated_cube_contact.exo'
    print(f"Generating {filename}...")

    n = 10
    nodes_per_dim = n + 1

    num_nodes_per_cube = nodes_per_dim ** 3
    num_nodes = 2 * num_nodes_per_cube  # No shared nodes due to rotation

    num_elem_per_cube = n ** 3
    num_elem = 2 * num_elem_per_cube
    num_el_blk = 2

    nc = create_exodus_file(filename, 3, num_nodes, num_elem, num_el_blk)

    coords = np.linspace(0.0, 1.0, nodes_per_dim)

    # First cube (not rotated)
    coords_x = []
    coords_y = []
    coords_z = []

    for k in range(nodes_per_dim):
        for j in range(nodes_per_dim):
            for i in range(nodes_per_dim):
                coords_x.append(coords[i])
                coords_y.append(coords[j])
                coords_z.append(coords[k])

    # Second cube (rotated 45 degrees about Z axis)
    # Center the rotation at (0.5, 0.5) so the cube rotates around its center
    angle = np.pi / 4  # 45 degrees
    cos_a = np.cos(angle)
    sin_a = np.sin(angle)

    for k in range(nodes_per_dim):
        for j in range(nodes_per_dim):
            for i in range(nodes_per_dim):
                # Original position (centered at origin)
                x = coords[i] - 0.5
                y = coords[j] - 0.5
                z = coords[k] + 1.0  # offset up by 1 inch

                # Rotate about Z axis
                x_rot = cos_a * x - sin_a * y + 0.5
                y_rot = sin_a * x + cos_a * y + 0.5

                coords_x.append(x_rot)
                coords_y.append(y_rot)
                coords_z.append(z)

    nc.variables['coordx'][:] = np.array(coords_x)
    nc.variables['coordy'][:] = np.array(coords_y)
    nc.variables['coordz'][:] = np.array(coords_z)

    # Generate connectivity for first cube
    nc.createDimension('num_el_in_blk1', num_elem_per_cube)
    nc.createDimension('num_nod_per_el1', 8)
    connect1 = nc.createVariable('connect1', 'i4', ('num_el_in_blk1', 'num_nod_per_el1'))
    connect1.elem_type = 'HEX8'

    elem_idx = 0
    for k in range(n):
        for j in range(n):
            for i in range(n):
                n0 = k * nodes_per_dim * nodes_per_dim + j * nodes_per_dim + i
                n1 = n0 + 1
                n2 = n0 + nodes_per_dim + 1
                n3 = n0 + nodes_per_dim
                n4 = n0 + nodes_per_dim * nodes_per_dim
                n5 = n4 + 1
                n6 = n4 + nodes_per_dim + 1
                n7 = n4 + nodes_per_dim

                connect1[elem_idx] = [n0+1, n1+1, n2+1, n3+1, n4+1, n5+1, n6+1, n7+1]
                elem_idx += 1

    # Generate connectivity for second cube (rotated)
    nc.createDimension('num_el_in_blk2', num_elem_per_cube)
    nc.createDimension('num_nod_per_el2', 8)
    connect2 = nc.createVariable('connect2', 'i4', ('num_el_in_blk2', 'num_nod_per_el2'))
    connect2.elem_type = 'HEX8'

    base_offset = num_nodes_per_cube

    elem_idx = 0
    for k in range(n):
        for j in range(n):
            for i in range(n):
                n0 = base_offset + k * nodes_per_dim * nodes_per_dim + j * nodes_per_dim + i
                n1 = n0 + 1
                n2 = n0 + nodes_per_dim + 1
                n3 = n0 + nodes_per_dim
                n4 = n0 + nodes_per_dim * nodes_per_dim
                n5 = n4 + 1
                n6 = n4 + nodes_per_dim + 1
                n7 = n4 + nodes_per_dim

                connect2[elem_idx] = [n0+1, n1+1, n2+1, n3+1, n4+1, n5+1, n6+1, n7+1]
                elem_idx += 1

    nc.variables['eb_status'][:] = [1, 1]
    nc.variables['eb_prop1'][:] = [1, 2]

    nc.close()
    print(f"  Created: {filename}")


def generate_cube_cylinder_contact():
    """
    Generate a cube (10x10x10 elements) in contact with a cylinder.
    The flat face of the cylinder is in contact with the top face of the cube.
    """
    filename = 'test-data/cube_cylinder_contact.exo'
    print(f"Generating {filename}...")

    # Cube parameters
    n_cube = 10
    nodes_per_dim = n_cube + 1

    # Cylinder parameters (radial, circumferential, height)
    n_radial = 5
    n_circum = 20
    n_height = 10
    radius = 0.5
    height = 1.0

    # Calculate totals
    num_nodes_cube = nodes_per_dim ** 3
    num_nodes_cylinder = (n_radial + 1) * n_circum * (n_height + 1)
    num_nodes = num_nodes_cube + num_nodes_cylinder

    num_elem_cube = n_cube ** 3
    # Skip innermost radial layer to avoid degenerate elements at center
    num_elem_cylinder = (n_radial - 1) * n_circum * n_height
    num_elem = num_elem_cube + num_elem_cylinder
    num_el_blk = 2

    nc = create_exodus_file(filename, 3, num_nodes, num_elem, num_el_blk)

    coords = np.linspace(0.0, 1.0, nodes_per_dim)

    # Generate cube coordinates
    coords_x = []
    coords_y = []
    coords_z = []

    for k in range(nodes_per_dim):
        for j in range(nodes_per_dim):
            for i in range(nodes_per_dim):
                coords_x.append(coords[i])
                coords_y.append(coords[j])
                coords_z.append(coords[k])

    # Generate cylinder coordinates (centered at 0.5, 0.5, starting at z=1.0)
    for k in range(n_height + 1):
        z = 1.0 + (k / n_height) * height
        for j in range(n_circum):
            theta = (j / n_circum) * 2 * np.pi
            for i in range(n_radial + 1):
                r = (i / n_radial) * radius
                x = 0.5 + r * np.cos(theta)
                y = 0.5 + r * np.sin(theta)
                coords_x.append(x)
                coords_y.append(y)
                coords_z.append(z)

    nc.variables['coordx'][:] = np.array(coords_x)
    nc.variables['coordy'][:] = np.array(coords_y)
    nc.variables['coordz'][:] = np.array(coords_z)

    # Generate connectivity for cube
    nc.createDimension('num_el_in_blk1', num_elem_cube)
    nc.createDimension('num_nod_per_el1', 8)
    connect1 = nc.createVariable('connect1', 'i4', ('num_el_in_blk1', 'num_nod_per_el1'))
    connect1.elem_type = 'HEX8'

    elem_idx = 0
    for k in range(n_cube):
        for j in range(n_cube):
            for i in range(n_cube):
                n0 = k * nodes_per_dim * nodes_per_dim + j * nodes_per_dim + i
                n1 = n0 + 1
                n2 = n0 + nodes_per_dim + 1
                n3 = n0 + nodes_per_dim
                n4 = n0 + nodes_per_dim * nodes_per_dim
                n5 = n4 + 1
                n6 = n4 + nodes_per_dim + 1
                n7 = n4 + nodes_per_dim

                connect1[elem_idx] = [n0+1, n1+1, n2+1, n3+1, n4+1, n5+1, n6+1, n7+1]
                elem_idx += 1

    # Generate connectivity for cylinder
    nc.createDimension('num_el_in_blk2', num_elem_cylinder)
    nc.createDimension('num_nod_per_el2', 8)
    connect2 = nc.createVariable('connect2', 'i4', ('num_el_in_blk2', 'num_nod_per_el2'))
    connect2.elem_type = 'HEX8'

    base_offset = num_nodes_cube
    nodes_per_ring = (n_radial + 1) * n_circum

    elem_idx = 0
    for k in range(n_height):
        for j in range(n_circum):
            j_next = (j + 1) % n_circum
            # Skip i=0 to avoid degenerate elements at the center axis
            # Start from i=1 to create a hollow cylinder
            for i in range(1, n_radial):
                # Bottom ring nodes
                n0 = base_offset + k * nodes_per_ring + j * (n_radial + 1) + i
                n1 = n0 + 1
                n2 = base_offset + k * nodes_per_ring + j_next * (n_radial + 1) + i + 1
                n3 = n2 - 1

                # Top ring nodes
                n4 = n0 + nodes_per_ring
                n5 = n1 + nodes_per_ring
                n6 = n2 + nodes_per_ring
                n7 = n3 + nodes_per_ring

                connect2[elem_idx] = [n0+1, n1+1, n2+1, n3+1, n4+1, n5+1, n6+1, n7+1]
                elem_idx += 1

    nc.variables['eb_status'][:] = [1, 1]
    nc.variables['eb_prop1'][:] = [1, 2]

    nc.close()
    print(f"  Created: {filename}")


if __name__ == "__main__":
    print("Generating test Exodus II mesh files...")
    print("=" * 60)

    generate_single_hex_contact()
    generate_aligned_cubes_10x10x10()
    generate_rotated_cube_contact()
    generate_cube_cylinder_contact()

    print("=" * 60)
    print("All test mesh files generated successfully!")
    print("\nGenerated files:")
    print("  - test-data/single_hex_contact.exo")
    print("  - test-data/aligned_cubes_10x10x10.exo")
    print("  - test-data/rotated_cube_contact.exo")
    print("  - test-data/cube_cylinder_contact.exo")
