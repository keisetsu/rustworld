use tcod::colors;

use consts;
use game::Game;
use log;
use log::MessageLog;
use object::{self, Object};
use object::item::{self, Function};
use map::{self, Map};
use ui::Ui;
use util;


#[derive(Clone, Copy, Debug, PartialEq, RustcEncodable, RustcDecodable)]
pub enum DeathCallback {
    Player,
    Monster,
}

impl DeathCallback {
    pub fn callback(self, object: &mut Object, messages: &mut log::Messages) {
        let callback: fn(&mut Object, &mut log::Messages) = match self {
            DeathCallback::Player => player_death,
            DeathCallback::Monster => monster_death,
        };
        callback(object, messages);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, RustcEncodable, RustcDecodable)]
pub struct Fighter {
    pub max_hp: i32,
    pub hp: i32,
    pub defense: i32,
    pub power: i32,
    pub on_death: DeathCallback,
}

pub fn move_by(id: usize, dx: i32, dy: i32, map: &Map, actors: &mut[Object]) {
    let (x, y) = actors[id].pos();
    if map::is_blocked(x + dx, y + dy, map, actors) == object::Blocks::No {
        actors[id].set_pos(x + dx, y + dy);
    }
}

pub fn move_towards(id: usize, target_x: i32, target_y: i32, map: &Map,
                actors: &mut [Object]) {
    let dx = target_x - actors[id].x;
    let dy = target_y - actors[id].y;
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();

    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;

    move_by(id, dx, dy, map, actors);
}

pub fn player_move_or_attack(dx: i32, dy: i32, game: &mut Game,
                         actors: &mut [Object]) {
    let x = actors[consts::PLAYER].x + dx;
    let y = actors[consts::PLAYER].y + dy;

    let target_id = actors.iter().position(|object| {
        object.fighter.is_some() && object.pos() == (x, y)
    });

    match target_id {
        Some(target_id) => {
            let (player, target) =
                util::mut_two(consts::PLAYER, target_id, actors);
            player.attack(target, &mut game.log);
        }
        None => {
            move_by(consts::PLAYER, dx, dy, &mut game.map, actors);
        }
    }
}

fn player_death(player: &mut Object, log: &mut log::Messages) {
    log.alert("You died!");
    player.symbol = '%';
    player.color = colors::DARK_RED;
}

fn monster_death(monster: &mut Object, log: &mut log::Messages) {
    log.status_change(format!("{} is dead!", monster.name));
    monster.symbol = '%';
    monster.color = colors::DARK_RED;
    monster.blocks = object::Blocks::No;
    monster.blocks_view = object::Blocks::No;
    monster.fighter = None;
    monster.ai = None;
    monster.name = format!("remains of {}", monster.name);
}

pub fn drop_item(x: i32, y: i32, inventory_id: usize, game: &mut Game,
             inventory: &mut Vec<Object>) {
    let mut item = inventory.remove(inventory_id);
    item.set_pos(x, y);
    game.log.info(format!("You dropped a {}.", item.name));
    game.map[x as usize][y as usize].items.push(item);
}

pub fn pick_up_items(x: i32, y: i32, inventory: &mut Vec<Object>, game: &mut Game) {
    let mut names = vec![];
    let ref mut items = game.map[x as usize][y as usize].items;
    for item_ix in (0..items.len()).rev() {
        if items[item_ix].can_pick_up {
            let mut item = items.swap_remove(item_ix);
            item.set_pos(-1, -1);
            names.push(item.name.clone());
            inventory.push(item);
        }
    }
    if names.len() > 0{
        game.log.info(format!("You picked up {}", names.join(", ")));
    }

}

pub fn use_item(game_ui: &mut Ui, game: &mut Game,
                inventory_id: usize, inventory: &mut Vec<Object>) {
    println!("{:?}", inventory.len());
    if let Some(function) = inventory[inventory_id].function {
        let on_use:
        fn(&mut Ui, &mut Game, &mut [Object])
           -> item::UseResult = match function {
            Function::Confuse => item::cast_confuse,
            Function::Fireball => item::cast_fireball,
            Function::Heal => item::heal_player,
            Function::Lightning => item::cast_lightning,
        };
        match on_use(game_ui, game, inventory) {
            item::UseResult::UsedUp => {
                inventory.remove(inventory_id);
            }
            item::UseResult::Cancelled => {
                game.log.info( "Cancelled");
            }
        }
    } else {
        game.log.alert(
            format!("The {} cannot be used.",
                    inventory[inventory_id].name));
    }
}
