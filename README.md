## http/https/tcp tunneling

support the following:

- http/https tunneling with `CONNECT`
- plain `http` proxy without `CONNECT`
- https tunneling with TLS sni, no other configuration needed
- transparent tcp proxy

example usage:

```
[http]
listen_port = 8081

[https]
listen_port = 8443

[[tcp]]
listen_port = 8082
remote_addr = "iot-broker.xeewo.com:8883"

[[tcp]]
listen_port = 8083
remote_addr = "192.168.31.197:80"

[target_connection]
dns_cache_ttl = "600s"
connect_timeout = "2s"
```

## build
```
cargo build --release
```

## run

```
./target/release/http-tunnel-rs -c ./conf.toml
```