//! PDF signing implementation.
//!
//! This module handles the creation of digital signatures for PDF documents.

use super::byterange::ByteRangeCalculator;
use super::types::{DigestAlgorithm, SignOptions, SigningCredentials};
use crate::error::{Error, Result};

#[cfg(feature = "signatures")]
use sha2::{Digest, Sha256, Sha384, Sha512};

#[cfg(feature = "signatures")]
use sha1::Sha1;

/// PDF signer that creates digital signatures.
pub struct PdfSigner {
    credentials: SigningCredentials,
    options: SignOptions,
    byte_range_calc: ByteRangeCalculator,
}

impl PdfSigner {
    /// Create a new PDF signer with the given credentials and options.
    pub fn new(credentials: SigningCredentials, options: SignOptions) -> Self {
        let byte_range_calc = ByteRangeCalculator::new(options.estimated_size);
        Self {
            credentials,
            options,
            byte_range_calc,
        }
    }

    /// Get the placeholder size for the signature.
    pub fn placeholder_size(&self) -> usize {
        self.byte_range_calc.placeholder_size()
    }

    /// Generate the placeholder for the /Contents value.
    pub fn generate_contents_placeholder(&self) -> String {
        self.byte_range_calc.generate_placeholder()
    }

    /// Build the signature dictionary content (without /Contents value).
    ///
    /// This returns the dictionary entries that should appear in the signature
    /// dictionary. The actual /Contents value should be set to the placeholder.
    pub fn build_signature_dictionary(&self) -> String {
        let mut dict = String::new();

        // Required fields
        dict.push_str("/Type /Sig\n");
        dict.push_str("/Filter /Adobe.PPKLite\n");
        dict.push_str(&format!("/SubFilter /{}\n", self.options.sub_filter.as_pdf_name()));

        // ByteRange placeholder - will be filled in after file is assembled
        dict.push_str("/ByteRange [0 0 0 0]\n");

        // Optional fields
        if let Some(ref name) = self.options.name {
            dict.push_str(&format!("/Name ({})\n", escape_pdf_string(name)));
        }

        if let Some(ref reason) = self.options.reason {
            dict.push_str(&format!("/Reason ({})\n", escape_pdf_string(reason)));
        }

        if let Some(ref location) = self.options.location {
            dict.push_str(&format!("/Location ({})\n", escape_pdf_string(location)));
        }

        if let Some(ref contact) = self.options.contact_info {
            dict.push_str(&format!("/ContactInfo ({})\n", escape_pdf_string(contact)));
        }

        // Signing time (M field)
        let signing_time = format_pdf_date();
        dict.push_str(&format!("/M ({})\n", signing_time));

        dict
    }

    /// Compute the digest of the signed bytes.
    #[cfg(feature = "signatures")]
    pub fn compute_digest(&self, signed_bytes: &[u8]) -> Vec<u8> {
        match self.options.digest_algorithm {
            DigestAlgorithm::Sha1 => {
                let mut hasher = Sha1::new();
                hasher.update(signed_bytes);
                hasher.finalize().to_vec()
            },
            DigestAlgorithm::Sha256 => {
                let mut hasher = Sha256::new();
                hasher.update(signed_bytes);
                hasher.finalize().to_vec()
            },
            DigestAlgorithm::Sha384 => {
                let mut hasher = Sha384::new();
                hasher.update(signed_bytes);
                hasher.finalize().to_vec()
            },
            DigestAlgorithm::Sha512 => {
                let mut hasher = Sha512::new();
                hasher.update(signed_bytes);
                hasher.finalize().to_vec()
            },
        }
    }

    /// Sign the document and return the PKCS#7 signature.
    ///
    /// This is the main signing method that:
    /// 1. Computes the digest of the signed bytes
    /// 2. Creates the PKCS#7/CMS signature structure
    /// 3. Returns the DER-encoded signature
    #[cfg(feature = "signatures")]
    pub fn sign(&self, signed_bytes: &[u8]) -> Result<Vec<u8>> {
        // Compute digest
        let digest = self.compute_digest(signed_bytes);

        // Create PKCS#7 signature
        self.create_pkcs7_signature(&digest)
    }

    /// Create a PKCS#7 detached signature.
    #[cfg(feature = "signatures")]
    fn create_pkcs7_signature(&self, _digest: &[u8]) -> Result<Vec<u8>> {
        // This would use the cms crate to create a proper PKCS#7 structure
        // For now, return a placeholder indicating the feature needs more implementation

        // TODO: Implement full PKCS#7 signature creation using:
        // - cms::signed_data::SignedDataBuilder
        // - rsa::pkcs1v15::SigningKey for RSA signatures
        // - x509_parser for certificate parsing

        Err(Error::InvalidPdf(
            "Full PKCS#7 signature creation not yet implemented. \
             This requires additional integration with the cms and rsa crates."
                .to_string(),
        ))
    }

