#![allow(dead_code)]
#![allow(unused_must_use)]

extern crate rustc_serialize;
extern crate bincode;

use rustc_serialize::{Encodable};

mod db {
	extern crate rustc_serialize;
	extern crate bincode;
	use std::collections::{HashMap};

	use std::fs;
	use std::fs::{File};
	use std::io::{BufReader};
	use std::path::Path;

	use rustc_serialize::{Encodable, Decodable};

	use bincode::SizeLimit;

	// TODO: return an error instead of panicing
	fn load_from_file<T>(directory: &str, filename: &str) -> T where T: Decodable {
		let dir_and_filename = format!("{}{}", directory, filename);
		
		let path = Path::new(&dir_and_filename);

		let file = match File::open(path) {
			Ok(file) => file,
			Err(error)  => panic!("Could not open file: {}", error)
		};

		// TODO: check if empty, if so then do something useful
		let mut reader = BufReader::new(&file);

		match bincode::decode_from(&mut reader, bincode::SizeLimit::Infinite) {
			Ok(value) => value,
			Err(error) => panic!("Could not load file, {}: {}", filename, error)
		}
	}

	// TODO: return an error instead of panicing
	fn save_to_file<T>(directory: &str, filename: &str, value: &T) where T: Encodable {	
		let dir_and_filename = format!("{}{}", directory, filename);
		
		let path = Path::new(&dir_and_filename);
		
		// FIXME: PathExt is unstable, grrrrr
		//if !path.is_dir() {
			fs::create_dir(directory); // ignore the return until PathExt is stable
		//}

		// FIXME: PathExt is unstable, grrrrr
		//if !path.is_file() {
			create_file(&path);
		//}

		let mut file = match File::create(path) {
			Ok(file) => file,
			Err(error) => panic!("Could not open file, {}: {}", filename, error)
		};

		// save definitions back to file
		match bincode::encode_into(&value, &mut file, bincode::SizeLimit::Infinite) {
			Err(error) => panic!("Could not save to file, {}: {}", filename, error),
			_ => ()
		};
	}

	// TODO: return an error instead of panicing
	fn create_file(path: &Path) {
		match File::create(path) {
			Ok(file) => file,
			Err(error) => panic!("Could not open file: {}", error)
		};
	}

	// TODO: return an error instead of panicing
	fn encode<T>(value: &T) -> Vec<u8> where T: Encodable {
		bincode::encode(&value, SizeLimit::Infinite).unwrap_or(panic!("Could not encode Value<T>."))
	}

	// TODO: return an error instead of panicing
	fn decode<T>(encoded: &Vec<u8>) -> T where T: Decodable {
		bincode::decode(&encoded[..]).unwrap_or(panic!("Could not decode to Value<T>."))
	}

	#[derive(RustcEncodable, RustcDecodable, PartialEq)]
	pub struct Layout {
		version : usize, // version of the database
		resources: HashMap<String, usize>, // resource name : resource id 
		resource_models: HashMap<usize, Vec<usize>>, // resource id : list of variable ids
		resource_instances: HashMap<usize, Vec<usize>>, // resource id : list of instances (if no resource id is in the map then it is treated as a single page, ex /login/)
		
		/// The InstanceTypes are defined by the user and therefore, it is left up
		/// to the user to properly version each instance_type and migrate from one to
		/// the next. Each InstanceType has an ID which is used as the key and a
		/// version ID which is used to keep different versions/types separate.
		/// The instance ID is used to look up the file where the data lies. This 
		/// is by convention defined in the API (not by the user) and could be something like 
		/// /data/instance_types/[instance_type_id]/[version_id]/[instance_id].dat
		instance_type: HashMap<usize, HashMap<usize, usize>> // instance_type id : [version id : instance id]
	}

	impl Layout {
		pub fn init() {
			fs::create_dir("data/"); // ignore result until PathExt is stable
			fs::create_dir("data/layout/"); // ignore result until PathExt is stable
			fs::create_dir("data/instance_types/"); // ignore result until PathExt is stable

			let init_layout = Layout {
				version: 0,
				resources: HashMap::new(),
				resource_models: HashMap::new(),
				resource_instances: HashMap::new(),
				instance_type: HashMap::new()
			};	

			save_to_file::<Layout>("data/layout/", "layout.dat", &init_layout);
		}

		pub fn load() -> Layout {
			load_from_file::<Layout>("data/layout/", "layout.dat")
		}
		
		pub fn save(&self) {
			save_to_file("data/layout/", "layout.dat", &self);
		}
	
		//pub fn add_resource(
	}
	

	/// Defines one variable (or "bucket") in which instances will be placed 
	/// Each InstanceType must know how to encode its value and where the instances live
	pub trait InstanceType: Encodable + Decodable + PartialEq {
		fn instance_id(&self) -> usize;
		fn instance_type_id() -> &'static usize;
		fn version(&self) -> &'static usize;
		fn migrate(migration: &'static (&'static usize, &'static usize, fn())) {
		    migration.2();
		}
	}

	// TODO: return an error instead of panicing
	pub fn load_instance<T>(instance_id: usize) -> T where T: InstanceType {
		let directory = format!("data/instance_types/{}/", T::instance_type_id());
		let filename = format!("{}.dat", instance_id);

		load_from_file(&directory, &filename)
	}

	// TODO: return an error instead of panicing
	pub fn save_instance<T>(instance: &T) where T: InstanceType {
		let directory = format!("data/instance_types/{}/", T::instance_type_id());
		let filename = format!("{}.dat", instance.instance_id());

		save_to_file(&directory, &filename, &instance);
	}

	#[test]
	fn test_db_init_load_save() {
		Layout::init();
		let mut layout: Layout = Layout::load();
		assert_eq!(layout.version, 0);
		layout.version = 1;
		layout.save();
	
		let layout2: Layout = Layout::load();
		assert_eq!(layout2.version, 1);
	}
}

/// From here down it is user end code, above is the API interface

#[derive(RustcEncodable, RustcDecodable, PartialEq)]
struct Text {
	instance_id: usize,
	text: String
}

impl db::InstanceType for Text { 
	fn instance_id(&self) -> usize {
		self.instance_id
	}

    fn instance_type_id() -> &'static usize {
        static ID: &'static usize = &0;
        ID
    }
    
    fn version(&self) -> &'static usize {
        static VERSION: &'static usize = &0;
        VERSION
    }
}

#[test]
fn test_instance() {
	let instance = Text {
		instance_id: 0,
		text: "Hello, world!".to_string()
	};
	
	db::save_instance(&instance);
	
	let loaded: Text = db::load_instance(0);
	assert_eq!(instance.text, loaded.text);
}

