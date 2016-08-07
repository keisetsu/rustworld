extern crate rustc_serialize;

extern crate tcod;

use tcod::colors::{self, Color};
use tcod::console::{
    BackgroundFlag,
    Console
};

pub mod actor;
pub mod item;

use ai::Ai;
use log::{self, MessageLog};

#[derive(Debug, RustcEncodable, RustcDecodable)]
pub struct Object {
    pub x: i32,
    pub y: i32,
    pub symbol: char,
    pub color: Color,
    pub name: String,
    pub blocks: bool,
    pub alive: bool,
    pub fighter: Option<actor::Fighter>,
    pub ai: Option<Ai>,
    pub item: Option<item::Item>,
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
