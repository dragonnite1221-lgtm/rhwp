//! 바이너리 데이터 쓰기 유틸리티
//!
//! HWP 레코드 내부의 바이너리 필드를 순차적으로 쓰기 위한 버퍼 기반 라이터.
//! `ByteReader`의 역방향으로, 리틀 엔디안과 UTF-16LE 문자열을 지원한다.

/// 바이트 라이터 (버퍼 기반)
pub struct ByteWriter {
    buf: Vec<u8>,
}

impl ByteWriter {
    /// 새 ByteWriter 생성
    pub fn new() -> Self {
        ByteWriter { buf: Vec::new() }
    }

    /// 현재 쓰기 위치 (바이트 수)
    pub fn position(&self) -> usize {
        self.buf.len()
    }

    /// u8 쓰기
    pub fn write_u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    /// u16 쓰기 (LE)
    pub fn write_u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    /// u32 쓰기 (LE)
    pub fn write_u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    /// i8 쓰기
    pub fn write_i8(&mut self, v: i8) {
        self.buf.push(v as u8);
    }

    /// i16 쓰기 (LE)
    pub fn write_i16(&mut self, v: i16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    /// i32 쓰기 (LE)
    pub fn write_i32(&mut self, v: i32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    /// f64 쓰기 (LE)
    pub fn write_f64(&mut self, v: f64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    /// 바이트 슬라이스 쓰기
    pub fn write_bytes(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
    }

    /// HWP 문자열 쓰기 (u16 글자수 + UTF-16LE 바이트)
    ///
    /// `ByteReader::read_hwp_string()`의 역방향.
    /// 형식: [u16 글자수] + [UTF-16LE 바이트 * 글자수]
    pub fn write_hwp_string(&mut self, s: &str) {
        let utf16: Vec<u16> = s.encode_utf16().collect();
        self.write_u16(utf16.len() as u16);
        for code_unit in &utf16 {
            self.write_u16(*code_unit);
        }
    }

    /// ColorRef 쓰기 (4바이트, 0x00BBGGRR 형식)
    pub fn write_color_ref(&mut self, color: u32) {
        self.write_u32(color)
    }

    /// 0으로 채운 패딩 쓰기
    pub fn write_zeros(&mut self, count: usize) {
        self.buf.extend(std::iter::repeat(0u8).take(count));
    }

    /// 내부 버퍼를 소유권 이전하여 반환
    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    /// 내부 버퍼 참조 반환
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::byte_reader::ByteReader;

    #[test]
    fn test_write_u8() {
        let mut w = ByteWriter::new();
        w.write_u8(0x42);
        assert_eq!(w.into_bytes(), [0x42]);
    }

    #[test]
    fn test_write_u16_le() {
        let mut w = ByteWriter::new();
        w.write_u16(0x1234);
        assert_eq!(w.into_bytes(), [0x34, 0x12]);
    }

    #[test]
    fn test_write_u32_le() {
        let mut w = ByteWriter::new();
        w.write_u32(0x12345678);
        assert_eq!(w.into_bytes(), [0x78, 0x56, 0x34, 0x12]);
    }

    #[test]
    fn test_write_i8() {
        let mut w = ByteWriter::new();
        w.write_i8(-1);
        assert_eq!(w.into_bytes(), [0xFF]);
    }

    #[test]
    fn test_write_i16_negative() {
        let mut w = ByteWriter::new();
        w.write_i16(-100);
        let bytes = w.into_bytes();
        let mut reader = ByteReader::new(&bytes);
        assert_eq!(reader.read_i16().unwrap(), -100);
    }

    #[test]
    fn test_write_i32_negative() {
        let mut w = ByteWriter::new();
        w.write_i32(-7200);
        let bytes = w.into_bytes();
        let mut reader = ByteReader::new(&bytes);
        assert_eq!(reader.read_i32().unwrap(), -7200);
    }

    #[test]
    fn test_write_bytes() {
        let mut w = ByteWriter::new();
        w.write_bytes(&[0x01, 0x02, 0x03]);
        assert_eq!(w.into_bytes(), [0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_write_hwp_string_korean() {
        // "한글" → u16 글자수(2) + UTF-16LE
        let mut w = ByteWriter::new();
        w.write_hwp_string("한글");
        let bytes = w.into_bytes();

        let mut reader = ByteReader::new(&bytes);
        assert_eq!(reader.read_hwp_string().unwrap(), "한글");
    }

    #[test]
    fn test_write_hwp_string_ascii() {
        let mut w = ByteWriter::new();
        w.write_hwp_string("ABC");
        let bytes = w.into_bytes();

        let mut reader = ByteReader::new(&bytes);
        assert_eq!(reader.read_hwp_string().unwrap(), "ABC");
    }

    #[test]
    fn test_write_hwp_string_empty() {
        let mut w = ByteWriter::new();
        w.write_hwp_string("");
        let bytes = w.into_bytes();

        let mut reader = ByteReader::new(&bytes);
        assert_eq!(reader.read_hwp_string().unwrap(), "");
    }

    #[test]
    fn test_write_hwp_string_mixed() {
        // 한글 + ASCII 혼합
        let mut w = ByteWriter::new();
        w.write_hwp_string("Hello 세계!");
        let bytes = w.into_bytes();

        let mut reader = ByteReader::new(&bytes);
        assert_eq!(reader.read_hwp_string().unwrap(), "Hello 세계!");
    }

    #[test]
    fn test_write_color_ref() {
        let mut w = ByteWriter::new();
        w.write_color_ref(0x00FF8040);
        let bytes = w.into_bytes();

        let mut reader = ByteReader::new(&bytes);
        assert_eq!(reader.read_color_ref().unwrap(), 0x00FF8040);
    }

    #[test]
    fn test_write_zeros() {
        let mut w = ByteWriter::new();
        w.write_zeros(5);
        assert_eq!(w.into_bytes(), [0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_position() {
        let mut w = ByteWriter::new();
        assert_eq!(w.position(), 0);
        w.write_u16(0x1234);
        assert_eq!(w.position(), 2);
        w.write_u32(0);
        assert_eq!(w.position(), 6);
    }

    #[test]
    fn test_sequential_writes_roundtrip() {
        let mut w = ByteWriter::new();
        w.write_u8(42);
        w.write_u16(1000);
        w.write_i32(-500);
        let bytes = w.into_bytes();

        let mut reader = ByteReader::new(&bytes);
        assert_eq!(reader.read_u8().unwrap(), 42);
        assert_eq!(reader.read_u16().unwrap(), 1000);
        assert_eq!(reader.read_i32().unwrap(), -500);
        assert!(reader.is_empty());
    }
}
