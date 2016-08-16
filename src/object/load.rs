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
    base_chance: u32,
    context: String,
    fighter: Option<String>,
    ai: Option<String>,
    function: Option<String>,
    inventory: Option<Vec<String>>,
    object_type: String,
}

#[derive(Debug, RustcDecodable)]
struct JsonObjectType {
    type_name: String,
    classes: Vec<JsonObjectClass>
}

pub struct ObjectTypes {
    types: HashMap<String, Vec<Weighted<object::ObjectClass>>>
}

impl ObjectTypes {

    pub fn new(object_type: object::ObjectType) -> Self {
        let mut types = HashMap::new();
        let type_groups = match object_type {
            object::ObjectType::Actor => object::ACTOR_TYPES,
            object::ObjectType::Item => object::ITEM_TYPES
        };
        for type_ in type_groups {
            let weighted_objects: Vec<Weighted<object::ObjectClass>> = vec![];
            types.insert(type_.to_string(), weighted_objects);
        }
        ObjectTypes{
            types: types
        }
    }
    pub fn add_class(&mut self, base_chance: u32, type_name: &str,
                     object_class: object::ObjectClass) {
        let weighted_object = Weighted {weight: base_chance,
                                         item: object_class};
        self.types.get_mut(type_name).unwrap().push(weighted_object);
    }

    pub fn get_object(&mut self, type_name: &str) -> object::Object {
        // This is a big mess that I just landed on after hours of kicking and
        // screaming. I don't understand why WeightedChoice needs a mutable
        // reference to the array of Weighted objects, but there it is.

        let mut object_classes: &mut Vec<Weighted<object::ObjectClass>> =
            self.types.get_mut(type_name).unwrap();
        let wc = WeightedChoice::new(object_classes);
        let mut rng = rand::thread_rng();
        let object_class = wc.ind_sample(&mut rng);
        object::Object{
            x: 0,
            y: 0,
            symbol: object_class.symbol,
            color: object_class.color,
            name: object_class.name.into(),
            blocks: object_class.blocks,
            blocks_view: object_class.blocks_view,
            alive: object_class.alive,
            fighter: object_class.fighter,
            ai: object_class.ai,
            function: object_class.function,
            inventory: object_class.inventory,
        }
    }
}

pub fn load_objects(filename: &str, object_type: object::ObjectType) ->
    Result<ObjectTypes, Box<Error>>
{

    let mut json = String::new();
    let mut file = File::open(filename).unwrap();
    file.read_to_string(&mut json).unwrap();
    let types: Vec<JsonObjectType> = json::decode(&json).unwrap();
    let mut return_val = ObjectTypes::new(object_type);
    for type_list in types {
        let type_name = &type_list.type_name;
        for item in type_list.classes {
            let (r, g, b) = item.color;
            let color = Color::new(r, g, b);
            let blocks = get_blocks(&item.blocks);
            let blocks_view = get_blocks(&item.blocks_view);
            let ai = get_ai(item.ai);
            let function = get_function(item.function);
            let fighter = get_fighter(item.fighter);
            let new_item = object::ObjectClass::new(item.symbol, &item.name,
                                                    &item.description,
                                                    &item.context, color,
                                                    blocks, blocks_view,
                                                    item.alive, fighter, ai,
                                                    function, None
            );
            return_val.add_class(item.base_chance, &type_name, new_item);
        }
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
