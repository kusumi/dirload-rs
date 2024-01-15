use std::io::Read;
use std::io::Write;

use crate::util;
use crate::worker;
use crate::UserOpt;

pub const MAX_BUFFER_SIZE: usize = 128 * 1024;
const WRITE_PATHS_PREFIX: &str = "dirload";

#[derive(Debug, Default)]
pub struct ThreadDir {
    read_buffer: Vec<u8>,
    write_buffer: Vec<u8>,
    write_paths: Vec<String>,
    write_paths_counter: u64,
}

pub fn newread(bufsiz: usize) -> ThreadDir {
    ThreadDir {
        read_buffer: vec![0; bufsiz],
        ..Default::default()
    }
}

pub fn newwrite(bufsiz: usize) -> ThreadDir {
    ThreadDir {
        write_buffer: vec![0x41; bufsiz],
        ..Default::default()
    }
}

#[derive(Debug, Default)]
pub struct Dir {
    random_write_data: Vec<u8>,
    write_paths_ts: String,
    write_paths_type: Vec<util::FileType>,
}

pub fn newdir(random: bool, write_paths_type: &str) -> Dir {
    let mut dir = Dir {
        ..Default::default()
    };
    if random {
        for _ in 0..MAX_BUFFER_SIZE * 2 {
            // doubled
            dir.random_write_data.push(util::get_random(32..128));
        }
    }
    dir.write_paths_ts = util::get_time_string();

    assert!(!write_paths_type.is_empty());
    for x in write_paths_type.chars() {
        dir.write_paths_type.push(match x {
            'd' => util::DIR,
            'r' => util::REG,
            's' => util::SYMLINK,
            'l' => util::LINK,
            _ => panic!("{}", x),
        });
    }
    dir
}

