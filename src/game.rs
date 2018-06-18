use rustc_serialize::json;

use tcod::colors;
use tcod::input;

use std::io::{Read, Write};
use std::fs::File;
use std::error::Error;

use ai;
use consts;
use keys;
use object::{self, actor};
use map::{self, Map, BuildingPlan};
use log;
use log::MessageLog;
use object::Object;
use ui;

#[derive(RustcEncodable, RustcDecodable)]
pub struct Game {
    pub map: BuildingPlan,
    pub log: log::Messages,
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

pub fn save_game(actors: &[Object], game: &Game) -> Result<(), Box<Error>> {
    let save_data = try!{ json::encode(&(actors, game)) };
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

    let mut player = Object::new(4, 4, '@', "player", false, colors::WHITE,
                                 object::Blocks::Full, object::Blocks::No);
    player.alive = true;
    player.fighter = Some(actor::Fighter{
        max_hp: 30, hp: 30, defense: 2, power: 5,
        on_death: actor::DeathCallback::Player,
    });
    player.inventory = Some(vec![]);
    let mut actors = vec![player];
    let mut game = Game {
        map: map::make_map(&mut actors),
        log: vec![],
    };

    ui::initialize_fov(&game.map[0].map, &actors, game_ui);

    game.log.info("Meow!");

    (actors, game)

}

pub fn play_game(actors: &mut Vec<Object>, game: &mut Game, game_ui: &mut ui::Ui) {

    let mut previous_player_position = (-1, -1);
    let mut key = Default::default();


    while !game_ui.root.window_closed() {
        let fov_recompute = previous_player_position !=
            (actors[consts::PLAYER].pos());
        match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
            Some((_, input::Event::Mouse(m))) => game_ui.mouse = m,
            Some((_, input::Event::Key(k))) => key = k,
            _ => key = Default::default(),
        }

        ui::render_all(game_ui, game, &actors, fov_recompute);

        game_ui.root.flush();

        for object in actors.iter_mut() {
            object.clear(&mut game_ui.con)
        }

        previous_player_position = actors[consts::PLAYER].pos();
        let player_action = keys::handle_keys(key, game_ui, game, actors);
        if player_action == PlayerAction::Exit {
            //# TODO: Catch and handle save game errors
            save_game(actors, game).unwrap();
            break
        }

        if actors[consts::PLAYER].alive &&
            player_action != PlayerAction::DidntTakeTurn {
                for id in 0..actors.len() {
                    if actors[id].ai.is_some() {
                        ai::take_turn(id, game, actors, &game_ui.fov);
                    }
                }
            }

    }
}

pub fn next_level(game_ui: &mut ui::Ui, actors: &mut Vec<Object>, game: &mut Game) {
    game.map = map::make_map(actors);
    ui::initialize_fov(&game.map, &actors, game_ui);
}
