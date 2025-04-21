use {
  super::ui::Ui,
  crate::{context::Context, instance_state::InstanceState},
  colored::*,
  log::info,
};

/// Run the fix command side effects
pub fn run(ctx: Context) -> ! {
  let ui = Ui::new(&ctx);
  let mut is_invalid = false;

  let get_instance_state_icon = |state: &InstanceState| -> String {
    if state.is_valid() || state.is_fixable() {
      ui.ok_icon().to_string()
    } else if state.is_suspect() {
      ui.warn_icon().to_string()
    } else {
      ui.err_icon().to_string()
    }
  };

  ctx
    .version_groups
    .iter()
    .filter(|group| group.matches_cli_filter)
    .for_each(|group| {
      if group.dependencies.is_empty() || group.has_ignored_variant() {
        return;
      }
      let mut has_shown_group_header = false;
      group.get_sorted_dependencies(&ctx.config.cli.sort).for_each(|dependency| {
        let mut has_shown_dependency_header = false;
        dependency.get_sorted_instances().for_each(|instance| {
          let cannot_autofix = instance.is_unfixable() || instance.is_suspect() && ctx.config.rcfile.strict;
          if instance.is_fixable() || cannot_autofix {
            if !has_shown_group_header {
              ui.print_group_header(group);
              has_shown_group_header = true;
            }
            if !has_shown_dependency_header {
              let alias_hint = ui.get_alias_hint(dependency);
              let state = dependency.get_state();
              let icon = get_instance_state_icon(&state);
              let line = ui.join_line(vec![&icon, &dependency.internal_name, &alias_hint]);
              info!("{line}");
              has_shown_dependency_header = true;
            }
            if instance.is_fixable() {
              if instance.is_banned() {
                let name = &instance.descriptor.name;
                let location = ui.instance_location(instance).dimmed();
                let state_link = ui.get_instance_state_link_in_parens(instance, &group.variant);
                info!("  {location} {state_link}");
                instance.remove()
              } else {
                let name = &instance.descriptor.name;
                let expected = ui.get_expected(instance).dimmed();
                let location = ui.instance_location(instance).dimmed();
                info!("  {expected} {location}");
                instance.descriptor.package.borrow().copy_expected_specifier(instance);
              }
            } else if cannot_autofix {
              is_invalid = true;
              let name = &instance.descriptor.name;
              let actual = ui.get_actual(instance);
              let location = ui.instance_location(instance).dimmed();
              let state_link = ui.get_instance_state_link_in_parens(instance, &group.variant);
              info!("  {actual} {location} {state_link}");
            }
          }
        });
      });
    });

  if !ctx.config.cli.dry_run {
    ctx.packages.all.iter().for_each(|package| {
      package.borrow().write_to_disk(&ctx.config);
    });
  }

  std::process::exit(if is_invalid { 1 } else { 0 });
}
