use crate::errors::Result;
use crate::pattern::Pattern;
use std::fs;
use std::os::unix::fs::MetadataExt;

#[derive(Clone)]
pub struct File {
    name: String,
    ftype: bool, /* True for directory */
    uid: u32,
    gid: u32,
    perms: u32,
    pub path: String,
}
impl File {
    pub fn new(path: String, ftype: bool, uid: u32, gid: u32, perms: u32) -> Self {
        Self {
            name: path
                .split('/')
                .last()
                .and_then(|s| Some(s.to_string()))
                .unwrap(),
                ftype,
                uid,
                gid,
                path,
                perms,
        }
    }
}

pub struct FileMatcher {
    files: Vec<File>,
    npatterns: Vec<Pattern>,
    ppatterns: Vec<Pattern>,
    gid: Option<u32>,
    uid: Option<u32>,
    perms: Option<u32>,
    ftype: Option<char>,
}
impl FileMatcher {
    pub fn from_dir(dir: impl ToString, depth: bool) -> Result<Self> {
        Ok(Self {
            files: get_files(dir.to_string(), depth)?,
            npatterns: vec![],
            ppatterns: vec![],
            gid: None,
            uid: None,
            perms: None,
            ftype: None,
        })
    }
    pub fn set_ftype(&mut self, ftype: Option<char>) {
        self.ftype = ftype;
    }
    pub fn add_npatterns(&mut self, patterns: &[Pattern]) {
        self.npatterns.extend_from_slice(patterns);
    }
    pub fn add_ppatterns(&mut self, patterns: &[Pattern]) {
        self.ppatterns.extend_from_slice(patterns);
    }
    pub fn set_gid(&mut self, id: Option<u32>) {
        self.gid = id;
    }
    pub fn set_uid(&mut self, id: Option<u32>) {
        self.uid = id;
    }
    pub fn set_perms(&mut self, perms: Option<u32>) {
        self.perms = perms;
    }
    pub fn matches(&self) -> Vec<File> {
        self.files
            .clone()
            .into_iter()
            .map(|f| {
                let name = self
                    .npatterns
                    .iter()
                    .fold(true, |acc, p| acc && p.matches(f.name.clone()));
                let path = self
                    .ppatterns
                    .iter()
                    .fold(true, |acc, p| acc && p.matches(f.path.clone()));
                let gid = self.gid.map_or(true, |i| i == f.gid);
                let uid = self.uid.map_or(true, |i| i == f.uid);
                let mode = self.perms.map_or(true, |m| m == f.perms);
                let ftype = if let Some(c) = self.ftype {
                    if c == 'f' && f.ftype {
                        false
                    } else if c == 'd' && !f.ftype {
                        false
                    } else {
                        true
                    }
                } else {
                    true
                };
                if name && path && ftype && gid && uid && mode {
                    Some(f)
                } else {
                    None
                }
            })
        .filter(|f| f.is_some())
            .map(|f| f.unwrap())
            .collect()
    }
}
const MODE_MASK: u32 = 0b111111111111;
fn get_files(dir: String, depth: bool) -> Result<Vec<File>> {
    Ok(fs::read_dir(dir)?
       .map(|e| {
           let entry = e?;
           let path = entry.path();
           let md = fs::metadata(&path)?;
           let stringified = path.to_str().unwrap().to_string();
           let f = File::new(stringified.clone(), path.is_dir(), md.uid(), md.gid(), md.mode() & MODE_MASK);
           Ok(if path.is_dir() {
               let mut files = if !depth {
                   vec![f.clone()]
               } else {
                   vec![]
               };
               files.extend(get_files(stringified, depth)?);
               if depth {
                   files.push(f);
               }
               files
           } else {
               vec![f]
           })
       })
       .collect::<Result<Vec<Vec<File>>>>()?
       .into_iter()
       .flatten()
       .collect())
}
