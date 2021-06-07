use crate::pattern::Pattern;
use crate::{error, errors::Result};
use std::os::unix::fs::MetadataExt;
use std::fs;

#[derive(Clone)]
pub struct File {
    name: String,
    ftype: bool, /* True for directory */
    uid: u32,
    gid: u32,
    pub path: String,
}
impl File {
    pub fn new(path: String, ftype: bool, uid: u32, gid: u32) -> Self {
        Self {
            name: path.split('/').last().and_then(|s| Some(s.to_string())).unwrap(),
            ftype,
            uid,
            gid,
            path
        }
    }
}

pub struct FileMatcher {
    files: Vec<File>,
    npatterns: Vec<Pattern>,
    ppatterns: Vec<Pattern>,
}
impl FileMatcher {
    pub fn from_dir(dir: impl ToString) -> Result<Self> {
        Ok(Self {
            files: get_files(dir.to_string())?,
            npatterns: vec![],
            ppatterns: vec![],
        })
    }
    pub fn add_npatterns(&mut self, patterns: &[Pattern]) {
        self.npatterns.extend_from_slice(patterns);
    }
    pub fn add_ppatterns(&mut self, patterns: &[Pattern]) {
        self.ppatterns.extend_from_slice(patterns);
    }
    pub fn matches(&self) -> Vec<File> {
        self.files
            .clone()
            .into_iter()
            .map(|f| {
                let name = self.npatterns.iter().fold(true, |acc, p| acc && p.matches(f.name.clone()));
                let path = self.ppatterns.iter().fold(true, |acc, p| acc && p.matches(f.path.clone()));
                if name && path {
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
fn get_files(dir: String) -> Result<Vec<File>> {
    Ok(fs::read_dir(dir)?
       .map(|e| {
           let entry = e?;
            let path = entry.path();
            let md = fs::metadata(&path)?;
            let stringified = path.to_str().unwrap().to_string();
            let f = File::new(stringified.clone(), path.is_dir(), md.uid(), md.gid());
            Ok(if path.is_dir() {
                let mut files = vec![f];
                files.extend(get_files(stringified)?);
                files
            } else {
                vec![f]
            })
       })
       .collect::<Result<Vec<Vec<File>>>>()?.into_iter().flatten().collect())
}
