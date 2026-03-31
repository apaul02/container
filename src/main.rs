use std::os::unix::process::CommandExt;
use std::{env, path::PathBuf, process::Command};

fn main() {
    let mut args = env::args();

    // The program root
    let _program = args.next();

    let current_exe = std::env::current_exe().unwrap();

    let x = args.next();

    let a = args.collect();

    match x {
        Some(cmd) => match cmd.as_str() {
            "run" => {
                println!("In the run block");
                let _child = run(current_exe, a).wait().expect("Failed to wait");
            }
            "child" => child(a),
            _ => println!("Error whith the command"),
        },
        None => {
            println!("Error occured")
        }
    }
}

pub fn run(binary: PathBuf, args: Vec<String>) -> std::process::Child {
    Command::new(binary)
        .arg("child")
        .args(args)
        .spawn()
        .expect("Failed to spawn")
}

pub fn child(args: Vec<String>) {
    if let Err(e) = nix::unistd::chroot("./rootfs") {
        println!("Error occured while setting root dir: {}", e);
    }
    if let Err(e) = nix::unistd::chdir("/") {
        println!("Error while setting current working dir: {}", e);
    }
    let cmd = &args[0];
    let _ = Command::new(cmd).args(&args[1..]).exec();
}
