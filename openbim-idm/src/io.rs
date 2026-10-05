//! Encoding-aware, bounded file and stream I/O.

use super::*;
use std::io::{Read, Write};

/// Character encodings the reader and writer understand without extra dependencies.
///
/// Serialized names are accepted by [`Encoding::from_label`], so a reported
/// encoding can be passed straight back to the writer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Encoding {
    #[serde(rename = "utf-8")]
    Utf8,
    #[serde(rename = "utf-16le")]
    Utf16Le,
    #[serde(rename = "utf-16be")]
    Utf16Be,
    /// ISO-8859-1 (also used for `US-ASCII`).
    #[serde(rename = "iso-8859-1")]
    Latin1,
    /// Windows-1252.
    #[serde(rename = "windows-1252")]
    Windows1252,
}

impl Encoding {
    /// Canonical XML declaration name.
    #[must_use]
    pub fn xml_name(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Utf16Le | Self::Utf16Be => "UTF-16",
            Self::Latin1 => "ISO-8859-1",
            Self::Windows1252 => "windows-1252",
        }
    }

    /// Parse a user- or declaration-supplied encoding label.
    pub fn from_label(label: &str) -> Result<Self> {
        match label.trim().to_ascii_lowercase().as_str() {
            "utf-8" | "utf8" => Ok(Self::Utf8),
            "utf-16le" => Ok(Self::Utf16Le),
            "utf-16be" | "utf-16" => Ok(Self::Utf16Be),
            "iso-8859-1" | "latin1" | "latin-1" | "us-ascii" | "ascii" => Ok(Self::Latin1),
            "windows-1252" | "cp1252" => Ok(Self::Windows1252),
            other => Err(Error::Encoding(format!("unsupported encoding `{other}`"))),
        }
    }
}

/// A parsed document together with what was learned about its byte encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedBytes {
    pub document: Document,
    /// Encoding the bytes were decoded with (detected from BOM/declaration).
    pub encoding: Encoding,
    /// Label from the XML declaration, if any.
    pub declared_encoding: Option<String>,
    pub had_bom: bool,
    /// Human-readable notes about information that serialization will change.
    pub warnings: Vec<String>,
}

impl Document {
    /// Parse bytes in any supported encoding with the default size limit.
    pub fn parse_bytes(bytes: &[u8]) -> Result<ParsedBytes> {
        Self::parse_bytes_with_limit(bytes, DEFAULT_MAX_XML_BYTES)
    }

    pub fn parse_bytes_with_limit(bytes: &[u8], max_xml_bytes: usize) -> Result<ParsedBytes> {
        if bytes.len() > max_xml_bytes {
            return Err(Error::InputTooLarge {
                actual: bytes.len(),
                maximum: max_xml_bytes,
            });
        }
        let (encoding, had_bom, body) = detect(bytes);
        let text = decode(encoding, body)?;
        let declared = declared_encoding(&text);
        let mut warnings = Vec::new();
        if let Some(label) = &declared {
            match Encoding::from_label(label) {
                Ok(declared_encoding) if declared_encoding != encoding && !had_bom => {
                    warnings.push(format!(
                        "declaration says {label} but the bytes were decoded as {}",
                        encoding.xml_name()
                    ));
                }
                Err(error) => return Err(error),
                _ => {}
            }
        }
        if had_bom {
            warnings.push("byte order mark will not be preserved on write".into());
        }
        if encoding != Encoding::Utf8 || declared.as_deref().is_some_and(|l| !is_utf8_label(l)) {
            warnings.push(format!(
                "input is {}; output is UTF-8 unless another encoding is chosen",
                declared.as_deref().unwrap_or(encoding.xml_name())
            ));
        }
        let document = Self::parse_with_limit(&text, max_xml_bytes)?;
        Ok(ParsedBytes {
            document,
            encoding,
            declared_encoding: declared,
            had_bom,
            warnings,
        })
    }

    /// Serialize to UTF-8 bytes (no BOM).
    pub fn to_bytes(&self, pretty: bool) -> Result<Vec<u8>> {
        self.to_bytes_with_encoding(pretty, Encoding::Utf8)
    }

