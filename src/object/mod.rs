use std::fmt::{self, Debug, Display};

use rustc_serialize;

use tcod::colors::Color;
use tcod::console::{
    BackgroundFlag,
    Console
};

pub mod actor;
pub mod item;
pub mod load;

use ai::Ai;
use log::{self, MessageLog};

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Ord, Eq,
         RustcEncodable, RustcDecodable)]
pub enum Blocks {
    No,
    Half,
    Full
}

pub enum ObjectType{
    Actor,
    Item
}

// #[derive(Debug, PartialEq, Eq, Hash)]
// pub enum ObjectType {
//     ActorZombie,
//     ItemAmmo,
//     ItemDrink,
//     ItemEnvironmental,
//     ItemEnvironmentalWeapon,
//     ItemFood,
//     ItemHealth,
//     ItemMeleeWeapon,
//     ItemRangedWeapon,
// }

// impl fmt::Display for ObjectType {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         fmt::Debug::fmt(self, f)
//     }
// }

const ACTOR_TYPES: &'static [ &'static str ] = &[ "ActorZombie" ];

const ITEM_TYPES: &'static [ &'static str ] = &[
    "ItemAmmo",
    "ItemDrink",
    "ItemEnvironmental",
    "ItemEnvironmentalWeapon",
    "ItemFood",
    "ItemHealth",
    "ItemMeleeWeapon",
    "ItemRangedWeapon",
];

#[derive(Debug, Clone)]
pub struct ObjectClass {
//    pub object_type: String,
    pub symbol: char,
    pub name: String,
    pub description: String,
    pub context: String,
    pub color: Color,
    pub blocks: Blocks,
    pub blocks_view: Blocks,
    pub alive: bool,
    pub fighter: Option<actor::Fighter>,
    pub ai: Option<Ai>,
    pub function: Option<item::Function>,
    pub inventory: Option<Box<Vec<Object>>>,
}

impl ObjectClass {
    pub fn new(symbol: char, name: &str,
               description: &str, context: &str, color: Color,
               blocks: Blocks, blocks_view: Blocks, alive: bool,
               fighter: Option<actor::Fighter>,
               ai: Option<Ai>,
               function: Option<item::Function>,
               inventory: Option<Box<Vec<Object>>>)
               -> Self {
        ObjectClass {
            ai: ai,
            alive: alive,
            blocks: blocks,
            blocks_view: blocks_view,
            color: color,
            context: context.into(),
            description: description.into(),
            fighter: fighter,
            inventory: inventory,
            function: function,
            name: name.into(),
//            object_type: object_type,
            symbol: symbol,
        }
    }

    pub fn create_instance(self, x: i32, y: i32) -> Object {
        Object {
            x: x,
            y: y,
            ai: self.ai,
            alive: self.alive,
            blocks: self.blocks,
            blocks_view: self.blocks_view,
            color: self.color,
            fighter: self.fighter,
            inventory: self.inventory,
            function: self.function,
            name: self.name,
            symbol: self.symbol,
        }
    }
}

#[derive(Debug, Clone, RustcEncodable, RustcDecodable)]
pub struct Object {
    pub x: i32,
    pub y: i32,
    pub symbol: char,
    pub color: Color,
    pub name: String,
    pub blocks: Blocks,
    pub blocks_view: Blocks,
    pub alive: bool,
    pub fighter: Option<actor::Fighter>,
    pub ai: Option<Ai>,
    pub function: Option<item::Function>,
    pub inventory: Option<Box<Vec<Object>>>,
}

impl Object {
    pub fn new(x: i32, y: i32, symbol: char, name: &str,
               color: Color, blocks: Blocks,
               blocks_view: Blocks) -> Self {
        Object {
            x: x,
            y: y,
            symbol: symbol,
            color: color,
            name: name.into(),
            blocks: blocks,
            blocks_view: blocks_view,
            alive: false,
            fighter: None,
            ai: None,
            function: None,
            inventory: None,
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

    pub fn take_damage(&mut self, damage: i32, messages: &mut log::Messages) {
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

    pub fn attack(&mut self, target: &mut Object, log: &mut log::Messages) {
        let damage = self.fighter.map_or(0, |f| f.power) -
            target.fighter.map_or(0, |f| f.defense);
        if damage > 0 {
            log.info(format!("{} attacks {} for {} hit points.", self.name,
                             target.name, damage));
            target.take_damage(damage, log);
        } else {
            log.info(format!("{} attacks {} but whatevs!",
                             self.name, target.name));
        }
    }

}
