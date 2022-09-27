use std::{mem::swap, ops::Index};

use unicode_segmentation::UnicodeSegmentation;

use super::TAB_STOP;

#[derive(Default)]
pub struct Row {
    pub raw: String,    // non-rendered string
    pub render: String, // where tabs and the like are visually represented
    indices: Vec<usize>,
    positions: Vec<usize>,
}

pub type Error = Box<dyn std::error::Error>;
pub type Result<T> = std::result::Result<T, Error>;

impl Row {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn new<S: Into<String>>(line: S) -> Self {
        let mut row = Self {
            raw: line.into(),
            render: String::new(),
            indices: Vec::new(),
            positions: Vec::new(),
        };
        row.do_render().unwrap();
        row
    }

    pub fn len(&self) -> usize {
        if self.indices.is_empty() {
            self.raw.len()
        } else {
            self.indices.len()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn do_render(&mut self) -> Result<()> {
        let mut graphemes_count = 0;
        let mut index: usize = 0;
        let raw_len = self.raw.len();
        self.render.clear();
        self.positions.clear();
        self.positions.reserve(self.raw.len());
        for c in self.raw.graphemes(true) {
            if c == "\t" {
                loop {
                    self.render.push(' ');
                    index += 1;
                    if index % super::TAB_STOP == 0 {
                        break;
                    }
                }
                self.positions.push(TAB_STOP);
            } else {
                index += c.len();
                self.render.push_str(c);
                self.positions.push(1);
            }
            graphemes_count += 1;
        }
        if graphemes_count == self.raw.len() {
            self.indices = Vec::with_capacity(0);
        } else {
            self.indices.clear();
            self.indices.reserve(graphemes_count);
            for (idx, _) in self.raw.grapheme_indices(true) {
                self.indices.push(idx);
            }
        }
        Ok(())
    }

    fn byte_idx_of(&self, char: usize) -> usize {
        let len = self.indices.len();
        if len == 0 {
            char
        } else if len == char {
            self.raw.len()
        } else {
            self.indices[char]
        }
    }

    pub fn visual_distance(&self, mut from: usize, mut to: usize) -> usize {
        let mut sum: usize = 0;
        if to < from {
            swap(&mut from, &mut to);
        }
        for num in &self.positions[from..to] {
            sum += num;
        }
        sum
    }

    pub fn insert_char(&mut self, at: usize, c: char) {
        if at == self.len() {
            self.raw.push(c);
        } else {
            self.raw.insert(self.byte_idx_of(at), c);
        }
        self.do_render().unwrap()
    }

    pub fn remove_char(&mut self, at: usize) -> Result<()> {
        self.raw = self
            .raw
            .grapheme_indices(true)
            .filter(|(index, _)| *index != self.byte_idx_of(at))
            .map(|(_, graphemes)| graphemes)
            .collect();
        self.do_render().unwrap();
        Ok(())
    }
}

impl Index<usize> for Row {
    type Output = str;

    fn index(&self, index: usize) -> &Self::Output {
        self.raw
            .grapheme_indices(true)
            .filter(|(pos, _)| *pos == index)
            .map(|x| x.1)
            .next()
            .unwrap()
    }
}

// impl Default for Row {
//     fn default() -> Self {
//         Self { raw: String::new() , render: String::new() }
//     }
// }
