use super::*;
use calamine::Data;
use std::path::PathBuf;

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/aena")
}

fn open_test_file(filename: &str) -> calamine::Sheets<std::io::BufReader<std::fs::File>> {
    let path = data_dir().join(filename);
    open_workbook_auto(&path).unwrap_or_else(|e| panic!("Failed to open {}: {}", filename, e))
}

fn get_data_range(wb: &mut calamine::Sheets<std::io::BufReader<std::fs::File>>) -> calamine::Range<Data> {
    let sheet = wb.sheet_names().to_vec()
        .into_iter()
        .find(|s| s.to_lowercase() != "mozart reports")
        .expect("No data sheet");
    wb.worksheet_range(&sheet).expect("Failed to read sheet")
}

/// Build search terms the same way the real code does, from an Airport-like struct.
fn madrid_terms() -> Vec<String> {
    vec!["madrid barajas".into(), "madrid".into(), "madrid-barajas".into()]
}

fn barcelona_terms() -> Vec<String> {
    vec!["barcelona el prat".into(), "barcelona".into(), "barcelona-el-prat".into()]
}

fn malaga_terms() -> Vec<String> {
    vec!["málaga-costa del sol".into(), "málaga".into(), "malaga".into()]
}

fn valencia_terms() -> Vec<String> {
    vec!["valencia".into()]
}

#[test]
fn year_from_filename_works() {
    assert_eq!(year_from_filename("DEFINITIVOS+2024.xlsx"), Some(2024));
    assert_eq!(year_from_filename("0.Anual_Definitivo_2018.xls"), Some(2018));
    assert_eq!(year_from_filename("TOTAL_2004.xls"), Some(2004));
    assert_eq!(year_from_filename("PROVISIONALES+2025.xlsx"), Some(2025));
    assert_eq!(year_from_filename("no_year_here.xls"), None);
}

#[test]
fn parse_2024_madrid() {
    let mut wb = open_test_file("DEFINITIVOS+2024.xlsx");
    let range = get_data_range(&mut wb);
    let pax = extract_airport_pax(&range, &madrid_terms());
    assert_eq!(pax, Some(66197066), "MAD 2024 expected 66,197,066");
}

#[test]
fn parse_2024_barcelona() {
    let mut wb = open_test_file("DEFINITIVOS+2024.xlsx");
    let range = get_data_range(&mut wb);
    let pax = extract_airport_pax(&range, &barcelona_terms());
    assert_eq!(pax, Some(55037892), "BCN 2024 expected 55,037,892");
}

#[test]
fn parse_2024_malaga() {
    let mut wb = open_test_file("DEFINITIVOS+2024.xlsx");
    let range = get_data_range(&mut wb);
    let pax = extract_airport_pax(&range, &malaga_terms());
    assert!(pax.is_some(), "AGP 2024 should have data");
    assert!(pax.unwrap() > 20_000_000, "AGP 2024 should be >20M, got {:?}", pax);
}

#[test]
fn parse_2024_valencia() {
    let mut wb = open_test_file("DEFINITIVOS+2024.xlsx");
    let range = get_data_range(&mut wb);
    let pax = extract_airport_pax(&range, &valencia_terms());
    assert!(pax.is_some(), "VLC 2024 should have data");
    assert!(pax.unwrap() > 5_000_000, "VLC 2024 should be >5M, got {:?}", pax);
}

#[test]
fn parse_2023_madrid() {
    let mut wb = open_test_file("DEFINITIVOS_2023.xlsx");
    let range = get_data_range(&mut wb);
    let pax = extract_airport_pax(&range, &madrid_terms());
    assert_eq!(pax, Some(60221163), "MAD 2023 expected 60,221,163");
}

#[test]
fn parse_2023_barcelona() {
    let mut wb = open_test_file("DEFINITIVOS_2023.xlsx");
    let range = get_data_range(&mut wb);
    let pax = extract_airport_pax(&range, &barcelona_terms());
    assert_eq!(pax, Some(49910900), "BCN 2023 expected 49,910,900");
}

#[test]
fn parse_2019_madrid() {
    let mut wb = open_test_file("00.Definitivo_2019.xls");
    let range = get_data_range(&mut wb);
    let pax = extract_airport_pax(&range, &madrid_terms());
    assert_eq!(pax, Some(61734944), "MAD 2019 expected 61,734,944");
}

#[test]
fn parse_2019_barcelona() {
    let mut wb = open_test_file("00.Definitivo_2019.xls");
    let range = get_data_range(&mut wb);
    let pax = extract_airport_pax(&range, &barcelona_terms());
    assert_eq!(pax, Some(52688455), "BCN 2019 expected 52,688,455");
}

#[test]
fn parse_2018_madrid() {
    let mut wb = open_test_file("0.Anual_Definitivo_2018.xls");
    let range = get_data_range(&mut wb);
    let pax = extract_airport_pax(&range, &madrid_terms());
    assert_eq!(pax, Some(57890057), "MAD 2018 expected 57,890,057");
}

#[test]
fn parse_2014_madrid() {
    let mut wb = open_test_file("Definitivo+2014.xls");
    let range = get_data_range(&mut wb);
    let pax = extract_airport_pax(&range, &madrid_terms());
    assert_eq!(pax, Some(41833686), "MAD 2014 expected 41,833,686");
}

#[test]
fn parse_2014_barcelona() {
    let mut wb = open_test_file("Definitivo+2014.xls");
    let range = get_data_range(&mut wb);
    let pax = extract_airport_pax(&range, &barcelona_terms());
    assert_eq!(pax, Some(37558981), "BCN 2014 expected 37,558,981");
}

#[test]
fn parse_2008_madrid() {
    let mut wb = open_test_file("12.Estadistica_Diciembre_2008.xls");
    let range = get_data_range(&mut wb);
    let pax = extract_airport_pax(&range, &madrid_terms());
    assert_eq!(pax, Some(50846494), "MAD 2008 expected 50,846,494");
}

#[test]
fn parse_2008_barcelona() {
    let mut wb = open_test_file("12.Estadistica_Diciembre_2008.xls");
    let range = get_data_range(&mut wb);
    let pax = extract_airport_pax(&range, &barcelona_terms());
    assert_eq!(pax, Some(30272084), "BCN 2008 expected 30,272,084");
}

#[test]
fn parse_2004_madrid() {
    let mut wb = open_test_file("TOTAL_2004.xls");
    let range = get_data_range(&mut wb);
    let pax = extract_airport_pax(&range, &madrid_terms());
    assert!(pax.is_some(), "MAD 2004 should have data");
    assert!(pax.unwrap() > 30_000_000, "MAD 2004 should be >30M, got {:?}", pax);
}

#[test]
fn parse_2025_provisional_madrid() {
    let mut wb = open_test_file("PROVISIONALES+2025.xlsx");
    let range = get_data_range(&mut wb);
    let pax = extract_airport_pax(&range, &madrid_terms());
    assert!(pax.is_some(), "MAD 2025 provisional should have data");
    assert!(pax.unwrap() > 10_000_000, "MAD 2025 should be >10M, got {:?}", pax);
}
