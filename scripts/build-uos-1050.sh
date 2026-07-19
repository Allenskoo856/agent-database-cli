#!/usr/bin/env bash
set -euo pipefail

export DEBIAN_FRONTEND=noninteractive

echo "[uos-1050] Installing build dependencies on Debian 10..."

# Debian 10 (buster) is archived; use archive.debian.org
cat > /etc/apt/sources.list <<'EOF'
deb http://archive.debian.org/debian buster main contrib non-free
deb http://archive.debian.org/debian-security buster/updates main contrib non-free
deb http://archive.debian.org/debian buster-updates main contrib non-free
EOF
echo 'Acquire::Check-Valid-Until "false";' > /etc/apt/apt.conf.d/99no-check-valid-until
apt-get update
apt-get install -y --no-install-recommends \
  ca-certificates curl build-essential pkg-config git python3 file binutils xz-utils unzip zip
rm -rf /var/lib/apt/lists/*

echo "[uos-1050] Installing Node.js ${NODE_VERSION}..."
curl -fsSL "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-x64.tar.xz" -o /tmp/node.tar.xz
mkdir -p /usr/local/lib/nodejs
tar -xJf /tmp/node.tar.xz -C /usr/local/lib/nodejs
export PATH="/usr/local/lib/nodejs/node-v${NODE_VERSION}-linux-x64/bin:${PATH}"
node -v
npm -v

echo "[uos-1050] Installing Rust ${RUST_VERSION}..."
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain "${RUST_VERSION}" --profile minimal
export PATH="${HOME}/.cargo/bin:${PATH}"
rustc --version
cargo --version
ldd --version | head -n1

echo "[uos-1050] Building agent-database-cli native binary..."
cargo build --release --bin agent-database-cli
BIN="target/release/agent-database-cli"
test -x "${BIN}"

VERSION="$(node -p "require('./package.json').version")"
OUT_DIR="dist/uos-1050"
PKG_DIR="packages/linux-x64"
mkdir -p "${OUT_DIR}/npm" "${PKG_DIR}/bin"

cp "${BIN}" "${OUT_DIR}/agent-database-cli"
chmod +x "${OUT_DIR}/agent-database-cli"
cp "${BIN}" "${PKG_DIR}/bin/agent-database-cli"
chmod +x "${PKG_DIR}/bin/agent-database-cli"

if [ -f npm/package-template.json ]; then
  sed \
    -e "s#__NAME__#@agent-database-cli/linux-x64#g" \
    -e "s#__VERSION__#${VERSION}#g" \
    -e "s#__TARGET__#x86_64-unknown-linux-gnu#g" \
    -e "s#__OS__#linux#g" \
    -e "s#__CPU__#x64#g" \
    npm/package-template.json > "${PKG_DIR}/package.json"
else
  node <<'NODE'
const fs = require('fs');
const version = JSON.parse(fs.readFileSync('package.json', 'utf8')).version;
const pkg = {
  name: '@agent-database-cli/linux-x64',
  version,
  description: 'Platform binary for agent-database-cli (Debian 10 / UOS 1050)',
  license: 'MIT',
  os: ['linux'],
  cpu: ['x64'],
  files: ['bin'],
  publishConfig: { access: 'public' },
  repository: {
    type: 'git',
    url: 'https://github.com/sleepinginsummer/agent-database-cli'
  }
};
fs.writeFileSync('packages/linux-x64/package.json', JSON.stringify(pkg, null, 2) + '\n');
NODE
fi

node <<'NODE'
const fs = require('fs');
const p = 'packages/linux-x64/package.json';
const pkg = JSON.parse(fs.readFileSync(p, 'utf8'));
pkg.description = (pkg.description || 'agent-database-cli linux-x64') + ' [Debian 10 / UOS 1050 glibc 2.28]';
fs.writeFileSync(p, JSON.stringify(pkg, null, 2) + '\n');
NODE

npm pack --pack-destination "${OUT_DIR}/npm" "./${PKG_DIR}"
npm pack --pack-destination "${OUT_DIR}/npm" .

{
  echo "project=agent-database-cli"
  echo "version=${VERSION}"
  echo "git_sha=${GITHUB_SHA:-unknown}"
  echo "target=x86_64-unknown-linux-gnu"
  echo "build_image=debian:10"
  echo "intended_os=UOS 1050 / Debian 10 (glibc 2.28)"
  echo "rustc=$(rustc --version)"
  echo "node=$(node -v)"
  ldd --version | head -n1
  file "${OUT_DIR}/agent-database-cli"
  echo "GLIBC symbols:"
  strings "${OUT_DIR}/agent-database-cli" | grep -o 'GLIBC_[0-9.]*' | sort -u || true
} > "${OUT_DIR}/BUILD_INFO.txt"

cat > "${OUT_DIR}/INSTALL_UOS1050.md" <<'EOF'
# UOS 1050 offline install (agent-database-cli)

Built in debian:10 (glibc 2.28) for UOS 1050.

## Files
- agent-database-cli
- npm/agent-database-cli-*.tgz
- npm/*linux-x64*.tgz
- BUILD_INFO.txt

## Install
```bash
# Need Node.js >= 20 first
npm install -g ./npm/agent-database-cli-*.tgz ./npm/*linux-x64*.tgz
agent-database-cli --help
```

Standalone:
```bash
sudo cp ./agent-database-cli /usr/local/bin/agent-database-cli
sudo chmod +x /usr/local/bin/agent-database-cli
```

Config: ~/.agent-database-cli/config.json

Oracle notes:
- default driver needs SQLcl + Java 21
- native driver needs Oracle Instant Client
EOF

echo "[uos-1050] Artifacts:"
find "${OUT_DIR}" -type f | sort
cat "${OUT_DIR}/BUILD_INFO.txt"
