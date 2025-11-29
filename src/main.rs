use clap::{Arg, ArgAction, Command};
use fuser::MountOption;

use crate::kriptofs::KriptoFs;

mod kriptofs;

fn main() {
    let matches = Command::new("kriptofs")
        .arg(
            Arg::new("MOUNT_POINT")
                .required(true)
                .index(1)
                .help("Act as a client, and mount FUSE at given path"),
        )
        .arg(
            Arg::new("auto_unmount")
                .long("auto_unmount")
                .action(ArgAction::SetTrue)
                .help("Automatically unmount on process exit"),
        )
        .arg(
            Arg::new("allow-root")
                .long("allow-root")
                .action(ArgAction::SetTrue)
                .help("Allow root user to access filesystem"),
        )
        .get_matches();
    env_logger::init();
    let mountpoint = matches.get_one::<String>("MOUNT_POINT").unwrap();
    let mut options = vec![
        MountOption::RW,
        MountOption::FSName("kriptofs".to_string()),
        MountOption::AllowOther,
    ];
    if matches.get_flag("auto_unmount") {
        options.push(MountOption::AutoUnmount);
    }
    if matches.get_flag("allow-root") {
        options.push(MountOption::AllowRoot);
    }
    let kripto_filesystem = KriptoFs::new();

    fuser::mount2(kripto_filesystem, mountpoint, &options).unwrap();
}
