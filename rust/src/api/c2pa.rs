use c2pa::{Context, Reader, Settings};

fn read_path_bytes(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("failed to read {path}: {e}"))
}

fn mime_type_from_path(path: &str) -> Result<String, String> {
    mime_guess::from_path(path)
        .first()
        .ok_or_else(|| format!("failed to guess MIME type for {path}"))
        .map(|m| m.to_string())
}

pub async fn get_file_manifest(
    file_bytes: Vec<u8>,
    path: String,
) -> Result<Option<String>, String> {
    let mime_type = mime_type_from_path(&path)?;
    get_file_manifest_format(file_bytes, mime_type).await
}

pub async fn get_file_manifest_format(
    file_bytes: Vec<u8>,
    format: String,
) -> Result<Option<String>, String> {
    // L1 `GET /manifests/{hash}` returns detached manifest-store JUMBF bytes.
    // Default Reader settings verify those bytes against ingredient data_hash
    // bindings, which fails with "Hashes do not match" because the body is not
    // the referenced media asset.
    let settings_json = serde_json::json!({
        "verify": {
            "verify_after_reading": false,
        }
    });
    let settings = Settings::new()
        .with_json(&settings_json.to_string())
        .map_err(|e| format!("settings error: {e}"))?;
    let context = Context::new()
        .with_settings(settings)
        .map_err(|e| format!("context error: {e}"))?;

    let manifest_format = normalize_detached_manifest_format(&format);
    let stream = std::io::Cursor::new(file_bytes);

    let reader = Reader::from_context(context)
        .with_stream(manifest_format, stream)
        .ok();

    Ok(reader.map(|r| r.json()))
}

/// UTF-8 JSON bytes for [`get_file_manifest_format`]. See
/// [`get_manifest_with_validation_utf8`] for the web FRB rationale.
pub async fn get_file_manifest_format_utf8(
    file_bytes: Vec<u8>,
    format: String,
) -> Result<Option<Vec<u8>>, String> {
    Ok(get_file_manifest_format(file_bytes, format)
        .await?
        .map(|s| s.into_bytes()))
}

/// Maps HTTP Content-Type values to a manifest-store MIME understood by c2pa-rs.
fn normalize_detached_manifest_format(format: &str) -> &str {
    match format.to_ascii_lowercase().as_str() {
        "application/c2pa" | "application/x-c2pa-manifest-store" | "c2pa" => format,
        _ => "application/c2pa",
    }
}

fn reader_manifest_json_value(reader: Reader) -> Result<serde_json::Value, String> {
    let mut value: serde_json::Value =
        serde_json::from_str(&reader.json()).map_err(|e| e.to_string())?;
    if let Some(statuses) = reader.validation_status() {
        value["validation_status"] =
            serde_json::to_value(statuses).map_err(|e| e.to_string())?;
    }
    Ok(value)
}

pub async fn get_manifest_with_validation(
    file_bytes: Vec<u8>,
    format: String,
) -> Result<Option<String>, String> {
    let stream = std::io::Cursor::new(file_bytes);
    let reader = Reader::from_stream(&format, stream).ok();
    match reader {
        Some(r) => Ok(Some(
            reader_manifest_json_value(r)?.to_string(),
        )),
        None => Ok(None),
    }
}

/// UTF-8 JSON bytes for [`get_manifest_with_validation`].
///
/// Web FRB sync can fail to decode very large [`String`] returns from WASM
/// (Dart `TypeError` on DCO decode). Callers should `utf8.decode` on the VM
/// or web.
pub async fn get_manifest_with_validation_utf8(
    file_bytes: Vec<u8>,
    format: String,
) -> Result<Option<Vec<u8>>, String> {
    let stream = std::io::Cursor::new(file_bytes);
    let reader = Reader::from_stream(&format, stream).ok();
    match reader {
        Some(r) => Ok(Some(
            reader_manifest_json_value(r)?.to_string().into_bytes(),
        )),
        None => Ok(None),
    }
}

pub async fn get_manifest_with_validation_from_path(
    path: String,
) -> Result<Option<String>, String> {
    let file_bytes = read_path_bytes(&path)?;
    let mime_type = mime_type_from_path(&path)?;
    get_manifest_with_validation(file_bytes, mime_type).await
}

/// Validate a C2PA asset against provided trust anchor PEM bundles.
///
/// `trust_anchors_pem` should contain the C2PA Trust List and optionally
/// the TSA Trust List concatenated as a single PEM bundle.
pub async fn get_manifest_with_trust_validation(
    file_bytes: Vec<u8>,
    format: String,
    trust_anchors_pem: String,
) -> Result<Option<String>, String> {
    let settings_json = serde_json::json!({
        "trust": {
            "trust_anchors": trust_anchors_pem,
        },
        "verify": {
            "verify_trust": true,
        }
    });
    let settings = Settings::new()
        .with_json(&settings_json.to_string())
        .map_err(|e| format!("trust settings error: {e}"))?;

    let context = Context::new()
        .with_settings(settings)
        .map_err(|e| format!("context error: {e}"))?;

    let stream = std::io::Cursor::new(file_bytes);
    let reader = Reader::from_context(context)
        .with_stream(&format, stream)
        .ok();

    match reader {
        Some(r) => {
            let mut value: serde_json::Value =
                serde_json::from_str(&r.json()).map_err(|e| e.to_string())?;
            if let Some(statuses) = r.validation_status() {
                value["validation_status"] =
                    serde_json::to_value(statuses).map_err(|e| e.to_string())?;
            }
            Ok(Some(value.to_string()))
        }
        None => Ok(None),
    }
}

/// Convenience wrapper that guesses MIME from file path and reads bytes on the
/// Rust side.
pub async fn get_manifest_with_trust_validation_from_path(
    path: String,
    trust_anchors_pem: String,
) -> Result<Option<String>, String> {
    let file_bytes = read_path_bytes(&path)?;
    let mime_type = mime_type_from_path(&path)?;
    get_manifest_with_trust_validation(file_bytes, mime_type, trust_anchors_pem).await
}
