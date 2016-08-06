use std::cmp;
use std::ascii::AsciiExt;

extern crate rand;
use rand::Rng;

extern crate rustc_serialize;

extern crate tcod;

use std::io::{Read, Write};
use std::fs::File;
use std::error::Error;

use rustc_serialize::json;

use tcod::console::{
    BackgroundFlag,
    blit,
    Console,
    FontLayout,
    FontType,
    Offscreen,
    Root,
    TextAlignment,
};

use tcod::map::Map as FovMap;
use tcod::colors::{self, Color};
use tcod::input::{self, Event, Key, KeyCode, Mouse};


mod consts;

type Map = Vec<Vec<Tile>>;
type Messages = Vec<(String, Color)>;

trait MessageLog {
    fn add<T: Into<String>>(&mut self, message: T, color: Color);
}

#[derive(Clone, Copy, Debug)]
struct Rect {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32)
        -> Self {
        Rect { x1: x, y1: y, x2: x + w, y2: y + h }
    }

    pub fn center(&self) -> (i32, i32) {
        let center_x = (self.x1 + self.x2) / 2;
        let center_y = (self.y1 + self.y2) / 2;
        (center_x, center_y)
    }

    pub fn intersects_with(&self, other: &Rect) -> bool {
        (self.x1 <= other.x2) && (self.x2 >= other.x1) &&
            (self.y1 <= other.y2) && (self.y2 >= other.y1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

enum UseResult {
    UsedUp,
    Cancelled,
}

#[derive(Clone, Copy, Debug, RustcEncodable, RustcDecodable)]
struct Tile {
    impassable: bool,
    blocks_sight: bool,
    explored: bool,
}

impl Tile {
    pub fn empty() -> Self {
        Tile{ blocks_sight: false, explored: false, impassable: false, }
    }

    pub fn wall() -> Self {
        Tile{ blocks_sight: true, explored: false, impassable: true,  }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, RustcEncodable, RustcDecodable)]
struct Fighter {
    max_hp: i32,
    hp: i32,
    defense: i32,
    power: i32,
    on_death: DeathCallback,
}

#[derive(Clone, Copy, Debug, PartialEq, RustcEncodable, RustcDecodable)]
enum DeathCallback {
    Player,
    Monster,
}

impl DeathCallback {
    fn callback(self, object: &mut Object, messages: &mut Messages) {
        use DeathCallback::*;
        let callback: fn(&mut Object, &mut Messages) = match self {
            Player => player_death,
            Monster => monster_death,
        };
        callback(object, messages);
    }
}

#[derive(Debug, RustcEncodable, RustcDecodable)]
enum Ai {
    Basic,
    Chrysalis,
    Confused{previous_ai: Box<Ai>, num_turns: i32},
}

#[derive(Debug, RustcEncodable, RustcDecodable)]
struct Object {
    x: i32,
    y: i32,
    symbol: char,
    color: Color,
    name: String,
    blocks: bool,
    alive: bool,
    fighter: Option<Fighter>,
    ai: Option<Ai>,
    item: Option<Item>,
}

impl Object {
    pub fn new(x: i32, y: i32, symbol: char, name: &str,
               color: Color, blocks: bool) -> Self {
        Object {
            x: x,
            y: y,
            symbol: symbol,
            color: color,
            name: name.into(),
            blocks: blocks,
            alive: false,
            fighter: None,
            ai: None,
            item: None,
        }
    }

    pub fn draw(&self, con: &mut Console) {
        con.set_default_foreground(self.color);
        con.put_char(self.x, self.y, self.symbol,
                     BackgroundFlag::None);
    }

    pub fn clear(&self, con: &mut Console) {
        con.put_char(self.x, self.y, ' ', BackgroundFlag::None);
    }

    pub fn pos(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    pub fn distance(&self, x: i32, y: i32) -> f32 {
        (((x - self.x).pow(2) + (y - self.y).pow(2)) as f32).sqrt()
    }

    pub fn distance_to(&self, other: &Object) -> f32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        ((dx.pow(2) + dy.pow(2)) as f32).sqrt()
    }

    pub fn take_damage(&mut self, damage: i32, messages: &mut Messages) {
        if let Some(ref mut fighter) = self.fighter {
            if damage > 0 {
                if damage >= fighter.hp {
                    fighter.hp = 0;
                    self.alive = false;
                } else {
                    fighter.hp -= damage;
                }
            }
        }

        if !self.alive {
            if let Some(fighter) = self.fighter {
                fighter.on_death.callback(self, messages);
            }
        }
    }

    pub fn heal(&mut self, amount: i32) {
        if let Some(ref mut fighter) = self.fighter {
            fighter.hp += amount;
            if fighter.hp > fighter.max_hp {
                fighter.hp = fighter.max_hp;
            }
        }
    }

    pub fn attack(&mut self, target: &mut Object, log: &mut Messages) {
        let damage = self.fighter.map_or(0, |f| f.power) -
            target.fighter.map_or(0, |f| f.defense);
        if damage > 0 {
            log.add(
                    format!("{} attacks {} for {} hit points.", self.name,
                            target.name, damage),
                    colors::WHITE);
            target.take_damage(damage, log);
        } else {
            log.add(
                    format!("{} attacks {} but whatevs!",
                            self.name, target.name),
                    colors::WHITE);
        }
    }

}

struct Tcod {
    root: Root,
    con: Offscreen,
    panel: Offscreen,
    fov: FovMap,
    mouse: Mouse,
}

impl MessageLog for Vec<(String, Color)> {
    fn add<T: Into<String>>(&mut self, message: T, color: Color) {
        self.push((message.into(), color));
    }
}

#[derive(RustcEncodable, RustcDecodable)]
struct Game {
    map: Map,
    log: Messages,
    inventory: Vec<Object>,
}


#[derive(Clone, Copy, Debug, PartialEq, RustcEncodable, RustcDecodable)]
enum Item {
    Confuse,
    Fireball,
    Heal,
    Lightning,
}

fn create_room(room: Rect, map: &mut Map) {
    for x in (room.x1 + 1)..room.x2 {
        for y in (room.y1 + 1)..room.y2 {
            map[x as usize][y as usize] = Tile::empty();
        }
    }
}

fn create_h_tunnel(x1: i32, x2: i32, y: i32, map: &mut Map) {
    for x in cmp::min(x1, x2)..(cmp::max(x1, x2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn create_v_tunnel(y1: i32, y2: i32, x: i32, map: &mut Map) {
    for y in cmp::min(y1, y2)..(cmp::max(y1, y2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn is_blocked(x: i32, y: i32, map: &Map, objects: &[Object]) -> bool {
    // first test the map tile
    if map[x as usize][y as usize].impassable {
        return true;
    }
    // now check for any blocking objects
    objects.iter().any(|object| {
        object.blocks && object.pos() == (x, y)
    })
}

fn place_objects(room: Rect, map: &Map, objects: &mut Vec<Object>) {
    let num_monsters = rand::thread_rng().gen_range(0, consts::MAX_ROOM_MONSTERS + 1);

    for _ in 0..num_monsters {
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);
        if !is_blocked(x, y, map, objects) {
            let mut monster = if rand::random::<f32>() < 0.8 {
                let mut c_zombie = Object::new(x, y, 'Z', "Chrysalis zombie",
                                          colors::DESATURATED_GREEN, true);
                c_zombie.fighter = Some(Fighter{
                    max_hp: 10, hp: 10, defense: 0, power: 3,
                    on_death: DeathCallback::Monster,
                });
                c_zombie.ai = Some(Ai::Chrysalis);
                c_zombie
            } else {
                let mut zombie = Object::new(x, y, 'Z', "runner zombie",
                                            colors::DARKER_GREEN, true);
                zombie.fighter = Some(Fighter{
                    max_hp: 16, hp: 16, defense: 1, power: 4,
                    on_death: DeathCallback::Monster,
                });
                zombie.ai = Some(Ai::Basic);
                zombie
            };
            monster.alive = true;
            objects.push(monster);
        }
    }

    let num_items = rand::thread_rng().gen_range(0, consts::MAX_ROOM_ITEMS + 1);

    for _ in 0..num_items {
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        if !is_blocked(x, y, map, objects) {
            let dice = rand::random::<f32>();
            let item = if dice < 0.7 {
                let mut object = Object::new(x, y, '!', "First aid kit",
                                             colors::VIOLET, false);
                object.item = Some(Item::Heal);
                object
            } else if dice < 0.7 + 0.1 {
                let mut object = Object::new(x, y, '#',
                                             "scroll of lightning bolt",
                                             colors::LIGHT_YELLOW, false);
                object.item = Some(Item::Lightning);
                object
            } else if dice < 0.7 + 0.1 + 0.1 {
                let mut object = Object::new(x, y, '#', "molotov cocktail",
                                             colors::LIGHT_YELLOW, false);
                object.item = Some(Item::Fireball);
                object
            } else {
                let mut object = Object::new(x, y, '#', "scroll of confusion",
                                             colors::LIGHT_YELLOW, false);
                object.item = Some(Item::Confuse);
                object
            };
            objects.push(item);
        }
    }
}

fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut[Object]) {
    let (x, y) = objects[id].pos();
    if !is_blocked(x + dx, y + dy, map, objects) {
        objects[id].set_pos(x + dx, y + dy);
    }
}

fn move_towards(id: usize, target_x: i32, target_y: i32, map: &Map,
                objects: &mut [Object]) {
    let dx = target_x - objects[id].x;
    let dy = target_y - objects[id].y;
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();

    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;

    move_by(id, dx, dy, map, objects);
}

fn player_move_or_attack(dx: i32, dy: i32, game: &mut Game,
                         objects: &mut [Object]) {
    let x = objects[consts::PLAYER].x + dx;
    let y = objects[consts::PLAYER].y + dy;

    let target_id = objects.iter().position(|object| {
        object.fighter.is_some() && object.pos() == (x, y)
    });

    match target_id {
        Some(target_id) => {
            let (player, target) = mut_two(consts::PLAYER, target_id, objects);
            player.attack(target, &mut game.log);
        }
        None => {
            move_by(consts::PLAYER, dx, dy, &mut game.map, objects);
        }
    }
}

fn target_tile(tcod: &mut Tcod, game: &mut Game, objects: &[Object],
               max_range: Option<f32>)
    -> Option<(i32, i32)> {
    loop {
        tcod.root.flush();
        let event = input::check_for_event(input::KEY_PRESS |
                                           input::MOUSE)
            .map(|e| e.1);
        let mut key = None;
        match event {
            Some(Event::Mouse(m)) => tcod.mouse = m,
            Some(Event::Key(k)) => key = Some(k),
            None => {}
        }
        render_all(tcod, game, objects, false);

        let (x, y) = (tcod.mouse.cx as i32, tcod.mouse.cy as i32);
        let in_fov = (x < consts::MAP_WIDTH) && (y < consts::MAP_HEIGHT) &&
            tcod.fov.is_in_fov(x, y);
        let in_range = max_range.map_or(true,
                                        |range| objects[consts::PLAYER]
                                    .distance(x, y) <= range);
        if tcod.mouse.lbutton_pressed && in_fov && in_range {
            return Some((x, y))
        }

        let escape = key.map_or(false, |k| k.code == KeyCode::Escape);
        if tcod.mouse.rbutton_pressed || escape {
            return None
        }
    }
}

fn target_monster(tcod: &mut Tcod, game: &mut Game, objects: &[Object],
                  max_range: Option<f32>) -> Option<usize> {
    loop {
        match target_tile(tcod, game, objects, max_range) {
            Some((x, y)) => {
                for(id, obj) in objects.iter().enumerate() {
                    if obj.pos() == (x, y) && obj.fighter.is_some() &&
                        id != consts::PLAYER {
                            return Some(id)
                        }
                }
            }
            None => return None,
        }
    }
}

fn drop_item(inventory_id: usize, game: &mut Game,
             objects: &mut Vec<Object>) {
    let mut item = game.inventory.remove(inventory_id);
    item.set_pos(objects[consts::PLAYER].x, objects[consts::PLAYER].y);
    game.log.add(format!("You dropped a {}.", item.name),
                 colors::YELLOW);
    objects.push(item);
}

fn pick_item_up(object_id: usize, game: &mut Game,
                objects: &mut Vec<Object>) {
    if game.inventory.len() as i32 >= consts::MAX_INVENTORY_ITEMS {
        game.log.add(
                format!("Your inventory is full, cannot pickup {}.",
                        objects[object_id].name),
                colors::RED);
    } else {
        let item = objects.swap_remove(object_id);
        game.log.add( format!("You picked up a {}!", item.name),
                colors::GREEN);
        game.inventory.push(item);
    }
}

fn mut_two<T>(first_index: usize, second_index: usize, items: &mut [T]) ->
    (&mut T, &mut T) {
        assert!(first_index != second_index);
        let split_at_index = cmp::max(first_index, second_index);
        let (first_slice, second_slice) = items.split_at_mut(split_at_index);
        if first_index < second_index {
            (&mut first_slice[first_index], &mut second_slice[0])
        } else {
            (&mut second_slice[0], &mut first_slice[second_index])
        }
    }

fn ai_take_turn(monster_id: usize, game: &mut Game, objects: &mut [Object],
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
    move_by(monster_id,
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
            move_towards(monster_id, player_x, player_y, &mut game.map, objects);
        } else if objects[consts::PLAYER].fighter.map_or(false, |f| f.hp > 0) {
            let (monster, player) = mut_two(monster_id, consts::PLAYER, objects);
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
            move_towards(monster_id, player_x, player_y, &mut game.map, objects);
        } else if objects[consts::PLAYER].fighter.map_or(false, |f| f.hp > 0) {
            let (monster, player) = mut_two(monster_id, consts::PLAYER, objects);
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

fn player_death(player: &mut Object, log: &mut Messages) {
    log.add("You died!", colors::RED);
    player.symbol = '%';
    player.color = colors::DARK_RED;
}

fn monster_death(monster: &mut Object, log: &mut Messages) {
    log.add(format!("{} is dead!", monster.name), colors::ORANGE);
    monster.symbol = '%';
    monster.color = colors::DARK_RED;
    monster.blocks = false;
    monster.fighter = None;
    monster.ai = None;
    monster.name = format!("remains of {}", monster.name);
}

// fn message<T: Into<String>>(messages: &mut Messages, message: T, color: Color) {
//     if messages.len() == consts::MSG_HEIGHT {
//         messages.remove(0);
//     }
//     messages.push((message.into(), color));
// }

fn make_map(objects: &mut Vec<Object>) -> Map {
    let mut map = vec![vec![Tile::wall(); consts::MAP_HEIGHT as usize];
                       consts::MAP_WIDTH as usize];
    let mut rooms = vec![];
    assert_eq!(&objects[consts::PLAYER] as *const _, &objects[0] as *const _);
    objects.truncate(1);

    for _ in 0..consts::MAX_ROOMS {
        let w = rand::thread_rng().gen_range(consts::ROOM_MIN_SIZE,
                                             consts::ROOM_MAX_SIZE + 1);
        let h = rand::thread_rng().gen_range(consts::ROOM_MIN_SIZE,
                                             consts::ROOM_MAX_SIZE + 1);
        let x = rand::thread_rng().gen_range(0, consts::MAP_WIDTH - w);
        let y = rand::thread_rng().gen_range(0, consts::MAP_HEIGHT - h);

        let new_room = Rect::new(x, y, w, h);

        let failed = rooms.iter().any(|other_room
                                      |new_room.intersects_with(
                                          other_room));

        if !failed {

            create_room(new_room, &mut map);
            place_objects(new_room, &map, objects);

            let (new_x, new_y) = new_room.center();

            if rooms.is_empty() {
                objects[consts::PLAYER].set_pos(new_x, new_y);
            } else {
                let (prev_x, prev_y) =
                    rooms[rooms.len() - 1].center();

                if rand::random() {
                    create_h_tunnel(prev_x, new_x,
                                    prev_y, &mut map);
                    create_v_tunnel(prev_y, new_y,
                                    prev_x, &mut map);
                } else {
                    create_v_tunnel(prev_y, new_y,
                                    prev_x, &mut map);
                    create_h_tunnel(prev_x, new_x,
                                    prev_y, &mut map);
                }
            }
            rooms.push(new_room);
        }

    }
    let (last_room_x, last_room_y) = rooms[rooms.len() - 1].center();
    let stairs = Object::new(last_room_x, last_room_y, '>', "stairs up",
                             colors::WHITE, false);
    objects.push(stairs);
    map
}

fn render_bar(panel: &mut Offscreen,
              x: i32,
              y: i32,
              total_width: i32,
              name: &str,
              value: i32,
              maximum: i32,
              bar_color: Color,
              back_color: Color) {
    let bar_width = (value as f32 / maximum as f32 * total_width as f32) as i32;
    panel.set_default_background(back_color);
    panel.rect(x, y, total_width, 1, false, BackgroundFlag::Screen);

    panel.set_default_background(bar_color);
    if bar_width > 0 {
        panel.rect(x, y, bar_width, 1, false, BackgroundFlag::Screen);
    }

    panel.set_default_foreground(colors::WHITE);
    panel.print_ex(x + total_width / 2, y, BackgroundFlag::None,
                   TextAlignment::Center, &format!("{}: {}/{}",
                                                   name, value, maximum));
}

fn render_all(tcod: &mut Tcod, game: &mut Game, objects: &[Object],
              fov_recompute: bool) {
    if fov_recompute {
        let player = &objects[consts::PLAYER];
        tcod.fov.compute_fov(player.x, player.y, consts::TORCH_RADIUS,
                             consts::FOV_LIGHT_WALLS, consts::FOV_ALGO);

        for y in 0..consts::MAP_HEIGHT {
            for x in 0..consts::MAP_WIDTH {
                let visible = tcod.fov.is_in_fov(x, y);
                // let visible = true;
                let wall = game.map[x as usize][y as usize].blocks_sight;
                let color = match(visible, wall) {
                    (false, true) => consts::COLOR_DARK_WALL,
                    (false, false) => consts::COLOR_DARK_GROUND,
                    (true, true) => consts::COLOR_LIGHT_WALL,
                    (true, false) => consts::COLOR_LIGHT_GROUND,
                };

                let explored =
                    &mut game.map[x as usize][y as usize].explored;
                if visible {
                    *explored = true;
                }

                if *explored {
                    tcod.con.set_char_background(x, y, color,
                                            BackgroundFlag::Set);
                }
            }
        }
    }

    let mut to_draw: Vec<_> = objects.iter()
        .filter(|o| tcod.fov.is_in_fov(o.x, o.y)).collect();

    to_draw.sort_by(|o1, o2| { o1.blocks.cmp(&o2.blocks) });
    for object in &to_draw {
        object.draw(&mut tcod.con);
    }

    blit(&mut tcod.con, (0, 0), (consts::MAP_WIDTH, consts::MAP_HEIGHT),
         &mut tcod.root, (0, 0), 1.0, 1.0);

    tcod.panel.set_default_background(colors::BLACK);
    tcod.panel.clear();

    // print the game messages, one line at a time
    let mut y = consts::MSG_HEIGHT as i32;
    for &(ref msg, color) in game.log.iter().rev() {
        let msg_height = tcod.panel.get_height_rect(consts::MSG_X, y,
                                               consts::MSG_WIDTH, 0, msg);
        y -= msg_height;
        if y < 0 {
            break;
        }
        tcod.panel.set_default_foreground(color);
        tcod.panel.print_rect(consts::MSG_X, y, consts::MSG_WIDTH, 0, msg);
    }

    // show the player's stats
    let hp = objects[consts::PLAYER].fighter.map_or(0, |f| f.hp);
    let max_hp = objects[consts::PLAYER].fighter.map_or(0, |f| f.max_hp);
    render_bar(&mut tcod.panel, 1, 1, consts::BAR_WIDTH, "HP", hp, max_hp,
               colors::LIGHT_RED, colors::DARKER_RED);

    tcod.panel.set_default_foreground(colors::LIGHT_GREY);
    tcod.panel.print_ex(1, 0, BackgroundFlag::None, TextAlignment::Left,
                   get_names_under_mouse(tcod.mouse, objects, &tcod.fov));
    // blit the contents of `panel` to the root console
    blit(&mut tcod.panel, (0, 0), (consts::SCREEN_WIDTH, consts::PANEL_HEIGHT),
         &mut tcod.root, (0, consts::PANEL_Y), 1.0, 1.0);
}

fn handle_keys(key: Key, tcod: &mut Tcod, game: &mut Game,
               objects: &mut Vec<Object>) -> PlayerAction {
    use PlayerAction::*;

    let player_alive = objects[consts::PLAYER].alive;
    match (key, player_alive) {
        // Exit: Ctrl+q
        (Key { printable: 'q', ctrl: true, .. }, _) => Exit,
        //*************************************************
        // Movement keys
        //*************************************************

        ///////////////////////////////////////////////////
        // Up
        ///////////////////////////////////////////////////
        (Key { code: KeyCode::Up, .. }, true) |
        (Key { code: KeyCode::NumPad8, ..}, true) |
        (Key { printable: 'k', ..}, true) => {
            player_move_or_attack(0, -1, game, objects);
            TookTurn
        }
        ///////////////////////////////////////////////////
        // Down
        ///////////////////////////////////////////////////
        (Key { code: KeyCode::Down, .. }, true) |
        (Key { code: KeyCode::NumPad2, ..}, true) |
        (Key { printable: 'j', ..}, true) => {
            player_move_or_attack(0, 1, game, objects);
            TookTurn
        }
        ///////////////////////////////////////////////////
        // Left
        ///////////////////////////////////////////////////
        (Key { code: KeyCode::Left, .. }, true) |
        (Key { code: KeyCode::NumPad4, ..}, true) |
        (Key { printable: 'h', ..}, true) => {
            player_move_or_attack(-1, 0, game, objects);
            TookTurn
        }
        ///////////////////////////////////////////////////
        // Right
        ///////////////////////////////////////////////////
        (Key { code: KeyCode::Right, .. }, true) |
        (Key { code: KeyCode::NumPad6, ..}, true) |
        (Key { printable: 'l', ..}, true) => {
            player_move_or_attack(1, 0, game, objects);
            TookTurn
        }
        ///////////////////////////////////////////////////
        // Up Left
        ///////////////////////////////////////////////////
        (Key { code: KeyCode::Home, .. }, true) |
        (Key { code: KeyCode::NumPad7, ..}, true) |
        (Key { printable: 'y', ..}, true) => {
            player_move_or_attack(-1, -1, game, objects);
            TookTurn
        }
        ///////////////////////////////////////////////////
        // Up Right
        ///////////////////////////////////////////////////
        (Key { code: KeyCode::PageUp, .. }, true) |
        (Key { code: KeyCode::NumPad9, ..}, true) |
        (Key { printable: 'u', ..}, true) => {
            player_move_or_attack(1, -1, game, objects);
            TookTurn
        }
        ///////////////////////////////////////////////////
        // Down Left
        ///////////////////////////////////////////////////
        (Key { code: KeyCode::End, .. }, true) |
        (Key { code: KeyCode::NumPad1, ..}, true) |
        (Key { printable: 'b', ..}, true) => {
            player_move_or_attack(-1, 1, game, objects);
            TookTurn
        }
        ///////////////////////////////////////////////////
        // Down Right
        ///////////////////////////////////////////////////
        (Key { code: KeyCode::PageDown, .. }, true) |
        (Key { code: KeyCode::NumPad3, ..}, true) |
        (Key { printable: 'n', ..}, true) => {
            player_move_or_attack(1, 1, game, objects);
            TookTurn
        }
        ///////////////////////////////////////////////////
        // Wait (Don't move)
        ///////////////////////////////////////////////////
        (Key { code: KeyCode::Spacebar, ..}, true) |
        (Key { code: KeyCode::NumPad5, ..}, true) => {
            TookTurn
        }
        //*************************************************
        // End movement keys
        //*************************************************

        ///////////////////////////////////////////////////
        // Pick up
        ///////////////////////////////////////////////////
        (Key { printable: ',', ..}, true) => {
            let item_id = objects.iter().position(
                |object| {
                    object.pos() == objects[consts::PLAYER].pos() &&
                        object.item.is_some()
                });
            if let Some(item_id) = item_id {
                pick_item_up(item_id, game, objects);
            }
            DidntTakeTurn
        }
        (Key { printable: 'i', .. }, true) => {
            let inventory_index = inventory_menu(
                &mut game.inventory,
                "Press the key next to an item to use it, \
                 or any other to cancel.\n",
                &mut tcod.root);
            if let Some(inventory_index) = inventory_index {
                use_item(tcod, game, inventory_index, objects);
            }
            DidntTakeTurn
        }
        (Key { printable: 'd', .. }, true) => {
            let inventory_index = inventory_menu(
                &mut game.inventory,
                "Press the key next to an item to drop it, \
                 or any other to cancel.\n",
                &mut tcod.root);
            if let Some(inventory_index) = inventory_index {
                drop_item(inventory_index, game, objects);
            }
            DidntTakeTurn
        }
        (Key { printable: '>', .. }, true) => {
            let player_on_stairs = objects.iter().any(
                |object| {
                    object.pos() == objects[consts::PLAYER].pos() &&
                        object.name == "stairs up"
                });
            if player_on_stairs {
                next_level(tcod, objects, game);
            }
            DidntTakeTurn
        }
        _ => DidntTakeTurn,
    }
}

fn menu<T: AsRef<str>>(header: &str, options: &[T], width: i32,
                       root: &mut Root) -> Option<usize> {
    let header_height = if header.is_empty() {
        0
    } else {
        root.get_height_rect(0, 0, width, consts::SCREEN_HEIGHT, header)
    };

    let options_len = options.len() as i32;
    assert!(options_len <= consts::MAX_INVENTORY_ITEMS,
            format!("Cannot have a menu with more than {} options.",
            consts::MAX_INVENTORY_ITEMS));

    let header_height = root.get_height_rect(0, 0,
                                             width, consts::SCREEN_HEIGHT, header);
    let height = options_len + header_height;

    let mut window = Offscreen::new(width, height);

    window.set_default_foreground(colors::WHITE);
    window.print_rect_ex(0, 0, width, height, BackgroundFlag::None,
                         TextAlignment::Left, header);

    for (index, option_text) in options.iter().enumerate() {
        let menu_letter = (b'a' + index as u8) as char;
        let text = format!("({}) {}", menu_letter, option_text.as_ref());
        window.print_ex(0, header_height + index as i32,
                        BackgroundFlag::None, TextAlignment::Left, text);
    }

    let x = consts::SCREEN_WIDTH / 2 - width / 2;
    let y = consts::SCREEN_HEIGHT / 2 - height / 2;
    blit(&mut window, (0, 0), (width, height), root, (x, y),
         1.0, 0.7);

    root.flush();
    let key = root.wait_for_keypress(true);

    if key.printable.is_alphabetic() {
        let index = key.printable.to_ascii_lowercase() as usize - 'a' as usize;
        if index < options.len() {
            Some(index)
        } else {
            None
        }
    } else {
        None
    }
}

fn inventory_menu(inventory: &[Object], header: &str, root: &mut Root)
                  -> Option<usize> {
    let options = if inventory.len() == 0 {
        vec!["Inventory is empty.".into()]
    } else {
        inventory.iter().map(|item| {item.name.clone() }).collect()
    };

    let inventory_index = menu(header, &options,
                               consts::INVENTORY_WIDTH, root);


    if inventory.len() > 0 {
        inventory_index
    } else {
        None
    }
}

fn use_item(tcod: &mut Tcod, game: &mut Game, inventory_id: usize,
            objects: &mut [Object]) {
    if let Some(item) = game.inventory[inventory_id].item {
        let on_use:
        fn(&mut Tcod, &mut Game, usize, &mut [Object])
           -> UseResult = match item {
            Item::Confuse => cast_confuse,
            Item::Fireball => cast_fireball,
            Item::Heal => heal_player,
            Item::Lightning => cast_lightning,
        };
        match on_use(tcod, game, inventory_id, objects) {
            UseResult::UsedUp => {
                game.inventory.remove(inventory_id);
            }
            UseResult::Cancelled => {
                game.log.add( "Cancelled", colors::WHITE);
            }
        }
    } else {
        game.log.add(
                format!("The {} cannot be used.",
                        game.inventory[inventory_id].name),
                colors::WHITE);
    }
}

fn closest_monster(max_range: i32, objects: &mut [Object], tcod: &Tcod)
    -> Option<usize> {
    let mut closest_enemy = None;
    let mut closest_dist = (max_range + 1) as f32;

    for (id, object) in objects.iter().enumerate() {
        if (id != consts::PLAYER) &&
            object.fighter.is_some() &&
            object.ai.is_some() &&
            tcod.fov.is_in_fov(object.x, object.y) {
                let dist = objects[consts::PLAYER].distance_to(object);
                if dist < closest_dist {
                    closest_enemy = Some(id);
                    closest_dist = dist;
                }
            }
    }
    closest_enemy
}

fn heal_player(tcod: &mut Tcod, game: &mut Game, inventory_id: usize,
                 objects: &mut [Object]) -> UseResult {
    if let Some(fighter) = objects[consts::PLAYER].fighter {
        if fighter.hp == fighter.max_hp {
            game.log.add( "You are already at full health.", colors::RED);
            return UseResult::Cancelled;
        }
        game.log.add( "Your wounds start to feel better!",
                colors::LIGHT_VIOLET);
        objects[consts::PLAYER].heal(3);
        return UseResult::UsedUp;
    }
    UseResult::Cancelled
}

fn cast_confuse(tcod: &mut Tcod, game: &mut Game, inventory_id: usize,
                 objects: &mut [Object]) -> UseResult {
    game.log.add( "Left-click an enemy to confuse it, or right-click \
                       to cancel.",
            colors::LIGHT_CYAN);
    let monster_id = target_monster(tcod, game, objects, Some(5.0));
    if let Some(monster_id) = monster_id {
        let old_ai = objects[monster_id].ai.take().unwrap_or(Ai::Basic);
        objects[monster_id].ai = Some(Ai::Confused {
            previous_ai: Box::new(old_ai),
            num_turns: 3,
        });
        game.log.add(
                format!("The eyes of the {} look vacant and it starts to \
                         stumble around!", objects[monster_id].name),
                colors::LIGHT_GREEN);
        UseResult::UsedUp
    } else {
        game.log.add( "No enemy is within range.", colors::RED);
        UseResult::Cancelled
    }
}

fn cast_fireball(tcod: &mut Tcod, game: &mut Game, inventory_id: usize,
                 objects: &mut [Object]) -> UseResult {
    game.log.add( "Left-click a target tile for the fireball, \
                       or right-click to cancel.",
            colors::LIGHT_CYAN);
    let (x, y) = match target_tile(tcod, game, objects, None) {
        Some(tile_pos) => tile_pos,
        None => return UseResult::Cancelled,
        };

    game.log.add(
            format!("The fireball explodes, burning everything within a {} \
                     radius!", 5),
            colors::GREEN);

    for obj in objects {
        if obj.distance(x, y) <= 5.0 && obj.fighter.is_some() {
            game.log.add(
                    format!("The {} gets burned for {} hit points.",
                            obj.name, 10),
                    colors::ORANGE);
            obj.take_damage(10, &mut game.log);
        }
    }
    UseResult::UsedUp
}


fn cast_lightning(tcod: &mut Tcod, game: &mut Game, inventory_id: usize,
                 objects: &mut [Object]) -> UseResult {
    let monster_id = closest_monster(10, objects, tcod);
    if let Some(monster_id) = monster_id {
        game.log.add(
                format!("A lightning bolt strikes the {} with loud thunder! \
                         The damage is {} hit points.",
                        objects[monster_id].name, 10),
                colors::LIGHT_BLUE);
        objects[monster_id].take_damage(10, &mut game.log);
        UseResult::UsedUp
    } else {
        game.log.add( "No enemy is within range.", colors::RED);
        UseResult::Cancelled
    }
}

fn get_names_under_mouse(mouse: Mouse, objects: &[Object], fov_map: &FovMap)
    -> String {
    let (x, y) = (mouse.cx as i32, mouse.cy as i32);

    let names = objects.iter().filter(
        |obj| {obj.pos() == (x, y) && fov_map.is_in_fov(obj.x, obj.y)})
        .map(|obj| obj.name.clone())
        .collect::<Vec<_>>();

    names.join(", ")
}

fn save_game(objects: &[Object], game: &Game) -> Result<(), Box<Error>> {
    let save_data = try!{ json::encode(&(objects, game)) };
    let mut file = try!{ File::create("savegame") };
    try!{ file.write_all(save_data.as_bytes()) };
    Ok(())
}

fn load_game() -> Result<(Vec<Object>, Game), Box<Error>> {
    let mut json_save_state = String::new();
    let mut file = try! { File::open("savegame") };
    try! { file.read_to_string(&mut json_save_state) };
    let result = try! { json::decode::<(Vec<Object>, Game)>(&json_save_state) };
    Ok(result)
}

fn new_game(tcod: &mut Tcod) -> (Vec<Object>, Game) {
    let mut player = Object::new(0, 0, '@', "player", colors::WHITE, true);
    player.alive = true;
    player.fighter = Some(Fighter{
        max_hp: 30, hp: 30, defense: 2, power: 5, on_death: DeathCallback::Player,
    });
    let mut objects = vec![player];
    let mut game = Game {
        map: make_map(&mut objects),
        log: vec![],
        inventory: vec![],
    };

    initialize_fov(&game.map, tcod);

    game.log.add("Meow!", colors::RED);

    (objects, game)

}

fn initialize_fov(map: &Map, tcod: &mut Tcod) {
    for y in 0..consts::MAP_HEIGHT {
        for x in 0..consts::MAP_WIDTH {
            tcod.fov.set(x, y,
                         !map[x as usize][y as usize].blocks_sight,
                         !map[x as usize][y as usize].impassable);
        }
    }
    tcod.con.clear();
}

fn play_game(objects: &mut Vec<Object>, game: &mut Game, tcod: &mut Tcod) {

    let mut previous_player_position = (-1, -1);
    let mut key = Default::default();

    while !tcod.root.window_closed() {
        let fov_recompute = previous_player_position !=
            (objects[consts::PLAYER].pos());
        match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
            Some((_, Event::Mouse(m))) => tcod.mouse = m,
            Some((_, Event::Key(k))) => key = k,
            _ => key = Default::default(),
        }

        render_all(tcod, game, &objects, fov_recompute);

        tcod.root.flush();

        for object in objects.iter_mut() {
            object.clear(&mut tcod.con)
        }

        previous_player_position = objects[consts::PLAYER].pos();
        let player_action = handle_keys(key, tcod, game, objects);
        if player_action == PlayerAction::Exit {
            //# TODO: Catch and handle save game errors
            save_game(objects, game).unwrap();
            break
        }

        if objects[consts::PLAYER].alive &&
            player_action != PlayerAction::DidntTakeTurn {
                for id in 0..objects.len() {
                    if objects[id].ai.is_some() {
                        ai_take_turn(id, game, objects, &tcod.fov);
                    }
                }
            }

    }
}

fn next_level(tcod: &mut Tcod, objects: &mut Vec<Object>, game: &mut Game) {
    game.map = make_map(objects);
    initialize_fov(&game.map, tcod);
}

fn msgbox(text: &str, width: i32, root: &mut Root) {
    let options: &[&str] = &[];
    menu(text, options, width, root);
}

fn main_menu(tcod: &mut Tcod) {
    let img = tcod::image::Image::from_file("menu_background.png")
        .ok().expect("Background image not found");

    while !tcod.root.window_closed() {
        tcod::image::blit_2x(&img, (0, 0), (-1, -1), &mut tcod.root, (0, 0));
        tcod.root.set_default_foreground(colors::LIGHT_YELLOW);
        tcod.root.print_ex(consts::SCREEN_WIDTH/2, consts::SCREEN_HEIGHT/2 - 4,
                           BackgroundFlag::None, TextAlignment::Center,
                           "RustWorld");
        tcod.root.print_ex(consts::SCREEN_WIDTH/2, consts::SCREEN_HEIGHT/2 - 2,
                           BackgroundFlag::None, TextAlignment::Center,
                           "Meow");

        let choices = &["Play a new game", "Continue last game", "Quit"];
        let choice = menu("", choices, 24, &mut tcod.root);

        match choice {
            Some(0) => {
                let (mut objects, mut game) = new_game(tcod);
                play_game(&mut objects, &mut game, tcod);
            }
            Some(1) => {
                match load_game() {
                    Ok((mut objects, mut game)) => {
                        initialize_fov(&game.map, tcod);
                        play_game(&mut objects, &mut game, tcod);
                    }
                    Err(_e) => {
                        msgbox("\nSaved game failed to load.\n",
                               24, &mut tcod.root);
                        continue;
                    }
                }
            }
            Some(2) => {
                break;
            }
            _ => {}
        }
    }
}

fn main() {
    let root = Root::initializer()
        // .font("arial10x10.png", FontLayout::Tcod)
        .font("bluebox.png", FontLayout::AsciiInRow)
        .font_type(FontType::Greyscale)
        .size(consts::SCREEN_WIDTH, consts::SCREEN_HEIGHT)
        .title("RustWorld")
        .init();

    tcod::system::set_fps(consts::LIMIT_FPS);

    let mut tcod = Tcod {
        root: root,
        con: Offscreen::new(consts::MAP_WIDTH, consts::MAP_HEIGHT),
        panel: Offscreen::new(consts::SCREEN_WIDTH, consts::PANEL_HEIGHT),
        fov: FovMap::new(consts::MAP_WIDTH, consts::MAP_HEIGHT),
        mouse: Default::default(),
    };

    main_menu(&mut tcod);
}
