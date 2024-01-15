use rand::distributions::uniform::SampleRange;
use rand::Rng;
use std::os::unix::fs::FileTypeExt;

// XXX Rust has std::fs::FileType
pub type FileType = i32;

pub const DIR: FileType = 0;
pub const REG: FileType = 1;
pub const DEVICE: FileType = 2;
pub const SYMLINK: FileType = 3;
pub const UNSUPPORTED: FileType = 4;
pub const INVALID: FileType = 5;
pub const LINK: FileType = 6; // hardlink

pub fn read_link(f: &str) -> std::io::Result<String> {
    let p = std::fs::read_link(f)?;
    Ok(p.into_os_string().into_string().unwrap())
}

pub fn get_abspath(f: &str) -> std::io::Result<String> {
    let p = std::fs::canonicalize(f)?; // XXX keep symlink unresolved
    Ok(p.into_os_string().into_string().unwrap())
}

pub fn get_dirpath(f: &str) -> std::io::Result<String> {
    let p = std::path::Path::new(f)
        .parent()
        .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?;
    Ok(p.to_str().unwrap().to_string())
}

pub fn get_basename(f: &str) -> std::io::Result<String> {
    let s = std::path::Path::new(f)
        .file_name()
        .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?;
    Ok(s.to_str().unwrap().to_string())
}

pub fn is_abspath(f: &str) -> bool {
    //f == get_abspath(f).unwrap() // XXX doesn't work with symlink
    &f[0..1] == "/"
}

// XXX behaves differently from filepath.Join which resolves ".." entries
pub fn join_path(f1: &str, f2: &str) -> String {
    let p = std::path::Path::new(f1);
    p.join(f2).as_path().to_str().unwrap().to_string()
}

#[allow(dead_code)]
pub fn is_linux() -> bool {
    std::env::consts::OS == "linux"
}

pub fn is_windows() -> bool {
    std::env::consts::OS == "windows"
}

pub fn get_path_separator() -> char {
    std::path::MAIN_SEPARATOR
}

pub fn get_raw_file_type(f: &str) -> std::io::Result<FileType> {
    match std::fs::symlink_metadata(f) {
        Ok(v) => Ok(get_mode_type(&v.file_type())),
        Err(e) => Err(e),
    }
}

pub fn get_file_type(f: &str) -> std::io::Result<FileType> {
    match std::fs::metadata(f) {
        Ok(v) => Ok(get_mode_type(&v.file_type())),
        Err(e) => Err(e),
    }
}

pub fn get_mode_type(t: &std::fs::FileType) -> FileType {
    if t.is_dir() {
        DIR
    } else if t.is_file() {
        REG
    } else if t.is_symlink() {
        SYMLINK
    } else if t.is_block_device() || t.is_char_device() {
        DEVICE
    } else {
        UNSUPPORTED
    }
}

pub fn path_exists(f: &str) -> std::io::Result<std::fs::Metadata> {
    std::fs::symlink_metadata(f)
}

pub fn is_dot_path(f: &str) -> bool {
    match get_basename(f) {
        Ok(v) => v.starts_with('.') || f.contains("/."),
        Err(_) => false,
    }
}

pub fn is_dir_writable(f: &str) -> std::io::Result<bool> {
    if get_raw_file_type(f)? != DIR {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
    }

    let x = join_path(f, &format!("dirload_write_test_{}", get_time_string()));
    match std::fs::create_dir(&x) {
        Ok(_) => {
            match std::fs::remove_dir(&x) {
                Ok(_) => Ok(true), // read+write
                Err(e) => Err(e),
            }
        }
        Err(_) => Ok(false), // assume readonly
    }
}

pub fn remove_dup_string(input: &[String]) -> Vec<&str> {
    let mut l = vec![];
    for a in input.iter() {
        let mut exists = false;
        for b in l.iter() {
            if a.as_str() == *b {
                exists = true;
            }
        }
        if !exists {
            l.push(a.as_str());
        }
    }
    l
}

pub fn panic_file_type(f: &str, how: &str, t: FileType) {
    if !f.is_empty() {
        panic!("{} has {} file type {}", f, how, t);
    } else {
        panic!("{} file type {}", how, t);
    }
}

