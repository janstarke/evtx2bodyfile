use std::fmt;

pub struct BodyfileLine {
    md5: String,
    name: String,
    inode: u32,
    mode_as_string: String,
    uid: u32,
    gid: u32,
    size: u32,
    atime: i64,
    mtime: i64,
    ctime: i64,
    crtime: i64,
}

impl BodyfileLine {
    pub fn new(name: String, ctime: i64) -> Self {
        Self {
            md5: "0".to_owned(),
            name: name,
            inode: 0,
            mode_as_string: "0".to_owned(),
            uid: 0,
            gid: 0,
            size: 0,
            atime: ctime,
            mtime: ctime,
            ctime: ctime,
            crtime: ctime,
        }
    }
}

impl fmt::Display for BodyfileLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.md5,
            self.name,
            self.inode,
            self.mode_as_string,
            self.uid,
            self.gid,
            self.size,
            self.atime,
            self.mtime,
            self.ctime,
            self.crtime)
    }
}