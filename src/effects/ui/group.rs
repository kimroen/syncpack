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
  let instances_count = ui::util::count_column(instances_count);
  let dependencies_count = group.dependencies.len();
  let dep_plurality = if dependencies_count == 1 { "dependency" } else { "dependencies" };
  let line = format!("{instances_count} {instance_plurality} ignored inside {dependencies_count} {dep_plurality}").dimmed();
  info!("{line}");
}
