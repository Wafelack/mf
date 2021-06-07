#[derive(Debug, PartialEq, Clone)]
pub enum PT {
    Start(String),
    Contains(String),
    End(String),
}
#[derive(Clone)]
pub struct Pattern {
    rules: Vec<PT>,
}
impl Pattern {
    pub fn new(text: impl ToString) -> Self {
        let text = text.to_string();
        let (start, end) = (text.starts_with('*'), text.ends_with('*'));
        let count = text.matches('*').count();
        let rules = text
                    .split('*')
                    .enumerate()
                    .map(|(idx, s)| {
                        let s = s.to_string();
                        if !start && idx == 0 {
                            PT::Start(s)
                        } else if !end && idx == count {
                            PT::End(s)
                        }  else {
                            PT::Contains(s)
                        }
                    })
                    .filter(|p| if let PT::Contains(s) = p { s.as_str() != "" } else { true })
                    .collect();

        Pattern {
            rules
        }
    }
    pub fn matches(&self, s: impl ToString) -> bool {
        let s = s.to_string();
        self.rules
            .iter()
            .fold(true, |acc, p| acc && if let PT::Start(start) = p {
                s.starts_with(start)
            } else if let PT::End(end) = p {
                s.ends_with(end)
            } else if let PT::Contains(sub) = p{
                s.contains(sub)
            } else {
                panic!("Impossible pattern type.");
            })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    
    #[test]
    fn pat_new() {
        assert_eq!(Pattern::new("*.rs").rules, vec![PT::End(".rs".to_string())]);
        assert_eq!(Pattern::new("src*m*.rs").rules, vec![PT::Start("src".to_string()), PT::Contains("m".to_string()), PT::End(".rs".to_string())]);
    }
    #[test]
    fn pat_matches() {
        let pat = Pattern::new("*src/*.rs");
        assert!(!pat.matches("./Cargo.toml"));
        assert!(pat.matches("./src/main.rs"));
    }
}