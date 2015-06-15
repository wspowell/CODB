extern crate rustc_serialize;
extern crate bincode;

use std::fs;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::{BufReader};
use std::io::prelude::*;
use std::os::unix;
use std::path::Path;

use std::collections::HashMap;

use bincode::SizeLimit;

use rustc_serialize::json;
use rustc_serialize::base64;
use rustc_serialize::base64::{ToBase64, FromBase64};
use rustc_serialize::{Encodable, Decodable};

trait DBVariable {
	/// Loads the variable instance from the database file
	fn load<T>(var: T, instance_id: usize) -> Result<T, String> where T: DBVariable + Decodable {
		let filename = format!("database/{}_{}.db", var.name(), instance_id);		
		let mut options = OpenOptions::new();
		let path = Path::new(&filename);

		let mut file = match options.open(path) {
			Ok(file) => file,
			Err(error)  => panic!("Could not open file: {}", error)
		};

		// Read the file contents into a string
		let mut data = String::new();
		match file.read_to_string(&mut data) {
			Err(error) => panic!("Could not open file: {}", error),
			_ => ()
		}

		let decoded: T = match bincode::decode(&data.as_bytes()) {
			Ok(d) => d,
			Err(e) => T::new()
		};
		Ok(decoded)
	}
	
	/// Saves the struct data into the variable instance in the database file.
	fn save<T>(var: T, instance_id: usize) -> Result<(), String> where T: DBVariable + Encodable {
		let filename = format!("database/{}_{}.db", var.name(), instance_id);		
		let path = Path::new(&filename);

		let mut file = match File::create(path) {
			Ok(file) => file,
			Err(error)  => panic!("Could not open file: {}", error)
		};
	
		// save definitions back to file
		match bincode::encode_into(&var, &mut file, bincode::SizeLimit::Infinite) {
			Ok(d) => d,
			Err(e) => panic!("{}", e)
		};
	
		Ok(())
	}


	fn new() -> Self;
	
	/// Returns the name of the variable
	fn name(&self) -> &'static str;
}

/// The Instance is filled with data given a resource name and instance id.
struct Instance {
	data: HashMap<String, Box<DBVariable>>
}

impl Instance {
	fn new(resource_name: &str, instance_id: usize) -> Result<Instance, String> {
		let mut data: HashMap<String, Box<DBVariable>> = HashMap::new();

		// open resource definition file
		let definitions = ResourceDefinition::read();		

		// get variable IDs associated with the resource
		let resource_def = definitions.get(resource_name);

		if resource_def == None {
			panic!("No definition for resource: {}", resource_name);
		}

		let references = &resource_def.unwrap().variables_referenced;

		// open each variable file and find each instance
		for reference in references {
			
		}

		Ok(Instance {
			data: data
		})
	}
}

#[derive(RustcDecodable, RustcEncodable, PartialEq)]
struct ResourceDefinition {
	resource_name: String,
	variables_referenced: Vec<String>
}

impl ResourceDefinition {
	pub fn read() -> HashMap<String, ResourceDefinition> {
		let mut options = OpenOptions::new();
		let path = Path::new("database/resource_definition.db");

		let mut file = match options.open(path) {
			Ok(file) => file,
			Err(error)  => panic!("Could not open file: {}", error)
		};

		// Read the file contents into a string
		let mut data = String::new();
		match file.read_to_string(&mut data) {
			Err(error) => panic!("Could not open file: {}", error),
			_ => ()
		}

		let decoded: HashMap<String, ResourceDefinition> = match bincode::decode(&data.as_bytes()) {
			Ok(d) => d,
			Err(e) => HashMap::new() // FIXME: could potentially lose all data if error occurs
		};
		decoded
	}

	fn write(definitions: &HashMap<String, ResourceDefinition>) {
		//let mut options = OpenOptions::new();
		//options.write(true).append(false);
		let path = Path::new("database/resource_definition.db");

		let mut file = match File::create(path) {
			Ok(file) => file,
			Err(error)  => panic!("Could not open file: {}", error)
		};
	
		// save definitions back to file
		match bincode::encode_into(&definitions, &mut file, bincode::SizeLimit::Infinite) {
			Ok(d) => d,
			Err(e) => panic!("{}", e)
		};
	}

	pub fn add(definition: ResourceDefinition) {
		// get the previous definitions
		let mut definitions = ResourceDefinition::read();
		// add/edit new definition
		definitions.insert(definition.resource_name.to_string(), definition); 
		// save definitions back to file
		ResourceDefinition::write(&definitions);
	}

	pub fn remove(resource_name: &str) {
		// get the previous definitions
		let mut definitions = ResourceDefinition::read();
		// add/edit new definition
		definitions.remove(resource_name); 
		// save definitions back to file
		ResourceDefinition::write(&definitions);
	}
}

#[test]
fn test_resource_definition() {
	let mut variables_referenced: Vec<String> = Vec::new();
	variables_referenced.push("username".to_string());
	variables_referenced.push("password".to_string());
	let login = ResourceDefinition {
		resource_name: "login".to_string(),
		variables_referenced: variables_referenced
	};

	let mut variables_referenced2: Vec<String> = Vec::new();
	variables_referenced2.push("resource_name".to_string());
	variables_referenced2.push("variables_referenced".to_string());
	let sw_admin = ResourceDefinition {
		resource_name: "sw_admin".to_string(),
		variables_referenced: variables_referenced2
	};

	ResourceDefinition::add(login);
	ResourceDefinition::add(sw_admin);

	assert!(ResourceDefinition::read().contains_key("login"));
	assert!(ResourceDefinition::read().contains_key("sw_admin"));

	ResourceDefinition::remove("login");
	ResourceDefinition::remove("sw_admin");

	assert!(!ResourceDefinition::read().contains_key("login"));
	assert!(!ResourceDefinition::read().contains_key("sw_admin"));
}

/*
#[test]
fn it_works() {
	let object: TestStruct = TestStruct {
        data_int: 1,
        data_str: "homura".to_string(),
        data_vector: vec![2,3,4,5],
    };

	let object2: TestStruct2 = TestStruct2 {
		data_int: 1,
        data_str: "homura".to_string(),
        data_vector: vec![2,3,4,5],
    };

	println!("{} {}", object.name(), object2.name());
	

	// Serialize using `json::encode`
    let encoded = json::encode(&object).unwrap();

	println!("{:?}", encoded);

    // Deserialize using `json::decode`

	let json = "{\"data_int\":\"1\",\"data_str\":\"homura\",\"data_vector\":[2,3,4,5]}";

    let decoded: TestStruct = json::decode(&json).unwrap();

	let encoded2 = json::encode(&decoded).unwrap();

	println!("{:?}", encoded2);


	let bin_encoded: Vec<u8> = bincode::encode(&object, SizeLimit::Infinite).unwrap();

    println!("{:?}", bin_encoded);

    let bin_decoded: TestStruct = bincode::decode(&bin_encoded[..]).unwrap();

    assert!(object == bin_decoded);
}
*/


