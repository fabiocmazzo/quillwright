//! Small helpers for writing XML-based formats (XHTML, EPUB, ODT) and zip packages.
use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

/// Escape text for XML content and attribute values.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            // Control characters are not allowed in XML 1.0.
            c if (c as u32) < 0x20 && !matches!(c, '\n' | '\t' | '\r') => {}
            c => out.push(c),
        }
    }
    out
}

/// A zip package (EPUB, ODT). The `mimetype` entry must come first and be
/// stored uncompressed, which both formats require.
pub struct Package {
    zip: ZipWriter<Cursor<Vec<u8>>>,
}

impl Package {
    pub fn new(mimetype: &str) -> zip::result::ZipResult<Self> {
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file(
            "mimetype",
            SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
        )?;
        zip.write_all(mimetype.as_bytes())?;
        Ok(Package { zip })
    }

    pub fn add(&mut self, name: &str, data: &[u8]) -> zip::result::ZipResult<()> {
        self.zip.start_file(
            name,
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
        )?;
        self.zip.write_all(data)?;
        Ok(())
    }

    pub fn finish(self) -> zip::result::ZipResult<Vec<u8>> {
        Ok(self.zip.finish()?.into_inner())
    }
}

/// Copy a zip archive, replacing some entries (used to apply reference styles).
pub fn replace_entries(
    archive: &[u8],
    replacements: &[(&str, Vec<u8>)],
) -> zip::result::ZipResult<Vec<u8>> {
    let mut input = zip::ZipArchive::new(Cursor::new(archive))?;
    let mut output = ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..input.len() {
        let mut entry = input.by_index(i)?;
        let name = entry.name().to_string();
        let method = if name == "mimetype" {
            CompressionMethod::Stored
        } else {
            CompressionMethod::Deflated
        };
        output.start_file(
            name.as_str(),
            SimpleFileOptions::default().compression_method(method),
        )?;
        match replacements.iter().find(|(n, _)| *n == name) {
            Some((_, data)) => output.write_all(data)?,
            None => {
                std::io::copy(&mut entry, &mut output)?;
            }
        }
    }
    Ok(output.finish()?.into_inner())
}

/// Read one entry from a zip archive (a reference document's styles).
pub fn read_entry(archive: &[u8], name: &str) -> Option<Vec<u8>> {
    use std::io::Read;
    let mut input = zip::ZipArchive::new(Cursor::new(archive)).ok()?;
    let mut entry = input.by_name(name).ok()?;
    let mut data = Vec::new();
    entry.read_to_end(&mut data).ok()?;
    Some(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaping() {
        assert_eq!(
            escape("a<b & \"c\" 'd'\u{1}"),
            "a&lt;b &amp; &quot;c&quot; &apos;d&apos;"
        );
    }

    #[test]
    fn packages_start_with_a_stored_mimetype_and_entries_can_be_replaced() {
        let mut package = Package::new("application/epub+zip").unwrap();
        package.add("a.txt", b"old").unwrap();
        package.add("b.txt", b"keep").unwrap();
        let bytes = package.finish().unwrap();
        // The mimetype is readable at a fixed offset, as e-readers expect.
        assert_eq!(&bytes[30..38], b"mimetype");
        assert_eq!(&bytes[38..58], b"application/epub+zip");

        let replaced = replace_entries(&bytes, &[("a.txt", b"new".to_vec())]).unwrap();
        assert_eq!(read_entry(&replaced, "a.txt").unwrap(), b"new");
        assert_eq!(read_entry(&replaced, "b.txt").unwrap(), b"keep");
        assert_eq!(&replaced[30..38], b"mimetype");
        assert!(read_entry(&replaced, "missing").is_none());
    }
}
