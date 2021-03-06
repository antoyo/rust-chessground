extern crate gdk;
extern crate gtk;
extern crate chessground;
#[macro_use]
extern crate relm;
#[macro_use]
extern crate relm_derive;

extern crate shakmaty;
extern crate rand;

use rand::seq::SliceRandom;

use gdk::ScrollDirection;
use gtk::prelude::*;
use relm::Widget;
use relm_derive::widget;

use shakmaty::{Square, Role, Move, Chess, Position};
use chessground::{Ground, UserMove, SetPos, Pos, Flip};

use self::Msg::*;

#[derive(Msg)]
pub enum Msg {
    Quit,
    MovePlayed(Square, Square, Option<Role>),
    KeyPressed(u8),
    Scroll(ScrollDirection),
}

#[derive(Default)]
pub struct Model {
    stack: Vec<Move>,
    switchyard: Vec<Move>,
    position: Chess,
}

impl Model {
    fn push(&mut self, m: &Move) {
        self.position.play_unchecked(m);
        self.stack.push(m.clone());
        self.switchyard.clear();
    }

    fn undo(&mut self) {
        self.stack.pop().map(|m| self.switchyard.push(m));
        self.replay();
    }

    fn undo_all(&mut self) {
        while !self.stack.is_empty() {
            self.undo();
        }
    }

    fn redo(&mut self) {
        self.switchyard.pop().map(|m| {
            self.position.play_unchecked(&m);
            self.stack.push(m);
        });
    }

    fn redo_all(&mut self) {
        while !self.switchyard.is_empty() {
            self.redo();
        }
    }

    fn replay(&mut self) {
        // replay
        self.position = Chess::default();
        for m in &self.stack {
            self.position.play_unchecked(m);
        }
    }

    fn pos(&self) -> Pos {
        let mut pos = Pos::new(&self.position);
        pos.set_last_move(self.stack.iter().last());
        pos
    }
}

#[widget]
impl Widget for Win {
    fn model() -> Model {
        Model::default()
    }

    fn update(&mut self, event: Msg) {
        match event {
            Quit => {
                gtk::main_quit()
            },
            MovePlayed(orig, dest, promotion) => {
                let legals = self.model.position.legals();
                let m = legals.iter().find(|m| {
                    m.from() == Some(orig) && m.to() == dest &&
                    m.promotion() == promotion
                });

                if let Some(m) = m {
                    self.model.push(m);
                    self.ground.emit(SetPos(self.model.pos()));
                }
            },
            KeyPressed(b' ') => {
                // play a random move
                let legals = self.model.position.legals();
                if let Some(m) = legals.choose(&mut rand::thread_rng()) {
                    self.model.push(m);
                    self.ground.emit(SetPos(self.model.pos()));
                }
            },
            KeyPressed(b'f') => {
                self.ground.emit(Flip)
            },
            KeyPressed(b'k') | Scroll(ScrollDirection::Up) => {
                self.model.undo();
                self.ground.emit(SetPos(self.model.pos()));
            },
            KeyPressed(b'j') | Scroll(ScrollDirection::Down) => {
                self.model.redo();
                self.ground.emit(SetPos(self.model.pos()));
            },
            KeyPressed(b'h') => {
                self.model.undo_all();
                self.ground.emit(SetPos(self.model.pos()));
            },
            KeyPressed(b'l') => {
                self.model.redo_all();
                self.ground.emit(SetPos(self.model.pos()));
            },
            _ => {},
        }
    }

    view! {
        gtk::Window {
            gtk::Box {
                #[name="ground"]
                Ground {
                    UserMove(orig, dest, promotion) => MovePlayed(orig, dest, promotion),
                    scroll_event(_, e) => (Scroll(e.get_direction()), Inhibit(false)),
                },
            },
            key_press_event(_, e) => (KeyPressed(e.get_keyval() as u8), Inhibit(false)),
            delete_event(_, _) => (Quit, Inhibit(false)),
        }
    }
}

fn main() {
    Win::run(()).expect("initialized gtk");
}
