use crate::{
    core::{Action, Bounded, GameState, Location, Minefield, PendingCommand, State},
    generator,
};
use std::{
    borrow::Cow,
    io::{Stdout, Write},
    iter,
};
use termion::{
    clear, cursor,
    input::{MouseTerminal, TermRead},
    raw::{IntoRawMode, RawTerminal},
    style, AsyncReader,
};

/// The string printed for flagged cells.
const FLAGGED: &'static [u8] = "F".as_bytes();
/// The string printed for mines in the game over revealing.
const MINE: &'static [u8] = "*".as_bytes();
/// The string printed for concealed cells.
const CONCEALED: &'static [u8] = "▒".as_bytes();

/// The game over screen.
const GAME_OVER: &'static [u8] = "╔═════════════════╗\n\r\
                                  ║───┬Game over──║\n\r\
                                  ║ r ┆ replay   ║\n\r\
                                  ║ q ┆ quit     ║\n\r\
                                  ╚═══╧═════════════╝"
    .as_bytes();

/// The upper and lower boundary char.
const HORZ_BOUNDARY: &'static [u8] = "─".as_bytes();
/// The left and right boundary char.
const VERT_BOUNDARY: &'static [u8] = "│".as_bytes();

/// The top-left corner
const TOP_LEFT_CORNER: &'static [u8] = "┌".as_bytes();
/// The top-right corner
const TOP_RIGHT_CORNER: &'static [u8] = "┐".as_bytes();
/// The bottom-left corner
const BOTTOM_LEFT_CORNER: &'static [u8] = "└".as_bytes();
/// The bottom-right corner
const BOTTOM_RIGHT_CORNER: &'static [u8] = "┘".as_bytes();
const NEW_LINE: &'static [u8] = "\n\r".as_bytes();

pub struct Term {
    stdin: AsyncReader,
    stdout: MouseTerminal<RawTerminal<Stdout>>,
    mine_field: Minefield,
}

impl Term {
    pub fn new<T, U, V>((width, height): (T, U), mines: V) -> Self
    where
        T: Into<Option<usize>>,
        U: Into<Option<usize>>,
        V: Into<Option<usize>>,
    {
        let termsize = termion::terminal_size()
            .ok()
            .map(|(w, h)| (w as usize - 2, h as usize - 5));
        let size = width.into().zip(height.into());
        let (width, height) = size.or(termsize).unwrap_or((70, 40));
        let mines = mines.into().unwrap_or(width * height / 6);

        Self {
            stdin: termion::async_stdin(),
            stdout: std::io::stdout().into_raw_mode().unwrap().into(),
            mine_field: generator::simple_generate(mines, width, height).into(),
        }
    }

    pub fn go(&mut self) {
        eprintln!("start");
        while self.run() {
            self.stdout.flush().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    pub fn run(&mut self) -> bool {
        self.print_info();

        use termion::event::{Event, Key::*, MouseButton, MouseEvent};

        match (&mut self.stdin).events().next().transpose().ok().flatten() {
            Some(Event::Mouse(MouseEvent::Press(btn, x, y))) => {
                let action = match btn {
                    MouseButton::Left => Action::Reveal,
                    MouseButton::Right => Action::ToggleMark,
                    _ => return true,
                };
                let minus2 = |b| b - Bounded::Valid(2);
                let l = Location::new(x, y).map_x(minus2).map_y(minus2);
                let cmd = PendingCommand::new(l, action);
                self.update_mine_field(cmd)
            }
            Some(Event::Key(Char('q'))) => false,
            _ => true,
        }
    }

    fn update_mine_field(&mut self, cmd: PendingCommand) -> bool {
        let cmd = match self.mine_field.execute(cmd) {
            Some(cmd) => cmd,
            None => return true,
        };

        eprintln!("Applied action.");
        for l in cmd.updated_locations {
            self.draw_location(l);
        }
        true
    }

    fn draw_location(&mut self, location: Location) {
        let goto = match self.location_to_cursor(location) {
            Some(g) => g,
            None => return,
        };

        write!(self.stdout, "{}", goto).unwrap();
        let mine_count;
        let element = match self.mine_field.fog.get(location) {
            Some(State::Hidden) => CONCEALED,
            Some(State::Exploded) => MINE,
            Some(State::Revealed { adj_mines }) => {
                mine_count = adj_mines.to_string();
                mine_count.as_bytes()
            }
            Some(State::Marked) => FLAGGED,
            None => return,
        };

        self.write(element);
    }

    fn location_to_cursor(&self, location: Location) -> Option<cursor::Goto> {
        let width = self.mine_field.width();
        let height = self.mine_field.height();
        location
            .as_tuple()
            .filter(|&(x, y)| x < width && y < height)
            .map(|(x, y)| cursor::Goto(x as u16 + 2, y as u16 + 2))
    }

    fn print_info(&mut self) {
        let total_mines = self.mine_field.mine_count();
        let marked_mines = self.mine_field.mark_count();
        use GameState::*;
        let status: Cow<_> = match self.mine_field.state() {
            Initial => "Ready to go.".into(),
            Win { game_duration } => format!("VICTORY! ({} sec)", game_duration.as_secs()).into(),
            Loss { game_duration } => format!("DEFEAT. ({} secs)", game_duration.as_secs()).into(),
            InProgress { start_time } => {
                format!("Time: {} seconds", start_time.elapsed().as_secs()).into()
            }
        };
        let goto = cursor::Goto(3, self.mine_field.height() as u16 + 3);
        write!(
            self.stdout,
            "{}Mines: {:>3}/{:>3}, {}",
            goto, marked_mines, total_mines, status
        )
        .unwrap();
    }

    fn write(&mut self, data: &[u8]) {
        self.stdout.write_all(data).unwrap();
    }

    fn write_iter<'a>(&'a mut self, data: impl Iterator<Item = &'a [u8]>) {
        for element in data {
            self.write(element);
        }
    }

    pub fn reset(&mut self) {
        // Reset the cursor.
        write!(
            self.stdout,
            "{}{}{}",
            clear::All,
            cursor::Hide,
            cursor::Goto(1, 1)
        )
        .unwrap();

        use iter::once;
        let cycle_n = |iter, n| iter::repeat(iter).take(n).flatten();
        // generate a single row of the mine field
        let row = |left, middle, right| {
            once(left)
                .chain(iter::repeat(middle).take(self.mine_field.width()))
                .chain(once(right))
        };

        let top_frame = row(TOP_LEFT_CORNER, HORZ_BOUNDARY, TOP_RIGHT_CORNER).chain(once(NEW_LINE));
        let body_line = row(VERT_BOUNDARY, CONCEALED, VERT_BOUNDARY).chain(once(NEW_LINE));
        let body = cycle_n(body_line, self.mine_field.height());
        let bottom_frame = row(BOTTOM_LEFT_CORNER, HORZ_BOUNDARY, BOTTOM_RIGHT_CORNER);
        self.write_iter(top_frame.chain(body).chain(bottom_frame));

        self.print_info();
        self.stdout.flush().unwrap();
    }
}

impl Drop for Term {
    fn drop(&mut self) {
        // When done, restore the defaults to avoid messing with the terminal.
        write!(
            self.stdout,
            "{}{}{}{}",
            clear::All,
            style::Reset,
            cursor::Goto(1, 1),
            cursor::Show,
        )
        .unwrap();
        self.stdout.flush().unwrap();
    }
}
