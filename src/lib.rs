#![allow(dead_code)]
#![allow(unused_must_use)]

extern crate regex;
extern crate rustc_serialize;
extern crate bincode;

mod db {
	use std::path::Path;
	use std::collections::{HashMap};
	use std::fs;
	use std::fs::{File, OpenOptions};
	use std::io::{BufReader, ErrorKind};
	use rustc_serialize::{Encodable, Decodable};
	use bincode::SizeLimit;
	use regex::Regex;

	pub type DBResult<T> = Result<T, DatabaseError>;

	#[derive(Debug)]
	pub enum DatabaseError {
		GenericError,
		FailedToLoadFile(String),
		FailedToSaveFile(String),
		FileNotFound(String),
		CouldNotOpenFile(String),
		CouldNotCreateFile(String),
		CouldNotCreateDirectory(String),
	}

	#[derive(RustcEncodable, RustcDecodable, Debug, PartialEq)]
	enum Data {
		BOOL(bool),
		CHAR(char),
		SIGNED_INT_8(i8),
		SIGNED_INT_16(i16),
		SIGNED_INT_32(i32),
		SIGNED_INT_64(i64),
		UNSIGNED_INT_8(u8),
		UNSIGNED_INT_16(u16),
		UNSIGNED_INT_32(u32),
		UNSIGNED_INT_64(u64),
		FLOAT_32(f32),
		FLOAT_64(f64),
		STRING(String)
	}

	#[derive(RustcEncodable, RustcDecodable, Debug, PartialEq)]
	enum TypeId {
		UINT,
		INT,
		STRING
	}

	#[derive(Debug)]
	enum IOType {
		OUTPUT,
		INPUT,
		BOTH
	}

	pub struct Tainted {
		data: String, // move the value to avoid using the it somewhere else
		filter: Filter
	}
	impl Tainted {
		pub fn new(data: String, filter: Filter) -> Tainted {
		    Tainted { 
		        data: data,
		        filter: filter
		    }
		}
		/// Moves Tainted out of scope and returns a safe String.
		/// The returned String MUST be a secure value.
		pub fn safe(self) -> String {
		    (self.filter)(self.data)
		}
	}

	pub type Filter = fn(String) -> String;

	pub fn alphanumeric_filter(tainted: String) -> String {
		let re = Regex::new(r"^[a-zA-Z0-9]$").unwrap();
		if re.is_match(&tainted) {
			// safe
			tainted.to_string()
		} else {
			String::new()
		}
		
	}

	pub fn reset() -> DBResult<()> {
		try!(internals::start_from_scratch());
		Ok(())
	}

	fn add_resource(resource_name: &str) -> DBResult<()> {
		Ok(())
	}

	fn add_component(component_name: &str, type_id: TypeId) -> DBResult<()> {
		Ok(())
	}

	fn add_component_to_model(resource_name: &str, component_name: &str, io_type: IOType) -> DBResult<()> {
		Ok(())
	}

	fn get_type_id(component_name: &str) -> DBResult<TypeId> {
		Ok(TypeId::STRING)
	}

	fn load_model(resource_name: &str, instance_id: usize) -> DBResult<HashMap<String, Data>> {
		Err(DatabaseError::GenericError)
	}

	fn save_model(model: HashMap<String, Data>, resource_name: &str, instance_id: usize) -> DBResult<()> {
		Ok(())
	}

	fn select(resource_name: &str, component_name: &str, instance_id: usize) -> DBResult<Data> {
		Err(DatabaseError::GenericError)
	}

	fn insert(resource_name: &str, component_name: &str, instance_id: usize, data: Tainted) -> DBResult<()> {
		Ok(())
	}
	fn update(resource_name: &str, component_name: &str, instance_id: usize, data: Tainted) -> DBResult<()> {
		Ok(())
	}

	fn merge(resource_name: &str, component_name: &str, instance_id: usize, data: Tainted) -> DBResult<()> {
		Ok(())
	}

	/// Defines the internal workings of the database. This includes filesystems layout,
	/// file I/O, and database design.
	mod internals {
		use db;
		use std::path::Path;
		use std::collections::{HashMap};
		use std::fs;
		use std::fs::{File, OpenOptions};
		use std::io::{BufReader, ErrorKind};
		use bincode::{decode, encode, decode_from, encode_into, SizeLimit};
		use rustc_serialize::{Encodable, Decodable};
		use regex::Regex;

		static DATA_FOLDER: &'static str = "data/";
		static RESOURCES_FILE: &'static str = "data/resources.db";
		static COMPONENTS_FILE: &'static str = "data/components.db";
		static INSTANCES_FILE: &'static str = "data/instances.db";

		struct Resource {
			resource_name: String,
			resource_id: usize,
			model: Vec<usize> // list of component ids
		}

		struct Component {
			component_name: String,
			component_id: usize,
			component_type: db::TypeId,
			is_static: bool
		}

		// structs to serialize to file

		#[derive(RustcEncodable, RustcDecodable, PartialEq, Debug)]
		struct Resources {
			resources: HashMap<String, usize>,
			models: HashMap<usize, Vec<usize>>,
			next_resource_id: usize, // keeps track of resource ids
			next_instance_type_id: usize, // keeps track of instance type ids
		}

		impl Resources {
			fn new() -> Resources {
				Resources {
					resources: HashMap::new(),
					models: HashMap::new(),
					next_resource_id: 1, // keeps track of resource ids
					next_instance_type_id: 1, // keeps track of instance type ids
				}
			}

