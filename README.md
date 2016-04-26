# koala-dns
DNS Server in Rust

## Badges of Honour (?)

[![Clippy Linting Result](https://clippy.bashy.io/github/stevenpack/koala-dns/master/badge.svg?style=plastic)](https://clippy.bashy.io/github/stevenpack/koala-dns/master/log)

## Learning project

### TODO
- Zone files
- IPv6
- Thread per core (not possible? https://github.com/carllerche/mio/pull/338), https://github.com/rust-lang-nursery/net2-rs/commit/3a031f462eddd1884bb05667dcea2b65addafe83
- TODOs
- [more] Integration tests
- Other RFCs (e.g. EDNS/DNSSEC)
- Check MIO part with carllerche, particularly reading whole packet and
  transition from reading to reading/writing.
- Message compression (label pointers) in outbound
- Harden (limits per client, limits on forwarding, pool of upstream resolvers)
- Try CVE-2015-7547 http://hn.premii.com/#/article/11195351 and the Ghost exploit and see how we would have fared.
