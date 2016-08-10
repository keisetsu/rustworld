use std::cmp;

use rand::{self, Rng};

use tcod::colors;
use tcod::bsp::{Bsp, TraverseOrder};

use consts;
use object::{actor, Object};
use object::item::Item;
use ai::Ai;

pub const MAP_WIDTH: i32 = 80;
pub const MAP_HEIGHT: i32 = 43;

pub const FLOOR_WIDTH: i32 = 20;
pub const FLOOR_HEIGHT: i32 = 20;

pub const ROOM_MAX_SIZE: i32 = 10;
pub const ROOM_MIN_SIZE: i32 = 6;
pub const MAX_ROOMS: i32 = 30;

pub const MAX_ROOM_MONSTERS: i32 = 3;
pub const MAX_ROOM_ITEMS:i32 = 4;

#[derive(Debug, RustcEncodable, RustcDecodable)]
pub struct Tile {
    pub explored: bool,
    pub items: Vec<Object>,
}

impl Tile {
    pub fn new(x: i32, y: i32) -> Self {
        let concrete = Object::new(x, y, ' ', "concrete floor",
                                   colors::GREY, false, false);
        Tile{ explored: false, items: vec![concrete],}
    }

    pub fn wall(x: i32, y: i32) -> Self {
        let mut tile = Tile::new(x, y);
        let drywall = Object::new(x, y, ' ', "drywall",
                                   colors::DARKEST_GREY, true, true);
        tile.items.push(drywall);
        tile
    }
//     objects.iter().any(|object| {
//         object.blocks && object.pos() == (x, y)
//     })

    pub fn is_blocked(&self) -> bool {
        self.items.iter().any(|item| {
            item.blocks
        })
    }

    pub fn blocks_view(&self) -> bool {
        self.items.iter().any(|item| {
            item.blocks_view
        })
    }
}

pub type Map = Vec<Vec<Tile>>;
//pub type Floor = Vec<Vec<Vec<Object>>>;

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

// pub fn is_blocked(x: i32, y: i32, map: &Map, objects: &[Object]) -> bool {
//     // first test the map tile
//     if map[x as usize][y as usize].is_blocked() {
//         return true;
//     }
//     // now check for any blocking objects
//     objects.iter().any(|object| {
//         object.blocks && object.pos() == (x, y)
//     })
// }

// fn create_room(room: Rect, map: &mut Map) {
//     for x in (room.x1 + 1)..room.x2 {
//         for y in (room.y1 + 1)..room.y2 {
//             map[x as usize][y as usize] = Tile::empty();
//         }
//     }
// }
fn create_room2(room: &mut Bsp, floor: &mut Map) {
    for x in (room.x + 1)..room.x + room.w {
        for y in (room.y + 1)..room.y + room.h {
            floor[x as usize][y as usize] = Tile::new(x, y);
        }
    }
}

fn create_h_tunnel(x1: i32, x2: i32, y: i32, map: &mut Map) {
    for x in cmp::min(x1, x2)..(cmp::max(x1, x2) + 1) {
        map[x as usize][y as usize] = Tile::new(x, y);
    }
}

fn create_v_tunnel(y1: i32, y2: i32, x: i32, map: &mut Map) {
    for y in cmp::min(y1, y2)..(cmp::max(y1, y2) + 1) {
        map[x as usize][y as usize] = Tile::new(x, y);
    }
}

fn vline_up(x: i32, y: i32, floor: &mut Map){
    let mut new_y = y;
    while new_y >= 1 &&
        floor[x as usize][new_y as usize].is_blocked() == true {
        floor[x as usize][new_y as usize] = Tile::new(x, new_y);
        new_y -= 1;
    }
}

fn vline_down(x: i32, y: i32, floor: &mut Map){
    let mut new_y = y;
    while new_y < FLOOR_HEIGHT  - 1&&
        floor[x as usize][new_y as usize].is_blocked() == true {
        floor[x as usize][new_y as usize] = Tile::new(x, new_y);
        new_y += 1;
    }
}

fn hline_left(x: i32, y: i32, floor: &mut Map) {
    let mut new_x = x;
    while new_x >= 1 &&
        floor[new_x as usize][y as usize].is_blocked() == true {
        floor[new_x as usize][y as usize] = Tile::new(new_x, y);
        new_x -= 1
    }
}

