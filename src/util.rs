use path_clean::PathClean;
use rand::distributions::uniform::SampleRange;
use rand::Rng;
use std::os::unix::fs::FileTypeExt;

macro_rules! function {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        type_name_of(f).strip_suffix("::f").unwrap()
    }};
}
pub(crate) use function;

#[derive(Debug)]
pub(crate) enum FileType {
    Dir,
    Reg,
    Device,
    Symlink,
    Unsupported,
}

impl FileType {
    pub(crate) fn is_dir(&self) -> bool {
        matches!(self, FileType::Dir)
    }

    pub(crate) fn is_reg(&self) -> bool {
        matches!(self, FileType::Reg)
    }

    #[allow(dead_code)]
    pub(crate) fn is_device(&self) -> bool {
        matches!(self, FileType::Device)
    }

    pub(crate) fn is_symlink(&self) -> bool {
        matches!(self, FileType::Symlink)
    }

    #[allow(dead_code)]
    pub(crate) fn is_unsupported(&self) -> bool {
        matches!(self, FileType::Unsupported)
    }
}

pub(crate) fn read_link(f: &str) -> std::io::Result<String> {
    std::fs::read_link(f)?
        .into_os_string()
        .into_string()
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))
}

// This function
// * does not resolve symlink
// * works with non existent path
pub(crate) fn get_abspath(f: &str) -> std::io::Result<String> {
    let p = std::path::Path::new(f);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()?.join(f)
    }
    .clean()
    .into_os_string()
    .into_string()
    .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))
}

// fails if f is "/" or equivalent
pub(crate) fn get_dirpath(f: &str) -> std::io::Result<String> {
    Ok(std::path::Path::new(&get_abspath(f)?)
        .parent()
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?
        .to_str()
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?
        .to_string())
}

// fails if f is "/" or equivalent
pub(crate) fn get_basename(f: &str) -> std::io::Result<String> {
    Ok(std::path::Path::new(&get_abspath(f)?)
        .file_name()
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?
        .to_str()
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?
        .to_string())
}

pub(crate) fn is_abspath(f: &str) -> bool {
    std::path::Path::new(f).is_absolute()
}

pub(crate) fn is_dir(f: &str) -> bool {
    if let Ok(v) = std::fs::metadata(f) {
        v.file_type().is_dir()
    } else {
        false
    }
}

// XXX behaves differently from filepath.Join which resolves ".." entries
pub(crate) fn join_path(f1: &str, f2: &str) -> String {
    std::path::Path::new(f1)
        .join(f2)
        .as_path()
        .to_str()
        .unwrap()
        .to_string()
}

pub(crate) fn get_home_path() -> String {
    home::home_dir()
        .unwrap()
        .into_os_string()
        .into_string()
        .unwrap()
}

#[allow(dead_code)]
pub(crate) fn is_linux() -> bool {
    std::env::consts::OS == "linux"
}

pub(crate) fn is_windows() -> bool {
    std::env::consts::OS == "windows"
}

pub(crate) fn get_path_separator() -> char {
    std::path::MAIN_SEPARATOR
}

pub(crate) fn get_raw_file_type(f: &str) -> std::io::Result<FileType> {
    match std::fs::symlink_metadata(f) {
        Ok(v) => Ok(get_mode_type(v.file_type())),
        Err(e) => Err(e),
    }
}

pub(crate) fn get_file_type(f: &str) -> std::io::Result<FileType> {
    match std::fs::metadata(f) {
        Ok(v) => Ok(get_mode_type(v.file_type())),
        Err(e) => Err(e),
    }
}

fn get_mode_type(t: std::fs::FileType) -> FileType {
    if t.is_dir() {
        FileType::Dir
    } else if t.is_file() {
        FileType::Reg
    } else if t.is_symlink() {
        FileType::Symlink
    } else if t.is_block_device() || t.is_char_device() {
        FileType::Device
    } else {
        FileType::Unsupported
    }
}

