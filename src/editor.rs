use std::io::{stdout, Write};
use std::{collections::HashMap, default, fs::read, path::PathBuf};

use crossterm::cursor::{MoveTo, self};
use crossterm::event::{self, Event, KeyEvent, KeyModifiers};
use crossterm::style::Print;
use crossterm::{terminal::{*, self}, ExecutableCommand, execute, queue};

use crate::buffer::Buffer;

pub type Error = Box<dyn std::error::Error>;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Default)]
enum OperationMode {
    #[default]
    Command,
    Insert,
    Escape,
}

pub struct Editor {
    wants_out: bool,
    mode: OperationMode,
    buffers: Vec<(String, Buffer)>,
    buffer_index: usize,
    last_line: CommandLine,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            wants_out: false,
            mode: OperationMode::default(),
            buffers: Vec::new(),
            buffer_index: 0,
            last_line: CommandLine { history: Vec::new() }
        }
    }

    pub fn run(&mut self) -> Result<()> {
        while !self.wants_out {
            let current_buffer = &mut self.buffers[self.buffer_index].1;
            let (_, mut rows) = size()?;

            rows -= 1;
            current_buffer.draw_rows(rows.into())?;
            let keypress = self.capture_keypress();
            self.process_keypress(keypress)?;
        }
        Ok(())
    }

    pub fn capture_keypress(&self) -> KeyEvent {
        loop {
            if let Ok(Event::Key(keypress)) = event::read() {
                break keypress;
            }
        }
    }

    pub fn process_keypress(&mut self, keypress: KeyEvent) -> Result<()> {
        let current_buffer = &mut self.buffers[self.buffer_index].1;
        let (_, mut rows) = size()?;
        rows -= 1;
        match &self.mode {
            OperationMode::Command => match keypress.code {
                event::KeyCode::Char('q') => {
                    self.wants_out = true;
                }
                event::KeyCode::Char('j') => {
                    current_buffer.move_down(1, rows.into())?;
                }
                event::KeyCode::Char('k') => {
                    current_buffer.move_up(1)?;
                }
                event::KeyCode::Char('l') => {
                    current_buffer.move_right(1)?;
                }
                event::KeyCode::Char('h') => {
                    current_buffer.move_left(1)?;
                }
                event::KeyCode::Char('i' | 'I') => {
                    if keypress.modifiers == KeyModifiers::SHIFT {
                        current_buffer.move_to_first_char()?;
                    }
                    self.mode = OperationMode::Insert;
                }
                event::KeyCode::Char('a' | 'A') => {
                    if keypress.modifiers == KeyModifiers::SHIFT {
                        current_buffer.move_end_of_line()?;
                        current_buffer.move_right_forced(1)?;
                    } else {
                        current_buffer.move_right_forced(1)?;
                    }
                    self.mode = OperationMode::Insert;
                }
                event::KeyCode::Char('x') => {
                    current_buffer.remove_char()?;
                }
                event::KeyCode::Char('o') => {
                    current_buffer.new_line_after_cursor()?;
                    self.mode = OperationMode::Insert;
                }
                event::KeyCode::Char('O') => {
                    current_buffer.new_line_before_cursor()?;
                    self.mode = OperationMode::Insert;
                }
                event::KeyCode::Char('_') => {
                    current_buffer.move_to_first_char()?;
                }
                event::KeyCode::Char('0') => {
                    current_buffer.move_start_of_line()?;
                }
                event::KeyCode::Char('$') => {
                    current_buffer.move_end_of_line()?;
                }
                event::KeyCode::Char(':') => {
                    self.last_line.get_command();
                    // self.mode = OperationMode::Escape;
                }
                _ => (),
            },
            OperationMode::Insert => match keypress.code {
                event::KeyCode::Esc => {
                    self.mode = OperationMode::default();
                    current_buffer.move_left(1)?;
                }
                event::KeyCode::Char(key) => {
                    current_buffer.insert_char(key)?;
                }
                event::KeyCode::Tab => {
                    current_buffer.insert_char('\t')?;
                }
                event::KeyCode::Backspace => {
                    current_buffer.remove_char_before_cursor()?;
                }
                _ => (),
            },
            OperationMode::Escape => {
                self.last_line.get_command();
                // ();
                self.mode = OperationMode::default();
            }
        }
        let mut stdout = stdout();
        stdout.execute(Clear(ClearType::All))?;
        current_buffer.draw_rows(rows.into())?;
        Ok(())
    }
}

impl From<Vec<String>> for Editor {
    fn from(buffers: Vec<String>) -> Self {
        let mut vector = Vec::new();
        for path in buffers {
            let file = read(&path).unwrap_or_default();
            let buffer = Buffer::new(&file);
            vector.push((path, buffer));
        }
        Editor {
            mode: OperationMode::default(),
            buffers: vector,
            ..Editor::new()
        }
    }
}

struct CommandLine {
    history: Vec<String>,
}

impl CommandLine {
    pub fn get_command(&mut self) -> String {
        let mut stdout = stdout();
        queue!(stdout, cursor::SavePosition).unwrap();
        queue!(stdout, MoveTo(0, 10000)).unwrap();
        queue!(stdout, Print(':')).unwrap();
        stdout.flush().unwrap();
        let mut command = String::new();
        while let Ok(Event::Key(keypress)) = event::read() {
            match keypress.code {
                event::KeyCode::Enter | event::KeyCode::Esc => break,
                event::KeyCode::Backspace => {
                    command.pop();
                }
                event::KeyCode::Char(key) => {
                    command.push(key);
                }
                _ => (),
            }
            queue!(stdout, terminal::Clear(ClearType::CurrentLine)).unwrap();
            queue!(stdout, Print("\r:")).unwrap();
            queue!(stdout, Print(&command)).unwrap();
            stdout.flush().unwrap();
        }
        execute!(stdout, cursor::RestorePosition).unwrap();
        self.history.push(command);
        self.history.last().unwrap().to_string()
    }
}
