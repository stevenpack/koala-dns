# koala-dns
DNS Server in Rust

## Learning project

### TODO
- TODOs
- [more] Integration tests
- IPv6
- Zone files
- Thread per core
- Other RFCs (e.g. EDNS/DNSSEC)
- Check MIO part with carllerche, particularly reading whole packet and
  transition from reading to reading/writing.
- Message compression (label pointers) in outbound
- Revisit Request<T> vs Request with a Box<IConnection>, Box<ISender> etc.
- Harden (limits per client, limits on forwarding, pool of upstream resolvers)
- Try CVE-2015-7547 http://hn.premii.com/#/article/11195351 and the Ghost exploit and see how we would have fared.
