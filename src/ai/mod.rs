extern crate rustc_serialize;

extern crate rand;
use rand::Rng;

extern crate tcod;
use tcod::map::Map as FovMap;
use tcod::colors;

use log::MessageLog;
use map::Map;
use object::Object;
use object::actor;

use consts;
use utils;

use game::Game;

#[derive(Debug, RustcEncodable, RustcDecodable)]
pub enum Ai {
    Basic,
    Chrysalis,
    Confused{previous_ai: Box<Ai>, num_turns: i32},
}

pub fn ai_take_turn(monster_id: usize, game: &mut Game, objects: &mut [Object],
                fov_map: &FovMap) {
    if let Some(ai) = objects[monster_id].ai.take() {
        let new_ai = match ai {
            Ai::Basic => ai_basic(monster_id, game, objects, fov_map),
            Ai::Chrysalis => ai_chrysalis(monster_id, game, objects, fov_map),
            Ai::Confused{previous_ai, num_turns} => ai_confused(
                monster_id, game, objects, previous_ai, num_turns)
        };
        objects[monster_id].ai = Some(new_ai);
    }
}

fn ai_move_randomly(monster_id: usize, map: &Map, objects: &mut[Object]) {
    actor::move_by(monster_id,
            rand::thread_rng().gen_range(-1, 2),
            rand::thread_rng().gen_range(-1, 2),
            map, objects);
}

fn ai_basic(monster_id: usize, game: &mut Game, objects: &mut [Object],
            fov_map: &FovMap) -> Ai {
    let (monster_x, monster_y) = objects[monster_id].pos();
    if fov_map.is_in_fov(monster_x, monster_y) {
        if objects[monster_id].distance_to(&objects[consts::PLAYER]) >= 2.0 {
            let (player_x, player_y) = objects[consts::PLAYER].pos();
            actor::move_towards(monster_id, player_x, player_y, &mut game.map, objects);
        } else if objects[consts::PLAYER].fighter.map_or(false, |f| f.hp > 0) {
            let (monster, player) = utils::mut_two(monster_id, consts::PLAYER, objects);
            monster.attack(player, &mut game.log);
        }
    } else {
        ai_move_randomly(monster_id, &game.map, objects);
    }
    Ai::Basic
}

fn ai_chrysalis(monster_id: usize, game: &mut Game, objects: &mut [Object],
                fov_map: &FovMap) -> Ai {
    let (monster_x, monster_y) = objects[monster_id].pos();
    if fov_map.is_in_fov(monster_x, monster_y) {
        if objects[monster_id].distance_to(&objects[consts::PLAYER]) >= 2.0 {
            let (player_x, player_y) = objects[consts::PLAYER].pos();
            actor::move_towards(monster_id, player_x, player_y, &mut game.map, objects);
        } else if objects[consts::PLAYER].fighter.map_or(false, |f| f.hp > 0) {
            let (monster, player) = utils::mut_two(monster_id, consts::PLAYER, objects);
            monster.attack(player, &mut game.log);
        }
    }
    Ai::Chrysalis
}

fn ai_confused(monster_id: usize, game: &mut Game, objects: &mut [Object],
               previous_ai: Box<Ai>, num_turns: i32)
               -> Ai {

    if num_turns >= 0 {
        game.log.add(format!("The {} is confused!",
                             objects[monster_id].name),
                     colors::LIGHT_BLUE);
        ai_move_randomly(monster_id, &game.map, objects);
        Ai::Confused{previous_ai: previous_ai, num_turns: num_turns - 1}
    } else {
        game.log.add(format!("The {} is no longer confused!",
                                  objects[monster_id].name),
                colors::RED);
        *previous_ai
    }
}
