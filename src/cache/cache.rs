// use dns::dns_entities::*;
// use std::cmp::Ordering;
// use std::fmt::Debug;
// use std::collections::{HashMap, BTreeSet, BTreeMap};
// use std::hash::*;
// use std::fmt::{Formatter,Error};
// use time::{Duration,Tm};
// use time;
//
// #[derive(PartialOrd)]
// #[derive(PartialEq)]
// #[derive(Eq)]
// #[derive(Clone)]
// #[derive(Hash)]
// pub struct DnsKey {
//     aname: String,
//     atype: u16,
//     aclass: u16,
// }
//
// impl Debug for DnsKey {
//     fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
//         write!(f, "DnsKey[ aname:{:?}, atype:{:?}, aclass:{:?}, ttl:{:?}, expiry:{} ",
//             self.aname, self.atype, self.aclass, self.ttl, self.expiry.rfc822())
//     }
// }
//
// pub struct ExpiryData {
//     ttl: u32,
//     expiry: Tm
// }
//
// pub trait Expires {
//     fn calc_expiry(ttl: u32) -> Tm {
//         time::now_utc() + Duration::milliseconds(ttl as i64)
//     }
//     fn ttl(&self) -> u32;
//     fn expires_at(&self) -> Tm;
//     fn is_expired(&self) -> bool {
//         time::now_utc() > self.expires_at()
//     }
//
//     fn x() -> bool {
//         true
//     }
// }
//
// impl Expires for ExpiryData {
//     fn expires_at(&self) -> Tm {
//         self.expiry
//     }
// }
//
// impl Ord for ExpiryData {
//     //order by ttl for fast iteration and expiry
//     fn cmp(&self, other: &Self) -> Ordering {
//         self.ttl.cmp(&other.ttl)
//     }
// }
//
// impl DnsKey {
//     pub fn empty() -> DnsKey {
//         DnsKey::new(String::from(""), 0, 0)
//     }
//
//     pub fn from_query(query: DnsQuestion) -> DnsKey {
//         DnsKey::new(query.qname, query.qtype, query.qclass)
//     }
//
//     pub fn new(aname: String, atype: u16, aclass: u16, ttl: u32) -> DnsKey {
//         DnsKey {
//             aname: aname,
//             atype: atype,
//             aclass: aclass
//         }
//     }
// }
//
// pub struct CacheEntry {
//     expiry_data: ExpiryData,
//     answers: Vec<DnsAnswer>
// }
//
// #[derive(Debug)]
// pub struct Cache {
//     pub base: ExpiringCache<DnsKey, CacheEntry>
// }
//
// impl Cache {
//     pub fn new() -> Cache {
//         let cache = ExpiringCache::<DnsKey, Vec<DnsAnswer>>::new();
//         return Cache {
//             base: cache
//         }
//     }
// }
//
// #[derive(Debug)]
// pub struct ExpiringCache<K,V> where K : Eq + Hash + Clone + Debug, V : Ord + Expires {
//     items: HashMap<K,V>, //hashmap for fast get
//     keys: BTreeMap<&V,K> //btreemap for fast iterating which keys need to be expired
// }
//
// impl<K,V> ExpiringCache<K,V> where K : Eq + Hash + Ord + Clone + Expires + Debug {
//
//     pub fn new() -> ExpiringCache<K,V> {
//         ExpiringCache {
//             items: HashMap::<K, V>::new(),
//             keys: BTreeMap::<K, &V>::new()
//         }
//     }
//
//     pub fn upsert(&mut self, key: K, val: V) {
//
//         self.remove_expired();
//
//         //For retrieval
//         self.items.entry(key.clone()).or_insert(val);
//         //For expiration
//         self.keys.insert(key.clone());
//         debug_assert!(self.keys.len() == self.items.len());
//     }
//
//     pub fn get(&self, key: &K) -> Option<&V> {
//         let val = self.items.get(key);
//         if val.is_expired() {
//             return None;
//         }
//         return val;
//     }
//
//     // fn remove(&mut self, key: &K) -> bool {
//     //     self.items.remove(key);
//     //     self.keys.remove(key)
//     // }
//
//     // fn contains(&self, key: &K) -> bool {
//     //     self.items.contains_key(key) && !key.is_expired()
//     // }
//     //
//     // fn is_expired(&self, key: &K) -> bool {
//     //     return self.items.contains_key(key) && key.is_expired();
//     // }
//
//     pub fn remove_expired(&mut self) -> usize {
//         //TODO: faster with btreeset.range() and difference()?
//         let mut to_remove = Vec::<K>::new();
//         let mut count = 0;
//         for entry in self.keys.iter() {
//             if entry.Value.is_expired() {
//                 println!("removing {:?}", key);
//                 self.items.remove(&key);
//                 to_remove.push(key.clone());
//                 count += 1;
//             } else {
//                 println!("breaking on {:?} after {:?}", key, count);
//                 //keys is ordered, so anything past here is not expired
//                 break;
//             }
//         }
//
//         for key in to_remove {
//             self.keys.remove(&key);
//         }
//         debug_assert!(self.keys.len() == self.items.len());
//         count
//     }
// }
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use dns::dns_entities::*;
//     use std::thread;
//     use std::time::{Duration};
//
//     #[test]
//     fn key_eq() {
//         let k1 = DnsKey::new(String::from("a"), 1, 1, 0);
//         let k1_2 = DnsKey::new(String::from("a"), 1, 1, 1);
//         let k3 = DnsKey::new(String::from("b"), 3, 3, 2);
//
//         assert_eq!(k1, k1_2);
//         assert!(k1 != k3);
//     }
//
//     #[test]
//     fn upsert_insert() {
//         let mut cache = Cache::new();
//         let key = test_key();
//         cache.base.upsert(key.clone(), test_answer("yahoo.com"));
//         let key2 = test_key();
//         let val = cache.base.get(&key2);
//         assert_eq!("yahoo.com", val.expect("upserted key missing")[0].name);
//     }
//
//     // #[test]
//     // fn upsert_update() {
//     //     //TODO
//     //     //update an existing item with new ttl
//     // }
//
//     #[test]
//     fn expire() {
//         let k1 = DnsKey::new(String::from("yahoo.com"), 1, 1, 5);
//         assert_eq!(k1.is_expired(), false);
//         thread::sleep(Duration::from_millis(10));
//         assert_eq!(k1.is_expired(), true);
//     }
//
//     #[test]
//     fn remove_expired() {
//         let mut cache = Cache::new();
//         let k1 = test_key_from("yahoo.com", 5);
//         let k2 = test_key_from("google.com", 50);
//         let k3 = test_key_from("lycos.com", 100);
//         cache.base.upsert(k1.clone(), test_answer("yahoo.com"));
//         cache.base.upsert(k2.clone(), test_answer("google.com"));
//         cache.base.upsert(k3.clone(), test_answer("lycos.com"));
//
//         thread::sleep(Duration::from_millis(75));
//
//         assert_eq!(cache.base.remove_expired(), 2);
//
//         assert!(cache.base.get(&k1).is_none());
//         assert!(cache.base.get(&k2).is_none());
//         assert!(cache.base.get(&k3).is_some());
//         assert_eq!(cache.base.get(&k3).unwrap()[0].name, "lycos.com");
//     }
//
//     fn test_answer(name: &str) -> Vec<DnsAnswer> {
//         let answer = DnsAnswer::new(String::from(name), 1, 1, 10, 1, Vec::<u8>::new());
//         vec![answer]
//     }
//
//     fn test_key() -> DnsKey {
//         DnsKey::new(String::from("yahoo.com"), 1, 1, 10)
//     }
//
//     fn test_key_from(name: &str, ttl: u32) -> DnsKey {
//         DnsKey::new(String::from(name), 1, 1, ttl)
//     }
//
//     fn test_key_with_ttl(ttl: u32) -> DnsKey {
//         DnsKey::new(String::from("yahoo.com"), 1, 1, ttl)
//     }
// }
