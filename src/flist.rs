use std::io::BufRead;
use std::io::Write;

use crate::util;

pub(crate) fn init_flist(input: &str, ignore_dot: bool) -> std::io::Result<Vec<String>> {
    let mut l = vec![];
    for entry in walkdir::WalkDir::new(input)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let f = util::parse_walkdir_entry(&entry)?;
        let t = util::get_raw_file_type(f)?;

        // ignore . entries if specified
        if ignore_dot && t != util::DIR && util::is_dot_path(f) {
            continue;
        }

        match t {
            util::DIR => (),
            util::REG => l.push(f.to_string()),
            util::DEVICE => (),
            util::SYMLINK => l.push(f.to_string()),
            util::UNSUPPORTED => (),
            util::INVALID => util::panic_file_type(f, "invalid", t),
            _ => util::panic_file_type(f, "unknown", t),
        }
    }
    Ok(l)
}

pub(crate) fn load_flist_file(flist_file: &str) -> std::io::Result<Vec<String>> {
    let mut fl = vec![];
    let fp = std::fs::File::open(flist_file)?;
    for s in std::io::BufReader::new(fp).lines() {
        match s {
            Ok(v) => fl.push(v),
            Err(e) => return Err(e),
        }
    }
    Ok(fl)
}

pub(crate) fn create_flist_file(
    input: &[String],
    flist_file: &str,
    ignore_dot: bool,
    force: bool,
) -> std::io::Result<()> {
    if util::path_exists_or_error(flist_file).is_ok() {
        if force {
            match std::fs::remove_file(flist_file) {
                Ok(_) => println!("Removed {}", flist_file),
                Err(e) => return Err(e),
            }
        } else {
            return Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists));
        }
    }

    let mut fl = vec![];
    for f in input.iter() {
        match init_flist(f, ignore_dot) {
            Ok(v) => {
                println!("{} files scanned from {}", v.len(), f);
                for s in v.iter() {
                    fl.push(s.to_string())
                }
            }
            Err(e) => return Err(e),
        }
    }
    fl.sort();

    let fp = std::fs::File::create(flist_file)?;
    let mut writer = std::io::BufWriter::new(fp);
    for s in fl.iter() {
        assert!(util::is_abspath(s));
        writeln!(writer, "{}", s)?;
    }
    writer.flush()?;
    Ok(())
}
