use crate::{context::Context, effects::ui::Ui, instance_state::InstanceState, version_group::VersionGroupVariant};

/// Run the lint command side effects
pub fn run(ctx: Context) -> ! {
  let ui = Ui { ctx: &ctx };

  ctx
    .version_groups
    .iter()
    .filter(|group| group.matches_cli_filter)
    .for_each(|group| {
      ui.print_group_header(group);
      if group.dependencies.borrow().is_empty() {
        ui.print_empty_group();
        return;
      }
      if !ctx.config.cli.show_ignored && matches!(group.variant, VersionGroupVariant::Ignored) {
        ui.print_ignored_group(group);
        return;
      }
      group.for_each_dependency(&ctx.config.cli.sort, |dependency| {
        if !dependency.matches_cli_filter {
          return;
        }
        ui.print_dependency(dependency, &group.variant);
        dependency.for_each_instance(|instance| {
          if !matches!(*instance.state.borrow(), InstanceState::Valid(_)) || ctx.config.cli.show_instances {
            if !instance.descriptor.matches_cli_filter {
              return;
            }
            ui.print_instance(instance, &group.variant);
          }
        });
      });
    });

  for instance in ctx.instances.iter() {
    match instance.state.borrow().clone() {
      InstanceState::Valid(_) => continue,
      InstanceState::Suspect(_) => {
        if ctx.config.rcfile.strict {
          std::process::exit(1);
        } else {
          continue;
        }
      }
      _ => std::process::exit(1),
    }
  }

  std::process::exit(0);
}
