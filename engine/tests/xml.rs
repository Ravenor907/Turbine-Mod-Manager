//! The small XML reader used for installer files.

use turbine_engine::xml::{self, decode, parse, unescape, MAX_DEPTH};

#[test]
fn reads_elements_attributes_text_and_cdata() {
    let r = parse(
        "<?xml version=\"1.0\"?>\n<!-- top -->\n<a x=\"1\" y='two &amp; three'>\n  hello <b/><![CDATA[<raw> & stuff]]><c k=\"v\">inner</c><!-- c --> world\n</a>\n",
    )
    .unwrap();
    assert_eq!(r.name, "a");
    assert_eq!(r.attr("X"), Some("1"));
    assert_eq!(r.attr("y"), Some("two & three"));
    assert_eq!(r.elements().count(), 2);
    assert_eq!(r.child("c").unwrap().text(), "inner");
    let t = r.text();
    assert!(t.starts_with("hello") && t.contains("<raw> & stuff") && t.ends_with("world"), "{t}");
}

#[test]
fn entities_and_character_references() {
    assert_eq!(unescape("&lt;&gt;&amp;&quot;&apos;"), "<>&\"'");
    assert_eq!(unescape("&#65;&#x42;&#X43;"), "ABC");
    // Unknown names and stray ampersands are kept as written (hand-written files have them).
    assert_eq!(unescape("Tom & Jerry &nbsp; &bogus;"), "Tom & Jerry &nbsp; &bogus;");
    assert_eq!(unescape("&#0;"), "&#0;");
}

#[test]
fn doctype_is_skipped_and_never_expanded() {
    let r = parse("<!DOCTYPE x [ <!ENTITY e SYSTEM \"file:///etc/passwd\"> <!ENTITY f \"]>\"> ]><x>&e;&f;</x>").unwrap();
    assert_eq!(r.text(), "&e;&f;");
}

#[test]
fn namespaces_and_case_in_names() {
    let r = parse("<ns:Config xsi:schema=\"s\"><ModuleName>M</ModuleName></ns:Config>").unwrap();
    assert!(r.is("config"));
    assert_eq!(r.attr("schema"), Some("s"));
    assert_eq!(r.child("moduleName").unwrap().text(), "M");
}

#[test]
fn bad_files_are_refused_with_a_line() {
    let e = parse("<a>\n<b></c></a>").unwrap_err();
    assert_eq!(e.line, 2);
    assert!(parse("just text").is_err());
    assert!(parse("<a x=1/>").is_err());
    assert!(parse("<a><!ENTITY x \"y\"></a>").is_err());
    assert!(parse("<a><!-- never closed").is_err());
    // Too deep.
    let deep = "<a>".repeat(MAX_DEPTH + 1);
    assert!(parse(&deep).unwrap_err().what.contains("deep"));
    // Cut off at the end: what's open is closed.
    let r = parse("<a><b>text").unwrap();
    assert_eq!(r.child("b").unwrap().text(), "text");
}

#[test]
fn decodes_utf8_utf16_and_windows_1252() {
    assert_eq!(decode(b"\xEF\xBB\xBF<a/>").unwrap(), "<a/>");
    let mut le = vec![0xFF, 0xFE];
    for u in "<a>ü</a>".encode_utf16() {
        le.extend_from_slice(&u.to_le_bytes());
    }
    assert_eq!(decode(&le).unwrap(), "<a>ü</a>");
    let mut be = vec![0xFE, 0xFF];
    for u in "<a/>".encode_utf16() {
        be.extend_from_slice(&u.to_be_bytes());
    }
    assert_eq!(decode(&be).unwrap(), "<a/>");
    // UTF-16 without a BOM, found from the leading '<'.
    let bare: Vec<u8> = "<a/>".encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    assert_eq!(decode(&bare).unwrap(), "<a/>");
    // Not UTF-8: read as Windows-1252.
    assert_eq!(decode(b"<a>caf\xe9 \x93q\x94</a>").unwrap(), "<a>caf\u{e9} \u{201c}q\u{201d}</a>");
    assert!(decode(&[0xFF, 0xFE, 0x3C]).is_err());
    assert_eq!(xml::parse_bytes(&le).unwrap().text(), "ü");
}
