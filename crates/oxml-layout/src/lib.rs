//! Format-neutral layout output and font primitives.

pub mod bundled_fonts;
pub mod error;
pub mod font;
pub mod line;
pub mod output;
pub mod paint;
pub mod path;
pub mod transform;

pub use error::{LayoutError, Result};
pub use font::{FontFile, FontManager, FontMetrics, ShapedText};
pub use line::{
    Align, InlineItem, LayoutLine, LineBreakParams, LineItem, LineSpacing, NoteRef, NoteStream,
    TabAlign, TabLeader, TabStop, TextSegment, Underline, break_into_lines,
};
pub use output::{
    Color, Diagnostic, DocumentMetadata, Effect, FieldKind, FontData, FontId, GlyphRun,
    GroupElement, LayoutResult, MediaId, OutlineEntry, PageFrame, PathElement, Point,
    PositionedElement, Rect, SourceNodeId, SourceSpan, walk,
};
pub use paint::{GradientStop, LineCap, LineJoin, Paint, Stroke};
pub use path::{FillRule, Path, PathCommand};
pub use transform::Transform;
