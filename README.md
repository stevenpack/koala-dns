# koala-dns
DNS Server in Rust

## Learning project

### TODO
- [some] Integration tests
- cache
- TCP / common traits
- Zone files
  - [some] serializing dns_entities to buffers
- Other RFCs
- Check MIO part with carllerche, particularly reading whole packet and
  transition from reading to reading/writing.
- Define interfaces to allow the dns lib, and (forthcoming) cache to be
   implented by other crates 
