<div align="center">

# database-cli

A CLI-based multi-database tool that exposes database connections, query execution, metadata inspection, and connection reuse as local commands callable by agents.

MySQL · PostgreSQL · Redis · Oracle · MongoDB · Read-only mode · Command blocklist · SQLcl Oracle · Local daemon

<p>
  <img src="https://img.shields.io/badge/CLI-database--cli-2ea44f" alt="CLI database-cli">
  <img src="https://img.shields.io/badge/License-MIT-green" alt="License MIT">
  <img src="https://img.shields.io/badge/Node.js-%3E%3D20-339933?logo=node.js&logoColor=white" alt="Node.js >=20">
  <img src="https://img.shields.io/badge/npm-%3E%3D10-CB3837?logo=npm&logoColor=white" alt="npm >=10">
  <img src="https://img.shields.io/badge/Windows-MacOS-0078D6?labelColor=0078D6&color=C0C0C0" alt="Windows/MacOS">
  <img src="https://img.shields.io/badge/release-v0.2.2-blue" alt="release v0.2.2">
</p>

[AI One-Click Installation](#ai-one-click-installation) · [Installation](#installation) · [Configuration](#configuration) · [Permission Configuration](#permission-configuration) · [Oracle SQLcl](#oracle-sqlcl) · [License](#license)

[中文](README.md) | English

</div>

## Introduction

`database-cli` references the database adapter, config loading, safety checking, and connection management layers from [Anarkh-Lee/universal-db-mcp](https://github.com/Anarkh-Lee/universal-db-mcp), but rewrites them as a standalone CLI. It does not include MCP, HTTP, or SSE services.

What it can do:

- List currently supported database types and locally configured connections
- Execute SQL, Redis commands, or MongoDB JSON commands against a configured database
- Inspect database metadata such as tables, columns, collections, and Redis keys
- Enable read-only mode and command blocklists per database configuration
- Auto-start the local daemon on demand; the daemon exits after `300` idle seconds by default
- Keep connections through the local daemon; each database connection is released after `180` idle seconds by default
- Switch Oracle between `oracledb` and SQLcl connection modes
- It does not store or print unmasked passwords, tokens, or secrets
- The daemon uses named pipes on Windows and Unix sockets on macOS/Linux

Driver configuration table:

| Database | `type` | Default driver | Driver switch configuration | Common configuration |
| --- | --- | --- | --- | --- |
| MySQL | `mysql` | npm package `mysql2` | Not switchable yet | `readonly`, `blacklist`, `keepAliveSeconds` |
| PostgreSQL | `postgres` | npm package `pg` | Not switchable yet | `readonly`, `blacklist`, `keepAliveSeconds` |
| Redis | `redis` | npm package `redis` | Not switchable yet | `readonly`, `blacklist`, `keepAliveSeconds` |
| Oracle | `oracle` | npm package `oracledb` | `oracleDriver: "oracledb" \| "sqlcl"`; SQLcl mode can set `sqlclPath` and `javaHome`. SQLcl is recommended for older Oracle versions | `readonly`, `blacklist`, `keepAliveSeconds` |
| MongoDB | `mongodb` | npm package `mongodb` | Not switchable yet; `database` can set the default database | `readonly`, `blacklist`, `keepAliveSeconds` |

## Installation

### Requirements

- Node.js `>= 20`
- npm `>= 10`
- Local network access to target databases
- Docker and Docker Compose for integration tests
- SQLcl and Java if Oracle uses SQLcl

### AI One-Click Installation

```text
Please read https://github.com/sleepinginsummer/database-cli/blob/main/AI_INSTALL.md, follow the instructions to install the CLI, and add `SKILL.md`.
```

### Manual Global Installation

```bash
npm install -g @sleepinsummer/database-cli
database-cli --help
```

If npm package installation is restricted, use the equivalent source installation flow:

```powershell
git clone https://github.com/sleepinginsummer/database-cli.git
cd database-cli
npm install
npm run build
npm link
database-cli --help
```

Add `SKILL.md` to the agent that needs to use this tool.

## Configuration

Default configuration file:

```text
~/.database-cli/config.json
```

Override the configuration path with an environment variable:

```bash
DATABASE_CLI_CONFIG=/path/to/config.json database-cli list
```

The configuration file is an object. Each key under `databases` is a database connection name:

- `type`: Database type. Supported values are `mysql`, `postgres`, `redis`, `oracle`, and `mongodb`
- `url`: Database connection URL
- `sshTunnel`: Optional SSH tunnel settings. When enabled, the database URL host and port are reached through the SSH tunnel
- `database`: Default MongoDB database name, optional
- `readonly`: Whether read-only mode is enabled. Defaults to `true`; only set it to `false` when write access is explicitly required
- `blacklist`: Command blocklist array, case-insensitive
- `keepAliveSeconds`: Per-database connection idle timeout in seconds, defaults to `180`
- `oracleDriver`: Oracle driver, either `oracledb` or `sqlcl`
- `sqlclPath`: SQLcl executable path, used only when `oracleDriver` is `sqlcl`
- `javaHome`: `JAVA_HOME` used by SQLcl, optional

`sshTunnel` supports password, private key, password plus private key, and passphrase-protected private key authentication:

- `host`: SSH jump host
- `port`: SSH port, defaults to `22`
- `username`: SSH username
- `password`: SSH password, optional
- `privateKeyPath`: Private key file path, optional, supports `~`
- `privateKey`: Private key content, optional, mutually exclusive with `privateKeyPath`
- `passphrase`: Private key passphrase, optional and only valid with a private key
- `readyTimeout`: SSH connection timeout in milliseconds, optional

The blocklist is checked before read-only mode. If a command matches the blocklist, it is rejected immediately; otherwise the read-only check is applied.

Read-only mode notes:

- Read-only mode is enabled by default. Write commands are rejected even when `readonly` is omitted
- It is recommended to keep all database connections read-only by default. If data changes are needed, let the AI generate the SQL or command first, then execute it after confirmation
- Only set `readonly: false` on a specific connection when write access is truly required

Reference configuration:

```json
{
  "databases": {
    "local-mysql": {
      "type": "mysql",
      "url": "mysql://user:password@localhost:3306/app",
      "readonly": true,
      "blacklist": ["drop", "truncate", "delete"],
      "keepAliveSeconds": 180
    },
    "remote-mysql": {
      "type": "mysql",
      "url": "mysql://user:password@db.internal:3306/app",
      "sshTunnel": {
        "host": "jump.example.com",
        "port": 22,
        "username": "deploy",
        "privateKeyPath": "~/.ssh/id_rsa",
        "passphrase": "key-passphrase"
      },
      "readonly": true,
      "keepAliveSeconds": 180
    },
    "cache": {
      "type": "redis",
      "url": "redis://localhost:6379",
      "readonly": false,
      "blacklist": ["flushall", "flushdb"],
      "keepAliveSeconds": 180
    },
    "oracle-test": {
      "type": "oracle",
      "url": "oracle://USER:password@127.0.0.1:1521/qftest201",
      "oracleDriver": "sqlcl",
      "sqlclPath": "/opt/homebrew/Caskroom/sqlcl/26.1.0.086.1709/sqlcl/bin/sql",
      "javaHome": "/Applications/IntelliJ IDEA Ultimate.app/Contents/jbr/Contents/Home",
      "readonly": true,
      "blacklist": ["drop", "truncate", "delete", "update", "insert", "merge", "alter", "create"],
      "keepAliveSeconds": 180
    }
  }
}
```

## Permission Configuration

Permission control should use both `readonly` and `blacklist` together. Do not rely on only one of them.

### Read-only Mode

- The default value is `true`
- When `readonly` is omitted, the connection is still treated as read-only
- It is recommended to keep all day-to-day query connections read-only by default
- If data changes are needed, let the AI generate the SQL or command first, then execute it after confirmation
- Only dedicated writable connections should explicitly set `readonly: false`

### Command Blocklist

- The blocklist has higher priority than read-only mode
- When a command matches the blocklist, it is rejected immediately
- It is suitable for blocking high-risk operations such as dropping data, schema changes, mass writes, and cache wipe commands
- It is recommended for production databases, shared test databases, and online Redis instances

### Execution Order

1. Check `blacklist` first
2. Reject immediately if matched
3. Check `readonly` only when not matched
4. When `readonly` is enabled, only read commands are allowed

### Common High-Risk Commands

Common high-risk SQL for MySQL / PostgreSQL / Oracle:

```json
["drop", "truncate", "delete", "update", "insert", "merge", "alter", "create", "replace", "grant", "revoke"]
```

Common high-risk Redis commands:

```json
["flushall", "flushdb", "del", "unlink", "set", "mset", "expire", "rename", "hset", "lpush", "rpush", "sadd", "zadd"]
```

Common high-risk MongoDB commands:

```json
["insertOne", "insertMany", "updateOne", "updateMany", "replaceOne", "deleteOne", "deleteMany", "findAndModify", "findOneAndUpdate", "findOneAndDelete", "drop", "dropDatabase", "createIndex", "dropIndex"]
```

### Recommended Configurations

Recommended for production databases:

```json
{
  "type": "mysql",
  "url": "mysql://user:password@prod-db:3306/app",
  "readonly": true,
  "blacklist": ["drop", "truncate", "delete", "update", "insert", "alter", "create"],
  "keepAliveSeconds": 180
}
```

Recommended for a dedicated writable connection:

```json
{
  "type": "postgres",
  "url": "postgres://user:password@write-db:5432/app",
  "readonly": false,
  "blacklist": ["drop", "truncate", "alter"],
  "keepAliveSeconds": 180
}
```

## Oracle SQLcl

Official link: https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

Oracle uses the npm package `oracledb` by default. Older Oracle servers may fail in Thin mode with errors such as `NJS-138`. In that case, switch that Oracle connection to SQLcl:

```json
{
  "type": "oracle",
  "url": "oracle://USER:password@127.0.0.1:1521/qftest201",
  "oracleDriver": "sqlcl",
  "sqlclPath": "/opt/homebrew/Caskroom/sqlcl/26.1.0.086.1709/sqlcl/bin/sql",
  "javaHome": "/Applications/IntelliJ IDEA Ultimate.app/Contents/jbr/Contents/Home",
  "readonly": true,
  "blacklist": ["drop", "truncate", "delete", "update", "insert", "merge", "alter", "create"]
}
```

SQLcl mode sends the connection script through stdin so the password does not appear in process arguments. Safety checks still run before execution, including blocklist and read-only mode.

## Uninstall and Cleanup

```bash
npm uninstall -g @sleepinsummer/database-cli
npm cache clean --force
rm -rf ~/.database-cli
docker compose down
```

## License

[MIT](LICENSE)
