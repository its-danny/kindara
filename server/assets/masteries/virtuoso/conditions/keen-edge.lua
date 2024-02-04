return {
  on_init = function(self, action, var)
    action.combat_log(var.source.entity, var.source.entity, {
      condition_applied = {
        condition = "Keen Edge",
        message = "Your blade is now keenly edged for critical strikes.",
      },
    })

    self.modifier_id = action.add_stat_modifier(var.source.entity, {
      stat = var.stat.CritStrikeChance,
      amount = 0.15,
    })
  end,

  on_end = function(self, action, var)
    action.combat_log(var.source.entity, var.source.entity, {
      condition_removed = {
        condition = "Keen Edge",
        message = "Your blade is no longer keenly edged for critical strikes.",
      },
    })

    action.remove_stat_modifier(var.source.entity, {
      id = self.modifier_id,
    })
  end,
}
