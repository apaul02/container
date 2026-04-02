use std::os::unix::process::CommandExt;
use std::{env, path::PathBuf, process::Command};

use nix::mount::{MsFlags, mount};
use nix::sched::CloneFlags;

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
            _ => println!("Error whith the command"),
        },
        None => {
            println!("Error occured")
        }
    }
}

pub fn run(binary: PathBuf, args: Vec<String>) {
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
    let mut child = Command::new(binary)
        .arg("child")
        .arg(&id)
        .args(args)
        .spawn()
        .expect("Failed to spawn");

    child.wait().expect("Failed to wait on child");

    println!("Container has stopped. Cleaning up {}..", container_dir);
    if let Err(e) = std::fs::remove_dir_all(&container_dir) {
        println!("Failed to remove container directory: {}", e);
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
