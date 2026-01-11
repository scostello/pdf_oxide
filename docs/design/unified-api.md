# Unified API Design

## Problem Statement

Currently, pdf_oxide has three separate entry points:
- `PdfDocument` - Reading and extraction
- `DocumentEditor` - Editing existing PDFs
- `DocumentBuilder` - Creating new PDFs

This feels like 3 libraries, not one unified toolkit.

## Goal

One object that does everything:

```python
# Python API (target)
doc = PdfDocument.open("input.pdf")

# Extract
doc.to_markdown("output.md")

# Read & Edit
for text in doc.texts:
    print(text.value)
    if text.value == "1":
        text.value = "2"

# Save
doc.save("output.pdf")
```

```rust
// Rust API (target)
let mut doc = PdfDocument::open("input.pdf")?;

// Extract
doc.to_markdown_file("output.md")?;

// Read & Edit
for text in doc.texts_mut(0)? {
    println!("{}", text.text());
    if text.text() == "1" {
        text.set_text("2");
    }
}

// Save
doc.save("output.pdf")?;
```

---

## Current Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  PdfDocument    │     │  DocumentEditor  │     │ DocumentBuilder │
│  (read-only)    │     │  (wraps source)  │     │  (create new)   │
├─────────────────┤     ├──────────────────┤     ├─────────────────┤
│ open()          │     │ open()           │     │ new()           │
│ to_markdown()   │     │ get_page()       │     │ page()          │
│ to_html()       │     │ save_page()      │     │ build()         │
│ extract_spans() │     │ save()           │     │ save()          │
│ page_count()    │     │                  │     │                 │
└─────────────────┘     └──────────────────┘     └─────────────────┘
```

**Issues:**
1. User must know which class to use for which operation
2. Can't seamlessly go from reading to editing
3. `DocumentEditor` wraps `PdfDocument` internally (duplication)

---

## Proposed Architecture

### Option A: Extend PdfDocument (Minimal Changes)

Add editing capabilities directly to `PdfDocument`:

```rust
impl PdfDocument {
    // Existing methods (unchanged)
    pub fn open(path: impl AsRef<Path>) -> Result<Self>;
    pub fn page_count(&mut self) -> Result<usize>;
    pub fn to_markdown(&mut self, page: usize, options: &ConversionOptions) -> Result<String>;
    pub fn to_html(&mut self, page: usize, options: &ConversionOptions) -> Result<String>;

    // NEW: Text access for editing
    pub fn texts(&mut self, page: usize) -> Result<TextCollection>;

    // NEW: Save changes
    pub fn save(&mut self, path: impl AsRef<Path>) -> Result<()>;

    // NEW: Check if modified
    pub fn is_modified(&self) -> bool;
}

/// Collection of text elements on a page
pub struct TextCollection {
    elements: Vec<TextElement>,
    modified: bool,
}

impl TextCollection {
    pub fn iter(&self) -> impl Iterator<Item = &TextElement>;
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut TextElement>;
    pub fn find(&self, needle: &str) -> Vec<&TextElement>;
    pub fn find_mut(&mut self, needle: &str) -> Vec<&mut TextElement>;
}

/// A single text element
pub struct TextElement {
    text: String,
    bbox: Rect,
    font: FontSpec,
    style: TextStyle,
    // ... tracking for modifications
}

impl TextElement {
    pub fn text(&self) -> &str;
    pub fn set_text(&mut self, value: impl Into<String>);
    pub fn bbox(&self) -> Rect;
    pub fn font_name(&self) -> &str;
    pub fn font_size(&self) -> f32;
    // ...
}
```

**Pros:**
- Minimal changes to existing code
- Single entry point
- Backwards compatible

**Cons:**
- `PdfDocument` becomes larger
- Mixes read-only and mutable concerns

### Option B: Make DocumentEditor the Primary API

Rename/rebrand `DocumentEditor` as the main entry point:

```rust
// Rename DocumentEditor -> PdfDocument (breaking change)
// Or keep both with DocumentEditor as primary
pub type Pdf = DocumentEditor;  // Alias for convenience

