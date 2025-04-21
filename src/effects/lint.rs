use crate::{context::Context, effects::ui::Ui};

/// Run the lint command side effects
pub fn run(ctx: Context) -> ! {
  let ui = Ui::new(&ctx);
  let mut is_invalid = false;

  ctx
    .version_groups
    .iter()
    .filter(|group| group.matches_cli_filter)
    .for_each(|group| {
      ui.print_group_header(group);
      if group.dependencies.is_empty() {
        ui.print_empty_group();
        return;
      }
      if !ctx.config.cli.show_ignored && group.has_ignored_variant() {
        ui.print_ignored_group(group);
        return;
      }
      group.get_sorted_dependencies(&ctx.config.cli.sort).for_each(|dependency| {
        ui.print_dependency(dependency, &group.variant);
        dependency.get_sorted_instances().for_each(|instance| {
          if !instance.is_valid() || ctx.config.cli.show_instances {
            ui.print_instance(instance, &group.variant);
          }
          if instance.is_invalid() || (instance.is_suspect() && ctx.config.rcfile.strict) {
            is_invalid = true;
          }
        });
      });
    });

  std::process::exit(if is_invalid { 1 } else { 0 });
}
