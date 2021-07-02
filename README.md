**OBSOLETE**

**The same thing this tool tries to do can be achieved by systemd:**

```sh
systemd-run --user --scope -p MemoryMax=1G firefox
```

# firefox-memlimit

Run Firefox in a temporary cgroup to limit its RAM usage.

## Installation

The installed binary needs the setuid bit to be set to create cgroups.

To build and install run:

```sh
cargo build --release
sudo install -o root -g root -m 04711 -s target/release/firefox-memlimit /usr/local/bin/firefox-memlimit
```
