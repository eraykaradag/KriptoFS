use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, Request,
};
use libc::{ENOENT, ENOTDIR};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::time::{Duration, UNIX_EPOCH};

const TTL: Duration = Duration::from_secs(1); // 1 second

const HELLO_DIR_ATTR: FileAttr = FileAttr {
    ino: 1,
    size: 0,
    blocks: 0,
    atime: UNIX_EPOCH, // 1970-01-01 00:00:00
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::Directory,
    perm: 0o755,
    nlink: 2,
    uid: 501,
    gid: 20,
    rdev: 0,
    flags: 0,
    blksize: 512,
};
pub struct KriptoFs {
    attrs: BTreeMap<u64, FileAttr>,
    tree: BTreeMap<u64, BTreeMap<String, u64>>,
    parents: BTreeMap<u64, u64>,
    next_inode: u64,
}
impl KriptoFs {
    pub fn new() -> Self {
        let mut fs = Self {
            attrs: BTreeMap::new(),
            tree: BTreeMap::new(),
            parents: BTreeMap::new(),
            next_inode: 2,
        };

        fs.attrs.insert(1, HELLO_DIR_ATTR);
        fs.tree.insert(1, BTreeMap::new());
        fs.parents.insert(1, 1);
        fs
    }
}
impl Filesystem for KriptoFs {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let name_str = name.to_str().unwrap().to_string();

        if let Some(children) = self.tree.get(&parent) {
            if let Some(&ino) = children.get(&name_str) {
                let attr = self.attrs.get(&ino).unwrap();
                reply.entry(&TTL, attr, 0);
                return;
            }
        }
        reply.error(ENOENT);
    }

    fn getattr(&mut self, _req: &Request, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        println!("getattr(ino={})", ino);
        match self.attrs.get(&ino) {
            Some(attr) => {
                reply.attr(&TTL, attr);
            }
            None => reply.error(ENOENT),
        };
    }
    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        match self.attrs.get(&ino) {
            Some(attr) => {
                if attr.kind != FileType::Directory {
                    reply.error(ENOTDIR);
                    return;
                }
            }
            None => {
                reply.error(ENOENT);
                return;
            }
        }

        let parent_ino = *self.parents.get(&ino).unwrap_or(&1);
        let mut entries = vec![
            (ino, FileType::Directory, ".".to_string()),
            (parent_ino, FileType::Directory, "..".to_string()),
        ];

        if let Some(children) = self.tree.get(&ino) {
            for (name, &child_ino) in children {
                let kind = self.attrs.get(&ino).unwrap().kind;
                entries.push((child_ino, kind, name.clone()));
            }
        }

        for (i, (inode, kind, name)) in entries.into_iter().enumerate().skip(offset as usize) {
            if reply.add(inode, (i + 1) as i64, kind, name) {
                break;
            }
        }
        reply.ok();
    }
}
