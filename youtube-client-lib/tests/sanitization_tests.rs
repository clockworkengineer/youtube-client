//! Exhaustive unit tests for security-hardened filename sanitization

use youtube_client_lib::utils::sanitize_filename;

#[test]
fn test_windows_reserved_device_names() {
    // Exact reserved names
    assert_eq!(sanitize_filename("CON"), "CON_");
    assert_eq!(sanitize_filename("prn"), "prn_");
    assert_eq!(sanitize_filename("AUX"), "AUX_");
    assert_eq!(sanitize_filename("NUL"), "NUL_");
    assert_eq!(sanitize_filename("com1"), "com1_");
    assert_eq!(sanitize_filename("lpt9"), "lpt9_");

    // Reserved names with extensions
    assert_eq!(sanitize_filename("CON.mp4"), "CON.mp4_");
    assert_eq!(sanitize_filename("aux.mp3"), "aux.mp3_");
    assert_eq!(sanitize_filename("NUL.tar.gz"), "NUL.tar.gz_");
}

#[test]
fn test_leading_dash_stripping_for_cli_safety() {
    assert_eq!(sanitize_filename("--option"), "option");
    assert_eq!(sanitize_filename("-v"), "v");
    assert_eq!(sanitize_filename("---dangerous_file"), "dangerous_file");
}

#[test]
fn test_empty_and_dot_edge_cases() {
    assert_eq!(sanitize_filename(""), "unnamed");
    assert_eq!(sanitize_filename("..."), "unnamed");
    assert_eq!(sanitize_filename("   "), "unnamed");
    assert_eq!(sanitize_filename("---"), "unnamed");
}

#[test]
fn test_unicode_and_special_characters() {
    let title = "日本語のタイトル [Official Video] 🎵";
    let sanitized = sanitize_filename(title);
    assert_eq!(sanitized, "日本語のタイトル [Official Video] 🎵");

    let with_illegal = "Title / With \\ Colons : And * Stars ? \"quotes\" <angles> |pipes|";
    let clean = sanitize_filename(with_illegal);
    assert!(!clean.contains('/'));
    assert!(!clean.contains('\\'));
    assert!(!clean.contains(':'));
    assert!(!clean.contains('*'));
    assert!(!clean.contains('?'));
    assert!(!clean.contains('"'));
    assert!(!clean.contains('<'));
    assert!(!clean.contains('>'));
    assert!(!clean.contains('|'));
}

#[test]
fn test_length_boundary_truncation() {
    let long_title = "a".repeat(120);
    let sanitized = sanitize_filename(&long_title);
    assert!(sanitized.chars().count() <= 60);
}
