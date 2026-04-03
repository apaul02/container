use std::collections::HashSet;
use std::os::unix::process::CommandExt;
use std::{env, path::PathBuf, process::Command};

use nix::mount::{MsFlags, mount};
use nix::sched::CloneFlags;
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;

fn main() {
    let mut args = env::args();

    // The program root
    let _program = args.next();

    let current_exe = std::env::current_exe().unwrap();

    let x = args.next();

    let a = args.collect();

    match x {
        Some(cmd) => match cmd.as_str() {
            "run" => run(current_exe, a),
            "child" => child(current_exe, a),
            "init" => init(a),
            "ps" => ps_command(),
            "rm" => rm_command(a),
            _ => println!("Error whith the command"),
        },
        None => {
            println!("Error occured")
        }
    }
}

pub fn run(binary: PathBuf, mut args: Vec<String>) {
    let id = std::process::id().to_string();

    let container_dir = format!("./containers/{}", id);
    let upper = format!("{}/upper", container_dir);
    let work = format!("{}/work", container_dir);
    let merged = format!("{}/merged", container_dir);
    let dirs = [&upper, &work, &merged];
    for dir in dirs {
        if let Err(e) = std::fs::create_dir_all(dir) {
            println!("Failed to created dirs: {}", e);
        }
    }

    let mut detached_mode = false;
    if let Some(pos) = args.iter().position(|x| x == "-d" || x == "-detach") {
        args.remove(pos);
        detached_mode = true;
        println!("running in detached mode");
    };
    let mut child = Command::new(binary)
        .arg("child")
        .arg(&id)
        .args(args)
        .spawn()
        .expect("Failed to spawn");

    if detached_mode {
        let registry = "./containers/registry";
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(registry)
            .unwrap();
        use std::io::Write;
        writeln!(file, "{}", id).unwrap();
        println!("Container {} started in background.", id);
    } else {
        child.wait().expect("Failed to wait on child");
        println!("Container stopped. Cleaning up {}..", container_dir);
        if let Err(e) = std::fs::remove_dir_all(&container_dir) {
            println!("Failed to remove {}: {}", container_dir, e);
        }
    }
}

pub fn child(binary: PathBuf, args: Vec<String>) {
    if let Err(e) = nix::sched::unshare(
        CloneFlags::CLONE_NEWUTS | CloneFlags::CLONE_NEWNS | CloneFlags::CLONE_NEWPID,
    ) {
        println!("Error occured while setting CloneFlags - newuts: {}", e);
    }
    let _ = mount(
        None::<&str>,
        "/",
        None::<&str>,
        MsFlags::MS_PRIVATE | MsFlags::MS_REC,
        None::<&str>,
    );
    nix::unistd::sethostname("container").unwrap();

    if let Err(e) = std::fs::create_dir_all("/sys/fs/cgroup/container/") {
        println!("Failed to create directory: {}", e);
    }
    if let Err(e) = std::fs::write(
        "/sys/fs/cgroup/container/memory.max",
        String::from("50000000"),
    ) {
        println!("Failed to write in memory.max: {}", e);
    }
    if let Err(e) = std::fs::write("/sys/fs/cgroup/container/memory.swap.max", 0.to_string()) {
        println!("Failed to write in memory.swap.max: {}", e);
    }
    let id = std::process::id();

    if let Err(e) = std::fs::write("/sys/fs/cgroup/container/cgroup.procs", id.to_string()) {
        println!("Failed to write in cgroup.procs: {}", e)
    }

    Command::new(binary)
        .arg("init")
        .arg(&args[0])
        .args(&args[1..])
        .spawn()
        .expect("failed to spawn init")
        .wait()
        .expect("failed to wait");
}

pub fn init(args: Vec<String>) {
    let id = &args[0];

    let container_dir = format!("./containers/{}", id);
    let merged = format!("{}/merged", container_dir);
    let overlay_data = format!(
        "lowerdir=./rootfs,upperdir={}/upper,workdir={}/work",
        container_dir, container_dir
    );
    if let Err(e) = mount(
        Some("overlay"),
        merged.as_str(),
        Some("overlay"),
        MsFlags::empty(),
        Some(overlay_data.as_str()),
    ) {
        println!("Error occured: {}", e);
    }
    nix::unistd::chroot(merged.as_str()).unwrap();
    nix::unistd::chdir("/").unwrap();

    let flags = MsFlags::MS_NOEXEC | MsFlags::MS_NOSUID | MsFlags::MS_NODEV;

    mount(Some("proc"), "/proc", Some("proc"), flags, None::<&str>).unwrap();

    let cmd = &args[1];

    let _ = Command::new(cmd).args(&args[2..]).exec();
}

fn ps_command() {
    let mut pids = HashSet::new();

    for entry in std::fs::read_dir("/proc").unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if let Ok(pid) = name.parse::<u32>() {
            pids.insert(pid);
        }
    }

    for entry in std::fs::read_dir("./containers").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_dir() {
            if let Some(name) = path.file_name() {
                let name = name.to_string_lossy();

                if let Ok(pid) = name.parse::<u32>() {
                    if pids.contains(&pid) {
                        println!("Current Containers: {}", pid);
                    }
                }
            }
        }
    }
}

fn rm_command(args: Vec<String>) {
    if args.is_empty() {
        println!("Please provide a container ID to remove. (e.g., cargo run -- rm 1234)");
        return;
    }
    let id = &args[0];
    let container_dir = format!("./containers/{}", id);
    if let Ok(pid) = id.parse::<i32>() {
        if let Err(e) = kill(Pid::from_raw(pid), Signal::SIGKILL) {
            println!("Failed to kill container {}: {}", id, e);
        } else {
            println!("Killed container {}", id);
        }
    }
    if let Err(e) = std::fs::remove_dir_all(&container_dir) {
        println!("Failed to remove {}: {}", container_dir, e);
    } else {
        println!("Removed {}", container_dir);
    }

    let registry = "./containers/registry";

    if let Ok(contents) = std::fs::read_to_string(registry) {
        let updated: String = contents
            .lines()
            .filter(|line| *line != id)
            .map(|line| format!("{}\n", line))
            .collect();
        std::fs::write(registry, updated).unwrap();
    }
}
