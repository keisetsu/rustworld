
// use std::collections::BTreeMap;
use rustc_serialize::json::Json;
use std::fs::File;
use std::error::Error;
use std::io::Read;

pub fn load_objects(filename: &str) {

    let mut json = String::new();
    let mut file = File::open(filename).unwrap();
    file.read_to_string(&mut json).unwrap();
    // let result = try! { json::decode::<(Vec<Object>, Game)>(&json_save_state) };
    let objects_json = Json::from_str(&json).unwrap();

    let objects = objects_json.as_object().unwrap();

    for (object_id, object_json) in objects {
        let object_hash = object_json.as_object().unwrap();
        println!("{}", object_id);
        for (key, value) in object_hash {
            println!("  {}: {}", key, value);
        }
    }
}
