use criterion::{Criterion, criterion_group, criterion_main};

fn setup_tracing() {
    #[cfg(feature = "bench-trace")]
    {
        use tracing_subscriber::fmt::format::FmtSpan;
        tracing_subscriber::fmt()
            .with_span_events(FmtSpan::CLOSE)
            .with_target(false)
            .with_level(false)
            .init();
    }
}

fn bench_swc(_c: &mut Criterion) {
    setup_tracing();
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/transform_examples/index.ts"
    ))
    .unwrap();

    #[cfg(feature = "bench-trace")]
    {
        checkpoint_parser::swc::transform_code(&source, "index.ts", false).unwrap();
    }

    #[cfg(not(feature = "bench-trace"))]
    _c.bench_function("swc::total", |b| {
        b.iter(|| checkpoint_parser::swc::transform_code(&source, "index.ts", false).unwrap());
    });
}

criterion_group!(benches, bench_swc);
criterion_main!(benches);
