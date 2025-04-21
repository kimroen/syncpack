use {
  crate::{
    context::Context,
    dependency::Dependency,
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

/// Indent level used across UI formatting
const DEFAULT_INDENT: usize = 4;

pub mod dependency;
pub mod group;
pub mod icon;
pub mod instance;
pub mod package;
pub mod util;
