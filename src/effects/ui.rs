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

/// Join lines that are not empty with a space separator
pub fn join_line(lines: Vec<&String>) -> String {
  lines.into_iter().filter(|line| !line.is_empty()).join(" ")
}

/// Group-related UI methods
pub mod group;

/// Dependency-related UI methods
pub mod dependency;

/// Instance-related UI methods
pub mod instance;

/// Package-related UI methods
pub mod package;

/// Icon and styling UI methods
pub mod icon;

/// Utility UI methods for links and paths
pub mod util {
  use super::*;

  /// Return a right-aligned column of a count of instances
  /// Example "    38x"
  pub fn count_column(count: usize) -> String {
    match DEFAULT_INDENT {
      0 => format!("{: >0}x", count),
      1 => format!("{: >1}x", count),
      2 => format!("{: >2}x", count),
      3 => format!("{: >3}x", count),
      4 => format!("{: >4}x", count),
      5 => format!("{: >5}x", count),
      6 => format!("{: >6}x", count),
      _ => format!("{: >7}x", count),
    }
    .dimmed()
    .to_string()
  }

  /// Render the reason code as a clickable link
  pub fn status_code_link(ctx: &Context, pascal_case: &str) -> String {
    let base_url = "https://jamiemason.github.io/syncpack/guide/status-codes/";
    let lower_case = pascal_case.to_lowercase();
    link(ctx, format!("{base_url}#{lower_case}"), pascal_case)
  }

  /// Render a clickable link
  pub fn link(ctx: &Context, url: impl Into<String>, text: impl Into<ColoredString>) -> String {
    if ctx.config.cli.disable_ansi {
      text.into().to_string()
    } else {
      format!("\u{1b}]8;;{}\u{1b}\\{}\u{1b}]8;;\u{1b}\\", url.into(), text.into())
    }
  }

  /// Convert eg. "/dependencies/react" to ".dependencies.react"
  pub fn format_path(path: &str) -> String {
    if path == "/" {
      "root".to_string()
    } else {
      path.replace("/", ".")
    }
  }
}