    /// Calculate the ByteRange for a prepared PDF.
    pub fn calculate_byte_range(&self, file_size: usize, contents_offset: usize) -> [i64; 4] {
        self.byte_range_calc
            .calculate_byte_range(file_size, contents_offset)
    }

    /// Extract the bytes to be signed from the PDF.
    pub fn extract_signed_bytes(pdf_data: &[u8], byte_range: &[i64; 4]) -> Result<Vec<u8>> {
        ByteRangeCalculator::extract_signed_bytes(pdf_data, byte_range)
    }

    /// Insert the signature into the prepared PDF.
    pub fn insert_signature(
        &self,
        pdf_data: &mut [u8],
        contents_offset: usize,
        signature: &[u8],
    ) -> Result<()> {
        // Convert signature to hex
        let signature_hex = bytes_to_hex(signature);
        self.byte_range_calc
            .insert_signature(pdf_data, contents_offset, &signature_hex)
    }

    /// Get the signing options.
    pub fn options(&self) -> &SignOptions {
        &self.options
    }

    /// Get the signing credentials (certificate info only).
    pub fn credentials(&self) -> &SigningCredentials {
        &self.credentials
    }
}

/// Convert bytes to uppercase hex string.
fn bytes_to_hex(bytes: &[u8]) -> String {
    const HEX_CHARS: &[u8] = b"0123456789ABCDEF";
    let mut hex = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        hex.push(HEX_CHARS[(byte >> 4) as usize] as char);
        hex.push(HEX_CHARS[(byte & 0x0F) as usize] as char);
    }
    hex
}

/// Escape special characters in a PDF string.
fn escape_pdf_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 10);
    for c in s.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            '(' => result.push_str("\\("),
            ')' => result.push_str("\\)"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            _ => result.push(c),
        }
    }
    result
}

/// Format current time as a PDF date string.
fn format_pdf_date() -> String {
    use std::time::SystemTime;

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Convert to a simple date format: D:YYYYMMDDHHmmSS
    // This is a simplified version - a real implementation would use chrono
    let secs_per_day = 86400;
    let days_since_1970 = now / secs_per_day;
    let secs_today = now % secs_per_day;

    // Very rough approximation for date calculation
    let years = 1970 + (days_since_1970 / 365);
    let hours = secs_today / 3600;
    let mins = (secs_today % 3600) / 60;
    let secs = secs_today % 60;

    format!("D:{:04}0101{:02}{:02}{:02}Z", years, hours, mins, secs)
}

// SignOptions is re-exported from super::types

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_pdf_string() {
        assert_eq!(escape_pdf_string("Hello"), "Hello");
        assert_eq!(escape_pdf_string("Hello (World)"), "Hello \\(World\\)");
        assert_eq!(escape_pdf_string("Line1\nLine2"), "Line1\\nLine2");
        assert_eq!(escape_pdf_string("Path\\to\\file"), "Path\\\\to\\\\file");
    }

    #[test]
    fn test_format_pdf_date() {
        let date = format_pdf_date();
        assert!(date.starts_with("D:"));
        assert!(date.ends_with("Z"));
    }

    #[test]
    fn test_signer_placeholder() {
        let creds = SigningCredentials::new(vec![], vec![]);
        let opts = SignOptions {
            estimated_size: 1024,
            ..Default::default()
        };
        let signer = PdfSigner::new(creds, opts);

        let placeholder = signer.generate_contents_placeholder();
        // 1024 * 2 + 2 = 2050 characters
        assert_eq!(placeholder.len(), 2050);
        assert!(placeholder.starts_with('<'));
        assert!(placeholder.ends_with('>'));
    }

    #[test]
    fn test_build_signature_dictionary() {
        let creds = SigningCredentials::new(vec![], vec![]);
        let opts = SignOptions {
            reason: Some("Test signing".to_string()),
            location: Some("Test City".to_string()),
            ..Default::default()
        };
        let signer = PdfSigner::new(creds, opts);

        let dict = signer.build_signature_dictionary();
        assert!(dict.contains("/Type /Sig"));
        assert!(dict.contains("/Filter /Adobe.PPKLite"));
        assert!(dict.contains("/SubFilter /adbe.pkcs7.detached"));
        assert!(dict.contains("/Reason (Test signing)"));
        assert!(dict.contains("/Location (Test City)"));
        assert!(dict.contains("/ByteRange"));
        assert!(dict.contains("/M (D:"));
    }

    #[test]
    fn test_calculate_byte_range() {
        let creds = SigningCredentials::new(vec![], vec![]);
        let opts = SignOptions {
            estimated_size: 50, // 50 bytes = 102 char placeholder
            ..Default::default()
        };
        let signer = PdfSigner::new(creds, opts);

        let byte_range = signer.calculate_byte_range(1000, 400);
        assert_eq!(byte_range[0], 0);
        assert_eq!(byte_range[1], 400);
        assert_eq!(byte_range[2], 502); // 400 + 102
        assert_eq!(byte_range[3], 498); // 1000 - 502
    }
}
