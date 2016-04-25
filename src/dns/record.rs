// #[derive(Eq)]
// #[derive(PartialEq)]
// #[derive(PartialOrd)]
// #[derive(Hash)]
// #[derive(Clone)]
// #[derive(Debug)]
// pub struct RecordKey {
// 	pub name: String,
// 	pub typex: u16,
// 	pub class: u16
// }

// #[derive(Debug)]
// pub struct Record {
// 	pub name: String,
// 	pub typex: u16,
// 	pub class: u16,
// 	pub ttl: u32	
// }

//TODO: How to model all the records... Option<> fields? base style inheritance? See http://www.dnsjava.org/dnsjava-current/doc/
//      for a sample inhertiance structure
// pub struct ARecord {
// 	pub base: Record,
// 	pub address: [u32]
// }