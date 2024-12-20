#![no_std]
// SPDX-License-Identifier: GPL-2.0

//! Rust read-only file system sample.

use kernel::{
    buf::UserSlicePtr,
    c_str,
    code::{EISDIR, ENOENT},
    error::KernelResult,
    fs,
    fs::{
        address_space, dentry, dentry::DEntry, file, file::File, inode, inode::INode, sb, Offset,
    },
    mm::folio::{Folio, PageCache},
    str::CStr,
    time::UNIX_EPOCH,
    types::{ARef, Either, Locked},
};
use kmacro::vtable;

type Result<T> = KernelResult<T>;

kernel::module_fs! {
    type: RoFs,
    name: "rust_rofs",
    author: "Rust for Linux Contributors",
    description: "Rust read-only file system sample",
    license: "GPL",
}

struct Entry {
    name: &'static [u8],
    ino: u64,
    etype: inode::Type,
    contents: &'static [u8],
}

const ENTRIES: [Entry; 4] = [
    Entry {
        name: b".",
        ino: 1,
        etype: inode::Type::Dir,
        contents: b"",
    },
    Entry {
        name: b"..",
        ino: 1,
        etype: inode::Type::Dir,
        contents: b"",
    },
    Entry {
        name: b"test.txt",
        ino: 2,
        etype: inode::Type::Reg,
        contents: b"hello world\n",
    },
    Entry {
        name: b"link.txt",
        ino: 3,
        etype: inode::Type::Lnk(None),
        contents: b"./test.txt",
    },
];

const DIR_FOPS: file::Ops<RoFs> = file::Ops::new::<RoFs>();
const DIR_IOPS: inode::Ops<RoFs> = inode::Ops::new::<RoFs>();
const FILE_AOPS: address_space::Ops<RoFs> = address_space::Ops::new::<RoFs>();

struct RoFs;

impl RoFs {
    fn iget(sb: &sb::SuperBlock<Self>, e: &'static Entry) -> Result<ARef<INode<Self>>> {
        let mut new = match sb.get_or_create_inode(e.ino)? {
            Either::Left(existing) => return Ok(existing),
            Either::Right(new) => new,
        };

        let (mode, nlink, size, typ) = match e.etype {
            inode::Type::Dir => {
                new.set_iops(DIR_IOPS).set_fops(DIR_FOPS);
                (0o555, 2, ENTRIES.len().try_into()?, inode::Type::Dir)
            }
            inode::Type::Reg => {
                new.set_fops(file::Ops::generic_ro_file())
                    .set_aops(FILE_AOPS);
                (0o444, 1, e.contents.len().try_into()?, inode::Type::Reg)
            }
            inode::Type::Lnk(_) => {
                new.set_iops(inode::Ops::simple_symlink_inode());
                (
                    0o444,
                    1,
                    e.contents.len().try_into()?,
                    inode::Type::Lnk(Some(e.contents.try_into()?)),
                )
            }
            _ => return Err(ENOENT),
        };

        new.init(inode::Params {
            typ,
            mode,
            size,
            blocks: (u64::try_from(size)? + 511) / 512,
            nlink,
            uid: 0,
            gid: 0,
            atime: UNIX_EPOCH,
            ctime: UNIX_EPOCH,
            mtime: UNIX_EPOCH,
            value: e,
        })
    }
}

impl fs::FileSystem for RoFs {
    type Data = ();
    type INodeData = &'static Entry;
    const NAME: &'static CStr = c_str!("rust_rofs");

    fn fill_super(sb: &mut sb::SuperBlock<Self, sb::New>, _: Option<inode::Mapper>) -> Result<()> {
        sb.set_magic(0x52555354);
        Ok(())
    }

    fn init_root(sb: &sb::SuperBlock<Self>) -> Result<dentry::Root<Self>> {
        let inode = Self::iget(sb, &ENTRIES[0])?;
        dentry::Root::try_new(inode)
    }
}

#[vtable]
impl inode::Operations for RoFs {
    type FileSystem = Self;

    fn lookup(
        parent: &Locked<&INode<Self>, inode::ReadSem>,
        dentry: dentry::Unhashed<'_, Self>,
    ) -> Result<Option<ARef<DEntry<Self>>>> {
        if parent.ino() != 1 {
            return dentry.splice_alias(None);
        }

        let name = dentry.name();
        for e in &ENTRIES {
            if name == e.name {
                let inode = Self::iget(parent.super_block(), e)?;
                return dentry.splice_alias(Some(inode));
            }
        }

        dentry.splice_alias(None)
    }
}

#[vtable]
impl address_space::Operations for RoFs {
    type FileSystem = Self;

    fn read_folio(
        _: Option<&File<Self>>,
        mut folio: Locked<&Folio<PageCache<Self>>>,
    ) -> Result<()> {
        let data = folio.inode().data().contents;
        let pos = usize::try_from(folio.pos()).unwrap_or(usize::MAX);
        let copied = if pos >= data.len() {
            0
        } else {
            let to_copy = core::cmp::min(data.len() - pos, folio.size());
            folio.write(0, &data[pos..][..to_copy])?;
            to_copy
        };

        folio.zero_out(copied, folio.size() - copied)?;
        folio.mark_uptodate();
        folio.flush_dcache();

        Ok(())
    }
}

#[vtable]
impl file::Operations for RoFs {
    type FileSystem = Self;

    fn seek(file: &File<Self>, offset: Offset, whence: file::Whence) -> Result<Offset> {
        file::generic_seek(file, offset, whence)
    }

    fn read(
        _file: &File<Self::FileSystem>,
        _buffer: &mut UserSlicePtr,
        _offset: &mut Offset,
    ) -> KernelResult<usize> {
        Err(EISDIR)
    }

    fn read_dir(
        _file: &File<Self>,
        inode: &Locked<&INode<Self>, inode::ReadSem>,
        emitter: &mut file::DirEmitter,
    ) -> Result<()> {
        if inode.ino() != 1 {
            return Ok(());
        }

        let pos = emitter.pos();
        if pos >= ENTRIES.len().try_into()? {
            return Ok(());
        }

        for e in ENTRIES.iter().skip(pos.try_into()?) {
            if !emitter.emit(1, e.name, e.ino, (&e.etype).into()) {
                break;
            }
        }

        Ok(())
    }
}
