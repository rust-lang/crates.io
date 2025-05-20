use crates_io_cdn_logs::{cloudfront, fastly};
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::io::Cursor;

fn criterion_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let bytes = include_bytes!("../test_data/cloudfront/basic.log");
    c.bench_function("cloudfront", |b| {
        // Insert a call to `to_async` to convert the bencher to async mode.
        // The timing loops are the same as with the normal bencher.
        b.to_async(&rt)
            .iter(|| cloudfront::count_downloads(black_box(Cursor::new(bytes))));
    });

    let bytes = include_bytes!("../test_data/fastly/basic.log");
    c.bench_function("fastly", |b| {
        // Insert a call to `to_async` to convert the bencher to async mode.
        // The timing loops are the same as with the normal bencher.
        b.to_async(&rt)
            .iter(|| fastly::count_downloads(black_box(Cursor::new(bytes))));
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