fn hline_right(x: i32, y: i32, floor: &mut Map) {
    let mut new_x = x;
    while new_x < FLOOR_WIDTH - 1 &&
        floor[new_x as usize][y as usize].is_blocked() == true {
        floor[new_x as usize][y as usize] = Tile::new(new_x, y);
        new_x += 1
    }
}



// fn place_objects(room: Rect, map: &Map, objects: &mut Vec<Object>) {
//     let num_monsters = rand::thread_rng().gen_range(0, MAX_ROOM_MONSTERS + 1);

//     for _ in 0..num_monsters {
//         let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
//         let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);
//         if !map[x as usize][ y as usize].is_blocked() {
//             let mut monster = if rand::random::<f32>() < 0.8 {
//                 let mut c_zombie = Object::new(x, y, 'Z', "Chrysalis zombie",
//                                           colors::DESATURATED_GREEN, true, false);
//                 c_zombie.fighter = Some(actor::Fighter{
//                     max_hp: 10, hp: 10, defense: 0, power: 3,
//                     on_death: actor::DeathCallback::Monster,
//                 });
//                 c_zombie.ai = Some(Ai::Chrysalis);
//                 c_zombie
//             } else {
//                 let mut zombie = Object::new(x, y, 'Z', "runner zombie",
//                                             colors::DARKER_GREEN, true, false);
//                 zombie.fighter = Some(actor::Fighter{
//                     max_hp: 16, hp: 16, defense: 1, power: 4,
//                     on_death: actor::DeathCallback::Monster,
//                 });
//                 zombie.ai = Some(Ai::Basic);
//                 zombie
//             };
//             monster.alive = true;
//             objects.push(monster);
//         }
//     }

//     let num_items = rand::thread_rng().gen_range(0, MAX_ROOM_ITEMS + 1);

//     for _ in 0..num_items {
//         let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
//         let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

//         if !map[x as usize][y as usize].is_blocked() {
//             let dice = rand::random::<f32>();
//             let item = if dice < 0.7 {
//                 let mut object = Object::new(x, y, '!', "First aid kit",
//                                              colors::VIOLET, false);
//                 object.item = Some(Item::Heal);
//                 object
//             } else if dice < 0.7 + 0.1 {
//                 let mut object = Object::new(x, y, '#',
//                                              "scroll of lightning bolt",
//                                              colors::LIGHT_YELLOW, false);
//                 object.item = Some(Item::Lightning);
//                 object
//             } else if dice < 0.7 + 0.1 + 0.1 {
//                 let mut object = Object::new(x, y, '#', "molotov cocktail",
//                                              colors::LIGHT_YELLOW, false);
//                 object.item = Some(Item::Fireball);
//                 object
//             } else {
//                 let mut object = Object::new(x, y, '#', "scroll of confusion",
//                                              colors::LIGHT_YELLOW, false);
//                 object.item = Some(Item::Confuse);
//                 object
//             };
//             objects.push(item);
//         }
//     }
// }

