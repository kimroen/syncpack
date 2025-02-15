use {
  super::ui::Ui,
  crate::{
    context::Context,
    instance_state::{FixableInstance, InstanceState, InvalidInstance, SuspectInstance},
  },
  colored::*,
  log::{info, warn},
};

/// Run the fix command side effects
pub fn run(ctx: Context) -> ! {
  let ui = Ui { ctx: &ctx };

  let mut valid = 0;
  let mut fixable = 0;
  let mut unfixable = 0;
  let mut suspect = 0;

  ctx.instances.iter().for_each(|instance| {
    let internal_name = &instance.descriptor.internal_name;

    if !instance.descriptor.matches_cli_filter {
      return;
    }

    let location = ui.instance_location(instance).dimmed();
    let state = instance.state.borrow().clone();
    let state_name = state.get_name();
    let state_link = ui.get_instance_state_link(&state_name);
    let state_link = format!("({state_link})").dimmed();

    match state {
      InstanceState::Unknown => {}
      InstanceState::Valid(_) => {
        valid += 1;
      }
      InstanceState::Invalid(variant) => match variant {
        InvalidInstance::Fixable(variant) => {
          fixable += 1;
          match variant {
            FixableInstance::IsBanned => instance.remove(),
            _ => {
              let actual = instance.descriptor.specifier.get_raw().red();
              let arrow = ui.dim_right_arrow();
              let expected = instance.expected_specifier.borrow().as_ref().unwrap().get_raw().green();
              info!("{internal_name} {actual} {arrow} {expected} {location} {state_link}");
              instance.descriptor.package.borrow().copy_expected_specifier(instance);
            }
          }
        }
        InvalidInstance::Conflict(_) | InvalidInstance::Unfixable(_) => {
          unfixable += 1;
          warn!("Unfixable: {internal_name} {location} {state_link}");
        }
      },
      InstanceState::Suspect(variant) => match variant {
        SuspectInstance::RefuseToBanLocal
        | SuspectInstance::DependsOnMissingSnapTarget
        | SuspectInstance::RefuseToPinLocal
        | SuspectInstance::RefuseToSnapLocal
        | SuspectInstance::InvalidLocalVersion => {
          suspect += 1;
          warn!("Suspect: {internal_name} {location} {state_link}");
        }
      },
    }
  });

  info!("{} {} Already Valid", ui.count_column(valid), ui.ok_icon());
  info!("{} {} Fixed", ui.count_column(fixable), ui.ok_icon());
  info!("{} {} Unfixable", ui.count_column(unfixable), ui.err_icon());
  info!("{} {} Suspect", ui.count_column(suspect), ui.warn_icon());

  if !ctx.config.cli.dry_run {
    ctx.packages.all.iter().for_each(|package| {
      package.borrow().write_to_disk(&ctx.config);
    });
  }

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
      InstanceState::Invalid(InvalidInstance::Fixable(_)) => {
        continue;
      }
      _ => std::process::exit(1),
    }
  }

  std::process::exit(0);
}
