use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};
use std::str;

use csv;
use docopt::Docopt;
use num_cpus;
use rustc_serialize::Decodable;

use CliResult;
use config::{Config, Delimiter};

pub fn num_cpus() -> usize {
    num_cpus::get()
}

pub fn version() -> String {
    let (maj, min, pat) = (
        option_env!("CARGO_PKG_VERSION_MAJOR"),
        option_env!("CARGO_PKG_VERSION_MINOR"),
        option_env!("CARGO_PKG_VERSION_PATCH"),
    );
    match (maj, min, pat) {
        (Some(maj), Some(min), Some(pat)) =>
            format!("{}.{}.{}", maj, min, pat),
        _ => "".to_owned(),
    }
}

pub fn get_args<T>(usage: &str, argv: &[&str]) -> CliResult<T>
        where T: Decodable {
    Docopt::new(usage)
           .and_then(|d| d.argv(argv.iter().map(|&x| x))
                          .version(Some(version()))
                          .decode())
           .map_err(From::from)
}

pub fn many_configs(inps: &[String], delim: Option<Delimiter>,
                    no_headers: bool) -> Result<Vec<Config>, String> {
    let mut inps = inps.to_vec();
    if inps.is_empty() {
        inps.push("-".to_owned()); // stdin
    }
    let confs = inps.into_iter()
                    .map(|p| Config::new(&Some(p))
                                    .delimiter(delim)
                                    .no_headers(no_headers))
                    .collect::<Vec<_>>();
    try!(errif_greater_one_stdin(&*confs));
    Ok(confs)
}

pub fn errif_greater_one_stdin(inps: &[Config]) -> Result<(), String> {
    let nstd = inps.iter().filter(|inp| inp.is_std()).count();
    if nstd > 1 {
        return Err("At most one <stdin> input is allowed.".to_owned());
    }
    Ok(())
}

pub fn empty_field() -> csv::ByteString { vec![] }

pub fn chunk_size(nitems: usize, njobs: usize) -> usize {
    if nitems < njobs {
        nitems
    } else {
        nitems / njobs
    }
}

pub fn num_of_chunks(nitems: usize, chunk_size: usize) -> usize {
    if chunk_size == 0 {
        return nitems;
    }
    let mut n = nitems / chunk_size;
    if nitems % chunk_size != 0 {
        n += 1;
    }
    n
}

pub fn last_modified(md: &fs::Metadata) -> u64 {
    use filetime::FileTime;
    FileTime::from_last_modification_time(md).seconds_relative_to_1970()
}

pub fn condense<'a>(val: Cow<'a, [u8]>, n: Option<usize>) -> Cow<'a, [u8]> {
    match n {
        None => val,
        Some(n) => {
            // It would be much nicer to just use a `match` here, but the
            // borrow checker won't allow it. ---AG
            //
            // (We could circumvent it by allocating a new Unicode string,
            // but that seems excessive.)
            let mut is_short_utf8 = false;
            if let Ok(s) = str::from_utf8(&*val) {
                if n >= s.chars().count() {
                    is_short_utf8 = true;
                } else {
                    let mut s = s.chars().take(n).collect::<String>();
                    s.push_str("...");
                    return Cow::Owned(s.into_bytes());
                }
            }
            if is_short_utf8 || n >= (*val).len() { // already short enough
                val
            } else {
                // This is a non-Unicode string, so we just trim on bytes.
                let mut s = val[0..n].to_vec();
                s.extend(b"...".iter().cloned());
                Cow::Owned(s)
            }
        }
    }
}

pub fn idx_path(csv_path: &Path) -> PathBuf {
    let mut p = csv_path.to_path_buf().into_os_string().into_string().unwrap();
    p.push_str(".idx");
    PathBuf::from(&p)
}

pub type Idx = Option<usize>;

pub fn range(start: Idx, end: Idx, len: Idx, index: Idx)
            -> Result<(usize, usize), String> {
    match (start, end, len, index) {
        (None, None, None, Some(i)) => Ok((i, i+1)),
        (_, _, _, Some(_)) =>
            Err("--index cannot be used with --start, --end or --len".to_owned()),
        (_, Some(_), Some(_), None) =>
            Err("--end and --len cannot be used at the same time.".to_owned()),
        (_, None, None, None) => Ok((start.unwrap_or(0), ::std::usize::MAX)),
        (_, Some(e), None, None) => {
            let s = start.unwrap_or(0);
            if s > e {
                Err(format!("The end of the range ({}) must be greater than or\n\
                             equal to the start of the range ({}).", e, s))
            } else {
                Ok((s, e))
            }
        }
        (_, None, Some(l), None) => {
            let s = start.unwrap_or(0);
            Ok((s, s + l))
        }
    }
}
