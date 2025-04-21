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

/// Core UI structure that contains child UI modules for different functionality areas
#[derive(Debug)]
pub struct Ui {
  indent: usize,
  pub group: GroupUI,
  pub dependency: DependencyUI,
  pub instance: InstanceUI,
  pub package: PackageUI,
  pub icon: IconUI,
  pub util: UtilUI,
}

/// Group-related UI methods
#[derive(Debug)]
pub struct GroupUI {
  indent: usize,
}

/// Dependency-related UI methods
#[derive(Debug)]
pub struct DependencyUI {
  indent: usize,
}

/// Instance-related UI methods
#[derive(Debug)]
pub struct InstanceUI {
  indent: usize,
}

/// Package-related UI methods
#[derive(Debug)]
pub struct PackageUI {
  indent: usize,
}

/// Icon and styling UI methods
#[derive(Debug)]
pub struct IconUI {
  indent: usize,
}

/// Utility UI methods for links and path formatting
#[derive(Debug)]
pub struct UtilUI {
  indent: usize,
}

// ===== Core UI Methods =====
impl Ui {
  pub fn new() -> Self {
    let indent = 4;
    Self {
      indent,
      group: GroupUI { indent },
      dependency: DependencyUI { indent },
      instance: InstanceUI { indent },
      package: PackageUI { indent },
      icon: IconUI { indent },
      util: UtilUI { indent },
    }
  }

  pub fn join_line(&self, lines: Vec<&String>) -> String {
    lines.into_iter().filter(|line| !line.is_empty()).join(" ")
  }
}

