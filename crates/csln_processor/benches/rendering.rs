use criterion::{black_box, criterion_group, criterion_main, Criterion};
use csln_core::{InputBibliography, Style};
use csln_processor::{Bibliography, Citation, CitationItem, Processor};
use std::fs;
use std::path::PathBuf;

fn bench_rendering(c: &mut Criterion) {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir.parent().unwrap().parent().unwrap();

    // Load style
    let style_path = root_dir.join("styles/apa-7th.yaml");
    let style_yaml = fs::read_to_string(&style_path).expect("failed to read apa-7th.yaml");
    let style: Style = serde_yaml::from_str(&style_yaml).expect("failed to parse style yaml");

    // Load bibliography
    let bib_path = root_dir.join("examples/comprehensive.yaml");
    let bib_yaml = fs::read_to_string(&bib_path).expect("failed to read comprehensive.yaml");
    let input_bib: InputBibliography =
        serde_yaml::from_str(&bib_yaml).expect("failed to parse bib yaml");

    // Convert to processor bibliography
    let mut bib = Bibliography::new();
    for r in input_bib.references {
        if let Some(id) = r.id() {
            bib.insert(id.to_string(), r);
        }
    }

    // Benchmark Citation Processing (single item)
    let first_id = bib.keys().next().unwrap().clone();
    let citation = Citation {
        items: vec![CitationItem {
            id: first_id,
            ..Default::default()
        }],
        ..Default::default()
    };

    c.bench_function("Process Citation (APA)", |b| {
        let processor = Processor::new(style.clone(), bib.clone());
        b.iter(|| {
            processor.process_citation(black_box(&citation)).unwrap();
        })
    });

    // Benchmark Bibliography Processing (full set)
    c.bench_function("Process Bibliography (APA, 10 items)", |b| {
        let processor = Processor::new(style.clone(), bib.clone());
        b.iter(|| {
            processor.process_references();
        })
    });
}

criterion_group!(benches, bench_rendering);
criterion_main!(benches);
