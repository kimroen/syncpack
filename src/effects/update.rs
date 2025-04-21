use {
  super::ui::Ui,
  crate::{
    context::Context,
    instance_state::{FixableInstance, InstanceState, InvalidInstance},
    version_group::VersionGroupVariant,
  },
};

/// Run the update command side effects
pub fn run(ctx: Context) -> ! {
  let ui = Ui::new();
  let mut is_invalid = false;

  ctx
    .version_groups
    .iter()
    .filter(|group| group.matches_cli_filter && matches!(group.variant, VersionGroupVariant::HighestSemver))
    .for_each(|group| {
      ui.group.print_header(&ctx, group);
      group.dependencies.values().for_each(|dependency| {
        let mut has_printed_header = false;
        dependency.instances.iter().for_each(|instance| {
          let state = instance.state.borrow().clone();
          if let InstanceState::Invalid(InvalidInstance::Fixable(FixableInstance::DiffersToNpmRegistry)) = state {
            is_invalid = true;
            if !has_printed_header {
              has_printed_header = true;
              ui.dependency.print_valid(&ctx, dependency, &group.variant);
            }
            ui.instance.print_fixable(&ctx, instance, &group.variant);
            if !ctx.config.cli.check {
              instance.descriptor.package.borrow().copy_expected_specifier(instance);
            }
          }
        });
      })
    });

  if ctx.config.cli.check {
    std::process::exit(if is_invalid { 1 } else { 0 });
  }

  if !ctx.config.cli.dry_run {
    ctx.packages.all.iter().for_each(|package| {
      package.borrow().write_to_disk(&ctx.config);
    });
  }
  std::process::exit(0);
}