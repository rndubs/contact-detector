//! Performance benchmarks for contact-detector
//!
//! # Running Benchmarks
//!
//! Run all benchmarks:
//! ```bash
//! cargo bench
//! ```
//!
//! Run specific benchmark group:
//! ```bash
//! cargo bench --bench performance surface_extraction
//! cargo bench --bench performance contact_detection
//! cargo bench --bench performance kdtree
//! cargo bench --bench performance pipeline
//! ```
//!
//! View HTML reports:
//! ```bash
//! open target/criterion/report/index.html
//! ```
//!
//! # Benchmark Groups
//!
//! - **surface_extraction**: Tests surface skinning algorithm at different scales
//! - **contact_detection**: Tests contact pair detection at different scales
//! - **kdtree**: Tests k-d tree construction and query performance
//! - **pipeline**: Tests complete end-to-end pipeline
//!
//! # Scale Targets
//!
//! - 1K elements: Small test case
//! - 10K elements: Medium test case
//! - 100K elements: Large test case
//! - 1M elements: Target scale (should complete in ≤30s)

use contact_detector::contact::detection::detect_contact_pairs;
use contact_detector::contact::types::ContactCriteria;
use contact_detector::mesh::surface::extract_surface;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use kiddo::KdTree;

mod synthetic_mesh;
use synthetic_mesh::{calculate_grid_dimensions, generate_hex_grid, generate_parallel_surfaces};

