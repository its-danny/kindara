return {
	on_use = function(_, action, var)
		action.combat_log({
			source = var.source.entity,
			target = var.source.entity,
			used = { message = "You strike swiftly." },
		})
	end,

	on_dodge = function(_, action, var)
		action.combat_log({
			source = var.target.entity,
			target = var.source.entity,
			dodged = { message = "They dodge your strike." },
		})

		action.combat_log({
			source = var.source.entity,
			target = var.target.entity,
			dodged = { message = "You dodge their strike." },
		})
	end,

	on_block = function(_, action, var)
		action.combat_log({
			source = var.target.entity,
			target = var.source.entity,
			blocked = { message = "They block your strike." },
		})

		action.combat_log({
			source = var.source.entity,
			target = var.target.entity,
			dodged = { message = "You block their strike." },
		})
	end,

	on_hit = function(_, action, var)
		action.apply_damage({
			target = var.target.entity,
			damage = var.source.stats:auto_attack_damage(),
			kind = "physical",
			after = function(damage, kind, _)
				action.combat_log({
					source = var.source.entity,
					target = var.source.entity,
					damaged = { message = "Your strike hits!", damage = damage, kind = kind },
				})

				action.combat_log({
					source = var.source.entity,
					target = var.target.entity,
					damaged = { message = "Their strike hits!", damage = damage, kind = kind },
				})
			end,
		})
	end,
}
