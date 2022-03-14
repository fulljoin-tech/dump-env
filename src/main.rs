//! # dump-env
//!
//! Environment variable utility tool that prints environment variables.
//! When provided with a template `dump-env` merges the template and existing environment variables.
//!
//! ## Why
//!
//! This tool is helpful in CI pipelines where you can store environment vars as part of the pipeline
//! and need a proper way to generate .env files.

use std::env;
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use clap::Parser;
use thiserror::Error;
use eyre::Result;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Source template file
    #[clap(short, long)]
    source: Option<String>,

    /// Template file
    #[clap(short, long)]
    template: Option<String>,

    /// Prefixes
    #[clap(short, long)]
    prefixes: Vec<String>
}

#[derive(Debug, Error)]
enum Error {
    #[error("Template not found")]
    TemplateNotFound
}

type EnvItem = (OsString, OsString);
type EnvItems = Vec<EnvItem>;

fn main() -> Result<()> {
    let args = Args::parse();

    if let Some(source_path) = args.source {
        let path = PathBuf::from(&source_path);
        print(left_join(parse_template(&path)?, get_env(&args.prefixes)));
        return Ok(());
    }

    if let Some(template_path) = args.template {
        let path = PathBuf::from(&template_path);
        print(full_join(parse_template(&path)?, get_env(&args.prefixes)));
        return Ok(());

    }

    print( get_env(&args.prefixes));
    Ok(())
}

fn strip_prefixes(prefixes: &[String], items: EnvItems) -> EnvItems {
    items.into_iter().map(|(k,v)| {
        let key_string = k.to_string_lossy().to_string();
        for pfx in prefixes {
            // Return after the first prefix hit.
            if let Some(x) = key_string.strip_prefix(pfx.as_str()) {
                return (x.into(), v);
            }
        }
        (k, v)
    }).collect()
}

/// Prints a list of EnvItem to stdout.
fn print(x: EnvItems) {
    for (k, v) in x {
        println!("{}={}", k.to_string_lossy(), v.to_string_lossy());
    }
}

/// Get environment vars as list of OsString tuples.
fn get_env(prefixes: &[String]) -> EnvItems {
    strip_prefixes(prefixes, env::vars_os().into_iter().map(|(k,v)| (k, v)).collect())
}


/// `left` is the template, `right` are the environment vars.
/// Include all that is in `left` and overwrite with `right`.
fn left_join(left: EnvItems, right: EnvItems) -> EnvItems {
    left.into_iter().map(|(lk, lv)| {
        for (rk, rv) in &right {
            if &lk == rk {
                return (lk, rv.clone());
            }
        }

        (lk, lv)
    }).collect()
}

/// `left` is the template, `right` are the environment vars.
/// Include all that is in `left` , overwrite with `right` extends results with
/// missing keys from `right`.
fn full_join(left: EnvItems, right: EnvItems) -> EnvItems {
    let mut x = left_join(left, right.clone());
    for (rk, rv) in &right {
        if !has_key(rk, &x) {
           x.push((rk.clone(), rv.clone()))
        }
    }
    x.sort();
    x
}

/// Has key helper.
fn has_key(key: &OsString, xs: &[EnvItem]) -> bool {
    for (k, _v) in xs {
        if key == k {
            return true;
        }
    }
    false
}

/// Parse a .env template file
/// This trims whitespace and skips lines that start with #.
fn parse_template(path: &Path) -> Result<EnvItems> {
    if !path.exists() {
        return Err(Error::TemplateNotFound.into());
    }
    let file = File::open(&path)?;

    Ok(BufReader::new(file)
        .lines()
        .filter_map(|x| x.ok())
        .filter(|x| !x.starts_with('#'))
        .filter_map(|line| {
            if let Some((left, right)) = line.split_once('=') {
                Some((OsString::from(left.trim()), OsString::from(right.trim())))
            } else {
                None
            }
        }).collect())
}


#[cfg(test)]
mod tests {

    use super::*;

    fn to_os_str(xs: Vec<(&str, &str)>) -> Vec<(OsString, OsString)> {
        xs.into_iter().map(|(k, v)|{
            (OsString::from(k), OsString::from(v))
        }).collect()
    }

    #[test]
    fn test_left_join() {
        {
            let source = to_os_str(vec![("a", "1"), ("b", "2")]);
            let env = to_os_str(vec![("a", "10"), ("b", "20")]);
            let expect = env.clone();

            let result = left_join(source, env);
            assert_eq!(result, expect);
        }

        {
            let source = to_os_str(vec![("a", "1"), ("b", "2")]);
            let env = to_os_str(vec![("b", "20")]);
            let expect = to_os_str(vec![("a", "1"), ("b", "20")]);

            let result = left_join(source, env);
            assert_eq!(result, expect);
        }

        {
            let source = to_os_str(vec![("a", "1"), ("b", "2")]);
            let env = to_os_str(vec![("a", "10")]);
            let expect = to_os_str(vec![("a", "10"), ("b", "2")]);

            let result = left_join(source, env);
            assert_eq!(result, expect);
        }

        {
            let source = to_os_str(vec![("a", "1"), ("b", "2"), ("c", "3")]);
            let env = to_os_str(vec![("a", "10")]);
            let expect = to_os_str(vec![("a", "10"), ("b", "2"), ("c", "3")]);

            let result = left_join(source, env);
            assert_eq!(result, expect);
        }

        {
            let source = to_os_str(vec![("a", "1"), ("b", "2"), ("c", "3")]);
            let env = to_os_str(vec![("a", "10"), ("b", "20"), ("c", "5"), ("d", "4")]);
            let expect = to_os_str(vec![("a", "10"), ("b", "20"), ("c", "5")]);

            let result = left_join(source, env);
            assert_eq!(result, expect);
        }
    }

    #[test]
    fn test_full_join() {
        {
            let source = to_os_str(vec![("a", "1"), ("b", "2")]);
            let env = to_os_str(vec![("a", "10"), ("b", "20")]);
            let expect = env.clone();

            let result = full_join(source, env);
            assert_eq!(result, expect);
        }
        {
            let source = to_os_str(vec![("a", "1"), ("b", "2")]);
            let env = to_os_str(vec![("a", "10"), ("b", "20"), ("c", "30")]);
            let expect = env.clone();

            let result = full_join(source, env);
            assert_eq!(result, expect);
        }

    }

    #[test]
    fn test_strip_prefixes() {
        let prefixes = vec![String::from("test_"), String::from("test2_")];
        let env = to_os_str(vec![("test_a", "10"), ("test2_b", "20"), ("test_test2_c", "30")]);
        let expect = to_os_str(vec![("a", "10"), ("b", "20"), ("test2_c", "30")]);

        let result = strip_prefixes(&prefixes, env);
        assert_eq!(result, expect);
    }
}
