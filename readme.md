# CoreBox: A Linux Container Runtime in Rust

A lightweight, from-scratch Linux container runtime built entirely in Rust. This project mimics the core functionality of professional runtimes like `runc`, demonstrating how Docker and other container engines interact with the Linux kernel to isolate processes.

## Features

- **Filesystem Isolation:** Uses `chroot` and `chdir` to jail processes.
- **Process Isolation (Namespaces):** Utilizes Linux `clone/unshare` syscalls to isolate UTS (Hostname), Mount, and PID namespaces.
- **Resource Limiting (Cgroups v2):** Dynamically provisions control groups to restrict memory usage and disable swap space, preventing host starvation.
- **Image Layering (OverlayFS):** Implements a Union Filesystem to allow multiple containers to share a single read-only base image while maintaining their own isolated read-write state.

## Prerequisites

- **OS:** Linux (or WSL2 on Windows)
- **Privileges:** `root` access (required for namespace manipulation and mounting)
- **Rust:** Latest stable toolchain

## Quick Start

### 1. Procure the Base Image

Before running the container, you need a base root filesystem. We use the Alpine Linux mini-rootfs.

```bash
mkdir rootfs
wget https://dl-cdn.alpinelinux.org/alpine/v3.19/releases/x86_64/alpine-minirootfs-3.19.1-x86_64.tar.gz
tar -xzf alpine-minirootfs-3.19.1-x86_64.tar.gz -C rootfs/
rm alpine-minirootfs-3.19.1-x86_64.tar.gz
```

### 2. Build the Project

```bash
cargo build
```

### 3. Launch a container

You must run the executable as `root`. Pass the command you want to execute inside the container as the final arguments.

```bash
sudo ./target/debug/container run /bin/sh
```

### 4. Isolation

Once Inside the you can verify the containerization:

- `hostname` - Will print `container` instead of your hostname
- `cat /etc/os-release` - Will print alpine linux details.
- `cat /etc/os-release` - Will only show your shell process, hiding all host processes.

## Architecture

This runtime utilizes a three-stage process execution model to bypass Linux kernel constraints

1. **The Boss**(`run`): The initial host process. Generates a unique instance ID, provisions the OverlayFS directories, and spawns the Worker process.
2. **The Worker**(`child`): Unshares the Linux namespaces (putting on the blindfolds), creates the cgroup resource limits, and spawns the Init process.
3. **The Init**(`init`): Mounts the OverlayFS and `/proc`, locks the door with `chroot`, and replaces its own execution thread with the user's requested payload using `exec`.

## Future Additions

- [ ] The `ps` command.
- [ ] `-d` detached mode.
- [ ] More security `caps` crate.
- [ ] Give Internet.