// do not resolve symlink in this implementation
pub(crate) fn path_exists_or_error(f: &str) -> std::io::Result<std::fs::Metadata> {
    std::fs::symlink_metadata(f)
}

// not usable as this resolves symlink
#[allow(dead_code)]
pub(crate) fn path_exists(f: &str) -> bool {
    std::path::Path::new(f).exists()
}

pub(crate) fn is_dot_path(f: &str) -> bool {
    match get_basename(f) {
        Ok(v) => v.starts_with('.') || f.contains("/."),
        Err(_) => false,
    }
}

pub(crate) fn is_dir_writable(f: &str) -> std::io::Result<bool> {
    if !get_raw_file_type(f)?.is_dir() {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
    }

    let x = join_path(f, &format!("dirload_write_test_{}", get_time_string()));
    match std::fs::create_dir(&x) {
        Ok(()) => {
            match std::fs::remove_dir(&x) {
                Ok(()) => Ok(true), // read+write
                Err(e) => Err(e),
            }
        }
        Err(_) => Ok(false), // assume readonly
    }
}

pub(crate) fn remove_dup_string(input: &[String]) -> Vec<&str> {
    let mut l = vec![];
    for a in input {
        let mut exists = false;
        for b in &l {
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

pub(crate) fn get_time_string() -> String {
    let dt: time::OffsetDateTime = std::time::SystemTime::now().into();
    let fmt = time::format_description::parse("[year][month][day][hour][minute][second]").unwrap();
    dt.format(&fmt).unwrap()
}

pub(crate) fn get_random<R, T>(range: R) -> T
where
    R: SampleRange<T>,
    T: rand::distributions::uniform::SampleUniform,
{
    rand::thread_rng().gen_range(range)
}

pub(crate) fn parse_walkdir_entry(entry: &walkdir::DirEntry) -> std::io::Result<&str> {
    entry
        .path()
        .to_str()
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))
}

pub(crate) struct Timer {
    time_begin: std::time::SystemTime,
    duration: u64,
    frequency: u64,
    counter: u64,
}

impl Default for Timer {
    fn default() -> Timer {
        Timer {
            time_begin: std::time::SystemTime::now(),
            duration: 0,
            frequency: 0,
            counter: 0,
        }
    }
}

impl Timer {
    pub(crate) fn new(duration: u64, frequency: u64) -> Self {
        Self {
            duration,
            frequency,
            ..Default::default()
        }
    }

    pub(crate) fn elapsed(&mut self) -> bool {
        if self.duration == 0 {
            return false; // consider 0 as unused
        }
        self.counter += 1;
        if self.frequency == 0 || self.counter % self.frequency == 0 {
            self.time_begin.elapsed().unwrap().as_secs() >= self.duration
        } else {
            false
        }
    }

    pub(crate) fn reset(&mut self) {
        self.time_begin = std::time::SystemTime::now();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_get_abspath() {
        let path_list = [
            ("/", "/"),
            ("/////", "/"),
            ("/..", "/"),
            ("/../", "/"),
            ("/root", "/root"),
            ("/root/", "/root"),
            ("/root/..", "/"),
            ("/root/../dev", "/dev"),
            ("/does/not/exist", "/does/not/exist"),
            ("/does/not/./exist", "/does/not/exist"),
            ("/does/not/../NOT/exist", "/does/NOT/exist"),
        ];
        for x in &path_list {
            match super::get_abspath(x.0) {
                Ok(v) => assert_eq!(v, x.1),
                Err(e) => panic!("{e} {x:?}"),
            }
        }
    }

    #[test]
    fn test_get_dirpath() {
        let path_list = [
            ("/root", "/"),
            ("/root/", "/"),
            ("/root/../dev", "/"),
            ("/does/not/exist", "/does/not"),
            ("/does/not/./exist", "/does/not"),
            ("/does/not/../NOT/exist", "/does/NOT"),
        ];
        for x in &path_list {
            match super::get_dirpath(x.0) {
                Ok(v) => assert_eq!(v, x.1),
                Err(e) => panic!("{e} {x:?}"),
            }
        }
    }

    #[test]
    fn test_get_basename() {
        let path_list = [
            ("/root", "root"),
            ("/root/", "root"),
            ("/root/../dev", "dev"),
            ("/does/not/exist", "exist"),
            ("/does/not/./exist", "exist"),
            ("/does/not/../NOT/exist", "exist"),
        ];
        for x in &path_list {
            match super::get_basename(x.0) {
                Ok(v) => assert_eq!(v, x.1),
                Err(e) => panic!("{e} {x:?}"),
            }
        }
    }

    #[test]
    fn test_is_abspath() {
        let path_list = [
            ("/", true),
            ("/////", true),
            ("/..", true),
            ("/../", true),
            ("/root", true),
            ("/root/", true),
            ("/root/..", true),
            ("/root/../dev", true),
            ("/does/not/exist", true),
            ("/does/not/../NOT/exist", true),
            ("xxx", false),
            ("does/not/exist", false),
        ];
        for x in &path_list {
            assert_eq!(super::is_abspath(x.0), x.1, "{x:?}");
        }
    }

    #[test]
    fn test_is_windows() {
        assert!(!super::is_windows());
    }

    #[test]
    fn test_get_path_separator() {
        assert_eq!(super::get_path_separator(), '/');
    }

    #[test]
    fn test_get_raw_file_type() {
        let dir_list = [".", "..", "/", "/dev"];
        for f in &dir_list {
            match super::get_raw_file_type(f) {
                Ok(v) => match v {
                    super::FileType::Dir => (),
                    x => panic!("{x:?}"),
                },
                Err(e) => panic!("{e}"),
            }
        }
        let invalid_list = ["", "516e7cb4-6ecf-11d6-8ff8-00022d09712b"];
        for f in &invalid_list {
            if let Ok(v) = super::get_raw_file_type(f) {
                panic!("{v:?}");
            }
        }
    }

    #[test]
    fn test_get_file_type() {
        let dir_list = [".", "..", "/", "/dev"];
        for f in &dir_list {
            match super::get_file_type(f) {
                Ok(v) => match v {
                    super::FileType::Dir => (),
                    x => panic!("{x:?}"),
                },
                Err(e) => panic!("{e}"),
            }
        }
        let invalid_list = ["", "516e7cb4-6ecf-11d6-8ff8-00022d09712b"];
        for f in &invalid_list {
            if let Ok(v) = super::get_file_type(f) {
                panic!("{v:?}");
            }
        }
    }

    #[test]
    fn test_get_file_type_is_xxx() {
        assert!(super::FileType::Dir.is_dir());
        assert!(super::FileType::Reg.is_reg());
        assert!(super::FileType::Device.is_device());
        assert!(super::FileType::Symlink.is_symlink());
        assert!(super::FileType::Unsupported.is_unsupported());
    }

    #[test]
    fn test_path_exists_or_error() {
        let dir_list = [".", "..", "/", "/dev"];
        for f in &dir_list {
            if let Err(e) = super::path_exists_or_error(f) {
                panic!("{e}");
            }
        }
        let invalid_list = ["", "516e7cb4-6ecf-11d6-8ff8-00022d09712b"];
        for f in &invalid_list {
            assert!(super::path_exists_or_error(f).is_err(), "{f}");
        }
    }

    #[test]
    fn test_path_exists() {
        let dir_list = [".", "..", "/", "/dev"];
        for f in &dir_list {
            assert!(super::path_exists(f), "{f}");
        }
        let invalid_list = ["", "516e7cb4-6ecf-11d6-8ff8-00022d09712b"];
        for f in &invalid_list {
            assert!(!super::path_exists(f), "{f}");
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
            assert!(super::is_dot_path(f), "{i} {f}");
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
            assert!(!super::is_dot_path(f), "{i} {f}");
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
                    assert!(v, "{i} {v}");
                }
                Err(e) => panic!("{i} {e}"),
            }
        }

        let unwritable_list = ["/proc"];
        for (i, f) in unwritable_list.iter().enumerate() {
            match super::is_dir_writable(f) {
                Ok(v) => {
                    assert!(!v, "{i} {v}");
                }
                Err(e) => panic!("{i} {e}"),
            }
        }

        let invalid_list = ["/proc/vmstat", "516e7cb4-6ecf-11d6-8ff8-00022d09712b"];
        for (i, f) in invalid_list.iter().enumerate() {
            if let Ok(v) = super::is_dir_writable(f) {
                assert!(!v, "{i} {v}");
            }
        }
    }

    #[test]
    fn test_remove_dup_string() {
        let uniq_ll = [
            vec![String::new()],
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
                String::new(),
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
        for l in &uniq_ll {
            let x = super::remove_dup_string(l.as_slice());
            for (i, a) in x.iter().enumerate() {
                for (j, b) in x.iter().enumerate() {
                    assert!(!(i != j && a == b), "{l:?}: {i} {a} vs {j} {b}");
                }
            }
            assert_eq!(l.len(), x.len(), "{:?}: {} != {}", l, l.len(), x.len());
            for i in 0..x.len() {
                assert_eq!(x[i], l[i].as_str(), "{:?}: {} {} != {}", l, i, x[i], l[i]);
            }
        }

        let dup_ll = [
            vec![String::new(), String::new()],
            vec![String::new(), String::new(), String::new()],
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
        for l in &dup_ll {
            let x = super::remove_dup_string(l.as_slice());
            for (i, a) in x.iter().enumerate() {
                for (j, b) in x.iter().enumerate() {
                    assert!(!(i != j && a == b), "{l:?}: {i} {a} vs {j} {b}");
                }
            }
            assert!(l.len() > x.len(), "{:?}: {} <= {}", l, l.len(), x.len());
            let mut v = vec![];
            for s in &x {
                v.push((*s).to_string());
            }
            let xx = super::remove_dup_string(&v);
            assert_eq!(x.len(), xx.len(), "{:?}: {} != {}", l, x.len(), xx.len());
            for i in 0..x.len() {
                assert_eq!(x[i], xx[i], "{:?}: {} {} != {}", l, i, x[i], xx[i]);
            }
        }
    }

    #[test]
    fn test_get_random() {
        for i in 1..10000 {
            let x = super::get_random(0..i);
            assert!(!(x < 0 || x >= i), "{i} {x}");
        }
        for i in 1..10000 {
            let x = super::get_random(-i..0);
            assert!(!(x < -i || x >= 0), "{i} {x}");
        }
    }

    #[test]
    fn test_timer1() {
        let mut timer = super::Timer::new(0, 0); // unused
        assert!(!timer.elapsed());
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(!timer.elapsed());
        assert!(!timer.elapsed());
        timer.reset();
        assert!(!timer.elapsed());

        let mut timer = super::Timer::new(1, 0);
        assert!(!timer.elapsed());
        std::thread::sleep(std::time::Duration::from_secs(1));
        assert!(timer.elapsed());
        assert!(timer.elapsed());
        timer.reset();
        assert!(!timer.elapsed());

        let mut timer = super::Timer::new(2, 0);
        assert!(!timer.elapsed());
        std::thread::sleep(std::time::Duration::from_secs(1));
        assert!(!timer.elapsed());
        assert!(!timer.elapsed());
        timer.reset();
        assert!(!timer.elapsed());
    }

    #[test]
    fn test_timer2() {
        let mut timer = super::Timer::new(0, 1000); // unused
        std::thread::sleep(std::time::Duration::from_secs(1));
        assert!(!timer.elapsed());
        std::thread::sleep(std::time::Duration::from_secs(1));
        assert!(!timer.elapsed());

        let mut timer = super::Timer::new(1, 1000);
        std::thread::sleep(std::time::Duration::from_secs(1));
        assert!(!timer.elapsed());
        std::thread::sleep(std::time::Duration::from_secs(1));
        assert!(!timer.elapsed());
    }
}
