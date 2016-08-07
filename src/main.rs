extern crate rand;

extern crate rustc_serialize;

extern crate tcod;

mod ai;
mod consts;
mod game;
mod keys;
mod log;
mod map;
mod object;
mod ui;
mod utils;

fn main() {
    game::start_game();
}
