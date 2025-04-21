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

/// Group-related UI methods
pub mod group {
  use super::*;

  pub fn print_header(ctx: &Context, group: &VersionGroup) {
    let print_width = 80;
    let label = &group.selector.label;
    let header = format!("= {label} ");
    let divider = if header.len() < print_width {
      "=".repeat(print_width - header.len())
    } else {
      "".to_string()
    };
    let full_header = format!("{header}{divider}");
    info!("{}", full_header.blue());
  }

  pub fn print_empty() {
    warn!("Version Group is empty");
  }

  pub fn print_ignored(group: &VersionGroup) {
    let instances_count = group.dependencies.values().fold(0, |acc, dep| {
      acc
        + dep
          .instances
          .iter()
          .filter(|instance| instance.descriptor.matches_cli_filter)
          .collect::<Vec<_>>()
          .len()
    });
    let instance_plurality = if instances_count == 1 { "instance" } else { "instances" };
    let instances_count = super::util::count_column(instances_count);
    let dependencies_count = group.dependencies.len();
    let dep_plurality = if dependencies_count == 1 { "dependency" } else { "dependencies" };
    let line = format!("{instances_count} {instance_plurality} ignored inside {dependencies_count} {dep_plurality}").dimmed();
    info!("{line}");
  }
}

/// Dependency-related UI methods
pub mod dependency {
  use super::*;

  pub fn print(ctx: &Context, dependency: &Dependency, group_variant: &VersionGroupVariant) {
    let instances_len = dependency.instances.len();
    let count = util::count_column(instances_len);
    let name = &dependency.internal_name;
    let local_hint = get_local_hint(ctx, dependency);

    match &dependency.get_state() {
      InstanceState::Valid(variant) => match variant {
        ValidInstance::IsIgnored => {
          print_ignored(ctx, dependency, group_variant);
        }
        ValidInstance::IsHighestOrLowestSemver
        | ValidInstance::IsIdenticalToLocal
        | ValidInstance::IsIdenticalToPin
        | ValidInstance::IsIdenticalToSnapTarget
        | ValidInstance::IsLocalAndValid
        | ValidInstance::IsNonSemverButIdentical
        | ValidInstance::SatisfiesHighestOrLowestSemver
        | ValidInstance::SatisfiesLocal
        | ValidInstance::SatisfiesSnapTarget => {
          print_valid(ctx, dependency, group_variant);
        }
        ValidInstance::SatisfiesSameRangeGroup => {
          let line = util::join_line(vec![&count, name, &local_hint]);
          info!("{line}");
        }
      },
      InstanceState::Invalid(variant) => {
        let name = name.red().to_string();
        let line = util::join_line(vec![&count, &name, &local_hint]);
        info!("{line}");
      }
      InstanceState::Suspect(variant) => {
        let name = name.yellow().to_string();
        let line = util::join_line(vec![&count, &name, &local_hint]);
        info!("{line}");
      }
      InstanceState::Unknown => {
        error!("Dependency '{name}' has an unknown state, this is a bug in syncpack");
        panic!("Unknown Dependency State");
      }
    }
  }

  pub fn print_ignored(ctx: &Context, dependency: &Dependency, group_variant: &VersionGroupVariant) {
    let instances_len = dependency.instances.len();
    let count = util::count_column(instances_len);
    let name = &dependency.internal_name.dimmed().to_string();
    let local_hint = get_local_hint(ctx, dependency);
    let line = util::join_line(vec![&count, &name]);
    info!("{line}");
  }

  pub fn print_valid(ctx: &Context, dependency: &Dependency, group_variant: &VersionGroupVariant) {
    let instances_len = dependency.instances.len();
    let count = util::count_column(instances_len);
    let name = &dependency.internal_name;
    let local_hint = get_local_hint(ctx, dependency);
    let expected = get_raw_expected_specifier(dependency);
    let expected = expected.dimmed().to_string();
    let line = util::join_line(vec![&count, name, &expected, &local_hint]);
    info!("{line}");
  }

  pub fn get_alias_hint(dependency: &Dependency) -> String {
    if dependency.has_alias {
      "[alias]".magenta().to_string()
    } else {
      "".to_string()
    }
  }

  fn get_local_hint(ctx: &Context, dependency: &Dependency) -> String {
    if ctx.config.cli.show_hints && dependency.local_instance.borrow().is_some() {
      "[local]".blue().to_string()
    } else {
      "".to_string()
    }
  }

  fn get_raw_expected_specifier(dependency: &Dependency) -> String {
    dependency
      .expected
      .borrow()
      .as_ref()
      .map(|expected| expected.get_raw())
      .unwrap_or_default()
  }
}

/// Instance-related UI methods
pub mod instance {
  use super::*;

