use crate::core::{
    Action, ExecutionResult, GameState, Location, Minefield, Parameters, PendingCommand, State,
};

use std::{
    borrow::Cow,
    collections::HashSet,
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
const FLAGGED: &[u8] = "F".as_bytes();
/// The string printed for mines in the game over revealing.
const MINE: &[u8] = "*".as_bytes();
/// The string printed for concealed cells.
const CONCEALED: &[u8] = "▒".as_bytes();

/// The upper and lower boundary char.
const HORZ_BOUNDARY: &[u8] = "─".as_bytes();
/// The left and right boundary char.
const VERT_BOUNDARY: &[u8] = "│".as_bytes();

/// The top-left corner
const TOP_LEFT_CORNER: &[u8] = "┌".as_bytes();
/// The top-right corner
const TOP_RIGHT_CORNER: &[u8] = "┐".as_bytes();
/// The bottom-left corner
const BOTTOM_LEFT_CORNER: &[u8] = "└".as_bytes();
/// The bottom-right corner
const BOTTOM_RIGHT_CORNER: &[u8] = "┘".as_bytes();
const NEW_LINE: &[u8] = "\n\r".as_bytes();

enum InputEvent {
    None,
    Quit,
    Restart,
    GameAction(Action, Location),
}

pub struct Term {
    io: TermIo,
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
            io: TermIo::new(width, height),
            mine_field: Minefield::new(Parameters::new(width, height, mines)),
        }
    }

    pub fn go(&mut self) {
        eprintln!("start");
        while self.run() {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    pub fn run(&mut self) -> bool {
        self.io.print_info(&self.mine_field);
        match self.mine_field.state() {
            GameState::Initial { .. } => self.run_initial(),
            GameState::InProgress { .. } => self.run_in_progress(),
            GameState::Loss { .. } | GameState::Win { .. } => self.run_after(),
        }
    }

    pub fn run_initial(&mut self) -> bool {
        match self.io.read_input() {
            InputEvent::GameAction(action, l) => {
                self.execute_action(l, action);
                true
            }
            InputEvent::Quit => false,
            _ => true,
        }
    }

    pub fn run_after(&mut self) -> bool {
        match self.io.read_input() {
            InputEvent::Quit => false,
            InputEvent::Restart => {
                self.mine_field.reset();
                self.io.reset();
                true
            }
            _ => true,
        }
    }

    pub fn run_in_progress(&mut self) -> bool {
        match self.io.read_input() {
            InputEvent::GameAction(action, l) => {
                self.execute_action(l, action);
                true
            }
            InputEvent::Quit => false,
            _ => true,
        }
    }

    fn execute_action(&mut self, l: Location, action: Action) {
        let commands = match self.lookup(l).map(State::is_revealed) {
            Some(true) => self.reveal_neighbours(l),
            Some(false) => vec![PendingCommand::new(l, action)],
            None => vec![],
        };
        let affected_locations: HashSet<_> = commands
            .into_iter()
            .flat_map(|pending| {
                use ExecutionResult::*;
                match self.mine_field.execute(pending) {
                    SuccessAndStateChange(done) | SuccessNoStateChange(done) => {
                        eprintln!("Applied action.");
                        done.updated_locations
                    }
                    Failed => vec![],
                }
            })
            .collect();

        if self.mine_field.state().is_loss() {
            self.mine_field.reveal_all();
            self.redraw(Location::generate_all(
                self.mine_field.width(),
                self.mine_field.height(),
            ));
        } else {
            self.redraw(affected_locations);
        }
    }

    fn lookup(&self, l: Location) -> Option<&State> {
        self.mine_field.fog().get(l)
    }

    fn reveal_neighbours(&self, l: Location) -> Vec<PendingCommand> {
        eprintln!("Trying to reveal all neighbours.");
        let expected = match self.lookup(l) {
            Some(&State::Revealed { adj_mines }) => adj_mines,
            _ => return vec![],
        };
        let actual = l
            .neighbours()
            .filter_map(|l| self.lookup(l))
            .filter(|s| s.is_marked())
            .count();

        eprintln!("Expected: {}, Actual: {}", expected, actual);

        if expected != actual {
            eprintln!("Not all mines marked.");
            return vec![];
        }
        eprintln!("Trying to reveal all neighbours.");

        l.neighbours()
            .filter(|&l| self.lookup(l).map(State::is_hidden).unwrap_or(false))
            .map(|l| PendingCommand::new(l, Action::Reveal))
            .collect()
    }

    fn redraw<I: IntoIterator<Item = Location>>(&mut self, locations: I) {
        let Self { io, mine_field, .. } = self;
        let location_states = locations
            .into_iter()
            .filter_map(|l| Some((l, mine_field.fog().get(l)?)));
        io.draw_many(location_states);
    }
}

struct TermIo {
    stdin: AsyncReader,
    stdout: MouseTerminal<RawTerminal<Stdout>>,
    width: usize,
    height: usize,
}

impl TermIo {
    pub fn new(width: usize, height: usize) -> Self {
        let mut s = Self {
            stdin: termion::async_stdin(),
            stdout: std::io::stdout().into_raw_mode().unwrap().into(),
            width,
            height,
        };
        s.reset();
        s
    }

    pub fn read_input(&mut self) -> InputEvent {
        use termion::event::{
            Event::{Key, Mouse},
            Key::*,
            MouseButton::{Left, Right},
            MouseEvent::Press,
        };

        let mouse2loc = |x, y| Location::new(x, y).x_minus(2u16).y_minus(2u16);
        let game_action = |a, x, y| InputEvent::GameAction(a, mouse2loc(x, y));

        match (&mut self.stdin).events().next().transpose().ok().flatten() {
            Some(Mouse(Press(Right, x, y))) => game_action(Action::ToggleMark, x, y),
            Some(Mouse(Press(Left, x, y))) => game_action(Action::Reveal, x, y),
            Some(Key(Char('q'))) => InputEvent::Quit,
            Some(Key(Char('r'))) => InputEvent::Restart,
            _ => InputEvent::None,
        }
    }

    pub fn draw_many<'a>(&mut self, fields: impl Iterator<Item = (Location, &'a State)>) {
        for (l, s) in fields {
            self.draw(l, s);
        }
        self.stdout.flush().unwrap();
    }

    pub fn draw_single(&mut self, location: Location, state: &State) {
        self.draw(location, state);
        self.stdout.flush().unwrap();
    }

    fn draw(&mut self, location: Location, state: &State) {
        let goto = match self.location_to_cursor(location) {
            Some(g) => g,
            None => return,
        };
        use termion::color::{Fg, Red, Reset};

        write!(self.stdout, "{}", goto).unwrap();
        let element: Cow<_> = match state {
            State::Hidden => CONCEALED.into(),
            State::Exploded => MINE.into(),
            State::Revealed { adj_mines } => adj_mines.to_string().as_bytes().to_vec().into(),
            State::Marked => format!(
                "{}{}{}",
                Fg(Red),
                std::str::from_utf8(FLAGGED).unwrap(),
                Fg(Reset)
            )
            .as_bytes()
            .to_vec()
            .into(),
        };

        self.write(&element);
    }

    fn location_to_cursor(&self, location: Location) -> Option<cursor::Goto> {
        let &Self { width, height, .. } = self;
        location
            .as_tuple()
            .filter(|&(x, y)| x < width && y < height)
            .map(|(x, y)| cursor::Goto(x as u16 + 2, y as u16 + 2))
    }

    pub fn print_info(&mut self, mf: &Minefield) {
        let total_mines = mf.mine_count();
        let marked_mines = mf.mark_count();
        use GameState::*;
        let status: Cow<_> = match mf.state() {
            Initial { .. } => "Ready to go.".into(),
            Win { game_duration } => format!("VICTORY! ({} sec)", game_duration.as_secs()).into(),
            Loss { game_duration } => format!("DEFEAT. ({} secs)", game_duration.as_secs()).into(),
            InProgress { start_time } => {
                format!("Time: {} seconds", start_time.elapsed().as_secs()).into()
            }
        };
        let goto = cursor::Goto(3, self.height as u16 + 3);
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
                .chain(iter::repeat(middle).take(self.width))
                .chain(once(right))
        };

        let top_frame = row(TOP_LEFT_CORNER, HORZ_BOUNDARY, TOP_RIGHT_CORNER).chain(once(NEW_LINE));
        let body_line = row(VERT_BOUNDARY, CONCEALED, VERT_BOUNDARY).chain(once(NEW_LINE));
        let body = cycle_n(body_line, self.height);
        let bottom_frame = row(BOTTOM_LEFT_CORNER, HORZ_BOUNDARY, BOTTOM_RIGHT_CORNER);
        self.write_iter(top_frame.chain(body).chain(bottom_frame));

        self.stdout.flush().unwrap();
    }
}

impl Drop for TermIo {
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
