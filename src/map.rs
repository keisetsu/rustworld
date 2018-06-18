use std::cmp;

use rand::{self, Rng};

use tcod::bsp::{Bsp, TraverseOrder};

use consts;
use object::{self, actor, Object, ObjectClass};
use object::load::ObjectRandomizer;
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

pub enum BuildingType {
    LargeHome,
    SmallHome,
    Office,
    ApartmentComplex,
    OfficeComplex,
    Store,
}

#[derive(Debug, RustcEncodable, RustcDecodable)]
pub struct Tile {
    pub floor: Object,
    pub explored: bool,
    pub items: Vec<Object>,
}

impl Tile {
    pub fn new(floor: &ObjectClass) -> Self {
        Tile{
            floor: floor.create_object(),
            explored: false,
            items: vec![],}
    }
}

pub fn is_blocked(x: i32, y: i32, map: &Floor, actors: &[Object]) -> object::Blocks {
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

pub fn blocks_view(x: i32, y: i32, map: &Floor, actors: &[Object]) -> object::Blocks {
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

pub struct Floor {
    map: Map,
    rooms: Vec<Rect>,
}

pub type BuildingPlan = Vec<Floor>;

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

fn create_room(room: &mut Bsp, floor_class: &ObjectClass, map: &mut Map) {
    for x in (room.x)..room.x + room.w {
        for y in (room.y)..room.y + room.h {
            map[x as usize][y as usize] = Tile::new(floor_class);
        }
    }
}

fn place_objects(floor: usize, rooms: &Vec<Rect>, map: &mut Map,
                 items: &object::load::ObjectTypes, stairs: (i32, i32))
                 -> (i32, i32) {
    let mut stairs_loc = stairs;
    let mut stairs_rooms = vec![];
    for (room_ix, room) in rooms.iter().enumerate() {
        let ref mut door_randomizer = items.create_randomizer("door").unwrap();
        if room.x1 == 1 && room.y1 == 1 {
            make_door(0, room.y2 / 2, door_randomizer, map);
        } else if stairs == (0, 0) && (room.y2 == FLOOR_HEIGHT - 1 ||
                                       room.x2 == FLOOR_WIDTH - 1) {
            stairs_rooms.push(room_ix);
        }
    }
    if stairs_loc == (0, 0) {
        let stairs_room = rooms[rand::thread_rng()
                                .gen_range(0, stairs_rooms.len())];

        let stairs_x = stairs_room.x1 + ((stairs_room.x2 - stairs_room.x1)/2);
        let stairs_y = stairs_room.y1 + ((stairs_room.y2 - stairs_room.y1)/2);
        stairs_loc = (stairs_x, stairs_y);
    }
    let (stairs_x, stairs_y) = stairs_loc;

    let mut stairs_key = "stairs";
    if floor == 1{
        stairs_key = "stairs up";
    }

    let mut stairs_obj = items.get_class(stairs_key).create_object();
    stairs_obj.set_pos(stairs_x, stairs_y);
    println!("{:?}", stairs_obj);
    map[stairs_x as usize][stairs_y as usize].items.push(stairs_obj);

    for _ in 0..rand::thread_rng().gen_range(1,3) {
        let room = rooms[rand::thread_rng().gen_range(0, rooms.len())];
        let brick_x = room.x1 + 1;
        let brick_y = room.y1 + 2;
        if let Some(ref mut brick_random) = items.create_randomizer(
            "environmental weapon") {
            let brick_class = &mut brick_random.get_class();
            let mut brick = brick_class.create_object();
            brick.set_pos(brick_x, brick_y);
            map[brick_x as usize][brick_y as usize].items.push(brick);
        };

    }

    stairs_loc
}

fn place_actors(floor: usize, rooms: &Vec<Rect>, map: &mut Map,
                actor_types: &object::load::ObjectTypes,
                actors: &mut Vec<Object>) {
    for room in rooms {
        if room.x1 == 1 && room.y1 == 1 {
            actors[consts::PLAYER].set_pos(1, room.y2 / 2);
        }
    }

    // for _ in 0..rand::thread_rng().gen_range(1,2) {
    //     let room = rooms[rand::thread_rng().gen_range(0, rooms.len())];
    //     let x = rand::thread_rng().gen_range(room.x1+1, room.x2);
    //     let y = rand::thread_rng().gen_range(room.y1+1, room.y2);
    //     if let Some(ref mut zombie_random) = actor_types.create_randomizer(
    //         "zombie") {
    //         let zombie_class = &mut zombie_random.get_class();
    //         let mut zombie = zombie_class.create_object();
    //         zombie.set_pos(x, y);
    //         actors.push(zombie);
    //     }
    // }
}

fn make_door(x: i32, y: i32, door_randomizer: &mut ObjectRandomizer,
             map: &mut Map) {
    let door_class = door_randomizer.get_class();
    let mut door = door_class.create_object();
    door.set_pos(x, y);
    map[x as usize][y as usize].items[0] = door;
}

fn traverse_node(node: &mut Bsp, rooms: &mut Vec<Rect>,
                 // object_types: &object::load::ObjectTypes,
                 floor_type: &ObjectClass,
                 mut map: &mut Map) -> bool {
    if node.is_leaf() {
        let minx = node.x + 1;
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
        create_room(node, floor_type, map);
        rooms.push(Rect::new(node.x, node.y, node.w, node.h));
    } else {
        if let (Some(left), Some(right)) = (node.left(), node.right()) {
            node.x = cmp::min(left.x, right.x);
            node.y = cmp::min(left.y, right.y);
            node.w = cmp::max(left.x + left.w, right.x + right.w) - node.x;
            node.h = cmp::max(left.y + left.h, right.y + right.h) - node.y;
            // let ref mut door_randomizer = object_types.create_randomizer("door")
            //     .unwrap();
            // if node.horizontal() {
            //     make_door(left.x, cmp::max(left.y, right.y) - 1,
            //               door_randomizer, &mut map);
            // } else {
            //     make_door(cmp::max(left.x, right.x) - 1, left.y,
            //               door_randomizer, &mut map);
            // }
        }
    }
    true
}

fn make_floor(floor_class: &object::ObjectClass,
              wall_class: &object::ObjectClass) -> Floor {
    let mut floor = vec![];
    for x in 0..FLOOR_WIDTH {
        floor.push(vec![]);
        for y in 0..FLOOR_HEIGHT {
            let mut wall_tile: Tile = Tile::new(&floor_class);
            let mut wall = wall_class.create_object();
            wall.set_pos(x, y);
            wall_tile.items.push(wall);
            floor[x as usize].push(wall_tile);
        }
    }
    let mut rooms = vec![];
    let mut bsp = Bsp::new_with_size(0, 0, FLOOR_WIDTH, FLOOR_HEIGHT);
    bsp.split_recursive(None, 3, ROOM_MIN_X, ROOM_MIN_Y, 1.25, 1.25);
    bsp.traverse(TraverseOrder::InvertedLevelOrder, |node| {
        traverse_node(node, &mut rooms, &floor_class, &mut floor)
    });

    Floor{
        rooms: rooms,
        map: floor,
    }
}

pub fn make_suburban_home(item_types: &object::load::ObjectTypes,
                          actor_types: &object::load::ObjectTypes,
                          mut actors: &mut Vec<Object>) -> BuildingPlan {
    let wall_class = item_types.get_class("drywall");
    let carpet = item_types.get_class("carpet");
    let mut stairs = (0, 0);
    let map: BuildingPlan = vec![];
    for level in 2..rand::thread_rng().gen_range(2,3) {
        let mut floor = make_floor(&carpet, &wall_class);
        place_objects(level, &mut map, &item_types, &stairs);
        place_actors(level, &mut map, &actor_types, &mut actors);
        map.push(floor);
    }
    map
}

pub fn make_map(mut actors: &mut Vec<Object>) -> BuildingPlan {
    let actor_types = object::load::load_objects(
        "data/objects/actors.json").unwrap();
    let item_types = object::load::load_objects(
        "data/objects/items.json").unwrap();
    make_suburban_home(&item_types, &actor_types, actors)
}
