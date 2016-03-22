extern crate time;

use std::collections::{HashMap};
use std::cmp::Ordering;
use time::*;
use dns::dns_entities::*;

pub trait Expires {
    fn expiry(&self) -> SteadyTime;
}

pub struct Cache {
    map: HashMap<CacheKey,CacheEntry>, //for retrieval
    keys: Vec<CacheExpiry> //for expiring (ordered). BTreeSet/Map doesn't work because it does't have any way to iterate and remove
}

impl Cache  {
    pub fn new() -> Cache {
        Cache {
            map: HashMap::new(),
            keys: Vec::new()
        }
    }

    pub fn upsert(&mut self, key: CacheKey, val: CacheEntry) {
        self.remove_expired();
        let expiry_data = CacheExpiry::new(key.clone(), val.expiry());
        self.keys.insert(0, expiry_data);
        self.keys.sort(); //only 1 item should ever be out-of-order. 
        // TODO Check internal impl. for vec.sort. Could be binary-like search to find appropriate insert point
        self.map.entry(key).or_insert(val);
    }

    pub fn get(&self, key: &CacheKey) -> Option<&CacheEntry> {
        self.map.get(key)
    }

    // pub fn contains(&self, key: &CacheKey) -> bool {
    //     self.map.contains_key(&key)
    // }

    // pub fn len(&self) -> usize {
    //     self.map.len()
    // }

    pub fn remove_expired(&mut self) -> usize {
        let now = SteadyTime::now();
        let mut count = 0;
        for cache_expiry in self.keys.iter() {
            if cache_expiry.expiry <= now {
                self.map.remove(&cache_expiry.key);
                count += 1;
            } else {
                break;
            }
        }
        self.keys.retain(|cache_expiry| cache_expiry.expiry > now);

        debug_assert!(self.map.len() == self.keys.len(), format!("map.len {:?} != keys.len {:?}", self.map.len(), self.keys.len()));        
        count
    }
}

#[derive(Eq)]
#[derive(PartialEq)]
#[derive(PartialOrd)]
#[derive(Hash)]
#[derive(Clone)]
#[derive(Debug)]
pub struct CacheKey {
    qname: String,
    qtype: u16,
    qclass: u16
}

impl CacheKey {
    pub fn new(qname: String, qtype: u16, qclass: u16) -> CacheKey {
        CacheKey {
            qname: qname,
            qtype: qtype,
            qclass: qclass
        }
    }
}

impl Ord for CacheKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.qname.cmp(&other.qname)
    }
}


#[derive(PartialOrd)]
#[derive(PartialEq)]
#[derive(Eq)]
#[derive(Debug)]
pub struct CacheEntry {
    key: CacheKey, //for expiring
    answers: Vec<DnsAnswer>,
    ttl: u32,
    expiry: SteadyTime
}

impl CacheEntry {
    pub fn new(key: CacheKey, answers: Vec<DnsAnswer>, ttl: u32) -> CacheEntry {
        CacheEntry {
            key: key,
            answers: answers,
            ttl: ttl,
            expiry: SteadyTime::now() + Duration::milliseconds(ttl as i64)
        }
    }
}

impl Ord for CacheEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.expiry.cmp(&other.expiry)
    }
}

impl Expires for CacheEntry {
    fn expiry(&self) -> SteadyTime {
        self.expiry
    }
}

#[derive(PartialOrd)]
#[derive(PartialEq)]
#[derive(Eq)]
#[derive(Debug)]
pub struct CacheExpiry {
    pub key: CacheKey,
    pub expiry: SteadyTime
}

impl CacheExpiry {
    pub fn new(key: CacheKey, expiry: SteadyTime) -> CacheExpiry {
        CacheExpiry {
            key: key,
            expiry: expiry
        }
    }
}

impl Ord for CacheExpiry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.expiry.cmp(&other.expiry)
    }
}

#[cfg(test)]
mod test {
    use super::{Cache, CacheEntry, CacheKey};
    use std::thread;
    use std::time::Duration;

    fn test_cache() -> Cache {
        let mut cache = Cache::new();
        let key = CacheKey::new(String::from("yahoo.com"), 1, 1);
        let val = CacheEntry::new(key.clone(), vec![String::from("yahoo.com")], 5);
        cache.upsert(key.clone(), val);
        cache
    }

    #[test]
    fn upsert() {
        let cache = test_cache();
        let key = CacheKey::new(String::from("yahoo.com"), 1, 1);
        assert_eq!(cache.get(&key).unwrap().answers[0], String::from("yahoo.com"));
    }

    #[test]
    fn expiry() {
        let mut cache = test_cache();
        let key2 = CacheKey::new(String::from("lycos.com"), 1, 1);
        let val2 = CacheEntry::new(key2.clone(), vec![String::from("lycos.com")], 100);
        cache.upsert(key2, val2);

        assert_eq!(2, cache.len());
        thread::sleep(Duration::from_millis(10));
        assert_eq!(1, cache.remove_expired());
        assert_eq!(1, cache.len());
    }

     #[test]
    fn len() {
        let cache = test_cache();
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn contains() {
        let cache = test_cache();
        let key = CacheKey::new(String::from("yahoo.com"), 1, 1);
        assert!(cache.contains(&key));
    }
}
