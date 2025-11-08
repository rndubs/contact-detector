# /// script
# dependencies = [
#  "numpy",
#  "netCDF4",
# ]
# ///
"""
Simple script to inspect Exodus II file structure.
Run this script with uv so it will install the dependencies for you:
    - uv run inspect_exodus.py
"""

import sys
try:
    from netCDF4 import Dataset
    import numpy as np
except ImportError:
    print("Error: netCDF4 not installed. Run: pip install netCDF4 numpy")
    sys.exit(1)


def inspect_exodus(filename):
    """Inspect an Exodus II file and print its structure."""
    print(f"\n{'='*60}")
    print(f"Exodus II File: {filename}")
    print(f"{'='*60}\n")

    with Dataset(filename, 'r') as nc:
        # Print dimensions
        print("DIMENSIONS:")
        for dim_name, dim in nc.dimensions.items():
            print(f"  {dim_name}: {len(dim)}")

        print("\nGLOBAL ATTRIBUTES:")
        for attr_name in nc.ncattrs():
            print(f"  {attr_name}: {nc.getncattr(attr_name)}")

        print("\nVARIABLES:")
        for var_name, var in nc.variables.items():
            shape_str = 'x'.join(str(d) for d in var.shape) if var.shape else 'scalar'
            print(f"  {var_name}: {var.dtype} ({shape_str})")

        # Print coordinate info
        num_nodes = len(nc.dimensions.get('num_nodes', []))
        num_elem = len(nc.dimensions.get('num_elem', []))
        num_dim = len(nc.dimensions.get('num_dim', []))

        print(f"\nMESH STATISTICS:")
        print(f"  Dimensions: {num_dim}")
        print(f"  Nodes: {num_nodes}")
        print(f"  Elements: {num_elem}")

        # Print element blocks
        print(f"\nELEMENT BLOCKS:")
        num_el_blk = len(nc.dimensions.get('num_el_blk', []))
        if num_el_blk > 0:
            for i in range(1, num_el_blk + 1):
                try:
                    # Element block metadata
                    blk_name = f"eb_{i}"

                    # Get element type if available
                    conn_var_name = f"connect{i}"
                    if conn_var_name in nc.variables:
                        conn = nc.variables[conn_var_name]
                        elem_type = conn.elem_type if hasattr(conn, 'elem_type') else 'unknown'
                        num_elems_in_blk = conn.shape[0] if len(conn.shape) > 0 else 0
                        num_nodes_per_elem = conn.shape[1] if len(conn.shape) > 1 else 0
                        print(f"  Block {i}: {elem_type} ({num_elems_in_blk} elements, {num_nodes_per_elem} nodes/elem)")
                except:
                    pass

        # Print nodesets
        num_node_sets = len(nc.dimensions.get('num_node_sets', []))
        if num_node_sets > 0:
            print(f"\nNODE SETS: {num_node_sets}")

        # Print sidesets
        num_side_sets = len(nc.dimensions.get('num_side_sets', []))
        if num_side_sets > 0:
            print(f"SIDE SETS: {num_side_sets}")

        print(f"\n{'='*60}\n")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python inspect_exodus.py <exodus_file.exo>")
        sys.exit(1)

    inspect_exodus(sys.argv[1])
