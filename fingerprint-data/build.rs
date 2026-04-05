use std::fs;
use std::io::BufReader;
use zip::ZipArchive;

fn validate_zip(path: &str) {
    println!("cargo:rerun-if-changed={path}");

    let file = fs::File::open(path)
        .unwrap_or_else(|e| panic!("fingerprint-data: Cannot open '{path}': {e}"));

    let reader = BufReader::new(file);
    let mut archive = ZipArchive::new(reader)
        .unwrap_or_else(|e| panic!("fingerprint-data: '{path}' is not a valid ZIP: {e}"));

    let has_json = (0..archive.len()).any(|i| {
        archive
            .by_index(i)
            .map(|entry| entry.name().ends_with(".json"))
            .unwrap_or(false)
    });

    if !has_json {
        panic!("fingerprint-data: '{path}' contains no .json entries — corrupted or wrong file");
    }
}

fn main() {
    validate_zip("data/header-network-definition.zip");
    validate_zip("data/fingerprint-network-definition.zip");
}
