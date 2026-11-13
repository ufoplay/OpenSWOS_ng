//! inih's rules, each on a file built for it.

use super::*;

/// The settings read from `text`, as `section.name=value` strings.
fn values(text: &[u8]) -> Vec<String> {
    read(text)
        .values
        .iter()
        .map(|value| {
            format!(
                "{}.{}={}",
                String::from_utf8_lossy(&value.section),
                String::from_utf8_lossy(&value.name),
                String::from_utf8_lossy(&value.value)
            )
        })
        .collect()
}

#[test]
fn sections_names_and_values_are_trimmed() {
    let text = b"top=1\n[video] \nrenderer = accel \r\nvsync:true\n\t\n[audio]\nmode=\n";
    assert_eq!(
        values(text),
        [
            ".top=1",
            "video.renderer=accel",
            "video.vsync=true",
            "audio.mode="
        ]
    );
    assert_eq!(read(text).first_error, None);
    // Faithful to the original: indented after a pair, a section is a continuation of it.
    assert_eq!(
        values(b"top=1\n  [video]\nk=v\n"),
        [".top=1", ".top=[video]", ".k=v"]
    );
    assert_eq!(values(b"  [video]\nk=v\n"), ["video.k=v"]);
}

#[test]
fn comments_start_a_line_or_follow_whitespace() {
    let text = b"; a comment\n# another\n[video]\nscale=3 ; three\ncrt=a;b\nkey= ;c\nx=1\t;y\n";
    assert_eq!(
        values(text),
        ["video.scale=3", "video.crt=a;b", "video.key=", "video.x=1"]
    );
}

#[test]
fn an_indented_line_continues_the_last_name() {
    let text = b"[chat]\nquick1=hello\n  world ; not this\n\n; a comment\n\tagain\n[x]\n  lost\n";
    assert_eq!(
        values(text),
        [
            "chat.quick1=hello",
            "chat.quick1=world",
            "chat.quick1=again"
        ]
    );
    // After a section there is no name to continue: the indented line is a pair or an
    // error.
    assert_eq!(read(text).first_error, Some(8));
}

#[test]
fn lines_it_cannot_read_are_errors_and_reading_goes_on() {
    let text = b"[video\nno separator\n[a ;b]\n[ok]\nk=v\n";
    let read = read(text);
    assert_eq!(read.first_error, Some(1));
    assert_eq!(values(text), ["ok.k=v"]);
    // A section without `]` keeps the one before.
    assert_eq!(values(b"[a]\n[b\nk=v\n"), ["a.k=v"]);
}

#[test]
fn a_byte_order_mark_is_skipped_on_the_first_line_only() {
    assert_eq!(values(b"\xEF\xBB\xBF[a]\nk=v\n"), ["a.k=v"]);
    assert_eq!(
        read(b"[a]\n\xEF\xBB\xBFk=v\n")
            .values
            .first()
            .map(|v| v.name.clone()),
        Some(b"\xEF\xBB\xBFk".to_vec())
    );
}

#[test]
fn a_long_line_loses_what_its_buffer_cannot_hold() {
    let long = [b"k=".as_slice(), &[b'x'; INI_MAX_LINE * 2], b"\nnext=1\n"].concat();
    let read = read(&long);
    assert_eq!(read.first_error, Some(1));
    let [first, next] = read.values.as_slice() else {
        panic!("two values");
    };
    assert_eq!(first.value.len(), INI_MAX_LINE - 1 - b"k=".len());
    assert_eq!(next.name, b"next");
    // 198 bytes and the newline fit.
    let fits = [b"k=".as_slice(), &[b'x'; INI_MAX_LINE - 4], b"\n"].concat();
    assert_eq!(super::read(&fits).first_error, None);
}

#[test]
fn a_nul_ends_what_a_read_holds() {
    assert_eq!(values(b"[a]\nk=v\0junk\nm=w\n"), ["a.k=v", "a.m=w"]);
}

#[test]
fn long_sections_and_continued_names_are_cut_to_their_buffers() {
    let section = [b'S'; MAX_SECTION + 5];
    let name = [b'n'; MAX_NAME + 5];
    let text = [b"[".as_slice(), &section, b"]\n", &name, b"=1\n  2\n"].concat();
    let read = read(&text);
    let [first, continued] = read.values.as_slice() else {
        panic!("two values");
    };
    assert_eq!(first.section.len(), MAX_SECTION - 1);
    assert_eq!(first.name.len(), name.len());
    assert_eq!(continued.name.len(), MAX_NAME - 1);
}

#[test]
fn a_value_is_found_by_section_and_name_ignoring_case() {
    let read = read(b"[Video]\nVSync=true\n");
    assert!(
        read.values
            .first()
            .is_some_and(|value| value.is("video", "vsync"))
    );
}
