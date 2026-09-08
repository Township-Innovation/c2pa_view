//! Integration tests that validate a C2PA evidence corpus through
//! `c2pa_view`'s `get_manifest_with_validation` API.
//!
//! Set `EVIDENCE_DIR` to the corpus root. The suite covers:
//!
//! 1. **Generator outputs** — `generator-<platform>/samples/`: signed files must
//!    have `active_manifest`. When `C2PA_EXPECTED_GENERATOR_NAME` is set, the
//!    active manifest's `claim_generator_info[0].name` must match.
//! 2. **Conformance corpus** — `validator/conformance-samples/`: C2PA program
//!    sample media. Manifests may show `signingCredential.untrusted` and/or
//!    `signingCredential.expired` when no trust list is applied.
//! 3. **Additional samples** — `validator/additional-samples/`: extra media not
//!    in the published conformance corpus. Validated through the same path as (2).
//!
//! When `C2PA_TRUST_ANCHORS_PEM` points to a PEM bundle file, the tests run
//! through `get_manifest_with_trust_validation` instead.
//!
//! Logs are written to `validator_utility/c2pa_view_*.json` under the evidence
//! tree unless `C2PA_VIEW_LOGS_DIR` overrides the output directory.

use c2pa_view::api::c2pa::{
    get_manifest_with_trust_validation, get_manifest_with_validation,
};
use pollster::FutureExt as _;
use std::fs;
use std::path::{Path, PathBuf};

fn evidence_dir() -> Option<PathBuf> {
    std::env::var("EVIDENCE_DIR")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

fn logs_dir(evidence: &Path) -> PathBuf {
    if let Ok(dir) = std::env::var("C2PA_VIEW_LOGS_DIR") {
        PathBuf::from(dir)
    } else {
        evidence.join("validator_utility")
    }
}

/// Read the trust anchor PEM bundle from `C2PA_TRUST_ANCHORS_PEM` when set.
fn trust_anchors_pem() -> Option<String> {
    let path = std::env::var("C2PA_TRUST_ANCHORS_PEM").ok()?;
    let path = PathBuf::from(path);
    if !path.is_file() {
        eprintln!(
            "  C2PA_TRUST_ANCHORS_PEM points to {} which is not a file; \
             falling back to default trust evaluation",
            path.display()
        );
        return None;
    }
    fs::read_to_string(&path).ok().filter(|s| !s.trim().is_empty())
}

fn expected_generator_name() -> Option<String> {
    std::env::var("C2PA_EXPECTED_GENERATOR_NAME")
        .ok()
        .filter(|s| !s.is_empty())
}

const MEDIA_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "heic", "dng", "png", "mp4", "m4a", "m4v", "mov",
];

fn mime_for_extension(ext: &str) -> &str {
    match ext.to_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "heic" | "heif" => "image/heif",
        "png" => "image/png",
        "dng" => "image/x-adobe-dng",
        "mp4" | "m4v" => "video/mp4",
        "m4a" => "audio/mp4",
        "mov" => "video/quicktime",
        other => panic!("unknown extension: {other}"),
    }
}

fn is_media_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| MEDIA_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn stem_ends_with_unsigned(path: &Path) -> bool {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.ends_with("_unsigned"))
        .unwrap_or(false)
}

