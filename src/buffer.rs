use crossterm::{cursor, terminal::size, ExecutableCommand, QueueableCommand};
use std::{
    convert::TryInto,
    io::{stdout, BufRead, Write},
};
use unicode_segmentation::UnicodeSegmentation;
mod row;
use row::Row;

pub type Error = Box<dyn std::error::Error>;
pub type Result<T> = std::result::Result<T, Error>;

const TAB_STOP: usize = 8;

struct Cursor {
    previous_x: Option<usize>,
    x: usize,
    y: usize,
}

impl Cursor {
    fn preserve_x(&mut self, line_length: usize) -> Result<()> {
        let mut stdout = stdout();
        if self
            .previous_x
            .filter(|value| value <= &line_length)
            .is_some()
        {
            self.x = self.previous_x.take().unwrap();
            stdout.queue(cursor::MoveTo(
                self.x.try_into().unwrap(),
                self.y.try_into().unwrap(),
            ))?;
        } else if self.x < line_length {
            if self.previous_x.is_some() {
                self.x = line_length;
                stdout.queue(cursor::MoveTo(
                    self.x.try_into().unwrap(),
                    self.y.try_into().unwrap(),
                ))?;
            }
        } else if self.x > line_length {
            if self.previous_x.is_none() {
                self.previous_x = Some(self.x);
            }
            self.x = line_length;
            stdout.queue(cursor::MoveTo(
                self.x.try_into().unwrap(),
                self.y.try_into().unwrap(),
            ))?;
        }
        Ok(())
    }
}

pub struct Buffer {
    contents: Vec<Row>,
    offset: usize,
    cursor: Cursor,
}

impl Buffer {
    pub fn new(buf: &[u8]) -> Self {
        let mut contents = Vec::new();
        if buf.is_empty() {
            contents.push(Row::empty());
        } else {
            for line in buf.lines() {
                let row = Row::new(line.unwrap());
                contents.push(row);
            }
        }
        Self {
            contents,
            offset: 0,
            cursor: Cursor {
                previous_x: None,
                x: 0,
                y: 0,
            },
        }
    }

    pub fn draw_rows(&mut self, screen_rows: usize) -> Result<()> {
        let mut stdout = stdout();
        stdout.queue(cursor::SavePosition)?;
        stdout.queue(cursor::MoveTo(0, 0))?;
        let mut contents_iter = self.contents.iter().skip(self.offset);
        let mut count = 0;
        while count != screen_rows {
            if let Some(line) = contents_iter.next() {
                stdout.queue(crossterm::style::Print(&line.render))?;
                stdout.queue(cursor::MoveToNextLine(1))?;
            } else {
                stdout.queue(crossterm::style::Print('~'))?;
                stdout.queue(cursor::MoveToNextLine(1))?;
            }
            count += 1;
        }
        stdout.queue(cursor::RestorePosition)?;
        stdout.flush()?;
        Ok(())
    }

    pub fn move_down(&mut self, count: u16, screen_lines: usize) -> Result<()> {
        if self.cursor.y + self.offset >= self.contents.len().saturating_sub(1) {
            return Ok(());
        }
        let mut stdout = stdout();
        if self.cursor.y + usize::from(count) < screen_lines {
            stdout.queue(cursor::MoveDown(count))?;
            self.cursor.y += usize::from(count);
        } else {
            self.offset += usize::from(count);
        }
        let line_length = self.current_line().len().saturating_sub(1);
        self.cursor.preserve_x(line_length)?;
        stdout.flush()?;
        Ok(())
    }

    pub fn move_up(&mut self, count: u16) -> Result<()> {
        if self.cursor.y == 0 && self.offset == 0 {
            return Ok(());
        }
        let mut stdout = stdout();
        if self.cursor.y.checked_sub(count.into()) != None {
            stdout.execute(cursor::MoveUp(count))?;
            self.cursor.y -= usize::from(count);
        } else {
            self.offset -= usize::from(count);
        }
        let line_length = self.current_line().len().saturating_sub(1);
        self.cursor.preserve_x(line_length)?;
        Ok(())
    }

    pub fn move_right(&mut self, count: u16) -> Result<()> {
        let line = self.contents.get(self.cursor.y + self.offset).unwrap();
        let line_length = line.len().saturating_sub(1);
        if self.cursor.x >= line_length || line_length == 0 {
            return Ok(());
        }
        if self.cursor.previous_x.is_some() {
            self.cursor.previous_x = None;
        }
        let mut final_position = self.cursor.x + usize::from(count);
        if final_position > line_length {
            final_position = line_length;
        }
        let distance = line.visual_distance(self.cursor.x, final_position);
        stdout().execute(cursor::MoveRight(distance.try_into().unwrap()))?;
        self.cursor.x = final_position;
        Ok(())
    }