fn traverse_node(node: &mut Bsp, mut floor: &mut Map) -> bool {
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

        create_room2(node, floor);
    } else {
        if let (Some(left), Some(right)) = (node.left(), node.right()) {
            node.x = cmp::min(left.x, right.x);
            node.y = cmp::min(left.y, right.y);
            node.w = cmp::max(left.x + left.w, right.x + right.w) - node.x;
            node.h = cmp::max(left.y + left.h, right.y + right.h) - node.y;
            if node.horizontal() {
                if left.x + left.w - 1 < right.x ||
                    right.x + right.w - 1 < left.x {
                        let x1 = rand::thread_rng()
                            .gen_range(left.x, left.x + left.w - 1);
                        let x2 = rand::thread_rng()
                            .gen_range(right.x, right.x + right.w - 1);
                        let y = rand::thread_rng()
                            .gen_range(left.y + left.h, right.y);
                        vline_up(x1, y - 1, &mut floor);
                        create_h_tunnel(x1, x2, y, &mut floor);
                        vline_down(x2, y + 1, &mut floor);
                    } else {
                        let minx = cmp::max(left.x, right.x);
                        let maxx = cmp::min(left.x + left.w - 1,
                                            right.x + right.w - 1);
                        let x = rand::thread_rng().gen_range(minx, maxx);
                        vline_down(x, right.y, &mut floor);
                        vline_up(x, right.y - 1, &mut floor);
                    }

            } else {
                if left.y + left.h - 1 < right.y ||
                    right.y + right.h - 1 < left.y {
                        let y1 = rand::thread_rng()
                            .gen_range(left.y, left.y + left.h - 1);
                        let y2 = rand::thread_rng()
                            .gen_range(right.y, right.y + right.h - 1);
                        let x = rand::thread_rng()
                            .gen_range(left.x + left.w, right.x);
                        hline_left(x - 1, y1, &mut floor);
                        create_v_tunnel(y1, y2, x, &mut floor);
                        hline_right(x + 1, y2, &mut floor);
                    } else {
                        let miny = cmp::max(left.y, right.y);
                        let maxy = cmp::min(left.y + left.h - 1,
                                            right.y + right.h - 1);
                        let y = rand::thread_rng().gen_range(miny, maxy);
                        hline_left(right.x - 1, y, &mut floor);
                        hline_right(right.x, y, &mut floor);
                    }
            }
        }
    }
    true
}


pub fn make_floor(actors: &mut Vec<Object>) -> Map {
    let mut floor = vec![];
    for x in 0..FLOOR_WIDTH {
        floor.push(vec![]);
        for y in 0..FLOOR_HEIGHT {
            let mut tile: Tile = Tile::wall(x, y);
            floor[x as usize].push(tile);
        }
    }
    println!("{:?}", floor);
    // let mut rooms = vec![];

    let mut bsp = Bsp::new_with_size(0, 0, FLOOR_WIDTH, FLOOR_HEIGHT);
    bsp.split_recursive(None, 10, 6, 6, 1.5, 1.5);
    bsp.traverse(TraverseOrder::InvertedLevelOrder, |node| {
        traverse_node(node, &mut floor)
    });

    floor
}

// pub fn make_map(objects: &mut Vec<Object>) -> Map {
//     make_floor();
//     let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize];
//                        MAP_WIDTH as usize];
//     let mut rooms = vec![];
//     assert_eq!(&objects[consts::PLAYER] as *const _, &objects[0] as *const _);
//     objects.truncate(1);

//     for _ in 0..MAX_ROOMS {
//         let w = rand::thread_rng().gen_range(ROOM_MIN_SIZE,
//                                              ROOM_MAX_SIZE + 1);
//         let h = rand::thread_rng().gen_range(ROOM_MIN_SIZE,
//                                              ROOM_MAX_SIZE + 1);
//         let x = rand::thread_rng().gen_range(0, MAP_WIDTH - w);
//         let y = rand::thread_rng().gen_range(0, MAP_HEIGHT - h);

//         let new_room = Rect::new(x, y, w, h);

//         let failed = rooms.iter().any(|other_room
//                                       |new_room.intersects_with(
//                                           other_room));

//         if !failed {

//             create_room(new_room, &mut map);
//             place_objects(new_room, &map, objects);

//             let (new_x, new_y) = new_room.center();

//             if rooms.is_empty() {
//                 objects[consts::PLAYER].set_pos(new_x, new_y);
//             } else {
//                 let (prev_x, prev_y) =
//                     rooms[rooms.len() - 1].center();

//                 if rand::random() {
//                     create_h_tunnel(prev_x, new_x,
//                                     prev_y, &mut map);
//                     create_v_tunnel(prev_y, new_y,
//                                     prev_x, &mut map);
//                 } else {
//                     create_v_tunnel(prev_y, new_y,
//                                     prev_x, &mut map);
//                     create_h_tunnel(prev_x, new_x,
//                                     prev_y, &mut map);
//                 }
//             }
//             rooms.push(new_room);
//         }

//     }
//     let (last_room_x, last_room_y) = rooms[rooms.len() - 1].center();
//     let stairs = Object::new(last_room_x, last_room_y, '>', "stairs up",
//                              colors::WHITE, false);
//     objects.push(stairs);
//     map
// }
