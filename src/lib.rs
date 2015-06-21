#![allow(dead_code)]
#![allow(unused_must_use)]

extern crate regex;
extern crate rustc_serialize;
extern crate bincode;

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

	/// The layout of data in the site. This gets loaded into a shared instance that each
	/// request can access. The admin pages can edit the data in the database and then
	/// reload the shared instance (by locking it and writing, should be quick enough not 
	/// to affecting any requests).
	#[derive(RustcEncodable, RustcDecodable, PartialEq)]
	pub struct Layout {
		version: usize, // version of the database
		next_resource_id: usize, // keeps track of resource ids
		next_instance_type_id: usize, // keeps track of instance type ids

		resources: HashMap<String, usize>, // resource name : (resource id, static flag(ex. a login page never has more than one instance))
		instance_types: HashMap<usize, String>, // instance type id : instance type name
		resource_models: HashMap<usize, Vec<(usize, bool, bool)>>, // resource id : list of (instance type id, read only, static)
		instance_type_references: HashMap<usize, Vec<(usize, bool)>>, // instance type id : (resource id, read only)

		resource_instances: HashMap<usize, Vec<usize>>, // resource id : list of instance ids (if no resource id is in the map then it is treated as a single page, ex /login/)
		

		static_instances: HashMap<usize, (usize, usize)>, // instance type id : (version id, instance id)

		/// The InstanceTypes are defined by the user and therefore, it is left up
		/// to the user to properly version each instance_type and migrate from one to
		/// the next. Each InstanceType has an ID which is used as the key and a
		/// version ID which is used to keep different versions/types separate.
		/// The instance ID is used to look up the file where the data lies. This 
		/// is by convention defined in the API (not by the user) and could be something like 
		/// /data/instance_types/[instance_type_id]/[version_id]/[instance_id].dat
		instances: HashMap<usize, HashMap<usize, usize>> // instance type id : [version id : instance id]
	}

	impl Layout {
		pub fn init() {
			fs::create_dir("data/"); // ignore result until PathExt is stable
			fs::create_dir("data/layout/"); // ignore result until PathExt is stable
			fs::create_dir("data/instance_types/"); // ignore result until PathExt is stable

			let init_layout = Layout {
				version: 1,
				next_resource_id: 1,
				next_instance_type_id: 1,
				resources: HashMap::new(),
				instance_types: HashMap::new(),
				resource_models: HashMap::new(),
				instance_type_references: HashMap::new(),
				resource_instances: HashMap::new(),
				static_instances: HashMap::new(),
				instances: HashMap::new()
			};	

			save_to_file::<Layout>("data/layout/", "layout.dat", &init_layout);
		}

		pub fn load() -> Layout {
			load_from_file::<Layout>("data/layout/", "layout.dat")
		}
		
		pub fn save(&self) {
			save_to_file("data/layout/", "layout.dat", &self);
		}

		fn bump_version(&mut self) {
			self.version = self.version + 1;
		}

		fn bump_resource_id(&mut self) {
			self.next_resource_id = self.next_resource_id + 1;
		}

		fn bump_instance_type_id(&mut self) {
			self.next_instance_type_id = self.next_instance_type_id + 1;
		}
	
		pub fn add_resource(&mut self, resource_name: &str, is_static: bool) {
			if self.resources.insert(resource_name.to_string(), self.next_resource_id) != None {
				panic!("Resource name already exists: {}", resource_name);
			}
			self.bump_resource_id();
		}

		pub fn add_instance_type(&mut self, instance_type_name: &str) {
			self.instance_types.insert(self.next_instance_type_id, instance_type_name.to_string());
			self.bump_instance_type_id();
		}

		pub fn add_instance_type_to_model(&mut self, resource_id: usize, instance_type: (usize, bool, bool)) {
			if !self.resource_models.contains_key(&resource_id) {
				self.resource_models.insert(resource_id, Vec::new());
			}

			self.resource_models.get_mut(&resource_id).unwrap().push(instance_type);
		}
		
	}

	pub trait Tainted<T>: InstanceType {
		/// The return value of this function MUST be a safe value.
		/// Any malicious user input is assumed to be filtered out.
		fn safe(&self) -> T where T: Encodable;
		fn raw(&self) -> &T;
		fn raw_mut(&mut self) -> &mut T;
		/// Sets the value of the data. This is required to retrieve
		/// data from the database.
		fn set(&mut self, data: T) where T: Decodable;
	}

	/// Defines one variable (or "bucket") in which instances will be placed 
	/// Each InstanceType must know how to encode its value and where the instances live
	pub trait InstanceType {
		fn new() -> Self;
		fn instance_id(&self) -> usize;
		fn instance_type_id() -> &'static usize;
		fn version(&self) -> &'static usize;
		fn migrate(migration: &'static (&'static usize, &'static usize, fn())) {
		    migration.2();
		}
	}

	// TODO: return an error instead of panicing
	pub fn load_instance<I, T>(instance_id: usize) -> I where I: InstanceType + Tainted<T>, T: Decodable {
		let directory = format!("data/instance_types/{}/", I::instance_type_id());
		let filename = format!("{}.dat", instance_id);

		let decoded: T = load_from_file(&directory, &filename);
		let mut instance: I = I::new();
		instance.set(decoded);

		instance
	}

	// TODO: return an error instead of panicing
	pub fn save_instance<I, T>(instance: &I) where I: InstanceType + Tainted<T>, T: Encodable {
		let directory = format!("data/instance_types/{}/", I::instance_type_id());
		let filename = format!("{}.dat", instance.instance_id());

		let safe: T = instance.safe();

		save_to_file(&directory, &filename, &safe);
	}

	/// Return a map of instance type id : instance
	pub fn load_model(instance_id: usize) -> HashMap<usize, Box<InstanceType>> {
		let instance_types: HashMap<usize, Box<InstanceType>> = HashMap::new();

		

		instance_types
	}

	#[test]
	fn test_db_init_load_save() {
		Layout::init();
		let mut layout: Layout = Layout::load();
		assert_eq!(layout.version, 1);
		layout.version = 1;
		layout.save();
	
		let layout2: Layout = Layout::load();
		assert_eq!(layout2.version, 1);
	}
}

mod checks {
	use regex::Regex;

	pub fn is_alphanumeric<T>(value: &str) -> bool {
		// TODO: convert to regex macro
		let re = Regex::new(r"^\[A-Za-z0-9$").unwrap();
		re.is_match(value)
	}
}

/// From here down it is user end code, above is the API interface

struct Text {
	instance_id: usize,
	text: String
}

impl db::InstanceType for Text { 
	fn new() -> Text {
		Text {
			instance_id: 0,
			text: String::new()
		}
	}

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

impl db::Tainted<String> for Text {
	fn safe(&self) -> String {
		self.text.to_string()
	}
	fn raw(&self) -> &String {
		&self.text
	}
	fn raw_mut(&mut self) -> &mut String {
		&mut self.text
	}
	fn set(&mut self, data: String) {
		self.text = data;
	}
}

#[test]
fn test_instance() {
	let instance = Text {
		instance_id: 1,
		text: "Hello, world!".to_string()
	};
	
	db::save_instance(&instance);
	
	let loaded: Text = db::load_instance(1);
	assert_eq!(instance.text, loaded.text);
}

