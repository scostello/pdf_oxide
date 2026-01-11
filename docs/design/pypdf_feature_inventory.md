# pypdf Feature Inventory

## Overview

**pypdf** is the actively maintained successor to PyPDF2, PyPDF3, and PyPDF4. PyPDF2 was deprecated in 2023, with all development continuing under the `pypdf` package (all lowercase, no number). The library is a pure-Python PDF toolkit under BSD license.

- **Latest Version**: 6.6.0 (January 9, 2026)
- **Python Support**: 3.9 through 3.14
- **License**: BSD
- **Repository**: https://github.com/py-pdf/pypdf

---

## 1. Text Extraction

### Basic Extraction
- `page.extract_text()` - Extract all text from a page
- Orientation filtering - Extract text at specific angles (0, 90, 180, 270 degrees)

### Layout Mode
- `extraction_mode="layout"` - Preserves fixed-width formatting mirroring rendered PDF appearance
- `layout_mode_space_vertically=False` - Removes blank lines while maintaining horizontal positioning
- `layout_mode_scale_weight=1.0` - Adjusts horizontal spacing scale
- `layout_mode_strip_rotated=False` - Controls inclusion of rotated text

### Visitor Functions (Advanced)
Custom callbacks for fine-grained text extraction control:
- `visitor_text(text, user_matrix, tm_matrix, font_dictionary, font_size)` - Called for each text fragment
- `visitor_operand_before` - Processes operators with transformation matrices
- Access to x/y coordinates via transformation matrix indices 4 and 5
- Font dictionary and font size information available
- Can filter text by position, font size, or other criteria

### Limitations
- Not OCR software - cannot extract text from images or scanned documents
- PDFs lack semantic structure (no inherent knowledge of headers, footers, tables, paragraphs)
- Performance: ~10-20x slower than PyMuPDF/pypdfium2 for text extraction
- Complex PDFs may have incorrect coordinate information

---

## 2. Image Extraction

### Capabilities
- Extract images from pages via `page.images` iterator
- Extract images from annotations (stamps, etc.)
- Access image properties (name, data)
- `decode_as_image()` method for converting to image objects

### Requirements
- Pillow library required: `pip install pypdf[image]`

### Supported Operations
- Save extracted images to files
- Access image names via `page.images.keys()`
- Graceful error handling for problematic images

### Limitations
- Image filenames may not be unique across pages
- No built-in format conversion (relies on Pillow)

---

## 3. Page Manipulation

### Merging PDFs
- `PdfWriter.append()` - Add pages to end of document
- `PdfWriter.merge()` - Insert pages at specific position
- Page range selection: tuple `(0, 10)` or list `[0, 9]`
- Page repetition allowed: `[0, 1, 0, 2, 0]`
- Named destinations imported during merge
- Form field name conflict prevention via `add_form_topname()`

### Splitting PDFs
- Extract specific pages to new PDF
- `del` operator removes pages from writer
- `remove_page()` method with clean option

### Rotation
- `page.rotate(angle)` - Rotate by multiples of 90 degrees
- `transfer_rotation_to_content()` - Fix rotation issues in page boxes

### Cropping
- Adjust viewbox to crop visible content
- Content not deleted, just hidden (can be restored)
- MediaBox/CropBox manipulation

### Scaling
- Content scaling (scale page contents)
- Page scaling (scale canvas size)
- Combined scaling for uniform results

### Transformations
- `Transformation()` class for translate, rotate, scale operations
- `merge_transformed_page()` for applying transformations during merge

### Cloning
- `clone_from` parameter in PdfWriter constructor
- `clone_document_from_reader()` - Full document copy
- `clone_reader_document_root()` - Copy root with all sub-elements
- Automatic cloning during append/merge/add_page operations

---

## 4. Form Field Handling (AcroForm)

### Reading Forms
- `reader.get_fields()` - Returns all field objects with full properties
- `reader.get_form_text_fields()` - Returns simple text field values dict
- Access via page annotations: `page.annotations`
- `get_pages_showing_field()` - Find pages displaying a specific field

### Field Types Supported
- Text fields (`/FT: /Tx`)
- Button fields (checkboxes, radio buttons, push buttons) (`/FT: /Btn`)
- Choice fields (dropdowns, list boxes) (`/FT: /Ch`)
- Signature fields (`/FT: /Sig`)

### Writing/Filling Forms
```python
writer.update_page_form_field_values(
    writer.pages[0],
    {"fieldname": "value"},
    auto_regenerate=False
)
```
- `auto_regenerate=False` recommended to avoid "save changes" dialogs
- Hierarchical field names supported (e.g., "sender.city")

### Form Flattening
- `flatten=True` parameter converts fields to static content
- `remove_annotations(subtypes="/Widget")` to eliminate field widgets
- `reattach_fields()` repairs broken field structures

### Important Notes
- Fields stored in document, not pages - use `append()` not `add_page()`
- Radio buttons: `get_fields()` returns parent, `page.annotations` returns children

---

## 5. Encryption/Decryption

