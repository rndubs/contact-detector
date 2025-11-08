# Installation Guide

## Prerequisites

### Required
- Rust toolchain (1.70+): https://rustup.rs/
- C compiler (gcc or clang)

### Optional - For Exodus II Support

You have **two options** for building with Exodus II (.exo) file support:

#### Option 1: Static Build (Recommended)
Builds HDF5 and NetCDF from source - **no system libraries required!**

**Requirements:**
- CMake
- C++ compiler (g++ or clang++)

**Installing build tools on Debian/Ubuntu:**
```bash
sudo apt-get install cmake g++
```

**Installing build tools on macOS:**
```bash
brew install cmake
# Xcode Command Line Tools provides g++
```

#### Option 2: System Libraries
Uses pre-installed HDF5 and NetCDF libraries.

**Requirements:**
- libhdf5-dev
- libnetcdf-dev
- pkg-config

**Installing on Debian/Ubuntu:**
```bash
sudo apt-get install libhdf5-dev libnetcdf-dev pkg-config
```

**Installing on macOS:**
```bash
brew install hdf5 netcdf pkg-config
```

## Building

### Standard Build (JSON mesh format only)
```bash
cargo build --release
```

### Build with Exodus II Support (Static - Recommended)
Builds HDF5/NetCDF from source (takes ~3-5 minutes on first build):
```bash
cargo build --release --features exodus-static
```

### Build with Exodus II Support (System Libraries)
Uses system-installed libraries (faster builds):
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

Install to your system (JSON format only):
```bash
cargo install --path .
```

With Exodus support (static build):
```bash
cargo install --path . --features exodus-static
```

With Exodus support (system libraries):
```bash
cargo install --path . --features exodus
```

## Troubleshooting

### Static Build Errors

If you get errors when building with `--features exodus-static`:

1. **CMake not found**: Install CMake (`sudo apt-get install cmake` or `brew install cmake`)
2. **C++ compiler errors**: Install g++ (`sudo apt-get install g++`) or ensure Xcode Command Line Tools are installed on macOS
3. **Long build times**: The first build compiles HDF5 and NetCDF from source (~3-5 minutes). Subsequent builds are faster.

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
