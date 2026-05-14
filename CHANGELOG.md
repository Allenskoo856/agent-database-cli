# Changelog

## 0.2.19

- 安全优化：数据库 URL 明文密码、SSH 隧道密码和私钥口令在首次使用连接时自动迁移到本地加密存储，配置文件仅保留 `passwordRef` / `passphraseRef`。
- 兼容性优化：保留明文字段改密入口，重新填写明文密码后下次使用会覆盖旧密文。

## 0.2.18

- Bugfix: `daemon status` 在 daemon 未运行或 Unix socket 残留时返回明确的未运行状态，避免直接暴露 `Connection refused`。

## 0.2.17

- 性能优化：daemon 数据库请求不再持有全局配置锁，改为按数据库连接粒度串行执行；不同数据库的慢查询不会互相阻塞。
- 稳定性优化：daemon 首次初始化同一数据库连接时增加初始化占位，避免并发冷启动重复创建 SSH 隧道和数据库连接。
- 安全优化：Redis keys 元信息查询改用 `SCAN` 分批读取，避免使用阻塞式 `KEYS`；只读模式下 Redis `KEYS` 命令会被拒绝。
- 安全优化：只读模式额外拒绝 PostgreSQL `SELECT INTO`、CTE 后接写操作，以及 MongoDB aggregate `$out` / `$merge` 等具备写入语义的查询。
- 易用性优化：daemon 空响应时返回明确错误信息，避免暴露底层 `EOF while parsing a value`。
- 质量优化：清理 clippy 报出的冗余代码写法，保持 `cargo fmt`、`cargo test`、`cargo clippy --all-targets -- -D warnings` 通过。

## 0.2.16

- Bugfix: 修复 Windows 下 daemon named pipe 客户端和服务端未实现，导致 `test`、`exec`、`meta`、`reset`、`daemon status` 等命令无法正常通过本地 daemon 工作的问题。
