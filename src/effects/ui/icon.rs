use {
  crate::{
    context::Context,
    dependency::Dependency,
    effects::ui,
    instance::Instance,
    instance_state::{
      FixableInstance, InstanceState, InvalidInstance, SemverGroupAndVersionConflict, SuspectInstance, UnfixableInstance, ValidInstance,
    },
    package_json::{FormatMismatch, FormatMismatchVariant, PackageJson},
    version_group::{VersionGroup, VersionGroupVariant},
  },
  colored::*,
  itertools::Itertools,
  log::{error, info, warn},
  std::{cell::RefCell, rc::Rc},
};

pub fn ok() -> ColoredString {
  "\u{2713}".green()
}

pub fn err() -> ColoredString {
  "\u{2718}".red()
}

pub fn warn() -> ColoredString {
  "!".yellow()
}

pub fn unknown() -> ColoredString {
  "?".dimmed()
}

pub fn dim_right_arrow() -> ColoredString {
  "\u{2192}".dimmed()
}
