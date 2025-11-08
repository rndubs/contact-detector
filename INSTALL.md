# Installation Guide

## Prerequisites

### Required
- Rust toolchain (1.70+): https://rustup.rs/
- C compiler (gcc or clang)

### Optional - For Exodus II Support
To build with Exodus II (.exo) file support, you need:

- libhdf5-dev
- libnetcdf-dev
- pkg-config

#### Installing on Debian/Ubuntu:
```bash
sudo apt-get install libhdf5-dev libnetcdf-dev pkg-config
```

#### Installing on macOS:
```bash
brew install hdf5 netcdf pkg-config
```

## Building

### Standard Build (JSON mesh format only)
```bash
cargo build --release
```

### Build with Exodus II Support
```bash
cargo build --release --features exodus
```

## Testing

Run the unit tests:
```bash
cargo test
```

Run with example data:
```bash
./target/release/contact-detector info test-data/simple-cube.json
```

## Installation

Install to your system:
```bash
cargo install --path .
```

Or with Exodus support:
```bash
cargo install --path . --features exodus
```

## Troubleshooting

### HDF5/NetCDF Build Errors

If you get build errors related to HDF5 or NetCDF:

1. Make sure you have the development libraries installed (not just runtime libraries)
2. Check that pkg-config can find them:
   ```bash
   pkg-config --libs hdf5
   pkg-config --libs netcdf
   ```
3. If libraries are in non-standard locations, set environment variables:
   ```bash
   export HDF5_DIR=/path/to/hdf5
   export NETCDF_DIR=/path/to/netcdf
   ```

### Alternative: Use JSON Format

If you can't install HDF5/NetCDF, you can:
1. Build without the `exodus` feature (JSON support only)
2. Convert Exodus files to JSON using the provided Python script:
   ```bash
   python test-data/exodus_to_json.py input.exo output.json
   ```