  pub fn print(ctx: &Context, instance: &Instance, group_variant: &VersionGroupVariant) {
    let state = instance.state.borrow().clone();
    let indent = " ".repeat(DEFAULT_INDENT);
    match &state {
      InstanceState::Valid(variant) => match variant {
        ValidInstance::IsIgnored => {
          let no_icon = " ";
          let actual = get_actual(instance).dimmed();
          let location = get_location(ctx, instance).dimmed();
          let state_link = get_state_link_in_parens(ctx, instance, group_variant);
          info!("{indent}{no_icon} {actual} {location} {state_link}");
        }
        ValidInstance::IsHighestOrLowestSemver
        | ValidInstance::IsIdenticalToLocal
        | ValidInstance::IsIdenticalToPin
        | ValidInstance::IsIdenticalToSnapTarget
        | ValidInstance::IsLocalAndValid
        | ValidInstance::IsNonSemverButIdentical
        | ValidInstance::SatisfiesHighestOrLowestSemver
        | ValidInstance::SatisfiesLocal
        | ValidInstance::SatisfiesSameRangeGroup
        | ValidInstance::SatisfiesSnapTarget => {
          let no_icon = " ";
          let actual = get_actual(instance).dimmed();
          let location = get_location(ctx, instance).dimmed();
          let state_link = get_state_link_in_parens(ctx, instance, group_variant);
          info!("{indent}{no_icon} {actual} {location} {state_link}");
        }
      },
      InstanceState::Invalid(variant) => match variant {
        InvalidInstance::Unfixable(UnfixableInstance::DependsOnInvalidLocalPackage)
        | InvalidInstance::Unfixable(UnfixableInstance::NonSemverMismatch)
        | InvalidInstance::Unfixable(UnfixableInstance::SameRangeMismatch)
        | InvalidInstance::Conflict(SemverGroupAndVersionConflict::MatchConflictsWithHighestOrLowestSemver)
        | InvalidInstance::Conflict(SemverGroupAndVersionConflict::MatchConflictsWithLocal)
        | InvalidInstance::Conflict(SemverGroupAndVersionConflict::MatchConflictsWithSnapTarget)
        | InvalidInstance::Conflict(SemverGroupAndVersionConflict::MismatchConflictsWithHighestOrLowestSemver)
        | InvalidInstance::Conflict(SemverGroupAndVersionConflict::MismatchConflictsWithLocal)
        | InvalidInstance::Conflict(SemverGroupAndVersionConflict::MismatchConflictsWithSnapTarget) => {
          let icon = icon::err();
          let actual = get_actual(instance).red();
          let location = get_location(ctx, instance).dimmed();
          let state_link = get_state_link_in_parens(ctx, instance, group_variant);
          info!("{indent}{icon} {actual} {location} {state_link}");
        }
        InvalidInstance::Fixable(FixableInstance::DiffersToHighestOrLowestSemver)
        | InvalidInstance::Fixable(FixableInstance::DiffersToLocal)
        | InvalidInstance::Fixable(FixableInstance::DiffersToNpmRegistry)
        | InvalidInstance::Fixable(FixableInstance::DiffersToPin)
        | InvalidInstance::Fixable(FixableInstance::DiffersToSnapTarget)
        | InvalidInstance::Fixable(FixableInstance::IsBanned)
        | InvalidInstance::Fixable(FixableInstance::PinOverridesSemverRange)
        | InvalidInstance::Fixable(FixableInstance::PinOverridesSemverRangeMismatch)
        | InvalidInstance::Fixable(FixableInstance::SemverRangeMismatch) => {
          print_fixable(ctx, instance, group_variant);
        }
      },
      InstanceState::Suspect(variant) => match variant {
        SuspectInstance::DependsOnMissingSnapTarget
        | SuspectInstance::InvalidLocalVersion
        | SuspectInstance::RefuseToBanLocal
        | SuspectInstance::RefuseToPinLocal
        | SuspectInstance::RefuseToSnapLocal => {
          let icon = icon::warn();
          let actual = get_actual(instance).yellow();
          let location = get_location(ctx, instance).dimmed();
          let state_link = get_state_link_in_parens(ctx, instance, group_variant);
          info!("{indent}{icon} {actual} {location} {state_link}");
        }
      },
      InstanceState::Unknown => {
        let location = get_location(ctx, instance);
        error!("Instance '{location}' has an unknown state, this is a bug in syncpack");
        panic!("Unknown Instance State");
      }
    }
  }

  pub fn print_fixable(ctx: &Context, instance: &Instance, group_variant: &VersionGroupVariant) {
    let indent = " ".repeat(DEFAULT_INDENT);
    let icon = icon::err();
    let suggested_fix = get_suggested_fix(instance);
    let location = get_location(ctx, instance).dimmed();
    let state_link = get_state_link_in_parens(ctx, instance, group_variant);
    info!("{indent}{icon} {suggested_fix} {location} {state_link}");
  }

