use dns::dns_entities::*;
use std::cmp::Ordering;
use std::collections::{HashMap, BTreeSet};
use std::hash::Hash;
use time::{Duration,SteadyTime,Tm};
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
    fn expires(&self) -> Tm;
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
    fn expires(&self) -> Tm {
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
pub struct ExpiringCache<K,V> where K : Eq + Hash + Ord + Clone + Expires {
    items: HashMap<K, V>, //hashmap for fast get
    keys: BTreeSet<K> //btreeset for storing keys in order they need to be removed
}

impl<K,V> ExpiringCache<K,V> where K : Eq + Hash + Ord + Clone + Expires {

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
    }

    pub fn get(&self, key: &K) -> Option<&V> {

        //if not expired...

        return self.items.get(key);
    }
    //
    // fn remove(&mut self, key: &K) -> bool {
    //     return false;
    // }
    fn contains(&self, key: &K) -> bool {
        //if not expired...
        return false;
    }
    fn remove_expired(&mut self) -> usize {
        return 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dns::dns_entities::*;

    #[test]
    fn key_eq() {
        let k1 = DnsKey::new(String::from("a"), 1, 1, 0);
        let k1_2 = DnsKey::new(String::from("a"), 1, 1, 1);
        let k3 = DnsKey::new(String::from("b"), 3, 3, 2);

        assert_eq!(k1, k1_2);
        assert!(k1 != k3);
    }

    #[test]
    fn upsert() {
        let mut cache = ResolverCache::new();
        let key = DnsKey::empty();
        cache.base.upsert(key.clone(), DnsAnswer::new(String::from("yahoo.com"), 1,1,10,1,Vec::<u8>::new()));
        let val = cache.base.get(&key);
        assert_eq!("yahoo.com", val.expect("upserted key missing").name);
    }
}
