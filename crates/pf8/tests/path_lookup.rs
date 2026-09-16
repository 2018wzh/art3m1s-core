use pf8::{Pf8Builder, Pf8Reader};
use std::fs;

#[test]
fn nested_entries_use_the_same_key_for_index_and_lookup() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("input.txt");
    let output = directory.path().join("archive.pfs");
    let contents = b"archive separator regression";
    fs::write(&input, contents).unwrap();
    let mut builder = Pf8Builder::new();
    builder
        .add_file_as(&input, "Mixed/Nested/Text.txt")
        .unwrap();
    builder.write_to_file(&output).unwrap();

    let mut reader = Pf8Reader::open(&output).unwrap();
    for query in [
        "mixed/nested/text.txt",
        r"MIXED\NESTED\TEXT.TXT",
        r"Mixed/Nested\Text.txt",
    ] {
        assert!(reader.contains(query));
        assert_eq!(reader.read_file(query).unwrap(), contents);
        let mut part = [0; 9];
        assert_eq!(reader.read_range(query, 8, &mut part).unwrap(), part.len());
        assert_eq!(&part, &contents[8..17]);
    }
    assert!(!reader.contains("mixed/nested/missing.txt"));
}