### Algorithms Supported
- RC4-40 (legacy, insecure)
- RC4-128 (legacy, insecure)
- AES-128
- AES-256-R5
- **AES-256** (recommended)

### Dependencies
- AES requires extra: `pip install pypdf[crypto]`
- Supports `cryptography` or `pycryptodome` libraries

### Encrypting
```python
writer.encrypt("user_password", algorithm="AES-256")
# Optional owner password as second argument
```

### Decrypting
```python
if reader.is_encrypted:
    reader.decrypt("password")
```

### Features
- User password vs Owner password distinction
- Permissions control
- PDF 2.0 encryption standard support

---

## 6. Metadata Handling

### Document Information Dictionary
- `reader.metadata` returns `DocumentInformation` object
- Standard fields: `/Author`, `/Title`, `/Subject`, `/Creator`, `/Producer`, `/CreationDate`, `/ModDate`
- Dictionary-style access and methods

### XMP Metadata
- `reader.xmp_metadata` returns `XmpInformation` object
- `XmpInformation.create()` - Create new XMP metadata

### XMP Properties Available
**Dublin Core (dc:)**
- `dc_contributor` - Contributors to the resource
- `dc_creator` - Authors in order of precedence
- `dc_description` - Language-keyed descriptions
- `dc_title` - Language-keyed titles

**PDF Namespace**
- `pdf_producer` - Tool that created the PDF

**XMP Namespace**
- `xmp_creator_tool` - First known creation tool
- `xmp_metadata_date` - Last metadata modification (UTC datetime)

**XMPMM Namespace**
- `xmpmm_document_id` - Common identifier for all versions
- `xmpmm_instance_id` - Specific incarnation identifier

---

## 7. Annotation Support

### Reading Annotations
Supports reading 28 PDF 2.0 annotation types:
- Text, FreeText, Link
- Line, PolyLine, Polygon
- Square, Circle, Ellipse
- Highlight, Underline, Squiggly, StrikeOut
- Caret, Stamp, Ink
- Popup, FileAttachment, Sound, Movie
- Screen, Widget, PrinterMark, TrapNet
- Watermark, 3D, Redact, Projection, RichMedia

### Adding Annotations
Annotation types that can be created:
- **FreeText** - Text boxes with font, color, border customization
- **Text** - Sticky note style annotations
- **Line** - Single lines with endpoints
- **PolyLine** - Multi-segment lines
- **Rectangle/Square** - Box shapes with optional fill
- **Ellipse/Circle** - Circular/oval shapes
- **Polygon** - Multi-sided shapes
- **Popup** - Expandable windows
- **Link** - External URLs and internal page references
- **Highlight** - Text markup (requires quad points)
- **Attachments** - Embedded files

### Customization Options
- Color: Grayscale (1), RGB (3), or CMYK (4) values
- Font: family, size, bold, italic
- Border color and background color
- Annotation flags (printable, invisible, etc.)
- Rectangle positioning

---

## 8. Watermarking/Stamping

### Basic Operation
- Stamp = overlay on top (`over=True`)
- Watermark = underlay behind content (`over=False`)

### Methods
- `page.merge_page(stamp_page, over=False)` - No transformation
- `page.merge_transformed_page(stamp_page, Transformation())` - With transformation

### Transformation Support
- Translate (move position)
- Rotate (any angle)
- Scale (resize)

### Image Watermarks
- Convert image to PDF first using Pillow
- Then merge as normal PDF page

### Troubleshooting
- `transfer_rotation_to_content()` fixes rotation issues

---

## 9. PDF/A Support

### Compliance Levels Documented
- **PDF/A-1b** (Level B): Basic visual preservation
- **PDF/A-1a** (Level A): Accessibility with tagging and Unicode
- **PDF/A-2b/u/a**: PDF 1.7 features, transparency layers
- **PDF/A-3**: Allows non-PDF/A file attachments (useful for invoices with XML)

### Current Status
- pypdf can work with PDF/A files
- No built-in validation - recommends external validators
- Documentation available for compliance considerations

### Limitations
- No automatic PDF/A conversion
- No built-in validation of PDF/A conformance
- External tools (like ConvertAPI) recommended for validation

---

## 10. Additional Features

### Bookmarks/Outlines
- `reader.outline` - Read-only access to document outline
- `writer.add_bookmark()` - Add bookmarks
- Hierarchical structure support
- Named destinations via `reader.named_destinations`

### Page Labels
- `reader.page_labels` - Read page label list
- `writer.set_page_label()` - Set label for page range
- Supports different numbering styles (decimal, roman, letters)
- Custom prefixes supported

### Attachments/Embedded Files
- `writer.add_attachment(filename, data)` - Embed files
- `reader.attachments` - Mapping of filenames to content
- `reader.attachment_list` - Iterable of attachment objects
- Extract attached files from annotations

### JavaScript
- `writer.add_js(javascript_code)` - Add JavaScript actions
- Common use: auto-print on open
- Reader support varies significantly

