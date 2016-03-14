use dns::dns_entities::*;
use std::cmp::Ordering;
use std::fmt::Debug;
use std::collections::{HashMap, BTreeSet};
use std::hash::Hash;
use time::{Duration,Tm};
use time;

#[derive(Debug)]
#[derive(PartialOrd)]
#[derive(Eq)]
#[derive(Hash)]
#[derive(Clone)]
pub struct DnsKey {
    aname: String,
    atype: u16,
    aclass: u16,
    ttl: u32,
    expiry: Tm
}

pub trait Expires {
    fn expires_at(&self) -> Tm;
    fn is_expired(&self) -> bool {
        time::now_utc() > self.expires_at()
    }
}

impl PartialEq for DnsKey {
    //identity fields but not ttl
    fn eq(&self, other: &Self) -> bool {
        self.aname == other.aname &&
        self.atype == other.atype &&
        self.aclass == other.aclass
    }
}

impl Ord for DnsKey {
    //order by ttl for fast iteration and expiry
    fn cmp(&self, other: &Self) -> Ordering {
        self.ttl.cmp(&other.ttl)
    }
}

impl Expires for DnsKey {
    fn expires_at(&self) -> Tm {
        self.expiry
    }
}

impl DnsKey {
    pub fn empty() -> DnsKey {
        DnsKey::new(String::from(""), 0, 0, 0)
    }

    pub fn new(aname: String, atype: u16, aclass: u16, ttl: u32) -> DnsKey {
        DnsKey {
            aname: aname,
            atype: atype,
            aclass: aclass,
            ttl: ttl,
            expiry: time::now_utc() + Duration::milliseconds(ttl as i64)
        }
    }
}

#[derive(Debug)]
pub struct ResolverCache {
    pub base: ExpiringCache<DnsKey, DnsAnswer>
}

impl ResolverCache {
    pub fn new() -> ResolverCache {
        let cache = ExpiringCache::<DnsKey, DnsAnswer>::new();
        return ResolverCache {
            base: cache
        }
    }
}

#[derive(Debug)]
pub struct ExpiringCache<K,V> where K : Eq + Hash + Ord + Clone + Expires + Debug {
    items: HashMap<K, V>, //hashmap for fast get
    keys: BTreeSet<K> //btreeset for storing keys in order they need to be removed
}

impl<K,V> ExpiringCache<K,V> where K : Eq + Hash + Ord + Clone + Expires + Debug {

    pub fn new() -> ExpiringCache<K,V> {
        ExpiringCache {
            items: HashMap::<K, V>::new(),
            keys: BTreeSet::<K>::new()
        }
    }

    pub fn upsert(&mut self, key: K, val: V) {

        self.remove_expired();

        //For retrieval
        self.items.entry(key.clone()).or_insert(val);
        //For expiration
        self.keys.insert(key.clone());
        debug_assert!(self.keys.len() == self.items.len());
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        if key.is_expired() {
            println!("key expired {:?}", key);
            return None
        }
        return self.items.get(key);
    }

    // fn remove(&mut self, key: &K) -> bool {
    //     self.items.remove(key);
    //     self.keys.remove(key)
    // }

    // fn contains(&self, key: &K) -> bool {
    //     self.items.contains_key(key) && !key.is_expired()
    // }
    //
    // fn is_expired(&self, key: &K) -> bool {
    //     return self.items.contains_key(key) && key.is_expired();
    // }

    pub fn remove_expired(&mut self) -> usize {
        //TODO: faster with btreeset.range() and difference()?
        let mut to_remove = Vec::<K>::new();
        let mut count = 0;
        for key in self.keys.iter() {
            if key.is_expired() {
                println!("removing {:?}", key);
                self.items.remove(&key);
                to_remove.push(key.clone());
                count += 1;
            } else {
                println!("breaking on {:?} after {:?}", key, count);
                //keys is ordered, so anything past here is not expired
                break;
            }
        }

        for key in to_remove {
            self.keys.remove(&key);
        }
        debug_assert!(self.keys.len() == self.items.len());
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dns::dns_entities::*;
    use std::thread;
    use std::time::{Duration};

    #[test]
    fn key_eq() {
        let k1 = DnsKey::new(String::from("a"), 1, 1, 0);
        let k1_2 = DnsKey::new(String::from("a"), 1, 1, 1);
        let k3 = DnsKey::new(String::from("b"), 3, 3, 2);

        assert_eq!(k1, k1_2);
        assert!(k1 != k3);
    }

    #[test]
    fn upsert_insert() {
        let mut cache = ResolverCache::new();
        let key = test_key();
        cache.base.upsert(key.clone(), test_answer("yahoo.com"));
        let val = cache.base.get(&key);
        assert_eq!("yahoo.com", val.expect("upserted key missing").name);
    }

    // #[test]
    // fn upsert_update() {
    //     //TODO
    //     //update an existing item with new ttl
    // }

    #[test]
    fn expire() {
        let k1 = DnsKey::new(String::from("yahoo.com"), 1, 1, 5);
        assert_eq!(k1.is_expired(), false);
        thread::sleep(Duration::from_millis(10));
        assert_eq!(k1.is_expired(), true);
    }

    #[test]
    fn remove_expired() {
        let mut cache = ResolverCache::new();
        let k1 = test_key_from("yahoo.com", 5);
        let k2 = test_key_from("google.com", 50);
        let k3 = test_key_from("lycos.com", 100);
        cache.base.upsert(k1.clone(), test_answer("yahoo.com"));
        cache.base.upsert(k2.clone(), test_answer("google.com"));
        cache.base.upsert(k3.clone(), test_answer("lycos.com"));

        thread::sleep(Duration::from_millis(75));

        assert_eq!(cache.base.remove_expired(), 2);

        assert!(cache.base.get(&k1).is_none());
        assert!(cache.base.get(&k2).is_none());
        assert!(cache.base.get(&k3).is_some());
    }

    fn test_answer(name: &str) -> DnsAnswer {
        DnsAnswer::new(String::from(name), 1, 1, 10, 1, Vec::<u8>::new())
    }

    fn test_key() -> DnsKey {
        DnsKey::new(String::from("yahoo.com"), 1, 1, 10)
    }

    fn test_key_from(name: &str, ttl: u32) -> DnsKey {
        DnsKey::new(String::from(name), 1, 1, ttl)
    }

    fn test_key_with_ttl(ttl: u32) -> DnsKey {
        DnsKey::new(String::from("yahoo.com"), 1, 1, ttl)
    }
}
