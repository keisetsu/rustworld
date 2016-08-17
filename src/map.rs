use std::cmp;

use rand::{self, Rng};

use tcod::colors;
use tcod::bsp::{Bsp, TraverseOrder};

use consts;
use object::{self, actor, Object};
use object::item::Function;
use ai::Ai;

pub const MAP_WIDTH: i32 = 80;
pub const MAP_HEIGHT: i32 = 43;

pub const FLOOR_WIDTH: i32 = 30;
pub const FLOOR_HEIGHT: i32 = 30;

pub const ROOM_MAX_SIZE: i32 = 10;
pub const ROOM_MIN_X: i32 = 8;
pub const ROOM_MIN_Y: i32 = 8;
pub const MAX_ROOMS: i32 = 30;

pub const MAX_ROOM_MONSTERS: i32 = 3;
pub const MAX_ROOM_ITEMS:i32 = 4;

#[derive(Debug, RustcEncodable, RustcDecodable)]
pub struct Tile {
    pub floor: Option<Object>,
    pub explored: bool,
    pub items: Vec<Object>,
}

impl Tile {
    pub fn new(x: i32, y: i32) -> Self {
        let concrete = Object::new(x, y, ' ', "concrete floor",
                                   colors::GREY, object::Blocks::No,
                                   object::Blocks::No);
        Tile{
            floor: Some(concrete),
            explored: false,
            items: vec![],}
    }
}

pub fn is_blocked(x: i32, y: i32, map: &Map, actors: &[Object]) -> object::Blocks {
    // Because actors are stored in a separate place from the map, we need
    // to check both for actors marked as being in a place on the map,
    // as well as all objects in the map location to see if they block

    // If only one thing blocks fully we know nothing new can move
    // onto that tile, so we are done. If something only partially blocks, we
    // have to keep checking in case there is something fully blocking.
    let mut blocks = object::Blocks::No;
    for actor in actors {
        if actor.x == x && actor.y == y {
            blocks = cmp::max(blocks, actor.blocks);
            if blocks == object::Blocks::Full {
                return blocks
            }
        }
    }

    for item in &map[x as usize][y as usize].items {
        blocks = cmp::max(blocks, item.blocks);
        if blocks == object::Blocks::Full {
            return blocks
        }
    }
    blocks
}

