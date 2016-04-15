use std::collections::HashMap;

pub struct Master {
	records: HashMap<RecordKey, Record>
}

impl Master {
	fn new() -> Master {
		Master {
			records: HashMap::new()
		}
	}

	fn add(&mut self, key: RecordKey, record: Record) {
		self.records.insert(key, record);
	}

	pub fn get(&self, key: &RecordKey) -> Option<&Record> {
		self.records.get(key)
	}
}

//TODO: use same for CacheKey
#[derive(Eq)]
#[derive(PartialEq)]
#[derive(PartialOrd)]
#[derive(Hash)]
#[derive(Clone)]
#[derive(Debug)]
pub struct RecordKey {
	pub name: String,
	pub typex: u16,
	pub class: u16
}

#[derive(Debug)]
pub struct Record {
	pub name: String,
	pub typex: u16,
	pub class: u16,
	pub ttl: u32	
}

//Returns an instance of a Master of recrods, from a file, db etc.
pub trait AuthorityProvider {
	fn create(&mut self) -> Master;
}

pub struct MasterFile {
	path: String,
	record_count: usize
}

impl MasterFile {
	pub fn new(path: String) -> MasterFile {
		MasterFile {
			path: path,
			record_count: 0
		}
	}

	fn parse(&mut self, path: &String) -> Master {
		let mut master = Master::new();
		//TODO: Hack testing only
		loop {
			match self.next_record() {
				Some(record) => {
					let key = RecordKey {
						name: record.name.clone(),
						typex: record.typex,
						class: record.class
					};
					debug!("Adding record {:?}", key);
					master.add(key, record);
				},
				None => break
			}
		}
		debug!("Parsed {} records", master.records.len());
		master
	}

	fn next_record(&mut self) -> Option<Record> {
		//see DNSJava impl for full logic
		//TODO: enums
		self.record_count += 1;

		match self.record_count {
			1 => Some(Record {
					name: String::from("example.org"),
					typex: 1, //A 
					class: 1, //IN (ternet)
					ttl: 300
				}),
			_ => None
		}
	}
}

impl AuthorityProvider for MasterFile {
	fn create(&mut self) -> Master {
		let path = self.path.clone();
		self.parse(&path)
	}


}