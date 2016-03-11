# koala-dns
DNS Server in Rust

## Learning project

### TODO
- [more] Integration tests
- TCP / common traits
- IPv6
- Zone files
  - [some] serializing dns_entities to buffers
- Other RFCs
- Check MIO part with carllerche, particularly reading whole packet and
  transition from reading to reading/writing.

- Try CVE-2015-7547 http://hn.premii.com/#/article/11195351 and the Ghost exploit and see how we would have fared.
