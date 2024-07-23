use std::io::Read;
use std::io::Write;

use crate::util;
use crate::worker;
use crate::Opt;

pub(crate) const MAX_BUFFER_SIZE: usize = 128 * 1024;
const WRITE_PATHS_PREFIX: &str = "dirload";

#[derive(Clone, Copy, Debug)]
pub(crate) enum WritePathsType {
    Dir,
    Reg,
    Symlink,
    Link, // hardlink
}

impl WritePathsType {
    #[allow(dead_code)]
    pub(crate) fn is_dir(&self) -> bool {
        matches!(self, WritePathsType::Dir)
    }

    pub(crate) fn is_reg(&self) -> bool {
        matches!(self, WritePathsType::Reg)
    }

    #[allow(dead_code)]
    pub(crate) fn is_symlink(&self) -> bool {
        matches!(self, WritePathsType::Symlink)
    }

    pub(crate) fn is_link(&self) -> bool {
        matches!(self, WritePathsType::Link)
    }
}

#[derive(Debug, Default)]
pub(crate) struct ThreadDir {
    read_buffer: Vec<u8>,
    write_buffer: Vec<u8>,
    write_paths: Vec<String>,
    write_paths_counter: u64,
}

impl ThreadDir {
    pub(crate) fn newread(bufsiz: usize) -> Self {
        Self {
            read_buffer: vec![0; bufsiz],
            ..Default::default()
        }
    }

