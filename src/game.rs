use rustc_serialize::json;

use tcod::colors;
use tcod::input;

use std::io::{Read, Write};
use std::fs::File;
use std::error::Error;

use ai;
use consts;
use keys;
use object::actor;
use map::{self, Map};
use log;
use log::MessageLog;
use object::Object;
use ui;

#[derive(RustcEncodable, RustcDecodable)]
pub struct Game {
    pub map: Map,
    pub log: log::Messages,
    pub inventory: Vec<Object>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

pub fn start_game() {
    let mut game_ui: ui::Ui = ui::initialize("RustWorld");

    ui::main_menu(&mut game_ui);
}

pub fn save_game(objects: &[Object], game: &Game) -> Result<(), Box<Error>> {
    let save_data = try!{ json::encode(&(objects, game)) };
    let mut file = try!{ File::create("savegame") };
    try!{ file.write_all(save_data.as_bytes()) };
    Ok(())
}

pub fn load_game() -> Result<(Vec<Object>, Game), Box<Error>> {
    let mut json_save_state = String::new();
    let mut file = try! { File::open("savegame") };
    try! { file.read_to_string(&mut json_save_state) };
    let result = try! { json::decode::<(Vec<Object>, Game)>(&json_save_state) };
    Ok(result)
}

pub fn new_game(game_ui: &mut ui::Ui) -> (Vec<Object>, Game) {
    let mut player = Object::new(0, 0, '@', "player", colors::WHITE, true);
    player.alive = true;
    player.fighter = Some(actor::Fighter{
        max_hp: 30, hp: 30, defense: 2, power: 5,
        on_death: actor::DeathCallback::Player,
    });
    let mut objects = vec![player];
    let mut game = Game {
        map: map::make_map(&mut objects),
        log: vec![],
        inventory: vec![],
    };

    ui::initialize_fov(&game.map, game_ui);

    game.log.info("Meow!");

    (objects, game)

}

pub fn play_game(objects: &mut Vec<Object>, game: &mut Game, game_ui: &mut ui::Ui) {

    let mut previous_player_position = (-1, -1);
    let mut key = Default::default();

    while !game_ui.root.window_closed() {
        let fov_recompute = previous_player_position !=
            (objects[consts::PLAYER].pos());
        match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
            Some((_, input::Event::Mouse(m))) => game_ui.mouse = m,
            Some((_, input::Event::Key(k))) => key = k,
            _ => key = Default::default(),
        }

        ui::render_all(game_ui, game, &objects, fov_recompute);

        game_ui.root.flush();

        for object in objects.iter_mut() {
            object.clear(&mut game_ui.con)
        }

        previous_player_position = objects[consts::PLAYER].pos();
        let player_action = keys::handle_keys(key, game_ui, game, objects);
        if player_action == PlayerAction::Exit {
            //# TODO: Catch and handle save game errors
            save_game(objects, game).unwrap();
            break
        }

        if objects[consts::PLAYER].alive &&
            player_action != PlayerAction::DidntTakeTurn {
                for id in 0..objects.len() {
                    if objects[id].ai.is_some() {
                        ai::ai_take_turn(id, game, objects, &game_ui.fov);
                    }
                }
            }

    }
}

pub fn next_level(game_ui: &mut ui::Ui, objects: &mut Vec<Object>, game: &mut Game) {
    game.map = map::make_map(objects);
    ui::initialize_fov(&game.map, game_ui);
}
