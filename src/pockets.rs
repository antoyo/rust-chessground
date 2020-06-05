use std::f64::consts::PI;

use cairo::{Context, Rectangle};
use gdk::EventButton;
use shakmaty::{Color, Material, Piece, Role, Square};
use time::SteadyTime;

use boardstate::BoardState;
use ground::{EventContext, GroundMsg};
use pieces::{Drag, Figurine};
use util::pos_to_square;

pub struct Pockets {
    drag: Option<Drag>,
    figurines: Vec<Figurine>,
}

impl Pockets {
    pub fn new() -> Self {
        Self {
            drag: None,
            figurines: vec![],
        }
    }

    pub fn draw(&self, cr: &Context, state: &BoardState) {
        for figurine in &self.figurines {
            self.draw_figurine(cr, figurine, state);
        }
    }

    fn draw_figurine(&self, cr: &Context, figurine: &Figurine, state: &BoardState) {
        // draw ghost when dragging
        let dragging =
            figurine.dragging &&
            self.drag.as_ref().map_or(false, |d| d.threshold && d.square == figurine.square);

        cr.push_group();

        let (x, y) = figurine.pos();
        cr.translate(x, y);
        cr.rotate(state.orientation().fold(0.0, PI));
        cr.translate(-0.5, -0.5);
        cr.scale(state.piece_set().scale(), state.piece_set().scale());

        let renderer = librsvg::CairoRenderer::new(state.piece_set().by_piece(&figurine.piece));
        renderer.render_document(cr, &Rectangle {
            x: 0.0,
            y: 0.0,
            width: 177.0,
            height: 177.0,
        }).expect("render");

        cr.pop_group_to_source();

        cr.paint_with_alpha(if dragging { 0.2 } else { figurine.alpha() });
    }

    pub(crate) fn draw_drag(&self, cr: &Context, state: &BoardState) {
        match self.drag {
            Some(ref drag) if drag.threshold => {
                cr.push_group();
                cr.translate(drag.pos.0, drag.pos.1);
                cr.rotate(state.orientation().fold(0.0, PI));
                cr.translate(-0.5, -0.5);
                cr.scale(state.piece_set().scale(), state.piece_set().scale());
                let renderer = librsvg::CairoRenderer::new(state.piece_set().by_piece(&drag.piece));
                renderer.render_document(cr, &Rectangle {
                    x: 0.0,
                    y: 0.0,
                    width: 177.0,
                    height: 177.0,
                }).expect("render");
                cr.pop_group_to_source();
                cr.paint();
            }
            _ => {}
        }
    }

    pub fn set_pockets(&mut self, material: Material, turn: Color) {
        self.figurines.clear();

        let now = SteadyTime::now();
        let (color, material) =
            match turn {
                Color::White => (Color::White, &material.white),
                Color::Black => (Color::Black, &material.black),
            };
        let mut y = 7.5;
        if material.pawns > 0 {
            self.figurines.push(new_figurine(Piece {
                color,
                role: Role::Pawn,
            }, y, now));
            y -= 1.0;
        }
        if material.knights > 0 {
            self.figurines.push(new_figurine(Piece {
                color,
                role: Role::Knight,
            }, y, now));
            y -= 1.0;
        }
        if material.bishops > 0 {
            self.figurines.push(new_figurine(Piece {
                color,
                role: Role::Bishop,
            }, y, now));
            y -= 1.0;
        }
        if material.rooks > 0 {
            self.figurines.push(new_figurine(Piece {
                color,
                role: Role::Rook,
            }, y, now));
            y -= 1.0;
        }
        if material.queens > 0 {
            self.figurines.push(new_figurine(Piece {
                color,
                role: Role::Queen,
            }, y, now));
        }
    }

    pub(crate) fn drag_mouse_down(&mut self, ctx: &EventContext, e: &EventButton) {
        if e.get_button() == 1 {
            if let Some(pocket) = ctx.pocket() {
                let piece = if let Some(figurine) = self.figurines.get_mut(pocket) {
                    figurine.dragging = true;
                    figurine.piece
                } else {
                    return;
                };

                self.drag = Some(Drag {
                    square: Square::A1, // Dummy square.
                    piece,
                    start: ctx.pos(),
                    pos: ctx.pos(),
                    threshold: false,
                });
            }
        }
    }

    pub(crate) fn drag_mouse_move(&mut self, ctx: &EventContext) {
        if let Some(ref mut drag) = self.drag {
            ctx.widget().queue_draw_rect(drag.pos.0 - 0.5, drag.pos.1 - 0.5, 1.0, 1.0);
            if let Some(sq) = pos_to_square(drag.pos) {
                ctx.widget().queue_draw_square(sq);
            }
            drag.pos = ctx.pos();
            ctx.widget().queue_draw_rect(drag.pos.0 - 0.5, drag.pos.1 - 0.5, 1.0, 1.0);
            if let Some(sq) = pos_to_square(drag.pos) {
                ctx.widget().queue_draw_square(sq);
            }

            let (dx, dy) = (drag.start.0 - drag.pos.0, drag.start.1 - drag.pos.1);
            let (pdx, pdy) = ctx.widget().matrix().transform_distance(dx, dy);
            drag.threshold |= dx.hypot(dy) >= 0.1 || pdx.hypot(pdy) >= 4.0;

            if drag.threshold {
                ctx.widget().queue_draw_square(drag.square);
            }
        }
    }

    pub(crate) fn drag_mouse_up(&mut self, ctx: &EventContext) {
        let (piece, dest) =
            if let Some(drag) = self.drag.take() {
                ctx.widget().queue_draw();

                if let Some(ref mut figurine) = self.dragging_mut() {
                    figurine.last_drag = SteadyTime::now();
                    figurine.dragging = false;
                }

                let dest = ctx.square().unwrap_or(drag.square);

                // TODO: check if dropping in the board.
                (drag.piece, dest)
            }
            else {
                return;
            };

        ctx.stream().emit(GroundMsg::UserDrop(piece, dest));
    }

    pub fn dragging_mut(&mut self) -> Option<&mut Figurine> {
        self.figurines.iter_mut().find(|f| f.dragging)
    }
}

fn new_figurine(piece: Piece, y: f64, now: SteadyTime) -> Figurine {
    Figurine {
        square: Square::A1, // Dummy square.
        piece,
        start: (9.0, y),
        elapsed: 0.0,
        time: now,
        last_drag: now,
        fading: false,
        replaced: false,
        dragging: false,
    }
}
