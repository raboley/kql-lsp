use lsp_types::{Position, Uri};
use ropey::Rope;
use std::collections::HashMap;

/// A single open document backed by a Rope.
pub struct Document {
    pub _uri: Uri,
    pub version: i32,
    pub rope: Rope,
    pub _language_id: String,
}

/// Manages all open documents.
pub struct DocumentStore {
    documents: HashMap<Uri, Document>,
}

impl DocumentStore {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    pub fn open(&mut self, uri: Uri, version: i32, text: &str, language_id: &str) {
        let doc = Document {
            _uri: uri.clone(),
            version,
            rope: Rope::from_str(text),
            _language_id: language_id.to_string(),
        };
        self.documents.insert(uri, doc);
    }

    pub fn change_full(&mut self, uri: &Uri, version: i32, text: &str) {
        if let Some(doc) = self.documents.get_mut(uri) {
            doc.rope = Rope::from_str(text);
            doc.version = version;
        }
    }

    pub fn close(&mut self, uri: &Uri) {
        self.documents.remove(uri);
    }

    pub fn get(&self, uri: &Uri) -> Option<&Document> {
        self.documents.get(uri)
    }

    /// Convert a byte offset to an LSP Position (line, UTF-16 character offset).
    pub fn offset_to_position(rope: &Rope, offset: usize) -> Position {
        let offset = offset.min(rope.len_chars());
        let line = rope.char_to_line(offset);
        let line_start = rope.line_to_char(line);
        let col_char = offset - line_start;

        // Convert char offset within line to UTF-16 code units
        let line_slice = rope.line(line);
        let mut utf16_col: u32 = 0;
        for (i, ch) in line_slice.chars().enumerate() {
            if i >= col_char {
                break;
            }
            utf16_col += ch.len_utf16() as u32;
        }

        Position {
            line: line as u32,
            character: utf16_col,
        }
    }

    /// Convert an LSP Position (line, UTF-16 character offset) to a char offset in the rope.
    pub fn position_to_offset(rope: &Rope, pos: Position) -> usize {
        let line = (pos.line as usize).min(rope.len_lines().saturating_sub(1));
        let line_start = rope.line_to_char(line);
        let line_slice = rope.line(line);

        // Convert UTF-16 code units to char offset
        let mut utf16_count: u32 = 0;
        let mut char_offset = 0;
        for ch in line_slice.chars() {
            if utf16_count >= pos.character {
                break;
            }
            utf16_count += ch.len_utf16() as u32;
            char_offset += 1;
        }

        line_start + char_offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn open_creates_document() {
        let mut store = DocumentStore::new();
        let uri = Uri::from_str("file:///test.kql").unwrap();
        store.open(uri.clone(), 1, "StormEvents | take 10", "kql");

        let doc = store.get(&uri).unwrap();
        assert_eq!(doc.version, 1);
        assert_eq!(doc.rope.to_string(), "StormEvents | take 10");
        assert_eq!(doc._language_id, "kql");
    }

    #[test]
    fn change_full_replaces_content() {
        let mut store = DocumentStore::new();
        let uri = Uri::from_str("file:///test.kql").unwrap();
        store.open(uri.clone(), 1, "old content", "kql");
        store.change_full(&uri, 2, "new content");

        let doc = store.get(&uri).unwrap();
        assert_eq!(doc.version, 2);
        assert_eq!(doc.rope.to_string(), "new content");
    }

    #[test]
    fn close_removes_document() {
        let mut store = DocumentStore::new();
        let uri = Uri::from_str("file:///test.kql").unwrap();
        store.open(uri.clone(), 1, "content", "kql");
        store.close(&uri);

        assert!(store.get(&uri).is_none());
    }

    #[test]
    fn offset_to_position_simple() {
        let rope = Rope::from_str("hello\nworld");
        // 'w' is at char offset 6, line 1, col 0
        let pos = DocumentStore::offset_to_position(&rope, 6);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 0);
    }

    #[test]
    fn offset_to_position_multiline() {
        let rope = Rope::from_str("line1\nline2\nline3");
        // 'l' of "line3" is at char offset 12
        let pos = DocumentStore::offset_to_position(&rope, 12);
        assert_eq!(pos.line, 2);
        assert_eq!(pos.character, 0);

        // '3' of "line3" is at char offset 16
        let pos = DocumentStore::offset_to_position(&rope, 16);
        assert_eq!(pos.line, 2);
        assert_eq!(pos.character, 4);
    }

    #[test]
    fn position_to_offset_simple() {
        let rope = Rope::from_str("hello\nworld");
        let offset = DocumentStore::position_to_offset(&rope, Position { line: 1, character: 0 });
        assert_eq!(offset, 6);
    }

    #[test]
    fn offset_to_position_utf16_non_ascii() {
        // Japanese characters are 1 char but 1 UTF-16 code unit each (BMP)
        let rope = Rope::from_str("日本語");
        let pos = DocumentStore::offset_to_position(&rope, 2);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 2); // Each CJK char = 1 UTF-16 code unit
    }

    #[test]
    fn offset_to_position_utf16_surrogate_pairs() {
        // Emoji like 🎉 requires a surrogate pair (2 UTF-16 code units)
        let rope = Rope::from_str("a🎉b");
        // 'b' is at char index 2
        let pos = DocumentStore::offset_to_position(&rope, 2);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 3); // 'a'=1 + '🎉'=2 UTF-16 code units
    }

    #[test]
    fn position_to_offset_utf16_surrogate_pairs() {
        let rope = Rope::from_str("a🎉b");
        // UTF-16 character 3 = after 'a'(1) + '🎉'(2) = 'b' at char index 2
        let offset = DocumentStore::position_to_offset(&rope, Position { line: 0, character: 3 });
        assert_eq!(offset, 2);
    }

    #[test]
    fn roundtrip_position_conversion() {
        let rope = Rope::from_str("StormEvents\n| where State == '日本語'\n| take 10");
        for offset in 0..rope.len_chars() {
            let pos = DocumentStore::offset_to_position(&rope, offset);
            let back = DocumentStore::position_to_offset(&rope, pos);
            assert_eq!(offset, back, "Roundtrip failed at offset {}", offset);
        }
    }
}
