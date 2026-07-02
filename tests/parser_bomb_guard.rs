//! 신뢰할 수 없는 HWP 입력에 대한 자원 상한(압축 폭탄·선할당 폭탄·정수
//! 오버플로) 방어 회귀 테스트. 코드리뷰 H1~H4 대응.
//!
//! 손상/악성 입력이 panic이나 대용량 선할당(OOM)을 일으키지 않고 `Err`로
//! 조용히 종료되는지 확인한다. 실제 수 GB 폭탄을 만들지 않고, "거대한 길이를
//! 선언하지만 데이터는 짧은" 케이스로 선할당 경로를 자극한다.

use rhwp::parser::cfb_reader::{decompress_stream, MAX_DECOMPRESSED_SIZE};
use rhwp::parser::record::Record;

#[test]
fn decompress_roundtrip_still_works() {
    // 정상 동작 보존: 압축 → 해제 라운드트립.
    use flate2::write::DeflateEncoder;
    use flate2::Compression;
    use std::io::Write;
    let original = b"hello codegraph review \x00\x01\x02 body text".repeat(50);
    let mut enc = DeflateEncoder::new(Vec::new(), Compression::default());
    enc.write_all(&original).unwrap();
    let compressed = enc.finish().unwrap();
    let out = decompress_stream(&compressed).expect("정상 스트림은 해제되어야 함");
    assert_eq!(out, original);
}

#[test]
#[allow(clippy::assertions_on_constants)]
fn decompress_cap_is_bounded() {
    // 상한이 비현실적으로 크지 않게(폭탄 방어가 실제로 작동하도록) 고정.
    assert!(MAX_DECOMPRESSED_SIZE <= 512 * 1024 * 1024);
}

#[test]
fn decompress_rejects_garbage_without_panic() {
    // 유효하지 않은 deflate/zlib 입력은 panic 없이 Err.
    let err = decompress_stream(&[0xFF, 0xFF, 0xFF, 0x00, 0x13]);
    assert!(err.is_err());
}

#[test]
fn record_oversized_size_field_errors_not_allocates() {
    // H4: 레코드 헤더가 거대한 크기를 선언(확장 크기 0xFFF → 다음 u32 =
    // 0xFFFF_FFF0)하지만 실제 데이터는 몇 바이트뿐. checked_add 경계 검사가
    // 없으면 wasm32에서 오버플로 후 ~4GB 선할당을 시도한다. 여기서는 panic/OOM
    // 없이 Err를 돌려줘야 한다.
    let mut data = Vec::new();
    // header: tag_id=0x10, level=0, size=0xFFF (확장 크기 신호)
    let header: u32 = 0x10 | (0xFFF << 20);
    data.extend_from_slice(&header.to_le_bytes());
    // 확장 크기: 거대한 값
    data.extend_from_slice(&0xFFFF_FFF0u32.to_le_bytes());
    // 뒤따르는 실제 데이터는 4바이트뿐
    data.extend_from_slice(&[1, 2, 3, 4]);

    let result = Record::read_all(&data);
    assert!(result.is_err(), "거대한 size 선언은 Err여야 함 (선할당/패닉 금지)");
}

#[test]
fn record_normal_roundtrip_still_parses() {
    // 정상 레코드는 그대로 파싱되어야 함(동작 보존).
    let mut data = Vec::new();
    let payload = [0xAAu8, 0xBB, 0xCC];
    let header: u32 = 0x20 | ((payload.len() as u32) << 20);
    data.extend_from_slice(&header.to_le_bytes());
    data.extend_from_slice(&payload);

    let records = Record::read_all(&data).expect("정상 레코드는 파싱되어야 함");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].data, payload);
}
