use criterion::{black_box, criterion_group, criterion_main, Criterion};
use csln_core::{InputBibliography, Style};
use std::fs;
use std::path::PathBuf;

fn bench_formats(c: &mut Criterion) {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir.parent().unwrap().parent().unwrap();

    let style_path = root_dir.join("styles/apa-7th.yaml");
    let style_yaml = fs::read_to_string(&style_path).expect("failed to read apa-7th.yaml");
    let style: Style = serde_yaml::from_str(&style_yaml).expect("failed to parse style yaml");

    let style_json = serde_json::to_string(&style).expect("failed to serialize style to json");
    let style_cbor = serde_cbor::to_vec(&style).expect("failed to serialize style to cbor");

    let mut group = c.benchmark_group("Style Deserialization");

    group.bench_function("YAML", |b| {
        b.iter(|| {
            let _: Style = serde_yaml::from_str(black_box(&style_yaml)).unwrap();
        })
    });

    group.bench_function("JSON", |b| {
        b.iter(|| {
            let _: Style = serde_json::from_str(black_box(&style_json)).unwrap();
        })
    });

    group.bench_function("CBOR", |b| {
        b.iter(|| {
            let _: Style = serde_cbor::from_slice(black_box(&style_cbor)).unwrap();
        })
    });

    group.finish();

    let bib_path = root_dir.join("examples/comprehensive.yaml");
    let bib_yaml = fs::read_to_string(&bib_path).expect("failed to read comprehensive.yaml");
    let bib: InputBibliography = serde_yaml::from_str(&bib_yaml).expect("failed to parse bib yaml");

    let bib_json = serde_json::to_string(&bib).expect("failed to serialize bib to json");
    let bib_cbor = serde_cbor::to_vec(&bib).expect("failed to serialize bib to cbor");

    let mut group = c.benchmark_group("Bibliography Deserialization");

    group.bench_function("YAML", |b| {
        b.iter(|| {
            let _: InputBibliography = serde_yaml::from_str(black_box(&bib_yaml)).unwrap();
        })
    });

    group.bench_function("JSON", |b| {
        b.iter(|| {
            let _: InputBibliography = serde_json::from_str(black_box(&bib_json)).unwrap();
        })
    });

    group.bench_function("CBOR", |b| {
        b.iter(|| {
            let _: InputBibliography = serde_cbor::from_slice(black_box(&bib_cbor)).unwrap();
        })
    });

    group.finish();
}

criterion_group!(benches, bench_formats);
criterion_main!(benches);