### Hyperlinks
- Read links from annotations (`/Annots` with `/Link` subtype)
- Access `/URI` for external URLs
- Access `/Dest` for internal destinations
- Add link annotations to pages

### Viewer Preferences
- Set initial view mode
- Control toolbar/menubar visibility
- Set page layout (single, continuous, two-column)

### File Size Reduction
- `compress_identical_objects(remove_identicals=True, remove_orphans=True)`
- `page.compress_content_streams(level=9)` - Lossless zlib compression
- Image quality reduction via `img.replace(img.image, quality=80)`
- `writer.remove_images()` - Complete image removal

### Incremental Writes
- `incremental=True` in PdfWriter constructor
- Preserves signatures in signed documents
- Appends changes rather than rewriting entire document
- `list_objects_in_increment()` for debugging

### Strict Mode
- `strict=True` - Raises exceptions on spec violations
- `strict=False` (default) - Logs warnings, attempts recovery
- Best-effort approach for malformed PDFs

### Robustness
- Handles malformed PDFs gracefully
- Workarounds for common specification violations
- Configurable strictness level

---

## 11. What pypdf Does NOT Support

### Digital Signatures
- Can detect if PDF is signed (via custom code)
- **Cannot validate signatures**
- **Cannot create signatures**
- Recommendation: Use **pyHanko** or **endesive** for signatures

### OCR
- Cannot extract text from images
- Cannot process scanned documents
- Recommendation: Use **pytesseract** or **Tesseract** preprocessing

### True Redaction
- No built-in secure redaction
- Visual overlay is not secure redaction
- Recommendation: Use **PyMuPDF** for proper redaction with content removal

### Table Extraction
- No built-in table detection/extraction
- Recommendation: Use **camelot-py**, **pdfplumber**, **tabula-py**, or **PyMuPDF**

### PDF Creation from Scratch
- Cannot generate PDFs from text/HTML
- Only manipulates existing PDFs
- Recommendation: Use **fpdf2**, **ReportLab**, or **weasyprint** for PDF creation

### Advanced Layout Analysis
- No semantic structure detection
- No header/footer detection
- No column detection
- Recommendation: Use **pdfplumber** or **PyMuPDF** for layout analysis

---

## 12. Performance Characteristics

### Speed
- Pure Python implementation (no C extensions)
- Text extraction: 10-20x slower than PyMuPDF/pypdfium2
- CPU-intensive operations: content stream compression

### Memory
- Large uncompressed streams may require significant memory
- Large file merging may require increased recursion limit

### Advantages
- No compilation required
- Works in restricted environments (Lambda, containers)
- No external dependencies for basic operations

---

## 13. API Classes Summary

### PdfReader
- Read existing PDF documents
- Access pages, metadata, outlines, forms
- Extract text and images
- Read annotations

### PdfWriter
- Create new PDF documents
- Clone and modify existing PDFs
- Add pages, merge documents
- Set encryption, metadata
- Add annotations, bookmarks, forms

### Transformation
- Define geometric transformations
- Translate, rotate, scale operations
- Used with merge_transformed_page

### PageObject
- Individual page manipulation
- Extract text and images
- Rotate, scale, crop
- Compress content streams

---

## Sources

- [pypdf Official Documentation](https://pypdf.readthedocs.io/en/stable/)
- [pypdf vs X Comparisons](https://pypdf.readthedocs.io/en/stable/meta/comparisons.html)
- [pypdf GitHub Repository](https://github.com/py-pdf/pypdf)
- [pypdf PyPI Package](https://pypi.org/project/pypdf/)
- [History of pypdf](https://pypdf.readthedocs.io/en/stable/meta/history.html)
- [PyPDF2 Deprecation Discussion](https://github.com/py-pdf/pypdf/discussions/2198)
- [pypdf Text Extraction](https://pypdf.readthedocs.io/en/stable/user/extract-text.html)
- [pypdf Forms Documentation](https://pypdf.readthedocs.io/en/stable/user/forms.html)
- [pypdf Encryption Documentation](https://pypdf.readthedocs.io/en/stable/user/encryption-decryption.html)
- [pypdf Metadata Documentation](https://pypdf.readthedocs.io/en/stable/user/metadata.html)
- [pypdf Annotations Documentation](https://pypdf.readthedocs.io/en/stable/user/adding-pdf-annotations.html)
- [pypdf Watermarks Documentation](https://pypdf.readthedocs.io/en/stable/user/add-watermark.html)
- [pypdf PDF/A Compliance](https://pypdf.readthedocs.io/en/stable/user/pdfa-compliance.html)
- [pypdf Merging PDFs](https://pypdf.readthedocs.io/en/stable/user/merging-pdfs.html)
- [pypdf File Size Reduction](https://pypdf.readthedocs.io/en/stable/user/file-size.html)
- [pypdf PdfWriter Class](https://pypdf.readthedocs.io/en/stable/modules/PdfWriter.html)
- [pypdf PdfReader Class](https://pypdf.readthedocs.io/en/stable/modules/PdfReader.html)