pub fn get_time_string() -> String {
    let dt: time::OffsetDateTime = std::time::SystemTime::now().into();
    let fmt = time::format_description::parse("[year][month][day][hour][minute][second]").unwrap();
    dt.format(&fmt).unwrap()
}

pub fn get_random<R, T>(range: R) -> T
where
    R: SampleRange<T>,
    T: rand::distributions::uniform::SampleUniform,
{
    rand::thread_rng().gen_range(range)
}

pub fn parse_walkdir_entry(entry: &walkdir::DirEntry) -> std::io::Result<&str> {
    match entry.path().to_str() {
        Some(v) => Ok(v),
        None => Err(std::io::Error::from(std::io::ErrorKind::InvalidInput)),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_is_windnows() {
        assert!(!super::is_windows());
    }

    #[test]
    fn test_get_path_separator() {
        assert_eq!(super::get_path_separator(), '/');
    }

    #[test]
    fn test_get_raw_file_type() {
        let dir_list = [".", "..", "/", "/dev"];
        for f in dir_list.iter() {
            match super::get_raw_file_type(f) {
                Ok(v) => match v {
                    super::DIR => (),
                    x => panic!("{}", x),
                },
                Err(e) => panic!("{}", e),
            }
        }
        let invalid_list = ["", "516e7cb4-6ecf-11d6-8ff8-00022d09712b"];
        for f in invalid_list.iter() {
            if let Ok(v) = super::get_raw_file_type(f) {
                panic!("{}", v);
            }
        }
    }

    #[test]
    fn test_get_file_type() {
        let dir_list = [".", "..", "/", "/dev"];
        for f in dir_list.iter() {
            match super::get_file_type(f) {
                Ok(v) => match v {
                    super::DIR => (),
                    x => panic!("{}", x),
                },
                Err(e) => panic!("{}", e),
            }
        }
        let invalid_list = ["", "516e7cb4-6ecf-11d6-8ff8-00022d09712b"];
        for f in invalid_list.iter() {
            if let Ok(v) = super::get_file_type(f) {
                panic!("{}", v);
            }
        }
    }

    #[test]
    fn test_path_exists() {
        let dir_list = [".", "..", "/", "/dev"];
        for f in dir_list.iter() {
            if let Err(e) = super::path_exists(f) {
                panic!("{}", e);
            }
        }
        let invalid_list = ["", "516e7cb4-6ecf-11d6-8ff8-00022d09712b"];
        for f in invalid_list.iter() {
            if super::path_exists(f).is_ok() {
                panic!("{}", f);
            }
        }
    }

    #[test]
    fn test_is_dot_path() {
        // XXX commented out paths behave differently vs dirload
        let dot_list = [
            //"/.",
            //"/..",
            //"./", // XXX
            //"./.",
            //"./..",
            //".",
            //"..",
            ".git",
            "..git",
            "/path/to/.",
            //"/path/to/..",
            "/path/to/.git/xxx",
            "/path/to/.git/.xxx",
            "/path/to/..git/xxx",
            "/path/to/..git/.xxx",
        ];
        for (i, f) in dot_list.iter().enumerate() {
            if !super::is_dot_path(f) {
                panic!("{} {}", i, f);
            }
        }

        let non_dot_list = [
            "/",
            "xxx",
            "xxx.",
            "xxx..",
            "/path/to/xxx",
            "/path/to/xxx.",
            "/path/to/x.xxx.",
            "/path/to/git./xxx",
            "/path/to/git./xxx.",
            "/path/to/git./x.xxx.",
        ];
        for (i, f) in non_dot_list.iter().enumerate() {
            if super::is_dot_path(f) {
                panic!("{} {}", i, f);
            }
        }
    }

    #[test]
    fn test_is_dir_writable() {
        if !super::is_linux() {
            return;
        }

        let writable_list = ["/tmp"];
        for (i, f) in writable_list.iter().enumerate() {
            match super::is_dir_writable(f) {
                Ok(v) => {
                    if !v {
                        panic!("{} {}", i, v);
                    }
                }
                Err(e) => panic!("{} {}", i, e),
            }
        }

        let unwritable_list = ["/proc"];
        for (i, f) in unwritable_list.iter().enumerate() {
            match super::is_dir_writable(f) {
                Ok(v) => {
                    if v {
                        panic!("{} {}", i, v);
                    }
                }
                Err(e) => panic!("{} {}", i, e),
            }
        }

        let invalid_list = ["/proc/vmstat", "516e7cb4-6ecf-11d6-8ff8-00022d09712b"];
        for (i, f) in invalid_list.iter().enumerate() {
            if let Ok(v) = super::is_dir_writable(f) {
                if v {
                    panic!("{} {}", i, v);
                }
            }
        }
    }

    #[test]
    fn test_remove_dup_string() {
        let uniq_ll = vec![
            vec!["".to_string()],
            vec!["/path/to/xxx".to_string()],
            vec!["/path/to/xxx".to_string(), "/path/to/yyy".to_string()],
            vec!["xxx1".to_string(), "xxx2".to_string()],
            vec!["xxx1".to_string(), "xxx2".to_string(), "xxx3".to_string()],
            vec![
                "xxx1".to_string(),
                "xxx2".to_string(),
                "xxx3".to_string(),
                "xxx4".to_string(),
            ],
            vec![
                "xxx1".to_string(),
                "xxx2".to_string(),
                "xxx3".to_string(),
                "".to_string(),
            ],
            vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string(),
                "e".to_string(),
                "f".to_string(),
            ],
        ];
        for l in uniq_ll.iter() {
            let x = super::remove_dup_string(l.as_slice());
            for (i, a) in x.iter().enumerate() {
                for (j, b) in x.iter().enumerate() {
                    if i != j && a == b {
                        panic!("{:?}: {} {} == {} {}", l, i, a, j, b);
                    }
                }
            }
            if l.len() != x.len() {
                panic!("{:?}: {} != {}", l, l.len(), x.len());
            }
            for i in 0..x.len() {
                if x[i] != l[i].as_str() {
                    panic!("{:?}: {} {} != {}", l, i, x[i], l[i]);
                }
            }
        }

        let dup_ll = vec![
            vec!["".to_string(), "".to_string()],
            vec!["".to_string(), "".to_string(), "".to_string()],
            vec!["/path/to/xxx".to_string(), "/path/to/xxx".to_string()],
            vec!["xxx1".to_string(), "xxx2".to_string(), "xxx1".to_string()],
            vec![
                "xxx1".to_string(),
                "xxx2".to_string(),
                "xxx1".to_string(),
                "xxx1".to_string(),
            ],
            vec![
                "xxx1".to_string(),
                "xxx1".to_string(),
                "xxx2".to_string(),
                "xxx1".to_string(),
            ],
            vec![
                "xxx1".to_string(),
                "xxx2".to_string(),
                "xxx1".to_string(),
                "xxx2".to_string(),
            ],
            vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string(),
                "e".to_string(),
                "f".to_string(),
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string(),
                "e".to_string(),
                "f".to_string(),
            ],
        ];
        for l in dup_ll.iter() {
            let x = super::remove_dup_string(l.as_slice());
            for (i, a) in x.iter().enumerate() {
                for (j, b) in x.iter().enumerate() {
                    if i != j && a == b {
                        panic!("{:?}: {} {} == {} {}", l, i, a, j, b);
                    }
                }
            }
            if l.len() <= x.len() {
                panic!("{:?}: {} <= {}", l, l.len(), x.len());
            }
            let mut v = vec![];
            for s in x.iter() {
                v.push(s.to_string());
            }
            let xx = super::remove_dup_string(&v);
            if x.len() != xx.len() {
                panic!("{:?}: {} != {}", l, x.len(), xx.len());
            }
            for i in 0..x.len() {
                if x[i] != xx[i] {
                    panic!("{:?}: {} {} != {}", l, i, x[i], xx[i]);
                }
            }
        }
    }

    #[test]
    fn test_get_random() {
        for i in 1..10000 {
            let x = super::get_random(0..i);
            if x < 0 || x >= i {
                panic!("{} {}", i, x);
            }
        }
        for i in 1..10000 {
            let x = super::get_random(-i..0);
            if x < -i || x >= 0 {
                panic!("{} {}", i, x);
            }
        }
    }
}