			fn load() -> db::DBResult<Resources> {
				match load_from_file::<Resources>(RESOURCES_FILE) {
					Ok(r) => Ok(r),
					Err(error) => Err(error)
				}
			}

			fn save(&self) -> db::DBResult<()> {
				save_to_file::<Resources>(RESOURCES_FILE, &self)
			}
		}

		#[derive(RustcEncodable, RustcDecodable, PartialEq, Debug)]
		struct Components {
			components: HashMap<String, usize>,
			component_types: HashMap<usize, db::TypeId>,
			component_static_flags: HashMap<usize, bool>,
			next_component_id: usize
		}

		impl Components {
			fn new() -> Components {
				Components {
					components: HashMap::new(),
					component_types: HashMap::new(),
					component_static_flags: HashMap::new(),
					next_component_id: 0
				}
			}

			fn load() -> db::DBResult<Components> {
				match load_from_file::<Components>(COMPONENTS_FILE) {
					Ok(r) => Ok(r),
					Err(error) => Err(error)
				}
			}

			fn save(&self) -> db::DBResult<()> {
				save_to_file::<Components>(COMPONENTS_FILE, &self)
			}
		}

		#[derive(RustcEncodable, RustcDecodable, PartialEq, Debug)]
		struct Instances {
			instances: HashMap<usize, HashMap<usize, db::Data>>,
			next_instance_id: usize
		}

		impl Instances {
			fn new() -> Instances {
				Instances {
					instances: HashMap::new(),
					next_instance_id: 0
				}
			}

			fn load() -> db::DBResult<Instances> {
				match load_from_file::<Instances>(INSTANCES_FILE) {
					Ok(r) => Ok(r),
					Err(error) => Err(error)
				}
			}

			fn save(&self) -> db::DBResult<()> {
				save_to_file::<Instances>(INSTANCES_FILE, &self)
			}
		}

		pub fn start_from_scratch() -> db::DBResult<()> {
			// create all directories and files
			try!(create_directory(DATA_FOLDER));
			try!(create_file(RESOURCES_FILE));
			try!(create_file(COMPONENTS_FILE));
			try!(create_file(INSTANCES_FILE));

			// create default data
			let resources = Resources::new();
			try!(resources.save());

			let components = Components::new();
			try!(components.save());

			let instances = Instances::new();
			try!(instances.save());

			Ok(())
		}

		fn create_directory(filename: &'static str) -> db::DBResult<()> {
			let path = Path::new(&filename);
			match fs::create_dir(&filename) {
				Ok(dir) => Ok(()),
				Err(error) => {
					match error.kind() {
						ErrorKind::AlreadyExists => Ok(()),
						_ => Err(db::DatabaseError::CouldNotCreateDirectory(format!("Could not create directory, {}: {}", filename, error)))
					}
				}
			}
		}

		fn create_file(filename: &'static str) -> db::DBResult<File> {
			let path = Path::new(&filename);
			match File::create(path) {
				Ok(file) => Ok(file),
				Err(error)  => Err(db::DatabaseError::CouldNotCreateFile(format!("Could not create file, {}: {}", filename, error)))
			}
		}

		fn open_file_for_reading(filename: &'static str) -> db::DBResult<File> {
			let path = Path::new(&filename);
			match OpenOptions::new().read(true).write(false).open(path) {
				Ok(file) => Ok(file),
				Err(error)  => return Err(db::DatabaseError::FileNotFound(format!("Could not open file, {}: {}", filename, error)))
			}
		}

		fn open_file_for_writing(filename: &'static str) -> db::DBResult<File> {
			let path = Path::new(&filename);
			match OpenOptions::new().read(true).write(true).open(path) {
				Ok(file) => Ok(file),
				Err(error)  => return Err(db::DatabaseError::FileNotFound(format!("Could not open file, {}: {}", filename, error)))
			}
		}

		fn load_from_file<T>(filename: &'static str) -> db::DBResult<T> where T: Decodable {
			let file = try!(open_file_for_reading(&filename));

			// TODO: check if empty, if so then do something useful
			let mut reader = BufReader::new(&file);

			match decode_from(&mut reader, SizeLimit::Infinite) {
				Ok(value) => Ok(value),
				Err(error) => {
					return Err(db::DatabaseError::FileNotFound(format!("Could not decode file, {}: {}", filename, error)));
				}
			}
		}

		fn save_to_file<T>(filename: &'static str, value: &T) -> db::DBResult<()> where T: Encodable {	
			let mut file = try!(open_file_for_writing(&filename));

			// save definitions back to file
			match encode_into(&value, &mut file, SizeLimit::Infinite) {
				Err(error) => { return Err(db::DatabaseError::FailedToSaveFile(format!("Failed to save to file: {}, Reason: {}", filename, error))); }
				_ => ()
			};

			Ok(())
		}

		/*
		// TODO: return an error instead of panicing
		fn encode<T>(value: &T) -> Vec<u8> where T: Encodable {
			encode(&value, SizeLimit::Infinite).unwrap_or(panic!("Could not encode Value<T>."))
		}

		// TODO: return an error instead of panicing
		fn decode<T>(encoded: &Vec<u8>) -> T where T: Decodable {
			bincode::decode(&encoded[..]).unwrap_or(panic!("Could not decode to Value<T>."))
		}
		*/
	}
}


#[test]
fn test_setup() {
	match db::reset() {
		Err(error) => panic!("{:?}", error),
		_ => ()
	};
}


#[test]
fn test_resources() {
	
}




















































