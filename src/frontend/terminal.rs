use crate::{
    core::{Action, Bounded, Command, Direction, GameState, Location, Minefield, State},
    generator,
};
use std::{
    io::{Stdin, Stdout, Write},
    iter,
};
use termion::{
    clear, color, cursor,
    input::{MouseTerminal, TermRead},
    raw::{IntoRawMode, RawTerminal},
    style,
};

/// The string printed for flagged cells.
const FLAGGED: &'static [u8] = "F".as_bytes();
/// The string printed for mines in the game over revealing.
const MINE: &'static [u8] = "*".as_bytes();
/// The string printed for concealed cells.
const CONCEALED: &'static [u8] = "▒".as_bytes();

/// The game over screen.
const GAME_OVER: &'static [u8] = "╔═════════════════╗\n\r\
                                 ║───┬Game over────║\n\r\
                                 ║ r ┆ replay      ║\n\r\
                                 ║ q ┆ quit        ║\n\r\
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
    stdin: Stdin,
    stdout: MouseTerminal<RawTerminal<Stdout>>,
    mine_field: Option<Minefield>,
}

impl Term {
    pub fn new() -> Self {
        let stdout: MouseTerminal<_> = std::io::stdout().into_raw_mode().unwrap().into();
        Self {
            stdin: std::io::stdin(),
            stdout,
            mine_field: None,
        }
    }

    pub fn init<T, U, V>(&mut self, width: T, height: U, mines: V)
    where
        T: Into<Option<usize>>,
        U: Into<Option<usize>>,
        V: Into<Option<usize>>,
    {
        let termsize = termion::terminal_size().ok();
        let termwidth = termsize.map(|(w, _)| w as usize - 2);
        let termheight = termsize.map(|(_, h)| h as usize - 2);
        let width = width.into().or(termwidth).unwrap_or(70);
        let height = height.into().or(termheight).unwrap_or(40);
        let mines = mines.into().unwrap_or(width * height / 5);

        self.mine_field = generator::simple_generate(mines, width, height).into();
        write!(self.stdout, "{}", clear::All).unwrap();
    }

    pub fn run(&mut self) -> bool {
        if self.mine_field.is_none() {
            return false;
        }

        use termion::event::Event;
        use termion::event::Key::*;
        use termion::event::MouseButton;
        use termion::event::MouseEvent;

        let events: Vec<_> = (&mut self.stdin).events().take(1).collect();
        for e in events.into_iter() {
            match e {
                Ok(Event::Mouse(MouseEvent::Press(btn, x, y))) => {
                    let action = match btn {
                        MouseButton::Left => Action::Reveal,
                        MouseButton::Right => Action::ToggleMark,
                        _ => continue,
                    };
                    let minus2 = |b| b - Bounded::Valid(2);
                    let l = Location::new(x, y).map_x(minus2).map_y(minus2);
                    let cmd = Command::new(l, action);
                    return self.update(cmd);
                }
                Ok(Event::Key(Char('q'))) => return false,
                Err(_) => return false,
                _ => {}
            }
        }

        true
    }

    fn update(&mut self, cmd: Command) -> bool {
        if self.mine_field.is_none() {
            return false;
        }

        let cmds = cmd.apply_recursively(self.mine_field.as_mut().unwrap());
        let update_locations = cmds.iter().map(|l| l.location);
        //cmd.apply(self.mine_field.as_mut().unwrap());

        eprintln!("Applied action.");
        for l in update_locations {
            self.draw_location(l);
        }

        match self.mine_field.as_mut().unwrap().game_state() {
            GameState::Lost => return false,
            GameState::Won => return false,
            GameState::InProgress => {}
        }

        self.print_points();
        self.stdout.flush().unwrap();
        true
    }

    fn draw_location(&mut self, location: Location) {
        let goto = match self.location_to_cursor(location) {
            Some(g) => g,
            None => return,
        };

        write!(self.stdout, "{}", goto).unwrap();
        let mine_count;
        let element = match self.mine_field.as_mut().unwrap().fog.get(location) {
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
        self.stdout.flush().unwrap();
    }

    fn location_to_cursor(&self, location: Location) -> Option<cursor::Goto> {
        let width = self.mine_field.as_ref()?.width();
        let height = self.mine_field.as_ref()?.height();
        location
            .as_tuple()
            .filter(|&(x, y)| x < width && y < height)
            .map(|(x, y)| cursor::Goto(x as u16 + 2, y as u16 + 2))
    }

    fn cursor_to_location(&self, cursor: cursor::Goto) -> Location {
        let cursor::Goto(x, y) = cursor;
        Location::new(x, y)
    }

    /// Print the point count.
    fn print_points(&mut self) {
        let mine_field = self.mine_field.as_mut().unwrap();
        let goto = cursor::Goto(3, mine_field.height() as u16 + 2);
        write!(self.stdout, "{}", goto).unwrap();
        let mc = mine_field.mine_count();
        write!(self.stdout, "{}", mc).unwrap();
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
        write!(self.stdout, "{}", cursor::Goto(1, 1)).unwrap();

        let mine_field = match self.mine_field.as_mut() {
            Some(mf) => mf,
            _ => return,
        };

        use iter::once;
        let cycle_n = |iter, n| iter::repeat(iter).take(n).flatten();
        // generate a single row of the mine field
        let row = |left, middle, right| {
            once(left)
                .chain(iter::repeat(middle).take(mine_field.width()))
                .chain(once(right))
        };

        let top_frame = row(TOP_LEFT_CORNER, HORZ_BOUNDARY, TOP_RIGHT_CORNER).chain(once(NEW_LINE));
        let body_line = row(VERT_BOUNDARY, CONCEALED, VERT_BOUNDARY).chain(once(NEW_LINE));
        let body = cycle_n(body_line, mine_field.height());
        let bottom_frame = row(BOTTOM_LEFT_CORNER, HORZ_BOUNDARY, BOTTOM_RIGHT_CORNER);
        self.write_iter(top_frame.chain(body).chain(bottom_frame));

        // let (x, y) = self.cursor.as_tuple().unwrap();

        // write!(self.stdout, "{}", cursor::Goto(x as u16 + 2, y as u16 + 2)).unwrap();
        self.stdout.flush().unwrap();
    }
}

impl Drop for Term {
    fn drop(&mut self) {
        // When done, restore the defaults to avoid messing with the terminal.
        write!(
            self.stdout,
            "{}{}{}",
            clear::All,
            style::Reset,
            cursor::Goto(1, 1),
        )
        .unwrap();
        self.stdout.flush().unwrap();
    }
}