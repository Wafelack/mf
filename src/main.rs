mod errors;
mod matcher;
mod pattern;
use clap::{App, Arg};
use errors::Result;
use libc::{c_char, getgrnam, getpwnam};
use matcher::FileMatcher;
use pattern::Pattern;
use std::process::{exit, Command};

const NAME: &str = env!("CARGO_PKG_NAME");
macro_rules! gen_app {
    () => {
        App::new(NAME)
            .about("Find files")
            .author(env!("CARGO_PKG_AUTHORS"))
            .version(env!("CARGO_PKG_VERSION"))
            .arg(
                Arg::with_name("dir")
                .index(1)
                .help("The directory to search in."),
                )
            .arg(
                Arg::with_name("name")
                .short("n")
                .long("name")
                .multiple(true)
                .value_name("pat")
                .takes_value(true)
                .help("File name matches pattern(s) `pat'."),
                )
            .arg(
                Arg::with_name("type")
                .short("t")
                .long("type")
                .value_name("t")
                .help("File is of type `t'. `f' stands for file and `d' for directory."),
                )
            .arg(
                Arg::with_name("path")
                .short("p")
                .long("path")
                .multiple(true)
                .takes_value(true)
                .value_name("pat")
                .help("File path matches pattern(s) `pat'."),
                )
            .arg(
                Arg::with_name("gid")
                .short("G")
                .long("gid")
                .value_name("id")
                .help("File owner belongs to the group that has GID `id'."),
                )
            .arg(
                Arg::with_name("uid")
                .short("U")
                .long("uid")
                .value_name("id")
                .help("File owner has UID `id'."),
                )
            .arg(
                Arg::with_name("user")
                .short("u")
                .long("user")
                .value_name("name")
                .help("File owner has username `name'."),
                )
            .arg(
                Arg::with_name("group")
                .short("g")
                .long("group")
                .value_name("name")
                .help("File owner belongs to the group named `name'."),
                )
            .arg(Arg::with_name("execute")
                 .short("x")
                 .long("exec")
                 .value_name("command")
                 .help("The command to run for each file. `{}' is replaced by the file name in the command."))
            .arg(Arg::with_name("permissions")
                 .short("P")
                 .long("perms")
                 .value_name("bits")
                 .help("File has premissions bits set to `bits'."))
            .after_help(
                "Patterns work with wildcards, a wildcard matches every set of characters.
For example, `*.rs` will match all the files ending in .rs.",
)

    }
}

fn main() {
    match try_main() {
        Ok(()) => {}
        Err(e) => {
            println!("{}: {}", NAME, e);
            exit(1);
        }
    }
}

fn try_main() -> Result<()> {
    let matches = gen_app!().get_matches();
    let dir = matches.value_of("dir").unwrap_or(".");
    let name = matches
        .values_of("name")
        .and_then(|n| Some(n.collect::<Vec<&str>>()))
        .unwrap_or(vec![])
        .into_iter()
        .map(|n| Pattern::new(n))
        .collect::<Vec<Pattern>>();
    let ftype = matches.value_of("type");
    let path = matches
        .values_of("path")
        .and_then(|n| Some(n.collect::<Vec<&str>>()))
        .unwrap_or(vec![])
        .into_iter()
        .map(|n| Pattern::new(n))
        .collect::<Vec<Pattern>>();
    let gid = matches
        .value_of("gid")
        .and_then(|v| v.parse::<u32>().map_or(None, |v| Some(v)));
    let uid = matches
        .value_of("uid")
        .and_then(|v| v.parse::<u32>().map_or(None, |v| Some(v)));
    let uid = match uid {
        Some(u) => Ok(Some(u)),
        None => match matches.value_of("user") {
            Some(n) => {
                let passwd = unsafe { getpwnam(to_cchar(n)) };
                if passwd.is_null() {
                    error!("Failed to query passwd for `{}'.", n)
                } else {
                    Ok(Some(unsafe { (*passwd).pw_uid }))
                }
            }
            None => Ok(None),
        },
    }?;
    let gid = match gid {
        Some(u) => Ok(Some(u)),
        None => match matches.value_of("group") {
            Some(n) => {
                let grp = unsafe { getgrnam(to_cchar(n)) };
                if grp.is_null() {
                    error!("Failed to query group for `{}'.", n)
                } else {
                    Ok(Some(unsafe { (*grp).gr_gid }))
                }
            }
            None => Ok(None),
        },
    }?;
    let perms = matches.value_of("perms").map_or(Ok(None), |v| match u32::from_str_radix(v, 8) {
        Ok(v) => Ok(Some(v)),
        Err(_) => error!("Invalid permission bits: `{}'", v),
    })?;
    let mut matcher = FileMatcher::from_dir(dir)?;
    matcher.set_ftype(ftype.map_or(Ok(None), |t| {
        if t == "f" {
            Ok(Some('f'))
        } else if t == "d" {
            Ok(Some('d'))
        } else {
            error!("Invalid file type: {}.", t)
        }
    })?);
    matcher.add_npatterns(&name);
    matcher.add_ppatterns(&path);
    matcher.set_uid(uid);
    matcher.set_gid(gid);
    matcher.set_perms(perms);
    let matched = matcher.matches();
    match matches.value_of("execute") {
        Some(v) => {
            let (cmd, rargs) = v.split_once(' ').unwrap_or((v, ""));
            let args = to_args(rargs);

            let codes = matched
                .into_iter()
                .map(|f| {
                    let replaced_args = args.iter().map(|(t, s)| {
                        let s = s.replace("{}", &f.path);
                        if *t == 0 {
                            format!("'{}'", s)
                        } else if *t == 1 {
                            format!("\"{}\"", s)
                        } else {
                            format!("{}", s)
                        }
                    });
                    let status = match Command::new(cmd).args(replaced_args.clone()).status() {
                        Ok(s) => Ok(s),
                        Err(_) => error!(
                            "Failed to summon command: `{}`",
                            replaced_args.fold(cmd.to_string(), |acc, s| format!("{} {}", acc, s))
                            ),
                    }?;

                    Ok(status.code().unwrap_or(-1))
                })
            .collect::<Result<Vec<i32>>>()?;
            for (idx, code) in codes.into_iter().enumerate() {
                if code != 0 {
                    return error!("Command #{} failed, exit code: {}.", idx, code);
                }
            }
            Ok(())
        }
        None => {
            matched.into_iter().for_each(|f| println!("{}", f.path));
            Ok(())
        }
    }
}

fn to_args<'a>(s: &'a str) -> Vec<(u8, &'a str)> {
    s.split('\'')
        .enumerate()
        .map(|(idx, s)| {
            if idx % 2 == 0 {
                s.split('"')
                    .enumerate()
                    .map(|(idx, s)| {
                        if idx % 2 == 0 {
                            s.split(' ').map(|s| (2, s)).collect()
                        } else {
                            vec![(1, s)]
                        }
                    })
                .flatten()
                    .filter(|(_, s)| !s.is_empty())
                    .collect()
            } else {
                vec![(0, s)]
            }
        })
    .flatten()
        .filter(|(_, s)| !s.is_empty())
        .collect()
}

fn to_cchar(s: &str) -> *const c_char {
    let mut bytes = s.chars().map(|c| {
        println!("{};{}", c, c as i8);
        c as i8
    }).collect::<Vec<i8>>();
    bytes.push(0);
    bytes.as_ptr()
}
