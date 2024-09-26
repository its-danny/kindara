return {
  on_use = function(_, action, var)
    action.combat_log(var.source.entity, var.source.entity, {
      used = {
        message =
        "You launch into forceful dash, swiftly closing the gap with a strategic maneuver. Your blade gleams sharply as you prepare to strike.",
      },
    })
  end,

  on_miss = function(_, action, var)
    action.combat_log(var.source.entity, var.source.entity, {
      missed = {
        message = "You attempt to dash forward, but your target is too close, leaving you unable to gain momentum.",
      },
    })
  end,

  on_dodge = function(_, action, var)
    action.set_distance(var.source.entity, { distance = var.distance.Near })

    action.combat_log(var.target.entity, var.source.entity, {
      dodged = {
        message = "As you initiate your dash, your target swiftly evades, leaving your strike to meet only air.",
      },
    })
  end,

  on_block = function(_, action, var)
    action.set_distance(var.source.entity, { distance = var.distance.Near })
    action.apply_damage(var.target.entity, { damage = 5, kind = "physical", stat = var.stat.Dexterity })

    action.combat_log(var.target.entity, var.source.entity, {
      blocked = {
        message = "Your dash meets resistance as your foe skillfully blocks your advance, leaving you unbalanced.",
      },
    })

    -- action.apply_condition({ target = source.entity, id = "unbalanced", duration = 5.0 })
  end,

  on_hit = function(_, action, var)
    action.set_distance(var.source.entity, { distance = var.distance.Near })
    action.set_approach(var.source.entity, { approach = var.approach.Rear })

    action.apply_damage(var.target.entity, {
      damage = 10.0 + (var.source.stats.attributes.dexterity * 2.0),
      kind = "physical",
      after = function(damage, kind, crit)
        local message =
        "You cut through their defenses, landing a solid hit. In the fluid motion, you position yourself advantageously, your blade now keenly edged for critical strikes."

        if crit then
          message =
          "You not only land your advance, but do so with devastating precision. The <fg.red>critical impact</> of your strike deeply wounds your foe, while the thrill of the perfect hit further sharpens your combat senses."
        end

        action.combat_log(var.source.entity, var.source.entity, {
          damaged = { message = message, damage = damage, kind = kind, crit = crit },
        })
      end,
    })

    action.apply_condition(var.source.entity, { id = "keen-edge", duration = 15.0 })
  end,
}
