//
//
// aaa
// äää
// y̆y̆y̆
// ❤❤❤
//

use std::env;
use std::io::{stdout, Write};

use crossterm::cursor;
use crossterm::execute;
use crossterm::terminal::*;
use crossterm::QueueableCommand;

mod buffer;
mod editor;
use editor::Editor;

pub type Error = Box<dyn std::error::Error>;
pub type Result<T> = std::result::Result<T, Error>;

fn main() -> Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let mut stdout = stdout();
    stdout.queue(Clear(ClearType::All))?;
    stdout.queue(cursor::MoveTo(0, 0))?;
    stdout.flush()?;

    let mut editor = Editor::from(env::args().into_iter().skip(1).collect::<Vec<String>>());
    editor.run()?;
    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen)?;
    Ok(())
}