let mut doc = Pdf::open("input.pdf")?;
doc.to_markdown(0)?;
for text in doc.texts(0)? { ... }
doc.save("output.pdf")?;
```

**Pros:**
- Clean separation maintained internally
- DocumentEditor already has editing infrastructure

**Cons:**
- Breaking change if we rename
- Or confusing if we have both

### Option C: Facade Pattern (Recommended)

Create a unified `Pdf` struct that delegates to internal components:

```rust
/// The Complete PDF Toolkit - unified API
pub struct Pdf {
    /// Source document (for reading)
    source: PdfDocument,
    /// Page modifications (lazy-initialized)
    modifications: HashMap<usize, PageModifications>,
    /// Track if document was modified
    modified: bool,
    /// Original file path (for save())
    path: Option<PathBuf>,
}

impl Pdf {
    // === Opening ===
    pub fn open(path: impl AsRef<Path>) -> Result<Self>;
    pub fn from_bytes(data: &[u8]) -> Result<Self>;

    // === Document Info ===
    pub fn page_count(&mut self) -> Result<usize>;
    pub fn metadata(&self) -> &DocumentMetadata;

    // === Extraction (delegates to PdfDocument) ===
    pub fn to_markdown(&mut self, page: usize) -> Result<String>;
    pub fn to_html(&mut self, page: usize) -> Result<String>;
    pub fn to_text(&mut self, page: usize) -> Result<String>;
    pub fn to_markdown_file(&mut self, path: impl AsRef<Path>) -> Result<()>;

    // === Text Access ===
    pub fn texts(&mut self, page: usize) -> Result<TextCollection>;
    pub fn texts_mut(&mut self, page: usize) -> Result<TextCollectionMut>;

    // === Image Access ===
    pub fn images(&mut self, page: usize) -> Result<ImageCollection>;

    // === Annotations ===
    pub fn annotations(&mut self, page: usize) -> Result<AnnotationCollection>;
    pub fn add_annotation(&mut self, page: usize, annotation: Annotation) -> Result<()>;

    // === Saving ===
    pub fn save(&mut self) -> Result<()>;  // Save to original path
    pub fn save_as(&mut self, path: impl AsRef<Path>) -> Result<()>;
    pub fn to_bytes(&mut self) -> Result<Vec<u8>>;

    // === Status ===
    pub fn is_modified(&self) -> bool;
}
```

**Pros:**
- Clean unified API
- Internal architecture unchanged
- Can optimize later without breaking API
- Clear ownership model

**Cons:**
- New struct to maintain
- Some delegation boilerplate

---

## Recommended Approach: Option C (Facade)

### Phase 1: Create `Pdf` struct (v0.4.0)

1. Create `src/pdf.rs` with unified `Pdf` struct
2. Implement delegation to `PdfDocument` for reading
3. Implement `texts()` returning `TextCollection`
4. Implement `save()` using `PdfWriter`

### Phase 2: Python Bindings Update

Update Python bindings to expose `Pdf` as `PdfDocument`:

```python
from pdf_oxide import PdfDocument

doc = PdfDocument("input.pdf")
print(doc.to_markdown(0))

for text in doc.texts(0):
    if "old" in text.value:
        text.value = text.value.replace("old", "new")

doc.save("output.pdf")
```

### Phase 3: Deprecation (v0.5.0+)

- Mark `DocumentEditor` as deprecated
- Update all docs to use `Pdf`
- Keep `PdfDocument` for low-level access

---

## Text Collection Design

```rust
/// Immutable collection of text elements
pub struct TextCollection<'a> {
    page: &'a PdfPage,
}

