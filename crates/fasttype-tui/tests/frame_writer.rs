use fasttype_tui::terminal::FrameWriter;
use std::io::Write;

#[test]
fn clones_share_one_buffer_and_present_once() {
    let frame = FrameWriter::new();
    let mut backend_side = frame.clone();
    backend_side.write_all(b"abc").unwrap();
    backend_side.flush().unwrap();
    backend_side.write_all(b"def").unwrap();
    assert_eq!(frame.pending(), b"abcdef");
    let mut out = Vec::new();
    frame.present(&mut out).unwrap();
    assert_eq!(out, b"abcdef");
    assert!(frame.pending().is_empty());
}