pub fn cleanup_write_paths(tdv: &[&ThreadDir], opt: &UserOpt) -> std::io::Result<usize> {
    let mut l = vec![];
    for tdir in tdv.iter() {
        for f in tdir.write_paths.iter() {
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

pub fn unlink_write_paths(l: &mut Vec<String>, count: isize) -> std::io::Result<()> {
    let mut n = l.len(); // unlink all by default
    if count > 0 {
        n = count as usize;
        if n > l.len() {
            n = l.len();
        }
    }
    println!("Unlink {} write paths", n);
    l.sort();

    while n > 0 {
        let f = &l[l.len() - 1];
        let t = util::get_raw_file_type(f)?;
        if t == util::DIR || t == util::REG || t == util::SYMLINK {
            if util::path_exists(f).is_err() {
                continue;
            }
            if t == util::DIR {
                std::fs::remove_dir(f)?;
            } else {
                std::fs::remove_file(f)?;
            }
            l.truncate(l.len() - 1);
            n -= 1;
        } else {
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
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

pub fn read_entry(f: &str, thr: &mut worker::Thread, opt: &UserOpt) -> std::io::Result<()> {
    assert_file_path(f);
    let mut t = util::get_raw_file_type(f)?;

    // stats by dirwalk itself are not counted
    thr.stat.inc_num_stat();

    // ignore . entries if specified
    if opt.ignore_dot && t != util::DIR && util::is_dot_path(f) {
        return Ok(());
    }

    // beyond this is for file read
    if opt.stat_only {
        return Ok(());
    }

    let mut x = f.to_string();

    // find target if symlink
    if t == util::SYMLINK {
        let l = x.clone();
        x = util::read_link(&x)?;
        thr.stat.add_num_read_bytes(x.len());
        if !util::is_abspath(&x) {
            x = util::join_path(&util::get_dirpath(&l)?, &x);
            assert!(util::is_abspath(&x));
        }
        t = util::get_file_type(&x)?;
        thr.stat.inc_num_stat(); // count twice for symlink
        assert!(t != util::SYMLINK); // symlink chains resolved
        if opt.lstat {
            return Ok(());
        }
    }

    match t {
        util::DIR => (),
        util::REG => return read_file(&x, thr, opt),
        util::DEVICE => (),
        util::UNSUPPORTED => (),
        util::INVALID => util::panic_file_type(&x, "invalid", t),
        _ => util::panic_file_type(&x, "unknown", t),
    }
    Ok(())
}

fn read_file(f: &str, thr: &mut worker::Thread, opt: &UserOpt) -> std::io::Result<()> {
    let mut fp = std::fs::File::open(f)?;
    let mut b: &mut [u8] = &mut thr.dir.read_buffer;
    let mut resid = opt.read_size; // negative resid means read until EOF

    if resid == 0 {
        resid = util::get_random(0..b.len()) as isize + 1;
        assert!(resid > 0);
        assert!(resid as usize <= b.len());
    }
    assert!(resid == -1 || resid > 0);

    loop {
        // cut slice size if > positive residual
        if resid > 0 && b.len() > resid as usize {
            b = &mut b[..resid as usize];
        }

        let siz = fp.read(b)?;
        thr.stat.inc_num_read();
        thr.stat.add_num_read_bytes(siz);
        if siz == 0 {
            break;
        }

        // end if positive residual becomes <= 0
        if resid > 0 {
            resid -= siz as isize;
            if resid >= 0 {
                if opt.debug {
                    assert!(resid == 0);
                }
                break;
            }
        }
    }
    Ok(())
}

pub fn write_entry(
    f: &str,
    thr: &mut worker::Thread,
    dir: &Dir,
    opt: &UserOpt,
) -> std::io::Result<()> {
    assert_file_path(f);
    let t = util::get_raw_file_type(f)?;

    // stats by dirwalk itself are not counted
    thr.stat.inc_num_stat();

    // ignore . entries if specified
    if opt.ignore_dot && t != util::DIR && util::is_dot_path(f) {
        return Ok(());
    }

    match t {
        util::DIR => return write_file(f, f, thr, dir, opt),
        util::REG => return write_file(&util::get_dirpath(f)?, f, thr, dir, opt),
        util::DEVICE => (),
        util::SYMLINK => (),
        util::UNSUPPORTED => (),
        util::INVALID => util::panic_file_type(f, "invalid", t),
        _ => util::panic_file_type(f, "unknown", t),
    }
    Ok(())
}

fn write_file(
    d: &str,
    f: &str,
    thr: &mut worker::Thread,
    dir: &Dir,
    opt: &UserOpt,
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
    let i = util::get_random(0..dir.write_paths_type.len());
    let t = dir.write_paths_type[i];
    create_inode(f, &newf, t)?;
    if opt.fsync_write_paths {
        fsync_inode(&newf)?;
    }
    if opt.dirsync_write_paths {
        fsync_inode(d)?;
    }

    // register the write path, and return unless regular file
    thr.dir.write_paths.push(newf.clone());
    if t != util::REG {
        thr.stat.inc_num_write();
        return Ok(());
    }

    // open the write path and start writing
    let mut fp = std::fs::OpenOptions::new()
        .write(true)
        .append(true)
        .open(newf)?;
    let mut b: &mut [u8] = &mut thr.dir.write_buffer;
    let mut resid = opt.write_size; // negative resid means no write
    match resid {
        x if x < 0 => {
            thr.stat.inc_num_write();
            return Ok(());
        }
        0 => {
            resid = util::get_random(0..b.len() as isize) + 1;
            assert!(resid > 0);
            assert!(resid <= b.len() as isize);
        }
        _ => (),
    }
    assert!(resid > 0);

    if opt.truncate_write_paths {
        fp.set_len(resid as u64)?;
        thr.stat.inc_num_write();
    } else {
        loop {
            // cut slice size if > residual
            if resid > 0 && b.len() > resid as usize {
                b = &mut b[..resid as usize];
            }
            if opt.random_write_data {
                let i = util::get_random(0..dir.random_write_data.len() / 2);
                b.clone_from_slice(&dir.random_write_data[i..i + b.len()]);
            }

            let siz = fp.write(b)?;
            thr.stat.inc_num_write();
            thr.stat.add_num_write_bytes(siz);

            // end if residual becomes <= 0
            resid -= siz as isize;
            if resid <= 0 {
                if opt.debug {
                    assert!(resid == 0);
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

fn create_inode(oldf: &str, newf: &str, t: util::FileType) -> std::io::Result<()> {
    let mut t = t;
    if t == util::LINK {
        if util::get_raw_file_type(oldf)? == util::REG {
            return std::fs::hard_link(oldf, newf);
        }
        t = util::DIR // create a directory instead
    }

    if t == util::DIR {
        std::fs::create_dir(newf)?;
    } else if t == util::REG {
        std::fs::File::create(newf)?;
    } else if t == util::SYMLINK {
        std::os::unix::fs::symlink(oldf, newf)?;
    }
    Ok(())
}

fn fsync_inode(f: &str) -> std::io::Result<()> {
    std::fs::File::open(f)?.flush()
}

pub fn is_write_done(thr: &worker::Thread, opt: &UserOpt) -> bool {
    if !worker::is_writer(thr, opt) || opt.num_write_paths <= 0 {
        false
    } else {
        thr.dir.write_paths.len() as isize >= opt.num_write_paths
    }
}

fn get_write_paths_base(opt: &UserOpt) -> String {
    format!("{}_{}", WRITE_PATHS_PREFIX, opt.write_paths_base)
}

pub fn collect_write_paths(input: &[String], opt: &UserOpt) -> std::io::Result<Vec<String>> {
    let b = get_write_paths_base(opt);
    let mut l = vec![];
    for f in util::remove_dup_string(input) {
        for entry in walkdir::WalkDir::new(f).into_iter().filter_map(|e| e.ok()) {
            let x = util::parse_walkdir_entry(&entry)?;
            let t = util::get_raw_file_type(x)?;
            if (t == util::DIR || t == util::REG || t == util::SYMLINK)
                && util::get_basename(x)?.starts_with(&b)
            {
                l.push(x.to_string());
            }
        }
    }
    Ok(l)
}