    fn current_line(&self) -> &Row {
        self.contents.get(self.cursor.y + self.offset).unwrap()
    }

    pub fn move_right_forced(&mut self, count: u16) -> Result<()> {
        if self.current_line().is_empty() {
            return Ok(());
        }
        stdout().execute(cursor::MoveRight(count))?;
        self.cursor.x += usize::from(count);
        Ok(())
    }

    pub fn move_left(&mut self, count: u16) -> Result<()> {
        if self.cursor.x == 0 {
            return Ok(());
        }
        if self.cursor.previous_x.is_some() {
            self.cursor.previous_x = None;
        }
        let line = self.current_line();
        let mut final_position = self.cursor.x.saturating_sub(usize::from(count));
        let distance = line.visual_distance(final_position, self.cursor.x);
        stdout().execute(cursor::MoveLeft(distance.try_into().unwrap()))?;
        self.cursor.x = final_position;
        Ok(())
    }

    pub fn move_end_of_line(&mut self) -> Result<()> {
        self.move_right(self.current_line().len().try_into().unwrap())?;
        Ok(())
    }

    pub fn move_start_of_line(&mut self) -> Result<()> {
        self.move_left(self.current_line().len().try_into().unwrap())?;
        Ok(())
    }

    pub fn move_to_first_char(&mut self) -> Result<()> {
        self.cursor.x = self.current_line().raw.chars().position(|x| !x.is_whitespace()).unwrap_or(0);
        stdout().execute(cursor::MoveTo(
            self.cursor.x.try_into().unwrap(),
            self.cursor.y.try_into().unwrap(),
        ))?;
        Ok(())
    }

    pub fn insert_char(&mut self, character: char) -> Result<()> {
        let line = self.contents.get_mut(self.cursor.y + self.offset).unwrap();
        line.insert_char(self.cursor.x, character);
        self.move_right_forced(1)?;
        Ok(())
    }

    pub fn remove_char(&mut self) -> Result<()> {
        let line = self.contents.get_mut(self.cursor.y + self.offset).unwrap();
        if line.is_empty() {
            return Ok(());
        }
        line.remove_char(self.cursor.x)?;
        if self.cursor.x >= line.len() {
            self.move_left(1)?;
        }
        Ok(())
    }

    pub fn remove_char_before_cursor(&mut self) -> Result<()> {
        if self.cursor.x == 0 {
            return Ok(());
        }
        self.move_left(1)?;
        let line = self.contents.get_mut(self.cursor.y + self.offset).unwrap();
        line.remove_char(self.cursor.x)?;
        Ok(())
    }

    pub fn new_line_after_cursor(&mut self) -> Result<()> {
        let (_, mut rows) = size()?;
        rows -= 1;
        self.contents.insert(self.cursor.y + self.offset + 1, Row::empty());
        self.move_down(1, rows.into())?;
        Ok(())
    }

    pub fn new_line_before_cursor(&mut self) -> Result<()> {
        self.move_start_of_line()?;
        self.contents.insert(self.cursor.y + self.offset, Row::empty());
        Ok(())
    }
    //
    // pub fn remove_char_leftwards(&mut self) -> Result<()> {
    //     if self.cursor.x == 0 {
    //         return Ok(());
    //     }
    //     self.move_left(1)?;
    //     let mut result = String::new();
    //     let line = self.contents.get_mut(self.cursor.y + self.offset).unwrap();
    //     for (index, grapheme) in line.graphemes(true).enumerate() {
    //         if self.cursor.x != index {
    //             result.push_str(grapheme);
    //         }
    //     }
    //     *line = result;
    //     Ok(())
    // }
    //
    // pub fn delete_line(&mut self) -> Result<()> {
    //     if self.contents.len() == 1 {
    //         *self.contents.get_mut(0).unwrap() = String::from(" ");
    //         return Ok(())
    //     }
    //     self.contents.remove(self.cursor.y + self.offset);
    //     if self.contents.get(self.cursor.y + self.offset).is_none() {
    //         self.move_up(1)?;
    //         if self.contents.get(self.cursor.y + self.offset).is_some() {
    //             self.move_to_first_char()?;
    //         }
    //     }
    //     Ok(())
    // }
    //
    //
}
