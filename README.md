# redito — Redis Toolkit CLI

**`redito`** is a command-line utility for working with Redis in a structured and
extensible way. It’s designed to support multiple Redis operations through modular
subcommands — starting with some tools for working with **Redis Streams**.

---

## Why

I frequently work with Redis streams, and Redis in general, and often wish I had some
tools that could make my life easier here and there. Couple with a desired to
code in Rust, and to get some experience maintaining a solid CLI tool, I decided to
lay the foundation for an extensible tool that should be useful to me, and could be
useful for others too.

---

## Features

* Supports configuration through:
  * Command-line arguments
  * Environment variables (prefixed with `REDITO_`)
  * Configuration files (`/etc/redito.toml`, `./.redito.toml`, `redito.toml`, or `local_config.toml`)
* TLS and Sentinel connection support.
* Stream utilities:
  * `stream-tail` — follow a stream like `tail -f`
  * `stream-copy` — copy entries between Redis instances
* Built with Rust, so blazingly fast and safe! (sorry, couldn't resist...)

---

## Installation

You can build from source:

```bash
cargo install --path .
```

Or:

```bash
cargo install redito
```

---

## Configuration

`redito` loads configuration in this order (later entries override earlier ones):

1. `/etc/redito.toml`
2. `./.redito.toml`
3. `./redito.toml`
4. `local_config.toml`
5. Environment variables prefixed with `REDITO_`
6. Command-line flags

I intend to use this tool both as an everyday cli tool, and as an automation helper to
be configured in cronjobs and such, which is why I'm putting some effort into it being
configurable through config files, environment variables, and command-line arguments all
at once.

### Example TOML config

```toml
[redis]
host = "localhost"
port = 6379
db = 0
tls = false
```

You can override any of these via CLI or environment variables.
For example:

```bash
REDITO_REDIS__HOST=redis.example.com redito stream-tail --stream mystream
```

---

## Commands

### `stream-tail`

Tails a Redis Stream, continuously reading new entries (similar to `tail -f`).

```bash
redito --host localhost stream-tail --stream mystream
```

**Options:**

| Option               | Description                                    | Default      |
| -------------------- | ---------------------------------------------- | ------------ |
| `--stream`           | Stream name to read from                       | *(required)* |
| `--plaintext`        | Print values as plain text                     | `false`      |
| `--raw-key`          | Specific field to print instead of full entry  | `None`       |
| `--group`            | Consumer group name                            | `None`       |
| `--consumer`         | Consumer name within group                     | `None`       |
| `--block-ms`         | Milliseconds to block waiting for new messages | `5000`       |
| `--count`            | Number of entries to fetch per request         | `1000`       |
| `--start-id`         | Stream ID to start reading from                | `$`          |
| `--retry-when-empty` | Retry when no messages are found               | `false`      |

---

### `stream-copy`

Copies entries from a source Redis stream to a target Redis instance.

```bash
redito --host src.redis.local stream-copy --stream mystream \
  --target-host dst.redis.local
```

**Options:**

| Option                     | Description                    | Default      |
| -------------------------- | ------------------------------ | ------------ |
| `--stream`                 | Source stream name             | *(required)* |
| `--target-host`            | Target Redis hostname          | *(required)* |
| `--target-port`            | Target Redis port              | `6379`       |
| `--target-username`        | Target Redis username          | `None`       |
| `--target-password`        | Target Redis password          | `None`       |
| `--target-db`              | Target Redis DB index          | `0`          |
| `--target-tls`             | Use TLS for target connection  | `false`      |
| `--target-sentinel`        | Use Redis Sentinel for target  | `false`      |
| `--target-sentinel-master` | Sentinel master name           | `None`       |
| `--group`                  | Source group name              | `None`       |
| `--consumer`               | Source consumer name           | `None`       |
| `--block-ms`               | Block duration while polling   | `5000`       |
| `--count`                  | Number of entries per fetch    | `1000`       |
| `--start-id`               | Start ID in stream             | `$`          |
| `--retry-when-empty`       | Retry on empty reads           | `false`      |

---

## Environment Variables

`redito` recognizes any setting as an environment variable, using the prefix `REDITO_`.

Nested fields (like Redis connection options) use double underscores `__`.

Example:

```bash
export REDITO_REDIS__HOST=redis.example.com
export REDITO_REDIS__PASSWORD=secret
redito stream-tail --stream mystream
```

---

## License

Dual-licensed under [Apache 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT).
