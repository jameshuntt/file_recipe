use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRecipe {
    pub recipe_id: String,
    pub extension: String,
    pub mime_type: String,
    pub filename_template: String,
    pub storage_class: StorageClass,
    pub read_modes: Vec<ReadMode>,
    pub validation: ValidationRuleSet,
    pub projections: Vec<ProjectionRule>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageClass {
    InlineBytes,
    ContentAddressedBlob,
    JsonDocument,
    JsonLines,
    BinaryObject,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadMode {
    RawBytes,
    Utf8Text,
    Hex,
    Base64,
    Json,
    ParsedFields,
    PublicProjection,
    DigestOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRecipeInput {
    pub source_kind: String,
    pub fields: BTreeMap<String, String>,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializedFileObject {
    pub recipe_id: String,
    pub filename: String,
    pub extension: String,
    pub mime_type: String,
    pub content_digest: String,
    pub content_length: usize,
    pub bytes: Vec<u8>,
    pub readable_views: Vec<ReadableView>,
    pub source_kind: String,
    pub source_fields: BTreeMap<String, String>,
    pub projection_rules: Vec<ProjectionRule>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadableView {
    pub mode: ReadMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationRuleSet {
    pub min_bytes: Option<usize>,
    pub max_bytes: Option<usize>,
    pub required_fields: Vec<String>,
    pub require_utf8: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionRule {
    pub name: String,
    pub include_fields: Vec<String>,
    pub include_digest: bool,
    pub include_length: bool,
    pub include_mime_type: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadResult {
    Bytes(Vec<u8>),
    Text(String),
    Hex(String),
    Base64(String),
    Digest {
        digest: String,
        length: usize,
        mime_type: String,
    },
    PublicProjection(BTreeMap<String, String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileRecipeError {
    EmptyRecipeId,
    EmptyExtension,
    ExtensionMustStartWithDot { extension: String },
    EmptyMimeType,
    EmptyFilenameTemplate,
    MissingRequiredField { field: String },
    EmptyRequiredField { field: String },
    MissingTemplateField { field: String },
    EmptyTemplateField { field: String },
    UnclosedTemplatePlaceholder { template: String },
    EmptyTemplatePlaceholder { template: String },
    UnmatchedTemplateCloseBrace { template: String },
    PayloadTooSmall { min: usize, actual: usize },
    PayloadTooLarge { max: usize, actual: usize },
    InvalidUtf8Payload,
    ReadModeNotAllowed { mode: ReadMode },
    UnsupportedReadMode { mode: ReadMode },
    ProjectionFieldMissing { field: String },
}

impl fmt::Display for FileRecipeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileRecipeError::EmptyRecipeId => write!(f, "file recipe requires a recipe_id"),
            FileRecipeError::EmptyExtension => write!(f, "file recipe requires a non-empty extension"),
            FileRecipeError::ExtensionMustStartWithDot { extension } => {
                write!(f, "file recipe extension must start with '.', got {extension:?}")
            }
            FileRecipeError::EmptyMimeType => write!(f, "file recipe requires a mime_type"),
            FileRecipeError::EmptyFilenameTemplate => {
                write!(f, "file recipe requires a filename_template")
            }
            FileRecipeError::MissingRequiredField { field } => {
                write!(f, "file recipe input is missing required field {field:?}")
            }
            FileRecipeError::EmptyRequiredField { field } => {
                write!(f, "file recipe input field {field:?} cannot be empty")
            }
            FileRecipeError::MissingTemplateField { field } => {
                write!(f, "filename template requires missing field {field:?}")
            }
            FileRecipeError::EmptyTemplateField { field } => {
                write!(f, "filename template field {field:?} cannot render from an empty value")
            }
            FileRecipeError::UnclosedTemplatePlaceholder { template } => {
                write!(f, "filename template has an unclosed placeholder: {template:?}")
            }
            FileRecipeError::EmptyTemplatePlaceholder { template } => {
                write!(f, "filename template has an empty placeholder: {template:?}")
            }
            FileRecipeError::UnmatchedTemplateCloseBrace { template } => {
                write!(f, "filename template has an unmatched closing brace: {template:?}")
            }
            FileRecipeError::PayloadTooSmall { min, actual } => {
                write!(f, "payload is too small: minimum {min} bytes, got {actual}")
            }
            FileRecipeError::PayloadTooLarge { max, actual } => {
                write!(f, "payload is too large: maximum {max} bytes, got {actual}")
            }
            FileRecipeError::InvalidUtf8Payload => {
                write!(f, "payload must be valid UTF-8 for this file recipe")
            }
            FileRecipeError::ReadModeNotAllowed { mode } => {
                write!(f, "read mode {mode:?} is not allowed by this materialized object")
            }
            FileRecipeError::UnsupportedReadMode { mode } => {
                write!(f, "read mode {mode:?} is not implemented by file_recipe yet")
            }
            FileRecipeError::ProjectionFieldMissing { field } => {
                write!(f, "public projection requested missing source field {field:?}")
            }
        }
    }
}

impl Error for FileRecipeError {}

pub fn materialize_file_object(
    recipe: &FileRecipe,
    input: FileRecipeInput,
) -> Result<MaterializedFileObject, FileRecipeError> {
    validate_recipe_and_input(recipe, &input)?;

    let filename = render_filename(&recipe.filename_template, &input.fields, &recipe.extension)?;
    let content_digest = digest_bytes(&input.bytes);
    let content_length = input.bytes.len();
    let readable_views = recipe
        .read_modes
        .iter()
        .cloned()
        .map(|mode| ReadableView { mode })
        .collect();

    Ok(MaterializedFileObject {
        recipe_id: recipe.recipe_id.clone(),
        filename,
        extension: recipe.extension.clone(),
        mime_type: recipe.mime_type.clone(),
        content_digest,
        content_length,
        bytes: input.bytes,
        readable_views,
        source_kind: input.source_kind,
        source_fields: input.fields,
        projection_rules: recipe.projections.clone(),
    })
}

pub fn render_filename(
    template: &str,
    fields: &BTreeMap<String, String>,
    extension: &str,
) -> Result<String, FileRecipeError> {
    validate_extension(extension)?;
    if template.is_empty() {
        return Err(FileRecipeError::EmptyFilenameTemplate);
    }

    let mut rendered = String::new();
    let mut chars = template.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '{' => {
                let mut name = String::new();
                let mut closed = false;
                for next in chars.by_ref() {
                    if next == '}' {
                        closed = true;
                        break;
                    }
                    if next == '{' {
                        return Err(FileRecipeError::UnclosedTemplatePlaceholder {
                            template: template.to_string(),
                        });
                    }
                    name.push(next);
                }
                if !closed {
                    return Err(FileRecipeError::UnclosedTemplatePlaceholder {
                        template: template.to_string(),
                    });
                }
                if name.is_empty() {
                    return Err(FileRecipeError::EmptyTemplatePlaceholder {
                        template: template.to_string(),
                    });
                }
                let value = fields
                    .get(&name)
                    .ok_or_else(|| FileRecipeError::MissingTemplateField { field: name.clone() })?;
                if value.is_empty() {
                    return Err(FileRecipeError::EmptyTemplateField { field: name });
                }
                rendered.push_str(&safe_filename_component(value));
            }
            '}' => {
                return Err(FileRecipeError::UnmatchedTemplateCloseBrace {
                    template: template.to_string(),
                });
            }
            '*' => rendered.push('_'),
            '/' | '\\' | ':' | ';' | ',' | '|' | '?' | '!' | '@' | '#' | '$' | '%' | '^'
            | '&' | '+' | '=' | '~' | '`' | '\'' | '"' | '<' | '>' | '[' | ']' | '(' | ')'
            | ' ' | '\t' | '\n' | '\r' => rendered.push('_'),
            other => rendered.push(other),
        }
    }

    let rendered = collapse_underscores(rendered.trim_matches('_'));
    if rendered.is_empty() {
        return Err(FileRecipeError::EmptyFilenameTemplate);
    }

    if rendered.ends_with(extension) {
        Ok(rendered)
    } else {
        Ok(format!("{rendered}{extension}"))
    }
}

pub fn digest_bytes(bytes: &[u8]) -> String {
    format!("sha256:{}", encode_hex(&sha256(bytes)))
}

pub fn read_view(
    object: &MaterializedFileObject,
    mode: ReadMode,
) -> Result<ReadResult, FileRecipeError> {
    if !object.readable_views.iter().any(|view| view.mode == mode) {
        return Err(FileRecipeError::ReadModeNotAllowed { mode });
    }

    match mode {
        ReadMode::RawBytes => Ok(ReadResult::Bytes(object.bytes.clone())),
        ReadMode::Utf8Text => String::from_utf8(object.bytes.clone())
            .map(ReadResult::Text)
            .map_err(|_| FileRecipeError::InvalidUtf8Payload),
        ReadMode::Hex => Ok(ReadResult::Hex(encode_hex(&object.bytes))),
        ReadMode::Base64 => Ok(ReadResult::Base64(encode_base64(&object.bytes))),
        ReadMode::DigestOnly => Ok(ReadResult::Digest {
            digest: object.content_digest.clone(),
            length: object.content_length,
            mime_type: object.mime_type.clone(),
        }),
        ReadMode::PublicProjection => Ok(ReadResult::PublicProjection(public_projection(object)?)),
        unsupported @ (ReadMode::Json | ReadMode::ParsedFields) => {
            Err(FileRecipeError::UnsupportedReadMode { mode: unsupported })
        }
    }
}

pub fn solana_pda_account_file_recipe() -> FileRecipe {
    FileRecipe {
        recipe_id: "solana.pda.account.v1".to_string(),
        extension: ".pda.bin".to_string(),
        mime_type: "application/x-solana-pda".to_string(),
        filename_template: "{chain}*{program_id}*{pda_address}.pda.bin".to_string(),
        storage_class: StorageClass::ContentAddressedBlob,
        read_modes: vec![
            ReadMode::RawBytes,
            ReadMode::Hex,
            ReadMode::Base64,
            ReadMode::DigestOnly,
            ReadMode::PublicProjection,
        ],
        validation: ValidationRuleSet {
            min_bytes: Some(1),
            max_bytes: Some(10 * 1024 * 1024),
            required_fields: vec![
                "chain".to_string(),
                "program_id".to_string(),
                "pda_address".to_string(),
            ],
            require_utf8: false,
        },
        projections: vec![ProjectionRule {
            name: "public".to_string(),
            include_fields: vec![
                "chain".to_string(),
                "program_id".to_string(),
                "pda_address".to_string(),
            ],
            include_digest: true,
            include_length: true,
            include_mime_type: true,
        }],
    }
}

fn validate_recipe_and_input(
    recipe: &FileRecipe,
    input: &FileRecipeInput,
) -> Result<(), FileRecipeError> {
    if recipe.recipe_id.is_empty() {
        return Err(FileRecipeError::EmptyRecipeId);
    }
    validate_extension(&recipe.extension)?;
    if recipe.mime_type.is_empty() {
        return Err(FileRecipeError::EmptyMimeType);
    }
    if recipe.filename_template.is_empty() {
        return Err(FileRecipeError::EmptyFilenameTemplate);
    }

    for field in &recipe.validation.required_fields {
        match input.fields.get(field) {
            Some(value) if !value.is_empty() => {}
            Some(_) => return Err(FileRecipeError::EmptyRequiredField { field: field.clone() }),
            None => return Err(FileRecipeError::MissingRequiredField { field: field.clone() }),
        }
    }

    let actual = input.bytes.len();
    if let Some(min) = recipe.validation.min_bytes {
        if actual < min {
            return Err(FileRecipeError::PayloadTooSmall { min, actual });
        }
    }
    if let Some(max) = recipe.validation.max_bytes {
        if actual > max {
            return Err(FileRecipeError::PayloadTooLarge { max, actual });
        }
    }
    if recipe.validation.require_utf8 && std::str::from_utf8(&input.bytes).is_err() {
        return Err(FileRecipeError::InvalidUtf8Payload);
    }

    render_filename(&recipe.filename_template, &input.fields, &recipe.extension)?;
    Ok(())
}

fn validate_extension(extension: &str) -> Result<(), FileRecipeError> {
    if extension.is_empty() {
        return Err(FileRecipeError::EmptyExtension);
    }
    if !extension.starts_with('.') {
        return Err(FileRecipeError::ExtensionMustStartWithDot {
            extension: extension.to_string(),
        });
    }
    Ok(())
}

fn public_projection(
    object: &MaterializedFileObject,
) -> Result<BTreeMap<String, String>, FileRecipeError> {
    let rule = object.projection_rules.first();
    let mut projection = BTreeMap::new();

    projection.insert("recipe_id".to_string(), object.recipe_id.clone());
    projection.insert("filename".to_string(), object.filename.clone());
    projection.insert("extension".to_string(), object.extension.clone());

    let include_digest = rule.map(|r| r.include_digest).unwrap_or(true);
    let include_length = rule.map(|r| r.include_length).unwrap_or(true);
    let include_mime_type = rule.map(|r| r.include_mime_type).unwrap_or(true);

    if include_mime_type {
        projection.insert("mime_type".to_string(), object.mime_type.clone());
    }
    if include_digest {
        projection.insert("content_digest".to_string(), object.content_digest.clone());
    }
    if include_length {
        projection.insert("content_length".to_string(), object.content_length.to_string());
    }

    if let Some(rule) = rule {
        for field in &rule.include_fields {
            let value = object
                .source_fields
                .get(field)
                .ok_or_else(|| FileRecipeError::ProjectionFieldMissing { field: field.clone() })?;
            projection.insert(field.clone(), value.clone());
        }
    }

    Ok(projection)
}

fn safe_filename_component(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    collapse_underscores(out.trim_matches('_'))
}

fn collapse_underscores(value: &str) -> String {
    let mut out = String::new();
    let mut last_was_underscore = false;
    for ch in value.chars() {
        if ch == '_' {
            if !last_was_underscore {
                out.push(ch);
            }
            last_was_underscore = true;
        } else {
            out.push(ch);
            last_was_underscore = false;
        }
    }
    out
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn encode_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(((bytes.len() + 2) / 3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);

        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(b2 & 0b0011_1111) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn sha256(input: &[u8]) -> [u8; 32] {
    const H0: [u32; 8] = [
        0x6a09e667,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];
    const K: [u32; 64] = [
        0x428a2f98,
        0x71374491,
        0xb5c0fbcf,
        0xe9b5dba5,
        0x3956c25b,
        0x59f111f1,
        0x923f82a4,
        0xab1c5ed5,
        0xd807aa98,
        0x12835b01,
        0x243185be,
        0x550c7dc3,
        0x72be5d74,
        0x80deb1fe,
        0x9bdc06a7,
        0xc19bf174,
        0xe49b69c1,
        0xefbe4786,
        0x0fc19dc6,
        0x240ca1cc,
        0x2de92c6f,
        0x4a7484aa,
        0x5cb0a9dc,
        0x76f988da,
        0x983e5152,
        0xa831c66d,
        0xb00327c8,
        0xbf597fc7,
        0xc6e00bf3,
        0xd5a79147,
        0x06ca6351,
        0x14292967,
        0x27b70a85,
        0x2e1b2138,
        0x4d2c6dfc,
        0x53380d13,
        0x650a7354,
        0x766a0abb,
        0x81c2c92e,
        0x92722c85,
        0xa2bfe8a1,
        0xa81a664b,
        0xc24b8b70,
        0xc76c51a3,
        0xd192e819,
        0xd6990624,
        0xf40e3585,
        0x106aa070,
        0x19a4c116,
        0x1e376c08,
        0x2748774c,
        0x34b0bcb5,
        0x391c0cb3,
        0x4ed8aa4a,
        0x5b9cca4f,
        0x682e6ff3,
        0x748f82ee,
        0x78a5636f,
        0x84c87814,
        0x8cc70208,
        0x90befffa,
        0xa4506ceb,
        0xbef9a3f7,
        0xc67178f2,
    ];

    let mut message = input.to_vec();
    let bit_len = (message.len() as u64) * 8;
    message.push(0x80);
    while (message.len() % 64) != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());

    let mut h = H0;
    let mut w = [0u32; 64];

    for chunk in message.chunks_exact(64) {
        for (i, word) in chunk.chunks_exact(4).take(16).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 32];
    for (i, value) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&value.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture_input() -> FileRecipeInput {
        FileRecipeInput {
            source_kind: "solana_pda_account".to_string(),
            fields: BTreeMap::from([
                ("chain".to_string(), "solana".to_string()),
                ("program_id".to_string(), "demo_program".to_string()),
                ("pda_address".to_string(), "demo_pda".to_string()),
            ]),
            bytes: vec![1, 2, 3, 4, 5, 6, 7, 8],
        }
    }

    #[test]
    fn materializes_raw_solana_pda_file_with_extension_and_digest() {
        let recipe = solana_pda_account_file_recipe();
        let object = materialize_file_object(&recipe, fixture_input()).unwrap();

        assert_eq!(object.recipe_id, "solana.pda.account.v1");
        assert_eq!(object.filename, "solana_demo_program_demo_pda.pda.bin");
        assert_eq!(object.extension, ".pda.bin");
        assert_eq!(object.mime_type, "application/x-solana-pda");
        assert_eq!(object.content_length, 8);
        assert_eq!(object.content_digest, digest_bytes(&[1, 2, 3, 4, 5, 6, 7, 8]));
        assert_eq!(
            object.content_digest,
            "sha256:66840dda154e8a113c31dd0ad32f7f3a366a80e8136979d8f5a101d3d29d6f72"
        );
        assert!(object.content_digest.starts_with("sha256:"));
        assert_eq!(object.content_digest.len(), "sha256:".len() + 64);
        assert_eq!(object.bytes, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn filename_template_requires_all_fields() {
        let fields = BTreeMap::from([("chain".to_string(), "solana".to_string())]);
        let err = render_filename("{chain}*{program_id}.pda.bin", &fields, ".pda.bin")
            .expect_err("program_id is required by template");

        assert_eq!(
            err,
            FileRecipeError::MissingTemplateField {
                field: "program_id".to_string()
            }
        );
    }

    #[test]
    fn read_raw_bytes_returns_exact_original_bytes() {
        let recipe = solana_pda_account_file_recipe();
        let object = materialize_file_object(&recipe, fixture_input()).unwrap();

        assert_eq!(
            read_view(&object, ReadMode::RawBytes).unwrap(),
            ReadResult::Bytes(vec![1, 2, 3, 4, 5, 6, 7, 8])
        );
    }

    #[test]
    fn read_hex_and_base64_views_are_derived_from_real_bytes() {
        let recipe = solana_pda_account_file_recipe();
        let object = materialize_file_object(&recipe, fixture_input()).unwrap();

        assert_eq!(
            read_view(&object, ReadMode::Hex).unwrap(),
            ReadResult::Hex("0102030405060708".to_string())
        );
        assert_eq!(
            read_view(&object, ReadMode::Base64).unwrap(),
            ReadResult::Base64("AQIDBAUGBwg=".to_string())
        );
    }

    #[test]
    fn utf8_read_fails_for_non_utf8_when_required() {
        let mut recipe = solana_pda_account_file_recipe();
        recipe.validation.require_utf8 = true;
        recipe.read_modes.push(ReadMode::Utf8Text);
        let mut input = fixture_input();
        input.bytes = vec![0xff, 0xfe, 0xfd];

        let err = materialize_file_object(&recipe, input).expect_err("invalid UTF-8 must fail");
        assert_eq!(err, FileRecipeError::InvalidUtf8Payload);
    }

    #[test]
    fn public_projection_excludes_raw_bytes() {
        let recipe = solana_pda_account_file_recipe();
        let object = materialize_file_object(&recipe, fixture_input()).unwrap();

        let projection = match read_view(&object, ReadMode::PublicProjection).unwrap() {
            ReadResult::PublicProjection(projection) => projection,
            other => panic!("expected public projection, got {other:?}"),
        };

        assert_eq!(projection.get("recipe_id"), Some(&"solana.pda.account.v1".to_string()));
        assert_eq!(projection.get("filename"), Some(&"solana_demo_program_demo_pda.pda.bin".to_string()));
        assert_eq!(projection.get("extension"), Some(&".pda.bin".to_string()));
        assert_eq!(projection.get("mime_type"), Some(&"application/x-solana-pda".to_string()));
        assert_eq!(projection.get("content_length"), Some(&"8".to_string()));
        assert_eq!(projection.get("chain"), Some(&"solana".to_string()));
        assert_eq!(projection.get("program_id"), Some(&"demo_program".to_string()));
        assert_eq!(projection.get("pda_address"), Some(&"demo_pda".to_string()));
        assert!(projection.contains_key("content_digest"));
        assert!(!projection.contains_key("bytes"));
        assert!(!projection.contains_key("raw_bytes"));
        assert!(!projection.values().any(|value| value == "[1, 2, 3, 4, 5, 6, 7, 8]"));
    }

    #[test]
    fn validation_rejects_payload_larger_than_max_bytes() {
        let mut recipe = solana_pda_account_file_recipe();
        recipe.validation.max_bytes = Some(4);

        let err = materialize_file_object(&recipe, fixture_input())
            .expect_err("payload larger than max_bytes must fail");

        assert_eq!(err, FileRecipeError::PayloadTooLarge { max: 4, actual: 8 });
    }

    #[test]
    fn materialized_object_can_be_written_and_read_from_temp_file() {
        let recipe = solana_pda_account_file_recipe();
        let object = materialize_file_object(&recipe, fixture_input()).unwrap();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "file_recipe_{}_{}_{}",
            std::process::id(),
            unique,
            object.filename
        ));

        fs::write(&path, &object.bytes).unwrap();
        let read_back = fs::read(&path).unwrap();
        let _ = fs::remove_file(&path);

        assert_eq!(read_back, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn sha256_digest_matches_known_empty_vector() {
        assert_eq!(
            digest_bytes(&[]),
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