  pub fn get_actual(instance: &Instance) -> String {
    let actual = instance.descriptor.specifier.get_raw();
    if actual.is_empty() {
      "VERSION_IS_MISSING".yellow().to_string()
    } else {
      actual
    }
  }

  pub fn get_expected(instance: &Instance) -> String {
    instance.expected_specifier.borrow().as_ref().unwrap().get_raw()
  }

  pub fn get_suggested_fix(instance: &Instance) -> String {
    let actual = get_actual(instance).red();
    let arrow = icon::dim_right_arrow();
    let expected = get_expected(instance).green();
    format!("{actual} {arrow} {expected}")
  }

  /// Return a location hint for an instance
  pub fn get_location(ctx: &Context, instance: &Instance) -> ColoredString {
    let path_to_prop = instance.descriptor.dependency_type.path.replace("/", ".");
    let file_link = package::package_json_link(ctx, &instance.descriptor.package.borrow());
    format!("in {file_link} at {path_to_prop}").normal()
  }

  fn get_state_name(instance: &Instance, group_variant: &VersionGroupVariant) -> String {
    let state = instance.state.borrow().clone();
    let state_name = state.get_name();
    // Issues related to whether a specifier is the highest or lowest semver are
    // all the same logic internally, so we have combined enum branches for
    // them, but from an end user point of view though it is clearer to have a
    // specific status code related to what has happened.
    if matches!(group_variant, VersionGroupVariant::HighestSemver) {
      state_name.replace("HighestOrLowestSemver", "HighestSemver")
    } else if matches!(group_variant, VersionGroupVariant::LowestSemver) {
      state_name.replace("HighestOrLowestSemver", "LowestSemver")
    } else {
      state_name
    }
  }

  /// If enabled, render the reason code as a clickable link
  pub fn get_state_link(ctx: &Context, instance: &Instance, group_variant: &VersionGroupVariant) -> String {
    if ctx.config.cli.show_status_codes {
      let state_name = get_state_name(instance, group_variant);
      util::status_code_link(ctx, &state_name)
    } else {
      "".to_string()
    }
  }

  pub fn get_state_link_in_parens(ctx: &Context, instance: &Instance, group_variant: &VersionGroupVariant) -> String {
    let state_link = get_state_link(ctx, instance, group_variant);
    if !state_link.is_empty() {
      format!("({state_link})").dimmed().to_string()
    } else {
      state_link
    }
  }
}

/// Package-related UI methods
pub mod package {
  use super::*;

  /// Packages which are correctly formatted
  pub fn print_formatted(ctx: &Context, packages: &[Rc<RefCell<PackageJson>>]) {
    if !packages.is_empty() {
      let icon = icon::ok();
      let count = util::count_column(packages.len());
      let status = "Valid".green();
      info!("{count} {icon} {status}");
      if ctx.config.cli.show_packages {
        packages
          .iter()
          .sorted_by_key(|package| package.borrow().name.clone())
          .for_each(|package| {
            print_formatted_package(ctx, &package.borrow());
          });
      }
    }
  }

  /// Print a package.json which is correctly formatted
  fn print_formatted_package(ctx: &Context, package: &PackageJson) {
    if package.formatting_mismatches.borrow().is_empty() {
      let icon = "-".dimmed();
      let file_link = package_json_link(ctx, package).dimmed();
      info!("          {icon} {file_link}");
    }
  }

  /// Print every package.json which has the given formatting mismatch
  pub fn print_formatting_mismatches(ctx: &Context, variant: &FormatMismatchVariant, mismatches: &[Rc<FormatMismatch>]) {
    let count = util::count_column(mismatches.len());
    let icon = icon::err();
    let status_code = format!("{:?}", variant);
    let link = util::status_code_link(ctx, &status_code).red();
    info!("{count} {icon} {link}");
    if ctx.config.cli.show_packages {
      mismatches
        .iter()
        .sorted_by_key(|mismatch| mismatch.package.borrow().name.clone())
        .for_each(|mismatch| {
          let icon = "-".dimmed();
          let package = mismatch.package.borrow();
          let property_path = util::format_path(&mismatch.property_path);
          let file_link = package_json_link(ctx, &package);
          let msg = format!("          {icon} {property_path} of {file_link}").red();
          info!("{msg}");
        });
    }
  }

  /// Render a clickable link to a package.json file
  pub fn package_json_link(ctx: &Context, package: &PackageJson) -> String {
    let file_path = package.file_path.to_str().unwrap();
    util::link(ctx, format!("file:{file_path}"), package.name.clone())
  }
}

/// Icon and styling UI methods
pub mod icon {
  use super::*;

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
}

/// Utility UI methods for links and paths
pub mod util {
  use super::*;
  
  /// Join lines that are not empty with a space separator
  pub fn join_line(lines: Vec<&String>) -> String {
    lines.into_iter().filter(|line| !line.is_empty()).join(" ")
  }

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