    /// Serialize in `encoding`, rewriting the XML declaration to match.
    ///
    /// Characters that cannot be represented in the target encoding are an
    /// [`Error::Encoding`]; nothing is silently replaced.
    pub fn to_bytes_with_encoding(&self, pretty: bool, encoding: Encoding) -> Result<Vec<u8>> {
        let xml = self.to_xml(pretty)?;
        if encoding == Encoding::Utf8 {
            return Ok(xml.into_bytes());
        }
        let xml = xml.replacen(
            "encoding=\"UTF-8\"",
            &format!("encoding=\"{}\"", encoding.xml_name()),
            1,
        );
        encode(encoding, &xml)
    }

    /// Read from a file, honoring the size limit before and during the read.
    pub fn from_path(path: impl AsRef<Path>) -> Result<ParsedBytes> {
        let path = path.as_ref();
        let file = fs::File::open(path)
            .map_err(|error| Error::Io(format!("could not open `{}`: {error}", path.display())))?;
        Self::from_reader(file)
    }

    /// Read from any stream, never buffering more than the size limit plus one byte.
    pub fn from_reader(reader: impl Read) -> Result<ParsedBytes> {
        Self::from_reader_with_limit(reader, DEFAULT_MAX_XML_BYTES)
    }

    pub fn from_reader_with_limit(reader: impl Read, max_xml_bytes: usize) -> Result<ParsedBytes> {
        let mut bytes = Vec::new();
        reader
            .take(max_xml_bytes as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| Error::Io(error.to_string()))?;
        Self::parse_bytes_with_limit(&bytes, max_xml_bytes)
    }

    /// Serialize into a stream as UTF-8.
    pub fn write_to(&self, mut writer: impl Write, pretty: bool) -> Result<()> {
        let bytes = self.to_bytes(pretty)?;
        writer
            .write_all(&bytes)
            .and_then(|()| writer.flush())
            .map_err(|error| Error::Io(error.to_string()))
    }

    /// Write a file atomically: the content goes to a temporary file in the same
    /// directory and replaces the target by rename, so a failure leaves the
    /// original untouched.
    pub fn write_path(&self, path: impl AsRef<Path>, pretty: bool) -> Result<()> {
        self.write_path_with_encoding(path, pretty, Encoding::Utf8)
    }

    pub fn write_path_with_encoding(
        &self,
        path: impl AsRef<Path>,
        pretty: bool,
        encoding: Encoding,
    ) -> Result<()> {
        let path = path.as_ref();
        let bytes = self.to_bytes_with_encoding(pretty, encoding)?;
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .ok_or_else(|| Error::Io(format!("`{}` has no file name", path.display())))?;
        let temporary = parent.join(format!(".{name}.{}.tmp", Uuid::new_v4().simple()));
        let result = fs::write(&temporary, &bytes).and_then(|()| fs::rename(&temporary, path));
        if let Err(error) = result {
            let _ = fs::remove_file(&temporary);
            return Err(Error::Io(format!(
                "could not write `{}`: {error}",
                path.display()
            )));
        }
        Ok(())
    }
}

fn is_utf8_label(label: &str) -> bool {
    matches!(label.to_ascii_lowercase().as_str(), "utf-8" | "utf8")
}

fn detect(bytes: &[u8]) -> (Encoding, bool, &[u8]) {
    if let Some(body) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return (Encoding::Utf8, true, body);
    }
    if let Some(body) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return (Encoding::Utf16Le, true, body);
    }
    if let Some(body) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return (Encoding::Utf16Be, true, body);
    }
    // Declaration-less UTF-16 starts with `<` in the matching byte order.
    match bytes {
        [b'<', 0, ..] => (Encoding::Utf16Le, false, bytes),
        [0, b'<', ..] => (Encoding::Utf16Be, false, bytes),
        _ => (declared_single_byte(bytes), false, bytes),
    }
}