    pub(crate) fn newwrite(bufsiz: usize) -> Self {
        Self {
            write_buffer: vec![0x41; bufsiz],
            ..Default::default()
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct Dir {
    random_write_data: Vec<u8>,
    write_paths_ts: String,
}

impl Dir {
    pub(crate) fn new(random: bool) -> Self {
        let mut dir = Self {
            ..Default::default()
        };
        if random {
            for _ in 0..MAX_BUFFER_SIZE * 2 {
                // doubled
                dir.random_write_data.push(util::get_random(32..128));
            }
        }
        dir.write_paths_ts = util::get_time_string();
        dir
    }
}

pub(crate) fn cleanup_write_paths(tdv: &[&ThreadDir], opt: &Opt) -> std::io::Result<usize> {
    let mut l = vec![];
    for tdir in tdv {
        for f in &tdir.write_paths {
            l.push(f.to_string());
        }
    }

    let mut num_remain = 0;
    if opt.keep_write_paths {
        num_remain += l.len();
    } else {
        unlink_write_paths(&mut l, -1)?;
        num_remain += l.len();
    }
    Ok(num_remain)
}

pub(crate) fn unlink_write_paths(l: &mut Vec<String>, count: isize) -> std::io::Result<()> {
    let mut n = l.len(); // unlink all by default
    if count > 0 {
        n = count.try_into().unwrap();
        if n > l.len() {
            n = l.len();
        }
    }
    println!("Unlink {n} write paths");
    l.sort();

    while n > 0 {
        let f = &l[l.len() - 1];
        let t = util::get_raw_file_type(f)?;
        match t {
            util::FileType::Dir | util::FileType::Reg | util::FileType::Symlink => {
                if util::path_exists_or_error(f).is_err() {
                    continue;
                }
                if t.is_dir() {
                    std::fs::remove_dir(f)?;
                } else {
                    std::fs::remove_file(f)?;
                }
                l.truncate(l.len() - 1);
                n -= 1;
            }
            _ => return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput)),
        }
    }
    Ok(())
}

fn assert_file_path(f: &str) {
    // must always handle file as abs
    assert!(util::is_abspath(f));

    // file must not end with "/"
    assert!(!f.ends_with('/'));
}

pub(crate) fn read_entry(f: &str, thr: &mut worker::Thread, opt: &Opt) -> std::io::Result<()> {
    assert_file_path(f);
    let mut t = util::get_raw_file_type(f)?;

    // stats by dirwalk itself are not counted
    thr.stat.inc_num_stat();

    // ignore . entries if specified
    if opt.ignore_dot && !t.is_dir() && util::is_dot_path(f) {
        return Ok(());
    }

    // beyond this is for file read
    if opt.stat_only {
        return Ok(());
    }

    // find target if symlink
    let mut x;
    if t.is_symlink() {
        x = util::read_link(f)?;
        thr.stat.add_num_read_bytes(x.len());
        if !util::is_abspath(&x) {
            x = util::join_path(&util::get_dirpath(f)?, &x);
            assert!(util::is_abspath(&x));
        }
        t = util::get_file_type(&x)?; // update type
        thr.stat.inc_num_stat(); // count twice for symlink
        assert!(!t.is_symlink()); // symlink chains resolved
        if !opt.follow_symlink {
            return Ok(());
        }
    } else {
        x = f.to_string();
    }

    match t {
        util::FileType::Reg => return read_file(&x, thr, opt),
        util::FileType::Dir | util::FileType::Device | util::FileType::Unsupported => (),
        util::FileType::Symlink => panic!("{x} is symlink"),
    }
    Ok(())
}

fn read_file(f: &str, thr: &mut worker::Thread, opt: &Opt) -> std::io::Result<()> {
    let mut fp = std::fs::File::open(f)?;
    let mut b: &mut [u8] = &mut thr.dir.read_buffer;
    let mut resid = opt.read_size; // negative resid means read until EOF

    if resid == 0 {
        resid = isize::try_from(util::get_random(0..b.len())).unwrap() + 1;
        assert!(resid > 0);
        assert!(resid <= b.len().try_into().unwrap());
    }
    assert!(resid == -1 || resid > 0);

    loop {
        // cut slice size if > positive residual
        if resid > 0 && b.len() > resid.try_into().unwrap() {
            b = &mut b[..resid.try_into().unwrap()];
        }

        let siz = fp.read(b)?;
        thr.stat.inc_num_read();
        thr.stat.add_num_read_bytes(siz);
        if siz == 0 {
            break;
        }

        // end if positive residual becomes <= 0
        if resid > 0 {
            resid -= isize::try_from(siz).unwrap();
            if resid >= 0 {
                if opt.debug {
                    assert_eq!(resid, 0);
                }
                break;
            }
        }
    }
    Ok(())
}

pub(crate) fn write_entry(
    f: &str,
    thr: &mut worker::Thread,
    dir: &Dir,
    opt: &Opt,
) -> std::io::Result<()> {
    assert_file_path(f);
    let t = util::get_raw_file_type(f)?;

    // stats by dirwalk itself are not counted
    thr.stat.inc_num_stat();

    // ignore . entries if specified
    if opt.ignore_dot && !t.is_dir() && util::is_dot_path(f) {
        return Ok(());
    }

    match t {
        util::FileType::Dir => return write_file(f, f, thr, dir, opt),
        util::FileType::Reg => return write_file(&util::get_dirpath(f)?, f, thr, dir, opt),
        util::FileType::Device | util::FileType::Symlink | util::FileType::Unsupported => (),
    }
    Ok(())
}

fn write_file(
    d: &str,
    f: &str,
    thr: &mut worker::Thread,
    dir: &Dir,
    opt: &Opt,
) -> std::io::Result<()> {
    if is_write_done(thr, opt) {
        return Ok(());
    }

    // construct a write path
    let newb = format!(
        "{}_gid{}_{}_{}",
        get_write_paths_base(opt),
        thr.gid,
        dir.write_paths_ts,
        thr.dir.write_paths_counter
    );
    thr.dir.write_paths_counter += 1;
    let newf = util::join_path(d, &newb);

    // create an inode
    let i = util::get_random(0..opt.write_paths_type.len());
    let t = opt.write_paths_type[i];
    create_inode(f, &newf, t)?;
    if opt.fsync_write_paths {
        fsync_inode(&newf)?;
    }
    if opt.dirsync_write_paths {
        fsync_inode(d)?;
    }

    // register the write path, and return unless regular file
    thr.dir.write_paths.push(newf.clone());
    if !t.is_reg() {
        thr.stat.inc_num_write();
        return Ok(());
    }

    // open the write path and start writing
    let mut fp = std::fs::OpenOptions::new().append(true).open(newf)?;
    let mut b: &mut [u8] = &mut thr.dir.write_buffer;
    let mut resid = opt.write_size; // negative resid means no write
    match resid {
        x if x < 0 => {
            thr.stat.inc_num_write();
            return Ok(());
        }
        0 => {
            resid = isize::try_from(util::get_random(0..b.len())).unwrap() + 1;
            assert!(resid > 0);
            assert!(resid <= b.len().try_into().unwrap());
        }
        _ => (),
    }
    assert!(resid > 0);

    if opt.truncate_write_paths {
        fp.set_len(resid.try_into().unwrap())?;
        thr.stat.inc_num_write();
    } else {
        loop {
            // cut slice size if > residual
            if resid > 0 && b.len() > resid.try_into().unwrap() {
                b = &mut b[..resid.try_into().unwrap()];
            }
            if opt.random_write_data {
                let i = util::get_random(0..dir.random_write_data.len() / 2);
                b.clone_from_slice(&dir.random_write_data[i..i + b.len()]);
            }

            let siz = fp.write(b)?;
            thr.stat.inc_num_write();
            thr.stat.add_num_write_bytes(siz);

            // end if residual becomes <= 0
            resid -= isize::try_from(siz).unwrap();
            if resid <= 0 {
                if opt.debug {
                    assert_eq!(resid, 0);
                }
                break;
            }
        }
    }

    if opt.fsync_write_paths {
        fp.flush()?;
    }
    Ok(())
}

fn create_inode(oldf: &str, newf: &str, t: WritePathsType) -> std::io::Result<()> {
    let mut t = t;
    if t.is_link() {
        if util::get_raw_file_type(oldf)?.is_reg() {
            return std::fs::hard_link(oldf, newf);
        }
        t = WritePathsType::Dir; // create a directory instead
    }
    match t {
        WritePathsType::Dir => {
            std::fs::create_dir(newf)?;
        }
        WritePathsType::Reg => {
            std::fs::File::create(newf)?;
        }
        WritePathsType::Symlink => {
            std::os::unix::fs::symlink(oldf, newf)?;
        }
        WritePathsType::Link => (),
    }
    Ok(())
}

fn fsync_inode(f: &str) -> std::io::Result<()> {
    std::fs::File::open(f)?.flush()
}

pub(crate) fn is_write_done(thr: &worker::Thread, opt: &Opt) -> bool {
    if !thr.is_writer(opt) || opt.num_write_paths <= 0 {
        false
    } else {
        thr.dir.write_paths.len() >= opt.num_write_paths.try_into().unwrap()
    }
}

fn get_write_paths_base(opt: &Opt) -> String {
    format!("{}_{}", WRITE_PATHS_PREFIX, opt.write_paths_base)
}

pub(crate) fn collect_write_paths(input: &[String], opt: &Opt) -> std::io::Result<Vec<String>> {
    let b = get_write_paths_base(opt);
    let mut l = vec![];
    for f in util::remove_dup_string(input) {
        for entry in walkdir::WalkDir::new(f)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            let x = util::parse_walkdir_entry(&entry)?;
            let t = util::get_raw_file_type(x)?;
            match t {
                util::FileType::Dir | util::FileType::Reg | util::FileType::Symlink => {
                    if util::get_basename(x)?.starts_with(&b) {
                        l.push(x.to_string());
                    }
                }
                _ => (),
            }
        }
    }
    Ok(l)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_get_write_paths_type_is_xxx() {
        assert!(super::WritePathsType::Dir.is_dir());
        assert!(super::WritePathsType::Reg.is_reg());
        assert!(super::WritePathsType::Symlink.is_symlink());
        assert!(super::WritePathsType::Link.is_link());
    }
}
