//! High-level API for reading, writing, and converting DOCX documents.
//!
//! Provides a Python-docx-like interface for working with Word documents.
//!
//! # Examples
//!
//! ```no_run
//! use rdocx::Document;
//!
//! // Create a new document
//! let mut doc = Document::new();
//! doc.add_paragraph("Hello, World!");
//! doc.save("output.docx").unwrap();
//!
//! // Open an existing document
//! let doc = Document::open("existing.docx").unwrap();
//! for para in doc.paragraphs() {
//!     println!("{}", para.text());
//! }
//! ```

#![allow(clippy::too_many_arguments)]

mod comments;
mod comparison;
mod content_control;
mod document;
mod error;
mod field;
pub mod paragraph;
mod redaction;
mod revision;
pub mod run;
pub mod style;
pub mod table;
mod template;

pub use comments::{BookmarkRef, CommentRef, RunPosition, RunRange};
pub use comparison::ComparisonDiagnostic;
pub use content_control::ContentControlRef;
pub use document::{
    AccessibilityIssue, BodyItemRef, Document, ImageInfo, IssueSeverity, LinkInfo, ListLevel,
    ListNumberFormat, OutlineNode, RenderOptions,
};
pub use error::{Error, Result};
pub use field::{FieldDateTime, FieldEvaluation, FieldEvaluationContext, FieldOutcome};
pub use oxml_chart::{ChartData, ChartKind};
pub use oxml_core::Length;
pub use oxml_opc::PackageReadLimits;
#[cfg(feature = "digital-signatures")]
pub use oxml_opc::{
    CoveredRelationship, SignatureIssue, SignatureReport, SignerCertificateIdentity,
};
pub use paragraph::{
    Alignment, BorderStyle, Paragraph, ParagraphBorderRef, ParagraphRef, SectionBreak,
    TabAlignment, TabLeader,
};
pub use rdocx_layout::RevisionView;
// SVG PoC patch: renderer-facing provenance types.
pub use rdocx_layout::{WordLayoutResult, WordSourcePath, WordStory};
pub use rdocx_oxml::settings::{
    CryptAlgorithmClass, CryptAlgorithmType, CryptProviderType, DocumentProtection, ProtectionMode,
};
pub use redaction::RedactionReport;
pub use revision::{RevisionKind, RevisionRef};
pub use run::{Run, RunRef, UnderlineStyle};
pub use style::{Style, StyleBuilder};
pub use table::{Cell, CellRef, Row, RowRef, Table, TableRef, VerticalAlignment};

#[cfg(test)]
mod tests {
    use super::Length;

    #[test]
    fn public_length_is_the_shared_type() {
        let _: oxml_core::Length = Length::emu(1);
    }

    #[cfg(feature = "digital-signatures")]
    #[test]
    fn native_document_forwards_signature_verification() {
        let document = crate::Document::new();
        assert!(document.verify_signatures().unwrap().is_empty());
    }

    #[cfg(all(feature = "digital-signatures", not(target_arch = "wasm32")))]
    #[test]
    fn native_document_exposes_atomic_signature_creation() {
        let sign: fn(&mut crate::Document, &[u8], &[u8]) -> crate::Result<crate::SignatureReport> =
            crate::Document::sign;
        let mut document = crate::Document::new();
        document.add_paragraph("atomic native signature creation");
        let before = document.to_bytes().unwrap();
        assert!(sign(&mut document, b"not-pkcs8", b"not-x509").is_err());
        assert_eq!(document.to_bytes().unwrap(), before);
        assert!(document.verify_signatures().unwrap().is_empty());
    }
}
