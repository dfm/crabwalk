use std::collections::HashMap;

pub(crate) type Result<T> = std::result::Result<T, WildcardError>;

#[derive(Debug, Clone)]
pub enum WildcardError {
  InvalidConstraint(String),
  MissingName(String),
  RegexError(regex::Error),
}

impl From<regex::Error> for WildcardError {
  fn from(value: regex::Error) -> Self {
    Self::RegexError(value)
  }
}

impl std::fmt::Display for WildcardError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      Self::InvalidConstraint(s) => write!(
        f,
        "Wildcard constraints must only be provided for the first appearence of the field '{s}'"
      ),
      Self::MissingName(s) => write!(f, "The field '{s}' is not constrained by the match"),
      Self::RegexError(e) => write!(f, "{e}"),
    }
  }
}

#[derive(Debug, Clone)]
pub struct Wildcard {
  pub(crate) pattern: String,
  re: regex::Regex,
  names: HashMap<String, Vec<usize>>,
}

impl Wildcard {
  pub fn new(pattern: &str) -> Result<Self> {
    let wildcard_regex = wildcard_regex()?;

    let mut re = "^".to_string();
    let mut last = 0;
    let mut constraints: HashMap<&str, &str> = HashMap::new();
    let mut names: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, cap) in wildcard_regex.captures_iter(pattern).enumerate() {
      let full = cap.get(0).unwrap();
      re.push_str(&regex::escape(&pattern[last..full.start()]));
      let name = cap.name("name").unwrap().as_str();

      if let Some(&constraint) = constraints.get(name) {
        // If we've already seen this named part, check that it doesn't include a new constraint
        if cap.name("constraint").is_some() {
          return Err(WildcardError::InvalidConstraint(name.to_string()));
        }
        re.push_str(&format!("({constraint})"));
        names.get_mut(name).unwrap().push(idx);
      } else {
        let constraint = cap.name("constraint").map_or(".+", |c| c.as_str());
        constraints.insert(name, constraint);
        names.insert(name.to_string(), Vec::new());
        re.push_str(&format!("(?P<{name}>{})", constraint));
      }
      last = full.end();
    }

    re.push_str(&regex::escape(&pattern[last..]));
    re.push('$');

    Ok(Self {
      pattern: pattern.to_string(),
      re: regex::Regex::new(&re)?,
      names,
    })
  }

  pub fn extract(&self, input: &str) -> Option<WildcardMap> {
    let cap = self.re.captures(input)?;
    let mut map = HashMap::new();
    for (name, dupes) in self.names.iter() {
      let value = cap.name(name)?.as_str();
      if dupes
        .iter()
        .any(|&d| cap.get(d + 1).map_or("", |c| c.as_str()) != value)
      {
        return None;
      }
      map.insert(name.to_string(), value.to_string());
    }
    WildcardMap::new(map).ok()
  }
}

#[derive(Debug, Clone)]
pub struct WildcardMap {
  re: regex::Regex,
  map: HashMap<String, String>,
}

impl WildcardMap {
  fn new(map: HashMap<String, String>) -> Result<Self> {
    let re = wildcard_regex()?;
    Ok(Self { re, map })
  }

  pub fn apply(&self, input: &str) -> Result<String> {
    let mut last = 0;
    let mut result = String::new();
    for cap in self.re.captures_iter(input) {
      let full = cap.get(0).unwrap();
      result.push_str(&input[last..full.start()]);
      let name = cap.name("name").unwrap().as_str();
      let value = self
        .map
        .get(name)
        .ok_or_else(|| WildcardError::MissingName(name.to_string()))?;
      result.push_str(value);
      last = full.end();
    }
    result.push_str(&input[last..]);
    Ok(result)
  }
}

fn wildcard_regex() -> std::result::Result<regex::Regex, regex::Error> {
  regex::Regex::new(
    r"(?x)
\{
  \s*(?P<name>\w+)\s*
  (\s*,\s*
    (?<constraint>([^{}]+ | \{\d+(,\d+)?\})*)
  \s*)?\s*
\}
",
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  macro_rules! test_wildcard_patterns {
    ($($name:ident: $pattern:expr, $path:expr,)*) => {
      $(
        #[test]
        fn $name() {
          let wc = Wildcard::new($pattern).unwrap();
          let map = wc.extract($path).unwrap();
          assert_eq!(map.apply($pattern).unwrap(), $path);
        }
      )*
    };
  }

  test_wildcard_patterns!(
    same_name: "path/to/{name}/{name}_{name}.txt", "path/to/output/output_output.txt",
    digits: "path/to/{name,\\d+}.txt", "path/to/0123.txt",
  );
}
