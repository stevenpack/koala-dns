use dns::dns_entities::*;
use std::collections::HashMap;
use std::hash::Hash;


#[derive(Debug)]
#[derive(PartialEq)]
#[derive(Eq)]
#[derive(Hash)]
pub struct DnsKey {
    aname: String,
    atype: u16,
    aclass: u16
}

impl DnsKey {
    pub fn empty() -> DnsKey {
        DnsKey::new(String::from(""), 0, 0)
    }

    pub fn new(aname: String, atype: u16, aclass: u16) -> DnsKey {
        DnsKey {
            aname: aname,
            atype: atype,
            aclass: aclass
        }
    }
}

#[derive(Debug)]
pub struct ResolverCache {
    pub base: Cache<DnsKey, DnsAnswer>
}

impl ResolverCache {
    pub fn new() -> ResolverCache {
        let cache = Cache::<DnsKey, DnsAnswer>::new();
        return ResolverCache {
            base: cache
        }
    }
}

#[derive(Debug)]
pub struct Cache<K,V> where K : Eq + Hash {
    items: HashMap<K, V>
}

pub trait CacheOps<K,V> {
    fn add(&mut self, key: K, val: V);
}

impl<K,V> CacheOps<K,V> for Cache<K,V> where K : Eq + Hash {
    fn add(&mut self, key: K, val: V) {
        self.items.insert(key, val);
    }
}

impl<K,V> Cache<K,V> where K : Eq + Hash {

    pub fn new() -> Cache<K,V> {
        Cache {
            items: HashMap::<K, V>::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_eq() {
        let k1 = DnsKey::new(String::from("a"), 1, 1);
        let k1_2 = DnsKey::new(String::from("a"), 1, 1);
        let k3 = DnsKey::new(String::from("b"), 3, 3);

        assert_eq!(k1, k1_2);
        assert!(k1 != k3);
    }
}
