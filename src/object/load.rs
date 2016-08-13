
// use std::collections::BTreeMap;
use rustc_serialize::json::Json;
use std::fs::File;
use std::error::Error;
use std::io::Read;

pub fn load_objects(filename: &str) -> Result<(), Box<Error>> {

    let mut json = String::new();
    let mut file = try! { File::open(filename) };
    try! { file.read_to_string(&mut json) };
    let objects_json = try! {Json::from_str(&json)};
    let objects = objects_json.as_object().unwrap();

    for (object_id, object_json) in objects {
        let object_hash = object_json.as_object().unwrap();
        println!("{}", object_id);
        for (key, value) in object_hash {
            println!("  {}: {}", key, value);
        }
    }
    Ok(())
}
