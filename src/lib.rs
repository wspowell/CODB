#![allow(dead_code)]
#![allow(unused_must_use)]

extern crate regex;
extern crate rustc_serialize;
extern crate bincode;

use std::collections::{HashMap, HashSet};

mod db {
	use std::path::Path;
	use std::collections::{HashMap, HashSet};
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
		ComponentNotDefined(String),
		ResourceNotDefined(String),
		InstanceNotDefined(String),
		MalformedStructure(String)
	}

	#[derive(RustcEncodable, RustcDecodable, Debug, PartialEq)]
	pub enum Data {
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

	impl Data {
		fn copy(&self) -> Data {
			match self {
				&Data::BOOL(d) => Data::BOOL(d),
				&Data::CHAR(d) => Data::CHAR(d),
				&Data::SIGNED_INT_8(d) => Data::SIGNED_INT_8(d),
				&Data::SIGNED_INT_16(d) => Data::SIGNED_INT_16(d),
				&Data::SIGNED_INT_32(d) => Data::SIGNED_INT_32(d),
				&Data::SIGNED_INT_64(d) => Data::SIGNED_INT_64(d),
				&Data::UNSIGNED_INT_8(d) => Data::UNSIGNED_INT_8(d),
				&Data::UNSIGNED_INT_16(d) => Data::UNSIGNED_INT_16(d),
				&Data::UNSIGNED_INT_32(d) => Data::UNSIGNED_INT_32(d),
				&Data::UNSIGNED_INT_64(d) => Data::UNSIGNED_INT_64(d),
				&Data::FLOAT_32(d) => Data::FLOAT_32(d),
				&Data::FLOAT_64(d) => Data::FLOAT_64(d),
				&Data::STRING(ref d) => Data::STRING(d.to_string())
			}
		}
	}

	#[derive(RustcEncodable, RustcDecodable, Copy, Clone, Debug, PartialEq)]
	pub enum DataType {
		UINT,
		INT,
		STRING,
		PASSWORD
	}

	#[derive(RustcEncodable, RustcDecodable, Copy, Clone, Debug, PartialEq)]
	pub enum DataIO {
		/// Data can only be read from this slot and put into a non-input element. This 
		// field is excluded when saving so that read only data is not corrupted.
		DB_READ_ONLY,
		/// Data can only be written into this slot. It cannot be inserted into a non-input
		/// form element. The reasoning is to make a clear definition between showing 
		/// data and inputting/updating data.
		DB_INPUT,
		/// Data can be put into non-input and input form elements. Unlike read only fields,
		/// this data will be included when saving.
		DB_BOTH,
		/// This indicates a field that is not tied to a database location, but is included
		/// for the purpose of data processing. Example, a username/password on a login page
		/// is not tied to a database location, but it is used to authenticate a user.
		STATIC
	}

	#[derive(RustcEncodable, RustcDecodable, Copy, Clone, Debug, PartialEq)]
	pub enum ResourceIO {
		/// A form type can create instances. This would be for resources like blog posts.
		/// Each form resource must have an associated instance id. This is usually in the
		/// form of an auto-generated number id, but can be overridden.
		FORM,
		/// A static page has no instances. It is a single page that does not handle data
		/// submission. An example would be a login page. Any static resource that submits
		/// information is required to have a data processing subroutine defined. Saving
		/// a static resource will result in an error.
		STATIC
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

	#[derive(Debug, PartialEq)]
	pub struct ComponentInstance {
		pub component_id: usize,
		pub component_name: String,
		pub component_data_type: DataType,
		pub component_io_type: DataIO,
		pub data: Data
	}

	// Admin functions

	pub fn reset() -> DBResult<()> {
		try!(internals::start_from_scratch());
		Ok(())
	}

	pub fn add_resource(resource_name: &str, resource_type: ResourceIO,	instance_id: Option<usize>) -> DBResult<()> {
		let mut resources = try!(internals::Resources::load());

		// add resource definition
		let resource_id = resources.next_resource_id;
		resources.next_resource_id = resources.next_resource_id + 1;
		resources.resources.insert(resource_name.to_string(), (resource_id, resource_type));
		if instance_id != None {
			// to have an instance id, the resource must be FORM IO
			if resource_type != ResourceIO::FORM {
				return Err(DatabaseError::MalformedStructure(format!("Resource is not FORM IO type: {}", resource_name)));
			}
			resources.resource_instances.insert(resource_id, instance_id.unwrap());
		}

		// add model definition
		resources.models.insert(resource_id, HashMap::new());

		// save changes
		try!(resources.save());

		Ok(())
	}

	pub fn add_component(component_name: &str, data_type: DataType) -> DBResult<()> {
		let mut components = try!(internals::Components::load());

		// add component definition
		let component_id = components.next_component_id;
		components.next_component_id = components.next_component_id + 1;
		components.components.insert(component_name.to_string(), component_id);
		components.component_names.insert(component_id, component_name.to_string());

		// add type
		components.component_data_types.insert(component_id, data_type);

		// save changes
		try!(components.save());

		Ok(())
	}

	pub fn add_component_to_model(resource_name: &str, component_name: &str, io_type: DataIO) -> DBResult<()> {
		let mut resources = try!(internals::Resources::load());
		let components = try!(internals::Components::load());
		let mut instances = try!(internals::Instances::load());
		
		// the resource must exist
		if !resources.resources.contains_key(resource_name) {
			return Err(DatabaseError::ResourceNotDefined(format!("Resource is not defined: {}", resource_name)));
		}

		// the component must exist
		if !components.components.contains_key(component_name) {
			return Err(DatabaseError::ComponentNotDefined(format!("Component is not defined: {}", component_name)));
		}

		let component_id = components.components.get(component_name).unwrap();
		let resource_id = resources.resources.get(resource_name).unwrap().0;
		{ // mutable scope for adding component id to resources
			let mut model = resources.models.get_mut(&resource_id).unwrap();
			model.insert(*component_id, io_type);
		}

		// add component instance to instances (if not static)
		if io_type != DataIO::STATIC && !instances.instances.contains_key(&component_id) {
			// if no instances defined yet for component, then insert a new one
			instances.instances.insert(*component_id, HashMap::new());
			try!(instances.save());		
		}

		// save changes
		try!(resources.save());

		Ok(())
	}

	// API helper functions

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

	pub fn next_instance_id() -> DBResult<usize> {
		let mut instances = try!(internals::Instances::load());
		let instance_id = instances.next_instance_id;
		instances.next_instance_id = instances.next_instance_id + 1;
		try!(instances.save());

		Ok(instance_id)
	}

	// API functions

	pub fn get_component_data_type(component_name: &str) -> DBResult<DataType> {
		let components = try!(internals::Components::load());

		// the component must exist
		if !components.components.contains_key(component_name) {
			return Err(DatabaseError::ComponentNotDefined(format!("Component is not defined: {}", component_name)));
		}

		// get component id
		let component_id = components.components.get(component_name).unwrap();

		// get type
		let data_type: &DataType = components.component_data_types.get(&component_id).unwrap();

		// ?? done with Copy/Clone? Without Copy/Clone you cannot move this out of scope
		Ok(*data_type)
	}

	pub fn is_static_resource(resource_name: &str) -> DBResult<bool> {
		let resources = try!(internals::Resources::load());

		// the resource must exist
		if !resources.resources.contains_key(resource_name) {
			return Err(DatabaseError::ResourceNotDefined(format!("Resource is not defined: {}", resource_name)));
		}

		// get static flag
		let static_flag = (ResourceIO::STATIC == resources.resources.get(resource_name).unwrap().1);

		Ok(static_flag)
	}

	pub fn load_static_model(resource_name: &str) -> DBResult<HashMap<String, ComponentInstance>> {
		let resources = try!(internals::Resources::load());
		let components = try!(internals::Components::load());
		let instances = try!(internals::Instances::load());

		// the resource must exist
		if !resources.resources.contains_key(resource_name) {
			return Err(DatabaseError::ResourceNotDefined(format!("Resource is not defined: {}", resource_name)));
		}

		// the resource must be static
		if !try!(is_static_resource(resource_name)) {
			return Err(DatabaseError::MalformedStructure(format!("Resource is not static: {}", resource_name)));
		}

		// get resource id
		let resource_id = resources.resources.get(resource_name).unwrap().0;

		// get model
		let mut model: HashMap<String, ComponentInstance> = HashMap::new();
		let model_component = resources.models.get(&resource_id).unwrap();
		for (component_id, io_type) in model_component {
			println!("Getting component");
			// get component data
			let component_name = components.component_names.get(&component_id).unwrap();
			let component_data_type = components.component_data_types.get(&component_id).unwrap();

			// the component must be static
			if *io_type != DataIO::STATIC {
				return Err(DatabaseError::MalformedStructure(format!("Component is not static: {}", resource_name)));
			}

			let instance = ComponentInstance {
				component_id: *component_id,
				component_name: component_name.to_string(),
				component_data_type: *component_data_type,
				component_io_type: *io_type,
				data: Data::STRING("".to_string())
			};

			model.insert(component_name.to_string(), instance);
		}

		Ok(model)
	}

	pub fn load_model(resource_name: &str, instance_id: usize) -> DBResult<HashMap<String, ComponentInstance>> {
		let resources = try!(internals::Resources::load());
		let components = try!(internals::Components::load());
		let instances = try!(internals::Instances::load());

		// the resource must exist
		if !resources.resources.contains_key(resource_name) {
			return Err(DatabaseError::ResourceNotDefined(format!("Resource is not defined: {}", resource_name)));
		}

		// get resource id
		let resource_id = resources.resources.get(resource_name).unwrap().0;

		// get model
		let mut model: HashMap<String, ComponentInstance> = HashMap::new();
		let model_components = resources.models.get(&resource_id).unwrap();
		for (component_id, io_type) in model_components {
			// get component data
			let component_name = components.component_names.get(&component_id).unwrap();
			let component_data_type = components.component_data_types.get(&component_id).unwrap();

			// get data
			let data: Data = match instances.instances.get(&component_id).unwrap().get(&instance_id) {
				Some(d) => d.copy(),
				None => Data::STRING("".to_string())
			};

			let instance = ComponentInstance {
				component_id: *component_id,
				component_name: component_name.to_string(),
				component_data_type: *component_data_type,
				component_io_type: *io_type,
				data: data
			};
			
			model.insert(component_name.to_string(), instance);
		}

		Ok(model)
	}

	pub fn save_model(model: HashMap<String, ComponentInstance>, resource_name: &str, instance_id: usize) -> DBResult<()> {
		let resources = try!(internals::Resources::load());
		let components = try!(internals::Components::load());
		let mut instances = try!(internals::Instances::load());

		// get resource id
		let resource_id = resources.resources.get(resource_name).unwrap().0;

		for (component_name, instance) in model {
			// get instance and update instances with data
			let component_id = instance.component_id;

			// check the component data type
			let component_data_type = components.component_data_types.get(&component_id).unwrap();
			// TODO: filter here? Tainted?

			// check the component io type
			let component_io_type = resources.models.get(&resource_id).unwrap().get(&component_id).unwrap(); // io type is per resource and comes from the model
			if *component_io_type == DataIO::DB_READ_ONLY || *component_io_type == DataIO::STATIC {
				// do not try to save any read only or static components
				continue;
			}

			instances.instances.get_mut(&component_id).unwrap().insert(instance_id, instance.data);
		}

		Ok(())
	}

	/// Defines the internal workings of the database. This includes filesystems layout,
	/// file I/O, and database design.
	mod internals {
		use db;
		use std::path::Path;
		use std::collections::{HashMap, HashSet};
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

		// structs to serialize to file

		#[derive(RustcEncodable, RustcDecodable, PartialEq, Debug)]
		pub struct Resources {
			pub resources: HashMap<String, (usize, db::ResourceIO)>, // resource name : (resource id, static flag)
			pub resource_instances: HashMap<usize, usize>, // resource id : instance id
			pub models: HashMap<usize, HashMap<usize, db::DataIO>>, // resource id : [component id : data io]
			pub next_resource_id: usize, // keeps track of resource ids
		}

		impl Resources {
			fn new() -> Resources {
				Resources {
					resources: HashMap::new(),
					resource_instances: HashMap::new(),
					models: HashMap::new(),
					next_resource_id: 1,
				}
			}

			pub fn load() -> db::DBResult<Resources> {
				match load_from_file::<Resources>(RESOURCES_FILE) {
					Ok(r) => Ok(r),
					Err(error) => Err(error)
				}
			}

			pub fn save(&self) -> db::DBResult<()> {
				save_to_file::<Resources>(RESOURCES_FILE, &self)
			}
		}

		#[derive(RustcEncodable, RustcDecodable, PartialEq, Debug)]
		pub struct Components {
			pub components: HashMap<String, usize>,
			pub component_names: HashMap<usize, String>,
			pub component_data_types: HashMap<usize, db::DataType>,
			pub next_component_id: usize
		}

		impl Components {
			fn new() -> Components {
				Components {
					components: HashMap::new(),
					component_names: HashMap::new(),
					component_data_types: HashMap::new(),
					next_component_id: 0
				}
			}

			pub fn load() -> db::DBResult<Components> {
				match load_from_file::<Components>(COMPONENTS_FILE) {
					Ok(r) => Ok(r),
					Err(error) => Err(error)
				}
			}

			pub fn save(&self) -> db::DBResult<()> {
				save_to_file::<Components>(COMPONENTS_FILE, &self)
			}
		}

		#[derive(RustcEncodable, RustcDecodable, PartialEq, Debug)]
		pub struct Instances {
			pub instances: HashMap<usize, HashMap<usize, db::Data>>, // component id : [instance id : Data]
			pub next_instance_id: usize
		}

		impl Instances {
			fn new() -> Instances {
				Instances {
					instances: HashMap::new(),
					next_instance_id: 0
				}
			}

			pub fn load() -> db::DBResult<Instances> {
				match load_from_file::<Instances>(INSTANCES_FILE) {
					Ok(r) => Ok(r),
					Err(error) => Err(error)
				}
			}

			pub fn save(&self) -> db::DBResult<()> {
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

/*
#[test]
fn test_setup() {
	match db::reset() {
		Err(error) => panic!("{:?}", error),
		_ => ()
	};
}
*/


/// A static resource means that there is only one instance (copy) of that resource. This means
/// that it can only hold static components.

/// A static component means that there is only one instance of it.
/// Types of IO: DBInput, DBReadOnly, DBBoth, and Static
/// DB types reference a specific data slot in the database where information is stored. A static 
/// type is a field that is only there for data processing, such as username and password on
/// a login page. DB types are only allowed on non-static resources.


fn get_model(resource_name: &str) -> HashMap<String, db::ComponentInstance> {
	let model: HashMap<String, db::ComponentInstance> = {
		let is_static = match db::is_static_resource("/login/") {
			Err(error) => panic!("{:?}", error),
			Ok(b) => b
		};
		if is_static {
			// static resource, so just get the associated model
			match db::load_static_model("/login/") {
				Err(error) => panic!("{:?}", error),
				Ok(model) => model
			}
		} else {
			// use instance id, if applicable
			match db::load_model("/login/", 1) {
				Err(error) => panic!("{:?}", error),
				Ok(model) => model
			}
		}
	};

	model
}

#[test]
fn test_database() {
	// test admin functions

	match db::reset() {
		Err(error) => panic!("{:?}", error),
		_ => ()
	};

	match db::add_resource("/login/", db::ResourceIO::STATIC, None) {
		Err(error) => panic!("{:?}", error),
		_ => ()
	};

	let blog_post_instance_id = match db::next_instance_id() {
		Err(error) => panic!("{:?}", error),
		Ok(id) => id
	};

	match db::add_resource("/blog/username/my_first_post/", db::ResourceIO::FORM, Some(blog_post_instance_id)) {
		Err(error) => panic!("{:?}", error),
		_ => ()
	};

	match db::add_component("blogpost", db::DataType::STRING) {
		Err(error) => panic!("{:?}", error),
		_ => ()
	};

	match db::add_component("username", db::DataType::STRING) {
		Err(error) => panic!("{:?}", error),
		_ => ()
	};

	match db::add_component("password", db::DataType::PASSWORD) {
		Err(error) => panic!("{:?}", error),
		_ => ()
	};

	match db::add_component_to_model("/login/", "username", db::DataIO::STATIC) {
		Err(error) => panic!("{:?}", error),
		_ => ()
	};

	match db::add_component_to_model("/login/", "password", db::DataIO::STATIC) {
		Err(error) => panic!("{:?}", error),
		_ => ()
	};

	match db::add_component_to_model("/blog/username/my_first_post/", "blogpost", db::DataIO::DB_READ_ONLY) {
		Err(error) => panic!("{:?}", error),
		_ => ()
	};

	// test API functions

	let data_type1 = match db::get_component_data_type("username") {
		Err(error) => panic!("{:?}", error),
		Ok(data_type) => data_type
	};
	assert_eq!(db::DataType::STRING, data_type1);

	let data_type2 = match db::get_component_data_type("password") {
		Err(error) => panic!("{:?}", error),
		Ok(data_type) => data_type
	};
	assert_eq!(db::DataType::PASSWORD, data_type2);

	let model1 = get_model("/login/");

	match db::save_model(model1, "/login/", 1) {
		Err(error) => panic!("{:?}", error),
		_ => ()
	};

	let model2 = get_model("/login/");

	assert!(model2.contains_key("username"));


	// test static resource
	{
		let is_static = match db::is_static_resource("/login/") {
			Err(error) => panic!("{:?}", error),
			Ok(b) => b
		};
		assert!(is_static);

		// static resource, so just get the associated model
		match db::load_static_model("/login/") {
			Err(error) => panic!("{:?}", error),
			Ok(model) => model
		};
	}

	// test form resource
	{
		let is_static = match db::is_static_resource("/blog/username/my_first_post/") {
			Err(error) => panic!("{:?}", error),
			Ok(b) => b
		};
		assert!(!is_static);

		// get the associated model
		match db::load_model("/blog/username/my_first_post/", blog_post_instance_id) {
			Err(error) => panic!("{:?}", error),
			Ok(model) => model
		};
	}


	// test input data


	// test read only data


	// test input and read only data


	// test static data	
}




















































