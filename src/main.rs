mod pattern;
mod matcher;
mod errors;
use clap::{App, Arg};
use matcher::FileMatcher;
use pattern::Pattern;
use std::process::exit;
use errors::Result;

const NAME: &str = "trouve";

fn main() {
    match try_main() {
        Ok(()) => {},
        Err(e) => {
            println!("{}: {}", NAME, e);
            exit(1);
        }
    }
}

fn try_main() -> Result<()> {
    let matches = App::new(NAME)
                    .about("Find files")
                    .author(env!("CARGO_PKG_AUTHORS"))
                    .version(env!("CARGO_PKG_VERSION"))
                    .arg(Arg::with_name("dir")
                         .index(1)
                         .help("The directory to search in."))
                    .arg(Arg::with_name("name")
                         .short("n")
                         .long("name")
                         .multiple(true)
                         .takes_value(true)
                         .help("The patterns the files have to match."))
                    .arg(Arg::with_name("path")
                         .short("p")
                         .long("path")
                         .multiple(true)
                         .takes_value(true)
                         .help("The patterns the paths have to match."))

                    .get_matches();
    let dir = matches.value_of("dir").unwrap_or(".");
    let name = matches.values_of("name").and_then(|n| Some(n.collect::<Vec<&str>>())).unwrap_or(vec![]).into_iter().map(|n| Pattern::new(n)).collect::<Vec<Pattern>>();
    let path = matches.values_of("path").and_then(|n| Some(n.collect::<Vec<&str>>())).unwrap_or(vec![]).into_iter().map(|n| Pattern::new(n)).collect::<Vec<Pattern>>();
    let mut matcher = FileMatcher::from_dir(dir)?;
    matcher.add_npatterns(&name);
    matcher.add_ppatterns(&path);
    let matched = matcher.matches();
    matched.into_iter().for_each(|f| println!("{}", f.path));
    Ok(())
}
