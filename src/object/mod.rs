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

#[derive(Debug)]
pub enum ObjectCategory{
    Actor,
    Item
}

#[derive(Debug, Clone)]
pub struct ObjectClass {
    pub ai: Option<Ai>,
    pub alive: bool,
    pub blocks: Blocks,
    pub blocks_view: Blocks,
    pub can_pick_up: bool,
    pub chance: u32,
    pub color: Color,
    pub context: String,
    pub description: String,
    pub fighter: Option<actor::Fighter>,
    pub function: Option<item::Function>,
    pub inventory: Option<Vec<Object>>,
    pub name: String,
    pub object_type: String,
    pub symbol: char,
}

impl ObjectClass {
    pub fn create_object(&self) -> Object {
        Object{
            ai: self.ai.clone(),
            alive: self.alive,
            blocks: self.blocks,
            blocks_view: self.blocks_view,
            can_pick_up: self.can_pick_up,
            color: self.color,
            fighter: self.fighter,
            function: self.function,
            inventory: self.inventory.clone(),
            name: self.name.to_string(),
            object_type: self.object_type.to_string(),
            symbol: self.symbol,
            x: 0,
            y: 0,
        }
    }
}

#[derive(Debug, Clone, RustcEncodable, RustcDecodable)]
pub struct Object {
    pub ai: Option<Ai>,
    pub alive: bool,
    pub blocks: Blocks,
    pub blocks_view: Blocks,
    pub can_pick_up: bool,
    pub color: Color,
    pub fighter: Option<actor::Fighter>,
    pub function: Option<item::Function>,
    pub inventory: Option<Vec<Object>>,
    pub name: String,
    pub object_type: String,
    pub symbol: char,
    pub x: i32,
    pub y: i32,
}

impl Object {
    pub fn new(x: i32, y: i32, symbol: char, name: &str,
               can_pick_up: bool,
               color: Color, blocks: Blocks,
               blocks_view: Blocks) -> Self {
        Object {
            ai: None,
            alive: false,
            blocks: blocks,
            blocks_view: blocks_view,
            can_pick_up: can_pick_up,
            color: color,
            fighter: None,
            function: None,
            inventory: None,
            name: name.into(),
            object_type: "".into(),
            symbol: symbol,
            x: x,
            y: y,
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

    pub fn take_damage(&mut self, damage: i32, log: &mut log::Messages) {
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
                fighter.on_death.callback(self, log);
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
