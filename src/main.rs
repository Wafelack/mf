mod errors;
mod matcher;
mod pattern;
use errors::Result;
use getopt_rs::getopt;
use matcher::FileMatcher;
use pattern::Pattern;
use std::{
    env,
    process::{exit, Command},
};

pub const NAME: &str = env!("CARGO_PKG_NAME");

fn main() {
    match try_main() {
        Ok(()) => {}
        Err(e) => {
            println!("{}: {}", NAME, e);
            exit(1);
        }
    }
}

fn help() {
    println!("Usage: {} [OPTION]... [FOLDER]
Find files in a directory hierarchy.

If no FOLDER is specified, . (the current directory) is used.
By default, mf goes into every directory beyond the FOLDER, howver, this could be changed with the --maxdepth option.

-d, --depth             Process the directory content before the directory itself.
-x, --exec COMMAND      The command to run for each file. `{{}}` is replaced by the file's name in the command.
-g, --gid ID            File owner belongs to the group that has id ID.
-n, --name PAT          File name matches pattern PAT.
-p, --path PAT          File path matches pattern PAT.
-P, --perms PAT         File has permission bits set to BITS.
-t, --type TYPE         File is of type TYPE. Type `f` stands for file and `d` for directory.
-u, --uid ID            File owner has user id ID.
-m, --maxdepth DEPTH    Reach at most DEPTH nested directories.
-h, --help              Display this help and exit.
-V, --version           Display version information and exit.

Patterns work with wildcards, a wildcard matches every set of characters.
For example, `*.rs` will match all the files which name ends in `.rs`, and 
`*foo*` will match all the files containing `foo` in their names.", NAME);
}

fn try_main() -> Result<()> {
    let mut args = env::args().collect();
    let (mut name, mut ftype, mut path, mut perms, mut uid, mut gid, mut command, mut maxdepth) =
        (None, None, None, None, None, None, None, None);
    let mut depth = false;
    while let Some(opt) = getopt(
        &mut args,
        "dx:g:n:p:P:t:u:m:hV",
        &[
            ('d', "depth"),
            ('x', "exec"),
            ('g', "gid"),
            ('n', "name"),
            ('p', "path"),
            ('P', "perms"),
            ('t', "type"),
            ('u', "uid"),
            ('m', "maxdepth"),
            ('h', "help"),
            ('V', "version"),
        ],
    ) {
        match opt {
            ('h', _) => {
                help();
                return Ok(());
            }
            ('v', _) => {
                println!("{} {}", NAME, env!("CARGO_PKG_NAME"));
                return Ok(());
            }
            ('d', _) => depth = true,
            ('x', val) => command = val,
            ('g', val) => {
                    let val = val.unwrap().clone();
                gid = Some(
                    val
                        .as_str()
                        .parse::<u32>()
                        .map_err(|_| errors::Error(format!("Invalid ID: `{}`.", val)))?,
                )
            }
            ('n', val) => name = Some(Pattern::new(val.unwrap())),
            ('p', val) => path = Some(Pattern::new(val.unwrap())),
            ('P', val) => {
                let val = val.unwrap().clone();
                perms = Some(u32::from_str_radix(&val, 8).map_err(|_| {
                    errors::Error(format!("Invalid permission bits: `{}`", val))
                })?)
            }
            ('t', val) => {
                ftype = Some(match val.unwrap() {
                    t if t == "f" || t == "d" => Ok(t.chars().nth(0).unwrap()),
                    x => error!("Invalid file type: `{}`.", x),
                }?)
            }
            ('u', val) => {
                let val = val.unwrap().clone();
                uid = Some(
                    val.parse::<u32>()
                        .map_err(|_| errors::Error(format!("Invalid ID: `{}`.", val)))?,
                )
            }
            ('m', val) => {
                let val = val.unwrap().clone();
                maxdepth =
                    Some(val.parse::<u32>().map_err(|_| {
                        errors::Error(format!("Invalid depth: `{}`.", val))
                    })?)
            }
            _ => exit(1),
        }
    }

    let mut matcher = FileMatcher::from_dir(
        args.into_iter().nth(1).unwrap_or(".".to_string()),
        depth,
        maxdepth,
    )?;
    matcher.set_ftype(ftype.map_or(Ok(None), |t| {
        if t == 'f' || t == 'd' {
            Ok(Some(t))
        } else {
            error!("Invalid file type: {}.", t)
        }
    })?);
    matcher.set_npattern(name);
    matcher.set_ppattern(path);
    matcher.set_uid(uid);
    matcher.set_gid(gid);
    matcher.set_perms(perms);
    let matched = matcher.matches();
    match command {
        Some(v) => {
            let (cmd, rargs) = v.split_once(' ').unwrap_or((v.as_str(), ""));
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
