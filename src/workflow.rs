use crate::wildcard::{Wildcard, WildcardMap};

pub struct Workflow {
  rules: Vec<Box<dyn Rule>>,
}

pub trait Rule {
  fn materialize(&self, path: &str) -> Option<Task>;
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub enum Error {
  Wildcard(crate::wildcard::WildcardError),
}

impl From<crate::wildcard::WildcardError> for Error {
  fn from(value: crate::wildcard::WildcardError) -> Self {
    Self::Wildcard(value)
  }
}

pub struct Task<'a> {
  func: Box<dyn FnOnce() -> Result<()> + 'a>,
}

pub struct WildcardRule<F>
where
  F: Fn(&[String], &[String], &WildcardMap) -> Result<()>,
{
  inputs: Vec<String>,
  outputs: Vec<Wildcard>,
  func: F,
}

impl<F> WildcardRule<F>
where
  F: Fn(&[String], &[String], &WildcardMap) -> Result<()>,
{
  pub fn new(inputs: &[String], outputs: &[String], func: F) -> Result<Self> {
    Ok(Self {
      inputs: inputs.to_vec(),
      outputs: outputs
        .iter()
        .map(|s| Wildcard::new(s))
        .collect::<crate::wildcard::Result<Vec<_>>>()?,
      func,
    })
  }
}

impl<F> Rule for WildcardRule<F>
where
  F: Fn(&[String], &[String], &WildcardMap) -> Result<()>,
{
  fn materialize(&self, path: &str) -> Option<Task> {
    for output in self.outputs.iter() {
      if let Some(map) = output.extract(path) {
        let inputs = self
          .inputs
          .iter()
          .map(|i| map.apply(i))
          .collect::<crate::wildcard::Result<Vec<_>>>()
          .ok()?;
        let outputs = self
          .outputs
          .iter()
          .map(|o| map.apply(&o.pattern))
          .collect::<crate::wildcard::Result<Vec<_>>>()
          .ok()?;
        return Some(Task {
          func: Box::new(move || {
            (self.func)(&inputs, &outputs, &map)?;
            println!("hi");
            Ok(())
          }),
        });
      }
    }
    None
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn workflow() {
    let rule = WildcardRule::new(
      &["path/to/{file}/{blah}.in".to_string()],
      &["path/to/{file}.out".to_string()],
      |i, o, m| {
        println!("inputs: {i:?}");
        println!("outputs: {o:?}");
        println!("wildcards: {m:?}");
        Ok(())
      },
    ).unwrap();
    // let workflow = Workflow {
    //   rules: vec![Box::new(rule)],
    // };
    let task = rule.materialize("path/to/filename.out").unwrap();
    (task.func)().unwrap();
    assert!(false);
  }
}
