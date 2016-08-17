use tcod::colors::Color;
// use std::collections::BTreeMap;
use rustc_serialize::json::{self, Json};
use std::fs::File;
use std::error::Error;
use std::io::Read;
use std::collections::HashMap;
use rand;
use rand::distributions::{Weighted, WeightedChoice, IndependentSample};

use ai;
use object;
use object::item;
use object::actor;

#[derive(Debug, RustcDecodable)]
struct JsonObjectClass {
    name: String,
    description: String,
    blocks: String,
    blocks_view: String,
    symbol: char,
    color: (u8, u8, u8),
    alive: bool,
    chance: u32,
    context: String,
    fighter: Option<String>,
    ai: Option<String>,
    function: Option<String>,
    inventory: Option<Vec<String>>,
    object_type: String,
}

#[derive(Debug)]
pub struct ObjectTypes {
    object_category: object::ObjectCategory,
    by_type: HashMap<String, Vec<object::ObjectClass>>,
    by_name: HashMap<String, object::ObjectClass>,
}

impl ObjectTypes {
    pub fn new(object_category: object::ObjectCategory) -> Self {
        let mut by_name = HashMap::new();
        let mut by_type = HashMap::new();
        let types = match object_category {
            object::ObjectCategory::Actor => object::ACTOR_TYPES,
            object::ObjectCategory::Item => object::ITEM_TYPES
        };

        for type_ in types {
            by_type.insert(type_.to_string(), vec![]);
        }

        ObjectTypes{
            object_category: object_category,
            by_type: by_type,
            by_name: by_name,
        }
    }
    pub fn add_class(&mut self, object_type: String,
                     object_class: object::ObjectClass) {
        println!("{:?}", object_class);
        self.by_name.insert(object_class.name.to_string(), object_class.clone());
        self.by_type.get_mut(&object_type).unwrap().push(object_class.clone());
    }

    pub fn get(&self, object_name: &str) -> object::Object {
        let object_class = self.by_name.get(object_name).unwrap();
        object::Object::from_class(object_class)
    }

    pub fn get_random(&mut self, type_name: &str) -> Option<object::Object> {
        if let Some(classes) = self.by_type.get(type_name) {
            let mut weighted = vec![];
            for class in classes {
                weighted.push(Weighted{weight: class.chance,
                                       item: class});
            }
            let wc = WeightedChoice::new(&mut weighted);
            let mut rng = rand::thread_rng();
            let object_class = wc.ind_sample(&mut rng);
            return Some(object::Object::from_class(object_class));
        }
        None
    }
}

pub fn load_objects(filename: &str, object_category: object::ObjectCategory) ->
    Result<ObjectTypes, Box<Error>>
{

    let mut json = String::new();
    let mut file = File::open(filename).unwrap();
    file.read_to_string(&mut json).unwrap();
    let classes: Vec<JsonObjectClass> = json::decode(&json).unwrap();
    let mut return_val = ObjectTypes::new(object_category);
    for class in classes {
        let (r, g, b) = class.color;
        let color = Color::new(r, g, b);
        let blocks = get_blocks(&class.blocks);
        let blocks_view = get_blocks(&class.blocks_view);
        let ai = get_ai(class.ai);
        let function = get_function(class.function);
        let fighter = get_fighter(class.fighter);
        let new_class = object::ObjectClass{
            ai: ai,
            alive: class.alive,
            chance: class.chance,
            blocks: blocks,
            blocks_view: blocks_view,
            color: color,
            context: class.context,
            description: class.description,
            fighter: fighter,
            function: function,
            inventory: None,
            name: class.name,
            object_type: class.object_type.clone(),
            symbol: class.symbol,
        };
        return_val.add_class(class.object_type, new_class);
    }
    Ok(return_val)
}

fn get_ai(ai_option: Option<String>) -> Option<ai::Ai> {
    if let Some(ai) = ai_option  {
        match ai.as_str() {
            "basic" => return Some(ai::Ai::Basic),
            "chrysalis" => return Some(ai::Ai::Chrysalis),
            _ => return None
        }
    }
    None
}

fn get_function(function_option: Option<String>) -> Option<item::Function> {
    if let Some(function) = function_option {
        match function.as_str() {
            "fireball" => return Some(item::Function::Fireball),
            "heal" => return Some(item::Function::Heal),
            _ => return None
        }
    }
    None
}

fn get_blocks(blocks: &str) -> object::Blocks {
    match blocks {
        "no" => object::Blocks::No,
        "half" => object::Blocks::Half,
        "full" => object::Blocks::Full,
        _ => unreachable!()
    }
}

fn get_fighter(fighter: Option<String>) -> Option<actor::Fighter> {
    None
}
