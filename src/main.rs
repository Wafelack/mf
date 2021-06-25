mod errors;
mod matcher;
mod pattern;
use errors::Result;
use matcher::FileMatcher;
use pattern::Pattern;
use std::process::{exit, Command};
use structopt::StructOpt;

pub const NAME: &str = env!("CARGO_PKG_NAME");
#[derive(StructOpt)]
#[structopt(name = NAME, about = "Find files", after_help = "Patterns work with wildcards, a wildcard matches every set of characters.\nFor example, `*.rs` will match all the files which name ends in `.rs`." )]
struct Mf {
    #[structopt(
        default_value = ".",
        value_name = "DIR",
        help = "The directory to search in."
    )]
    dir: String,
    #[structopt(
        short,
        long,
        multiple = true,
        value_name = "PAT",
        help = "File name matches pattern(s) PAT."
    )]
    name: Vec<String>,
    #[structopt(
        short,
        long,
        value_name = "TYPE",
        help = "File is of type TYPE. 'f' stands for file and 'd' for directory."
    )]
    r#type: Option<char>,
    #[structopt(
        short,
        long,
        multiple = true,
        value_name = "PAT",
        help = "File path matches pattern(s) PAT."
    )]
    path: Vec<String>,
    #[structopt(
        short = "G",
        long,
        value_name = "ID",
        help = "File owner group has id ID."
    )]
    gid: Option<u32>,
    #[structopt(short = "U", long, value_name = "ID", help = "File owner has id ID.")]
    uid: Option<u32>,
    #[structopt(
        short,
        long,
        value_name = "COMMAND",
        help = "The command to run for each file. `{}` is replaced by the file name in the command."
    )]
    exec: Option<String>,
    #[structopt(
        short = "P",
        long,
        value_name = "BITS",
        help = "File has permissions bits set to BITS."
    )]
    perms: Option<String>,
    #[structopt(
        short,
        long,
        help = "Process the directory content before the directory itself."
    )]
    depth: bool,
    #[structopt(
        short,
        long,
        value_name = "DEPTH",
        help = "Reach at most DEPTH level of nested directories."
    )]
    maxdepth: Option<u32>,
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
    let args = Mf::from_args();
    let name = args
        .name
        .into_iter()
        .map(|n| Pattern::new(n))
        .collect::<Vec<Pattern>>();
    let ftype = args.r#type;
    let path = args
        .path
        .into_iter()
        .map(|n| Pattern::new(n))
        .collect::<Vec<Pattern>>();
    let perms = args
        .perms
        .map_or(Ok(None), |v| match u32::from_str_radix(v.as_str(), 8) {
            Ok(v) => Ok(Some(v)),
            Err(_) => error!("Invalid permission bits: `{}'", v),
        })?;
    let mut matcher = FileMatcher::from_dir(args.dir, args.depth, args.maxdepth)?;
    matcher.set_ftype(ftype.map_or(Ok(None), |t| {
        if t == 'f' || t == 'd' {
            Ok(Some(t))
        } else {
            error!("Invalid file type: {}.", t)
        }
    })?);
    matcher.add_npatterns(&name);
    matcher.add_ppatterns(&path);
    matcher.set_uid(args.uid);
    matcher.set_gid(args.gid);
    matcher.set_perms(perms);
    let matched = matcher.matches();
    match args.exec {
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
