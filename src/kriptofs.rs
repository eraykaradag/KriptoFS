use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, Request,
};
use libc::{EEXIST, EIO, ENOENT, ENOTDIR, EPERM};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::cryption;

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
    file_data: BTreeMap<u64, Vec<u8>>,
    next_inode: u64,
}
impl KriptoFs {
    pub fn new() -> Self {
        let mut fs = Self {
            attrs: BTreeMap::new(),
            tree: BTreeMap::new(),
            parents: BTreeMap::new(),
            file_data: BTreeMap::new(),
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
                let kind = self.attrs.get(&child_ino).unwrap().kind;
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
    fn mkdir(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        mode: u32,
        umask: u32,
        reply: ReplyEntry,
    ) {
        let name_str = name.to_str().unwrap().to_string();

        if let Some(children) = self.tree.get(&parent) {
            if children.contains_key(&name_str) {
                reply.error(EEXIST);
                return;
            }
        }

        let ino = self.next_inode;
        self.next_inode += 1;
        let ts = SystemTime::now();
        let attr = FileAttr {
            ino,
            size: 0,
            blocks: 0,
            atime: ts,
            mtime: ts,
            ctime: ts,
            crtime: ts,
            kind: FileType::Directory,
            perm: 0o755,
            nlink: 2,
            uid: 501,
            gid: 20,
            rdev: 0,
            flags: 0,
            blksize: 512,
        };

        self.attrs.insert(ino, attr);
        self.tree.insert(ino, BTreeMap::new());
        self.tree.entry(parent).or_default().insert(name_str, ino);
        self.parents.insert(ino, parent);

        reply.entry(&TTL, &attr, 0);
    }
    fn create(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        mode: u32,
        umask: u32,
        flags: i32,
        reply: fuser::ReplyCreate,
    ) {
        let name_str = name.to_str().unwrap().to_string();

        if let Some(children) = self.tree.get(&parent) {
            if children.contains_key(&name_str) {
                reply.error(EEXIST);
                return;
            }
        }
        let uid = _req.uid();
        let gid = _req.gid();
        let ino = self.next_inode;
        self.next_inode += 1;
        let ts = SystemTime::now();
        let attr = FileAttr {
            ino,
            size: 0,
            blocks: 0,
            atime: ts,
            mtime: ts,
            ctime: ts,
            crtime: ts,
            kind: FileType::RegularFile,
            perm: 0o644,
            nlink: 1,
            uid,
            gid,
            rdev: 0,
            flags: 0,
            blksize: 512,
        };

        self.attrs.insert(ino, attr);
        self.file_data.insert(ino, Vec::new());
        self.tree.entry(parent).or_default().insert(name_str, ino);
        self.parents.insert(ino, parent);

        reply.created(&TTL, &attr, 0, 0, 0);
    }
    fn write(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        data: &[u8],
        write_flags: u32,
        flags: i32,
        lock_owner: Option<u64>,
        reply: fuser::ReplyWrite,
    ) {
        if let Some(file_content) = self.file_data.get_mut(&ino) {
            let mut plain_text = if file_content.is_empty() {
                Vec::new()
            } else {
                cryption::decrypt_message(file_content)
            };
            if offset as usize + data.len() > plain_text.len() {
                plain_text.resize(offset as usize + data.len(), 0);
            }
            plain_text[offset as usize..(offset as usize + data.len())].copy_from_slice(data);

            let encrypted_data = cryption::encrypt_message(&plain_text);

            *file_content = encrypted_data;

            if let Some(attr) = self.attrs.get_mut(&ino) {
                attr.size = file_content.len() as u64;
            }

            reply.written(data.len() as u32);
        } else {
            reply.error(ENOENT);
        }
    }
    fn read(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        size: u32,
        flags: i32,
        lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        let file_owner = self.attrs.get(&ino).expect("File not exist").uid;
        if let Some(file_content) = self.file_data.get(&ino) {
            if offset < file_content.len() as i64 {
                let data = &file_content[offset as usize..];
                if _req.uid() == file_owner {
                    let decrypted_data = cryption::decrypt_message(data);
                    reply.data(&decrypted_data);
                } else {
                    let hex_view: String = data.iter().map(|b| format!("{:02X}", b)).collect();
                    let hex_bytes = hex_view.into_bytes();
                    reply.data(&hex_bytes[offset as usize..]);
                }
            } else {
                reply.data(&[]); //EOF
            }
        } else {
            reply.error(ENOENT);
        }
    }
    fn setattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        _atime: Option<fuser::TimeOrNow>,
        _mtime: Option<fuser::TimeOrNow>,
        _ctime: Option<std::time::SystemTime>,
        fh: Option<u64>,
        _crtime: Option<std::time::SystemTime>,
        _chgtime: Option<std::time::SystemTime>,
        _bkuptime: Option<std::time::SystemTime>,
        flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        let file_owner = self.attrs.get(&ino).expect("File not exist").uid;
        if _req.uid() != file_owner {
            reply.error(EPERM);
            return;
        }
        if let Some(new_size) = size {
            if let Some(file_content) = self.file_data.get_mut(&ino) {
                file_content.resize(new_size as usize, 0);
            }
            if let Some(attr) = self.attrs.get_mut(&ino) {
                attr.size = new_size;
            }
        }
        reply.attr(&TTL, self.attrs.get(&ino).unwrap());
    }
    fn statfs(&mut self, _req: &Request<'_>, _ino: u64, reply: fuser::ReplyStatfs) {
        let block_size = 512;

        let total_capacity_bytes: u64 = 1024 * 1024 * 1024;
        let total_blocks = total_capacity_bytes / block_size;

        let mut used_bytes: u64 = 0;
        for data in self.file_data.values() {
            used_bytes += data.len() as u64;
        }

        let used_blocks = (used_bytes + block_size - 1) / block_size;

        let free_blocks = total_blocks.saturating_sub(used_blocks);
        reply.statfs(
            total_blocks,
            free_blocks,
            free_blocks,
            1000000,
            1000000 - self.attrs.len() as u64,
            block_size as u32,
            255,
            block_size as u32,
        );
    }
}
