use std::ffi::OsStr;
use std::time::{Duration, UNIX_EPOCH};

use fuser::{
    Config, Errno, FileAttr, FileHandle, FileType, Filesystem, FopenFlags, Generation, INodeNo,
    MountOption, OpenAccMode, OpenFlags, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    ReplyOpen, Request,
};

const ROOT_INO: INodeNo = INodeNo::ROOT;
const TEST_INO: INodeNo = INodeNo(2);
const TEST_NAME: &str = "test.md";
const TEST_CONTENT: &[u8] = b"Hello World!\n";
const TTL: Duration = Duration::from_secs(1);

struct BootstrapFs {
    uid: u32,
    gid: u32,
}

impl BootstrapFs {
    fn root_attr(&self) -> FileAttr {
        FileAttr {
            ino: ROOT_INO,
            size: 0,
            blocks: 0,
            atime: UNIX_EPOCH,
            mtime: UNIX_EPOCH,
            ctime: UNIX_EPOCH,
            crtime: UNIX_EPOCH,
            kind: FileType::Directory,
            perm: 0o555,
            nlink: 2,
            uid: self.uid,
            gid: self.gid,
            rdev: 0,
            flags: 0,
            blksize: 512,
        }
    }

    fn test_attr(&self) -> FileAttr {
        FileAttr {
            ino: TEST_INO,
            size: TEST_CONTENT.len() as u64,
            blocks: 1,
            atime: UNIX_EPOCH,
            mtime: UNIX_EPOCH,
            ctime: UNIX_EPOCH,
            crtime: UNIX_EPOCH,
            kind: FileType::RegularFile,
            perm: 0o444,
            nlink: 1,
            uid: self.uid,
            gid: self.gid,
            rdev: 0,
            flags: 0,
            blksize: 512,
        }
    }
}

impl Filesystem for BootstrapFs {
    fn lookup(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
        if parent == ROOT_INO && name == OsStr::new(TEST_NAME) {
            reply.entry(&TTL, &self.test_attr(), Generation(0));
            return;
        }

        reply.error(Errno::ENOENT);
    }

    fn getattr(&self, _req: &Request, ino: INodeNo, _fh: Option<FileHandle>, reply: ReplyAttr) {
        match ino {
            ROOT_INO => reply.attr(&TTL, &self.root_attr()),
            TEST_INO => reply.attr(&TTL, &self.test_attr()),
            _ => reply.error(Errno::ENOENT),
        }
    }

    fn readdir(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        mut reply: ReplyDirectory,
    ) {
        if ino != ROOT_INO {
            reply.error(Errno::ENOENT);
            return;
        }

        let entries = [
            (ROOT_INO, FileType::Directory, "."),
            (ROOT_INO, FileType::Directory, ".."),
            (TEST_INO, FileType::RegularFile, TEST_NAME),
        ];

        for (idx, (entry_ino, kind, name)) in entries.iter().enumerate().skip(offset as usize) {
            let next_offset = (idx + 1) as u64;
            if reply.add(*entry_ino, next_offset, *kind, name) {
                break;
            }
        }

        reply.ok();
    }

    fn open(&self, _req: &Request, ino: INodeNo, flags: OpenFlags, reply: ReplyOpen) {
        if ino != TEST_INO {
            reply.error(Errno::ENOENT);
            return;
        }

        if flags.acc_mode() != OpenAccMode::O_RDONLY {
            reply.error(Errno::EROFS);
            return;
        }

        reply.opened(FileHandle(0), FopenFlags::empty());
    }

    fn read(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        size: u32,
        _flags: OpenFlags,
        _lock_owner: Option<fuser::LockOwner>,
        reply: ReplyData,
    ) {
        if ino != TEST_INO {
            reply.error(Errno::ENOENT);
            return;
        }

        let start = offset as usize;
        if start >= TEST_CONTENT.len() {
            reply.data(&[]);
            return;
        }

        let end = start.saturating_add(size as usize).min(TEST_CONTENT.len());
        reply.data(&TEST_CONTENT[start..end]);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os();
    let _program = args.next();
    let mountpoint = match args.next() {
        Some(path) => path,
        None => {
            return Err("usage: cargo run -- <mountpoint>".into());
        }
    };

    let fs = BootstrapFs {
        uid: unsafe { libc::geteuid() },
        gid: unsafe { libc::getegid() },
    };

    let mut config = Config::default();
    config.mount_options.extend([
        MountOption::RO,
        MountOption::FSName("fs-jira".to_string()),
        MountOption::DefaultPermissions,
    ]);

    fuser::mount2(fs, mountpoint, &config)?;
    Ok(())
}
