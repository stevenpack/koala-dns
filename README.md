# koala-dns
DNS Server in Rust

## Why?

To learn Rust. 

## Why the name?

My startup is [KoalaSafe](http://koalasafe.com). We run a DNS server on our OpenWRT access point. I'd like to convert it to Rust one day.

## What does it do?

* Accepts 'standard' DNS requests as per RFC1035 over TCP and UDP
* Responds Authoritively to requests for example.org
* Forwards upstream if it doesn't know the answer
* Caches and expires responses

## Non Functional notes

* Is a non-blocking server using [Mio](https://github.com/carllerche/mio). No thread-per-connection here. It was Robert Graham's the [C10M post](http://c10m.robertgraham.com/p/manifesto.html) that got me thinking about such things.

### TODO... so much to do.
- Load test
- Thread per core. (Some [issues](https://www.bountysource.com/issues/18319479-expose-api-to-set-so_reuseaddr-so_reuseport) around multiple listeners on a single UDP socket 
- Benchmark against other. BIND? Trust-DNS? It hasn't been optimized, but would be interesting.
- Master file parsing for authoritive servers. Example in java [here](https://sourceforge.net/p/dnsjava/code/HEAD/tree/tags/dnsjava-2.1.7/org/xbill/DNS/Master.java)
- IPv6 (easy)
- Thread per core (not possible? https://github.com/carllerche/mio/pull/338), https://github.com/rust-lang-nursery/net2-rs/commit/3a031f462eddd1884bb05667dcea2b65addafe83
- [more] Integration tests
- Other RFCs (e.g. EDNS/DNSSEC)
- Message compression (label pointers) in outbound
- Harden (limits per client, limits on forwarding, pool of upstream resolvers)
- Pluggable impls of cache. For example, a Redis cache.

## Build

`cargo build --release`

To run a server on port 10001 and forward to Google public DNS with a 500ms timeout:

`RUST_LOG=debug ./target/release/koala_dns_server -p 10001 -s 8.8.8.8:53 -t 500`

To query it:

`dig yahoo.com @127.0.0.1 -p 10001`

First time, the query time will be however long it takes to forward upstream (here 27ms).

<pre>; <<>> DiG 9.8.3-P1 <<>> yahoo.com @127.0.0.1 -p 10001
;; global options: +cmd
;; Got answer:
;; ->>HEADER<<- opcode: QUERY, status: NOERROR, id: 232
;; flags: qr rd ra; QUERY: 1, ANSWER: 3, AUTHORITY: 0, ADDITIONAL: 0

;; QUESTION SECTION:
;yahoo.com.			IN	A

;; ANSWER SECTION:
yahoo.com.		1056	IN	A	206.190.36.45
yahoo.com.		1056	IN	A	98.139.183.24
yahoo.com.		1056	IN	A	98.138.253.109

;; Query time: 27 msec
;; SERVER: 127.0.0.1#10001(127.0.0.1)
;; WHEN: Wed Apr 27 21:16:48 2016
;; MSG SIZE  rcvd: 75
</pre>

Second time it will be faster (here 1ms), and the cached response will have the ttl adjusted down.

<pre>dig yahoo.com @127.0.0.1 -p 10001

; <<>> DiG 9.8.3-P1 <<>> yahoo.com @127.0.0.1 -p 10001
;; global options: +cmd
;; Got answer:
;; ->>HEADER<<- opcode: QUERY, status: NOERROR, id: 27768
;; flags: qr rd ra; QUERY: 1, ANSWER: 3, AUTHORITY: 0, ADDITIONAL: 0

;; QUESTION SECTION:
;yahoo.com.			IN	A

;; ANSWER SECTION:
yahoo.com.		1053	IN	A	206.190.36.45
yahoo.com.		1053	IN	A	98.139.183.24
yahoo.com.		1053	IN	A	98.138.253.109

;; Query time: 1 msec
;; SERVER: 127.0.0.1#10001(127.0.0.1)
;; WHEN: Wed Apr 27 21:16:50 2016
;; MSG SIZE  rcvd: 102
</pre>
