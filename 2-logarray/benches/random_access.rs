use criterion::{
    black_box, criterion_group, criterion_main, measurement::Measurement, Bencher, BenchmarkGroup,
    Criterion, Throughput,
};

use basic_logarray::naive::LogArray;
use rand::prelude::*;

fn random_data<R: Rng>(width: u8, len: usize, mut rng: R) -> Vec<u64> {
    if width == 64 {
        (0..len).map(|_| rng.gen()).collect()
    } else {
        (0..len).map(|_| rng.gen_range(0..(1 << width))).collect()
    }
}

fn random_indexes<R: Rng>(len: usize, number_of_indexes: usize, mut rng: R) -> Vec<usize> {
    (0..number_of_indexes)
        .map(|_| rng.gen_range(0..len))
        .collect()
}

fn vec16_load(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(0x533D);
    let data: Vec<u16> = random_data(10, 1_000_000, &mut rng)
        .into_iter()
        .map(|i| i as u16)
        .collect();
    let indexes = random_indexes(1_000_000, 1_000, &mut rng);
    let mut g = c.benchmark_group("vec16 load");
    g.throughput(Throughput::Elements(1_000));
    g.bench_function("vec16 load", |b| {
        b.iter(|| {
            for &i in indexes.iter() {
                black_box(data[i]);
            }
        })
    });
}

fn vec32_load(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(0x533D);
    let data: Vec<u32> = random_data(10, 1_000_000, &mut rng)
        .into_iter()
        .map(|i| i as u32)
        .collect();
    let indexes = random_indexes(1_000_000, 1_000, &mut rng);
    let mut g = c.benchmark_group("vec32 load");
    g.throughput(Throughput::Elements(1_000));
    g.bench_function("vec32 load", |b| {
        b.iter(|| {
            for &i in indexes.iter() {
                black_box(data[i]);
            }
        })
    });
}

fn vec64_load(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(0x533D);
    let data: Vec<u64> = random_data(10, 1_000_000, &mut rng);
    let indexes = random_indexes(1_000_000, 1_000, &mut rng);
    let mut g = c.benchmark_group("vec64 load");
    g.throughput(Throughput::Elements(1_000));
    g.bench_function("vec64 load", |b| {
        b.iter(|| {
            for &i in indexes.iter() {
                black_box(data[i]);
            }
        })
    });
}

fn logarray_load(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(0x533D);
    let mut logarray = LogArray::new(10, 1_000_000);
    for (ix, v) in random_data(10, 1_000_000, &mut rng).into_iter().enumerate() {
        logarray.store(ix, v);
    }
    let indexes = random_indexes(1_000_000, 1_000, &mut rng);
    let mut g = c.benchmark_group("logarray load");
    g.throughput(Throughput::Elements(1_000));
    g.bench_function("logarray load", |b| {
        b.iter(|| {
            for &i in indexes.iter() {
                black_box(logarray.load(i));
            }
        })
    });
}

criterion_group!(benches, vec16_load, vec32_load, vec64_load, logarray_load);
criterion_main!(benches);