impl<'a> TextCollection<'a> {
    pub fn iter(&self) -> impl Iterator<Item = TextRef<'a>>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn find(&self, needle: &str) -> Vec<TextRef<'a>>;
    pub fn get(&self, index: usize) -> Option<TextRef<'a>>;
}

/// Mutable collection of text elements
pub struct TextCollectionMut<'a> {
    page: &'a mut PdfPage,
}

impl<'a> TextCollectionMut<'a> {
    pub fn iter_mut(&mut self) -> impl Iterator<Item = TextRefMut<'a>>;
    pub fn find_mut(&mut self, needle: &str) -> Vec<TextRefMut<'a>>;

    /// Replace all occurrences of `old` with `new`
    pub fn replace_all(&mut self, old: &str, new: &str) -> usize;
}

/// Reference to a text element (immutable)
pub struct TextRef<'a> {
    element: &'a TextContent,
}

impl TextRef<'_> {
    pub fn text(&self) -> &str;
    pub fn value(&self) -> &str { self.text() }  // Alias for Python-like API
    pub fn bbox(&self) -> Rect;
    pub fn font(&self) -> &str;
    pub fn size(&self) -> f32;
}

/// Reference to a text element (mutable)
pub struct TextRefMut<'a> {
    element: &'a mut TextContent,
    modified: &'a mut bool,
}

impl TextRefMut<'_> {
    pub fn text(&self) -> &str;
    pub fn value(&self) -> &str { self.text() }

    pub fn set_text(&mut self, value: impl Into<String>);
    pub fn set_value(&mut self, value: impl Into<String>) {
        self.set_text(value)
    }
}
```

---

## File Changes Required

### New Files
- `src/pdf.rs` - Unified `Pdf` struct
- `src/collections/mod.rs` - TextCollection, ImageCollection, etc.
- `src/collections/text.rs` - Text collection implementation

### Modified Files
- `src/lib.rs` - Export `Pdf` as primary API
- `python/pdf_oxide/__init__.py` - Update Python bindings
- `README.md` - Update examples

### Unchanged
- `src/document.rs` - Keep PdfDocument as-is (internal use)
- `src/editor/` - Keep DocumentEditor as-is (internal use)
- `src/writer/` - Keep as-is (used by save())

---

## Migration Path

### For Rust Users

```rust
// Before (v0.3.x)
use pdf_oxide::PdfDocument;
use pdf_oxide::editor::DocumentEditor;

let mut doc = PdfDocument::open("input.pdf")?;
let text = doc.to_markdown(0, &options)?;

let mut editor = DocumentEditor::open("input.pdf")?;
let page = editor.get_page(0)?;
// ...

// After (v0.4.0+)
use pdf_oxide::Pdf;

let mut doc = Pdf::open("input.pdf")?;
let text = doc.to_markdown(0)?;

for text in doc.texts_mut(0)? {
    text.set_text("modified");
}
doc.save()?;
```

### For Python Users

```python
# Before (v0.3.x) - extraction only
from pdf_oxide import PdfDocument
doc = PdfDocument("input.pdf")
text = doc.to_markdown(0)

# After (v0.4.0+) - unified
from pdf_oxide import PdfDocument
doc = PdfDocument("input.pdf")
text = doc.to_markdown(0)

for t in doc.texts(0):
    if t.value == "old":
        t.value = "new"
doc.save("output.pdf")
```

---

## Questions to Resolve

1. **Naming**: `Pdf` vs `PdfDocument` vs `Document`?
2. **Mutability**: Should `texts()` return mutable by default, or have separate `texts()` / `texts_mut()`?
3. **Save behavior**: Auto-save on drop? Explicit save only?
4. **Memory**: Load all text upfront or lazy-load per page?
5. **Thread safety**: Should `Pdf` be `Send + Sync`?

---

## Next Steps

1. Review and approve this design
2. Create `src/pdf.rs` with basic structure
3. Implement `texts()` method
4. Implement `save()` method
5. Add tests
6. Update Python bindings
7. Update documentation
