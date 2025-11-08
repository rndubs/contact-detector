# Installation Guide

## Prerequisites

### Required
- Rust toolchain (1.70+): https://rustup.rs/
- CMake and C++ compiler (for building HDF5/NetCDF from source)

**Installing on Debian/Ubuntu:**
```bash
sudo apt-get install cmake g++
```

**Installing on macOS:**
```bash
brew install cmake
# Xcode Command Line Tools provides g++
```

## Building

### Standard Build (Exodus II support included)
Builds HDF5 and NetCDF from source - **no system libraries required!**

```bash
cargo build --release
```

*Note: First build takes ~3-5 minutes as it compiles HDF5 and NetCDF from source. Subsequent builds are faster.*

### Alternative Build Options

#### Build without Exodus II support (JSON only, faster builds)
```bash
cargo build --release --no-default-features
```

#### Build with system libraries (if you have libhdf5-dev and libnetcdf-dev installed)
```bash
cargo build --release --no-default-features --features exodus
```

**Requirements for system library build:**
```bash
# Debian/Ubuntu
sudo apt-get install libhdf5-dev libnetcdf-dev pkg-config

# macOS
brew install hdf5 netcdf pkg-config
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

Install to your system (with Exodus II support):
```bash
cargo install --path .
```

Install without Exodus support (JSON only):
```bash
cargo install --path . --no-default-features
```

Install with system libraries instead of static build:
```bash
cargo install --path . --no-default-features --features exodus
```

## Troubleshooting

### Build Errors

If you get errors during the default build:

1. **CMake not found**: Install CMake (`sudo apt-get install cmake` or `brew install cmake`)
2. **C++ compiler errors**: Install g++ (`sudo apt-get install g++`) or ensure Xcode Command Line Tools are installed on macOS
3. **Long build times**: The first build compiles HDF5 and NetCDF from source (~3-5 minutes). Subsequent builds are faster.
4. **Want faster builds?**: Use `--no-default-features` to skip Exodus support, or install system libraries and use `--features exodus`

### System Library Build Errors

If you get build errors with `--features exodus`:

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

If you can't install build tools or system libraries, you can:
1. Build without Exodus support (JSON format only): `cargo build --release`
2. Convert Exodus files to JSON using the provided Python script:
   ```bash
   python test-data/exodus_to_json.py input.exo output.json
   ```
