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

#[derive(Debug)]
pub struct Ui<'a> {
  pub ctx: &'a Context,
}

impl Ui<'_> {
  pub fn print_group_header(&self, group: &VersionGroup) {
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

  pub fn join_line(&self, lines: Vec<&String>) -> String {
    lines.into_iter().filter(|line| !line.is_empty()).join(" ")
  }

  pub fn print_ignored_dependency(&self, dependency: &Dependency, group_variant: &VersionGroupVariant) {
    let instances_len = dependency.instances.borrow().len();
    let count = self.count_column(instances_len);
    let name = &dependency.internal_name.dimmed().to_string();
    let local_hint = self.get_local_dependency_hint(dependency);
    let state_links = self.get_dependency_state_links(dependency, group_variant);
    let line = self.join_line(vec![&count, &name, &state_links]);
    info!("{line}");
  }

  pub fn print_valid_dependency(&self, dependency: &Dependency, group_variant: &VersionGroupVariant) {
    let instances_len = dependency.instances.borrow().len();
    let count = self.count_column(instances_len);
    let name = &dependency.internal_name;
    let local_hint = self.get_local_dependency_hint(dependency);
    let expected = self.get_raw_expected_specifier_for_dependency(dependency);
    let expected = expected.dimmed().to_string();
    let line = self.join_line(vec![&count, name, &expected, &local_hint]);
    info!("{line}");
  }

  pub fn print_dependency(&self, dependency: &Dependency, group_variant: &VersionGroupVariant) {
    let instances_len = dependency.instances.borrow().len();
    let count = self.count_column(instances_len);
    let name = &dependency.internal_name;
    let local_hint = self.get_local_dependency_hint(dependency);

    match &dependency.get_state() {
      InstanceState::Valid(variant) => match variant {
        ValidInstance::IsIgnored => {
          self.print_ignored_dependency(dependency, group_variant);
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
          self.print_valid_dependency(dependency, group_variant);
        }
        ValidInstance::SatisfiesSameRangeGroup => {
          let line = self.join_line(vec![&count, name, &local_hint]);
          info!("{line}");
        }
      },
      InstanceState::Invalid(variant) => {
        let name = name.red().to_string();
        let line = self.join_line(vec![&count, &name, &local_hint]);
        info!("{line}");
      }
      InstanceState::Suspect(variant) => {
        let name = name.yellow().to_string();
        let line = self.join_line(vec![&count, &name, &local_hint]);
        info!("{line}");
      }
      InstanceState::Unknown => {
        error!("Dependency '{name}' has an unknown state, this is a bug in syncpack");
        panic!("Unknown Dependency State");
      }
    }
  }

  fn get_local_dependency_hint(&self, dependency: &Dependency) -> String {
    if self.ctx.config.cli.show_hints && dependency.local_instance.borrow().is_some() {
      "(local)".blue().to_string()
    } else {
      "".to_string()
    }
  }

  fn get_instance_state_name(&self, state: &InstanceState, group_variant: &VersionGroupVariant) -> String {
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

  fn get_dependency_state_links(&self, dependency: &Dependency, group_variant: &VersionGroupVariant) -> String {
    let state_links = dependency
      .get_states()
      .iter()
      .map(|state| self.get_instance_state_name(state, group_variant))
      .sorted()
      .unique()
      .map(|state_name| self.get_instance_state_link(&state_name))
      .filter(|state_link| !state_link.is_empty())
      .join(", ");
    if !state_links.is_empty() {
      format!("({state_links})").dimmed().to_string()
    } else {
      "".to_string()
    }
  }

  pub fn print_instance(&self, instance: &Instance, group_variant: &VersionGroupVariant) {
    let state = instance.state.borrow().clone();
    let state_name = self.get_instance_state_name(&state, group_variant);
    let state_link = self.get_instance_state_link_in_parens(&state_name);
    let actual = instance.descriptor.specifier.get_raw();
    let actual = if actual.is_empty() {
      "VERSION_IS_MISSING".yellow().to_string()
    } else {
      actual
    };
    let location = self.instance_location(instance).dimmed();
    let indent = "      ";
    match &state {
      InstanceState::Valid(variant) => match variant {
        ValidInstance::IsIgnored => {
          let icon = self.unknown_icon();
          let actual = actual.dimmed();
          info!("{indent}{icon} {actual} {location} {state_link}");
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
          let icon = self.ok_icon().dimmed();
          let actual = actual.green();
          info!("{indent}{icon} {actual} {location} {state_link}");
        }
      },
      InstanceState::Invalid(variant) => match variant {
        InvalidInstance::Fixable(FixableInstance::IsBanned)
        | InvalidInstance::Fixable(FixableInstance::DiffersToHighestOrLowestSemver)
        | InvalidInstance::Fixable(FixableInstance::DiffersToLocal)
        | InvalidInstance::Fixable(FixableInstance::DiffersToNpmRegistry)
        | InvalidInstance::Fixable(FixableInstance::DiffersToPin)
        | InvalidInstance::Fixable(FixableInstance::DiffersToSnapTarget)
        | InvalidInstance::Fixable(FixableInstance::PinOverridesSemverRange)
        | InvalidInstance::Fixable(FixableInstance::PinOverridesSemverRangeMismatch)
        | InvalidInstance::Unfixable(UnfixableInstance::DependsOnInvalidLocalPackage)
        | InvalidInstance::Unfixable(UnfixableInstance::NonSemverMismatch)
        | InvalidInstance::Unfixable(UnfixableInstance::SameRangeMismatch)
        | InvalidInstance::Conflict(SemverGroupAndVersionConflict::MatchConflictsWithHighestOrLowestSemver)
        | InvalidInstance::Conflict(SemverGroupAndVersionConflict::MatchConflictsWithLocal)
        | InvalidInstance::Conflict(SemverGroupAndVersionConflict::MatchConflictsWithSnapTarget)
        | InvalidInstance::Conflict(SemverGroupAndVersionConflict::MismatchConflictsWithHighestOrLowestSemver)
        | InvalidInstance::Conflict(SemverGroupAndVersionConflict::MismatchConflictsWithLocal)
        | InvalidInstance::Conflict(SemverGroupAndVersionConflict::MismatchConflictsWithSnapTarget) => {
          let icon = self.err_icon().dimmed();
          let actual = actual.red();
          info!("{indent}{icon} {actual} {location} {state_link}");
        }
        InvalidInstance::Fixable(FixableInstance::SemverRangeMismatch) => {
          self.print_fixable_instance(instance, group_variant);
        }
      },
      InstanceState::Suspect(variant) => match variant {
        SuspectInstance::DependsOnMissingSnapTarget
        | SuspectInstance::InvalidLocalVersion
        | SuspectInstance::RefuseToBanLocal
        | SuspectInstance::RefuseToPinLocal
        | SuspectInstance::RefuseToSnapLocal => {
          let icon = self.warn_icon();
          info!("{indent}{icon} {actual} {location} {state_link}");
        }
      },
      InstanceState::Unknown => {
        error!("Instance '{location}' has an unknown state, this is a bug in syncpack");
        panic!("Unknown Instance State");
      }
    }
  }

  pub fn print_fixable_instance(&self, instance: &Instance, group_variant: &VersionGroupVariant) {
    let indent = "      ";
    let icon = self.err_icon().dimmed();
    let actual = instance.descriptor.specifier.get_raw();
    let actual = actual.red();
    let arrow = self.dim_right_arrow();
    let expected = instance.expected_specifier.borrow().as_ref().unwrap().get_raw();
    let expected = expected.green();
    let location = self.instance_location(instance).dimmed();
    let state = instance.state.borrow().clone();
    let state_name = self.get_instance_state_name(&state, group_variant);
    let state_link = self.get_instance_state_link_in_parens(&state_name);
    info!("{indent}{icon} {actual} {arrow} {expected} {location} {state_link}");
  }

  pub fn ok_icon(&self) -> ColoredString {
    "✓".green()
  }

  pub fn err_icon(&self) -> ColoredString {
    "✘".red()
  }

  pub fn warn_icon(&self) -> ColoredString {
    "!".yellow()
  }

  fn unknown_icon(&self) -> ColoredString {
    "?".dimmed()
  }

  pub fn dim_right_arrow(&self) -> ColoredString {
    "→".dimmed()
  }

  /// Return a right-aligned column of a count of instances
  /// Example "    38x"
  pub fn count_column(&self, count: usize) -> String {
    format!("{: >6}x", count).dimmed().to_string()
  }

  /// Return a location hint for an instance
  pub fn instance_location(&self, instance: &Instance) -> ColoredString {
    let path_to_prop = instance.descriptor.dependency_type.path.replace("/", ".");
    let file_link = self.package_json_link(&instance.descriptor.package.borrow());
    format!("in {file_link} at {path_to_prop}").normal()
  }

  pub fn print_empty_group(&self) {
    warn!("Version Group is empty");
  }

  pub fn print_ignored_group(&self, group: &VersionGroup) {
    let dependencies_count = group.dependencies.borrow().len();
    let count = self.count_column(dependencies_count);
    let message = "Ignored Dependencies".dimmed();
    info!("{count} {message}");
    let instances_count = group
      .dependencies
      .borrow()
      .values()
      .fold(0, |acc, dep| acc + dep.instances.borrow().len());
    let count = self.count_column(instances_count);
    let message = "Ignored Instances".dimmed();
    info!("{count} {message}");
  }

  /// Packages which are correctly formatted
  pub fn print_formatted_packages(&self, packages: &[Rc<RefCell<PackageJson>>]) {
    if !packages.is_empty() {
      let icon = self.ok_icon();
      let count = self.count_column(packages.len());
      let status = "Valid".green();
      info!("{count} {icon} {status}");
      if self.ctx.config.cli.show_packages {
        packages
          .iter()
          .sorted_by_key(|package| package.borrow().name.clone())
          .for_each(|package| {
            self.print_formatted_package(&package.borrow());
          });
      }
    }
  }

  /// Print a package.json which is correctly formatted
  fn print_formatted_package(&self, package: &PackageJson) {
    if package.formatting_mismatches.borrow().is_empty() {
      let icon = "-".dimmed();
      let file_link = self.package_json_link(package).dimmed();
      info!("          {icon} {file_link}");
    }
  }

  /// Print every package.json which has the given formatting mismatch
  pub fn print_formatting_mismatches(&self, variant: &FormatMismatchVariant, mismatches: &[Rc<FormatMismatch>]) {
    let count = self.count_column(mismatches.len());
    let icon = self.err_icon();
    let status_code = format!("{:?}", variant);
    let link = self.status_code_link(&status_code).red();
    info!("{count} {icon} {link}");
    if self.ctx.config.cli.show_packages {
      mismatches
        .iter()
        .sorted_by_key(|mismatch| mismatch.package.borrow().name.clone())
        .for_each(|mismatch| {
          let icon = "-".dimmed();
          let package = mismatch.package.borrow();
          let property_path = self.format_path(&mismatch.property_path);
          let file_link = self.package_json_link(&package);
          let msg = format!("          {icon} {property_path} of {file_link}").red();
          info!("{msg}");
        });
    }
  }

  /// Render a clickable link to a package.json file
  fn package_json_link(&self, package: &PackageJson) -> String {
    let file_path = package.file_path.to_str().unwrap();
    self.link(format!("file:{file_path}"), package.name.clone())
  }

  /// If enabled, render the reason code as a clickable link
  pub fn get_instance_state_link(&self, pascal_case: &str) -> String {
    if self.ctx.config.cli.show_status_codes {
      self.status_code_link(pascal_case)
    } else {
      "".to_string()
    }
  }

  pub fn get_instance_state_link_in_parens(&self, state_name: &str) -> String {
    let state_link = self.get_instance_state_link(state_name);
    if !state_link.is_empty() {
      format!("({state_link})").dimmed().to_string()
    } else {
      state_link
    }
  }

  /// Render the reason code as a clickable link
  fn status_code_link(&self, pascal_case: &str) -> String {
    let base_url = "https://jamiemason.github.io/syncpack/guide/status-codes/";
    let lower_case = pascal_case.to_lowercase();
    self.link(format!("{base_url}#{lower_case}"), pascal_case)
  }

  /// Render a clickable link
  fn link(&self, url: impl Into<String>, text: impl Into<ColoredString>) -> String {
    if self.ctx.config.cli.disable_ansi {
      text.into().to_string()
    } else {
      format!("\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\", url.into(), text.into())
    }
  }

  /// Convert eg. "/dependencies/react" to ".dependencies.react"
  fn format_path(&self, path: &str) -> String {
    if path == "/" {
      "root".to_string()
    } else {
      path.replace("/", ".")
    }
  }

  fn get_raw_expected_specifier_for_dependency(&self, dependency: &Dependency) -> String {
    dependency
      .expected
      .borrow()
      .as_ref()
      .map(|expected| expected.get_raw())
      .unwrap_or_default()
  }
}
