# SightNet Crawler

## Usage

#### Args:

- Path to file with sites list
- Number of threads

#### Example:

```shell
cargo run ./sites.txt 3
```

## Config.toml

- `db_url` - db's url
- `lru_cache_capacity` - size of sites cache
- `user_agent` - user agent, which crawler will send to server and match robots.txt
- `http_reqs_timeout_for_thread` - timeout for http reqs for every thread