/// Benchmark surface extraction at different scales
fn benchmark_surface_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("surface_extraction");

    // Test at different scales
    let scales = vec![
        ("1K", 1_000),
        ("10K", 10_000),
        ("100K", 100_000),
        // Commenting out 1M for initial runs - uncomment when ready for full scale testing
        // ("1M", 1_000_000),
    ];

    for (name, target_elements) in scales {
        let (nx, ny, nz) = calculate_grid_dimensions(target_elements);
        let actual_elements = nx * ny * nz;

        // Generate mesh once before benchmark
        let mesh = generate_hex_grid(nx, ny, nz, 1.0);

        group.throughput(Throughput::Elements(actual_elements as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &mesh,
            |b, mesh| {
                b.iter(|| {
                    let surfaces = extract_surface(black_box(mesh)).unwrap();
                    black_box(surfaces);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark k-d tree construction and queries
fn benchmark_kdtree(c: &mut Criterion) {
    let mut group = c.benchmark_group("kdtree");

    // Test k-d tree operations at different scales
    let scales = vec![
        ("1K_faces", 1_000),
        ("10K_faces", 10_000),
        ("100K_faces", 100_000),
        // ("1M_faces", 1_000_000),
    ];

    for (name, num_points) in scales {
        // Generate points for k-d tree with sufficient variation
        // Use a simple pseudo-random approach based on index
        let points: Vec<[f64; 3]> = (0..num_points)
            .map(|i| {
                let i_f = i as f64;
                // Use different prime number multiples to avoid alignment
                let x = ((i_f * 0.123456) % 100.0) + (i_f * 0.000001);
                let y = ((i_f * 0.234567) % 100.0) + (i_f * 0.000002);
                let z = ((i_f * 0.345678) % 100.0) + (i_f * 0.000003);
                [x, y, z]
            })
            .collect();

        // Benchmark tree construction
        group.throughput(Throughput::Elements(num_points as u64));
        group.bench_with_input(
            BenchmarkId::new("construction", name),
            &points,
            |b, points| {
                b.iter(|| {
                    let mut tree = KdTree::new();
                    for (idx, point) in points.iter().enumerate() {
                        tree.add(point, idx as u64);
                    }
                    black_box(tree);
                });
            },
        );

        // Build tree for query benchmarks
        let mut tree = KdTree::new();
        for (idx, point) in points.iter().enumerate() {
            tree.add(point, idx as u64);
        }

        // Benchmark radius queries
        let query_point = [50.0, 50.0, 50.0];
        let radius = 5.0;

        group.bench_with_input(
            BenchmarkId::new("radius_query", name),
            &tree,
            |b, tree| {
                b.iter(|| {
                    let results = tree.within::<kiddo::SquaredEuclidean>(
                        black_box(&query_point),
                        black_box(radius * radius),
                    );
                    black_box(results);
                });
            },
        );

        // Benchmark nearest neighbor queries
        group.bench_with_input(
            BenchmarkId::new("nearest_query", name),
            &tree,
            |b, tree| {
                b.iter(|| {
                    let results = tree.nearest_n::<kiddo::SquaredEuclidean>(
                        black_box(&query_point),
                        black_box(10),
                    );
                    black_box(results);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark contact detection at different scales
fn benchmark_contact_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("contact_detection");
    // Increase sample size and measurement time for slower operations
    group.sample_size(10);

    let scales = vec![
        ("100_faces", 10, 10), // 100 faces per surface
        ("1K_faces", 32, 32),  // ~1K faces per surface
        ("10K_faces", 100, 100), // 10K faces per surface
        // ("100K_faces", 316, 316), // ~100K faces per surface
    ];

    for (name, nx, ny) in scales {
        let actual_faces = nx * ny;

        // Generate parallel surfaces
        let (mesh_a, mesh_b) = generate_parallel_surfaces(nx, ny, 0.001, 1.0);

        // Extract surfaces
        let surfaces_a = extract_surface(&mesh_a).unwrap();
        let surfaces_b = extract_surface(&mesh_b).unwrap();

        let surface_a = &surfaces_a[0];
        let surface_b = &surfaces_b[0];

        // Define contact criteria
        let criteria = ContactCriteria::new(0.005, 0.001, 45.0);

        group.throughput(Throughput::Elements(actual_faces as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &(surface_a, surface_b, &criteria),
            |b, (surface_a, surface_b, criteria)| {
                b.iter(|| {
                    let results = detect_contact_pairs(
                        black_box(surface_a),
                        black_box(surface_b),
                        black_box(criteria),
                    )
                    .unwrap();
                    black_box(results);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark complete pipeline (surface extraction + contact detection)
fn benchmark_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline");
    // Reduce sample size for the full pipeline benchmarks as they take longer
    group.sample_size(10);

    let scales = vec![
        ("100_elements", 10, 10), // 100 elements per mesh
        ("1K_elements", 32, 32),  // ~1K elements per mesh
        ("10K_elements", 100, 100), // 10K elements per mesh
        // Uncomment for larger scale testing
        // ("100K_elements", 316, 316), // ~100K elements per mesh
    ];

    for (name, nx, ny) in scales {
        let actual_elements = nx * ny;

        // Generate parallel surfaces (meshes)
        let (mesh_a, mesh_b) = generate_parallel_surfaces(nx, ny, 0.001, 1.0);

        let criteria = ContactCriteria::new(0.005, 0.001, 45.0);

        group.throughput(Throughput::Elements(actual_elements as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &(mesh_a, mesh_b, &criteria),
            |b, (mesh_a, mesh_b, criteria)| {
                b.iter(|| {
                    // Extract surfaces
                    let surfaces_a = extract_surface(black_box(mesh_a)).unwrap();
                    let surfaces_b = extract_surface(black_box(mesh_b)).unwrap();

                    let surface_a = &surfaces_a[0];
                    let surface_b = &surfaces_b[0];

                    // Detect contact pairs
                    let results = detect_contact_pairs(
                        black_box(surface_a),
                        black_box(surface_b),
                        black_box(criteria),
                    )
                    .unwrap();

                    black_box(results);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark for 1M element target (separate group for long-running test)
#[allow(dead_code)]
fn benchmark_1m_target(c: &mut Criterion) {
    let mut group = c.benchmark_group("1M_target");

    // Configure for very long benchmarks
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(120));

    // Generate 1M element mesh
    let (nx, ny, nz) = calculate_grid_dimensions(1_000_000);
    let actual_elements = nx * ny * nz;

    println!("Generating mesh with {} elements ({}x{}x{})", actual_elements, nx, ny, nz);
    let mesh = generate_hex_grid(nx, ny, nz, 1.0);
    println!("Mesh generated with {} nodes, {} elements", mesh.num_nodes(), mesh.num_elements());

    group.throughput(Throughput::Elements(actual_elements as u64));
    group.bench_function("surface_extraction_1M", |b| {
        b.iter(|| {
            let surfaces = extract_surface(black_box(&mesh)).unwrap();
            black_box(surfaces);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_surface_extraction,
    benchmark_kdtree,
    benchmark_contact_detection,
    benchmark_pipeline,
);

// Separate group for 1M element benchmark (commented out by default)
// Uncomment to run the full 1M element test
criterion_group!(
    name = long_benches;
    config = Criterion::default();
    targets = benchmark_1m_target
);

// criterion_main!(benches);
// To include 1M element benchmarks, uncomment the line below and comment out the line above
criterion_main!(benches, long_benches);
