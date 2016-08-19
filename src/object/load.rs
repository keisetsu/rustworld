use tcod::colors::Color;
// use std::collections::BTreeMap;
use rustc_serialize::json::{self, Json};
use std::fs::File;
use std::error::Error;
use std::io::Read;
use std::collections::HashMap;
use rand;
use rand::distributions::{Weighted, IndependentSample};

use ai;
use object;
use object::item;
use object::actor;
use util::owned_weighted_choice::OwnedWeightedChoice;

#[derive(Debug, RustcDecodable)]
struct JsonObjectClass {
    name: String,
    description: String,
    blocks: object::Blocks,
    blocks_view: object::Blocks,
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

#[derive(Debug, RustcDecodable)]
struct JsonObjectTypes {
    types: Vec<String>,
    classes: Vec<JsonObjectClass>,
}

#[derive(Debug)]
pub struct ObjectTypes {
    object_types: Vec<String>,
    by_type: HashMap<String, Vec<object::ObjectClass>>,
    by_name: HashMap<String, object::ObjectClass>,
}

impl ObjectTypes {
    pub fn new(object_types: Vec<String>) -> Self {
        let mut by_name = HashMap::new();
        let mut by_type = HashMap::new();
        let mut types_list = vec![];
        for type_ in object_types {
            let type_string = type_.to_string();
            types_list.push(type_string.clone());
            by_type.insert(type_string, vec![]);
        }
        ObjectTypes{
            object_types: types_list,
            by_type: by_type,
            by_name: by_name,
        }
    }
    pub fn add_class(&mut self, object_type: String,
                     object_class: object::ObjectClass) {
        self.by_name.insert(object_class.name.to_string(), object_class.clone());
        self.by_type.get_mut(&object_type).unwrap().push(object_class.clone());
    }

    pub fn get_class(&self, class_name: &str) -> object::ObjectClass {
        let some_class = self.by_name.get(class_name).unwrap();
        some_class.clone()
    }

    pub fn create_randomizer(&self, type_name: &str) ->
        Option<ObjectRandomizer> {
        if let Some(classes) = self.by_type.get(type_name) {
            return Some(ObjectRandomizer::new(classes));
        }
        None
    }
}

pub struct ObjectRandomizer {
    weighted_choice: OwnedWeightedChoice<object::ObjectClass>,
    rng: rand::ThreadRng
}

impl ObjectRandomizer {
    fn new(classes: &Vec<object::ObjectClass>) -> Self {
        let mut weighted = vec![];
        for class in classes {
            weighted.push(Weighted{weight: class.chance,
                                   item: class.clone()});
        }
        let mut rng = rand::thread_rng();
        ObjectRandomizer{
            weighted_choice: OwnedWeightedChoice::new(weighted),
            rng: rng,
        }
    }
    pub fn get_class(&mut self) -> object::ObjectClass {
        self.weighted_choice.ind_sample(&mut self.rng)
    }
}

pub fn load_objects(filename: &str) ->
    Result<ObjectTypes, Box<Error>>
{

    let mut json = String::new();
    let mut file = File::open(filename).unwrap();
    file.read_to_string(&mut json).unwrap();
    let types_from_json: JsonObjectTypes = json::decode(&json).unwrap();
    let types = types_from_json.types;
    let classes = types_from_json.classes;
    let mut return_val = ObjectTypes::new(types);
    for class in classes {
        let (r, g, b) = class.color;
        let color = Color::new(r, g, b);
        // let blocks = get_blocks(&class.blocks);
        // let blocks_view = get_blocks(&class.blocks_view);
        let ai = get_ai(class.ai);
        let function = get_function(class.function);
        let fighter = get_fighter(class.fighter);
        let new_class = object::ObjectClass{
            ai: ai,
            alive: class.alive,
            chance: class.chance,
            blocks: class.blocks,
            blocks_view: class.blocks_view,
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
