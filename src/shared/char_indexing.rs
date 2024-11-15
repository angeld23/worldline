use std::ops::Range;

pub trait CharIndexing {
    fn char_to_byte_index(&self, char_index: u32) -> Option<usize>;
    fn char_to_byte_index_open_end(&self, char_index: u32) -> Option<usize>;
    fn char_to_byte_range(&self, char_range: Range<u32>) -> Option<Range<usize>>;
    fn char_to_byte_range_clamped(&self, char_range: Range<u32>) -> Range<usize>;
}

impl CharIndexing for str {
    fn char_to_byte_index(&self, char_index: u32) -> Option<usize> {
        Some(self.char_indices().nth(char_index as usize)?.0)
    }

    fn char_to_byte_index_open_end(&self, char_index: u32) -> Option<usize> {
        let mut n = 0;
        for (byte_index, _) in self.char_indices() {
            if n == char_index {
                return Some(byte_index);
            }
            n += 1;
        }

        if char_index == n {
            return Some(self.len());
        }

        None
    }

    fn char_to_byte_range(&self, char_range: Range<u32>) -> Option<Range<usize>> {
        Some(self.char_to_byte_index(char_range.start)?..self.char_to_byte_index(char_range.end)?)
    }

    fn char_to_byte_range_clamped(&self, char_range: Range<u32>) -> Range<usize> {
        self.char_to_byte_index_open_end(char_range.start)
            .unwrap_or(0)
            ..self
                .char_to_byte_index_open_end(char_range.end)
                .unwrap_or(self.len())
    }
}
