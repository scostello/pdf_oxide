//! Integration tests for the DOM-like PDF editing API.

#[cfg(test)]
mod dom_api_tests {
    use pdf_oxide::editor::{ElementId, PdfPage};
    use pdf_oxide::elements::{ContentElement, FontSpec, StructureElement, TextContent, TextStyle};
    use pdf_oxide::geometry::Rect;

    /// Create a test page with some sample content.
    fn create_test_page() -> PdfPage {
        let text1 = TextContent {
            text: "Hello World".to_string(),
            bbox: Rect::new(72.0, 720.0, 100.0, 12.0),
            font: FontSpec::default(),
            style: TextStyle::default(),
            reading_order: Some(0),
        };

        let text2 = TextContent {
            text: "This is a test".to_string(),
            bbox: Rect::new(72.0, 700.0, 100.0, 12.0),
            font: FontSpec::default(),
            style: TextStyle::default(),
            reading_order: Some(1),
        };

        let children = vec![ContentElement::Text(text1), ContentElement::Text(text2)];

        let root = StructureElement {
            structure_type: "Document".to_string(),
            bbox: Rect::new(0.0, 0.0, 612.0, 792.0),
            children,
            reading_order: Some(0),
            alt_text: None,
            language: None,
        };

        PdfPage::from_structure(0, root, 612.0, 792.0)
    }

    #[test]
    fn test_find_text_containing() {
        let page = create_test_page();

        let results = page.find_text_containing("Hello");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].text(), "Hello World");
    }

    #[test]
    fn test_find_text_with_predicate() {
        let page = create_test_page();

        // Find text starting with "This"
        let results = page.find_text(|t| t.text().starts_with("This"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].text(), "This is a test");
    }

    #[test]
    fn test_find_text_no_match() {
        let page = create_test_page();

        let results = page.find_text_containing("Nonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn test_pdf_text_properties() {
        let page = create_test_page();

        let results = page.find_text_containing("Hello");
        assert_eq!(results.len(), 1);

        let text = &results[0];
        assert_eq!(text.text(), "Hello World");
        assert_eq!(text.bbox().x, 72.0);
        assert_eq!(text.bbox().y, 720.0);
        assert_eq!(text.font_name(), "Helvetica");
        assert_eq!(text.font_size(), 12.0);
        assert!(!text.is_bold());
        assert!(!text.is_italic());
    }

    #[test]
    fn test_element_id_uniqueness() {
        let id1 = ElementId::new();
        let id2 = ElementId::new();

        assert_ne!(id1, id2);
    }

    #[test]
    fn test_children_access() {
        let page = create_test_page();

        let children = page.children();
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn test_match_pdf_element() {
        let page = create_test_page();
        let children = page.children();

        for element in children {
            match element {
                pdf_oxide::editor::PdfElement::Text(text) => {
                    assert!(!text.text().is_empty());
                },
                _ => panic!("Expected Text element"),
            }
        }
    }

    #[test]
    fn test_page_dimensions() {
        let page = create_test_page();

        assert_eq!(page.width, 612.0);
        assert_eq!(page.height, 792.0);
    }

    #[test]
    fn test_element_bbox() {
        let page = create_test_page();

        let results = page.find_text_containing("Hello");
        assert!(!results.is_empty());

        let bbox = results[0].bbox();
        assert_eq!(bbox.x, 72.0);
        assert_eq!(bbox.y, 720.0);
        assert_eq!(bbox.width, 100.0);
        assert_eq!(bbox.height, 12.0);
    }

    #[test]
    fn test_find_in_region() {
        let page = create_test_page();

        // Search in region covering first text
        let region = Rect::new(0.0, 710.0, 200.0, 30.0);
        let elements = page.find_in_region(region);

        assert!(!elements.is_empty());
        // Should find the first "Hello World" text
        if let Some(text) = elements[0].as_text() {
            assert_eq!(text.text(), "Hello World");
        }
    }

    #[test]
    fn test_find_in_region_no_match() {
        let page = create_test_page();

        // Search in region that doesn't contain any elements
        let region = Rect::new(0.0, 0.0, 50.0, 50.0);
        let elements = page.find_in_region(region);

        assert!(elements.is_empty());
    }

    #[test]
    fn test_pdf_page_creation() {
        let page = create_test_page();

        assert_eq!(page.page_index, 0);
        assert_eq!(page.width, 612.0);
        assert_eq!(page.height, 792.0);
    }

    #[test]
    fn test_all_find_methods() {
        let page = create_test_page();

        // find_text_containing
        let by_contains = page.find_text_containing("Hello");
        assert!(!by_contains.is_empty());

        // find_text with predicate
        let by_predicate = page.find_text(|t| t.text().contains("Hello"));
        assert!(!by_predicate.is_empty());

        // Should have same results
        assert_eq!(by_contains.len(), by_predicate.len());
    }

    #[test]
    fn test_empty_page() {
        let root = StructureElement {
            structure_type: "Document".to_string(),
            bbox: Rect::new(0.0, 0.0, 612.0, 792.0),
            children: Vec::new(),
            reading_order: Some(0),
            alt_text: None,
            language: None,
        };

        let page = PdfPage::from_structure(0, root, 612.0, 792.0);

        let results = page.find_text_containing("anything");
        assert!(results.is_empty());
    }

    #[test]
    fn test_fluent_api_find_and_modify() {
        let page = create_test_page();

        // Test fluent API with mutable modification
        let mut editor = pdf_oxide::editor::PageEditor { page };
        let result = editor
            .find_text_containing("Hello")
            .expect("find_text_containing failed");

        let modified = result
            .for_each(|mut text| {
                text.set_text("Hi");
                Ok(())
            })
            .expect("for_each failed")
            .done()
            .expect("done failed");

        // Verify the modification happened
        let modified_text = modified.find_text_containing("Hi");
        assert_eq!(modified_text.len(), 1);
        assert_eq!(modified_text[0].text(), "Hi");
    }

    #[test]
    fn test_fluent_api_chaining() {
        let page = create_test_page();

        // Test fluent API chaining style (XMLDocument-like)
        let result = pdf_oxide::editor::PageEditor { page }
            .find_text_containing("Hello")
            .expect("find_text_containing failed")
            .for_each(|mut text| {
                text.set_text("Modified");
                Ok(())
            })
            .expect("for_each failed")
            .done();

        assert!(result.is_ok());
        let modified_page = result.unwrap();
        let modified_text = modified_page.find_text_containing("Modified");
        assert_eq!(modified_text.len(), 1);
    }

    #[test]
    fn test_fluent_api_with_predicate() {
        let page = create_test_page();

        let result = pdf_oxide::editor::PageEditor { page }
            .find_text(|t| t.text().starts_with("Hello"))
            .expect("find_text failed")
            .for_each(|mut text| {
                text.set_text("Updated");
                Ok(())
            })
            .expect("for_each failed")
            .done();

        assert!(result.is_ok());
        let modified_page = result.unwrap();
        let updated_text = modified_page.find_text_containing("Updated");
        assert_eq!(updated_text.len(), 1);
        assert_eq!(updated_text[0].text(), "Updated");
    }
}