/// Collect signed media files from a `samples/` directory.
fn collect_signed_assets(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if !dir.exists() {
        return files;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && is_media_file(&path) && !stem_ends_with_unsigned(&path) {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// All `generator-<platform>/samples/` directories under [evidence].
fn collect_generator_signed_samples(evidence: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(evidence) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if !name.starts_with("generator-") {
            continue;
        }
        let samples = path.join("samples");
        files.extend(collect_signed_assets(&samples));
    }
    files.sort();
    files
}

/// All media under `conformance-samples/` (non-recursive; corpus is a flat directory).
fn collect_conformance_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if !dir.exists() {
        return files;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && is_media_file(&path) {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

struct ValidationResult {
    file_name: String,
    json: serde_json::Value,
    validation_statuses: Vec<String>,
}

fn validate_file(path: &Path, trust_pem: Option<&str>) -> ValidationResult {
    let file_name = path.file_name().unwrap().to_string_lossy().to_string();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp4");
    let mime = mime_for_extension(ext);
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));

    println!("  Validating: {file_name} ({mime}, {} bytes)", bytes.len());

    let result = if let Some(pem) = trust_pem {
        get_manifest_with_trust_validation(bytes, mime.to_string(), pem.to_string()).block_on()
    } else {
        get_manifest_with_validation(bytes, mime.to_string()).block_on()
    };
    let json_str = result
        .unwrap_or_else(|e| panic!("  API error for {file_name}: {e}"))
        .unwrap_or_else(|| panic!("  No manifest found in {file_name}"));

    let json: serde_json::Value =
        serde_json::from_str(&json_str).unwrap_or_else(|e| panic!("  Bad JSON for {file_name}: {e}"));

    let statuses: Vec<String> = json
        .get("validation_status")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s.get("code").and_then(|c| c.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default();

    ValidationResult {
        file_name,
        json,
        validation_statuses: statuses,
    }
}

fn save_log(logs_dir: &Path, id: &str, json: &serde_json::Value) {
    let _ = fs::create_dir_all(logs_dir);
    let path = logs_dir.join(format!("c2pa_view_{id}.json"));
    let pretty = serde_json::to_string_pretty(json).unwrap_or_default();
    fs::write(&path, pretty).unwrap_or_else(|e| {
        eprintln!("  Warning: cannot write log {}: {e}", path.display());
    });
}

// Test signing CAs that are not on the public C2PA trust list surface as
// `untrusted` regardless of whether a trust list is configured.
const ALLOWED_UNTRUSTED_STATUSES: &[&str] = &[
    "signingCredential.untrusted",
    "signingCredential.trusted",
];

#[test]
fn validate_signed_files() {
    let Some(edir) = evidence_dir() else {
        println!("Skipping validate_signed_files: EVIDENCE_DIR not set");
        return;
    };
    let logs_dir = logs_dir(&edir);
    let trust_pem = trust_anchors_pem();
    let expected_generator = expected_generator_name();

    println!("\n=== c2pa_view: generator-*/samples/ ===\n");
    println!(
        "  Trust list: {}",
        if trust_pem.is_some() { "configured" } else { "default (none)" }
    );
    if let Some(name) = &expected_generator {
        println!("  Expected generator name: {name}");
    }

    let files = collect_generator_signed_samples(&edir);
    assert!(
        !files.is_empty(),
        "No signed files under generator-*/samples/ in {}",
        edir.display()
    );

    let mut failures = Vec::new();

    for path in &files {
        let result = validate_file(path, trust_pem.as_deref());
        let id = path.file_stem().unwrap().to_string_lossy().to_string();
        save_log(&logs_dir, &id, &result.json);

        assert!(
            result.json.get("active_manifest").is_some(),
            "Missing active_manifest in {}",
            result.file_name
        );

        if let Some(expected) = &expected_generator {
            let manifests = result.json.get("manifests").and_then(|m| m.as_object());
            let active_id = result
                .json
                .get("active_manifest")
                .and_then(|a| a.as_str())
                .unwrap_or("");
            if let Some(manifest) = manifests.and_then(|ms| ms.get(active_id)) {
                let gen_name = manifest
                    .pointer("/claim_generator_info/0/name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("");
                if gen_name != expected {
                    let msg = format!(
                        "Wrong claim_generator name in {}: expected {expected}, got {gen_name}",
                        result.file_name
                    );
                    eprintln!("  FAIL: {msg}");
                    failures.push(msg);
                }
            }
        }

        let unexpected: Vec<&String> = result
            .validation_statuses
            .iter()
            .filter(|s| !ALLOWED_UNTRUSTED_STATUSES.contains(&s.as_str()))
            .collect();

        if !unexpected.is_empty() {
            let msg = format!("{}: unexpected validation_status: {:?}", result.file_name, unexpected);
            eprintln!("  FAIL: {msg}");
            failures.push(msg);
        } else {
            println!(
                "  OK: {} (statuses: {:?})",
                result.file_name, result.validation_statuses
            );
        }
    }

    assert!(
        failures.is_empty(),
        "Validation failures:\n{}",
        failures.join("\n")
    );
}

#[test]
fn validate_conformance_samples() {
    let Some(edir) = evidence_dir() else {
        println!("Skipping validate_conformance_samples: EVIDENCE_DIR not set");
        return;
    };
    let logs_dir = logs_dir(&edir);
    let validator_dir = edir.join("validator");
    let conf_dir = validator_dir.join("conformance-samples");
    let extra_dir = validator_dir.join("additional-samples");
    let trust_pem = trust_anchors_pem();

    println!(
        "\n=== c2pa_view: validator/conformance-samples/ + validator/additional-samples/ ===\n"
    );
    println!(
        "  Trust list: {}",
        if trust_pem.is_some() { "configured" } else { "default (none)" }
    );

    let mut files: Vec<PathBuf> = Vec::new();
    if conf_dir.is_dir() {
        files.extend(collect_conformance_files(&conf_dir));
    } else {
        println!("  No validator/conformance-samples/ -- skipping that directory");
    }
    if extra_dir.is_dir() {
        files.extend(collect_conformance_files(&extra_dir));
    } else {
        println!("  No validator/additional-samples/ -- skipping that directory");
    }
    files.sort();

    if files.is_empty() {
        println!("  No media files under conformance-samples/ or additional-samples/ -- skipping");
        return;
    }

    for path in &files {
        let result = validate_file(path, trust_pem.as_deref());

        let safe_id = format!(
            "conf_{}",
            path.file_stem()
                .unwrap()
                .to_string_lossy()
                .replace('.', "_")
        );
        save_log(&logs_dir, &safe_id, &result.json);

        assert!(
            result.json.get("active_manifest").is_some(),
            "Missing active_manifest in {}",
            result.file_name
        );

        println!(
            "  OK: {} (statuses: {:?})",
            result.file_name, result.validation_statuses
        );
    }
}