pub fn blocks_view(x: i32, y: i32, map: &Map, actors: &[Object]) -> object::Blocks {
    // Because actors are stored in a separate place from the map, we need
    // to check both for actors marked as being in a place on the map,
    // as well as all actors in the map location to see if they block

    // If only one thing blocks fully we know nothing can see through that
    // tile, so we are done. If something only partially blocks, we
    // have to keep checking in case there is something fully blocking.
    let mut blocks = object::Blocks::No;
    for actor in actors {
        if actor.x == x && actor.y == y {
            blocks = cmp::max(blocks, actor.blocks_view);
            if blocks == object::Blocks::Full {
                return blocks
            }
        }
    }

    for item in &map[x as usize][y as usize].items {
        blocks = cmp::max(blocks, item.blocks_view);
        if blocks == object::Blocks::Full {
            return blocks
        }
    }
    blocks
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

fn create_room(room: &mut Bsp, floor: &mut Map) {
    for x in (room.x)..room.x + room.w {
        for y in (room.y)..room.y + room.h {
            floor[x as usize][y as usize] = Tile::new(x, y);
        }
    }
}

fn place_objects(actors: &mut Vec<Object>, floor: usize,
                 rooms: Vec<Rect>, map: &mut Map) {
    let mut actor_types = object::load::load_objects(
        "data/objects/actors.json", object::ObjectCategory::Actor).unwrap();
    let mut item_types = object::load::load_objects(
        "data/objects/items.json", object::ObjectCategory::Item).unwrap();

    if floor == 1 {
        let mut stairs = (0, 0);
        for room in &rooms {
            if room.x1 == 1 && room.y1 == 1 {
                make_door(0, room.y2/ 2, map);
                actors[consts::PLAYER].set_pos(1, room.y2 / 2);
            } else if room.y2 == FLOOR_HEIGHT - 1 || room.x2 == FLOOR_WIDTH - 1 {
                if stairs == (0, 0) || rand::random() {
                    let stairs_x = room.x1 + ((room.x2 - room.x1)/2);
                    let stairs_y = room.y1 + ((room.y2 - room.y1)/2);
                    stairs = (stairs_x, stairs_y);
                }
            }
        }
        let (stairs_x, stairs_y) = stairs;
        let mut stairs_up = item_types.get("stairs up");
        stairs_up.set_pos(stairs_x, stairs_y);
        map[stairs_x as usize][stairs_y as usize].items.push(stairs_up);
    }

    for _ in 0..rand::thread_rng().gen_range(1,3) {
        let room = rooms[rand::thread_rng().gen_range(0, rooms.len() - 1)];
        let brick_x = room.x1 + 1;
        let brick_y = room.y1 + 2;
        if let Some(mut brick) = item_types.get_random(
            "environmental weapon") {
            brick.set_pos(brick_x, brick_y);
            map[brick_x as usize][brick_y as usize].items.push(brick);
        };
    }

}
fn make_door(x: i32, y: i32, map: &mut Map) {
    let door = Object::new(x, y, '+', "hardwood door",
                                  colors::SEPIA,
                                  object::Blocks::No,
                                  object::Blocks::Full);
        map[x as usize][y as usize].items[0] = door;
}

fn traverse_node(node: &mut Bsp, mut rooms: &mut Vec<Rect>, mut floor: &mut Map) -> bool {
    if node.is_leaf() {
        let mut minx = node.x + 1;
        let mut maxx = node.x + node.w - 1;
        let mut miny = node.y + 1;
        let mut maxy = node.y + node.h - 1;
        if maxx == FLOOR_WIDTH - 1 {
            maxx -= 1;
        }
        if maxy == FLOOR_HEIGHT - 1 {
            maxy -= 1;
        }
        node.x = minx;
        node.y = miny;
        node.w = maxx - minx + 1;
        node.h = maxy - miny + 1;
        create_room(node, floor);
        rooms.push(Rect::new(node.x, node.y, node.w, node.h));
    } else {
        if let (Some(left), Some(right)) = (node.left(), node.right()) {
            node.x = cmp::min(left.x, right.x);
            node.y = cmp::min(left.y, right.y);
            node.w = cmp::max(left.x + left.w, right.x + right.w) - node.x;
            node.h = cmp::max(left.y + left.h, right.y + right.h) - node.y;
            if node.horizontal() {
                make_door(left.x, cmp::max(left.y, right.y) - 1,
                          &mut floor);
            } else {
                make_door(cmp::max(left.x, right.x) - 1, left.y,
                          &mut floor);
            }
        }
    }
    true
}


pub fn make_floor(mut actors: &mut Vec<Object>) -> Map {
    let mut floor = vec![];
    for x in 0..FLOOR_WIDTH {
        floor.push(vec![]);
        for y in 0..FLOOR_HEIGHT {
            let mut wall_tile: Tile = Tile::new(x, y);
            let brick_wall = Object::new(x, y, ' ', "brick wall",
                                      colors::DARKEST_GREY,
                                      object::Blocks::Full,
                                      object::Blocks::Full);
            wall_tile.items.push(brick_wall);
            floor[x as usize].push(wall_tile);
        }
    }
    let mut rooms = vec![];
    let mut bsp = Bsp::new_with_size(0, 0, FLOOR_WIDTH, FLOOR_HEIGHT);
    bsp.split_recursive(None, 3, ROOM_MIN_X, ROOM_MIN_Y, 1.25, 1.25);
    bsp.traverse(TraverseOrder::InvertedLevelOrder, |node| {
        traverse_node(node, &mut rooms, &mut floor)
    });
    place_objects(&mut actors, 1, rooms, &mut floor);
    floor
}