// ===== Group-related Methods =====
impl GroupUI {
  pub fn print_header(&self, ctx: &Context, group: &VersionGroup) {
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

  pub fn print_empty(&self) {
    warn!("Version Group is empty");
  }

  pub fn print_ignored(&self, group: &VersionGroup) {
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
    let instances_count = self.count_column(instances_count);
    let dependencies_count = group.dependencies.len();
    let dep_plurality = if dependencies_count == 1 { "dependency" } else { "dependencies" };
    let line = format!("{instances_count} {instance_plurality} ignored inside {dependencies_count} {dep_plurality}").dimmed();
    info!("{line}");
  }
  
  /// Return a right-aligned column of a count of instances
  /// Example "    38x"
  fn count_column(&self, count: usize) -> String {
    match self.indent {
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
}

// ===== Dependency-related Methods =====
impl DependencyUI {
  pub fn print(&self, ctx: &Context, dependency: &Dependency, group_variant: &VersionGroupVariant) {
    let instances_len = dependency.instances.len();
    let count = self.count_column(instances_len);
    let name = &dependency.internal_name;
    let local_hint = self.get_local_hint(ctx, dependency);

    match &dependency.get_state() {
      InstanceState::Valid(variant) => match variant {
        ValidInstance::IsIgnored => {
          self.print_ignored(ctx, dependency, group_variant);
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
          self.print_valid(ctx, dependency, group_variant);
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

  pub fn print_ignored(&self, ctx: &Context, dependency: &Dependency, group_variant: &VersionGroupVariant) {
    let instances_len = dependency.instances.len();
    let count = self.count_column(instances_len);
    let name = &dependency.internal_name.dimmed().to_string();
    let local_hint = self.get_local_hint(ctx, dependency);
    let line = self.join_line(vec![&count, &name]);
    info!("{line}");
  }

  pub fn print_valid(&self, ctx: &Context, dependency: &Dependency, group_variant: &VersionGroupVariant) {
    let instances_len = dependency.instances.len();
    let count = self.count_column(instances_len);
    let name = &dependency.internal_name;
    let local_hint = self.get_local_hint(ctx, dependency);
    let expected = self.get_raw_expected_specifier(dependency);
    let expected = expected.dimmed().to_string();
    let line = self.join_line(vec![&count, name, &expected, &local_hint]);
    info!("{line}");
  }

  pub fn get_alias_hint(&self, dependency: &Dependency) -> String {
    if dependency.has_alias {
      "[alias]".magenta().to_string()
    } else {
      "".to_string()
    }
  }

  fn get_local_hint(&self, ctx: &Context, dependency: &Dependency) -> String {
    if ctx.config.cli.show_hints && dependency.local_instance.borrow().is_some() {
      "[local]".blue().to_string()
    } else {
      "".to_string()
    }
  }

  fn get_raw_expected_specifier(&self, dependency: &Dependency) -> String {
    dependency
      .expected
      .borrow()
      .as_ref()
      .map(|expected| expected.get_raw())
      .unwrap_or_default()
  }
  
  fn join_line(&self, lines: Vec<&String>) -> String {
    lines.into_iter().filter(|line| !line.is_empty()).join(" ")
  }
  
  /// Return a right-aligned column of a count of instances
  /// Example "    38x"
  fn count_column(&self, count: usize) -> String {
    match self.indent {
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
}

// ===== Instance-related Methods =====
impl InstanceUI {
  pub fn print(&self, ctx: &Context, instance: &Instance, group_variant: &VersionGroupVariant) {
    let state = instance.state.borrow().clone();
    let indent = " ".repeat(self.indent);
    match &state {
      InstanceState::Valid(variant) => match variant {
        ValidInstance::IsIgnored => {
          let no_icon = " ";
          let actual = self.get_actual(instance).dimmed();
          let location = self.get_location(ctx, instance).dimmed();
          let state_link = self.get_state_link_in_parens(ctx, instance, group_variant);
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
          let actual = self.get_actual(instance).dimmed();
          let location = self.get_location(ctx, instance).dimmed();
          let state_link = self.get_state_link_in_parens(ctx, instance, group_variant);
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
          let icon = self.err_icon();
          let actual = self.get_actual(instance).red();
          let location = self.get_location(ctx, instance).dimmed();
          let state_link = self.get_state_link_in_parens(ctx, instance, group_variant);
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
          self.print_fixable(ctx, instance, group_variant);
        }
      },
      InstanceState::Suspect(variant) => match variant {
        SuspectInstance::DependsOnMissingSnapTarget
        | SuspectInstance::InvalidLocalVersion
        | SuspectInstance::RefuseToBanLocal
        | SuspectInstance::RefuseToPinLocal
        | SuspectInstance::RefuseToSnapLocal => {
          let icon = self.warn_icon();
          let actual = self.get_actual(instance).yellow();
          let location = self.get_location(ctx, instance).dimmed();
          let state_link = self.get_state_link_in_parens(ctx, instance, group_variant);
          info!("{indent}{icon} {actual} {location} {state_link}");
        }
      },
      InstanceState::Unknown => {
        let location = self.get_location(ctx, instance);
        error!("Instance '{location}' has an unknown state, this is a bug in syncpack");
        panic!("Unknown Instance State");
      }
    }
  }

  pub fn print_fixable(&self, ctx: &Context, instance: &Instance, group_variant: &VersionGroupVariant) {
    let indent = " ".repeat(self.indent);
    let icon = self.err_icon();
    let suggested_fix = self.get_suggested_fix(instance);
    let location = self.get_location(ctx, instance).dimmed();
    let state_link = self.get_state_link_in_parens(ctx, instance, group_variant);
    info!("{indent}{icon} {suggested_fix} {location} {state_link}");
  }

  pub fn get_actual(&self, instance: &Instance) -> String {
    let actual = instance.descriptor.specifier.get_raw();
    if actual.is_empty() {
      "VERSION_IS_MISSING".yellow().to_string()
    } else {
      actual
    }
  }

  pub fn get_expected(&self, instance: &Instance) -> String {
    instance.expected_specifier.borrow().as_ref().unwrap().get_raw()
  }

  pub fn get_suggested_fix(&self, instance: &Instance) -> String {
    let actual = self.get_actual(instance).red();
    let arrow = self.dim_right_arrow();
    let expected = self.get_expected(instance).green();
    format!("{actual} {arrow} {expected}")
  }

  /// Return a location hint for an instance
  pub fn get_location(&self, ctx: &Context, instance: &Instance) -> ColoredString {
    let path_to_prop = instance.descriptor.dependency_type.path.replace("/", ".");
    let file_link = self.package_json_link(ctx, &instance.descriptor.package.borrow());
    format!("in {file_link} at {path_to_prop}").normal()
  }

  fn get_state_name(&self, instance: &Instance, group_variant: &VersionGroupVariant) -> String {
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
  pub fn get_state_link(&self, ctx: &Context, instance: &Instance, group_variant: &VersionGroupVariant) -> String {
    if ctx.config.cli.show_status_codes {
      let state_name = self.get_state_name(instance, group_variant);
      self.status_code_link(ctx, &state_name)
    } else {
      "".to_string()
    }
  }

  pub fn get_state_link_in_parens(&self, ctx: &Context, instance: &Instance, group_variant: &VersionGroupVariant) -> String {
    let state_link = self.get_state_link(ctx, instance, group_variant);
    if !state_link.is_empty() {
      format!("({state_link})").dimmed().to_string()
    } else {
      state_link
    }
  }
  
  /// Render a clickable link to a package.json file
  fn package_json_link(&self, ctx: &Context, package: &PackageJson) -> String {
    let file_path = package.file_path.to_str().unwrap();
    self.link(ctx, format!("file:{file_path}"), package.name.clone())
  }
  
  /// Render the reason code as a clickable link
  fn status_code_link(&self, ctx: &Context, pascal_case: &str) -> String {
    let base_url = "https://jamiemason.github.io/syncpack/guide/status-codes/";
    let lower_case = pascal_case.to_lowercase();
    self.link(ctx, format!("{base_url}#{lower_case}"), pascal_case)
  }

  /// Render a clickable link
  fn link(&self, ctx: &Context, url: impl Into<String>, text: impl Into<ColoredString>) -> String {
    if ctx.config.cli.disable_ansi {
      text.into().to_string()
    } else {
      format!("\u{1b}]8;;{}\u{1b}\\{}\u{1b}]8;;\u{1b}\\", url.into(), text.into())
    }
  }
  
  fn err_icon(&self) -> ColoredString {
    "\u{2718}".red()
  }
  
  fn warn_icon(&self) -> ColoredString {
    "!".yellow()
  }
  
  fn dim_right_arrow(&self) -> ColoredString {
    "\u{2192}".dimmed()
  }
}

// ===== Package-related Methods =====
impl PackageUI {
  /// Packages which are correctly formatted
  pub fn print_formatted(&self, ctx: &Context, packages: &[Rc<RefCell<PackageJson>>]) {
    if !packages.is_empty() {
      let icon = self.ok_icon();
      let count = self.count_column(packages.len());
      let status = "Valid".green();
      info!("{count} {icon} {status}");
      if ctx.config.cli.show_packages {
        packages
          .iter()
          .sorted_by_key(|package| package.borrow().name.clone())
          .for_each(|package| {
            self.print_formatted_package(ctx, &package.borrow());
          });
      }
    }
  }

  /// Print a package.json which is correctly formatted
  fn print_formatted_package(&self, ctx: &Context, package: &PackageJson) {
    if package.formatting_mismatches.borrow().is_empty() {
      let icon = "-".dimmed();
      let file_link = self.package_json_link(ctx, package).dimmed();
      info!("          {icon} {file_link}");
    }
  }

  /// Print every package.json which has the given formatting mismatch
  pub fn print_formatting_mismatches(&self, ctx: &Context, variant: &FormatMismatchVariant, mismatches: &[Rc<FormatMismatch>]) {
    let count = self.count_column(mismatches.len());
    let icon = self.err_icon();
    let status_code = format!("{:?}", variant);
    let link = self.status_code_link(ctx, &status_code).red();
    info!("{count} {icon} {link}");
    if ctx.config.cli.show_packages {
      mismatches
        .iter()
        .sorted_by_key(|mismatch| mismatch.package.borrow().name.clone())
        .for_each(|mismatch| {
          let icon = "-".dimmed();
          let package = mismatch.package.borrow();
          let property_path = self.format_path(&mismatch.property_path);
          let file_link = self.package_json_link(ctx, &package);
          let msg = format!("          {icon} {property_path} of {file_link}").red();
          info!("{msg}");
        });
    }
  }

  /// Render a clickable link to a package.json file
  fn package_json_link(&self, ctx: &Context, package: &PackageJson) -> String {
    let file_path = package.file_path.to_str().unwrap();
    self.link(ctx, format!("file:{file_path}"), package.name.clone())
  }
  
  /// Render a clickable link
  fn link(&self, ctx: &Context, url: impl Into<String>, text: impl Into<ColoredString>) -> String {
    if ctx.config.cli.disable_ansi {
      text.into().to_string()
    } else {
      format!("\u{1b}]8;;{}\u{1b}\\{}\u{1b}]8;;\u{1b}\\", url.into(), text.into())
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
  
  /// Return a right-aligned column of a count of instances
  /// Example "    38x"
  fn count_column(&self, count: usize) -> String {
    match self.indent {
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
  
  fn ok_icon(&self) -> ColoredString {
    "\u{2713}".green()
  }

  fn err_icon(&self) -> ColoredString {
    "\u{2718}".red()
  }
  
  /// Render the reason code as a clickable link
  fn status_code_link(&self, ctx: &Context, pascal_case: &str) -> String {
    let base_url = "https://jamiemason.github.io/syncpack/guide/status-codes/";
    let lower_case = pascal_case.to_lowercase();
    self.link(ctx, format!("{base_url}#{lower_case}"), pascal_case)
  }
}

// ===== Icon and Styling Methods =====
impl IconUI {
  pub fn ok(&self) -> ColoredString {
    "\u{2713}".green()
  }

  pub fn err(&self) -> ColoredString {
    "\u{2718}".red()
  }

  pub fn warn(&self) -> ColoredString {
    "!".yellow()
  }

  pub fn unknown(&self) -> ColoredString {
    "?".dimmed()
  }

  pub fn dim_right_arrow(&self) -> ColoredString {
    "\u{2192}".dimmed()
  }

  /// Return a right-aligned column of a count of instances
  /// Example "    38x"
  pub fn count_column(&self, count: usize) -> String {
    match self.indent {
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
}

// ===== Utility Methods for Links and Paths =====
impl UtilUI {
  /// Render the reason code as a clickable link
  pub fn status_code_link(&self, ctx: &Context, pascal_case: &str) -> String {
    let base_url = "https://jamiemason.github.io/syncpack/guide/status-codes/";
    let lower_case = pascal_case.to_lowercase();
    self.link(ctx, format!("{base_url}#{lower_case}"), pascal_case)
  }

  /// Render a clickable link
  pub fn link(&self, ctx: &Context, url: impl Into<String>, text: impl Into<ColoredString>) -> String {
    if ctx.config.cli.disable_ansi {
      text.into().to_string()
    } else {
      format!("\u{1b}]8;;{}\u{1b}\\{}\u{1b}]8;;\u{1b}\\", url.into(), text.into())
    }
  }

  /// Convert eg. "/dependencies/react" to ".dependencies.react"
  pub fn format_path(&self, path: &str) -> String {
    if path == "/" {
      "root".to_string()
    } else {
      path.replace("/", ".")
    }
  }
  
  pub fn join_line(&self, lines: Vec<&String>) -> String {
    lines.into_iter().filter(|line| !line.is_empty()).join(" ")
  }
}