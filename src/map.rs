extern crate rustc_serialize;

extern crate rand;
use rand::Rng;

extern crate tcod;
use tcod::colors;

use std::cmp;

use consts;
use object;
use object::actor;
use object::item::Item;
use ai::Ai;

pub const MAP_WIDTH: i32 = 80;
pub const MAP_HEIGHT: i32 = 43;

pub const ROOM_MAX_SIZE: i32 = 10;
pub const ROOM_MIN_SIZE: i32 = 6;
pub const MAX_ROOMS: i32 = 30;

pub const MAX_ROOM_MONSTERS: i32 = 3;
pub const MAX_ROOM_ITEMS:i32 = 4;

#[derive(Clone, Copy, Debug, RustcEncodable, RustcDecodable)]
pub struct Tile {
    pub impassable: bool,
    pub blocks_sight: bool,
    pub explored: bool,
}

impl Tile {
    pub fn empty() -> Self {
        Tile{ blocks_sight: false, explored: false, impassable: false, }
    }

    pub fn wall() -> Self {
        Tile{ blocks_sight: true, explored: false, impassable: true,  }
    }
}

pub type Map = Vec<Vec<Tile>>;

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

pub fn is_blocked(x: i32, y: i32, map: &Map, objects: &[object::Object]) -> bool {
    // first test the map tile
    if map[x as usize][y as usize].impassable {
        return true;
    }
    // now check for any blocking objects
    objects.iter().any(|object| {
        object.blocks && object.pos() == (x, y)
    })
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

fn place_objects(room: Rect, map: &Map, objects: &mut Vec<object::Object>) {
    let num_monsters = rand::thread_rng().gen_range(0, MAX_ROOM_MONSTERS + 1);

    for _ in 0..num_monsters {
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);
        if !is_blocked(x, y, map, objects) {
            let mut monster = if rand::random::<f32>() < 0.8 {
                let mut c_zombie = object::Object::new(x, y, 'Z', "Chrysalis zombie",
                                          colors::DESATURATED_GREEN, true);
                c_zombie.fighter = Some(actor::Fighter{
                    max_hp: 10, hp: 10, defense: 0, power: 3,
                    on_death: actor::DeathCallback::Monster,
                });
                c_zombie.ai = Some(Ai::Chrysalis);
                c_zombie
            } else {
                let mut zombie = object::Object::new(x, y, 'Z', "runner zombie",
                                            colors::DARKER_GREEN, true);
                zombie.fighter = Some(actor::Fighter{
                    max_hp: 16, hp: 16, defense: 1, power: 4,
                    on_death: actor::DeathCallback::Monster,
                });
                zombie.ai = Some(Ai::Basic);
                zombie
            };
            monster.alive = true;
            objects.push(monster);
        }
    }

    let num_items = rand::thread_rng().gen_range(0, MAX_ROOM_ITEMS + 1);

    for _ in 0..num_items {
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        if !is_blocked(x, y, map, objects) {
            let dice = rand::random::<f32>();
            let item = if dice < 0.7 {
                let mut object = object::Object::new(x, y, '!', "First aid kit",
                                             colors::VIOLET, false);
                object.item = Some(Item::Heal);
                object
            } else if dice < 0.7 + 0.1 {
                let mut object = object::Object::new(x, y, '#',
                                             "scroll of lightning bolt",
                                             colors::LIGHT_YELLOW, false);
                object.item = Some(Item::Lightning);
                object
            } else if dice < 0.7 + 0.1 + 0.1 {
                let mut object = object::Object::new(x, y, '#', "molotov cocktail",
                                             colors::LIGHT_YELLOW, false);
                object.item = Some(Item::Fireball);
                object
            } else {
                let mut object = object::Object::new(x, y, '#', "scroll of confusion",
                                             colors::LIGHT_YELLOW, false);
                object.item = Some(Item::Confuse);
                object
            };
            objects.push(item);
        }
    }
}

pub fn make_map(objects: &mut Vec<object::Object>) -> Map {
    let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize];
                       MAP_WIDTH as usize];
    let mut rooms = vec![];
    assert_eq!(&objects[consts::PLAYER] as *const _, &objects[0] as *const _);
    objects.truncate(1);

    for _ in 0..MAX_ROOMS {
        let w = rand::thread_rng().gen_range(ROOM_MIN_SIZE,
                                             ROOM_MAX_SIZE + 1);
        let h = rand::thread_rng().gen_range(ROOM_MIN_SIZE,
                                             ROOM_MAX_SIZE + 1);
        let x = rand::thread_rng().gen_range(0, MAP_WIDTH - w);
        let y = rand::thread_rng().gen_range(0, MAP_HEIGHT - h);

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
    let stairs = object::Object::new(last_room_x, last_room_y, '>', "stairs up",
                             colors::WHITE, false);
    objects.push(stairs);
    map
}