/// Without a BOM, a single-byte declaration decides; otherwise UTF-8.
fn declared_single_byte(bytes: &[u8]) -> Encoding {
    let head = &bytes[..bytes.len().min(256)];
    let text: String = head.iter().map(|&b| char::from(b)).collect();
    declared_encoding(&text)
        .and_then(|label| Encoding::from_label(&label).ok())
        .filter(|encoding| matches!(encoding, Encoding::Latin1 | Encoding::Windows1252))
        .unwrap_or(Encoding::Utf8)
}

fn declared_encoding(text: &str) -> Option<String> {
    let declaration = text.trim_start_matches('\u{feff}').strip_prefix("<?xml")?;
    let declaration = &declaration[..declaration.find("?>")?];
    let rest = &declaration[declaration.find("encoding")? + "encoding".len()..];
    let rest = rest.trim_start().strip_prefix('=')?.trim_start();
    let quote = rest.chars().next().filter(|c| matches!(c, '"' | '\''))?;
    let rest = &rest[1..];
    Some(rest[..rest.find(quote)?].to_owned())
}

fn decode(encoding: Encoding, bytes: &[u8]) -> Result<String> {
    match encoding {
        Encoding::Utf8 => String::from_utf8(bytes.to_vec())
            .map_err(|error| Error::Encoding(format!("input is not valid UTF-8: {error}"))),
        Encoding::Utf16Le | Encoding::Utf16Be => {
            if bytes.len() % 2 != 0 {
                return Err(Error::Encoding("UTF-16 input has an odd byte count".into()));
            }
            let units = bytes.chunks_exact(2).map(|pair| {
                if encoding == Encoding::Utf16Le {
                    u16::from_le_bytes([pair[0], pair[1]])
                } else {
                    u16::from_be_bytes([pair[0], pair[1]])
                }
            });
            char::decode_utf16(units)
                .collect::<std::result::Result<String, _>>()
                .map_err(|error| Error::Encoding(format!("invalid UTF-16: {error}")))
        }
        Encoding::Latin1 => Ok(bytes.iter().map(|&b| char::from(b)).collect()),
        Encoding::Windows1252 => Ok(bytes.iter().map(|&b| windows_1252(b)).collect()),
    }
}

fn encode(encoding: Encoding, xml: &str) -> Result<Vec<u8>> {
    match encoding {
        Encoding::Utf8 => Ok(xml.as_bytes().to_vec()),
        Encoding::Utf16Le | Encoding::Utf16Be => {
            let mut bytes = if encoding == Encoding::Utf16Le {
                vec![0xFF, 0xFE]
            } else {
                vec![0xFE, 0xFF]
            };
            for unit in xml.encode_utf16() {
                bytes.extend_from_slice(&if encoding == Encoding::Utf16Le {
                    unit.to_le_bytes()
                } else {
                    unit.to_be_bytes()
                });
            }
            Ok(bytes)
        }
        Encoding::Latin1 => xml
            .chars()
            .map(|c| {
                u8::try_from(u32::from(c)).map_err(|_| {
                    Error::Encoding(format!(
                        "U+{:04X} cannot be written as ISO-8859-1",
                        u32::from(c)
                    ))
                })
            })
            .collect(),
        Encoding::Windows1252 => xml
            .chars()
            .map(|c| {
                (0..=255u8).find(|&b| windows_1252(b) == c).ok_or_else(|| {
                    Error::Encoding(format!(
                        "U+{:04X} cannot be written as windows-1252",
                        u32::from(c)
                    ))
                })
            })
            .collect(),
    }
}

fn windows_1252(byte: u8) -> char {
    const HIGH: [char; 32] = [
        '\u{20AC}', '\u{81}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}',
        '\u{2021}', '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{8D}',
        '\u{017D}', '\u{8F}', '\u{90}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}',
        '\u{2013}', '\u{2014}', '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}',
        '\u{9D}', '\u{017E}', '\u{0178}',
    ];
    match byte {
        0x80..=0x9F => HIGH[usize::from(byte - 0x80)],
        _ => char::from(byte),
    }
}
